"""
address_spotter.py — wake-address detection over ambient "heard" segments.

The deterministic contract:
- Only non-suppressed ambient segments are ever submitted here.
- A segment is "addressed" iff its transcript contains the wake word.
  Jarvis's own TTS can never produce the word "jarvis" (see
  speaker.sanitize_for_speech), so a spotted address is provably human.
- Transcripts are used only for this check and then discarded — ambient
  audio must not become commands, memory, or logs. Only the detected
  remainder (text after the wake word) leaves this module, and only via
  the addressed callback, which enters the normal typed-intent pipeline.
- Depth-1 drop policy: a segment arriving while the worker is busy is
  dropped and counted. A missed summon is recoverable; a stalled mic
  capture loop is not.
"""

import queue
import re
import threading

from runtime_log import get_runtime_logger

WAKE_WORD = "jarvis"
_TOKEN_PATTERN = re.compile(r"[a-z0-9']+")
_SPOT_BEAM_SIZE = 1


def extract_address(text):
    """
    Return the text after the first wake-word occurrence, or None if the
    transcript does not address Jarvis. Empty string means he was called
    by name only.
    """
    if not text:
        return None
    tokens = _TOKEN_PATTERN.findall(text.lower())
    if WAKE_WORD not in tokens:
        return None
    idx = tokens.index(WAKE_WORD)
    return " ".join(tokens[idx + 1:])


class AddressSpotter:
    """Single-worker queue: transcribe heard segments, fire on_address
    only when the wake word is present."""

    def __init__(self, engine_getter, on_address):
        """
        Args:
            engine_getter: callable returning the shared WhisperSTT engine
                (lazy — the model loads on first ambient speech, not at
                app startup).
            on_address: callback(segment, remainder_text). Runs on the
                spotter thread; must be fast or re-dispatch itself.
        """
        self._engine_getter = engine_getter
        self._on_address = on_address
        self._queue = queue.Queue(maxsize=1)
        self._stop = threading.Event()
        self._thread = None
        self.dropped_segments = 0
        self.transcribed_segments = 0

    def start(self):
        if self._thread is not None and self._thread.is_alive():
            return
        self._stop.clear()
        self._thread = threading.Thread(
            target=self._run, name="jarvis-address-spotter", daemon=True
        )
        self._thread.start()

    def submit(self, segment):
        """Offer a heard segment for spotting. Returns False if dropped."""
        if self._stop.is_set():
            return False
        try:
            self._queue.put_nowait(segment)
            return True
        except queue.Full:
            self.dropped_segments += 1
            return False

    def stop(self):
        self._stop.set()
        try:
            self._queue.put_nowait(None)
        except queue.Full:
            pass
        if self._thread is not None:
            self._thread.join(timeout=2.0)

    def _run(self):
        while True:
            try:
                segment = self._queue.get(timeout=0.5)
            except queue.Empty:
                if self._stop.is_set():
                    return
                continue
            if segment is None or self._stop.is_set():
                return
            try:
                engine = self._engine_getter()
                text = engine.transcribe_pcm16(
                    segment.pcm, beam_size=_SPOT_BEAM_SIZE
                )
                self.transcribed_segments += 1
                remainder = extract_address(text)
                if remainder is None:
                    get_runtime_logger().debug(
                        "ambient heard segment not addressed seconds=%.2f",
                        segment.seconds,
                    )
                    continue
                get_runtime_logger().info(
                    "ambient address detected seconds=%.2f has_remainder=%s",
                    segment.seconds,
                    bool(remainder),
                )
                self._on_address(segment, remainder)
            except Exception as e:
                get_runtime_logger().warning(
                    "address spotter failed type=%s", type(e).__name__
                )
