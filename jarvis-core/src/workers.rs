//! Worker lifecycle types and bounded recovery.
//!
//! One `WorkerSupervisor` owns each worker's state. Spawning the actual
//! Python adapters is Phase Five/Six — this module already provides the
//! state machine, heartbeat staleness, exit accounting, exponential
//! backoff, and the restart budget that ends in an explicit `Degraded`
//! outcome instead of an infinite restart loop.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerStatus {
    Stopped,
    Starting,
    Ready,
    Unhealthy,
    Stopping,
    /// Recovery budget exhausted — reported, not silently retried forever.
    Failed,
}

impl WorkerStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkerStatus::Stopped => "stopped",
            WorkerStatus::Starting => "starting",
            WorkerStatus::Ready => "ready",
            WorkerStatus::Unhealthy => "unhealthy",
            WorkerStatus::Stopping => "stopping",
            WorkerStatus::Failed => "failed",
        }
    }

    pub fn may_transition_to(self, next: WorkerStatus) -> bool {
        use WorkerStatus::*;
        matches!(
            (self, next),
            (Stopped, Starting)
                | (Starting, Ready | Stopping | Failed)
                | (Ready, Unhealthy | Stopping | Failed)
                | (Unhealthy, Starting | Stopping | Failed)
                | (Stopping, Stopped | Failed)
                | (Failed, Starting | Stopped)
        )
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid worker transition {from:?} -> {to:?}")]
pub struct InvalidWorkerTransition {
    pub from: WorkerStatus,
    pub to: WorkerStatus,
}

/// How a worker proves it is alive. `Stdio` heartbeats arrive over the IPC
/// channel; `ProcessAlive` only checks the OS handle (last resort).
#[derive(Debug, Clone)]
pub enum Readiness {
    StdioHeartbeat,
    ProcessAlive,
}

#[derive(Debug, Clone)]
pub struct WorkerSpec {
    pub name: String,
    pub program: PathBuf,
    pub args: Vec<String>,
    pub readiness: Readiness,
}

/// Bounded restart policy — exponential backoff inside a rolling window,
/// then `Failed`. `max_attempts` counts *within* `window`.
#[derive(Debug, Clone)]
pub struct RestartPolicy {
    pub max_attempts: u32,
    pub window: Duration,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
}

impl RestartPolicy {
    /// Deterministic exponential backoff, capped. (Jitter belongs to the
    /// caller's scheduling, not the budget math, so tests stay exact.)
    pub fn backoff(&self, attempt: u32) -> Duration {
        let shift = attempt.min(16);
        self.base_backoff
            .saturating_mul(1u32 << shift)
            .min(self.max_backoff)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryAction {
    /// Restart is permitted after this delay.
    Restart { delay: Duration },
    /// Budget exhausted: supervisor must surface degraded state.
    Degrade,
}

pub struct WorkerSupervisor {
    spec: WorkerSpec,
    policy: RestartPolicy,
    status: WorkerStatus,
    last_heartbeat: Option<Instant>,
    /// Monotonic timestamps of recent restart attempts.
    attempts: VecDeque<Instant>,
    last_error: Option<String>,
}

impl WorkerSupervisor {
    pub fn new(spec: WorkerSpec, policy: RestartPolicy) -> Self {
        Self {
            spec,
            policy,
            status: WorkerStatus::Stopped,
            last_heartbeat: None,
            attempts: VecDeque::new(),
            last_error: None,
        }
    }

    pub fn spec(&self) -> &WorkerSpec {
        &self.spec
    }

    pub fn status(&self) -> WorkerStatus {
        self.status
    }

    pub fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub fn restart_attempts(&self) -> usize {
        self.attempts.len()
    }

    pub fn transition(
        &mut self,
        to: WorkerStatus,
    ) -> Result<WorkerStatus, InvalidWorkerTransition> {
        if !self.status.may_transition_to(to) {
            return Err(InvalidWorkerTransition {
                from: self.status,
                to,
            });
        }
        self.status = to;
        Ok(to)
    }

    pub fn note_heartbeat(&mut self, now: Instant) {
        self.last_heartbeat = Some(now);
    }

    /// A `Ready` worker whose heartbeat is older than `timeout` is suspect.
    pub fn is_stale(&self, now: Instant, timeout: Duration) -> bool {
        self.status == WorkerStatus::Ready
            && self
                .last_heartbeat
                .map(|t| now.duration_since(t) > timeout)
                .unwrap_or(true)
    }

    fn prune_attempts(&mut self, now: Instant) {
        while self
            .attempts
            .front()
            .is_some_and(|t| now.duration_since(*t) > self.policy.window)
        {
            self.attempts.pop_front();
        }
    }

    /// A worker exited or failed its readiness check. Either a bounded
    /// restart is owed, or the budget is exhausted and the supervisor must
    /// degrade honestly.
    pub fn note_failure(&mut self, now: Instant, error: impl Into<String>) -> RecoveryAction {
        self.last_error = Some(error.into());
        self.prune_attempts(now);
        self.attempts.push_back(now);
        if self.attempts.len() > self.policy.max_attempts as usize {
            self.status = WorkerStatus::Failed;
            RecoveryAction::Degrade
        } else {
            let delay = self.policy.backoff(self.attempts.len() as u32 - 1);
            self.status = WorkerStatus::Stopped;
            RecoveryAction::Restart { delay }
        }
    }

    /// Successful ready signal resets the failure accounting.
    pub fn note_ready(&mut self, now: Instant) {
        self.attempts.clear();
        self.last_error = None;
        self.last_heartbeat = Some(now);
        self.status = WorkerStatus::Ready;
    }
}

/// Phase Five/Six seam: something that can actually start/stop a process
/// and speak the transport codec. Not implemented in the shadow stage.
pub trait WorkerDriver {
    fn start(&mut self, spec: &WorkerSpec) -> std::io::Result<()>;
    fn stop(&mut self, name: &str) -> std::io::Result<()>;
    fn alive(&self, name: &str) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supervisor(max_attempts: u32) -> WorkerSupervisor {
        WorkerSupervisor::new(
            WorkerSpec {
                name: "stt".into(),
                program: PathBuf::from("python.exe"),
                args: vec![],
                readiness: Readiness::StdioHeartbeat,
            },
            RestartPolicy {
                max_attempts,
                window: Duration::from_secs(60),
                base_backoff: Duration::from_millis(100),
                max_backoff: Duration::from_secs(5),
            },
        )
    }

    #[test]
    fn status_transitions_are_validated() {
        let mut w = supervisor(3);
        assert!(w.transition(WorkerStatus::Ready).is_err());
        w.transition(WorkerStatus::Starting).unwrap();
        w.transition(WorkerStatus::Ready).unwrap();
        assert_eq!(w.status(), WorkerStatus::Ready);
    }

    #[test]
    fn backoff_grows_and_caps() {
        let w = supervisor(10);
        let p = &w.policy;
        assert_eq!(p.backoff(0), Duration::from_millis(100));
        assert_eq!(p.backoff(1), Duration::from_millis(200));
        assert_eq!(p.backoff(2), Duration::from_millis(400));
        assert_eq!(p.backoff(16), Duration::from_secs(5));
        assert_eq!(p.backoff(1000), Duration::from_secs(5));
    }

    #[test]
    fn budget_exhaustion_degrades_instead_of_looping() {
        let mut w = supervisor(2);
        let t0 = Instant::now();
        assert!(matches!(
            w.note_failure(t0, "crash"),
            RecoveryAction::Restart { .. }
        ));
        assert!(matches!(
            w.note_failure(t0 + Duration::from_secs(1), "crash"),
            RecoveryAction::Restart { .. }
        ));
        assert_eq!(
            w.note_failure(t0 + Duration::from_secs(2), "crash"),
            RecoveryAction::Degrade
        );
        assert_eq!(w.status(), WorkerStatus::Failed);
        // Attempts outside the window don't count against the budget.
        let mut w2 = supervisor(1);
        w2.note_failure(t0, "crash");
        let later = t0 + Duration::from_secs(120);
        assert!(matches!(
            w2.note_failure(later, "crash"),
            RecoveryAction::Restart { .. }
        ));
    }

    #[test]
    fn ready_resets_failure_accounting() {
        let mut w = supervisor(1);
        let t0 = Instant::now();
        w.note_failure(t0, "crash");
        w.transition(WorkerStatus::Starting).unwrap();
        w.note_ready(t0 + Duration::from_secs(1));
        assert_eq!(w.restart_attempts(), 0);
        assert_eq!(w.status(), WorkerStatus::Ready);
        assert_eq!(w.last_error(), None);
    }

    #[test]
    fn staleness_only_applies_to_ready_workers() {
        let mut w = supervisor(3);
        let t0 = Instant::now();
        assert!(!w.is_stale(t0 + Duration::from_secs(999), Duration::from_secs(30)));
        w.transition(WorkerStatus::Starting).unwrap();
        w.transition(WorkerStatus::Ready).unwrap();
        w.note_heartbeat(t0);
        assert!(!w.is_stale(t0 + Duration::from_secs(10), Duration::from_secs(30)));
        assert!(w.is_stale(t0 + Duration::from_secs(31), Duration::from_secs(30)));
    }
}
