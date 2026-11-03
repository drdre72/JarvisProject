# jarvis-core — Rust supervisory runtime (System A)

Persistent supervisor for JARVIS: typed events, bounded queues, an
attention scheduler with durable obligations, a transactional SQLite
journal, worker lifecycle/budget types, the authorization boundary, and
fault recovery. The Python stack (Flet UI, faster-whisper, pyttsx3,
llama.cpp, Obsidian vault) remains the intelligence runtime (System B).

Status: **ADR-0006 Stage A (shadow)** — the supervisor evaluates and
journals every decision but dispatches no consequential action. See
`docs/decisions/ADR-0006-rust-supervisory-runtime.md` and
`docs/rust-core-audit.md`.

## Build

Toolchain (this machine): `rustup` with default
`stable-x86_64-pc-windows-gnu` (rustc/cargo 1.99.1). `rusqlite/bundled`
compiles SQLite from C, so a MinGW `gcc.exe` must be on PATH — installed
here via `winget install BrechtSanders.WinLibs.POSIX.UCRT`. No Visual
Studio required. `tools\verify.ps1` probes `~/.cargo/bin` and the winget
WinLibs path when a stale PATH hides them.

```bash
cd jarvis-core
cargo build            # fetches crates once — an approved build-time step
cargo test             # unit + integration tests
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

Verified 2026-11-02: fmt clean, clippy `-D warnings` clean, 54 tests pass
(52 unit + 2 shadow-runtime integration).

Commit `Cargo.lock` after the first build; `cargo vendor` mirrors the
wheelhouse pattern for fully offline rebuilds (ADR-0001).

## Run (shadow)

```bash
cp jarvis-core.toml.example ../jarvis-core.toml   # then set enabled = true
cargo run -- --config ../jarvis-core.toml
```

With `enabled = false` the binary exits immediately — nothing else invokes
it, so the legacy runtime is unaffected. In shadow mode the supervisor:

- reads JSON-lines frames on stdin (`transport.rs` contract: version,
  size cap, validation before queue space is used),
- admits events to a bounded, deduplicating, priority queue,
- releases durable obligations from the SQLite journal on missed-tick-safe
  policies,
- evaluates policy, records idempotent dispatch attempts, and journals
  every accept/defer/reject/escalate reason,
- shuts down cleanly on Ctrl-C.

Obligations are journaled transactionally *before* acknowledgement and
reloaded on start — process death mid-cycle redelivers the work through the
same idempotency key instead of duplicating it.

## Layout

| File | Owns |
|---|---|
| `state.rs` | `CognitiveState` + `ModelResidency` machines, validated transitions |
| `events.rs` | `EventEnvelope` contract, classification, bounded admission queue |
| `attention.rs` | Deterministic attend/defer/ignore/escalate rules |
| `scheduler.rs` | Wall-clock obligations, missed policies, burst limits |
| `journal.rs` | SQLite operational store on one writer thread (bounded channel) |
| `policy.rs` | `Decision`/`AuthorizedAction` — minting only inside the engine |
| `workers.rs` | Worker status machine, staleness, restart budget + backoff |
| `transport.rs` | Versioned JSON-lines frame codec (stdio/pipe contract) |
| `health.rs` | Resource snapshot + nominal/constrained/critical assessment |
| `supervisor.rs` | Message-driven loop wiring all of the above |
| `main.rs` | Config → journal → restore → monitor → graceful shutdown |
