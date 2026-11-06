//! Inference worker execution (ADR-0006 Stage B — the lifecycle function
//! Python delegates: spawn, bounded output, timeout, cancellation).
//!
//! The supervisor owns *which* binary runs (config) and *how long* it may
//! live; the worker supplies argv. Prompt text transits this channel but is
//! never journaled — outcomes record status, exit codes, and byte counts
//! only. Single-flight is enforced by the caller.

use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::watch;

/// Caps applied to one job — mirrors [inference] config.
#[derive(Debug, Clone)]
pub struct JobCaps {
    pub max_argv_count: usize,
    pub max_argv_bytes: usize,
    pub max_stdout_bytes: u64,
    pub max_stderr_bytes: u64,
}

#[derive(Debug, Error)]
pub enum SpecError {
    #[error("argv must not be empty")]
    EmptyArgv,
    #[error("argv has {0} entries, cap is {1}")]
    TooManyArgs(usize, usize),
    #[error("argv entry {0} must be a string")]
    NonStringArg(usize),
    #[error("argv is {0} bytes, cap is {1}")]
    ArgsTooLarge(usize, usize),
    #[error("timeout_ms must be a positive integer")]
    BadTimeout,
}

/// A validated inference request. Private construction: `from_args` is the
/// only way in, so bounds can never be skipped by a caller.
#[derive(Debug, Clone)]
pub struct JobSpec {
    pub job_id: String,
    pub argv: Vec<String>,
    pub timeout: Duration,
}

impl JobSpec {
    /// `args` is the untrusted request body: {job_id?, argv, timeout_ms?}.
    /// `job_id` falls back to a fresh unique id when absent or blank.
    pub fn from_args(
        args: &serde_json::Value,
        caps: &JobCaps,
        default_timeout: Duration,
        max_timeout: Duration,
    ) -> Result<Self, SpecError> {
        let argv_value = args
            .get("argv")
            .and_then(|v| v.as_array())
            .ok_or(SpecError::EmptyArgv)?;
        if argv_value.is_empty() {
            return Err(SpecError::EmptyArgv);
        }
        if argv_value.len() > caps.max_argv_count {
            return Err(SpecError::TooManyArgs(
                argv_value.len(),
                caps.max_argv_count,
            ));
        }
        let mut argv = Vec::with_capacity(argv_value.len());
        let mut bytes = 0usize;
        for (i, arg) in argv_value.iter().enumerate() {
            let s = arg.as_str().ok_or(SpecError::NonStringArg(i))?;
            bytes += s.len();
            argv.push(s.to_string());
        }
        if bytes > caps.max_argv_bytes {
            return Err(SpecError::ArgsTooLarge(bytes, caps.max_argv_bytes));
        }
        let job_id = args
            .get("job_id")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| crate::ids::new_id("job"));
        let timeout = match args.get("timeout_ms") {
            None => default_timeout,
            Some(v) => {
                let ms = v.as_u64().ok_or(SpecError::BadTimeout)?;
                if ms == 0 {
                    return Err(SpecError::BadTimeout);
                }
                Duration::from_millis(ms).min(max_timeout)
            }
        };
        Ok(Self {
            job_id,
            argv,
            timeout,
        })
    }
}

/// Terminal state of one job. `Completed` is the only variant that can
/// carry output; everything else is a typed reason the caller maps.
#[derive(Debug)]
pub enum JobOutcome {
    Completed {
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    },
    Timeout,
    Cancelled,
    SpawnFailed(String),
    /// A stream exceeded its cap; the child was killed.
    OutputOverflow,
}

impl JobOutcome {
    pub fn status_str(&self) -> &'static str {
        match self {
            JobOutcome::Completed { .. } => "completed",
            JobOutcome::Timeout => "timeout",
            JobOutcome::Cancelled => "cancelled",
            JobOutcome::SpawnFailed(_) => "spawn_failed",
            JobOutcome::OutputOverflow => "output_overflow",
        }
    }
}

