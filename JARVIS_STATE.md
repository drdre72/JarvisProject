# Jarvis — Current State

> Single-page source of truth for what is true **right now**.
> Keep it current. Stable knowledge belongs in `AGENTS.md`.
> Historical reasoning belongs in `docs/decisions/`.
>
> This source-only snapshot does not include the models, binaries, wheelhouse,
> or private vault. Runtime checks below describe the original development
> checkout unless they explicitly refer to this snapshot.

## Project

- Windows-only Python 3.11 desktop companion (Flet UI), fully offline.
- LLM: llama.cpp `llama-completion.exe` + Qwen2.5-7B Q4_K_M (Apache-2.0), CPU.
- STT: faster-whisper `models/whisper-small/` (local-only).
- TTS: pyttsx3 / Windows SAPI.
- No vendor keys, no cloud calls, no wake word yet.
- Launch: `launch_jarvis.bat`.
- Active memory: local Obsidian-compatible Markdown vault under ignored
  `memory/obsidian-vault/`; an existing absolute vault can be opted in via
  `JARVIS_OBSIDIAN_VAULT`. Old JSON conversations remain read-only context.
- Optional self-purpose record: only a user-authored, manually approved
  `Jarvis/Core/Purpose.md` is read as Jarvis's declared goal. No file or
  purpose text is bootstrapped by Jarvis; Obsidian is an optional editor.
- A bounded, deterministic local decision controller now chooses whether to
  consult approved knowledge, read local CPU/memory status, converse, or
  explain that fresh external data is unavailable. No network capability
  exists; an outward API phase is deferred until security, functionality
  and UI are validated.

## Verified working

- Offline bundle checker: passes end-to-end (exit 0) — all required assets
  staged, wheels hash-verified, deps resolve with `--no-index`.
- Real LLM inference through `LlamaReasoning` (`'2+2'` → `'4'`, ~7.5 tok/s CPU).
- Real faster-whisper load of `models/whisper-small/` via `WhisperSTT`.
- Core dependency imports (flet, pyaudio, pyttsx3, pygame, numpy, websockets).
- Flet UI launches (after `page.window_*` API fix) — first reply succeeded,
  Whisper mic input worked.
- Legacy Picovoice path disabled even when a key is supplied (tested).
- Wheelhouse: 73 wheels staged + installed offline into `.venv`.
- Direct replay of the first uncanned request: intent classification 16.7s,
  conversational reply 9.2s; both completed locally with no deadlock.
- Unit tests cover UI stage feedback; inference writes stage timings (no raw
  prompts) to ignored `logs/runtime.log`. User verified multi-turn voice chat.
- Supported voice command registry currently contains only local note-taking;
  unsupported macOS actions and avatar playback are not advertised.
- Dispatcher matches explicit note/review/recall phrases locally. A separate
  controller then selects a local read-only path or one conversational reply;
  it never runs a model intent classifier. Earlier user logs confirmed that
  removal; two CPU replies took 18.4s and 18.9s.
- Unit tests cover current time/date from the local clock and accurate UI/tool
  capability answers. Both avoid the model; live voice retest is pending.
- Voice idle prompt points to the microphone button (keyless wake word is not
  implemented); persona now distinguishes supported note-taking and monitoring
  from unavailable arbitrary tool/screen access. User confirmed idle wording;
  prompt-only tool wording still failed, so the capability answer is now local.
- A parentless `llama-completion.exe` holding ~7.1 GiB RAM was stopped after
  user approval; available memory increased to ~21.8 GiB.
- Local vault and typed Lute-inspired NLP gate pass temporary-vault unit
  tests (notes, review proposals, approved retrieval, legacy JSON continuity,
  negation, provenance, rejected actions). No real vault/user microphone
  runtime test has occurred after this change.
- Required UI module imports now fail visibly instead of silently switching
  to no-op speech and memory functions; covered by regression tests.
