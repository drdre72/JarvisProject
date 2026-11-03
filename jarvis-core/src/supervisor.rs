//! The supervisor: single owner of cognitive state, event queue, scheduler,
//! journal, and policy engine, coordinated through typed messages on
//! bounded channels.
//!
//! Stage A (shadow): every verdict and decision is journaled; nothing
//! consequential is dispatched. The event flow is real — admission,
//! coalescing, prioritization, attention rules, obligation release,
//! policy evaluation, idempotent dispatch records — so Stage B can flip
//! behavior, not plumbing.

use crate::attention::{Attention, AttentionEvaluator};
use crate::config::CoreConfig;
use crate::events::{
    BoundedEventQueue, Disposition, EventClass, EventEnvelope, EventSource, EventType, QueuedEvent,
};
use crate::health::ResourceSnapshot;
use crate::journal::{JournalError, JournalHandle};
use crate::policy::{ActionRequest, Decision, PolicyEngine, Requester};
use crate::scheduler::{DueItem, Obligation, ObligationStatus, Scheduler, TickOutcome};
use crate::state::{CognitiveMachine, CognitiveState};
use serde_json::json;
use std::future::Future;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub struct Supervisor {
    cfg: CoreConfig,
    machine: CognitiveMachine,
    queue: BoundedEventQueue,
    evaluator: AttentionEvaluator,
    scheduler: Scheduler,
    journal: JournalHandle,
    policy: PolicyEngine,
    /// Consecutive journal write failures — durability problems degrade the
    /// machine honestly rather than hiding.
    journal_failures: u32,
}

impl Supervisor {
    pub fn new(cfg: CoreConfig, journal: JournalHandle, scheduler: Scheduler) -> Self {
        let queue = BoundedEventQueue::new(
            cfg.rust_core.event_queue_capacity,
            Duration::from_secs(cfg.rust_core.dedup_window_seconds),
            cfg.rust_core.per_source_burst,
        );
        Self {
            cfg,
            machine: CognitiveMachine::new(),
            queue,
            evaluator: AttentionEvaluator::default(),
            scheduler,
            journal,
            policy: PolicyEngine::new(),
            journal_failures: 0,
        }
    }

    pub fn state(&self) -> CognitiveState {
        self.machine.state()
    }

    /// Await an outcome from the journal actor without letting a failed
    /// write panic the loop. Repeated failures degrade the machine.
    async fn j<T>(&mut self, fut: impl Future<Output = Result<T, JournalError>>) -> Option<T> {
        match fut.await {
            Ok(v) => {
                self.journal_failures = 0;
                Some(v)
            }
            Err(e) => {
                self.journal_failures += 1;
                warn!(error = %e, "journal operation failed");
                if self.journal_failures >= 3 {
                    let _ = self
                        .machine
                        .transition(CognitiveState::Degraded, "journal_failures");
                }
                None
            }
        }
    }

    fn snapshot(&self) -> ResourceSnapshot {
        ResourceSnapshot {
            cpu_percent: None,
            memory_percent: None,
            queue_depth: self.queue.len(),
            queue_capacity: self.cfg.rust_core.event_queue_capacity,
            captured_at: OffsetDateTime::now_utc(),
        }
    }

    pub async fn run(
        mut self,
        mut intake: mpsc::Receiver<EventEnvelope>,
        mut shutdown: mpsc::Receiver<()>,
    ) {
        let mut tick =
            tokio::time::interval(Duration::from_secs(self.cfg.scheduler.tick_seconds.max(1)));
        // A busy tick skips rather than bursts — catch-up work is bounded
        // by collect_due, not by tokio.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut intake_open = true;
        info!(
            shadow = self.cfg.rust_core.shadow_mode,
            pending_obligations = self.scheduler.len(),
            "supervisor monitoring"
        );
        loop {
            tokio::select! {
                biased;
                _ = shutdown.recv() => break,
                _ = tick.tick() => self.on_tick().await,
                maybe = intake.recv(), if intake_open => match maybe {
                    Some(env) => self.on_event(env).await,
                    None => {
                        intake_open = false;
                        info!("event intake closed; supervisor continues on timers");
                    }
                },
            }
        }
        self.shutdown().await;
    }

