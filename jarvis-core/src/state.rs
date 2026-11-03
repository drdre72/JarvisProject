//! Authoritative lifecycle states. Invalid states are unrepresentable:
//! transitions are validated against an explicit edge table and produce
//! typed errors rather than silently corrupting state.
//!
//! Model residency is tracked independently — a model may stay loaded while
//! the supervisor monitors, and residency never implies cognition.

use std::collections::VecDeque;
use thiserror::Error;

/// The single source of truth for what the supervisor is doing.
/// `ShuttingDown` is terminal: every state may enter it, none may leave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CognitiveState {
    Monitoring,
    AttentionPending,
    WorkerStarting,
    Reasoning,
    Executing,
    Cooldown,
    Degraded,
    ShuttingDown,
}

/// Orthogonal to `CognitiveState` — residency is a resource policy, not a mood.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelResidency {
    Unloaded,
    Loading,
    Loaded,
    Unloading,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid cognitive transition {from:?} -> {to:?}")]
pub struct InvalidTransition {
    pub from: CognitiveState,
    pub to: CognitiveState,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid model-residency transition {from:?} -> {to:?}")]
pub struct InvalidResidencyTransition {
    pub from: ModelResidency,
    pub to: ModelResidency,
}

#[derive(Debug, Clone)]
pub struct TransitionRecord {
    pub from: CognitiveState,
    pub to: CognitiveState,
    /// Stable machine-readable reason code; logged and journaled.
    pub reason: &'static str,
}

impl CognitiveState {
    /// The complete legal edge set.
    pub fn may_transition_to(self, next: CognitiveState) -> bool {
        use CognitiveState::*;
        match (self, next) {
            (ShuttingDown, _) => false,
            (_, ShuttingDown) => true,
            (Monitoring, AttentionPending | Degraded) => true,
            (AttentionPending, WorkerStarting | Cooldown | Monitoring | Degraded) => true,
            (WorkerStarting, Reasoning | Cooldown | Degraded) => true,
            (Reasoning, Executing | Cooldown | Degraded) => true,
            (Executing, Cooldown | Degraded) => true,
            (Cooldown, Monitoring | Degraded) => true,
            (Degraded, Monitoring) => true,
            _ => false,
        }
    }
}

impl ModelResidency {
    pub fn may_transition_to(self, next: ModelResidency) -> bool {
        use ModelResidency::*;
        matches!(
            (self, next),
            (Unloaded, Loading)
                | (Loading, Loaded | Unloaded)
                | (Loaded, Unloading)
                | (Unloading, Unloaded)
        )
    }
}

/// Owns the authoritative state. Only this type mutates it, and only through
/// validated transitions. Transition history is bounded and reason-coded.
pub struct CognitiveMachine {
    state: CognitiveState,
    residency: ModelResidency,
    history: VecDeque<TransitionRecord>,
    max_history: usize,
}

impl CognitiveMachine {
    pub fn new() -> Self {
        Self {
            state: CognitiveState::Monitoring,
            residency: ModelResidency::Unloaded,
            history: VecDeque::with_capacity(64),
            max_history: 64,
        }
    }

    pub fn state(&self) -> CognitiveState {
        self.state
    }

    pub fn residency(&self) -> ModelResidency {
        self.residency
    }

    pub fn history(&self) -> &VecDeque<TransitionRecord> {
        &self.history
    }

    pub fn transition(
        &mut self,
        to: CognitiveState,
        reason: &'static str,
    ) -> Result<CognitiveState, InvalidTransition> {
        if !self.state.may_transition_to(to) {
            return Err(InvalidTransition {
                from: self.state,
                to,
            });
        }
        let from = std::mem::replace(&mut self.state, to);
        self.push_history(TransitionRecord { from, to, reason });
        Ok(to)
    }

    pub fn transition_residency(
        &mut self,
        to: ModelResidency,
    ) -> Result<ModelResidency, InvalidResidencyTransition> {
        if !self.residency.may_transition_to(to) {
            return Err(InvalidResidencyTransition {
                from: self.residency,
                to,
            });
        }
        self.residency = to;
        Ok(to)
    }

    fn push_history(&mut self, record: TransitionRecord) {
        if self.history.len() == self.max_history {
            self.history.pop_front();
        }
        self.history.push_back(record);
    }
}

impl Default for CognitiveMachine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_cycles_back_to_monitoring() {
        let mut m = CognitiveMachine::new();
        use CognitiveState::*;
        for (to, why) in [
            (AttentionPending, "event"),
            (WorkerStarting, "attend"),
            (Reasoning, "worker_ready"),
            (Executing, "authorized"),
            (Cooldown, "done"),
            (Monitoring, "cooled"),
        ] {
            m.transition(to, why).unwrap();
        }
        assert_eq!(m.state(), Monitoring);
        assert_eq!(m.history().len(), 6);
    }

    #[test]
    fn invalid_transition_is_rejected_and_state_unchanged() {
        let mut m = CognitiveMachine::new();
        let err = m.transition(CognitiveState::Executing, "skip").unwrap_err();
        assert_eq!(
            err,
            InvalidTransition {
                from: CognitiveState::Monitoring,
                to: CognitiveState::Executing
            }
        );
        assert_eq!(m.state(), CognitiveState::Monitoring);
        assert!(m.history().is_empty());
    }

    #[test]
    fn self_transition_is_rejected() {
        let mut m = CognitiveMachine::new();
        assert!(m.transition(CognitiveState::Monitoring, "noop").is_err());
    }

    #[test]
    fn degraded_recovers_only_to_monitoring() {
        let mut m = CognitiveMachine::new();
        m.transition(CognitiveState::Degraded, "fault").unwrap();
        assert!(m.transition(CognitiveState::Reasoning, "jump").is_err());
        m.transition(CognitiveState::Monitoring, "recovered")
            .unwrap();
        assert_eq!(m.state(), CognitiveState::Monitoring);
    }

    #[test]
    fn shutdown_is_terminal() {
        let mut m = CognitiveMachine::new();
        m.transition(CognitiveState::AttentionPending, "event")
            .unwrap();
        m.transition(CognitiveState::ShuttingDown, "sigterm")
            .unwrap();
        assert!(m.transition(CognitiveState::Monitoring, "reopen").is_err());
    }

    #[test]
    fn residency_is_independent() {
        let mut m = CognitiveMachine::new();
        // Model loading does not change cognitive state.
        m.transition_residency(ModelResidency::Loading).unwrap();
        m.transition_residency(ModelResidency::Loaded).unwrap();
        assert_eq!(m.state(), CognitiveState::Monitoring);
        assert_eq!(m.residency(), ModelResidency::Loaded);
        assert!(m.transition_residency(ModelResidency::Loading).is_err());
        m.transition_residency(ModelResidency::Unloading).unwrap();
        m.transition_residency(ModelResidency::Unloaded).unwrap();
    }

    #[test]
    fn history_is_bounded() {
        let mut m = CognitiveMachine::new();
        for _ in 0..200 {
            m.transition(CognitiveState::AttentionPending, "e").unwrap();
            m.transition(CognitiveState::Monitoring, "d").unwrap();
        }
        assert_eq!(m.history().len(), 64);
    }
}
