# Offline Windows Bundle: Requirements (Source-Only Snapshot)

Target: Jarvis runs without a network connection, cloud inference, or an API key.
Acquiring and verifying third-party assets is a separate build-time step. Nothing
in this repository constitutes a complete offline distribution. The entries
below describe requirements and the prior development-machine state, not
assets included in this source snapshot.

| Component | Expected location | Current state |
| --- | --- | --- |
| Python 3.11 x64 | Developer `.venv/`; embedded runtime in a future release | Developer environment ready; release runtime not bundled |
| Python dependencies and transitives | `wheelhouse/*.whl` for Windows x64 / Python 3.11 | Not included; `wheelhouse.sha256` is a historical manifest; exact-version lockfile still needed |
| Local LLM | `models/qwen2.5-7b-instruct-q4_k_m.gguf` | Not included; obtain separately and follow the upstream model license |
| llama.cpp CPU runtime | `bin/llama-completion.exe` and supporting DLLs | Not included; obtain separately and follow upstream terms |
| llama.cpp GPU runtime | Optional CUDA build; `JARVIS_GPU_LAYERS` env selects offload | Not staged; CPU is the verified default |
| Local speech-to-text | `models/whisper-small/` with `model.bin` and `config.json` | Not included; code expects a local model and uses `local_files_only=True` |
| Desktop UI | Flet Python package; optional avatar media | Media not included; `flet-video` is optional in code |
| Voice output | `pyttsx3`/Windows SAPI | No soundboard or third-party media included |
| Wake word | Future local, keyless implementation | Not implemented; legacy Picovoice listener is not a release dependency |

The legacy Vosk/WebSocket path and macOS launch scripts are not part of the
Windows release path. `pvporcupine` is deliberately excluded; the Picovoice
wake listener now exits unconditionally, even if an old key is supplied. A
Picovoice key licenses wake-word use; it is not a login to this computer.
Local code cannot prevent someone else from using a leaked key with the vendor.
The owner reports that Picovoice deleted the account, but key invalidation
has not been confirmed separately. The original public repository still has
the historical key; this source-only snapshot does not inherit that history.

## Known blockers

- A real microphone transcription test has not run; `model.bin` is staged and
  model-loading tests confirm the local-only path, but no recorded audio has
  been transcribed through the real `faster-whisper` backend.
- Dependencies are not pinned to exact versions. `numpy` and `pygame` are now
  listed; Flet is bounded to its existing UI generation pending migration.
- The PyInstaller build has not been exercised. Its one-file layout puts data
  inside the executable while the launcher checks for assets beside it; the
  build and portable test tools now resolve paths from the repository root.
- The full Flet UI has not been launched; `main.py` asset checks and the
  `LlamaReasoning` module were verified directly instead.

## Offline readiness checks

After separately staging the assets and compatible wheels, run
`.\.venv\Scripts\python.exe tools\check_offline_bundle.py` from the root.
This read-only check reports missing runtime artifacts, verifies every wheel
against `wheelhouse.sha256`, and resolves the entire Python dependency graph
using pip with `--no-index` and `--ignore-installed`. It does not download or
install anything. A passing wheel check does not yet prove that the packaged
app never attempts a network call; that requires a clean-machine,
network-disabled launch test.

The wheels were obtained from public PyPI at build time; `wheelhouse/` is
ignored by Git, while the verified inventory is tracked. Freeze and review
exact package versions before distributing a release. Obtain and hash-pin
the model and native binaries from their original sources, and review model
licensing. Do not run download or installer scripts during an offline runtime
test.