    async fn on_event(&mut self, env: EventEnvelope) {
        let class = BoundedEventQueue::class_of(&env);
        let admission = self.queue.admit(env.clone(), Instant::now());
        match &admission.disposition {
            Disposition::Accepted => {
                let j = self.journal.clone();
                self.j(async move { j.record_event(env, class, "accepted", "").await })
                    .await;
            }
            Disposition::Coalesced { into_event_id } => {
                let new_count = self.queue.coalesced_count(into_event_id).unwrap_or(1);
                let j = self.journal.clone();
                let into = into_event_id.clone();
                self.j(async move { j.record_coalesced(env, class, into, new_count).await })
                    .await;
            }
            Disposition::Rejected { reason } => {
                let j = self.journal.clone();
                self.j(async move { j.record_event(env, class, "rejected", *reason).await })
                    .await;
            }
        }
        if let Some(evicted) = admission.evicted {
            let j = self.journal.clone();
            self.j(async move {
                j.record_event(evicted.envelope, evicted.class, "evicted", "lower_priority")
                    .await
            })
            .await;
        }
        self.drain_queue().await;
    }

    async fn on_tick(&mut self) {
        let now = OffsetDateTime::now_utc();
        let outcomes = self
            .scheduler
            .collect_due(now, self.cfg.scheduler.max_due_per_tick);
        for outcome in outcomes {
            match outcome {
                TickOutcome::Due(item) => self.emit_due_event(item, now).await,
                TickOutcome::ConfirmationRequired(obligation) => {
                    info!(
                        obligation = %obligation.obligation_id,
                        "obligation parked for user confirmation"
                    );
                    let j = self.journal.clone();
                    self.j(async move { j.upsert_obligation(obligation).await })
                        .await;
                }
                TickOutcome::Skipped { obligation, reason } => {
                    info!(obligation = %obligation.obligation_id, reason, "obligation skipped");
                    let j = self.journal.clone();
                    let (id, status) = (obligation.obligation_id.clone(), obligation.status);
                    self.j(async move {
                        j.set_obligation_status(&id, status, Some(reason.to_string()))
                            .await
                    })
                    .await;
                }
                TickOutcome::Rescheduled(obligation) => {
                    let j = self.journal.clone();
                    self.j(async move { j.upsert_obligation(obligation).await })
                        .await;
                }
            }
        }
        self.drain_queue().await;
    }

    /// A due obligation re-enters the system as a typed event — same
    /// admission, attention, and policy path as everything else.
    async fn emit_due_event(&mut self, item: DueItem, now: OffsetDateTime) {
        let DueItem {
            obligation,
            operation_key,
            late_by_seconds,
        } = item;
        let payload = json!({
            "obligation": &obligation,
            "operation_key": &operation_key,
            "late_by_seconds": late_by_seconds,
        });
        let env = EventEnvelope::create(
            EventSource::Scheduler,
            EventType::ScheduledObligationDue,
            Some(operation_key),
            payload,
        );
        let admission = self.queue.admit(env.clone(), Instant::now());
        match admission.disposition {
            Disposition::Accepted | Disposition::Coalesced { .. } => {
                // Coalesced: the surviving queued copy carries an identical
                // embedded obligation and will complete it. Just journal.
                let label = match admission.disposition {
                    Disposition::Accepted => "accepted",
                    _ => "coalesced",
                };
                let j = self.journal.clone();
                self.j(async move {
                    j.record_event(env, EventClass::ScheduledDue, label, "")
                        .await
                })
                .await;
            }
            Disposition::Rejected { reason } => {
                // Never silently lose an obligation: re-arm for next tick.
                warn!(
                    reason,
                    obligation = %obligation.obligation_id,
                    "due event rejected by queue; re-arming"
                );
                let j = self.journal.clone();
                self.j(async move {
                    j.record_event(env, EventClass::ScheduledDue, "rejected", reason)
                        .await
                })
                .await;
                let mut rearmed = obligation;
                rearmed.next_run_at_utc =
                    now + time::Duration::seconds(self.cfg.scheduler.tick_seconds.max(1) as i64);
                if let Err(e) = self.scheduler.rearm(rearmed) {
                    warn!(error = %e, "failed to re-arm rejected obligation");
                }
            }
        }
        if let Some(evicted) = admission.evicted {
            let j = self.journal.clone();
            self.j(async move {
                j.record_event(evicted.envelope, evicted.class, "evicted", "lower_priority")
                    .await
            })
            .await;
        }
    }

