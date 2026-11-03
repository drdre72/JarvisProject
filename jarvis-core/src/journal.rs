//! Durable operational journal — SQLite, one owning writer thread.
//!
//! Operational memory only: events, obligations, worker status, dispatch
//! attempts, action intents and results. Personal/semantic memory stays in
//! the Markdown vault (ADR-0004); the two are never merged. All writes are
//! transactional and acknowledged before the caller proceeds; idempotent
//! inserts make duplicate delivery after recovery safe (attempt ids and
//! event ids are primary keys — a redelivered write inserts nothing).
//!
//! The async runtime never touches `rusqlite::Connection`: commands travel
//! a bounded channel to a dedicated blocking thread ("one component
//! coordinates database transactions").

use crate::events::{EventClass, EventEnvelope};
use crate::scheduler::{MissedPolicy, Obligation, ObligationStatus};
use rusqlite::{params, Connection};
use std::path::Path;
use thiserror::Error;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};
use tokio::sync::{mpsc, oneshot};

const JOURNAL_SCHEMA_VERSION: i64 = 1;
const CHANNEL_CAPACITY: usize = 512;

#[derive(Debug, Error)]
pub enum JournalError {
    #[error("journal init failed: {0}")]
    Init(String),
    #[error("journal worker is closed")]
    Closed,
    #[error("journal operation failed: {0}")]
    Op(String),
    #[error("journal is corrupt: {0}")]
    Corrupt(String),
}

type Reply<T> = oneshot::Sender<Result<T, String>>;

enum Command {
    RecordEvent {
        envelope: Box<EventEnvelope>,
        class: EventClass,
        disposition: &'static str,
        reason: String,
        reply: Reply<bool>,
    },
    RecordCoalesced {
        envelope: Box<EventEnvelope>,
        class: EventClass,
        into_event_id: String,
        new_count: u32,
        reply: Reply<()>,
    },
    UpsertObligation {
        obligation: Box<Obligation>,
        reply: Reply<()>,
    },
    SetObligationStatus {
        obligation_id: String,
        status: ObligationStatus,
        last_result: Option<String>,
        reply: Reply<()>,
    },
    LoadPendingObligations {
        reply: Reply<Vec<Obligation>>,
    },
    RecordDispatch {
        attempt_id: String,
        event_id: String,
        intent_id: Option<String>,
        mode: &'static str,
        outcome: String,
        reply: Reply<bool>,
    },
    RecordIntent {
        intent_id: String,
        requester: String,
        verb: String,
        scope: String,
        decision: String,
        denial_reason: Option<String>,
        reply: Reply<bool>,
    },
    RecordResult {
        result_id: String,
        intent_id: String,
        outcome: String,
        verified: bool,
        detail: Option<String>,
        reply: Reply<bool>,
    },
    UpsertWorker {
        name: String,
        state: String,
        restart_count: u32,
        last_error: Option<String>,
        reply: Reply<()>,
    },
    PruneEventsBefore {
        cutoff: OffsetDateTime,
        reply: Reply<usize>,
    },
    Shutdown {
        reply: oneshot::Sender<()>,
    },
}

/// Cloneable handle to the journal actor.
#[derive(Clone)]
pub struct JournalHandle {
    tx: mpsc::Sender<Command>,
}

