# AGENTS.md — Jarvis Agent Knowledge Cache

This public-source snapshot excludes the private working vault, staged models,
binaries, wheelhouse, and media. Any status that depends on those assets is
historical information from the development checkout, not proof they ship here.

Persistent, hand-maintained notes for AI agents working on this repo.
Stable facts live here; dynamic state belongs in `JARVIS_STATE.md`.
Private session notes and editor settings are intentionally not included.

## GOVERNING RULES

1. **Jarvis is fully offline at runtime.** No cloud inference, no vendor
   wake-word keys, no model downloads inside application code paths. Build-time
   asset staging (wheelhouse, models, binaries) is a separate, approved step.
   Outward APIs are deferred until the local brain/controller, security,
   functionality and UI have been validated. No network tools are enabled.
2. **No credentials in the repo.** The historical Picovoice key is compromised;
   `scripts/wake_listener.py` is deliberately disabled and must not be revived.
   `JARVIS_PICOVOICE_ACCESS_KEY` must not be reintroduced.
3. **Memory, imagination, and facts are separate.** Retrieved memory,
   generated content, and verified facts must not silently mix. This governs
   the future Obsidian memory layer.
4. **Prefer repository state over model memory.** Check files and run the
   verification loop before asserting something is done.
5. **Intent-to-fulfillment is the target, not phrase matching.** Interpret
   the user's goal, constraints and needed evidence; define what would
   satisfy the request; choose permitted capabilities; verify the outcome.
   Reply with a supported result, or a concise explanation of missing
   inputs, unavailable capabilities, failed checks or required permission.
   Label partial results as partial. Current keyword responses are temporary
   testing safeguards, not the long-term decision architecture. Model text
   may propose a plan but never authorize side effects.

## Communication failure contract (partially implemented)

- A voice turn passes through capture, transcription, semantic intent
  interpretation, and fulfillment. Each stage should return a typed outcome
  with stage, reason category, supporting measurements or observations,
  uncertainty, and a suitable retry or clarification action. Text input
  enters at interpretation. Do not collapse every empty transcription into
  "the user was silent," or confuse a known goal with unavailable tools.
- Candidate categories include no speech detected; speech too quiet relative
  to a calibrated noise floor; clipped/saturated samples; background
  interference with measured speech-to-noise evidence; recording cut short;
  uncertain transcription; ambiguous intent; missing critical inputs;
  unavailable capability or permission; device/model failure; and unknown
  cause. Classify only when the relevant evidence exists. Loudness alone
  cannot prove background noise or clipping; a failed intent parse does not
  establish an audio fault. If evidence conflicts, report uncertainty.
- `speech_recognition.py` now widens PCM16 samples before squaring, records
  raw/normalized RMS, clipping fraction, stop reason and basic transcription
  status. Capture waits up to five seconds for activity and can then record
  up to twelve seconds, stopping after two seconds of trailing quiet.
  A turn stopped by the speech deadline cannot authorize even an explicit
  memory command because the transcript may be incomplete. These are
  measurements/safety guards, not calibrated acoustic diagnoses. The fixed
  activity threshold still needs calibration with real recordings, plus a
  measured speech/noise estimate.
  Transcription quality signals require validation against real recordings
  before being treated as calibrated confidence.
- A constrained offline response renderer should turn the typed outcome into
  a brief, polite explanation and one actionable retry hint. It may vary
  wording but cannot invent an acoustic cause or change the reason category;
  avoid a table of canned category-to-sentence responses. If language
  generation is unavailable, the UI can show the structured category and
  hint without pretending to know more. Diagnostics in local logs should
  store reason codes/metrics, not raw audio or transcripts. Uncertain speech
  never authorizes a memory write or tool action without normal validation.

## Stack and runtime

- Windows-only desktop app: Python 3.11 x64 + Flet 0.21.2 UI.
- Inference: `bin/llama-completion.exe` (llama.cpp b11985, win-cpu-x64) driven
  by `scripts/reasoning.py`. `llama-cli.exe` is **chat-mode only** in modern
  llama.cpp — always use `llama-completion.exe` for raw prompt completion.
