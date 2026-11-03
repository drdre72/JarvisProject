"""Stage A shadow-feed checks: activation gates, failure isolation, and
the jarvis_app turn wiring. No binary, vault, or transcript is touched.
"""

import json
import sys
import tempfile
import unittest
from datetime import datetime, timezone
from pathlib import Path
from unittest.mock import Mock, patch

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

import rust_core_adapter as adapter
import rust_core_feed


class FakeProcess:
    def __init__(self, fail_on_send=False, running=True):
        self.sent = []
        self.fail_on_send = fail_on_send
        self.running = running
        self.stopped = False
        self.pid = 4242

    def start(self):
        return self

    def send_event(self, *args, **kwargs):
        if self.fail_on_send:
            raise RuntimeError("broken pipe")
        self.sent.append((args, kwargs))

    def is_running(self):
        return self.running

    def stop(self, timeout=5.0):
        self.stopped = True


class RequestingProcess(FakeProcess):
    """Fake that answers IPC requests with a queued response shape."""

    def __init__(self, response=None, **kwargs):
        super().__init__(**kwargs)
        self.response = response or {"ok": True, "body": {}}
        self.requests = []

    def request(self, op, args=None, request_id=None):
        self.requests.append((op, args))
        return "req-1"

    def await_response(self, request_id, timeout=5.0):
        return self.response


class ActivationGateTests(unittest.TestCase):
    def test_inactive_when_config_missing_or_disabled(self):
        with tempfile.TemporaryDirectory() as tmp:
            self.assertIsNone(rust_core_feed.start_if_enabled(root=tmp))
            adapter.write_config(Path(tmp) / "jarvis-core.toml", tmp, enabled=False)
            self.assertIsNone(rust_core_feed.start_if_enabled(root=tmp))

    def test_inactive_when_config_malformed(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "jarvis-core.toml").write_text("[rust_core\n", "utf-8")
            self.assertIsNone(rust_core_feed.start_if_enabled(root=tmp))

    def test_inactive_when_enabled_but_no_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            adapter.write_config(Path(tmp) / "jarvis-core.toml", tmp, enabled=True)
            self.assertIsNone(rust_core_feed.start_if_enabled(root=tmp))

    def test_active_when_enabled_with_binary(self):
        with tempfile.TemporaryDirectory() as tmp:
            adapter.write_config(Path(tmp) / "jarvis-core.toml", tmp, enabled=True)
            fake = FakeProcess()
            with patch.object(
                rust_core_feed.adapter, "RustCoreProcess", return_value=fake
            ):
                feed = rust_core_feed.start_if_enabled(root=tmp, binary="x/bin")
            self.assertIsInstance(feed, rust_core_feed.RustCoreFeed)


class EmitBehaviorTests(unittest.TestCase):
    def test_emit_forwards_to_process(self):
        proc = FakeProcess()
        feed = rust_core_feed.RustCoreFeed(proc)
        trace = feed.new_trace()
        self.assertTrue(
            feed.emit("ui", "UserRequestReceived", {"input": "text"}, trace_id=trace)
        )
        self.assertEqual(len(proc.sent), 1)
        args, kwargs = proc.sent[0]
        self.assertEqual(args[:2], ("ui", "UserRequestReceived"))
        self.assertEqual(kwargs["trace_id"], trace)

    def test_send_failure_never_raises_and_dead_feed_stops(self):
        proc = FakeProcess(fail_on_send=True, running=False)
        feed = rust_core_feed.RustCoreFeed(proc)
        self.assertFalse(feed.emit("ui", "UserRequestReceived"))
        self.assertTrue(feed.dead)
        self.assertFalse(feed.emit("ui", "UserRequestReceived"))

    def test_live_process_tolerates_bounded_failures(self):
        proc = FakeProcess(fail_on_send=True, running=True)
        feed = rust_core_feed.RustCoreFeed(proc)
        for _ in range(rust_core_feed._MAX_SEND_FAILURES):
            feed.emit("ui", "UserRequestReceived")
        self.assertTrue(feed.dead)

    def test_close_stops_process(self):
        proc = FakeProcess()
        rust_core_feed.RustCoreFeed(proc).close()
        self.assertTrue(proc.stopped)


