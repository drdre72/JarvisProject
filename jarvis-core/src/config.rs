//! Configuration loading and validation for the supervisor.
//!
//! `jarvis-core.toml` at the repo root is the source. Missing file means
//! defaults; an explicitly passed missing or malformed path is an error.

use serde::Deserialize;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const DEFAULT_CONFIG_PATH: &str = "jarvis-core.toml";

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("cannot read config file {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid config file {path}: {source}")]
    Parse {
        path: PathBuf,
        source: Box<toml::de::Error>,
    },
    #[error("invalid configuration: {0}")]
    Validation(&'static str),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RustCoreSection {
    /// Master feature flag. When false the binary exits immediately and the
    /// legacy Python path is the only runtime (ADR-0006 Stage A default).
    pub enabled: bool,
    /// Evaluate and journal every decision but dispatch nothing consequential.
    pub shadow_mode: bool,
    /// Operational data directory (journal database). Gitignored `logs/`.
    pub data_dir: PathBuf,
    pub event_queue_capacity: usize,
    pub dedup_window_seconds: u64,
    pub max_payload_bytes: usize,
    /// Per-source admission cap within one dedup window (rate limiting).
    pub per_source_burst: usize,
    /// Stage B gate: false answers every request frame with `ipc_disabled`
    /// so the limited function can be rolled back without killing the loop.
    pub ipc_requests: bool,
    /// Stage B gate for the inference-lifecycle function specifically.
    /// false answers `run_inference`/`cancel_inference` with
    /// `inference_disabled`; the Python side then keeps its own subprocess
    /// path — this flag IS the rollback.
    pub inference_relay: bool,
}

