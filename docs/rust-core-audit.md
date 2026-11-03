# Rust Core Audit — Phase Zero (2026-11-02)

Evidence-backed architecture inventory for the proposed Rust supervisory
runtime ("System A") alongside the existing Python intelligence runtime
("System B"). Every claim cites the file that proves it. Produced for the
Rust Core Engineering Directive; see `docs/decisions/ADR-0006-*` for the
proposed decision.

## 1. Which components already supervise execution?

**None as a dedicated supervisor.** The whole runtime is one Python process:

- `main.py` → env setup → dependency/asset checks → `ft.app(JarvisApp)`
  (`main.py:84-139`).
- `JarvisApp` owns two daemon threads (`scripts/jarvis_app.py`):
  - `voice_loop` — blocking PyAudio capture → faster-whisper →
    `process_user_command` (`jarvis_app.py:764-810`).
  - `monitor_system` — 2-second `psutil` polling that only repaints UI
    metrics (`jarvis_app.py:103-144`). It observes; it does not supervise.
- The only process boundary in the system is `subprocess.run` per inference
  call to `bin/llama-completion.exe` with a 120 s timeout
  (`scripts/reasoning.py:92-99, 214-217`). No readiness probe, heartbeat,
  restart budget, or circuit breaker exists anywhere.
