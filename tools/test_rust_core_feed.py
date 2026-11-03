"""Stage A shadow-feed checks: activation gates, failure isolation, and
the jarvis_app turn wiring. No binary, vault, or transcript is touched.
"""

import sys
import tempfile
import unittest
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


if __name__ == "__main__":
    unittest.main()
