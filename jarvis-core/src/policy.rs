//! The authorization boundary. The model may *request*; only the policy
//! engine may *authorize*.
//!
//! `AuthorizedAction` has private fields and no public constructor — the
//! compiler enforces that it can only come out of `PolicyEngine::evaluate`.
//! Authorization is also re-verified (expiry + revocation) at execution
//! time, because a minted token is not permission forever.
//!
//! Type-level guarantees are not an OS sandbox: process isolation, OS
//! permissions, and scoped IPC remain the executor's job (ADR-0006).

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use time::{Duration, OffsetDateTime};

use crate::ids;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Requester {
    User,
    Model,
    Scheduler,
    Worker,
    External,
}

impl Requester {
    pub fn as_str(self) -> &'static str {
        match self {
            Requester::User => "user",
            Requester::Model => "model",
            Requester::Scheduler => "scheduler",
            Requester::Worker => "worker",
            Requester::External => "external",
        }
    }
}

/// An untrusted ask. Everything here is treated as a claim until the engine
/// checks it against scope policy.
#[derive(Debug, Clone)]
pub struct ActionRequest {
    pub intent_id: String,
    pub requester: Requester,
    pub verb: String,
    /// Permission scope, e.g. "memory.note.write".
    pub scope: String,
    pub target: String,
    pub created_at_utc: OffsetDateTime,
    /// How long the request itself stays valid.
    pub ttl_seconds: u64,
    /// Externally visible or hard to undo — always requires confirmation.
    pub consequential: bool,
}

#[derive(Debug)]
pub enum Decision {
    Allow(AuthorizedAction),
    RequireConfirmation(ConfirmationRequest),
    Deny(DenialReason),
}

#[derive(Debug)]
pub struct ConfirmationRequest {
    pub intent_id: String,
    pub verb: String,
    pub scope: String,
    pub target: String,
    /// Stable reason shown to the user.
    pub why: &'static str,
    pub confirm_by_utc: OffsetDateTime,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DenialReason {
    #[error("request is malformed: {0}")]
    MalformedRequest(&'static str),
    #[error("request expired before evaluation")]
    ExpiredRequest,
    #[error("scope {0} is not a declared scope")]
    UnknownScope(String),
    #[error("{requester:?} may not request scope {scope}")]
    RequesterCannotAuthorize { requester: Requester, scope: String },
    #[error("scope {0} has been revoked")]
    RevokedScope(String),
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ExecutionError {
    #[error("authorized action expired")]
    Expired,
    #[error("authorized action was revoked")]
    Revoked,
}

/// Proof of a successful policy evaluation. Private fields, no public
/// constructor: minting is impossible outside this module.
#[derive(Debug)]
pub struct AuthorizedAction {
    operation_id: String,
    intent_id: String,
    verb: String,
    scope: String,
    target: String,
    issued_at_utc: OffsetDateTime,
    expires_at_utc: OffsetDateTime,
}

impl AuthorizedAction {
    fn mint(request: &ActionRequest, issued: OffsetDateTime, ttl: Duration) -> Self {
        Self {
            operation_id: ids::new_id("op"),
            intent_id: request.intent_id.clone(),
            verb: request.verb.clone(),
            scope: request.scope.clone(),
            target: request.target.clone(),
            issued_at_utc: issued,
            expires_at_utc: issued + ttl,
        }
    }

    pub fn operation_id(&self) -> &str {
        &self.operation_id
    }
    pub fn intent_id(&self) -> &str {
        &self.intent_id
    }
    pub fn verb(&self) -> &str {
        &self.verb
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
    pub fn target(&self) -> &str {
        &self.target
    }
    pub fn expires_at_utc(&self) -> OffsetDateTime {
        self.expires_at_utc
    }
    pub fn issued_at_utc(&self) -> OffsetDateTime {
        self.issued_at_utc
    }

    /// Execution-time check: expiry and *current* revocation state. An
    /// action valid at issue can still be revoked before it runs.
    pub fn verify_at_execution(
        &self,
        now: OffsetDateTime,
        engine: &PolicyEngine,
    ) -> Result<(), ExecutionError> {
        if now >= self.expires_at_utc {
            return Err(ExecutionError::Expired);
        }
        if engine.is_scope_revoked(&self.scope) || engine.is_operation_revoked(&self.operation_id) {
            return Err(ExecutionError::Revoked);
        }
        Ok(())
    }
}

struct ScopeRule {
    allowed_requesters: &'static [Requester],
    /// A model may propose it, but a human must confirm.
    model_may_propose: bool,
    /// Actions minted under this scope live this long.
    action_ttl: Duration,
}

/// Deterministic scope table. Mirrors — does not widen — the existing
/// Python gates (`intent_gate.py`, `capability_registry.py`).
fn default_rules() -> HashMap<String, ScopeRule> {
    let mut rules = HashMap::new();
    rules.insert(
        "memory.note.write".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::User],
            model_may_propose: true,
            action_ttl: Duration::seconds(60),
        },
    );
    rules.insert(
        "memory.candidate.queue".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::User],
            model_may_propose: true,
            action_ttl: Duration::seconds(60),
        },
    );
    rules.insert(
        "memory.knowledge.read".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::User, Requester::Scheduler, Requester::Worker],
            model_may_propose: true,
            action_ttl: Duration::seconds(30),
        },
    );
    rules.insert(
        "ops.obligation.run".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::Scheduler, Requester::User],
            model_may_propose: false,
            action_ttl: Duration::seconds(120),
        },
    );
    rules.insert(
        "ops.inference.run".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::Worker, Requester::User],
            model_may_propose: false,
            action_ttl: Duration::seconds(300),
        },
    );
    rules.insert(
        "ops.worker.restart".to_string(),
        ScopeRule {
            allowed_requesters: &[Requester::Scheduler, Requester::User],
            model_may_propose: false,
            action_ttl: Duration::seconds(120),
        },
    );
    rules.insert(
        "ops.telemetry.record".to_string(),
        ScopeRule {
            allowed_requesters: &[
                Requester::User,
                Requester::Scheduler,
                Requester::Worker,
                Requester::Model,
            ],
            model_may_propose: false,
            action_ttl: Duration::seconds(30),
        },
    );
    rules
}

