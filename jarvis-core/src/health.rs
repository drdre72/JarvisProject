//! Resource snapshot and health assessment.
//!
//! Phase One captures queue pressure only; CPU/memory probes arrive with
//! the Phase Seven resource monitor. Absent measurements stay `None` — a
//! missing probe is unknown, never zero.

use time::OffsetDateTime;

#[derive(Debug, Clone)]
pub struct ResourceSnapshot {
    pub cpu_percent: Option<f32>,
    pub memory_percent: Option<f32>,
    pub queue_depth: usize,
    pub queue_capacity: usize,
    pub captured_at: OffsetDateTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthLevel {
    Nominal,
    Constrained,
    Critical,
}

impl ResourceSnapshot {
    /// Moderate pressure: defer discretionary work.
    pub fn is_constrained(&self) -> bool {
        self.cpu_percent.is_some_and(|c| c > 80.0)
            || self.memory_percent.is_some_and(|m| m > 90.0)
            || self.queue_fill() > 0.75
    }

    /// Severe pressure: only user interrupts and safety events attend.
    pub fn is_critical(&self) -> bool {
        self.cpu_percent.is_some_and(|c| c > 95.0)
            || self.memory_percent.is_some_and(|m| m > 97.0)
            || self.queue_fill() >= 1.0
    }

    pub fn is_nominal(&self) -> bool {
        !self.is_constrained()
    }

    pub fn level(&self) -> HealthLevel {
        if self.is_critical() {
            HealthLevel::Critical
        } else if self.is_constrained() {
            HealthLevel::Constrained
        } else {
            HealthLevel::Nominal
        }
    }

    fn queue_fill(&self) -> f32 {
        if self.queue_capacity == 0 {
            return 1.0;
        }
        self.queue_depth as f32 / self.queue_capacity as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(cpu: Option<f32>, mem: Option<f32>, depth: usize, cap: usize) -> ResourceSnapshot {
        ResourceSnapshot {
            cpu_percent: cpu,
            memory_percent: mem,
            queue_depth: depth,
            queue_capacity: cap,
            captured_at: OffsetDateTime::now_utc(),
        }
    }

    #[test]
    fn missing_measurements_are_not_pressure() {
        let s = snap(None, None, 0, 100);
        assert_eq!(s.level(), HealthLevel::Nominal);
    }

    #[test]
    fn queue_pressure_counts() {
        assert!(snap(None, None, 80, 100).is_constrained());
        assert!(snap(None, None, 100, 100).is_critical());
        assert!(snap(Some(96.0), None, 0, 100).is_critical());
    }
}
