"""Contract-parity tests for the Python side of the jarvis-core protocol.

No binary or vault is touched here: these checks pin the Python encoder and
decoder to the exact wire shape asserted by jarvis-core's transport.rs and
events.rs unit tests. tools/test_rust_core_protocol.py covers live interop.
"""

import json
import sys
import tempfile
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import rust_core_adapter as adapter


class FrameShapeTests(unittest.TestCase):
    def test_event_frame_matches_versioned_wire_contract(self):
        wire = adapter.event_frame("ui", "UserRequestReceived", {"k": "v"})
        self.assertEqual(wire["protocol_version"], 1)
        self.assertEqual(wire["body"]["kind"], "event")
        data = wire["body"]["data"]
        self.assertEqual(data["schema_version"], 1)
        self.assertEqual(data["source"], "ui")
        self.assertEqual(data["event_type"], "UserRequestReceived")
        self.assertEqual(data["payload"], {"k": "v"})
        self.assertIsNone(data["dedup_key"])

    def test_heartbeat_frame_shape_matches_rust_contract_test(self):
        # transport.rs wire_shape_is_versioned_contract: kind tag + data.
        wire = adapter.heartbeat_frame("stt")
        line = adapter.encode(wire)
        decoded = json.loads(line)
        self.assertEqual(decoded["protocol_version"], 1)
        self.assertEqual(decoded["body"]["kind"], "heartbeat")
        self.assertEqual(decoded["body"]["data"]["worker"], "stt")

    def test_encode_is_single_newline_terminated_line(self):
        line = adapter.encode(adapter.shutdown_frame("test"))
        self.assertTrue(line.endswith("\n"))
        self.assertEqual(line.count("\n"), 1)
        self.assertNotIn(" ", line.split('"body"')[0])

    def test_ids_match_rust_format_and_length(self):
        ident = adapter.new_id("evt")
        prefix, nanos, counter = ident.split("-")
        self.assertEqual(prefix, "evt")
        self.assertGreaterEqual(len(nanos), 16)
        int(nanos, 16)
        int(counter, 16)
        self.assertLessEqual(len(ident), adapter.MAX_ID_LEN)

    def test_rfc3339_is_utc_z_format(self):
        stamp = adapter.rfc3339_now()
        self.assertTrue(stamp.endswith("Z"))
        parsed = datetime.fromisoformat(stamp.replace("Z", "+00:00"))
        self.assertEqual(parsed.tzinfo, timezone.utc)


class DecodeRejectionTests(unittest.TestCase):
    def assert_reason(self, line, reason):
        with self.assertRaises(adapter.FrameError) as caught:
            adapter.decode(line)
        self.assertEqual(caught.exception.reason, reason)

    def test_empty_and_oversized_lines(self):
        self.assert_reason("", "empty")
        self.assert_reason("   \n", "empty")
        self.assert_reason("x" * (adapter.MAX_FRAME_BYTES + 1), "oversized")

    def test_malformed_json(self):
        self.assert_reason("{not json", "malformed")
        self.assert_reason("[1,2,3]", "malformed")

    def test_wrong_protocol_version(self):
        line = adapter.encode(adapter.heartbeat_frame("w")).replace(
            '"protocol_version":1', '"protocol_version":99'
        )
        self.assert_reason(line, "unsupported_version")

    def test_bad_msg_id(self):
        wire = adapter.heartbeat_frame("w")
        wire["msg_id"] = ""
        self.assert_reason(adapter.encode(wire), "bad_id")
        wire["msg_id"] = "x" * 65
        self.assert_reason(adapter.encode(wire), "bad_id")

    def test_unknown_body_kind(self):
        line = (
            '{"protocol_version":1,"msg_id":"m",'
            '"sent_at_utc":"2026-01-01T00:00:00Z",'
            '"body":{"kind":"teleport","data":{}}}'
        )
        self.assert_reason(line, "malformed")


class EnvelopeValidationTests(unittest.TestCase):
    def test_unknown_source_and_type_rejected(self):
        with self.assertRaises(adapter.FrameError):
            adapter.event_envelope("telegram", "UserRequestReceived")
        with self.assertRaises(adapter.FrameError):
            adapter.event_envelope("ui", "UserTeleported")

    def test_oversized_payload_and_dedup_key(self):
        with self.assertRaises(adapter.FrameError) as caught:
            adapter.event_envelope(
                "ui", "UserRequestReceived", {"blob": "x" * (16 * 1024)}
            )
        self.assertEqual(caught.exception.reason, "oversized_payload")
        with self.assertRaises(adapter.FrameError) as caught:
            adapter.event_envelope(
                "ui", "UserRequestReceived", dedup_key="k" * 129
            )
        self.assertEqual(caught.exception.reason, "oversized_dedup_key")

    def test_future_timestamp_rejected(self):
        future = datetime.now(timezone.utc) + timedelta(seconds=3600)
        with self.assertRaises(adapter.FrameError) as caught:
            adapter.event_envelope("ui", "UserRequestReceived", occurred_at=future)
        self.assertEqual(caught.exception.reason, "future_timestamp")

    def test_schema_version_checked(self):
        envelope = adapter.event_envelope("ui", "UserRequestReceived")
        envelope["schema_version"] = 99
        with self.assertRaises(adapter.FrameError) as caught:
            adapter.validate_envelope(envelope)
        self.assertEqual(caught.exception.reason, "unsupported_version")


class RoundTripTests(unittest.TestCase):
    def test_encode_decode_roundtrip(self):
        wire = adapter.event_frame(
            "voice_pipeline", "UserRequestReceived", {"stage": "intent"}, "turn-1"
        )
        decoded = adapter.decode(adapter.encode(wire))
        self.assertEqual(decoded["msg_id"], wire["msg_id"])
        self.assertEqual(decoded["body"]["data"]["event_id"], wire["body"]["data"]["event_id"])
        adapter.validate_envelope(decoded["body"]["data"])

    def test_rust_shaped_line_decodes(self):
        # Exact shape produced by Rust serde (compact, Option::None -> null).
        line = (
            '{"protocol_version":1,"msg_id":"msg-1",'
            '"sent_at_utc":"2026-11-02T19:45:00.123456Z",'
            '"body":{"kind":"event","data":{"schema_version":1,'
            '"event_id":"evt-1","trace_id":"trace-1","source":"ui",'
            '"event_type":"UserRequestReceived",'
            '"occurred_at_utc":"2026-11-02T19:45:00Z",'
            '"dedup_key":null,"payload":{"note":"probe"}}}}\n'
        )
        wire = adapter.decode(line)
        self.assertEqual(wire["body"]["data"]["source"], "ui")
        adapter.validate_envelope(wire["body"]["data"])


class ConfigWriterTests(unittest.TestCase):
    def test_config_matches_example_shape(self):
        with tempfile.TemporaryDirectory() as tmp:
            config = adapter.write_config(
                Path(tmp) / "jarvis-core.toml", Path(tmp) / "data", enabled=True
            )
            text = config.read_text(encoding="utf-8")
        self.assertIn("enabled = true", text)
        self.assertIn("shadow_mode = true", text)
        self.assertIn("per_source_burst = 32", text)
        self.assertIn("max_due_per_tick = 8", text)


if __name__ == "__main__":
    unittest.main()
