//! The internal attention clock: durable, wall-clock obligations re-checked
//! on a monotonic tick.
//!
//! Persisted `next_run_at_utc` survives restarts; `Instant`/tick only decide
//! when to look. Every missed-execution policy is explicit — a scheduler
//! restart after downtime can never release an unbounded or silent storm.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;
use time::{Duration, OffsetDateTime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissedPolicy {
    /// Fire once on recovery, then continue normally.
    RunOnceOnRecovery,
    /// Too old to be useful — skip and, if recurring, jump to the next slot.
    SkipIfStale,
    /// Move to the next future slot without firing the missed one.
    Reschedule,
    /// Park until the user explicitly confirms.
    RequireConfirmation,
}

impl MissedPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            MissedPolicy::RunOnceOnRecovery => "run_once_on_recovery",
            MissedPolicy::SkipIfStale => "skip_if_stale",
            MissedPolicy::Reschedule => "reschedule",
            MissedPolicy::RequireConfirmation => "require_confirmation",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        Some(match value {
            "run_once_on_recovery" => MissedPolicy::RunOnceOnRecovery,
            "skip_if_stale" => MissedPolicy::SkipIfStale,
            "reschedule" => MissedPolicy::Reschedule,
            "require_confirmation" => MissedPolicy::RequireConfirmation,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObligationStatus {
    Pending,
    AwaitingConfirmation,
    Done,
    Cancelled,
    Expired,
}

impl ObligationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ObligationStatus::Pending => "pending",
            ObligationStatus::AwaitingConfirmation => "awaiting_confirmation",
            ObligationStatus::Done => "done",
            ObligationStatus::Cancelled => "cancelled",
            ObligationStatus::Expired => "expired",
        }
    }

    pub fn from_db(value: &str) -> Option<Self> {
        Some(match value {
            "pending" => ObligationStatus::Pending,
            "awaiting_confirmation" => ObligationStatus::AwaitingConfirmation,
            "done" => ObligationStatus::Done,
            "cancelled" => ObligationStatus::Cancelled,
            "expired" => ObligationStatus::Expired,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Obligation {
    pub obligation_id: String,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at_utc: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    pub next_run_at_utc: OffsetDateTime,
    /// Who scheduled it (audit trail, not authority).
    pub origin: String,
    /// Authorization scope the run requires — checked by the policy engine.
    pub scope: String,
    pub priority: u8,
    /// Fixed-interval recurrence; `None` = one-shot. Cron parsing is a
    /// deliberate non-goal for the first slice.
    pub recurrence_seconds: Option<u64>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub expires_at_utc: Option<OffsetDateTime>,
    pub missed_policy: MissedPolicy,
    /// Outcome code from the last completed run (e.g. "ok", "timeout").
    pub last_result: Option<String>,
    pub status: ObligationStatus,
}

#[derive(Debug, Error)]
pub enum ScheduleError {
    #[error("obligation {0} field must not be empty")]
    EmptyField(&'static str),
    #[error("obligation {id} has recurrence under {min}s")]
    RecurrenceTooShort { id: String, min: u64 },
    #[error("obligation {0} is already scheduled")]
    Duplicate(String),
    #[error("unknown obligation {0}")]
    Unknown(String),
}

impl Obligation {
    pub fn validate(&self) -> Result<(), ScheduleError> {
        if self.obligation_id.is_empty() {
            return Err(ScheduleError::EmptyField("obligation_id"));
        }
        if self.origin.is_empty() {
            return Err(ScheduleError::EmptyField("origin"));
        }
        if self.scope.is_empty() {
            return Err(ScheduleError::EmptyField("scope"));
        }
        if let Some(interval) = self.recurrence_seconds {
            if interval < 5 {
                return Err(ScheduleError::RecurrenceTooShort {
                    id: self.obligation_id.clone(),
                    min: 5,
                });
            }
        }
        Ok(())
    }
}

/// A due obligation released by `collect_due`.
#[derive(Debug)]
pub struct DueItem {
    pub obligation: Obligation,
    pub late_by_seconds: i64,
    /// Idempotency key for the dispatch record — deterministic per
    /// (obligation, occurrence) so a restart redelivers without duplicating.
    pub operation_key: String,
}

/// What one tick decided about one obligation — journaled by the caller.
#[derive(Debug)]
pub enum TickOutcome {
    Due(DueItem),
    /// Parked; runs only after `confirm`.
    ConfirmationRequired(Obligation),
    /// Dropped permanently (expired, stale one-shot, etc.).
    Skipped {
        obligation: Obligation,
        reason: &'static str,
    },
    /// Re-inserted at a future slot (missed recurring obligation).
    Rescheduled(Obligation),
}

#[derive(Default)]
pub struct Scheduler {
    /// Keyed by (next_run_at_utc, id): ordered, unique, no duplicates.
    queue: BTreeMap<(OffsetDateTime, String), Obligation>,
}

/// Lateness beyond this point counts as "missed" rather than "on time".
const MISSED_GRACE: Duration = Duration::seconds(2);

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Insert a new or recovered obligation.
    pub fn schedule(&mut self, obligation: Obligation) -> Result<(), ScheduleError> {
        obligation.validate()?;
        let key = (obligation.next_run_at_utc, obligation.obligation_id.clone());
        if self.queue.contains_key(&key) {
            return Err(ScheduleError::Duplicate(obligation.obligation_id));
        }
        self.queue.insert(key, obligation);
        Ok(())
    }

    /// Re-arm an obligation on a new slot (user confirmation, retry).
    pub fn rearm(&mut self, obligation: Obligation) -> Result<(), ScheduleError> {
        obligation.validate()?;
        self.cancel(&obligation.obligation_id);
        self.queue.insert(
            (obligation.next_run_at_utc, obligation.obligation_id.clone()),
            obligation,
        );
        Ok(())
    }

    pub fn cancel(&mut self, id: &str) -> bool {
        let key = self
            .queue
            .iter()
            .find(|(_, o)| o.obligation_id == id)
            .map(|(k, _)| k.clone());
        key.map(|k| self.queue.remove(&k)).is_some()
    }

    pub fn get(&self, id: &str) -> Option<&Obligation> {
        self.queue.values().find(|o| o.obligation_id == id)
    }

    /// User approved a parked obligation — it becomes pending again and is
    /// re-armed at `now` so the missed-tick policy does not park it again.
    pub fn confirm(&mut self, id: &str, now: OffsetDateTime) -> Result<Obligation, ScheduleError> {
        let key = self
            .queue
            .iter()
            .find(|(_, o)| {
                o.obligation_id == id && o.status == ObligationStatus::AwaitingConfirmation
            })
            .map(|(k, _)| k.clone())
            .ok_or_else(|| ScheduleError::Unknown(id.to_string()))?;
        let mut obligation = self.queue.remove(&key).unwrap();
        obligation.status = ObligationStatus::Pending;
        obligation.next_run_at_utc = now;
        self.queue
            .insert((now, obligation.obligation_id.clone()), obligation.clone());
        Ok(obligation)
    }

    /// Release at most `max_burst` due obligations. Anything left stays
    /// queued for later ticks — no unbounded overdue storms.
    pub fn collect_due(&mut self, now: OffsetDateTime, max_burst: usize) -> Vec<TickOutcome> {
        let mut keys = Vec::new();
        let mut released = 0usize;
        for (key, obligation) in self.queue.iter() {
            if key.0 > now {
                break;
            }
            // Parked obligations are visible for inspection but never fire.
            if obligation.status != ObligationStatus::Pending {
                continue;
            }
            keys.push(key.clone());
            released += 1;
            if released >= max_burst.max(1) {
                break;
            }
        }

        let mut outcomes = Vec::with_capacity(keys.len());
        for key in keys {
            let mut obligation = match self.queue.remove(&key) {
                Some(o) => o,
                None => continue,
            };
            let late_by = (now - obligation.next_run_at_utc).whole_seconds();

            if let Some(expires) = obligation.expires_at_utc {
                if expires <= now {
                    obligation.status = ObligationStatus::Expired;
                    outcomes.push(TickOutcome::Skipped {
                        obligation,
                        reason: "expired",
                    });
                    continue;
                }
            }

            if Duration::seconds(late_by) <= MISSED_GRACE || late_by <= 0 {
                outcomes.push(TickOutcome::Due(DueItem {
                    operation_key: Self::operation_key(&obligation),
                    obligation,
                    late_by_seconds: late_by.max(0),
                }));
                continue;
            }

            match obligation.missed_policy {
                MissedPolicy::RunOnceOnRecovery => {
                    outcomes.push(TickOutcome::Due(DueItem {
                        operation_key: Self::operation_key(&obligation),
                        obligation,
                        late_by_seconds: late_by,
                    }));
                }
                MissedPolicy::RequireConfirmation => {
                    obligation.status = ObligationStatus::AwaitingConfirmation;
                    self.queue.insert(
                        (obligation.next_run_at_utc, obligation.obligation_id.clone()),
                        obligation.clone(),
                    );
                    outcomes.push(TickOutcome::ConfirmationRequired(obligation));
                }
                MissedPolicy::Reschedule | MissedPolicy::SkipIfStale => {
                    match obligation.recurrence_seconds {
                        Some(interval) => {
                            let next =
                                Self::next_future_slot(obligation.next_run_at_utc, interval, now);
                            obligation.next_run_at_utc = next;
                            self.queue.insert(
                                (next, obligation.obligation_id.clone()),
                                obligation.clone(),
                            );
                            outcomes.push(TickOutcome::Rescheduled(obligation));
                        }
                        None => {
                            let reason = match obligation.missed_policy {
                                MissedPolicy::SkipIfStale => "stale_missed",
                                _ => "missed_nonrecurring",
                            };
                            obligation.status = ObligationStatus::Done;
                            outcomes.push(TickOutcome::Skipped { obligation, reason });
                        }
                    }
                }
            }
        }
        outcomes
    }

    /// Record a finished run; recurring obligations re-enter at the next
    /// strictly-future slot. Returns the re-armed obligation if any.
    pub fn complete(
        &mut self,
        mut obligation: Obligation,
        result: &'static str,
        now: OffsetDateTime,
    ) -> Option<Obligation> {
        obligation.last_result = Some(result.to_string());
        match obligation.recurrence_seconds {
            Some(interval) => {
                let next = Self::next_future_slot(obligation.next_run_at_utc, interval, now);
                obligation.next_run_at_utc = next;
                obligation.status = ObligationStatus::Pending;
                self.queue
                    .insert((next, obligation.obligation_id.clone()), obligation.clone());
                Some(obligation)
            }
            None => {
                obligation.status = ObligationStatus::Done;
                None
            }
        }
    }

    /// Same missed-slot key before and after a restart — deterministic.
    fn operation_key(obligation: &Obligation) -> String {
        format!(
            "disp-{}-{}",
            obligation.obligation_id,
            obligation.next_run_at_utc.unix_timestamp()
        )
    }

    /// The first slot strictly after `now` on a fixed interval grid.
    /// Arithmetic skip — a long-dead obligation must not loop here.
    fn next_future_slot(
        from: OffsetDateTime,
        interval_seconds: u64,
        now: OffsetDateTime,
    ) -> OffsetDateTime {
        let secs = interval_seconds.max(1) as i64;
        let skips = (now - from).whole_seconds().max(0) / secs + 1;
        from + Duration::seconds(secs.saturating_mul(skips))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: i64) -> OffsetDateTime {
        OffsetDateTime::UNIX_EPOCH + Duration::seconds(seconds)
    }

    fn obl(id: &str, run_at: i64, recurrence: Option<u64>, missed: MissedPolicy) -> Obligation {
        Obligation {
            obligation_id: id.to_string(),
            created_at_utc: at(0),
            next_run_at_utc: at(run_at),
            origin: "test".into(),
            scope: "ops.obligation.run".into(),
            priority: 5,
            recurrence_seconds: recurrence,
            expires_at_utc: None,
            missed_policy: missed,
            last_result: None,
            status: ObligationStatus::Pending,
        }
    }

    #[test]
    fn due_items_release_in_time_order() {
        let mut s = Scheduler::new();
        s.schedule(obl("b", 30, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        s.schedule(obl("a", 10, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        s.schedule(obl("future", 9999, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        let outcomes = s.collect_due(at(40), 10);
        let ids: Vec<String> = outcomes
            .iter()
            .filter_map(|o| match o {
                TickOutcome::Due(d) => Some(d.obligation.obligation_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(ids, ["a", "b"]);
        assert_eq!(s.len(), 1);
    }

    #[test]
    fn burst_limit_holds_overflow_for_later_ticks() {
        let mut s = Scheduler::new();
        for i in 0..10 {
            s.schedule(obl(
                &format!("o{i}"),
                10,
                None,
                MissedPolicy::RunOnceOnRecovery,
            ))
            .unwrap();
        }
        let first = s.collect_due(at(20), 3);
        assert_eq!(first.len(), 3);
        assert_eq!(s.len(), 7);
        let second = s.collect_due(at(21), 10);
        assert_eq!(second.len(), 7);
    }

    #[test]
    fn missed_policy_skip_if_stale_reschedules_recurring() {
        let mut s = Scheduler::new();
        // Due at 10, missed until 100, interval 30 -> next slot 130.
        s.schedule(obl("r", 10, Some(30), MissedPolicy::SkipIfStale))
            .unwrap();
        let outcomes = s.collect_due(at(100), 5);
        match &outcomes[0] {
            TickOutcome::Rescheduled(o) => {
                assert_eq!(o.next_run_at_utc, at(130));
            }
            other => panic!("expected Rescheduled, got {other:?}"),
        }
        // It must not be due again immediately.
        assert!(s.collect_due(at(100), 5).is_empty());
    }

    #[test]
    fn missed_policy_require_confirmation_parks() {
        let mut s = Scheduler::new();
        s.schedule(obl("park", 10, None, MissedPolicy::RequireConfirmation))
            .unwrap();
        let outcomes = s.collect_due(at(500), 5);
        assert!(matches!(outcomes[0], TickOutcome::ConfirmationRequired(_)));
        // Parked: never due again on its own.
        assert!(s.collect_due(at(600), 5).is_empty());
        let revived = s.confirm("park", at(600)).unwrap();
        assert_eq!(revived.status, ObligationStatus::Pending);
        assert_eq!(revived.next_run_at_utc, at(600));
        // Now it fires on time rather than re-parking as missed.
        let again = s.collect_due(at(601), 5);
        assert!(matches!(again[0], TickOutcome::Due(_)));
    }

    #[test]
    fn expired_obligation_is_skipped_not_run() {
        let mut s = Scheduler::new();
        let mut o = obl("x", 10, None, MissedPolicy::RunOnceOnRecovery);
        o.expires_at_utc = Some(at(50));
        s.schedule(o).unwrap();
        let outcomes = s.collect_due(at(60), 5);
        assert!(matches!(
            &outcomes[0],
            TickOutcome::Skipped {
                reason: "expired",
                ..
            }
        ));
    }

    #[test]
    fn completion_rearms_recurring_and_closes_oneshot() {
        let mut s = Scheduler::new();
        s.schedule(obl("rec", 10, Some(30), MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        s.schedule(obl("once", 10, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        let outcomes = s.collect_due(at(10), 5);
        let items: Vec<Obligation> = outcomes
            .into_iter()
            .filter_map(|o| match o {
                TickOutcome::Due(d) => Some(d.obligation),
                _ => None,
            })
            .collect();
        for o in items {
            s.complete(o, "ok", at(11));
        }
        assert_eq!(s.len(), 1);
        assert_eq!(s.get("rec").unwrap().next_run_at_utc, at(40));
    }

    #[test]
    fn operation_key_is_stable_for_idempotent_redelivery() {
        let mut s = Scheduler::new();
        s.schedule(obl("k", 10, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        let key1 = match s.collect_due(at(20), 1).remove(0) {
            TickOutcome::Due(d) => d.operation_key,
            _ => panic!(),
        };
        // Simulate restart: the DB row was never completed, so it re-loads.
        let mut s2 = Scheduler::new();
        s2.schedule(obl("k", 10, None, MissedPolicy::RunOnceOnRecovery))
            .unwrap();
        let key2 = match s2.collect_due(at(25), 1).remove(0) {
            TickOutcome::Due(d) => d.operation_key,
            _ => panic!(),
        };
        assert_eq!(key1, key2);
    }
}