impl Default for RustCoreSection {
    fn default() -> Self {
        Self {
            enabled: false,
            shadow_mode: true,
            data_dir: PathBuf::from("logs/rust-core"),
            event_queue_capacity: 256,
            dedup_window_seconds: 30,
            max_payload_bytes: 16 * 1024,
            per_source_burst: 32,
            ipc_requests: false,
            inference_relay: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct SchedulerSection {
    /// Wall-clock obligations are re-checked on this monotonic cadence.
    pub tick_seconds: u64,
    /// Hard cap on overdue work released per tick (burst control).
    pub max_due_per_tick: usize,
}

impl Default for SchedulerSection {
    fn default() -> Self {
        Self {
            tick_seconds: 1,
            max_due_per_tick: 8,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkersSection {
    pub heartbeat_timeout_seconds: u64,
    pub max_restart_attempts: u32,
    pub base_backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for WorkersSection {
    fn default() -> Self {
        Self {
            heartbeat_timeout_seconds: 30,
            max_restart_attempts: 3,
            base_backoff_ms: 500,
            max_backoff_ms: 30_000,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct InferenceSection {
    /// The binary the supervisor may spawn — workers supply argv only, so
    /// the executable itself stays under supervisor authority.
    pub binary: PathBuf,
    /// Default per-job wall-clock budget; requests may clamp lower.
    pub default_timeout_seconds: u64,
    /// Hard ceiling any request-side timeout is clamped to.
    pub max_timeout_seconds: u64,
    /// In-flight job cap. 1 matches the serialized turn pipeline today.
    pub max_concurrent: usize,
    pub max_argv_count: usize,
    /// Total bytes across all argv entries — the prompt travels inside it.
    pub max_argv_bytes: usize,
    /// Captured output caps; over-cap kills the child as output_overflow.
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
}

impl Default for InferenceSection {
    fn default() -> Self {
        Self {
            binary: PathBuf::from("bin/llama-completion.exe"),
            default_timeout_seconds: 120,
            max_timeout_seconds: 300,
            max_concurrent: 1,
            max_argv_count: 64,
            max_argv_bytes: 48 * 1024,
            max_stdout_bytes: 32 * 1024,
            max_stderr_bytes: 8 * 1024,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct CoreConfig {
    pub rust_core: RustCoreSection,
    pub scheduler: SchedulerSection,
    pub workers: WorkersSection,
    pub inference: InferenceSection,
    /// Directory the config file lived in — the anchor for relative paths
    /// like `inference.binary`. Set by `load`; empty for default configs.
    #[serde(skip)]
    pub config_dir: PathBuf,
}

impl CoreConfig {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            if path == Path::new(DEFAULT_CONFIG_PATH) {
                let cfg = Self::default();
                cfg.validate()?;
                return Ok(cfg);
            }
            return Err(ConfigError::Io {
                path: path.to_path_buf(),
                source: std::io::Error::new(std::io::ErrorKind::NotFound, "config file"),
            });
        }
        let text = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        let mut cfg: CoreConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;
        cfg.config_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();
        cfg.validate()?;
        Ok(cfg)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let core = &self.rust_core;
        if core.event_queue_capacity == 0 {
            return Err(ConfigError::Validation("event_queue_capacity must be >= 1"));
        }
        if core.per_source_burst == 0 {
            return Err(ConfigError::Validation("per_source_burst must be >= 1"));
        }
        if core.per_source_burst > core.event_queue_capacity {
            return Err(ConfigError::Validation(
                "per_source_burst must not exceed event_queue_capacity",
            ));
        }
        if core.max_payload_bytes == 0 || core.max_payload_bytes > 1024 * 1024 {
            return Err(ConfigError::Validation(
                "max_payload_bytes must be in 1..=1048576",
            ));
        }
        if core.dedup_window_seconds == 0 {
            return Err(ConfigError::Validation("dedup_window_seconds must be >= 1"));
        }
        if self.scheduler.tick_seconds == 0 {
            return Err(ConfigError::Validation(
                "scheduler.tick_seconds must be >= 1",
            ));
        }
        if self.scheduler.max_due_per_tick == 0 {
            return Err(ConfigError::Validation(
                "scheduler.max_due_per_tick must be >= 1",
            ));
        }
        if self.workers.heartbeat_timeout_seconds == 0 {
            return Err(ConfigError::Validation(
                "workers.heartbeat_timeout_seconds must be >= 1",
            ));
        }
        if self.workers.base_backoff_ms == 0 || self.workers.max_backoff_ms == 0 {
            return Err(ConfigError::Validation(
                "worker backoff values must be >= 1",
            ));
        }
        if self.workers.base_backoff_ms > self.workers.max_backoff_ms {
            return Err(ConfigError::Validation(
                "workers.base_backoff_ms must not exceed max_backoff_ms",
            ));
        }
        let inf = &self.inference;
        if inf.max_concurrent == 0 || inf.max_concurrent > 4 {
            return Err(ConfigError::Validation(
                "inference.max_concurrent must be in 1..=4",
            ));
        }
        if inf.default_timeout_seconds == 0 || inf.max_timeout_seconds == 0 {
            return Err(ConfigError::Validation("inference timeouts must be >= 1"));
        }
        if inf.default_timeout_seconds > inf.max_timeout_seconds {
            return Err(ConfigError::Validation(
                "inference.default_timeout_seconds must not exceed max_timeout_seconds",
            ));
        }
        if inf.max_argv_count == 0 || inf.max_argv_bytes == 0 {
            return Err(ConfigError::Validation("inference argv caps must be >= 1"));
        }
        if inf.max_stdout_bytes == 0 || inf.max_stderr_bytes == 0 {
            return Err(ConfigError::Validation(
                "inference output caps must be >= 1",
            ));
        }
        Ok(())
    }

    /// SQLite operational journal (separate from the Markdown personal vault).
    pub fn journal_path(&self) -> PathBuf {
        self.rust_core.data_dir.join("operational-journal.db")
    }

    /// The worker binary Rust may spawn, resolved against the config file's
    /// directory so a relative `inference.binary` does not depend on CWD.
    pub fn inference_binary(&self) -> PathBuf {
        let b = &self.inference.binary;
        if b.is_absolute() {
            b.clone()
        } else {
            self.config_dir.join(b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_validate() {
        CoreConfig::default().validate().unwrap();
    }

    #[test]
    fn missing_default_path_yields_defaults() {
        // Only meaningful while no local jarvis-core.toml sits in the crate dir.
        if Path::new(DEFAULT_CONFIG_PATH).exists() {
            return;
        }
        let cfg = CoreConfig::load(Path::new(DEFAULT_CONFIG_PATH)).unwrap();
        assert!(!cfg.rust_core.enabled);
        assert!(cfg.rust_core.shadow_mode);
    }

    #[test]
    fn parses_rust_core_flag_shape() {
        let text = r#"
[rust_core]
enabled = true
shadow_mode = true
data_dir = "logs/rust-core"
event_queue_capacity = 64
"#;
        let cfg: CoreConfig = toml::from_str(text).unwrap();
        cfg.validate().unwrap();
        assert!(cfg.rust_core.enabled);
        assert_eq!(cfg.rust_core.event_queue_capacity, 64);
        assert_eq!(cfg.scheduler.max_due_per_tick, 8);
    }

    #[test]
    fn rejects_zero_queue_capacity() {
        let text = r#"
[rust_core]
event_queue_capacity = 0
"#;
        let cfg: CoreConfig = toml::from_str(text).unwrap();
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn rejects_oversized_burst() {
        let text = r#"
[rust_core]
event_queue_capacity = 8
per_source_burst = 64
"#;
        let cfg: CoreConfig = toml::from_str(text).unwrap();
        assert!(cfg.validate().is_err());
    }
}
