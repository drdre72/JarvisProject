"""Unit tests for ambient hearing + deterministic playback attribution.

No real audio device: _feed_chunk is driven with synthetic PCM16 frames
and speaker.playback_state is patched. The rules under test:

  - a segment that STARTS during playback (or the echo tail) is
    suppressed — we cannot deterministically tell Jarvis's voice tail
    from a user's voice start, so the safe side wins;
  - a segment overlapping playback more than MAX_ECHO_OVERLAP is
    suppressed;
  - everything else is heard and left for a later judgment layer.
"""

import sys
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import ambient_listener
import speaker


def loud(n=1024):
    return np.full(n, 20000, dtype=np.int16).tobytes()


def quiet(n=1024):
    return np.zeros(n, dtype=np.int16).tobytes()


def make_listener(segments, silence_seconds=0.05):
    listener = ambient_listener.AmbientListener(
        segments.append, trailing_silence_seconds=silence_seconds
    )
    return listener


class EchoWindowTests(unittest.TestCase):
    def _window(self, state):
        with patch.object(ambient_listener, "playback_state", return_value=state):
            listener = ambient_listener.AmbientListener(Mock())
            return listener._in_echo_window()

    def test_speaking_is_echo(self):
        self.assertTrue(self._window((True, 0.0)))

    def test_short_tail_is_echo(self):
        self.assertTrue(self._window((False, 0.3)))

    def test_long_tail_is_not_echo(self):
        self.assertFalse(self._window((False, 5.0)))

    def test_never_spoke_is_not_echo(self):
        self.assertFalse(self._window((False, None)))


class SpeakerMarkerTests(unittest.TestCase):
    def test_markers_track_playback(self):
        speaker._mark_playback_start()
        speaking, _ = speaker.playback_state()
        self.assertTrue(speaking)
        speaker._mark_playback_end()
        speaking, tail = speaker.playback_state()
        self.assertFalse(speaking)
        self.assertIsNotNone(tail)

    def test_empty_text_never_marks(self):
        speaker._mark_playback_end()  # ensure baseline state
        speaker._playback_active.clear()
        speaker.speak("")
        speaking, _ = speaker.playback_state()
        self.assertFalse(speaking)


class SegmentationTests(unittest.TestCase):
    def test_speech_then_silence_emits_one_segment(self):
        segments = []
        listener = make_listener(segments)
        with patch.object(
            ambient_listener, "playback_state", return_value=(False, None)
        ):
            for _ in range(3):
                listener._feed_chunk(quiet())
            for _ in range(4):
                listener._feed_chunk(loud())
            for _ in range(6):
                listener._feed_chunk(quiet())
        self.assertEqual(len(segments), 1)
        seg = segments[0]
        self.assertFalse(seg.suppressed)
        self.assertEqual(seg.playback_overlap, 0.0)
        self.assertGreater(seg.seconds, 0.2)
        self.assertGreater(seg.rms, 1000)

    def test_no_activity_emits_nothing(self):
        segments = []
        listener = make_listener(segments)
        with patch.object(
            ambient_listener, "playback_state", return_value=(False, None)
        ):
            for _ in range(10):
                listener._feed_chunk(quiet())
        self.assertEqual(segments, [])

    def test_consumer_failure_is_isolated(self):
        bad = Mock(side_effect=RuntimeError("consumer blew up"))
        listener = ambient_listener.AmbientListener(
            bad, trailing_silence_seconds=0.05
        )
        acc = ambient_listener._SegAcc()
        acc.add(loud(), False)
        listener._emit(acc)  # must not raise
        self.assertEqual(listener.segments_emitted, 1)
        bad.assert_called_once()


class SuppressionTests(unittest.TestCase):
    def _run_chunks(self, states):
        """Feed loud+silence chunks while playback_state follows `states`
        per call; returns the emitted segment."""
        segments = []
        listener = make_listener(segments)
        state_iter = iter(states)
        with patch.object(
            ambient_listener,
            "playback_state",
            side_effect=lambda: next(state_iter),
        ):
            for _ in range(len(states)):
                listener._feed_chunk(loud())
            with patch.object(
                ambient_listener, "playback_state", return_value=(False, None)
            ):
                for _ in range(6):
                    listener._feed_chunk(quiet())
        self.assertEqual(len(segments), 1)
        return segments[0]

    def test_segment_starting_in_playback_is_suppressed(self):
        seg = self._run_chunks([(True, 0.0), (False, 5.0), (False, 5.0), (False, 5.0)])
        self.assertTrue(seg.suppressed)
        self.assertEqual(seg.suppress_reason, "playback_overlap")

    def test_majority_overlap_is_suppressed(self):
        seg = self._run_chunks(
            [(False, None), (True, 0.0), (True, 0.0), (True, 0.0)]
        )
        self.assertTrue(seg.suppressed)
        self.assertGreater(seg.playback_overlap, 0.5)

    def test_minority_overlap_is_heard(self):
        seg = self._run_chunks(
            [(False, None), (False, None), (False, None), (True, 0.0)]
        )
        self.assertFalse(seg.suppressed)
        self.assertLessEqual(seg.playback_overlap, 0.5)

    def test_no_overlap_is_heard(self):
        seg = self._run_chunks([(False, None)] * 4)
        self.assertFalse(seg.suppressed)
        self.assertEqual(seg.playback_overlap, 0.0)


class LifecycleTests(unittest.TestCase):
    def test_alive_before_start_is_false(self):
        listener = ambient_listener.AmbientListener(Mock())
        self.assertFalse(listener.alive())

    def test_stop_unstarted_is_safe(self):
        ambient_listener.AmbientListener(Mock()).stop()


class JarvisAppWiringTests(unittest.TestCase):
    def _app(self):
        import jarvis_app

        app = object.__new__(jarvis_app.JarvisApp)
        app.rust_feed = None
        app.add_chat_message = Mock()
        app.set_activity = Mock()
        return app

    def test_suspend_and_resume_cycle(self):
        app = self._app()
        ambient = Mock(spec=["stop", "alive"])
        app.ambient = ambient
        app._ambient_suspend()
        ambient.stop.assert_called_once()
        self.assertIsNone(app.ambient)
        with patch.object(
            ambient_listener, "AmbientListener"
        ) as cls:
            cls.return_value.start.return_value = "listener"
            app._ambient_resume()
        self.assertEqual(app.ambient, "listener")

    def test_dead_listener_is_restarted(self):
        app = self._app()
        app.ambient = Mock(spec=["stop", "alive", "start"])
        app.ambient.alive.return_value = False
        app.ambient.start.return_value = "restarted"
        app._ambient_resume()
        self.assertEqual(app.ambient, "restarted")

    def test_cleanup_stops_ambient(self):
        app = self._app()
        app._cleaned_up = False
        app.should_monitor = True
        app.is_listening = False
        app._voice_generation = 0
        ambient = Mock(spec=["stop", "alive"])
        app.ambient = ambient
        app.cleanup()
        ambient.stop.assert_called_once()

    def test_segment_consumer_logs_only(self):
        app = self._app()
        seg = Mock(
            seconds=1.0, rms=500.0, clipped_fraction=0.0,
            playback_overlap=0.9, suppressed=True,
        )
        app._on_ambient_segment(seg)  # must not raise, no dispatch


if __name__ == "__main__":
    unittest.main()
