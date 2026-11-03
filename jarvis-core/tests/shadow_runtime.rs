//! End-to-end shadow-stage check: durable obligation recovery, event
//! admission, idempotent dispatch, graceful shutdown — all journaled,
//! nothing consequential executed.

use jarvis_core::config::CoreConfig;
use jarvis_core::events::{EventEnvelope, EventSource, EventType};
use jarvis_core::journal::JournalHandle;
use jarvis_core::scheduler::{MissedPolicy, Obligation, ObligationStatus, Scheduler};
use jarvis_core::supervisor::Supervisor;
use serde_json::json;
use std::path::Path;
use tempfile::tempdir;
use time::{Duration, OffsetDateTime};
use tokio::sync::mpsc;

fn test_config(dir: &Path) -> CoreConfig {
    let mut cfg = CoreConfig::default();
    cfg.rust_core.enabled = true;
    cfg.rust_core.shadow_mode = true;
    cfg.rust_core.data_dir = dir.to_path_buf();
    cfg.rust_core.event_queue_capacity = 64;
    cfg.scheduler.tick_seconds = 1;
    cfg
}

fn past_due(id: &str) -> Obligation {
    Obligation {
        obligation_id: id.into(),
        created_at_utc: OffsetDateTime::now_utc() - Duration::minutes(10),
        next_run_at_utc: OffsetDateTime::now_utc() - Duration::seconds(60),
        origin: "test".into(),
        scope: "ops.obligation.run".into(),
        priority: 5,
        recurrence_seconds: None,
        expires_at_utc: None,
        missed_policy: MissedPolicy::RunOnceOnRecovery,
        last_result: None,
        status: ObligationStatus::Pending,
    }
}

#[tokio::test]
async fn obligation_survives_restart_and_completes_in_shadow() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("journal.db");

    // "Boot 1": persist a due-in-the-past obligation, then die.
    {
        let journal = JournalHandle::spawn(&db).unwrap();
        journal
            .upsert_obligation(past_due("check-in"))
            .await
            .unwrap();
        journal.shutdown().await;
    }

    // "Boot 2": restore and run the supervisor.
    let journal = JournalHandle::spawn(&db).unwrap();
    let loaded = journal.load_pending_obligations().await.unwrap();
    assert_eq!(loaded.len(), 1);
    let mut scheduler = Scheduler::new();
    for o in loaded {
        scheduler.schedule(o).unwrap();
    }

    let supervisor = Supervisor::new(test_config(dir.path()), journal.clone(), scheduler);
    let (event_tx, event_rx) = mpsc::channel(64);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, shutdown_rx));

    // Let at least one scheduler tick release the obligation.
    tokio::time::sleep(std::time::Duration::from_millis(1600)).await;

    // A user event still flows through the same pipeline.
    event_tx
        .send(EventEnvelope::create(
            EventSource::Ui,
            EventType::UserRequestReceived,
            None,
            json!({"text": "status?"}),
        ))
        .await
        .unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    shutdown_tx.send(()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("supervisor did not stop within 5s")
        .unwrap();

    // The obligation completed once: no pending rows, one dispatch record.
    let journal = JournalHandle::spawn(&db).unwrap();
    let pending = journal.load_pending_obligations().await.unwrap();
    assert!(
        pending.is_empty(),
        "completed obligation should not reload as pending"
    );
    journal.shutdown().await;

    let conn = rusqlite::Connection::open(&db).unwrap();
    let dispatches: i64 = conn
        .query_row("SELECT COUNT(*) FROM dispatch_attempts", [], |r| r.get(0))
        .unwrap();
    assert!(dispatches >= 1, "expected at least one journaled dispatch");
    let events: i64 = conn
        .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
        .unwrap();
    assert!(events >= 1, "expected journaled events");
    let intents: i64 = conn
        .query_row("SELECT COUNT(*) FROM action_intents", [], |r| r.get(0))
        .unwrap();
    assert!(intents >= 1, "policy evaluation should be journaled");
}

#[tokio::test]
async fn supervisor_outlives_closed_intake_and_stops_on_signal() {
    let dir = tempdir().unwrap();
    let journal = JournalHandle::spawn(&dir.path().join("j.db")).unwrap();
    let supervisor = Supervisor::new(test_config(dir.path()), journal, Scheduler::new());
    let (event_tx, event_rx) = mpsc::channel::<EventEnvelope>(8);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, shutdown_rx));

    drop(event_tx); // intake gone — supervisor must not die with it
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(!task.is_finished());

    shutdown_tx.send(()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("supervisor ignored shutdown")
        .unwrap();
}
