"""
ambient_listener.py — continuous local hearing for JARVIS.

One persistent input stream segments incoming audio by energy (the same
provisional threshold used by the turn capture path). Every completed
segment carries deterministic attribution: how much of it overlapped
Jarvis's own playback, per speaker.playback_state() markers. Segments
that overlap playback are suppressed — they can never reach
transcription or commands, so Jarvis cannot hear himself as the user.

Determinism boundary: this gate uses *our own* playback state, not an
acoustic model. A user who starts speaking during playback is also
suppressed — accepted and documented; real barge-in needs a separate
interrupt mechanism later.

Audio is held in memory only for the duration of a segment and is never
written to disk. Nothing here decides whether ambient speech deserves a
response — that judgment stays outside this module.
"""

import threading
import time
from collections import deque
from dataclasses import dataclass, field
from math import ceil

import pyaudio

from runtime_log import get_runtime_logger
from speaker import playback_state
from speech_recognition import pcm16_metrics


# Suppression tail: room reverb and SAPI/driver release mean playback
# keeps contaminating the mic briefly after speak() returns.
ECHO_TAIL_SECONDS = 0.6
# A segment is suppressed when it begins inside playback or the echo
# tail, or when more than this fraction of it overlaps playback.
MAX_ECHO_OVERLAP = 0.5


@dataclass(frozen=True)
class AmbientSegment:
    """One energy-bounded window of captured audio.

    pcm is raw int16 16kHz mono; it exists only for the lifetime of the
    segment object — nothing on disk. playback_overlap is the fraction of
    the segment captured while Jarvis was speaking.
    """

    pcm: bytes
    seconds: float
    rms: float
    clipped_fraction: float
    playback_overlap: float
    suppressed: bool
    suppress_reason: str


@dataclass
class _SegAcc:
    frames: list = field(default_factory=list)
    echo_chunks: int = 0
    silent_run: int = 0
    started_in_echo: bool = False

    def add(self, data, echoing):
        self.frames.append(data)
        self.echo_chunks += 1 if echoing else 0
        return len(self.frames)


class AmbientListener:
    """Persistent mic capture that emits segmented audio windows.

    start() spawns a daemon thread; stop() ends it. The stream is
    pyaudio shared-mode input — the existing per-turn capture can still
    open its own stream on drivers that allow shared capture.
    """

    def __init__(
        self,
        on_segment,
        sample_rate=16000,
        chunk_size=1024,
        silence_threshold=25.0,
        trailing_silence_seconds=1.2,
        max_segment_seconds=15.0,
        echo_tail_seconds=ECHO_TAIL_SECONDS,
        clock=time.monotonic,
    ):
        if silence_threshold < 0 or trailing_silence_seconds <= 0:
            raise ValueError("thresholds must be non-negative")
        self._on_segment = on_segment
        self.sample_rate = sample_rate
        self.chunk_size = chunk_size
        self.silence_threshold = silence_threshold
        self.echo_tail_seconds = echo_tail_seconds
        self._clock = clock
        self._pause_chunks = max(
            1, ceil(sample_rate * trailing_silence_seconds / chunk_size)
        )
        self._max_chunks = max(
            1, ceil(sample_rate * max_segment_seconds / chunk_size)
        )
        self._stop = threading.Event()
        self._thread = None
        self._acc = None
        self._pre_roll = deque(maxlen=3)
        self.segments_emitted = 0
        self.segments_suppressed = 0

    def _in_echo_window(self):
        speaking, tail = playback_state()
        if speaking:
            return True
        return tail is not None and tail < self.echo_tail_seconds

    def _emit(self, acc):
        recorded = b"".join(acc.frames)
        rms, clipped = pcm16_metrics(recorded)
        overlap = acc.echo_chunks / max(1, len(acc.frames))
        if acc.started_in_echo or overlap > MAX_ECHO_OVERLAP:
            suppressed, reason = True, "playback_overlap"
        else:
            suppressed, reason = False, ""
        seg = AmbientSegment(
            pcm=recorded,
            seconds=len(recorded) / 2 / self.sample_rate,
            rms=rms,
            clipped_fraction=clipped,
            playback_overlap=overlap,
            suppressed=suppressed,
            suppress_reason=reason,
        )
        self.segments_emitted += 1
        self.segments_suppressed += 1 if suppressed else 0
        try:
            self._on_segment(seg)
        except Exception as error:  # a consumer fault must not kill hearing
            get_runtime_logger().warning(
                "ambient segment consumer failed type=%s", type(error).__name__
            )

    def _feed_chunk(self, data):
        """One captured frame: segment it by energy and emit when the
        current window closes. Pure logic — the stream loop stays thin."""
        rms, _ = pcm16_metrics(data)
        echoing = self._in_echo_window()
        if self._acc is None:
            if rms > self.silence_threshold:
                self._acc = _SegAcc(started_in_echo=echoing)
                for frame in self._pre_roll:
                    self._acc.add(frame, echoing)
                self._acc.add(data, echoing)
                self._pre_roll.clear()
            else:
                self._pre_roll.append(data)
            return
        self._acc.add(data, echoing)
        if rms > self.silence_threshold:
            self._acc.silent_run = 0
        else:
            self._acc.silent_run += 1
        if (
            self._acc.silent_run >= self._pause_chunks
            or len(self._acc.frames) >= self._max_chunks
        ):
            self._emit(self._acc)
            self._acc = None
            self._pre_roll.clear()

    def _run(self):
        audio = pyaudio.PyAudio()
        stream = None
        try:
            stream = audio.open(
                format=pyaudio.paInt16,
                channels=1,
                rate=self.sample_rate,
                input=True,
                frames_per_buffer=self.chunk_size,
            )
            while not self._stop.is_set():
                self._feed_chunk(stream.read(self.chunk_size))
            if self._acc is not None:
                self._emit(self._acc)
        except Exception as error:
            get_runtime_logger().warning(
                "ambient listener stopped type=%s", type(error).__name__
            )
        finally:
            if stream is not None:
                try:
                    stream.stop_stream()
                    stream.close()
                except Exception:
                    pass
            audio.terminate()
            # Self-clear so a dead listener can be restarted by start().
            self._thread = None

    def alive(self):
        return self._thread is not None and self._thread.is_alive()

    def start(self):
        if self.alive():
            return self
        self._stop.clear()
        self._thread = threading.Thread(
            target=self._run, name="ambient-listener", daemon=True
        )
        self._thread.start()
        return self

    def stop(self, timeout=3.0):
        self._stop.set()
        if self._thread is not None:
            self._thread.join(timeout)
            self._thread = None
