# JarvisProject - Offline Windows Development Build

This is a source-only snapshot. It has no history from the original
JarvisAssistantFULL repository and does not include personal memory, media,
models, binaries, or dependency wheels. It has not been published.

Local voice assistant with an inspectable, review-gated Obsidian memory vault.

JarvisProject uses local faster-whisper speech recognition, Qwen2.5-7B
inference on a staged llama.cpp CPU build, and offline Windows TTS. This is
not yet a verified portable release. See `JARVIS_STATE.md` for current truth
and `CURRENT_ISSUES.md` for limitations.

## Key Features

- **Offline Voice Recognition** - Fast, accurate speech-to-text with Whisper
- **Local AI** - Qwen2.5-7B-Instruct on the staged CPU llama.cpp runtime
- **Natural Speech Synthesis** - Windows SAPI integration
- **Inspectable Memory** - Local Markdown vault with manually approved knowledge
- **Offline Decisions** - Local capability registry and a small rule-based
  controller select relevant memory, CPU/memory readings, or a model reply
- **System Monitoring** - Real-time performance dashboard
- **Desktop UI** - Flet interface with live metrics
- **Offline Development** - Local-only runtime; portable build still unverified

## Performance

| Configuration | Speed | Requirements |
|--------------|-------|-------------|
| **CPU runtime** | Prior development-checkout measurement: about 7.5 tokens/sec; 18-19s replies observed | 16GB+ RAM recommended |
| **GPU runtime** | Not staged or tested | Separate future build |

## Quick Start

### 1. Get the source
```bash
git clone https://github.com/OWNER/JarvisProject.git
cd JarvisProject
```
Replace `OWNER` with the GitHub account or organization that publishes it.

### 2. Download AI Model
Stage the ungated Qwen2.5-7B-Instruct Q4_K_M GGUF, CPU llama.cpp
binaries and faster-whisper files as documented in `OFFLINE_BUNDLE.md`.
These assets are not included in this source-only repository. Obtain them
separately from their upstream sources and follow their licenses.

Save as: `models/qwen2.5-7b-instruct-q4_k_m.gguf`

### 3. Install Dependencies
```bash
.\.venv\Scripts\python.exe -m pip install -r requirements.txt
```
This installation step may need package-index access. For an offline setup,
prepare a compatible wheelhouse separately and use `--no-index`; no wheels
are included here. The application runtime is designed to operate offline.

### 4. Launch
```bash
launch_jarvis.bat
```
This uses the project `.venv` when present; `python main.py` also works if your
system Python already has the dependencies installed.

## GPU Acceleration (Future)

The staged binaries are CPU-only. GPU offload defaults to zero; no CUDA
toolkit or automatic GPU detection is needed for the current build.

## Voice Interaction

1. **Start listening**: Use the microphone button; wake-word listening is disabled.
2. **Natural Conversation**: Ask questions or use explicit local memory commands.
3. **Local Responses**: Time, metrics and gratitude avoid inference; uncanned
   questions use the local model and may take longer on a CPU.
4. **Inspectable memory**: `OBSIDIAN_BRAIN.md` explains the local vault,
   reviewed purpose profile, bounded approved-note wikilink graph, review
   queue and deterministic memory-action gate.

**Example Interactions:**
- *"Jarvis, write this down: Call Mom tomorrow."*
- *"Remember that I prefer short answers."* (saved for review)
- *"What do you remember about my preferences?"* (approved notes only)
- *"Tell me about your capabilities"*
- *"What's my system status?"*
- *"Can you hear me, Jarvis?"*

## Technical Architecture

```
JarvisProject/
main.py                      # Launcher and offline asset checks
bin/                         # Ignored CPU llama.cpp binaries
models/                      # Ignored local GGUF and Whisper models
scripts/reasoning.py          # Qwen completion subprocess
scripts/speech_recognition.py # Local faster-whisper
scripts/speaker.py            # Offline Windows TTS
scripts/dispatcher.py         # Deterministic command routing
scripts/intent_gate.py        # Typed memory intent authorization
scripts/obsidian_memory.py    # Markdown vault reader/writer
scripts/capability_registry.py # Declared local abilities and evidence
scripts/decision_controller.py # Bounded offline response selection
memory/obsidian-vault/        # Ignored user-owned Obsidian vault
```

## System Requirements

### Minimum
- Windows 10/11 x64
- 16GB+ RAM recommended for the local model and speech pipeline
- 10GB free disk space
- Microphone & speakers

### Recommended
- Windows 11
- 16GB+ RAM
- CPU inference (no GPU binaries staged)

## Latest Updates

**Current development milestone:** offline voice conversation and local CPU
inference were verified on this machine. The Obsidian vault, deterministic
memory gate and first local-only decision controller are unit-tested; the
new controller needs a live voice/UI test. No online tools, general-purpose
simulation or portable release are available yet.

## Documentation

- **Setup Guide**: See `SETUP.md` for detailed installation
- **Current state and issues**: `JARVIS_STATE.md`, `CURRENT_ISSUES.md`
- **Memory setup and review**: `OBSIDIAN_BRAIN.md`
- **Development log**: `PROGRESS_LOG.md`
- **Architecture**: See `docs/decisions/` for recorded design decisions

## Contributing

Contributions welcome. Packaging and a clean-machine network-disabled launch
have not yet been verified. Run `tools/verify.ps1` for the current checks.

## License

The source code in this snapshot is offered under Apache-2.0; see `LICENSE`.
This license does not cover externally staged models, inference binaries,
dependency wheels, third-party audio, or visual assets. Obtain those from
their original sources under their respective terms. No media is included.

---

**Development status:** local voice conversation works on the development
machine; Obsidian memory and the logic gate still need live UI validation.