- Local capability registry and controller have unit/integration coverage:
  81 tests pass, including approved-note source revalidation, no redundant
  inference for metrics/live-data requests, and no transcript in decision
  logs. A typed local-model proposal now handles uncanned conversation in one
  inference pass; purpose and capability answers are grounded in the reviewed
  profile or registry, not the model's draft. Synthetic offline Qwen tests
  classified capability, self-purpose and ordinary-advice questions; live
  voice/Obsidian retesting remains pending. A conservative self-claim guard
  rejects model-authored ability/purpose claims when misclassified as dialogue.
- Manually approved flat Knowledge notes now form a local wikilink graph.
  Lexical matches expand through relevant outgoing approved links at most
  two hops, six notes and 3500 Knowledge characters, preserving relationship,
  optional note ID and source path. An in-memory index avoids re-reading all
  note bodies on every request; selected sources are re-read before use.
  Validator tests cover broken/unapproved links, duplicate IDs, invalid
  dependency cycles, orphans and read-only CLI behavior. User vault notes
  were not migrated, edited or auto-linked.

## Active issues (see CURRENT_ISSUES.md for the full list)

1. **Latency:** CPU conversational replies still cold-load the model each
   turn; the controller adds no extra model call. A live voice retest took
   21-23 seconds per reply.
2. Obsidian memory and action gate need live UI/microphone validation,
   including user-edited Knowledge approval and note-taking.
3. Dead code: `ollama.Client()` in `jarvis_commands.py`, macOS `osascript`
   commands, `download_binaries.py`, legacy Vosk/WebSocket path.
4. Dependencies not pinned to exact versions; `requirements.txt` bounds only.
5. PyInstaller build unexercised; one-file layout vs launcher's beside-exe
   asset check is a known mismatch.
6. `flet-video` optional and unstaged → avatar video placeholder.
7. This source-only snapshot has an Apache-2.0 LICENSE; excluded runtime
   assets and media retain their own licensing and provenance requirements.
8. Local capability registry records implementation and test evidence, but
   not live health checks; decision rules are intentionally narrow, not a
   general planner or simulation engine. Structured inference is still model
   classification and can be wrong; ordinary dialogue drafts are unverified.
   Graph relationships are also user-reviewed context, not verified truth.
9. The live session exposed a partial transcription, an Obsidian capability
   hallucination, and a reply arriving after voice deactivation. Routing
   and late-reply suppression have unit coverage. A second session confirmed
   gratitude routing but exposed "do you recognize ... tools or Obsidian?"
   as another hallucination path; that phrase now has a local registry-backed
   answer in tests. Live confirmation of the new route, voice-stop behavior,
   and transcription quality is pending.

## Next architectural targets

1. Validate default vault creation, note-taking, review, and recalled approved
   Knowledge in a live voice session; no user vault data was migrated.
2. Test a real external Obsidian vault only after the user selects its path.
3. Pin exact dependency versions (lockfile from staged wheelhouse).
4. Validate graph retrieval in a real reviewed vault, then consider fuller
   YAML/link syntax, incremental indexing and optional reviewed legacy JSON
   migration; keep memory, imagination and facts separate per rule 3.
5. Test the controller in the live UI, then replace temporary phrase routing
   with an offline intent-to-fulfillment flow: typed goal/constraints,
   satisfaction conditions, capability selection, outcome verification, and
   a supported answer or structured reason the goal could not be met. Carry
   evidence-based capture/transcription/intent failure categories through
   this flow to a polite, non-canned retry explanation. PCM16 RMS arithmetic
   is corrected and bounded wait/speech windows prevent two common cutoff
   paths; thresholds and diagnostic categories still need live calibration.
   Do not claim noise/clipping causes without measured evidence. Preserve the
   separate deterministic authorization boundary for actions.
6. Build a sandboxed tool execution boundary and UI review controls before
   any opt-in outward API connector.