- STT: `scripts/speech_recognition.py` → faster-whisper loading
  `models/whisper-small/` with `local_files_only=True`. Never pass a bare model
  name — that silently downloads.
- TTS: `scripts/speaker.py` → `pyttsx3` (Windows SAPI). Do not shell out with
  interpolated text (previous code had a command-injection bug here).
- Memory: `scripts/obsidian_memory.py` writes Markdown to the default ignored
  `memory/obsidian-vault/` or a deliberately chosen existing vault. Old
  `scripts/memory_system.py` JSON is read-only legacy context in the live app.
  `OBSIDIAN_BRAIN.md` explains approval and provenance.
- Ambient hearing: `scripts/ambient_listener.py` runs one persistent mic
  capture segmented by energy; `scripts/speaker.py` marks deterministic
  playback state, and segments overlapping TTS output (start-in-echo or
  >50% overlap, plus a 0.6s tail) are suppressed — Jarvis cannot hear
  himself as the user. This is state attribution, not acoustic echo
  cancellation: user speech starting during playback is also suppressed
  (real barge-in needs a separate interrupt mechanism). The voice loop
  suspends ambient while it owns the mic; resume is idempotent and
  restarts a dead listener. `tools/echo_probe.py` is the live
  self-hearing check.
- Wake word: encoded rule — `speaker.sanitize_for_speech` rewrites
  "jarvis" out of every utterance (he self-refers only as "Just a
  Rather Very Intelligent System"), so a heard "jarvis" is provably
  human even inside echo-tagged audio. Heard segments go to
  `scripts/address_spotter.py` (depth-1 drop policy, transcribe only to
  find the wake word, transcripts discarded; suppressed audio is never
  transcribed). A spotted "jarvis" engages the normal voice loop like a
  remote mic-button press and any words after the name dispatch as the
  first turn. Ambient audio still never becomes memory or commands
  except through that addressed-turn path.
- Speech interrupt plumbing exists (`speaker.stop_speaking` — SAPI
  engine stop + pygame music stop) but no trigger calls it yet;
  word-level barge-in during playback is deliberately deferred.
- The Markdown vault is part of Jarvis's local identity and memory; Obsidian
  is an optional viewer/editor, not an external vault-management tool.
  `Jarvis/Core/Purpose.md` is an optional user-authored, manually approved
  self-purpose record. Jarvis never creates or promotes it, and purpose text
  cannot grant permissions or override the offline policy.
- Voice dispatcher: only local note-taking is registered. Legacy macOS
  commands and avatar playback are not supported on Windows; do not re-add
  them before implementing and testing a sandboxed local replacement.
- Dispatcher recognizes explicit leading "write this down" and "remember
  that" requests, approved-memory recall and refused deletion before
  conversation. Never let model output authorize tool execution; future tool
  intents require deterministic validation.
- `scripts/intent_gate.py` adapts Lute's ordered parse -> typed intent ->
  authorization -> domain authority pattern. "Remember that" goes to Review,
  never Knowledge; only manually approved Knowledge files are retrieved.
- `scripts/capability_registry.py` declares actual local capabilities and
  evidence; `scripts/decision_controller.py` chooses only read-only local
  metrics, relevant approved-memory context, ordinary conversation, or a
  truthful offline/unknown response. This is not a general agent or sandbox.
- For uncanned conversation, `scripts/semantic_intent.py` validates a
  constrained local-model hypothesis. Capability and purpose answers ignore
  model drafts and use registry/profile data; ordinary dialogue remains
  generated and unverified. This is a first slice, not a general planner or
  independently verified intent classifier.
- `scripts/obsidian_graph.py` indexes only approved flat Knowledge notes;
  `scripts/context_retrieval.py` follows relevant outgoing wikilinks at
  most two hops under a context budget; `scripts/validate_vault.py` checks
  graph integrity without writing notes. No link promotes an unapproved
  note, no backlinks are traversed automatically, and relationship edges
  never prove a claim. Source paths remain in assembled context.
- `JARVIS_OBSIDIAN_VAULT` - optional existing absolute Obsidian vault path;
  omit for ignored project-local default. No network/plugin calls for memory.
- Current time/date and UI/tool visibility questions are answered from the
  local clock and capability registry, not model recollection. Do not ask
  the model to invent operational state or timestamps.
- Model: `models/qwen2.5-7b-instruct-q4_k_m.gguf` (Apache-2.0, ungated).
  ~7.5 tok/s on this CPU.
- `jarvis-core/` — proposed Rust supervisory runtime (ADR-0006, Stage A
  shadow: evaluates + journals, dispatches nothing). Off by default via
  `[rust_core] enabled` in `jarvis-core.toml`; the Python path is
  unchanged. `[rust_core] ipc_requests` (default false) gates the Stage B
  request surface: `schedule_obligation`/`cancel_obligation` over
  request/response frames; stdout is protocol-only, tracing logs go to
  stderr. `scripts/rust_core_adapter.py` is the Python peer;
  `tools/test_rust_core_protocol.py` runs live wire checks when the
  binary exists. With `[rust_core] enabled = true`,
  `scripts/rust_core_feed.py` emits a correlated turn start/outcome
  pair per user command — metadata only, never transcripts — so the
  shadow journal can be compared against live behavior via
  `tools/inspect_rust_journal.py --check-shadow`. The feed writes
  `feed.pid` in the data dir and reaps a stale feed-spawned supervisor
  before spawning (jarvis-core survives a dead parent by design);
  `cleanup()` is idempotent and wired to Flet `on_disconnect` plus a
  `main.py` finally. With `ipc_requests = true`, `jarvis_app` also
  registers one durable obligation at feed start — daily
  `obl-review-reminder` (scope `ops.obligation.run`, 24h recurrence,
  run-once-on-recovery) via failure-isolated `feed.ensure_obligation`;
  re-registration answers typed `duplicate_obligation`, `ipc_disabled`
  is the tested rollback, and due runs still dispatch shadow-only.
  `[rust_core] inference_relay` (default false) + `[inference]` give
  the supervisor ownership of the `llama-completion.exe` lifecycle:
  bounded argv/stdout/stderr, single-flight cap, `ops.inference.run`
  policy intent journaled pre-spawn, `watch`-channel cancel, timeout
  kill, shutdown drain — no orphaned child. `reasoning._dispatch`
  relays argv minus argv[0] through `feed.run_inference` when the flag
  is on; the identical local subprocess runs only when the request
  provably never left Python (`None` reply); a lost reply is typed
  `unknown`→TimeoutExpired and never re-dispatched. Voice-stop sends
  fire-and-forget `cancel_inference` (bypasses the request gate —
  it must not queue behind the job it kills). The journal stores
  status/byte counts/exit codes only — never prompt or output text.
  Toolchain installed: `rustup` default
  `stable-x86_64-pc-windows-gnu` (1.99.1) plus WinLibs mingw `gcc` on PATH
  (`rusqlite/bundled` compiles SQLite from C — `cc` needs `gcc.exe`; no
  MSVC/Visual Studio required). `tools\verify.ps1` runs fmt/clippy/test
  automatically when `cargo` is present.

## Environment variables

- `JARVIS_BASE_PATH`, `JARVIS_APP_PATH` — set by `main.py`; STT respects
  `JARVIS_BASE_PATH` for the models dir.
- `JARVIS_GPU_LAYERS` — default `0` (CPU build staged). Raise only when a
  GPU llama.cpp build is staged in `bin/`.
- `JARVIS_LLAMA_THREADS` — default `min(8, cpu_count)`.
- `JARVIS_LLAMA_TIMEOUT` — seconds, default `120`.

## Running

- Always launch via `launch_jarvis.bat` (uses `.venv\Scripts\python.exe`).
- Bare `python` resolves to the **system** Python 3.11 with no deps — or the
  Windows Store alias on a stale PATH. This was a real failure mode.
- Dev commands must use `.\.venv\Scripts\python.exe` explicitly; this terminal's
  PATH was created before Python was installed.

## Verification loop (run after every change)

```powershell
powershell -File tools\verify.ps1
```

Writes `reports\verify_<timestamp>.txt` (gitignored) covering: offline bundle
check, unit tests, `compileall`, and git hygiene. Individual pieces:

```powershell
.\.venv\Scripts\python.exe tools\check_offline_bundle.py
.\.venv\Scripts\python.exe -m unittest discover -s tools -p 'test_*.py' -v
.\.venv\Scripts\python.exe -m compileall -q main.py scripts tools
```

## Known gotchas (learned the hard way)

- **flet 0.21 API**: `page.window` does not exist — use `page.window_*`
  attributes (`page.window_width`, `page.window_maximized`, ...). The code was
  written against the newer `page.window.*` API once; 0.21.2 is the pin.
- **`page.window.*` vs `page.window_*`** was the first-launch crash.
- **Shell quoting**: this repo runs under PowerShell — `2>'$env:TEMP\x'`
  (single quotes) does not expand. Use double quotes for env paths.
- **PowerShell `curl`** is an alias for `Invoke-WebRequest`; use `curl.exe`
  for real downloads.
- **`[IO.File]::ReadAllBytes` fails on files >2 GB** — use a FileStream to
  read headers of GGUF files.
- **llama.cpp output**: `-v` writes logs to stderr; response stats lines look
  like `[ Prompt: ... t/s ]` on stdout — `reasoning.py` filters them.
- **Stale docs**: `progress.txt` and older README/SETUP claims in the original
  development tree were historical and partially false. `PROGRESS_LOG.md` +
  `JARVIS_STATE.md` are the project records in this snapshot.
- **Garbled emoji**: `tools/build_package.py` and `test_llama.py` contain
  mojibake from earlier
  encoding issues; replace rather than matching on emoji in patches.

## Repository layout

- `main.py` — launcher: env setup, dependency check, asset check, Flet start.
- `launch_jarvis.bat` — correct entry point (venv-aware).
- `scripts/` - app modules (`jarvis_app.py` UI, `reasoning.py`, `dispatcher.py`,
  `capability_registry.py`, `decision_controller.py`, `semantic_intent.py`,
  `obsidian_graph.py`, `context_retrieval.py`, `validate_vault.py`,
  `jarvis_commands.py`,
  `speech_recognition.py`, `speaker.py`,
  `runtime_log.py` (rotating stage diagnostics), `memory_system.py`,
  `wake_listener.py` DISABLED, `llama_trigger.py` legacy).
- `bin/` — llama.cpp binaries are staged separately and are not included here.
- `models/` — model assets are staged separately and are not included here.
- `wheelhouse/` — optional offline dependency wheels are not included here;
  `wheelhouse.sha256` is only a manifest.
- `tools/` - `check_offline_bundle.py`, `test_offline_bundle.py`,
  `test_runtime_flow.py`, `test_obsidian_brain.py`,
  `test_decision_controller.py`, `test_semantic_intent.py`,
  `test_obsidian_graph.py`, `verify.ps1`,
  `append_progress.py`, `checklist_tool.py`, `test_portable.py`, `build_package.py`.
- `OFFLINE_BUNDLE.md` — offline distribution baseline + readiness states.
- `JARVIS_STATE.md` — single-page current truth. `AGENTS.md` — stable facts.
- `PROGRESS_LOG.md` — append-only work log. `CURRENT_ISSUES.md` — buglist.
- `docs/decisions/` — ADRs for architectural choices.
- `docs/rust-core-audit.md` — Phase Zero architecture inventory for the
  Rust migration (evidence-backed, cites files).
- `jarvis-core/` — Rust supervisory crate (ADR-0006): `src/` modules per
  the crate README; `jarvis-core.toml.example` documents the feature flag.
- `.devin-context/` — local-only agent memory; excluded from this snapshot.
- `reports/` — gitignored verification output.
- `logs/runtime.log` — gitignored rotating inference stage diagnostics;
  timestamps and outcomes only, never prompts or transcripts.

## Non-goals / dead paths

- `scripts/wake_listener.py`, `scripts/llama_trigger.py`, `launch_jarvis_full.command`,
  `download_binaries.py`, Vosk model dir — legacy, not part of the release path.
- `ollama.Client()` reference in `scripts/jarvis_commands.py` — dead code.
- AppleScript/`osascript` commands in the registry — macOS leftovers.