impl JournalHandle {
    /// Open (and migrate) the journal on its writer thread. Fails on a
    /// corrupt database rather than guessing at contents.
    pub fn spawn(path: &Path) -> Result<Self, JournalError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| JournalError::Init(e.to_string()))?;
        }
        let (tx, rx) = mpsc::channel(CHANNEL_CAPACITY);
        let path_buf = path.to_path_buf();
        let (init_tx, init_rx) = std::sync::mpsc::channel();
        std::thread::Builder::new()
            .name("journal".into())
            .spawn(move || {
                match Connection::open(&path_buf)
                    .map_err(|e| e.to_string())
                    .and_then(|conn| init_schema(&conn).map(|_| conn))
                {
                    Ok(conn) => {
                        let _ = init_tx.send(Ok(()));
                        run_writer(conn, rx);
                    }
                    Err(err) => {
                        let _ = init_tx.send(Err(err));
                    }
                }
            })
            .map_err(|e| JournalError::Init(e.to_string()))?;
        match init_rx.recv() {
            Ok(Ok(())) => Ok(Self { tx }),
            Ok(Err(err)) => Err(JournalError::Init(err)),
            Err(_) => Err(JournalError::Init("journal thread died".into())),
        }
    }

    async fn call<T, F>(&self, build: F) -> Result<T, JournalError>
    where
        F: FnOnce(Reply<T>) -> Command,
    {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(build(reply_tx))
            .await
            .map_err(|_| JournalError::Closed)?;
        reply_rx
            .await
            .map_err(|_| JournalError::Closed)?
            .map_err(JournalError::Op)
    }

    /// Insert the event record. Returns false when this `event_id` was
    /// already journaled — duplicate delivery is expected after recovery.
    pub async fn record_event(
        &self,
        envelope: EventEnvelope,
        class: EventClass,
        disposition: &'static str,
        reason: impl Into<String>,
    ) -> Result<bool, JournalError> {
        let reason = reason.into();
        self.call(move |reply| Command::RecordEvent {
            envelope: Box::new(envelope),
            class,
            disposition,
            reason,
            reply,
        })
        .await
    }

    /// Journal the delivery *and* bump the surviving entry's count in one
    /// transaction — half-recorded coalescing is not a state.
    pub async fn record_coalesced(
        &self,
        envelope: EventEnvelope,
        class: EventClass,
        into_event_id: String,
        new_count: u32,
    ) -> Result<(), JournalError> {
        self.call(move |reply| Command::RecordCoalesced {
            envelope: Box::new(envelope),
            class,
            into_event_id,
            new_count,
            reply,
        })
        .await
    }

    pub async fn upsert_obligation(&self, obligation: Obligation) -> Result<(), JournalError> {
        self.call(move |reply| Command::UpsertObligation {
            obligation: Box::new(obligation),
            reply,
        })
        .await
    }

    pub async fn set_obligation_status(
        &self,
        obligation_id: &str,
        status: ObligationStatus,
        last_result: Option<String>,
    ) -> Result<(), JournalError> {
        let obligation_id = obligation_id.to_string();
        self.call(move |reply| Command::SetObligationStatus {
            obligation_id,
            status,
            last_result,
            reply,
        })
        .await
    }

    /// Pending + parked obligations; the only recovery read the scheduler
    /// needs. Completed/cancelled rows stay for audit, not re-arming.
    pub async fn load_pending_obligations(&self) -> Result<Vec<Obligation>, JournalError> {
        self.call(|reply| Command::LoadPendingObligations { reply })
            .await
    }

    /// Idempotent: returns false when `attempt_id` already exists — a
    /// redelivered due-obligation must not claim a second dispatch.
    pub async fn record_dispatch(
        &self,
        attempt_id: String,
        event_id: String,
        intent_id: Option<String>,
        mode: &'static str,
        outcome: impl Into<String>,
    ) -> Result<bool, JournalError> {
        let outcome = outcome.into();
        self.call(move |reply| Command::RecordDispatch {
            attempt_id,
            event_id,
            intent_id,
            mode,
            outcome,
            reply,
        })
        .await
    }

    pub async fn record_intent(
        &self,
        intent_id: String,
        requester: String,
        verb: String,
        scope: String,
        decision: String,
        denial_reason: Option<String>,
    ) -> Result<bool, JournalError> {
        self.call(move |reply| Command::RecordIntent {
            intent_id,
            requester,
            verb,
            scope,
            decision,
            denial_reason,
            reply,
        })
        .await
    }

    /// `verified` separates "produced output" from "confirmed outcome" — a
    /// timeout after dispatch records `unknown`, never an assumed success.
    pub async fn record_result(
        &self,
        result_id: String,
        intent_id: String,
        outcome: String,
        verified: bool,
        detail: Option<String>,
    ) -> Result<bool, JournalError> {
        self.call(move |reply| Command::RecordResult {
            result_id,
            intent_id,
            outcome,
            verified,
            detail,
            reply,
        })
        .await
    }

    pub async fn upsert_worker(
        &self,
        name: &str,
        state: &str,
        restart_count: u32,
        last_error: Option<String>,
    ) -> Result<(), JournalError> {
        let name = name.to_string();
        let state = state.to_string();
        self.call(move |reply| Command::UpsertWorker {
            name,
            state,
            restart_count,
            last_error,
            reply,
        })
        .await
    }

    /// Retention: drop old *telemetry-disposition* rows only. Obligations,
    /// dispatches, intents and results are never silently aged out.
    pub async fn prune_events_before(&self, cutoff: OffsetDateTime) -> Result<usize, JournalError> {
        self.call(move |reply| Command::PruneEventsBefore { cutoff, reply })
            .await
    }

    /// Flush and stop the writer thread. The FIFO channel guarantees every
    /// queued command commits before the writer acks shutdown.
    pub async fn shutdown(self) {
        let (done_tx, done_rx) = oneshot::channel();
        if self
            .tx
            .send(Command::Shutdown { reply: done_tx })
            .await
            .is_ok()
        {
            let _ = done_rx.await;
        }
    }
}