class SupervisorLifecycleTests(unittest.TestCase):
    """Pid-file bookkeeping: reap orphans from crashed runs, clean up ours."""

    def _enabled_root(self, tmp):
        adapter.write_config(Path(tmp) / "jarvis-core.toml", tmp, enabled=True)

    def test_start_writes_pid_file_and_close_removes_it(self):
        with tempfile.TemporaryDirectory() as tmp:
            self._enabled_root(tmp)
            with patch.object(
                rust_core_feed.adapter, "RustCoreProcess", return_value=FakeProcess()
            ):
                feed = rust_core_feed.start_if_enabled(root=tmp, binary="x/bin")
            pid_path = Path(tmp) / "feed.pid"  # test config uses tmp as data_dir
            self.assertEqual(pid_path.read_text("utf-8"), "4242")
            feed.close()
            self.assertFalse(pid_path.exists())

    def test_stale_supervisor_reaped_before_spawn(self):
        with tempfile.TemporaryDirectory() as tmp:
            self._enabled_root(tmp)
            (Path(tmp) / "feed.pid").write_text("4242", "utf-8")
            with patch.object(
                rust_core_feed, "_supervisor_alive", return_value=True
            ), patch.object(rust_core_feed.subprocess, "run") as run, patch.object(
                rust_core_feed.adapter, "RustCoreProcess", return_value=FakeProcess()
            ):
                feed = rust_core_feed.start_if_enabled(root=tmp, binary="x/bin")
            run.assert_called_once()
            self.assertEqual(run.call_args.args[0], ["taskkill", "/PID", "4242", "/F"])
            self.assertIsNotNone(feed)
            feed.close()

    def test_dead_or_foreign_pid_is_not_killed(self):
        with tempfile.TemporaryDirectory() as tmp:
            self._enabled_root(tmp)
            (Path(tmp) / "feed.pid").write_text("4242", "utf-8")
            with patch.object(
                rust_core_feed, "_supervisor_alive", return_value=False
            ), patch.object(rust_core_feed.subprocess, "run") as run, patch.object(
                rust_core_feed.adapter, "RustCoreProcess", return_value=FakeProcess()
            ):
                feed = rust_core_feed.start_if_enabled(root=tmp, binary="x/bin")
            run.assert_not_called()
            feed.close()

    def test_corrupt_pid_file_ignored(self):
        with tempfile.TemporaryDirectory() as tmp:
            self._enabled_root(tmp)
            (Path(tmp) / "feed.pid").write_text("not-a-pid", "utf-8")
            with patch.object(
                rust_core_feed, "_supervisor_alive"
            ) as alive, patch.object(
                rust_core_feed.adapter, "RustCoreProcess", return_value=FakeProcess()
            ):
                feed = rust_core_feed.start_if_enabled(root=tmp, binary="x/bin")
            alive.assert_not_called()
            feed.close()


class ObligationRequestTests(unittest.TestCase):
    """Stage B surface: request passthrough and idempotent registration."""

    def test_request_returns_response_data(self):
        proc = RequestingProcess(response={"ok": True, "body": {"obligation_id": "x"}})
        feed = rust_core_feed.RustCoreFeed(proc)
        data = feed.request("schedule_obligation", {"obligation_id": "x"})
        self.assertEqual(data["body"]["obligation_id"], "x")
        self.assertEqual(proc.requests[0][0], "schedule_obligation")

    def test_request_failure_is_isolated(self):
        proc = RequestingProcess()
        proc.request = Mock(side_effect=RuntimeError("pipe gone"))
        feed = rust_core_feed.RustCoreFeed(proc)
        self.assertIsNone(feed.request("schedule_obligation", {}))
        self.assertEqual(feed.emit_errors, 1)

    def test_request_on_dead_feed_is_none(self):
        proc = RequestingProcess(running=False)
        feed = rust_core_feed.RustCoreFeed(proc)
        feed.dead = True
        self.assertIsNone(feed.request("cancel_obligation", {}))

    def test_ensure_obligation_mappings(self):
        cases = [
            ({"ok": True, "body": {}}, "armed"),
            ({"ok": False, "body": {"error": "duplicate_obligation"}}, "already_armed"),
            ({"ok": False, "body": {"error": "ipc_disabled"}}, "rejected"),
        ]
        for response, expected in cases:
            feed = rust_core_feed.RustCoreFeed(RequestingProcess(response=response))
            self.assertEqual(feed.ensure_obligation({"obligation_id": "o"}), expected)
        dead = rust_core_feed.RustCoreFeed(RequestingProcess(running=False))
        dead.dead = True
        self.assertIsNone(dead.ensure_obligation({"obligation_id": "o"}))

    def test_review_reminder_shape(self):
        proc = RequestingProcess(response={"ok": True, "body": {}})
        feed = rust_core_feed.RustCoreFeed(proc)
        moment = datetime(2026, 11, 3, 12, 0, tzinfo=timezone.utc)
        result = rust_core_feed.ensure_review_reminder(feed, now=moment)
        self.assertEqual(result, "armed")
        _, args = proc.requests[0]
        self.assertEqual(args["obligation_id"], "obl-review-reminder")
        self.assertEqual(args["scope"], "ops.obligation.run")
        self.assertEqual(args["origin"], "jarvis-app")
        self.assertEqual(args["recurrence_seconds"], 86400)
        self.assertEqual(args["missed_policy"], "run_once_on_recovery")
        self.assertEqual(args["next_run_at_utc"], "2026-11-04T12:00:00Z")
        # No user data rides in the obligation record.
        self.assertNotIn("note", json.dumps(args).lower())
        self.assertNotIn("text", json.dumps(args).lower())


