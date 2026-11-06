# speaker.py
import os
import re
import time
import pygame
import pyttsx3
import threading

# Encoded rule: JARVIS never says his own name aloud — he refers to
# himself only as "Just a Rather Very Intelligent System". Then the word
# "jarvis" is a channel his own output can never occupy: any heard
# "jarvis" is provably a human addressing him, even inside audio that
# overlaps his own playback. Applied inside speak() so no call site can
# regress; on-screen text keeps the name — only the acoustic channel is
# constrained.
_SELF_REFERENCE = "Just a Rather Very Intelligent System"
_NAME_PATTERN = re.compile(r"\bjarvis\b", re.IGNORECASE)


def sanitize_for_speech(text: str) -> str:
    """Rewrite the wake word out of any utterance before it reaches the
    audio channel."""
    return _NAME_PATTERN.sub(_SELF_REFERENCE, text)

# Global lock to prevent multiple TTS from playing simultaneously
_tts_lock = threading.Lock()

# Deterministic playback markers: Jarvis knows exactly when he is
# speaking, so the ambient listener can attribute audio captured during
# this window as probable self-echo instead of user speech. This is
# state attribution, not acoustic echo cancellation.
_playback_active = threading.Event()
_last_playback_end = 0.0  # monotonic timestamp of last playback end


def _mark_playback_start():
    _playback_active.set()


def _mark_playback_end():
    global _last_playback_end
    _last_playback_end = time.monotonic()
    _playback_active.clear()


# Live engine reference so a barge-in interrupt can cut playing speech.
_active_engine = None


def stop_speaking():
    """Interrupt in-flight speech — SAPI engine or a playing soundboard
    file (best-effort; safe when idle)."""
    engine = _active_engine
    if engine is not None:
        try:
            engine.stop()
        except Exception:
            pass
    try:
        if pygame.mixer.get_init() and pygame.mixer.music.get_busy():
            pygame.mixer.music.stop()
    except Exception:
        pass


def playback_state():
    """(speaking, seconds_since_speech_end). speaking=True while audio
    plays; the tail value is None before the first utterance."""
    if _playback_active.is_set():
        return True, 0.0
    if not _last_playback_end:
        return False, None
    return False, time.monotonic() - _last_playback_end

def generate_filename_from_text(text: str) -> str:
    """Converts a sentence into a standardized, safe filename."""
    if not text:
        return ""
    # Remove punctuation, convert to lowercase, and replace spaces with underscores
    clean_text = re.sub(r'[^\w\s]', '', text).lower()
    return re.sub(r'\s+', '_', clean_text).strip() + ".mp3"

def speak(text: str):
    """
    Speaks the given text using a local audio file or offline Windows TTS.
    """
    global _active_engine
    if not text:
        return
    text = sanitize_for_speech(text)

    # Use lock to prevent multiple TTS from playing simultaneously
    with _tts_lock:
        _mark_playback_start()
        try:
            # --- 1. Find the local audio file in the 'soundboard' dir ---
            script_dir = os.path.dirname(os.path.abspath(__file__))
            project_root = os.path.dirname(script_dir)
            audio_dir = os.path.join(project_root, "data", "soundboard")

            filename = generate_filename_from_text(text)
            filepath = os.path.join(audio_dir, filename)

            # --- 2. Play the file if it exists ---
            if os.path.exists(filepath):
                print(f"[Speaker] Found audio file: '{filename}'. Playing...")
                try:
                    pygame.mixer.init()
                    pygame.mixer.music.load(filepath)
                    pygame.mixer.music.play()
                    while pygame.mixer.music.get_busy():
                        time.sleep(0.1)
                    pygame.mixer.quit()
                    return
                except Exception as e:
                    print(f"[Speaker] ERROR: Failed to play audio file '{filepath}'. Error: {e}")

            # --- 3. Fallback to offline TTS if file doesn't exist ---
            print(f"[Speaker] No local file found for '{text}'. Using system TTS.")
            try:
                engine = pyttsx3.init()
                _active_engine = engine
                try:
                    engine.say(text)
                    engine.runAndWait()
                finally:
                    _active_engine = None
            except Exception as e:
                print(f"[Speaker] TTS Error: {e}")
        finally:
            _mark_playback_end()