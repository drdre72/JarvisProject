//! End-to-end shadow-stage check: durable obligation recovery, event
//! admission, idempotent dispatch, graceful shutdown — all journaled,
//! nothing consequential executed.

use jarvis_core::config::CoreConfig;
use jarvis_core::events::{EventEnvelope, EventSource, EventType};
use jarvis_core::journal::JournalHandle;
use jarvis_core::scheduler::{MissedPolicy, Obligation, ObligationStatus, Scheduler};
use jarvis_core::supervisor::Supervisor;
use jarvis_core::transport::{FrameBody, InboundRequest, WireFrame};
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

    let (outbound_tx, _outbound_rx) = mpsc::channel::<WireFrame>(8);
    let supervisor = Supervisor::new(
        test_config(dir.path()),
        journal.clone(),
        scheduler,
        outbound_tx,
    );
    let (event_tx, event_rx) = mpsc::channel(64);
    let (_request_tx, request_rx) = mpsc::channel(8);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, request_rx, shutdown_rx));

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
    let (outbound_tx, _outbound_rx) = mpsc::channel::<WireFrame>(8);
    let supervisor = Supervisor::new(
        test_config(dir.path()),
        journal,
        Scheduler::new(),
        outbound_tx,
    );
    let (event_tx, event_rx) = mpsc::channel::<EventEnvelope>(8);
    let (_request_tx, request_rx) = mpsc::channel(8);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, request_rx, shutdown_rx));

    drop(event_tx); // intake gone — supervisor must not die with it
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    assert!(!task.is_finished());

    shutdown_tx.send(()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("supervisor ignored shutdown")
        .unwrap();
}

/// Stage B over the wire: a schedule request is validated, journaled, then
/// armed; the due run still passes policy evaluation and shadows (never
/// dispatches). A flag-off peer gets `ipc_disabled`, proving rollback.
#[tokio::test]
async fn schedule_over_wire_runs_through_policy_and_journal() {
    let dir = tempdir().unwrap();
    let db = dir.path().join("j.db");
    let mut cfg = test_config(dir.path());
    cfg.rust_core.ipc_requests = true;
    let journal = JournalHandle::spawn(&db).unwrap();
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<WireFrame>(8);
    let supervisor = Supervisor::new(cfg, journal, Scheduler::new(), outbound_tx);
    let (_event_tx, event_rx) = mpsc::channel::<EventEnvelope>(8);
    let (request_tx, request_rx) = mpsc::channel(8);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, request_rx, shutdown_rx));

    let obligation = json!({
        "obligation_id": "ob-wire-1",
        "created_at_utc": OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        "next_run_at_utc": (OffsetDateTime::now_utc() + Duration::seconds(1))
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap(),
        "origin": "python-worker",
        "scope": "ops.obligation.run",
        "priority": 5,
        "recurrence_seconds": null,
        "expires_at_utc": null,
        "missed_policy": "run_once_on_recovery",
        "last_result": "tampered",
        "status": "done"
    });
    request_tx
        .send(InboundRequest {
            request_id: "req-1".into(),
            op: "schedule_obligation".into(),
            args: obligation,
        })
        .await
        .unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(3), outbound_rx.recv())
        .await
        .expect("no response frame")
        .expect("outbound closed");
    match frame.body {
        FrameBody::Response {
            request_id,
            ok,
            body,
        } => {
            assert_eq!(request_id, "req-1");
            assert!(ok, "schedule rejected: {body}");
        }
        other => panic!("expected response, got {other:?}"),
    }

    // Wait for the due run to release and complete in shadow.
    tokio::time::sleep(std::time::Duration::from_millis(2500)).await;
    shutdown_tx.send(()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("supervisor ignored shutdown")
        .unwrap();

    let conn = rusqlite::Connection::open(&db).unwrap();
    let (status, last_result): (String, Option<String>) = conn
        .query_row(
            "SELECT status, last_result FROM scheduled_obligations WHERE obligation_id='ob-wire-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(status, "done");
    assert_eq!(last_result.as_deref(), Some("shadow_evaluated"));
    let intents: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM action_intents WHERE decision='allow'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(intents >= 1, "due run passed policy evaluation");
}

#[tokio::test]
async fn ipc_disabled_flag_rolls_back_requests() {
    let dir = tempdir().unwrap();
    let journal = JournalHandle::spawn(&dir.path().join("j.db")).unwrap();
    let (outbound_tx, mut outbound_rx) = mpsc::channel::<WireFrame>(8);
    let supervisor = Supervisor::new(
        test_config(dir.path()),
        journal,
        Scheduler::new(),
        outbound_tx,
    );
    let (_event_tx, event_rx) = mpsc::channel::<EventEnvelope>(8);
    let (request_tx, request_rx) = mpsc::channel(8);
    let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
    let task = tokio::spawn(supervisor.run(event_rx, request_rx, shutdown_rx));

    request_tx
        .send(InboundRequest {
            request_id: "req-off".into(),
            op: "schedule_obligation".into(),
            args: json!({}),
        })
        .await
        .unwrap();
    let frame = tokio::time::timeout(std::time::Duration::from_secs(3), outbound_rx.recv())
        .await
        .expect("no response frame")
        .expect("outbound closed");
    match frame.body {
        FrameBody::Response { ok, body, .. } => {
            assert!(!ok);
            assert_eq!(body["error"], "ipc_disabled");
        }
        other => panic!("expected response, got {other:?}"),
    }
    shutdown_tx.send(()).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), task)
        .await
        .expect("supervisor ignored shutdown")
        .unwrap();
}
