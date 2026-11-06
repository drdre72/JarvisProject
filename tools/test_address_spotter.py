"""Unit tests for the ambient address spotter + the speech-name encoding.

The encoded rule under test: Jarvis's TTS can never produce the word
"jarvis", so a spotted address is provably human. The spotter transcribes
heard segments solely to find the wake word; transcripts are discarded;
only the post-name remainder leaves the module via the callback.
"""

import re
import sys
import threading
import time
import unittest
from pathlib import Path
from unittest.mock import Mock, patch

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
sys.path.insert(0, str(Path(__file__).resolve().parent))

import address_spotter
import speaker


class SanitizeForSpeechTests(unittest.TestCase):
    def test_name_never_reaches_audio(self):
        samples = [
            "Good day, sir. I am JARVIS, Just a Rather Very Intelligent System. Ready when you are.",
            "jarvis",
            "Jarvis here, sir.",
            "As JARVIS, I cannot do that.",
            "J.A.R.V.I.S. mode enabled.",
        ]
        for text in samples:
            out = speaker.sanitize_for_speech(text)
            self.assertIsNone(
                re.search(r"\bjarvis\b", out, re.IGNORECASE),
                f"wake word survived sanitization: {out!r}",
            )

    def test_expansion_replaces_name(self):
        out = speaker.sanitize_for_speech("I am JARVIS.")
        self.assertIn("Just a Rather Very Intelligent System", out)

    def test_unrelated_text_unchanged(self):
        self.assertEqual(
            speaker.sanitize_for_speech("Awaiting orders, sir."),
            "Awaiting orders, sir.",
        )

    def test_speak_sanitizes_before_filename_lookup(self):
        # The soundboard lookup happens on sanitized text, so a cached
        # clip named after "jarvis..." can never replay the name.
        with patch.object(speaker.pyttsx3, "init") as init, \
             patch.object(speaker.os.path, "exists", return_value=False):
            engine = Mock()
            init.return_value = engine
            speaker.speak("I am JARVIS, at your service.")
        spoken = engine.say.call_args[0][0]
        self.assertIsNone(re.search(r"\bjarvis\b", spoken, re.IGNORECASE))

    def test_stop_speaking_is_safe_when_idle(self):
        speaker._active_engine = None
        speaker.stop_speaking()  # must not raise


class ExtractAddressTests(unittest.TestCase):
    def test_leading_wake_word(self):
        self.assertEqual(
            address_spotter.extract_address("jarvis what time is it"),
            "what time is it",
        )

    def test_name_only_means_called(self):
        self.assertEqual(address_spotter.extract_address("jarvis"), "")

    def test_mid_sentence_address(self):
        self.assertEqual(
            address_spotter.extract_address("hey jarvis lights on"),
            "lights on",
        )

    def test_punctuation_is_ignored(self):
        self.assertEqual(
            address_spotter.extract_address("Jarvis, what's the time?"),
            "what's the time",
        )

    def test_no_wake_word_is_unaddressed(self):
        self.assertIsNone(address_spotter.extract_address("what time is it"))

    def test_empty_is_unaddressed(self):
        self.assertIsNone(address_spotter.extract_address(""))
        self.assertIsNone(address_spotter.extract_address(None))


class FakeEngine:
    def __init__(self, text):
        self.text = text
        self.calls = []

    def transcribe_pcm16(self, pcm, beam_size=1):
        self.calls.append((bytes(pcm), beam_size))
        return self.text


def _seg(seconds=1.0, pcm=b"\x01" * 3200):
    return Mock(pcm=pcm, seconds=seconds)


def _drain(pred, timeout=3.0):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if pred():
            return True
        time.sleep(0.02)
    return False


