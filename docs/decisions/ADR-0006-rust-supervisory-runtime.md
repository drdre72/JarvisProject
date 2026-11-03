# ADR-0006: Rust supervisory runtime (shadow-first)

**Status:** proposed · **Date:** 2026-11-02

## Decision

Introduce a separate, persistent Rust process — `jarvis-core` — as JARVIS's
supervisory runtime ("System A"). It owns operational continuity: typed
events, bounded queues, an attention scheduler with durable obligations, a
transactional SQLite journal, worker lifecycle and restart budgets, a policy
engine that alone can mint authorized actions, and health-based fault
recovery. The existing Python stack remains the intelligence runtime
("System B"): UI, STT, TTS, llama.cpp subprocesses, and the Obsidian vault.

Adoption is staged and reversible:

- **Stage A — shadow:** `jarvis-core` runs standalone; it evaluates and
  journals decisions but dispatches no consequential action.
- **Stage B — limited:** Rust owns one low-risk lifecycle/scheduling
  function via versioned local IPC (stdio first; secured named pipes are the
  Windows option — never sockets, per ADR-0001).
- **Stage C — primary:** Rust becomes the default supervisor only after the
  directive's test and acceptance criteria pass.

A `jarvis-core.toml` flag (`[rust_core] enabled = false,
shadow_mode = true`) keeps the crate inert; `launch_jarvis.bat` and the
Python path are unchanged, and disabling the flag restores today's behavior
exactly.

## Context

- The Phase Zero audit (`docs/rust-core-audit.md`) found **no** supervisor,
  sleep/wake machine, scheduler, durable event journal, IPC protocol, or
  worker-recovery path anywhere in the Python runtime; inference is a
  per-request subprocess with only a timeout, and a parentless
  `llama-completion.exe` has already stranded ~7.1 GiB once.
- ADR-0005 (proposed) requires a deterministic sentinel, a durable
  inspectable journal, bounded background work, and a design that "permits a
  later separate process without a second identity." This ADR is that
  separate process. One identity and one permission authority are preserved:
  the Python decision controller keeps ruling user-facing intent; the Rust
  policy engine rules operation execution; neither can mint permissions.
- Journal reconciliation with ADR-0005: the *operational* journal (events,
  obligations, worker status, action intents/results — reason codes and
  evidence references, never transcripts or prompts) lives in SQLite under
  a gitignored `logs/rust-core/` data dir because it needs transactional,
  restart-safe writes, not human editing. *User-facing* concerns and review
  artifacts remain inspectable Markdown in the `Jarvis/` vault subtree, per
  ADR-0005 and ADR-0004. The two memory kinds are never merged.
- Rust toolchain is not installed on the development machine; installing it
  is a user-approved build-time step (like the wheelhouse), not a runtime
  dependency. `Cargo.lock` will be committed; `cargo vendor` can mirror the
  wheelhouse pattern for offline rebuilds.

## Consequences

- New top-level `jarvis-core/` crate; `target/` and runtime data are
  gitignored. Nothing in `scripts/` changes in Stage A.
- Deterministic operations (timers, dedup, cooldowns, restart budgets,
  authorization checks) never depend on model inference — the model proposes,
  the policy engine disposes.
- `AuthorizedAction` is constructible only inside the policy engine and is
  re-verified (expiry + revocation) at execution time; this is the type-level
  expression of the existing `intent_gate.py` boundary, not a replacement
  for an OS sandbox.
- A Windows autostart/service wrapper and real Python↔Rust IPC are **not**
  part of this decision's first milestone; they arrive with Stages B/C and
  need their own checks (peer identity, message caps, reconnection).
- Model-residency policy stays Python-side until benchmarked (ADR-0005
  adoption step 5); the supervisor merely records and enforces it later.

## Alternatives rejected

- **Grow the supervisor inside `JarvisApp`:** more daemon threads and shared
  mutable state in the UI process — the exact fragility this ADR removes; a
  UI crash would take continuity with it.
- **Wholesale Python→Rust rewrite of STT/LLM/UI:** violates the directive's
  primary requirement and strands 96 passing tests for speculative gains.
- **PyO3/Maturin in-process embedding:** couples Rust crashes to the Python
  process and vice versa before process-level IPC is proven; revisit only
  with profiling evidence.
- **TCP/loopback IPC:** ADR-0001 forbids runtime network surface; stdio and
  named pipes give peer-authenticated local IPC without any socket.
- **Markdown or log-file operational journal:** cannot give atomic,
  transactional obligation state; Markdown stays for user-inspectable memory.