    async fn drain_queue(&mut self) {
        while let Some(qe) = self.queue.pop() {
            let verdict = self.evaluator.evaluate(&qe, &self.snapshot());
            match verdict {
                Attention::Attend { reason } => self.attend(qe, reason).await,
                Attention::Defer { reason, .. } => {
                    self.note_verdict(&qe, "deferred", reason).await;
                }
                Attention::Ignore { reason } => {
                    self.note_verdict(&qe, "ignored", reason).await;
                }
                Attention::Escalate { reason } => {
                    warn!(event_id = %qe.envelope.event_id, reason, "event escalated");
                    self.note_verdict(&qe, "escalated", reason).await;
                }
            }
        }
    }

    /// Journal the attention verdict for events that were not attended.
    async fn note_verdict(&mut self, qe: &QueuedEvent, verdict: &str, reason: &'static str) {
        let event_id = qe.envelope.event_id.clone();
        let outcome = format!("{verdict}:{reason}");
        let j = self.journal.clone();
        self.j(async move {
            j.record_dispatch(
                format!("attn-{event_id}"),
                event_id,
                None,
                "attention",
                outcome,
            )
            .await
        })
        .await;
    }

    async fn attend(&mut self, qe: QueuedEvent, reason: &'static str) {
        if self.machine.state() == CognitiveState::Monitoring {
            let _ = self
                .machine
                .transition(CognitiveState::AttentionPending, reason);
        }

        if qe.envelope.event_type == EventType::ScheduledObligationDue {
            self.attend_obligation(&qe, OffsetDateTime::now_utc()).await;
        } else {
            let mode = if self.cfg.rust_core.shadow_mode {
                "shadow"
            } else {
                "live"
            };
            let event_id = qe.envelope.event_id.clone();
            let outcome = if self.cfg.rust_core.shadow_mode {
                format!("evaluated:{reason}")
            } else {
                format!("dispatched:{reason}")
            };
            let j = self.journal.clone();
            self.j(async move {
                j.record_dispatch(format!("disp-{event_id}"), event_id, None, mode, outcome)
                    .await
            })
            .await;
        }

        if self.machine.state() == CognitiveState::AttentionPending {
            let _ = self
                .machine
                .transition(CognitiveState::Cooldown, "attended");
            let _ = self
                .machine
                .transition(CognitiveState::Monitoring, "cooled");
        }
    }

    /// The full deterministic path for a released obligation:
    /// policy evaluation -> execution-time verification -> idempotent
    /// dispatch record -> verified completion bookkeeping.
    async fn attend_obligation(&mut self, qe: &QueuedEvent, now: OffsetDateTime) {
        let Some(obligation) = qe
            .envelope
            .payload
            .get("obligation")
            .cloned()
            .and_then(|v| serde_json::from_value::<Obligation>(v).ok())
        else {
            warn!(event_id = %qe.envelope.event_id, "obligation-due event missing payload");
            return;
        };
        let operation_key = qe
            .envelope
            .payload
            .get("operation_key")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("disp-{}", obligation.obligation_id));
        let event_id = qe.envelope.event_id.clone();
        let obligation_id = obligation.obligation_id.clone();

