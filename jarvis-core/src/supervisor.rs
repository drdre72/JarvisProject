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
use crate::inference::{run_job, JobCaps, JobOutcome, JobSpec};
use crate::journal::{JournalError, JournalHandle};
use crate::policy::{ActionRequest, Decision, PolicyEngine, Requester};
use crate::scheduler::{
    DueItem, Obligation, ObligationStatus, ScheduleError, Scheduler, TickOutcome,
};
use crate::state::{CognitiveMachine, CognitiveState};
use crate::transport::{FrameBody, InboundRequest, WireFrame};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use tokio::sync::{mpsc, watch};
use tokio::task::JoinHandle;
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
    /// Response frames back to the Python peer (Stage B IPC surface).
    outbound: mpsc::Sender<WireFrame>,
    /// In-flight inference jobs: job_id -> (cancel sender, task handle).
    /// Bounded by `inference.max_concurrent`; emptied on completion via the
    /// `done` channel or cancelled on shutdown.
    inflight: HashMap<String, (watch::Sender<bool>, JoinHandle<()>)>,
    /// Job-completion drain, created when `run` starts so `new` keeps its
    /// signature for tests that never open the request channel.
    done_tx: Option<mpsc::Sender<String>>,
}

impl Supervisor {
    pub fn new(
        cfg: CoreConfig,
        journal: JournalHandle,
        scheduler: Scheduler,
        outbound: mpsc::Sender<WireFrame>,
    ) -> Self {
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
            outbound,
            inflight: HashMap::new(),
            done_tx: None,
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
        mut requests: mpsc::Receiver<InboundRequest>,
        mut shutdown: mpsc::Receiver<()>,
    ) {
        let mut tick =
            tokio::time::interval(Duration::from_secs(self.cfg.scheduler.tick_seconds.max(1)));
        // A busy tick skips rather than bursts — catch-up work is bounded
        // by collect_due, not by tokio.
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        let mut intake_open = true;
        let (done_tx, mut done_rx) = mpsc::channel::<String>(16);
        self.done_tx = Some(done_tx);
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
                Some(req) = requests.recv() => self.on_request(req).await,
                maybe_done = done_rx.recv() => {
                    if let Some(job_id) = maybe_done {
                        self.inflight.remove(&job_id);
                    }
                }
            }
        }
        self.shutdown().await;
    }

    /// Stage B limited surface: durable obligation scheduling only. Every
    /// request gets a typed response; the flag keeps the function inert.
    async fn on_request(&mut self, req: InboundRequest) {
        if !self.cfg.rust_core.ipc_requests {
            self.respond(req.request_id, false, json!({"error": "ipc_disabled"}))
                .await;
            return;
        }
        match req.op.as_str() {
            "schedule_obligation" => self.op_schedule_obligation(req).await,
            "cancel_obligation" => self.op_cancel_obligation(req).await,
            "run_inference" => self.op_run_inference(req).await,
            "cancel_inference" => self.op_cancel_inference(req).await,
            _ => {
                self.respond(
                    req.request_id,
                    false,
                    json!({"error": "unknown_op", "op": req.op}),
                )
                .await;
            }
        }
    }

    async fn respond(&self, request_id: String, ok: bool, body: Value) {
        let frame = WireFrame::new(FrameBody::Response {
            request_id,
            ok,
            body,
        });
        if self.outbound.send(frame).await.is_err() {
            warn!("outbound channel closed; response dropped");
        }
    }

    /// The transferred function: validate, journal, then arm. Persistence
    /// before acknowledgement; a journal failure rolls back the in-memory
    /// schedule so the two never disagree silently.
    async fn op_schedule_obligation(&mut self, req: InboundRequest) {
        let request_id = req.request_id;
        let mut obligation: Obligation = match serde_json::from_value(req.args) {
            Ok(o) => o,
            Err(e) => {
                self.respond(
                    request_id,
                    false,
                    json!({"error": "malformed_obligation", "detail": e.to_string()}),
                )
                .await;
                return;
            }
        };
        // Untrusted input cannot self-assign status or a prior result.
        obligation.status = ObligationStatus::Pending;
        obligation.last_result = None;
        if let Err(e) = obligation.validate() {
            self.respond(
                request_id,
                false,
                json!({"error": "invalid_obligation", "detail": e.to_string()}),
            )
            .await;
            return;
        }
        if let Err(e) = self.scheduler.schedule(obligation.clone()) {
            // A re-registered stable ID is idempotent, not a fault: callers
            // distinguish "already armed" from real schedule failures.
            let code = match e {
                ScheduleError::Duplicate(_) => "duplicate_obligation",
                _ => "schedule_error",
            };
            self.respond(
                request_id,
                false,
                json!({"error": code, "detail": e.to_string()}),
            )
            .await;
            return;
        }
        let j = self.journal.clone();
        let stored = obligation.clone();
        let obligation_id = obligation.obligation_id.clone();
        match self
            .j(async move { j.upsert_obligation(stored).await })
            .await
        {
            Some(()) => {
                self.respond(request_id, true, json!({"obligation_id": obligation_id}))
                    .await;
            }
            None => {
                self.scheduler.cancel(&obligation_id);
                self.respond(request_id, false, json!({"error": "journal_failed"}))
                    .await;
            }
        }
    }

    /// Stage B: bounded child-process execution for one inference call.
    /// Intent is journaled before spawn; the terminal outcome is journaled
    /// before the response is sent. Prompt text and output never reach the
    /// journal — status, exit codes, and byte counts only. The job runs in
    /// its own task so a 45s generation never stalls the supervisor loop.
    async fn op_run_inference(&mut self, req: InboundRequest) {
        let request_id = req.request_id;
        if !self.cfg.rust_core.inference_relay {
            self.respond(request_id, false, json!({"error": "inference_disabled"}))
                .await;
            return;
        }
        let inf = &self.cfg.inference;
        let caps = JobCaps {
            max_argv_count: inf.max_argv_count,
            max_argv_bytes: inf.max_argv_bytes,
            max_stdout_bytes: inf.max_stdout_bytes,
            max_stderr_bytes: inf.max_stderr_bytes,
        };
        let spec = match JobSpec::from_args(
            &req.args,
            &caps,
            Duration::from_secs(inf.default_timeout_seconds),
            Duration::from_secs(inf.max_timeout_seconds),
        ) {
            Ok(s) => s,
            Err(e) => {
                self.respond(
                    request_id,
                    false,
                    json!({"error": "invalid_request", "detail": e.to_string()}),
                )
                .await;
                return;
            }
        };
        if self.inflight.len() >= inf.max_concurrent {
            self.respond(request_id, false, json!({"error": "inference_busy"}))
                .await;
            return;
        }
        let now = OffsetDateTime::now_utc();
        let intent_id = format!("intent-{}", spec.job_id);
        let action_req = ActionRequest {
            intent_id: intent_id.clone(),
            requester: Requester::Worker,
            verb: "run_inference".into(),
            scope: "ops.inference.run".into(),
            target: spec.job_id.clone(),
            created_at_utc: now,
            ttl_seconds: 120,
            consequential: false,
        };
        match self.policy.evaluate(&action_req, now) {
            Decision::Allow(_) => {}
            Decision::Deny(denial) => {
                let j = self.journal.clone();
                let iid = intent_id.clone();
                let reason = denial.to_string();
                self.j(async move {
                    j.record_intent(
                        iid,
                        "worker".into(),
                        "run_inference".into(),
                        "ops.inference.run".into(),
                        "deny".into(),
                        Some(reason),
                    )
                    .await
                })
                .await;
                self.respond(
                    request_id,
                    false,
                    json!({"error": "denied", "detail": denial.to_string()}),
                )
                .await;
                return;
            }
            Decision::RequireConfirmation(_) => {
                self.respond(request_id, false, json!({"error": "requires_confirmation"}))
                    .await;
                return;
            }
        }
        let j = self.journal.clone();
        let iid = intent_id;
        self.j(async move {
            j.record_intent(
                iid,
                "worker".into(),
                "run_inference".into(),
                "ops.inference.run".into(),
                "allow".into(),
                None,
            )
            .await
        })
        .await;

        let Some(done) = self.done_tx.clone() else {
            self.respond(request_id, false, json!({"error": "not_running"}))
                .await;
            return;
        };
        let (cancel_tx, cancel_rx) = watch::channel(false);
        let binary = self.cfg.inference_binary();
        let outbound = self.outbound.clone();
        let journal = self.journal.clone();
        let job_id = spec.job_id.clone();
        let map_key = spec.job_id.clone();
        let handle = tokio::spawn(async move {
            let (outcome, elapsed_ms) = run_job(binary, spec, caps, cancel_rx).await;
            let verified = !matches!(outcome, JobOutcome::SpawnFailed(_));
            let detail = outcome_detail(&outcome, elapsed_ms);
            let _ = journal
                .record_result(
                    format!("result-{job_id}"),
                    format!("intent-{job_id}"),
                    outcome.status_str().to_string(),
                    verified,
                    Some(detail),
                )
                .await;
            let _ = journal
                .upsert_worker("inference", outcome.status_str(), 0, None)
                .await;
            let (ok, body) = response_for_outcome(&job_id, outcome, elapsed_ms);
            let frame = WireFrame::new(FrameBody::Response {
                request_id,
                ok,
                body,
            });
            let _ = outbound.send(frame).await;
            let _ = done.send(job_id).await;
        });
        self.inflight.insert(map_key, (cancel_tx, handle));
    }

    /// Cancel the in-flight job: `job_id` targets a specific one, or with a
    /// single-flight cap the sole running job is implied. The job task's
    /// own response still arrives later with status `cancelled`.
    async fn op_cancel_inference(&mut self, req: InboundRequest) {
        let request_id = req.request_id;
        if !self.cfg.rust_core.inference_relay {
            self.respond(request_id, false, json!({"error": "inference_disabled"}))
                .await;
            return;
        }
        let named = req
            .args
            .get("job_id")
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let job_id = match (named, self.inflight.len()) {
            (Some(id), _) => Some(id),
            (None, 1) => self.inflight.keys().next().cloned(),
            (None, _) => None,
        };
        let hit = job_id.and_then(|id| {
            self.inflight
                .get(&id)
                .map(|(tx, _)| (id.clone(), tx.clone()))
        });
        match hit {
            Some((id, tx)) => {
                let _ = tx.send(true);
                self.respond(request_id, true, json!({"cancelled": id}))
                    .await;
            }
            None => {
                self.respond(request_id, false, json!({"error": "not_running"}))
                    .await;
            }
        }
    }

    /// Un-arm a queued obligation. An already-released due event completes
    /// through the normal path; the journal preserves the order either way.
    async fn op_cancel_obligation(&mut self, req: InboundRequest) {
        let request_id = req.request_id;
        let Some(id) = req.args.get("obligation_id").and_then(Value::as_str) else {
            self.respond(
                request_id,
                false,
                json!({"error": "malformed_args", "detail": "obligation_id required"}),
            )
            .await;
            return;
        };
        let id = id.to_string();
        if !self.scheduler.cancel(&id) {
            self.respond(request_id, false, json!({"error": "unknown_obligation"}))
                .await;
            return;
        }
        let j = self.journal.clone();
        let jid = id.clone();
        self.j(async move {
            j.set_obligation_status(
                &jid,
                ObligationStatus::Cancelled,
                Some("cancelled_via_ipc".into()),
            )
            .await
        })
        .await;
        self.respond(
            request_id,
            true,
            json!({"obligation_id": id, "status": "cancelled"}),
        )
        .await;
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
        // Cancel in-flight inference jobs; kill_on_drop plus the explicit
        // kill in run_job guarantees no child outlives the supervisor.
        for (job_id, (tx, handle)) in self.inflight.drain() {
            let _ = tx.send(true);
            let _ = tokio::time::timeout(Duration::from_secs(2), handle).await;
            info!(job_id = %job_id, "inference job cancelled at shutdown");
        }
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

/// Journal-safe detail for a job outcome: counts and codes, never content.
fn outcome_detail(outcome: &JobOutcome, elapsed_ms: u128) -> String {
    match outcome {
        JobOutcome::Completed {
            exit_code,
            stdout,
            stderr,
        } => format!(
            "exit={exit_code:?} stdout={}B stderr={}B elapsed={elapsed_ms}ms",
            stdout.len(),
            stderr.len()
        ),
        _ => format!("elapsed={elapsed_ms}ms"),
    }
}

/// Map a terminal job to the response frame. `ok` follows whether the
/// child ran to a real exit; terminal states carry the typed `error` the
/// Python adapter maps onto its legacy exception contract.
fn response_for_outcome(job_id: &str, outcome: JobOutcome, elapsed_ms: u128) -> (bool, Value) {
    match outcome {
        JobOutcome::Completed {
            exit_code,
            stdout,
            stderr,
        } => (
            true,
            json!({
                "job_id": job_id,
                "status": "completed",
                "exit_code": exit_code,
                "stdout": String::from_utf8_lossy(&stdout),
                "stderr_tail": tail_lossy(&stderr, 2048),
                "stdout_bytes": stdout.len(),
                "elapsed_ms": elapsed_ms,
            }),
        ),
        other => (
            false,
            json!({
                "job_id": job_id,
                "status": other.status_str(),
                "error": other.status_str(),
                "elapsed_ms": elapsed_ms,
            }),
        ),
    }
}

fn tail_lossy(bytes: &[u8], keep: usize) -> String {
    let start = bytes.len().saturating_sub(keep);
    String::from_utf8_lossy(&bytes[start..]).into_owned()
}