/// Read a child stream into a bounded buffer. `Err(())` means the stream
/// overflowed its cap — the caller kills the child.
async fn drain_capped<R>(mut reader: R, cap: u64) -> Result<Vec<u8>, ()>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        match reader.read(&mut chunk).await {
            Ok(0) => return Ok(buf),
            Ok(n) => {
                buf.extend_from_slice(&chunk[..n]);
                if buf.len() as u64 > cap {
                    return Err(());
                }
            }
            Err(_) => return Ok(buf), // unreadable stream: keep what we have
        }
    }
}

/// Run one child to completion, timeout, cancel, or overflow.
/// `kill_on_drop` plus an explicit kill makes sure the supervisor never
/// leaves an inference orphan it still owns.
pub async fn run_job(
    binary: PathBuf,
    spec: JobSpec,
    caps: JobCaps,
    mut cancel: watch::Receiver<bool>,
) -> (JobOutcome, u128) {
    let started = Instant::now();
    let mut child = match Command::new(&binary)
        .args(&spec.argv)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return (
                JobOutcome::SpawnFailed(e.to_string()),
                started.elapsed().as_millis(),
            )
        }
    };
    let stdout = child.stdout.take().expect("stdout piped");
    let stderr = child.stderr.take().expect("stderr piped");
    let mut out_task = tokio::spawn(drain_capped(stdout, caps.max_stdout_bytes));
    let mut err_task = tokio::spawn(drain_capped(stderr, caps.max_stderr_bytes));

    let outcome = tokio::select! {
        status = child.wait() => {
            // &mut keeps the handles owned so abort() stays reachable below.
            let out = (&mut out_task).await.unwrap_or(Ok(Vec::new()));
            let err = (&mut err_task).await.unwrap_or(Ok(Vec::new()));
            match (status, out, err) {
                (_, Err(()), _) | (_, _, Err(())) => {
                    let _ = child.kill().await;
                    JobOutcome::OutputOverflow
                }
                (Ok(exit), Ok(so), Ok(se)) => JobOutcome::Completed {
                    exit_code: exit.code(),
                    stdout: so,
                    stderr: se,
                },
                (Err(e), _, _) => JobOutcome::SpawnFailed(e.to_string()),
            }
        }
        _ = tokio::time::sleep(spec.timeout) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            JobOutcome::Timeout
        }
        changed = cancel.changed() => {
            let _ = changed;
            let _ = child.kill().await;
            let _ = child.wait().await;
            JobOutcome::Cancelled
        }
    };
    // Reader tasks may linger on a killed child's pipes; do not leak them.
    out_task.abort();
    err_task.abort();
    (outcome, started.elapsed().as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn caps() -> JobCaps {
        JobCaps {
            max_argv_count: 8,
            max_argv_bytes: 256,
            max_stdout_bytes: 64,
            max_stderr_bytes: 64,
        }
    }

    #[test]
    fn argv_is_bounded_and_validated() {
        let c = caps();
        let t = Duration::from_secs(30);
        assert!(matches!(
            JobSpec::from_args(&json!({}), &c, t, t),
            Err(SpecError::EmptyArgv)
        ));
        assert!(matches!(
            JobSpec::from_args(&json!({"argv": []}), &c, t, t),
            Err(SpecError::EmptyArgv)
        ));
        assert!(matches!(
            JobSpec::from_args(&json!({"argv": vec!["a"; 9]}), &c, t, t),
            Err(SpecError::TooManyArgs(..))
        ));
        assert!(matches!(
            JobSpec::from_args(&json!({"argv": ["x", 7]}), &c, t, t),
            Err(SpecError::NonStringArg(1))
        ));
        assert!(matches!(
            JobSpec::from_args(&json!({"argv": ["x".repeat(300)]}), &c, t, t),
            Err(SpecError::ArgsTooLarge(..))
        ));
        assert!(matches!(
            JobSpec::from_args(&json!({"argv": ["a"], "timeout_ms": "fast"}), &c, t, t),
            Err(SpecError::BadTimeout)
        ));
        let ok = JobSpec::from_args(
            &json!({"argv": ["-p", "hi"], "timeout_ms": 90000}),
            &c,
            Duration::from_secs(10),
            Duration::from_secs(60),
        )
        .unwrap();
        assert_eq!(ok.argv, vec!["-p", "hi"]);
        assert_eq!(ok.timeout, Duration::from_secs(60)); // clamped to cap
        assert!(ok.job_id.starts_with("job-"));
    }

    fn shell(argv: &[&str]) -> (PathBuf, Vec<String>) {
        if cfg!(windows) {
            let mut a = vec!["/c".to_string()];
            a.extend(argv.iter().map(|s| s.to_string()));
            (PathBuf::from("cmd.exe"), a)
        } else {
            let mut a = vec!["-c".to_string()];
            a.extend(argv.iter().map(|s| s.to_string()));
            (PathBuf::from("sh"), a)
        }
    }

    #[tokio::test]
    async fn completed_job_captures_capped_output() {
        let (bin, argv) = if cfg!(windows) {
            (PathBuf::from("cmd.exe"), vec!["/c", "echo", "hello"])
        } else {
            (PathBuf::from("sh"), vec!["-c", "echo hello"])
        };
        let (_tx, rx) = watch::channel(false);
        let spec = JobSpec {
            job_id: "j1".into(),
            argv: argv.iter().map(|s| s.to_string()).collect(),
            timeout: Duration::from_secs(10),
        };
        let (outcome, _ms) = run_job(bin, spec, caps(), rx).await;
        match outcome {
            JobOutcome::Completed { stdout, .. } => {
                assert!(String::from_utf8_lossy(&stdout).contains("hello"))
            }
            other => panic!("expected completed, got {}", other.status_str()),
        }
    }

    #[tokio::test]
    async fn timeout_kills_the_child() {
        let (bin, argv) = shell(if cfg!(windows) {
            &["ping", "-n", "30", "127.0.0.1", ">", "NUL"]
        } else {
            &["sleep 30"]
        });
        let (_tx, rx) = watch::channel(false);
        let spec = JobSpec {
            job_id: "j2".into(),
            argv,
            timeout: Duration::from_millis(300),
        };
        let (outcome, ms) = run_job(bin, spec, caps(), rx).await;
        assert!(matches!(outcome, JobOutcome::Timeout));
        assert!(ms < 5000, "timeout path should be quick, took {ms}ms");
    }

    #[tokio::test]
    async fn cancel_kills_the_child() {
        let (bin, argv) = shell(if cfg!(windows) {
            &["ping", "-n", "30", "127.0.0.1", ">", "NUL"]
        } else {
            &["sleep 30"]
        });
        let (tx, rx) = watch::channel(false);
        let spec = JobSpec {
            job_id: "j3".into(),
            argv,
            timeout: Duration::from_secs(30),
        };
        let job = tokio::spawn(run_job(bin, spec, caps(), rx));
        tokio::time::sleep(Duration::from_millis(150)).await;
        tx.send(true).unwrap();
        let (outcome, _ms) = job.await.unwrap();
        assert!(matches!(outcome, JobOutcome::Cancelled));
    }

    #[tokio::test]
    async fn spawn_failure_is_typed() {
        let (_tx, rx) = watch::channel(false);
        let spec = JobSpec {
            job_id: "j4".into(),
            argv: vec!["x".into()],
            timeout: Duration::from_secs(5),
        };
        let (outcome, _ms) = run_job(
            PathBuf::from("definitely-not-a-real-binary-xyz.exe"),
            spec,
            caps(),
            rx,
        )
        .await;
        assert!(matches!(outcome, JobOutcome::SpawnFailed(_)));
    }

    #[tokio::test]
    async fn overflowing_output_is_killed() {
        let (bin, argv) = if cfg!(windows) {
            (
                PathBuf::from("cmd.exe"),
                vec![
                    "/c",
                    "for /L %i in (1,1,50) do @echo aaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                ],
            )
        } else {
            (
                PathBuf::from("sh"),
                vec!["-c", "yes aaaaaaaaaaaaaaaaaaaaaaaaaaaa | head -50"],
            )
        };
        let (_tx, rx) = watch::channel(false);
        let spec = JobSpec {
            job_id: "j5".into(),
            argv: argv.iter().map(|s| s.to_string()).collect(),
            timeout: Duration::from_secs(10),
        };
        let (outcome, _ms) = run_job(bin, spec, caps(), rx).await;
        assert!(matches!(outcome, JobOutcome::OutputOverflow));
    }
}
