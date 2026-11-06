# speaker.py
import os
import re
import time
import pygame
import pyttsx3
import threading

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
    if not text:
        return

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
                engine.say(text)
                engine.runAndWait()
            except Exception as e:
                print(f"[Speaker] TTS Error: {e}")
        finally:
            _mark_playback_end()