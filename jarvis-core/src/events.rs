//! Typed internal events and the bounded admission queue.
//!
//! Every event carries a schema version, unique id, trace id, source, type,
//! and a validated payload. Sources never assign their own priority — the
//! supervisor maps (source, type) -> `EventClass`. The queue enforces
//! capacity, per-source rate limits, dedup/coalescing windows, and
//! priority eviction so bursts cannot starve time-sensitive work.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cmp::Reverse;
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};
use thiserror::Error;
use time::OffsetDateTime;

use crate::ids;

pub const SCHEMA_VERSION: u16 = 1;
const MAX_ID_LEN: usize = 64;
const MAX_DEDUP_KEY_LEN: usize = 128;
/// Clock-skew tolerance for incoming wall-clock timestamps.
const MAX_FUTURE_SKEW_SECONDS: i64 = 300;
/// Upper bound on the dedup bookkeeping map; entries expire with the window.
const MAX_DEDUP_ENTRIES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSource {
    WakeWord,
    Ui,
    VoicePipeline,
    Scheduler,
    WorkerSupervisor,
    ResourceMonitor,
    External,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    WakeWordDetected,
    UserRequestReceived,
    ScheduledObligationDue,
    WorkerHeartbeat,
    WorkerExited,
    ResourceSample,
    MaintenanceWindow,
    ExternalStimulus,
    ShutdownRequested,
}

/// Supervisor-assigned priority class. Ordering matters: later variants
/// outrank earlier ones. Untrusted input cannot raise its own class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventClass {
    DiagnosticTelemetry = 0,
    BackgroundMaintenance = 1,
    ExternalStimulus = 2,
    ScheduledDue = 3,
    SafetyCritical = 4,
    UserInterrupt = 5,
}

impl EventClass {
    pub fn as_str(self) -> &'static str {
        match self {
            EventClass::DiagnosticTelemetry => "diagnostic_telemetry",
            EventClass::BackgroundMaintenance => "background_maintenance",
            EventClass::ExternalStimulus => "external_stimulus",
            EventClass::ScheduledDue => "scheduled_due",
            EventClass::SafetyCritical => "safety_critical",
            EventClass::UserInterrupt => "user_interrupt",
        }
    }
}

/// The only place priority is decided.
pub fn classify(source: &EventSource, event_type: EventType) -> EventClass {
    use EventType::*;
    match (source, event_type) {
        (_, ShutdownRequested) => EventClass::SafetyCritical,
        (EventSource::Ui, UserRequestReceived) => EventClass::UserInterrupt,
        (EventSource::VoicePipeline, UserRequestReceived) => EventClass::UserInterrupt,
        (EventSource::WakeWord, WakeWordDetected) => EventClass::UserInterrupt,
        (_, ScheduledObligationDue) => EventClass::ScheduledDue,
        (_, WorkerExited) => EventClass::ExternalStimulus,
        (_, ExternalStimulus) => EventClass::ExternalStimulus,
        (_, MaintenanceWindow) => EventClass::BackgroundMaintenance,
        (_, WorkerHeartbeat | ResourceSample) => EventClass::DiagnosticTelemetry,
        _ => EventClass::ExternalStimulus,
    }
}

#[derive(Debug, Error)]
pub enum EventError {
    #[error("malformed event JSON: {0}")]
    Malformed(#[from] serde_json::Error),
    #[error("unsupported schema_version {0}")]
    UnsupportedVersion(u16),
    #[error("field {0} must be 1..=64 chars")]
    BadId(&'static str),
    #[error("dedup_key exceeds {MAX_DEDUP_KEY_LEN} chars")]
    OversizedDedupKey,
    #[error("payload exceeds {0} bytes")]
    OversizedPayload(usize),
    #[error("occurred_at_utc is more than {MAX_FUTURE_SKEW_SECONDS}s in the future")]
    FutureTimestamp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub schema_version: u16,
    pub event_id: String,
    pub trace_id: String,
    pub source: EventSource,
    pub event_type: EventType,
    #[serde(with = "time::serde::rfc3339")]
    pub occurred_at_utc: OffsetDateTime,
    #[serde(default)]
    pub dedup_key: Option<String>,
    #[serde(default)]
    pub payload: Value,
}

impl EventEnvelope {
    /// Supervisor-side constructor: fresh ids, explicit source/type.
    pub fn create(
        source: EventSource,
        event_type: EventType,
        dedup_key: Option<String>,
        payload: Value,
    ) -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            event_id: ids::new_id("evt"),
            trace_id: ids::new_id("trace"),
            source,
            event_type,
            occurred_at_utc: OffsetDateTime::now_utc(),
            dedup_key,
            payload,
        }
    }

