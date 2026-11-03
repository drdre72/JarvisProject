//! jarvis-core — JARVIS supervisory runtime (System A).
//!
//! Durable operational continuity only: typed events, bounded queues,
//! scheduling, a transactional journal, worker supervision, and the policy
//! boundary. Intelligence (STT/TTS/LLM/UI) stays in the Python runtime.
//! In shadow mode (ADR-0006 Stage A) the supervisor evaluates and journals
//! every decision but dispatches no consequential action.

pub mod attention;
pub mod config;
pub mod events;
pub mod health;
pub mod journal;
pub mod policy;
pub mod scheduler;
pub mod state;
pub mod supervisor;
pub mod transport;
pub mod workers;

/// Process-unique, sortable-ish identifiers without an external dependency.
/// Collisions across restarts are prevented by the nanosecond component.
pub mod ids {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    pub fn new_id(prefix: &str) -> String {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("{prefix}-{nanos:016x}-{n:06x}")
    }
}