class JarvisAppWiringTests(unittest.TestCase):
    def _app(self, feed):
        import jarvis_app

        app = object.__new__(jarvis_app.JarvisApp)
        app.rust_feed = feed
        app.add_chat_message = Mock()
        app.set_activity = Mock()
        return app

    def test_handled_turn_emits_correlated_pair(self):
        import jarvis_app

        feed = Mock(spec=["emit", "new_trace"])
        feed.new_trace.return_value = "trace-1"
        app = self._app(feed)
        with patch.object(jarvis_app, "speak"):
            app.process_user_command("exit")

        calls = [c.kwargs | {"positional": c.args} for c in feed.emit.call_args_list]
        self.assertEqual(len(calls), 2)
        start, end = calls
        self.assertEqual(start["positional"][:2], ("ui", "UserRequestReceived"))
        self.assertEqual(end["positional"][:2], ("ui", "ExternalStimulus"))
        self.assertEqual(end["positional"][2]["outcome"], "handled")
        self.assertEqual(end["positional"][2]["kind"], "turn_completed")
        # Same trace links request and outcome; no transcript content flows.
        self.assertEqual(start["trace_id"], end["trace_id"])
        for call in feed.emit.call_args_list:
            payload = call.args[2]
            self.assertNotIn("transcript", payload)
            self.assertNotIn("prompt", payload)
            self.assertNotIn("text", payload)

    def test_voice_turn_uses_voice_pipeline_source(self):
        import jarvis_app

        feed = Mock(spec=["emit", "new_trace"])
        feed.new_trace.return_value = "trace-v"
        app = self._app(feed)
        app.is_listening = True
        app._voice_generation = 1
        with patch.object(jarvis_app, "speak"):
            app.process_user_command("exit", voice_generation=1)
        start = feed.emit.call_args_list[0]
        self.assertEqual(start.args[0], "voice_pipeline")
        self.assertEqual(start.args[2]["input"], "voice")

    def test_dispatcher_result_counts_as_handled(self):
        import jarvis_app

        feed = Mock(spec=["emit", "new_trace"])
        feed.new_trace.return_value = "trace-2"
        app = self._app(feed)
        with patch.object(jarvis_app, "process_command", return_value="DONE"):
            with patch.object(jarvis_app, "speak"):
                app.process_user_command("anything")
        end = feed.emit.call_args_list[1]
        self.assertEqual(end.args[2]["outcome"], "handled")

    def test_no_feed_means_no_emission_and_no_crash(self):
        import jarvis_app

        app = self._app(None)
        with patch.object(jarvis_app, "speak"):
            app.process_user_command("exit")  # must not raise

    def test_cleanup_closes_feed_once(self):
        import jarvis_app

        feed = Mock(spec=["close"])
        app = self._app(feed)
        app.cleanup()
        app.cleanup()  # disconnect + exit both call it; feed closes once
        feed.close.assert_called_once()

    def test_register_obligations_delegates_reminder(self):
        import jarvis_app

        feed = Mock(spec=["ensure_obligation"])
        feed.ensure_obligation.return_value = "armed"
        app = self._app(feed)
        app._register_rust_obligations()
        self.assertEqual(feed.ensure_obligation.call_count, 1)

    def test_register_obligations_noop_without_feed(self):
        import jarvis_app

        app = self._app(None)
        app._register_rust_obligations()  # must not raise


if __name__ == "__main__":
    unittest.main()