class SpotterWorkerTests(unittest.TestCase):
    def test_addressed_segment_fires_callback_with_remainder(self):
        engine = FakeEngine("jarvis what time is it")
        got = []
        spotter = address_spotter.AddressSpotter(
            lambda: engine, lambda seg, r: got.append(r)
        )
        spotter.start()
        try:
            self.assertTrue(spotter.submit(_seg()))
            self.assertTrue(_drain(lambda: got))
        finally:
            spotter.stop()
        self.assertEqual(got, ["what time is it"])
        self.assertEqual(spotter.transcribed_segments, 1)

    def test_unaddressed_segment_does_not_fire(self):
        engine = FakeEngine("just some room chatter")
        got = []
        spotter = address_spotter.AddressSpotter(
            lambda: engine, lambda seg, r: got.append(r)
        )
        spotter.start()
        try:
            spotter.submit(_seg())
            _drain(lambda: spotter.transcribed_segments == 1)
        finally:
            spotter.stop()
        self.assertEqual(got, [])
        self.assertEqual(spotter.transcribed_segments, 1)

    def test_busy_worker_drops_and_counts(self):
        gate = threading.Event()

        class SlowEngine:
            def transcribe_pcm16(self, pcm, beam_size=1):
                gate.wait(2.0)
                return "jarvis"

        spotter = address_spotter.AddressSpotter(lambda: SlowEngine(), Mock())
        spotter.start()
        try:
            self.assertTrue(spotter.submit(_seg()))
            time.sleep(0.1)  # worker picks item, blocks in transcribe
            self.assertTrue(spotter.submit(_seg()))  # fills depth-1 queue
            self.assertFalse(spotter.submit(_seg()))  # dropped
        finally:
            gate.set()
            spotter.stop()
        self.assertEqual(spotter.dropped_segments, 1)

    def test_callback_failure_is_isolated(self):
        engine = FakeEngine("jarvis")
        fired = []

        def bad(seg, r):
            fired.append(r)
            raise RuntimeError("callback blew up")

        spotter = address_spotter.AddressSpotter(lambda: engine, bad)
        spotter.start()
        try:
            spotter.submit(_seg())
            self.assertTrue(_drain(lambda: fired))
            # Worker survives a blown callback and keeps consuming.
            spotter.submit(_seg())
            self.assertTrue(_drain(lambda: spotter.transcribed_segments >= 2))
        finally:
            spotter.stop()
        self.assertEqual(fired, [""] * len(fired))
        self.assertGreaterEqual(len(fired), 1)

    def test_stop_is_idempotent_and_safe(self):
        spotter = address_spotter.AddressSpotter(lambda: FakeEngine(""), Mock())
        spotter.start()
        spotter.stop()
        self.assertFalse(spotter.submit(_seg()))


class PcmTranscribeTests(unittest.TestCase):
    def _stt(self, text="hello there"):
        from speech_recognition import WhisperSTT

        stt = object.__new__(WhisperSTT)
        stt.language = "en"
        stt._transcribe_lock = threading.Lock()
        stt.model = Mock()
        seg = Mock()
        seg.text = text
        stt.model.transcribe.return_value = ([seg], Mock())
        return stt

    def test_pcm16_feeds_numpy_array(self):
        stt = self._stt()
        out = stt.transcribe_pcm16(
            np.full(1600, 100, dtype=np.int16).tobytes(), beam_size=1
        )
        self.assertEqual(out, "hello there")
        args, kwargs = stt.model.transcribe.call_args
        self.assertIsInstance(args[0], np.ndarray)
        self.assertEqual(kwargs["beam_size"], 1)

    def test_empty_pcm_short_circuits(self):
        stt = self._stt()
        self.assertEqual(stt.transcribe_pcm16(b""), "")
        stt.model.transcribe.assert_not_called()


class AppWiringTests(unittest.TestCase):
    def _app(self):
        import jarvis_app

        app = object.__new__(jarvis_app.JarvisApp)
        app.rust_feed = None
        app._spotter = Mock()
        app.add_chat_message = Mock()
        app.page = Mock()
        app._cleaned_up = False
        return app

    def _seg_mock(self, suppressed):
        return Mock(
            pcm=b"\x01" * 3200,
            seconds=1.0,
            rms=500.0,
            clipped_fraction=0.0,
            playback_overlap=0.9 if suppressed else 0.0,
            suppressed=suppressed,
        )

    def test_heard_segment_is_offered_to_spotter(self):
        app = self._app()
        seg = self._seg_mock(suppressed=False)
        app._on_ambient_segment(seg)
        app._spotter.submit.assert_called_once_with(seg)

    def test_suppressed_segment_never_reaches_spotter(self):
        app = self._app()
        seg = self._seg_mock(suppressed=True)
        app._on_ambient_segment(seg)
        app._spotter.submit.assert_not_called()

    def test_engage_starts_voice_and_dispatches_remainder(self):
        app = self._app()
        app.is_listening = False
        app._voice_generation = 3
        app.voice_button = None
        app.start_voice_recognition = Mock()
        app.process_user_command = Mock()
        app._engage_from_ambient("what time is it")
        app.start_voice_recognition.assert_called_once()
        app.add_chat_message.assert_any_call("User", "what time is it")
        app.process_user_command.assert_called_once_with(
            "what time is it", voice_generation=3
        )

    def test_name_only_engage_dispatches_nothing(self):
        app = self._app()
        app.is_listening = False
        app._voice_generation = 1
        app.voice_button = None
        app.start_voice_recognition = Mock()
        app.process_user_command = Mock()
        app._engage_from_ambient("")
        app.start_voice_recognition.assert_called_once()
        app.process_user_command.assert_not_called()

    def test_engage_skipped_when_already_listening(self):
        app = self._app()
        app.is_listening = True
        app.start_voice_recognition = Mock()
        app._engage_from_ambient("hello")
        app.start_voice_recognition.assert_not_called()

    def test_cleanup_stops_spotter(self):
        app = self._app()
        app.should_monitor = True
        app.is_listening = False
        app._voice_generation = 0
        app.ambient = None
        app.cleanup()
        app._spotter.stop.assert_called_once()


if __name__ == "__main__":
    unittest.main()