pub struct PolicyEngine {
    rules: HashMap<String, ScopeRule>,
    revoked_scopes: HashSet<String>,
    revoked_operations: HashSet<String>,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            rules: default_rules(),
            revoked_scopes: HashSet::new(),
            revoked_operations: HashSet::new(),
        }
    }

    pub fn revoke_scope(&mut self, scope: &str) {
        self.revoked_scopes.insert(scope.to_string());
    }

    pub fn revoke_operation(&mut self, operation_id: &str) {
        self.revoked_operations.insert(operation_id.to_string());
    }

    pub fn is_scope_revoked(&self, scope: &str) -> bool {
        self.revoked_scopes.contains(scope)
    }

    pub fn is_operation_revoked(&self, operation_id: &str) -> bool {
        self.revoked_operations.contains(operation_id)
    }

    /// Evaluate an untrusted request. Order matters: malformed and expired
    /// requests are denied before scope is even consulted.
    pub fn evaluate(&self, request: &ActionRequest, now: OffsetDateTime) -> Decision {
        if request.intent_id.is_empty() {
            return Decision::Deny(DenialReason::MalformedRequest("intent_id"));
        }
        if request.verb.is_empty() {
            return Decision::Deny(DenialReason::MalformedRequest("verb"));
        }
        if request.scope.is_empty() {
            return Decision::Deny(DenialReason::MalformedRequest("scope"));
        }
        if now > request.created_at_utc + Duration::seconds(request.ttl_seconds.max(1) as i64) {
            return Decision::Deny(DenialReason::ExpiredRequest);
        }
        if self.is_scope_revoked(&request.scope) {
            return Decision::Deny(DenialReason::RevokedScope(request.scope.clone()));
        }
        let Some(rule) = self.rules.get(&request.scope) else {
            return Decision::Deny(DenialReason::UnknownScope(request.scope.clone()));
        };

        let confirm = |why: &'static str| {
            Decision::RequireConfirmation(ConfirmationRequest {
                intent_id: request.intent_id.clone(),
                verb: request.verb.clone(),
                scope: request.scope.clone(),
                target: request.target.clone(),
                why,
                confirm_by_utc: now + Duration::seconds(300),
            })
        };

        if !rule.allowed_requesters.contains(&request.requester) {
            if request.requester == Requester::Model && rule.model_may_propose {
                // A model may propose, never mint.
                return confirm("model_proposal_requires_confirmation");
            }
            return Decision::Deny(DenialReason::RequesterCannotAuthorize {
                requester: request.requester,
                scope: request.scope.clone(),
            });
        }
        if request.consequential {
            // Consequential operations always ask a human first.
            return confirm("consequential_requires_confirmation");
        }
        Decision::Allow(AuthorizedAction::mint(request, now, rule.action_ttl))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(requester: Requester, scope: &str) -> ActionRequest {
        ActionRequest {
            intent_id: "intent-1".into(),
            requester,
            verb: "write_note".into(),
            scope: scope.into(),
            target: "Jarvis/Notes".into(),
            created_at_utc: OffsetDateTime::now_utc(),
            ttl_seconds: 60,
            consequential: false,
        }
    }

    #[test]
    fn user_note_write_is_allowed_and_mints() {
        let engine = PolicyEngine::new();
        match engine.evaluate(
            &req(Requester::User, "memory.note.write"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Allow(action) => {
                assert!(!action.operation_id().is_empty());
                action
                    .verify_at_execution(OffsetDateTime::now_utc(), &engine)
                    .unwrap();
            }
            other => panic!("expected Allow, got {other:?}"),
        }
    }

    #[test]
    fn model_proposal_needs_confirmation_never_mints() {
        let engine = PolicyEngine::new();
        match engine.evaluate(
            &req(Requester::Model, "memory.note.write"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::RequireConfirmation(c) => {
                assert_eq!(c.why, "model_proposal_requires_confirmation")
            }
            other => panic!("expected confirmation, got {other:?}"),
        }
        // And for scopes models may not even propose: hard deny.
        match engine.evaluate(
            &req(Requester::Model, "ops.worker.restart"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Deny(DenialReason::RequesterCannotAuthorize { .. }) => {}
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn unknown_scope_is_denied() {
        let engine = PolicyEngine::new();
        match engine.evaluate(
            &req(Requester::User, "os.shell.exec"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Deny(DenialReason::UnknownScope(s)) => assert_eq!(s, "os.shell.exec"),
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn stale_request_is_denied() {
        let engine = PolicyEngine::new();
        let mut r = req(Requester::User, "memory.note.write");
        r.created_at_utc = OffsetDateTime::now_utc() - Duration::hours(1);
        match engine.evaluate(&r, OffsetDateTime::now_utc()) {
            Decision::Deny(DenialReason::ExpiredRequest) => {}
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn revocation_invalidates_new_and_minted_actions() {
        let mut engine = PolicyEngine::new();
        let action = match engine.evaluate(
            &req(Requester::User, "memory.note.write"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Allow(a) => a,
            _ => panic!(),
        };
        engine.revoke_scope("memory.note.write");
        // Minted action now fails execution-time verification.
        assert_eq!(
            action.verify_at_execution(OffsetDateTime::now_utc(), &engine),
            Err(ExecutionError::Revoked)
        );
        // And no new action can be minted in the revoked scope.
        match engine.evaluate(
            &req(Requester::User, "memory.note.write"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Deny(DenialReason::RevokedScope(_)) => {}
            other => panic!("expected deny, got {other:?}"),
        }
    }

    #[test]
    fn consequential_always_confirms() {
        let engine = PolicyEngine::new();
        let mut r = req(Requester::User, "memory.note.write");
        r.consequential = true;
        assert!(matches!(
            engine.evaluate(&r, OffsetDateTime::now_utc()),
            Decision::RequireConfirmation(_)
        ));
    }

    #[test]
    fn minted_action_expires() {
        let engine = PolicyEngine::new();
        let action = match engine.evaluate(
            &req(Requester::User, "memory.note.write"),
            OffsetDateTime::now_utc(),
        ) {
            Decision::Allow(a) => a,
            _ => panic!(),
        };
        let later = action.expires_at_utc() + Duration::seconds(1);
        assert_eq!(
            action.verify_at_execution(later, &engine),
            Err(ExecutionError::Expired)
        );
    }
}
