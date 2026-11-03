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

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct CoreConfig {
    pub rust_core: RustCoreSection,
    pub scheduler: SchedulerSection,
    pub workers: WorkersSection,
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
        let cfg: CoreConfig = toml::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })?;
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
        Ok(())
    }

    /// SQLite operational journal (separate from the Markdown personal vault).
    pub fn journal_path(&self) -> PathBuf {
        self.rust_core.data_dir.join("operational-journal.db")
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
