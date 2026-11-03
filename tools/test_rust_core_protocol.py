"""Live protocol-compat test: Python adapter <-> real jarvis-core binary.

Stage A acceptance per the Rust directive (Phase Six): a versioned
JSON-lines stdio contract where malformed input is rejected before queue
space, valid events are journaled with their admission decisions, and the
supervisor outlives a closed intake. Skips cleanly when no binary exists.
"""

import json
import os
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import time
import unittest
from datetime import datetime, timedelta, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import rust_core_adapter as adapter
import rust_core_feed
import inspect_rust_journal


def _build_or_find_binary():
    binary = adapter.find_binary(ROOT)
    if binary is not None:
        return binary
    cargo = shutil.which("cargo") or (
        Path.home() / ".cargo" / "bin" / ("cargo.exe" if os.name == "nt" else "cargo")
    )
    if cargo and Path(cargo).exists():
        subprocess.run(
            [str(cargo), "build"], cwd=ROOT / "jarvis-core", timeout=300, check=False
        )
        binary = adapter.find_binary(ROOT)
    return binary


class LiveProtocolTests(unittest.TestCase):
    binary = None

    @classmethod
    def setUpClass(cls):
        cls.binary = _build_or_find_binary()
        if cls.binary is None:
            raise unittest.SkipTest("jarvis-core binary unavailable (cargo build it)")

    def test_disabled_flag_exits_inert(self):
        with tempfile.TemporaryDirectory() as tmp:
            config = adapter.write_config(
                Path(tmp) / "jarvis-core.toml", Path(tmp) / "data", enabled=False
            )
            result = subprocess.run(
                [str(self.binary), "--config", str(config)],
                capture_output=True,
                timeout=15,
            )
        self.assertEqual(result.returncode, 0)
        self.assertFalse((Path(tmp) / "data").exists(), "inert run created no data dir")

    def test_shadow_runtime_accepts_python_frames(self):
        with tempfile.TemporaryDirectory() as tmp:
            data_dir = Path(tmp) / "data"
            config = adapter.write_config(
                Path(tmp) / "jarvis-core.toml", data_dir, enabled=True, shadow_mode=True
            )
            core = adapter.RustCoreProcess(self.binary, config, data_dir=data_dir)
            try:
                core.start()
                event_id = core.send_event(
                    "ui",
                    "UserRequestReceived",
                    payload={"marker": "cobalt-1", "stage": "protocol_compat"},
                )
                # Every rejection path: transport malformed, oversized,
                # bad version, envelope validation, non-event frame kind.
                core.send_raw("{not json")
                core.send_raw("x" * (adapter.MAX_FRAME_BYTES + 1))
                bad_version = adapter.heartbeat_frame("w")
                bad_version["protocol_version"] = 99
                core.send_raw(adapter.encode(bad_version))
                future = adapter.event_frame("ui", "UserRequestReceived")
                future["body"]["data"]["occurred_at_utc"] = "2099-01-01T00:00:00Z"
                core.send_raw(adapter.encode(future))
                core.send_wire_frame(adapter.heartbeat_frame("stt"))

                time.sleep(2.0)  # let at least one scheduler tick drain
                core.close_stdin()
                time.sleep(0.4)
                self.assertTrue(
                    core.is_running(), "supervisor must outlive a closed intake"
                )
            finally:
                core.stop()

            journal = data_dir / "operational-journal.db"
            self.assertTrue(journal.is_file(), "durable journal was not created")
            conn = sqlite3.connect(str(journal))
            try:
                integrity = conn.execute("PRAGMA integrity_check").fetchone()[0]
                events = conn.execute(
                    "SELECT event_id, source, event_type, class, disposition,"
                    "       payload_json FROM events"
                ).fetchall()
                dispatches = conn.execute(
                    "SELECT mode, outcome FROM dispatch_attempts"
                ).fetchall()
            finally:
                conn.close()

        self.assertEqual(integrity, "ok")
        self.assertEqual(len(events), 1, f"only the valid event is journaled: {events}")
        row = events[0]
        self.assertEqual(row[0], event_id)
        self.assertEqual(row[1:5], ("ui", "UserRequestReceived", "user_interrupt", "accepted"))
        self.assertIn("cobalt-1", json.loads(row[5]).get("marker", ""))

        self.assertTrue(dispatches, "attended event produced no dispatch record")
        for mode, outcome in dispatches:
            self.assertEqual(mode, "shadow")
            self.assertTrue(
                outcome.startswith("evaluated"),
                f"shadow stage must not dispatch: {outcome}",
            )

        log_text = "\n".join(core.log_lines)
        self.assertIn("dropped malformed frame", log_text)
        self.assertIn("dropped invalid event", log_text)
        self.assertIn("ignoring non-event frame", log_text)