fn init_schema(conn: &Connection) -> Result<(), String> {
    // WAL for read/write concurrency; FULL sync so committed obligations
    // survive process death; named pragma values (not quoted strings).
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = FULL;
         PRAGMA foreign_keys = ON;",
    )
    .map_err(|e| e.to_string())?;

    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if integrity != "ok" {
        return Err(format!("integrity_check reported {integrity}"));
    }

    let version: i64 = conn
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    if version < JOURNAL_SCHEMA_VERSION {
        conn.execute_batch(
            "BEGIN;
             CREATE TABLE IF NOT EXISTS schema_migrations (
                 version INTEGER PRIMARY KEY,
                 applied_at_utc TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS events (
                 event_id TEXT PRIMARY KEY,
                 trace_id TEXT NOT NULL,
                 source TEXT NOT NULL,
                 event_type TEXT NOT NULL,
                 class TEXT NOT NULL,
                 occurred_at_utc TEXT NOT NULL,
                 received_at_utc TEXT NOT NULL,
                 disposition TEXT NOT NULL,
                 reason TEXT NOT NULL,
                 coalesced_count INTEGER NOT NULL DEFAULT 0,
                 schema_version INTEGER NOT NULL,
                 payload_json TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS scheduled_obligations (
                 obligation_id TEXT PRIMARY KEY,
                 created_at_utc TEXT NOT NULL,
                 next_run_at_utc TEXT NOT NULL,
                 origin TEXT NOT NULL,
                 scope TEXT NOT NULL,
                 priority INTEGER NOT NULL,
                 recurrence_seconds INTEGER,
                 expires_at_utc TEXT,
                 missed_policy TEXT NOT NULL,
                 last_result TEXT,
                 status TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS worker_status (
                 worker_name TEXT PRIMARY KEY,
                 state TEXT NOT NULL,
                 last_heartbeat_utc TEXT,
                 restart_count INTEGER NOT NULL DEFAULT 0,
                 last_error TEXT,
                 updated_at_utc TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS dispatch_attempts (
                 attempt_id TEXT PRIMARY KEY,
                 event_id TEXT NOT NULL,
                 intent_id TEXT,
                 dispatched_at_utc TEXT NOT NULL,
                 mode TEXT NOT NULL,
                 outcome TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS action_intents (
                 intent_id TEXT PRIMARY KEY,
                 requester TEXT NOT NULL,
                 verb TEXT NOT NULL,
                 scope TEXT NOT NULL,
                 decision TEXT NOT NULL,
                 denial_reason TEXT,
                 created_at_utc TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS action_results (
                 result_id TEXT PRIMARY KEY,
                 intent_id TEXT NOT NULL,
                 completed_at_utc TEXT NOT NULL,
                 outcome TEXT NOT NULL,
                 verified INTEGER NOT NULL,
                 detail TEXT
             );
             INSERT INTO schema_migrations (version, applied_at_utc)
                 VALUES (1, datetime('now'));
             COMMIT;",
        )
        .map_err(|e| e.to_string())?;
        conn.pragma_update(None, "user_version", JOURNAL_SCHEMA_VERSION)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn to_rfc3339(t: &OffsetDateTime) -> String {
    t.format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".into())
}

fn from_rfc3339(s: &str) -> Result<OffsetDateTime, String> {
    OffsetDateTime::parse(s, &Rfc3339).map_err(|e| e.to_string())
}

fn insert_event_row(
    conn: &Connection,
    env: &EventEnvelope,
    class: EventClass,
    disposition: &str,
    reason: &str,
    coalesced: u32,
) -> Result<bool, String> {
    let payload = serde_json::to_string(&env.payload).map_err(|e| e.to_string())?;
    let source = serde_json::to_string(&env.source).map_err(|e| e.to_string())?;
    let event_type = serde_json::to_string(&env.event_type).map_err(|e| e.to_string())?;
    let rows = conn
        .execute(
            "INSERT OR IGNORE INTO events (
                 event_id, trace_id, source, event_type, class,
                 occurred_at_utc, received_at_utc, disposition, reason,
                 coalesced_count, schema_version, payload_json
             ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                env.event_id,
                env.trace_id,
                source.trim_matches('"'),
                event_type.trim_matches('"'),
                class.as_str(),
                to_rfc3339(&env.occurred_at_utc),
                now_rfc3339(),
                disposition,
                reason,
                coalesced,
                env.schema_version,
                payload,
            ],
        )
        .map_err(|e| e.to_string())?;
    Ok(rows > 0)
}

fn upsert_obligation_row(conn: &Connection, o: &Obligation) -> Result<(), String> {
    conn.execute(
        "INSERT INTO scheduled_obligations (
             obligation_id, created_at_utc, next_run_at_utc, origin, scope,
             priority, recurrence_seconds, expires_at_utc, missed_policy,
             last_result, status
         ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(obligation_id) DO UPDATE SET
             next_run_at_utc = excluded.next_run_at_utc,
             priority = excluded.priority,
             recurrence_seconds = excluded.recurrence_seconds,
             expires_at_utc = excluded.expires_at_utc,
             missed_policy = excluded.missed_policy,
             last_result = excluded.last_result,
             status = excluded.status",
        params![
            o.obligation_id,
            to_rfc3339(&o.created_at_utc),
            to_rfc3339(&o.next_run_at_utc),
            o.origin,
            o.scope,
            o.priority,
            o.recurrence_seconds.map(|v| v as i64),
            o.expires_at_utc.as_ref().map(to_rfc3339),
            o.missed_policy.as_str(),
            o.last_result,
            o.status.as_str(),
        ],
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn load_obligations(conn: &Connection) -> Result<Vec<Obligation>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT obligation_id, created_at_utc, next_run_at_utc, origin, scope,
                    priority, recurrence_seconds, expires_at_utc, missed_policy,
                    last_result, status
             FROM scheduled_obligations
             WHERE status IN ('pending', 'awaiting_confirmation')
             ORDER BY next_run_at_utc",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, Option<i64>>(6)?,
                row.get::<_, Option<String>>(7)?,
                row.get::<_, String>(8)?,
                row.get::<_, Option<String>>(9)?,
                row.get::<_, String>(10)?,
            ))
        })
        .map_err(|e| e.to_string())?;

    let mut out = Vec::new();
    for row in rows {
        let (
            id,
            created,
            next_run,
            origin,
            scope,
            priority,
            recurrence,
            expires,
            missed,
            last_result,
            status,
        ) = row.map_err(|e| e.to_string())?;
        out.push(Obligation {
            obligation_id: id,
            created_at_utc: from_rfc3339(&created)?,
            next_run_at_utc: from_rfc3339(&next_run)?,
            origin,
            scope,
            priority: priority.clamp(0, 255) as u8,
            recurrence_seconds: recurrence.map(|v| v.max(0) as u64),
            expires_at_utc: expires.map(|s| from_rfc3339(&s)).transpose()?,
            missed_policy: MissedPolicy::from_db(&missed)
                .ok_or_else(|| format!("bad missed_policy {missed}"))?,
            last_result,
            status: ObligationStatus::from_db(&status)
                .ok_or_else(|| format!("bad status {status}"))?,
        });
    }
    Ok(out)
}

fn run_writer(mut conn: Connection, mut rx: mpsc::Receiver<Command>) {
    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            Command::RecordEvent {
                envelope,
                class,
                disposition,
                reason,
                reply,
            } => {
                let _ = reply.send(insert_event_row(
                    &conn,
                    &envelope,
                    class,
                    disposition,
                    &reason,
                    0,
                ));
            }
            Command::RecordCoalesced {
                envelope,
                class,
                into_event_id,
                new_count,
                reply,
            } => {
                let result = conn
                    .transaction()
                    .map_err(|e| e.to_string())
                    .and_then(|tx| {
                        insert_event_row(&tx, &envelope, class, "coalesced", &into_event_id, 0)?;
                        tx.execute(
                            "UPDATE events SET coalesced_count = ?1 WHERE event_id = ?2",
                            params![new_count, into_event_id],
                        )
                        .map_err(|e| e.to_string())?;
                        tx.commit().map_err(|e| e.to_string())
                    });
                let _ = reply.send(result);
            }
            Command::UpsertObligation { obligation, reply } => {
                let _ = reply.send(upsert_obligation_row(&conn, &obligation));
            }
            Command::SetObligationStatus {
                obligation_id,
                status,
                last_result,
                reply,
            } => {
                let r = conn
                    .execute(
                        "UPDATE scheduled_obligations
                         SET status = ?1, last_result = COALESCE(?2, last_result)
                         WHERE obligation_id = ?3",
                        params![status.as_str(), last_result, obligation_id],
                    )
                    .map_err(|e| e.to_string())
                    .map(|_| ());
                let _ = reply.send(r);
            }
            Command::LoadPendingObligations { reply } => {
                let _ = reply.send(load_obligations(&conn));
            }
            Command::RecordDispatch {
                attempt_id,
                event_id,
                intent_id,
                mode,
                outcome,
                reply,
            } => {
                let r = conn
                    .execute(
                        "INSERT OR IGNORE INTO dispatch_attempts (
                             attempt_id, event_id, intent_id,
                             dispatched_at_utc, mode, outcome
                         ) VALUES (?1,?2,?3,?4,?5,?6)",
                        params![
                            attempt_id,
                            event_id,
                            intent_id,
                            now_rfc3339(),
                            mode,
                            outcome
                        ],
                    )
                    .map_err(|e| e.to_string())
                    .map(|rows| rows > 0);
                let _ = reply.send(r);
            }
            Command::RecordIntent {
                intent_id,
                requester,
                verb,
                scope,
                decision,
                denial_reason,
                reply,
            } => {
                let r = conn
                    .execute(
                        "INSERT OR IGNORE INTO action_intents (
                             intent_id, requester, verb, scope, decision,
                             denial_reason, created_at_utc
                         ) VALUES (?1,?2,?3,?4,?5,?6,?7)",
                        params![
                            intent_id,
                            requester,
                            verb,
                            scope,
                            decision,
                            denial_reason,
                            now_rfc3339()
                        ],
                    )
                    .map_err(|e| e.to_string())
                    .map(|rows| rows > 0);
                let _ = reply.send(r);
            }
            Command::RecordResult {
                result_id,
                intent_id,
                outcome,
                verified,
                detail,
                reply,
            } => {
                let r = conn
                    .execute(
                        "INSERT OR IGNORE INTO action_results (
                             result_id, intent_id, completed_at_utc,
                             outcome, verified, detail
                         ) VALUES (?1,?2,?3,?4,?5,?6)",
                        params![
                            result_id,
                            intent_id,
                            now_rfc3339(),
                            outcome,
                            verified,
                            detail
                        ],
                    )
                    .map_err(|e| e.to_string())
                    .map(|rows| rows > 0);
                let _ = reply.send(r);
            }
            Command::UpsertWorker {
                name,
                state,
                restart_count,
                last_error,
                reply,
            } => {
                let r = conn
                    .execute(
                        "INSERT INTO worker_status (
                             worker_name, state, last_heartbeat_utc,
                             restart_count, last_error, updated_at_utc
                         ) VALUES (?1,?2,?3,?4,?5,?6)
                         ON CONFLICT(worker_name) DO UPDATE SET
                             state = excluded.state,
                             restart_count = excluded.restart_count,
                             last_error = excluded.last_error,
                             updated_at_utc = excluded.updated_at_utc",
                        params![
                            name,
                            state,
                            now_rfc3339(),
                            restart_count,
                            last_error,
                            now_rfc3339()
                        ],
                    )
                    .map_err(|e| e.to_string())
                    .map(|_| ());
                let _ = reply.send(r);
            }
            Command::PruneEventsBefore { cutoff, reply } => {
                let r = conn
                    .execute(
                        "DELETE FROM events
                         WHERE received_at_utc < ?1
                           AND class = 'diagnostic_telemetry'",
                        params![to_rfc3339(&cutoff)],
                    )
                    .map_err(|e| e.to_string());
                let _ = reply.send(r);
            }
            Command::Shutdown { reply } => {
                // All prior commands already committed (FIFO channel).
                let _ = reply.send(());
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventSource, EventType};
    use serde_json::json;
    use tempfile::tempdir;
    use time::Duration;

    fn obl(id: &str, status: ObligationStatus) -> Obligation {
        Obligation {
            obligation_id: id.into(),
            created_at_utc: OffsetDateTime::now_utc(),
            next_run_at_utc: OffsetDateTime::now_utc() + Duration::minutes(5),
            origin: "test".into(),
            scope: "ops.obligation.run".into(),
            priority: 5,
            recurrence_seconds: Some(60),
            expires_at_utc: None,
            missed_policy: MissedPolicy::RunOnceOnRecovery,
            last_result: None,
            status,
        }
    }

    #[tokio::test]
    async fn obligations_survive_restart() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("journal.db");
        {
            let j = JournalHandle::spawn(&path).unwrap();
            j.upsert_obligation(obl("keep-me", ObligationStatus::Pending))
                .await
                .unwrap();
            j.upsert_obligation(obl("dead", ObligationStatus::Done))
                .await
                .unwrap();
            j.shutdown().await;
        }
        // "Restart" — a fresh handle on the same file.
        let j = JournalHandle::spawn(&path).unwrap();
        let pending = j.load_pending_obligations().await.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].obligation_id, "keep-me");
        j.shutdown().await;
    }

    #[tokio::test]
    async fn duplicate_dispatch_is_suppressed() {
        let dir = tempdir().unwrap();
        let j = JournalHandle::spawn(&dir.path().join("j.db")).unwrap();
        assert!(j
            .record_dispatch("a1".into(), "e1".into(), None, "shadow", "ok")
            .await
            .unwrap());
        // Redelivery of the same attempt id inserts nothing.
        assert!(!j
            .record_dispatch("a1".into(), "e1".into(), None, "shadow", "ok")
            .await
            .unwrap());
        j.shutdown().await;
    }

    #[tokio::test]
    async fn coalesced_update_is_transactional() {
        let dir = tempdir().unwrap();
        let j = JournalHandle::spawn(&dir.path().join("j.db")).unwrap();
        let e1 = EventEnvelope::create(
            EventSource::WakeWord,
            EventType::WakeWordDetected,
            Some("wake".into()),
            json!({}),
        );
        let target = e1.event_id.clone();
        j.record_event(e1, EventClass::UserInterrupt, "accepted", "")
            .await
            .unwrap();
        let dup = EventEnvelope::create(
            EventSource::WakeWord,
            EventType::WakeWordDetected,
            Some("wake".into()),
            json!({}),
        );
        j.record_coalesced(dup, EventClass::UserInterrupt, target, 2)
            .await
            .unwrap();
        j.shutdown().await;
    }
}
