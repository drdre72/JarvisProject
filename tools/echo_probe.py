"""Manual probe: does the mic hear Jarvis's own TTS, and does the
deterministic playback gate suppress it?

Run from the repo root with the venv python:
    .venv\\Scripts\\python.exe tools\\echo_probe.py

Phases:
  1. baseline  ~4s of quiet to calibrate what "no speech" looks like
  2. playback  Jarvis speaks a ~6s sentence while the listener captures
  3. control   you speak (or stay quiet) to prove non-echo audio flows

Every captured segment is printed with its playback-overlap fraction and
whether the gate suppressed it. Verdict at the end.
"""

import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))

import ambient_listener
import speaker

segments = []


def on_segment(seg):
    segments.append((time.monotonic(), seg))
    tag = "SUPPRESSED(echo)" if seg.suppressed else "heard"
    print(
        f"  segment: {seg.seconds:4.1f}s rms={seg.rms:7.1f} "
        f"overlap={seg.playback_overlap:.2f} -> {tag}"
    )


def main():
    listener = ambient_listener.AmbientListener(on_segment).start()
    try:
        print("baseline: stay quiet for ~4s...")
        time.sleep(4)
        print("Jarvis speaking now (watch for suppressed segments):")
        speaker.speak(
            "This is JARVIS testing whether my own speech "
            "reaches the microphone as user input."
        )
        time.sleep(1.2)  # capture the echo tail after playback ends
        print("your turn: say something in the next 6s (optional)")
        time.sleep(6)
    finally:
        listener.stop()

    suppressed = [s for _, s in segments if s.suppressed]
    heard = [s for _, s in segments if not s.suppressed]
    print(
        f"\nsegments={len(segments)} suppressed={len(suppressed)} "
        f"heard={len(heard)}"
    )
    if suppressed:
        print(
            "self-hearing DETECTED and suppressed: TTS reached the mic "
            "during playback and could not become a command"
        )
    elif not segments:
        print("mic heard nothing at all — check the input device")
    else:
        print(
            "no playback overlap captured — TTS may not reach this mic, "
            "or playback was too quiet"
        )


if __name__ == "__main__":
    main()