- Failure evidence: an orphaned parentless `llama-completion.exe` holding
  ~7.1 GiB was found and killed after user approval (PROGRESS_LOG
  2026-10-29). Voice-stop discards a late reply but cannot cancel in-flight
  inference (`CURRENT_ISSUES.md` #16; `jarvis_app.py:573-578, 651-656`).

## 2. Which components implement sleeping and waking?

**None.** `self.is_listening: bool` toggles the voice thread
(`jarvis_app.py:71, 756-817`); there is no state machine, no sleep mode, no
wake path, no persistence across restarts. ADR-0005 (`docs/decisions/`,
status **proposed**) designs Idle/Foreground/Background attention modes — not
implemented today. Wake-word is dead code (`scripts/wake_listener.py`
exits unconditionally; keyless local wake word is roadmap item #13).

## 3. Do scheduling and event persistence exist?

**Neither.**

- No timers beyond `time.sleep` polling loops (monitor 2 s, TTS drain).
- No durable event journal. `logs/runtime.log` is diagnostics-only
  (rotating 1 MB × 2, stage timings and reason codes, never transcripts —
  `scripts/runtime_log.py`). It is not a recovery journal.
- No scheduled obligations. The Obsidian vault stores personal/semantic
  memory (Conversations/Notes/Review/Knowledge/Core), not operational
  events (`scripts/obsidian_memory.py:76-81`).
- In-flight user work: `tools/benchmark_inference.py` + a context-reduction
  change (`get_intent_context`, `obsidian_memory.py:318-329`) are ADR-0005
  adoption step 1 — measuring inference phases before scheduling work.

## 4. How do the two runtimes communicate?

**There is only one runtime today.** Model access = spawn-per-request with
CLI args and captured stdout/stderr (`reasoning.py:70-99, 196-217`). No IPC
protocol, no persistent worker, no message schema, no peer authentication.
`scripts/system_monitor.py` is a standalone stdout-JSON stats script that
nothing imports. Dead/legacy paths: `config.py` (Vosk leftover),
`jarvis_listen.py` (demo), `command_processor.py` (unreferenced CLI entry),
`llama_trigger.py`, `wake_listener.py`, `memory_system.py` (read-only legacy
context).

## 5. Which functions could benefit from Rust?

Per directive mapping, all of System A, none of which exists in Python today:

| Function | Directive phase | Today |
|---|---|---|
| Event bus, dedup, prioritization | 2 | Nothing |
| Attention clock / obligations | 3 | `time.sleep` polls only |
| Durable journal | 4 | `runtime.log` diagnostics only |
| Worker lifecycle/restart | 5 | Per-request subprocess, no recovery |
| Python ↔ supervisor IPC | 6 | Nothing |
| Resource monitor → decisions | 7 | `psutil` → UI text only |
| Policy/authorization engine | 8 | `intent_gate.py` covers 3 memory verbs only |
| Health/fault recovery | 5 | None |

Python stays responsible for STT, TTS, LLM subprocess, UI, vault —
those are working and tested (96 tests, verify loop green).

## 6. Which components must remain unchanged?

- `launch_jarvis.bat` → `main.py` → `JarvisApp` default startup path.
- Vault layout, approval gates, `_safe_file` symlink rules
  (`obsidian_memory.py`); `intent_gate.py` parse→authorize semantics;
  `capability_registry.py`; `decision_controller.py` routing;
  `speech_recognition.py` capture contract; `speaker.py`/`text_to_speech.py`.
- Private vault contents (`memory/`, incl. `Jarvis/Core/Purpose.md`) —
  untouched, uncommitted, never read into this audit.
- All uncommitted user work: ADR-0005 edits, `benchmark_inference.py`,
  `get_intent_context` context reduction, associated tests.

## 7. Conflicts with accepted ADRs

- **ADR-0001 (offline, accepted):** compatible — the supervisor does no
  network I/O; IPC must be stdio or named pipes, never sockets. `cargo
  build` fetches crates once at **build time** (user-approved staging step,
  same class as the wheelhouse); commit `Cargo.lock` afterwards and document
  `cargo vendor` for fully offline rebuilds.
- **ADR-0004 (vault + intent gate, accepted):** compatible — Rust handles
  *operational* memory (events, obligations, worker state); the Markdown
  vault stays the only *personal/semantic* memory. This implements the
  directive's memory separation, it does not blur it.
- **ADR-0005 (proposed, user-edited in flight):** the Rust supervisor **is**
  the sentinel + journal + inference-owner direction it describes, as "a
  later separate process without a second identity" (`ADR-0005` Consequences
  ¶1). Reconciliation needed on journal location: ADR-0005 suggests the
  journal lives under the inspectable `Jarvis/` vault subtree with "storage
  format to be chosen" — the proposal here puts the *operational* journal in
  SQLite under a gitignored data dir (durable, transactional, bounded),
  while *concern/user-facing* records stay Markdown in the vault. That split
  preserves both ADR-0005's inspectability and the directive's
  operational/personal memory separation; it is called out in ADR-0006
  rather than silently decided.
- **ADR-0003 (workflow):** followed — this audit, a proposed (not accepted)
  ADR, PROGRESS_LOG append, JARVIS_STATE update, verify loop run.

## Known risks / gaps the plan must handle

- **No Rust toolchain on this machine** — no `cargo`/`rustc`/`rustup`, no
  MSVC/`cl.exe` or Windows SDK. `windows-gnu` (bundles its own mingw linker)
  is the lightest install; rustup + `stable-x86_64-pc-windows-gnu` needs no
  Visual Studio. Toolchain install is a user-approved build-time step.
- **`rusqlite` `bundled` feature compiles sqlite3.c** — works under
  windows-gnu's bundled gcc without VS Build Tools. Chosen over `redb`
  (non-SQLite) to match the directive's tech table.
- **Two TTS modules** (`speaker.py` vs `text_to_speech.py`) — pre-existing
  duplication, not a Rust concern; noted for the record.
- **Feature flag**: nothing in Python invokes the Rust binary in Phase One;
  `[rust_core] enabled = false` in `jarvis-core.toml` keeps it inert.

## Migration plan (summary — details in ADR-0006)

| Stage | What | Execution authority |
|---|---|---|
| A — Shadow | `jarvis-core` runs standalone: config → journal → scheduler → event intake → decision logging only | Legacy Python unchanged; Rust dispatches nothing consequential |
| B — Limited | Rust owns one low-risk function (scheduled-obligation timing via stdio to a Python adapter) behind the flag | Rollback = `enabled = false` |
| C — Primary | Rust supervisors workers + IPC; Python becomes workers (STT/TTS/LLM/UI) | Only after directive DoD tests pass |

Phase One implemented now: crate skeleton with real, tested internals —
validated state machine, bounded event queue with dedup/rate limits,
deterministic attention rules, obligation scheduler with missed-tick
policies, SQLite journal actor, policy engine with private-ctor
`AuthorizedAction`, worker-state + restart-budget types, JSON-lines frame
codec, supervisor loop in shadow mode, graceful shutdown. Worker spawning
and Python IPC are typed interfaces only — deliberately not wired.