        let request = ActionRequest {
            intent_id: format!("intent-{operation_key}"),
            requester: Requester::Scheduler,
            verb: "run_obligation".into(),
            scope: obligation.scope.clone(),
            target: obligation_id.clone(),
            created_at_utc: now,
            ttl_seconds: 120,
            consequential: false,
        };
        let intent_id = request.intent_id.clone();
        let scope = obligation.scope.clone();

        match self.policy.evaluate(&request, now) {
            Decision::Deny(denial) => {
                warn!(obligation = %obligation_id, error = %denial, "obligation denied");
                let j = self.journal.clone();
                let reason = denial.to_string();
                self.j(async move {
                    j.record_intent(
                        intent_id,
                        "scheduler".into(),
                        "run_obligation".into(),
                        scope,
                        "deny".into(),
                        Some(reason),
                    )
                    .await
                })
                .await;
                let j = self.journal.clone();
                let id = obligation_id.clone();
                self.j(async move {
                    j.set_obligation_status(
                        &id,
                        ObligationStatus::Done,
                        Some("policy_denied".into()),
                    )
                    .await
                })
                .await;
            }
            Decision::RequireConfirmation(c) => {
                let j = self.journal.clone();
                let c_scope = c.scope.clone();
                self.j(async move {
                    j.record_intent(
                        c.intent_id,
                        "scheduler".into(),
                        c.verb,
                        c_scope,
                        "require_confirmation".into(),
                        None,
                    )
                    .await
                })
                .await;
                let mut parked = obligation;
                parked.status = ObligationStatus::AwaitingConfirmation;
                if let Err(e) = self.scheduler.rearm(parked.clone()) {
                    warn!(error = %e, "could not park obligation");
                }
                let j = self.journal.clone();
                self.j(async move { j.upsert_obligation(parked).await })
                    .await;
            }
            Decision::Allow(action) => {
                let j = self.journal.clone();
                self.j(async move {
                    j.record_intent(
                        intent_id,
                        "scheduler".into(),
                        "run_obligation".into(),
                        scope,
                        "allow".into(),
                        None,
                    )
                    .await
                })
                .await;

                // Minted is not enough — re-verify at execution time.
                let verified = action.verify_at_execution(now, &self.policy).is_ok();
                let outcome = if !verified {
                    "verification_failed"
                } else if self.cfg.rust_core.shadow_mode {
                    "shadow_evaluated"
                } else {
                    "dispatched"
                };
                let mode = if self.cfg.rust_core.shadow_mode {
                    "shadow"
                } else {
                    "live"
                };
                let j = self.journal.clone();
                let key = operation_key.clone();
                let inserted = self
                    .j(async move { j.record_dispatch(key, event_id, None, mode, outcome).await })
                    .await;
                if inserted == Some(false) {
                    // The deterministic operation key proves this exact
                    // obligation occurrence was already dispatched (e.g.
                    // before a restart). Never claim a second dispatch.
                    info!(
                        operation_key = %operation_key,
                        "duplicate dispatch suppressed by idempotency key"
                    );
                }
                if let Some(rearmed) = self.scheduler.complete(obligation, outcome, now) {
                    let j = self.journal.clone();
                    self.j(async move { j.upsert_obligation(rearmed).await })
                        .await;
                } else {
                    let j = self.journal.clone();
                    self.j(async move {
                        j.set_obligation_status(
                            &obligation_id,
                            ObligationStatus::Done,
                            Some(outcome.to_string()),
                        )
                        .await
                    })
                    .await;
                }
            }
        }
    }

    async fn shutdown(mut self) {
        let _ = self
            .machine
            .transition(CognitiveState::ShuttingDown, "shutdown_requested");
        let pending = self.queue.len();
        info!(pending_events = pending, "supervisor draining");
        while let Some(qe) = self.queue.pop() {
            let j = self.journal.clone();
            self.j(async move {
                j.record_event(qe.envelope, qe.class, "discarded_shutdown", "")
                    .await
            })
            .await;
        }
        self.journal.shutdown().await;
        info!("supervisor stopped cleanly");
    }
}