class StageBWireTests(unittest.TestCase):
    """The transferred function: durable obligation scheduling over IPC."""

    binary = None

    @classmethod
    def setUpClass(cls):
        cls.binary = _build_or_find_binary()
        if cls.binary is None:
            raise unittest.SkipTest("jarvis-core binary unavailable (cargo build it)")

    def _spawn(self, tmp, ipc_requests):
        config = adapter.write_config(
            Path(tmp) / "jarvis-core.toml",
            Path(tmp) / "data",
            enabled=True,
            shadow_mode=True,
            ipc_requests=ipc_requests,
        )
        core = adapter.RustCoreProcess(
            self.binary, config, data_dir=Path(tmp) / "data"
        )
        return core.start(), Path(tmp) / "data" / "operational-journal.db"

    def test_ipc_disabled_flag_rolls_back_over_the_wire(self):
        with tempfile.TemporaryDirectory() as tmp:
            core, _ = self._spawn(tmp, ipc_requests=False)
            try:
                req_id = core.request("schedule_obligation", adapter.obligation(
                    "ob-off", datetime.now(timezone.utc) + timedelta(hours=1),
                    "ops.obligation.run",
                ))
                response = core.await_response(req_id, timeout=5.0)
            finally:
                core.stop()
        self.assertFalse(response["ok"])
        self.assertEqual(response["body"]["error"], "ipc_disabled")

    def test_schedule_and_cancel_over_wire(self):
        with tempfile.TemporaryDirectory() as tmp:
            core, journal = self._spawn(tmp, ipc_requests=True)
            try:
                req_id = core.request(
                    "schedule_obligation",
                    adapter.obligation(
                        "ob-cancel",
                        datetime.now(timezone.utc) + timedelta(hours=1),
                        "ops.obligation.run",
                    ),
                )
                response = core.await_response(req_id, timeout=5.0)
                self.assertTrue(response["ok"], response)
                req_id = core.request("cancel_obligation", {"obligation_id": "ob-cancel"})
                response = core.await_response(req_id, timeout=5.0)
                self.assertTrue(response["ok"], response)
                self.assertEqual(response["body"]["status"], "cancelled")
            finally:
                core.stop()
            conn = sqlite3.connect(str(journal))
            try:
                status = conn.execute(
                    "SELECT status FROM scheduled_obligations"
                    " WHERE obligation_id='ob-cancel'"
                ).fetchone()
            finally:
                conn.close()
        self.assertEqual(status, ("cancelled",))

    def test_scheduled_obligation_shadow_evaluates_through_policy(self):
        with tempfile.TemporaryDirectory() as tmp:
            core, journal = self._spawn(tmp, ipc_requests=True)
            try:
                due = datetime.now(timezone.utc) + timedelta(seconds=2)
                req_id = core.request(
                    "schedule_obligation",
                    adapter.obligation("ob-live", due, "ops.obligation.run"),
                )
                response = core.await_response(req_id, timeout=5.0)
                self.assertTrue(response["ok"], response)
                self.assertEqual(response["body"]["obligation_id"], "ob-live")
                time.sleep(4.0)  # one tick releases it; shadow records the outcome
            finally:
                core.stop()
            conn = sqlite3.connect(str(journal))
            try:
                row = conn.execute(
                    "SELECT status, last_result FROM scheduled_obligations"
                    " WHERE obligation_id='ob-live'"
                ).fetchone()
                intent = conn.execute(
                    "SELECT decision, verb, scope FROM action_intents LIMIT 1"
                ).fetchone()
                dispatch = conn.execute(
                    "SELECT mode, outcome FROM dispatch_attempts LIMIT 1"
                ).fetchone()
            finally:
                conn.close()
        self.assertEqual(row, ("done", "shadow_evaluated"))
        self.assertEqual(intent, ("allow", "run_obligation", "ops.obligation.run"))
        self.assertEqual(dispatch, ("shadow", "shadow_evaluated"))

    def test_unknown_op_gets_typed_error(self):
        with tempfile.TemporaryDirectory() as tmp:
            core, _ = self._spawn(tmp, ipc_requests=True)
            try:
                req_id = core.request("teleport_user", {})
                response = core.await_response(req_id, timeout=5.0)
            finally:
                core.stop()
        self.assertFalse(response["ok"])
        self.assertEqual(response["body"]["error"], "unknown_op")


class StageAFeedTests(unittest.TestCase):
    """The comparison evidence: real turn events journaled by the shadow."""

    binary = None

    @classmethod
    def setUpClass(cls):
        cls.binary = _build_or_find_binary()
        if cls.binary is None:
            raise unittest.SkipTest("jarvis-core binary unavailable (cargo build it)")

    def test_live_feed_records_correlated_turn(self):
        with tempfile.TemporaryDirectory() as tmp:
            adapter.write_config(
                Path(tmp) / "jarvis-core.toml",
                Path(tmp) / "data",
                enabled=True,
                shadow_mode=True,
            )
            feed = rust_core_feed.start_if_enabled(root=tmp, binary=self.binary)
            self.assertIsNotNone(feed, "enabled config + binary must activate the feed")
            try:
                trace = feed.new_trace()
                self.assertTrue(
                    feed.emit(
                        "voice_pipeline",
                        "UserRequestReceived",
                        {"input": "voice"},
                        trace_id=trace,
                    )
                )
                self.assertTrue(
                    feed.emit(
                        "voice_pipeline",
                        "ExternalStimulus",
                        {"kind": "turn_completed", "outcome": "answered",
                         "elapsed_ms": 42},
                        trace_id=trace,
                    )
                )
                time.sleep(2.0)
            finally:
                feed.close()

            journal = Path(tmp) / "data" / "operational-journal.db"
            summary = inspect_rust_journal.summarize(journal)
            traces = summary["traces"]
            self.assertIn(trace, traces, "turn events missing from journal")
            rows = traces[trace]
            self.assertEqual(len(rows), 2)
            by_type = {row[3]: row for row in rows}
            self.assertEqual(by_type["UserRequestReceived"][4], "user_interrupt")
            self.assertEqual(by_type["ExternalStimulus"][4], "external_stimulus")
            # Stage A invariant: everything dispatched is shadow bookkeeping.
            self.assertEqual(inspect_rust_journal.shadow_violations(journal), [])

    def test_feed_inactive_without_config(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertIsNone(rust_core_feed.start_if_enabled(root=tmp))


if __name__ == "__main__":
    unittest.main()