    pub fn validate(&self, max_payload_bytes: usize) -> Result<(), EventError> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(EventError::UnsupportedVersion(self.schema_version));
        }
        if self.event_id.is_empty() || self.event_id.len() > MAX_ID_LEN {
            return Err(EventError::BadId("event_id"));
        }
        if self.trace_id.is_empty() || self.trace_id.len() > MAX_ID_LEN {
            return Err(EventError::BadId("trace_id"));
        }
        if let Some(key) = &self.dedup_key {
            if key.len() > MAX_DEDUP_KEY_LEN {
                return Err(EventError::OversizedDedupKey);
            }
        }
        let size = serde_json::to_vec(&self.payload)?.len();
        if size > max_payload_bytes {
            return Err(EventError::OversizedPayload(max_payload_bytes));
        }
        if self.occurred_at_utc
            > OffsetDateTime::now_utc() + time::Duration::seconds(MAX_FUTURE_SKEW_SECONDS)
        {
            return Err(EventError::FutureTimestamp);
        }
        Ok(())
    }

    /// Strict ingress path for untrusted JSON lines.
    pub fn from_json(line: &str, max_payload_bytes: usize) -> Result<Self, EventError> {
        let env: EventEnvelope = serde_json::from_str(line)?;
        env.validate(max_payload_bytes)?;
        Ok(env)
    }
}

