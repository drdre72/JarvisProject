//! jarvis-core binary — persistent supervisory runtime (ADR-0006 Stage A).
//!
//! Startup contract:
//!   1. Load + validate `jarvis-core.toml` (`--config PATH` overrides).
//!   2. `enabled = false` -> exit immediately; the legacy Python path is
//!      the only runtime.
//!   3. Open + migrate the SQLite operational journal; refuse to run
//!      without durability.
//!   4. Restore pending/parked obligations.
//!   5. Enter the monitoring loop: stdin JSON-lines intake + scheduler tick
//!      + graceful Ctrl-C shutdown.

use jarvis_core::config::{CoreConfig, DEFAULT_CONFIG_PATH};
use jarvis_core::events::EventEnvelope;
use jarvis_core::journal::JournalHandle;
use jarvis_core::scheduler::Scheduler;
use jarvis_core::supervisor::Supervisor;
use jarvis_core::transport::{self, FrameBody};
use std::path::PathBuf;
use std::process::ExitCode;
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

fn config_path_from_args() -> PathBuf {
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--config" {
            if let Some(p) = args.next() {
                return PathBuf::from(p);
            }
        } else if let Some(p) = arg.strip_prefix("--config=") {
            return PathBuf::from(p);
        }
    }
    PathBuf::from(DEFAULT_CONFIG_PATH)
}

/// One validated JSON-lines frame per stdin line. This is the shadow-stage
/// ingress: untrusted input is decoded, version-checked, and validated
/// before it can occupy a queue slot.
fn spawn_stdin_intake(
    tx: mpsc::Sender<EventEnvelope>,
    max_payload: usize,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        use std::io::BufRead;
        let stdin = std::io::stdin();
        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(e) => {
                    warn!(error = %e, "stdin read failed");
                    break;
                }
            };
            if line.trim().is_empty() {
                continue;
            }
            match transport::decode(&line, transport::MAX_FRAME_BYTES) {
                Ok(frame) => match frame.body {
                    FrameBody::Event(env) => {
                        if let Err(e) = env.validate(max_payload) {
                            warn!(error = %e, "dropped invalid event from stdin");
                            continue;
                        }
                        if tx.blocking_send(env).is_err() {
                            break;
                        }
                    }
                    _ => info!("ignoring non-event frame in shadow stage"),
                },
                Err(e) => warn!(error = %e, "dropped malformed frame"),
            }
        }
        info!("stdin intake closed");
    })
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .init();

    let config_path = config_path_from_args();
    let cfg = match CoreConfig::load(&config_path) {
        Ok(c) => c,
        Err(e) => {
            error!(path = %config_path.display(), error = %e, "config load failed");
            return ExitCode::FAILURE;
        }
    };

    if !cfg.rust_core.enabled {
        info!("rust_core.enabled=false; supervisor inert, legacy runtime unaffected");
        return ExitCode::SUCCESS;
    }

    if let Err(e) = std::fs::create_dir_all(&cfg.rust_core.data_dir) {
        error!(error = %e, dir = %cfg.rust_core.data_dir.display(), "data dir unavailable");
        return ExitCode::FAILURE;
    }

    // Durability first: no journal, no supervisor.
    let journal = match JournalHandle::spawn(&cfg.journal_path()) {
        Ok(j) => j,
        Err(e) => {
            error!(error = %e, "journal unavailable; refusing to run without durability");
            return ExitCode::FAILURE;
        }
    };

    let mut scheduler = Scheduler::new();
    match journal.load_pending_obligations().await {
        Ok(loaded) => {
            let count = loaded.len();
            for obligation in loaded {
                if let Err(e) = scheduler.schedule(obligation) {
                    warn!(error = %e, "skipped corrupt persisted obligation");
                }
            }
            info!(restored = count, "obligations restored from journal");
        }
        Err(e) => warn!(error = %e, "obligation restore failed; starting empty"),
    }

    let (event_tx, event_rx) = mpsc::channel(cfg.rust_core.event_queue_capacity);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);

    let _intake = spawn_stdin_intake(event_tx, cfg.rust_core.max_payload_bytes);

    tokio::spawn(async move {
        match tokio::signal::ctrl_c().await {
            Ok(()) => {
                let _ = shutdown_tx.send(()).await;
            }
            Err(e) => warn!(error = %e, "ctrl-c listener failed"),
        }
    });

    let supervisor = Supervisor::new(cfg, journal, scheduler);
    supervisor.run(event_rx, shutdown_rx).await;

    info!("jarvis-core stopped cleanly");
    ExitCode::SUCCESS
}