7. PyInstaller packaging once the app is stable in dev mode.
8. Rust supervisory runtime (`jarvis-core/`): ADR-0006 **proposed**; Phase
   Zero audit in `docs/rust-core-audit.md`. Stage A shadow scaffold exists —
   validated state machines, bounded event queue, deterministic attention,
   durable obligation scheduler, SQLite journal actor, policy engine,
   worker/restart types, versioned JSON-lines frame codec. Toolchain
   installed (rustup `stable-x86_64-pc-windows-gnu` 1.99.1 + WinLibs mingw
   gcc for `rusqlite/bundled`). **fmt clean, clippy -D warnings clean,
   56 tests pass** (52 unit + 4 shadow-runtime integration); `verify.ps1`
   runs the Rust stage automatically when cargo is found. Python path
   unchanged; `enabled` and `ipc_requests` default to `false`. Live wire
   checks in `tools/test_rust_core_protocol.py` prove: Python-encoded frames
   accepted and journaled, malformed/oversized/wrong-version/non-event
   input dropped, supervisor outlives closed stdin, `enabled=false` inert
   exit. First Stage B surface wired: `schedule_obligation`/`cancel_obligation`
   over request/response frames (stdout=protocol, stderr=logs), persisted
   before ack, still shadow-evaluated — no consequential dispatch. The app
   now exercises it: at feed start it registers durable obligation
   `obl-review-reminder` (daily, scope `ops.obligation.run`) — the first
   function Python genuinely delegates. Local jarvis-core.toml runs
   `ipc_requests = true`; flipping to false is the tested rollback.
   `duplicate_obligation` is a typed response so relaunches re-register
   idempotently. Stage A
   comparison feed live: `jarvis-core.toml` `enabled=true` makes
   `scripts/rust_core_feed.py` emit trace-correlated
   UserRequestReceived + turn_completed pairs (metadata only) from
   `process_user_command`; `tools/inspect_rust_journal.py --check-shadow`
   summarizes per-trace decisions and fails on any non-shadow dispatch.
   Live-tested 2026-11-02: 8 real voice turns journaled as correlated
   accepted pairs, shadow invariant held, zero transcript content. The
   test also exposed that cleanup() was never wired — now fixed via
   page.on_disconnect + main.py finally + feed.pid stale-supervisor
   reaping. Second Stage B surface wired: `[rust_core] inference_relay`
   (default false) + `[inference]` section give the supervisor ownership
   of the `llama-completion.exe` lifecycle — bounded argv/output,
   single-flight cap, policy-gated `ops.inference.run` intent
   journaled before spawn, watch-channel cancel, timeout kill, and
   shutdown drain guarantee no orphaned child. Python `reasoning.py`
   relays argv when the flag is on and falls back to the identical
   local subprocess only when the request provably never left Python;
   a lost reply maps to `unknown`→TimeoutExpired and is never
   re-dispatched. `stop_voice_recognition` sends fire-and-forget
   `cancel_inference` (bypasses the request gate so it cannot queue
   behind the job it kills). Journal records status/bytes/exit codes
   only — never prompt or output text. Live wire tests use python.exe
   as the worker: completed/timeout/cancel/spawn_failed/busy/disabled
   all verified. Live-proven 2026-11-06: first real relayed
   inference completed (exit 0, stdout 92B, stderr tail bounded at
   65536B, ~14s); llama.cpp -v floods stderr ~0.5 MiB per run — stderr
   is now drained with a bounded retained tail (never kills), stdout
   keeps the hard cap. Live cancel against a running job untested
   (answers were too fast to interrupt). Ambient hearing added:
   persistent segmented capture with deterministic playback
   attribution — segments overlapping TTS output are suppressed so
   Jarvis cannot hear himself; metrics-only, ambient judgment and
   voice barge-in remain unbuilt. Live echo check: tools/echo_probe.py.

## Rules of interpretation

- Prefer repository state over model memory.
- `AGENTS.md` = stable facts; this file = current status; `CURRENT_ISSUES.md`
  = buglist; `PROGRESS_LOG.md` = history; `.devin-context/memory.md` = session.
- `progress.txt` is historical and unreliable — do not trust it.
- A fix marked "applied" is not resolved until `tools\verify.ps1` (or the real
  runtime path) passes.
- Never reintroduce vendor keys or runtime downloads.
