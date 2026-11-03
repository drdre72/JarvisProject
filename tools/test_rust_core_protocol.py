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
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import rust_core_adapter as adapter


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


if __name__ == "__main__":
    unittest.main()