#[derive(Debug)]
pub struct QueuedEvent {
    pub envelope: EventEnvelope,
    pub class: EventClass,
    pub received_at: Instant,
    /// How many later deliveries were folded into this entry.
    pub coalesced: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Disposition {
    Accepted,
    Coalesced { into_event_id: String },
    Rejected { reason: &'static str },
}

#[derive(Debug)]
pub struct Admission {
    pub disposition: Disposition,
    /// A lower-priority entry evicted to make room; journaled by the caller.
    pub evicted: Option<QueuedEvent>,
}

/// Bounded, deduplicating, priority-aware intake. `Instant` drives all
/// windows (monotonic; immune to wall-clock changes).
pub struct BoundedEventQueue {
    capacity: usize,
    dedup_window: Duration,
    per_source_burst: usize,
    queue: VecDeque<QueuedEvent>,
    dedup: HashMap<String, (Instant, String)>,
    rate: HashMap<EventSource, VecDeque<Instant>>,
}

impl BoundedEventQueue {
    pub fn new(capacity: usize, dedup_window: Duration, per_source_burst: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            dedup_window,
            per_source_burst: per_source_burst.max(1),
            queue: VecDeque::new(),
            dedup: HashMap::new(),
            rate: HashMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Returns the queue priority class for an already-validated envelope.
    pub fn class_of(env: &EventEnvelope) -> EventClass {
        classify(&env.source, env.event_type)
    }

    pub fn admit(&mut self, env: EventEnvelope, now: Instant) -> Admission {
        self.prune_dedup(now);

        // Per-source rate limit (sliding monotonic window).
        let times = self.rate.entry(env.source.clone()).or_default();
        while times
            .front()
            .is_some_and(|t| now.duration_since(*t) > self.dedup_window)
        {
            times.pop_front();
        }
        if times.len() >= self.per_source_burst {
            return Admission {
                disposition: Disposition::Rejected {
                    reason: "rate_limited",
                },
                evicted: None,
            };
        }
        times.push_back(now);

        // Coalescing: same dedup key inside the window folds into the
        // existing queued entry instead of duplicating work.
        if let Some(key) = &env.dedup_key {
            if let Some((_, existing_id)) = self.dedup.get(key).cloned() {
                if let Some(existing) = self
                    .queue
                    .iter_mut()
                    .find(|q| q.envelope.event_id == existing_id)
                {
                    existing.coalesced += 1;
                    self.dedup.insert(key.clone(), (now, existing_id.clone()));
                    return Admission {
                        disposition: Disposition::Coalesced {
                            into_event_id: existing_id,
                        },
                        evicted: None,
                    };
                }
            }
        }

        let class = Self::class_of(&env);

        // Capacity: evict the lowest-priority, oldest entry only if the new
        // event strictly outranks it; otherwise reject the newcomer.
        let mut evicted = None;
        if self.queue.len() >= self.capacity {
            let candidate = self
                .queue
                .iter()
                .enumerate()
                .min_by_key(|(_, q)| (q.class, q.received_at))
                .map(|(idx, q)| (idx, q.class));
            match candidate {
                Some((idx, lowest_class)) if class > lowest_class => {
                    evicted = self.queue.remove(idx);
                }
                _ => {
                    return Admission {
                        disposition: Disposition::Rejected {
                            reason: "queue_full",
                        },
                        evicted: None,
                    };
                }
            }
        }

        if let Some(key) = &env.dedup_key {
            self.dedup.insert(key.clone(), (now, env.event_id.clone()));
        }
        self.queue.push_back(QueuedEvent {
            envelope: env,
            class,
            received_at: now,
            coalesced: 0,
        });
        Admission {
            disposition: Disposition::Accepted,
            evicted,
        }
    }

    /// Current coalesce count for a queued event (post-merge accounting).
    pub fn coalesced_count(&self, event_id: &str) -> Option<u32> {
        self.queue
            .iter()
            .find(|q| q.envelope.event_id == event_id)
            .map(|q| q.coalesced)
    }

    /// Highest class first; ties resolve to the oldest arrival so no entry
    /// starves inside its own class.
    pub fn pop(&mut self) -> Option<QueuedEvent> {
        let (idx, _) = self
            .queue
            .iter()
            .enumerate()
            .max_by_key(|(_, q)| (q.class, Reverse(q.received_at)))?;
        self.queue.remove(idx)
    }

    fn prune_dedup(&mut self, now: Instant) {
        if self.dedup.len() <= MAX_DEDUP_ENTRIES {
            // Still purge expired keys opportunistically.
            if self
                .dedup
                .values()
                .all(|(seen, _)| now.duration_since(*seen) <= self.dedup_window)
            {
                return;
            }
        }
        let window = self.dedup_window;
        self.dedup
            .retain(|_, (seen, _)| now.duration_since(*seen) <= window);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn env(source: EventSource, ty: EventType, dedup: Option<&str>) -> EventEnvelope {
        let e = EventEnvelope::create(source, ty, dedup.map(|s| s.to_string()), json!({}));
        e.validate(16 * 1024).unwrap();
        e
    }

    #[test]
    fn validates_and_rejects_bad_envelopes() {
        let mut e = env(EventSource::Ui, EventType::UserRequestReceived, None);
        e.schema_version = 99;
        assert!(matches!(
            e.validate(1024),
            Err(EventError::UnsupportedVersion(99))
        ));

        let mut e = env(EventSource::Ui, EventType::UserRequestReceived, None);
        e.event_id = String::new();
        assert!(matches!(
            e.validate(1024),
            Err(EventError::BadId("event_id"))
        ));

        let mut e = env(EventSource::Ui, EventType::UserRequestReceived, None);
        e.payload = json!({"blob": "x".repeat(2048)});
        assert!(matches!(
            e.validate(64),
            Err(EventError::OversizedPayload(64))
        ));

        let mut e = env(EventSource::Ui, EventType::UserRequestReceived, None);
        e.occurred_at_utc = OffsetDateTime::now_utc() + time::Duration::days(1);
        assert!(matches!(e.validate(1024), Err(EventError::FutureTimestamp)));
    }

    #[test]
    fn parses_contract_json_shape() {
        // Contract shape from the event spec, with a timestamp that stays
        // inside the clock-skew window regardless of when the test runs.
        let now = OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap();
        let line = format!(
            r#"{{"schema_version":1,"event_id":"evt-1","trace_id":"trace-1","source":"wake_word","event_type":"WakeWordDetected","occurred_at_utc":"{now}","payload":{{}}}}"#
        );
        let e = EventEnvelope::from_json(&line, 1024).unwrap();
        assert_eq!(e.source, EventSource::WakeWord);
        assert_eq!(e.event_type, EventType::WakeWordDetected);
        assert_eq!(BoundedEventQueue::class_of(&e), EventClass::UserInterrupt);
    }

    #[test]
    fn coalesces_repeated_dedup_keys() {
        let mut q = BoundedEventQueue::new(8, Duration::from_secs(30), 4);
        let t0 = Instant::now();
        let first = env(
            EventSource::WakeWord,
            EventType::WakeWordDetected,
            Some("wake"),
        );
        let first_id = first.event_id.clone();
        assert_eq!(q.admit(first, t0).disposition, Disposition::Accepted);
        for _ in 0..3 {
            let dup = env(
                EventSource::WakeWord,
                EventType::WakeWordDetected,
                Some("wake"),
            );
            assert_eq!(
                q.admit(dup, t0).disposition,
                Disposition::Coalesced {
                    into_event_id: first_id.clone()
                }
            );
        }
        assert_eq!(q.len(), 1);
        assert_eq!(q.pop().unwrap().coalesced, 3);
    }

    #[test]
    fn dedup_expires_after_window() {
        let mut q = BoundedEventQueue::new(8, Duration::from_secs(30), 8);
        let t0 = Instant::now();
        q.admit(
            env(
                EventSource::WakeWord,
                EventType::WakeWordDetected,
                Some("w"),
            ),
            t0,
        );
        q.pop();
        // A later identical key after the window is a fresh event.
        let later = q.admit(
            env(
                EventSource::WakeWord,
                EventType::WakeWordDetected,
                Some("w"),
            ),
            t0 + Duration::from_secs(31),
        );
        assert_eq!(later.disposition, Disposition::Accepted);
    }

    #[test]
    fn rate_limits_noisy_source_but_not_others() {
        let mut q = BoundedEventQueue::new(64, Duration::from_secs(30), 4);
        let t0 = Instant::now();
        let mut rejected = 0;
        for _ in 0..10 {
            let d = q
                .admit(
                    env(EventSource::External, EventType::ExternalStimulus, None),
                    t0,
                )
                .disposition;
            if matches!(d, Disposition::Rejected { .. }) {
                rejected += 1;
            }
        }
        assert_eq!(rejected, 6);
        // A different source still gets through.
        assert_eq!(
            q.admit(
                env(EventSource::Ui, EventType::UserRequestReceived, None),
                t0
            )
            .disposition,
            Disposition::Accepted
        );
    }

    #[test]
    fn full_queue_evicts_lower_priority_not_higher() {
        let mut q = BoundedEventQueue::new(2, Duration::from_secs(30), 64);
        let t0 = Instant::now();
        q.admit(
            env(
                EventSource::ResourceMonitor,
                EventType::ResourceSample,
                None,
            ),
            t0,
        );
        q.admit(
            env(
                EventSource::ResourceMonitor,
                EventType::ResourceSample,
                None,
            ),
            t0,
        );
        // A user interrupt evicts telemetry.
        let admission = q.admit(
            env(EventSource::Ui, EventType::UserRequestReceived, None),
            t0,
        );
        assert_eq!(admission.disposition, Disposition::Accepted);
        assert_eq!(
            admission.evicted.unwrap().class,
            EventClass::DiagnosticTelemetry
        );
        // Telemetry cannot evict a user interrupt.
        let refused = q.admit(
            env(
                EventSource::ResourceMonitor,
                EventType::ResourceSample,
                None,
            ),
            t0,
        );
        assert_eq!(
            refused.disposition,
            Disposition::Rejected {
                reason: "queue_full"
            }
        );
    }

    #[test]
    fn pop_orders_by_class_then_arrival() {
        let mut q = BoundedEventQueue::new(16, Duration::from_secs(30), 64);
        let t0 = Instant::now();
        q.admit(
            env(EventSource::External, EventType::ExternalStimulus, None),
            t0,
        );
        q.admit(
            env(EventSource::Ui, EventType::UserRequestReceived, None),
            t0 + Duration::from_millis(2),
        );
        q.admit(
            env(EventSource::Internal, EventType::MaintenanceWindow, None),
            t0 + Duration::from_millis(1),
        );
        q.admit(
            env(EventSource::Ui, EventType::UserRequestReceived, None),
            t0 + Duration::from_millis(3),
        );
        let classes: Vec<EventClass> = (0..4).map(|_| q.pop().unwrap().class).collect();
        assert_eq!(
            classes,
            [
                EventClass::UserInterrupt,
                EventClass::UserInterrupt,
                EventClass::ExternalStimulus,
                EventClass::BackgroundMaintenance,
            ]
        );
    }
}
