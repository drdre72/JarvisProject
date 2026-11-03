//! Deterministic attention evaluation — explicit rules, no inference.
//!
//! An LLM may later submit *advisory* relevance hints, but a hint can never
//! raise an event's class or grant authority. Every verdict carries a
//! stable reason code for the journal.

use crate::events::{EventClass, QueuedEvent};
use crate::health::ResourceSnapshot;
use std::time::Duration;

/// What the supervisor should do with a queued event right now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Attention {
    Attend {
        reason: &'static str,
    },
    Defer {
        after: Duration,
        reason: &'static str,
    },
    Ignore {
        reason: &'static str,
    },
    /// Something keeps recurring or failed beyond local recovery rules;
    /// flagged for higher-level review (a human or an authorized job).
    Escalate {
        reason: &'static str,
    },
}

pub struct AttentionEvaluator {
    /// Coalesced-delivery count at which a repeated stimulus escalates
    /// instead of being re-attended — the source itself needs review.
    pub escalate_coalesced_threshold: u32,
    pub constrained_defer: Duration,
    pub maintenance_defer: Duration,
}

impl Default for AttentionEvaluator {
    fn default() -> Self {
        Self {
            escalate_coalesced_threshold: 8,
            constrained_defer: Duration::from_secs(60),
            maintenance_defer: Duration::from_secs(300),
        }
    }
}

impl AttentionEvaluator {
    pub fn evaluate(&self, event: &QueuedEvent, snap: &ResourceSnapshot) -> Attention {
        // Repetition guard before the class rules: a stimulus that will not
        // stop needs review, not another attend cycle.
        if event.coalesced >= self.escalate_coalesced_threshold
            && matches!(
                event.class,
                EventClass::ExternalStimulus | EventClass::ScheduledDue
            )
        {
            return Attention::Escalate {
                reason: "repeated_stimulus",
            };
        }

        match event.class {
            EventClass::UserInterrupt => Attention::Attend {
                reason: "user_interrupt",
            },
            EventClass::SafetyCritical => Attention::Attend {
                reason: "safety_critical",
            },
            EventClass::ScheduledDue => {
                if snap.is_critical() {
                    Attention::Defer {
                        after: self.constrained_defer,
                        reason: "resources_critical",
                    }
                } else {
                    Attention::Attend {
                        reason: "obligation_due",
                    }
                }
            }
            EventClass::ExternalStimulus => {
                if snap.is_critical() {
                    Attention::Defer {
                        after: self.constrained_defer,
                        reason: "resources_critical",
                    }
                } else {
                    Attention::Attend {
                        reason: "external_stimulus",
                    }
                }
            }
            EventClass::BackgroundMaintenance => {
                if snap.is_nominal() {
                    Attention::Attend {
                        reason: "maintenance_window",
                    }
                } else {
                    Attention::Defer {
                        after: self.maintenance_defer,
                        reason: "resources_constrained",
                    }
                }
            }
            EventClass::DiagnosticTelemetry => Attention::Ignore {
                reason: "telemetry_journal_only",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{EventEnvelope, EventSource, EventType};
    use serde_json::json;
    use std::time::Instant;
    use time::OffsetDateTime;

    fn snap(cpu: Option<f32>, mem: Option<f32>, depth: usize, cap: usize) -> ResourceSnapshot {
        ResourceSnapshot {
            cpu_percent: cpu,
            memory_percent: mem,
            queue_depth: depth,
            queue_capacity: cap,
            captured_at: OffsetDateTime::now_utc(),
        }
    }

    fn queued(ty: EventType, class: EventClass, coalesced: u32) -> QueuedEvent {
        QueuedEvent {
            envelope: EventEnvelope::create(EventSource::Internal, ty, None, json!({})),
            class,
            received_at: Instant::now(),
            coalesced,
        }
    }

    #[test]
    fn user_interrupt_always_attends() {
        let ev = AttentionEvaluator::default();
        let q = queued(EventType::UserRequestReceived, EventClass::UserInterrupt, 0);
        assert!(matches!(
            ev.evaluate(&q, &snap(Some(99.0), Some(99.0), 10, 10)),
            Attention::Attend {
                reason: "user_interrupt"
            }
        ));
    }

    #[test]
    fn maintenance_defers_under_pressure() {
        let ev = AttentionEvaluator::default();
        let q = queued(
            EventType::MaintenanceWindow,
            EventClass::BackgroundMaintenance,
            0,
        );
        let nominal = ev.evaluate(&q, &snap(Some(10.0), Some(20.0), 1, 100));
        assert!(matches!(nominal, Attention::Attend { .. }));
        let constrained = ev.evaluate(&q, &snap(Some(85.0), None, 1, 100));
        assert!(matches!(
            constrained,
            Attention::Defer {
                reason: "resources_constrained",
                ..
            }
        ));
    }

    #[test]
    fn repeated_stimulus_escalates() {
        let ev = AttentionEvaluator::default();
        let q = queued(EventType::ExternalStimulus, EventClass::ExternalStimulus, 8);
        assert!(matches!(
            ev.evaluate(&q, &snap(None, None, 0, 100)),
            Attention::Escalate {
                reason: "repeated_stimulus"
            }
        ));
    }

    #[test]
    fn telemetry_is_journal_only() {
        let ev = AttentionEvaluator::default();
        let q = queued(
            EventType::ResourceSample,
            EventClass::DiagnosticTelemetry,
            0,
        );
        assert!(matches!(
            ev.evaluate(&q, &snap(None, None, 0, 100)),
            Attention::Ignore {
                reason: "telemetry_journal_only"
            }
        ));
    }
}
