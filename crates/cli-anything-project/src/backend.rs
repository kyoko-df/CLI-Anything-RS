//! Backend subprocess adapter.
//!
//! Generated packages delegate heavy operations (batch rendering,
//! headless rendering, diagram export) to the upstream GUI's command-line
//! entrypoint: `gimp -i -b`, `blender --background --python`,
//! `draw.io -x --format ...`, and so on. To keep that surface uniform
//! and testable the package crates talk to a [`Backend`] trait instead of
//! shelling out directly.
//!
//! The crate ships two implementations:
//!
//! * [`DryRunBackend`] records every requested invocation without
//!   touching the system. Generated packages use this by default so that
//!   smoke tests stay hermetic.
//! * [`SystemBackend`] resolves the configured command with
//!   `std::process::Command` and actually executes it. Package binaries
//!   opt in via a feature flag / env variable when they want to drive
//!   the real GUI.
//!
//! The goal is not to hide the backend command entirely – callers still
//! know they are orchestrating `gimp -i -b script-fu` – it is only to
//! provide a single seam for tests, dry-runs, and (later) transport
//! swaps (SSH, sandbox, remote worker).

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Fully-specified invocation of a backend command. Generated packages
/// build these from the user's CLI arguments and hand them to a
/// [`Backend`] implementation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendInvocation {
    /// Program name to execute (e.g. `gimp`, `blender`, `draw.io`).
    pub program: String,
    /// Arguments to pass, already split into tokens.
    pub args: Vec<String>,
    /// Optional working directory. `None` means inherit from the
    /// current process.
    pub working_dir: Option<PathBuf>,
    /// Human-readable label for log lines and dry-run output.
    pub label: String,
}

impl BackendInvocation {
    pub fn new(program: impl Into<String>, args: Vec<String>, label: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args,
            working_dir: None,
            label: label.into(),
        }
    }

    pub fn with_working_dir(mut self, working_dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(working_dir.into());
        self
    }

    /// Render the invocation as a shell-style command line for logs.
    pub fn display_command(&self) -> String {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(self.program.clone());
        parts.extend(self.args.iter().cloned());
        parts.join(" ")
    }
}

/// Outcome of running a backend invocation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackendOutcome {
    pub invocation: BackendInvocation,
    pub status: BackendStatus,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendStatus {
    /// The adapter did not execute the command; it only recorded it.
    DryRun,
    /// The real process exited with status 0.
    Success,
    /// The real process exited with a non-zero status.
    Failed,
}

/// Shared error type so callers can surface backend failures through the
/// normal `anyhow::Error` pipeline.
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("failed to spawn {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("backend {program} exited with status {code:?}: {stderr}")]
    NonZeroExit {
        program: String,
        code: Option<i32>,
        stderr: String,
    },
}

/// Trait implemented by every backend adapter. Consumers typically hold
/// a `Box<dyn Backend>` or an `Arc<dyn Backend>` so both the dry-run
/// and system implementations can be swapped at runtime.
pub trait Backend: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&self, invocation: BackendInvocation) -> Result<BackendOutcome>;
}

/// Records every invocation without running anything. Useful for tests,
/// validation, and the default configuration of generated packages.
#[derive(Debug, Default, Clone)]
pub struct DryRunBackend {
    recorded: Arc<Mutex<Vec<BackendInvocation>>>,
}

impl DryRunBackend {
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of the invocations recorded so far.
    pub fn recorded(&self) -> Vec<BackendInvocation> {
        self.recorded
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }
}

impl Backend for DryRunBackend {
    fn name(&self) -> &'static str {
        "dry-run"
    }

    fn execute(&self, invocation: BackendInvocation) -> Result<BackendOutcome> {
        let stdout = format!("[dry-run] {}", invocation.display_command());
        if let Ok(mut guard) = self.recorded.lock() {
            guard.push(invocation.clone());
        }
        Ok(BackendOutcome {
            invocation,
            status: BackendStatus::DryRun,
            stdout,
            stderr: String::new(),
            exit_code: None,
        })
    }
}

/// Executes invocations through [`std::process::Command`]. Only used
/// when the caller explicitly opts in – otherwise the dry-run backend is
/// the safer default.
#[derive(Debug, Default, Clone)]
pub struct SystemBackend;

impl SystemBackend {
    pub fn new() -> Self {
        Self
    }
}

impl Backend for SystemBackend {
    fn name(&self) -> &'static str {
        "system"
    }

    fn execute(&self, invocation: BackendInvocation) -> Result<BackendOutcome> {
        let mut command = Command::new(&invocation.program);
        command.args(&invocation.args);
        if let Some(dir) = &invocation.working_dir {
            command.current_dir(dir);
        }

        let output = command.output().map_err(|err| BackendError::Spawn {
            program: invocation.program.clone(),
            source: err,
        })?;

        let exit_code = output.status.code();
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        let status = if output.status.success() {
            BackendStatus::Success
        } else {
            BackendStatus::Failed
        };

        Ok(BackendOutcome {
            invocation,
            status,
            stdout,
            stderr,
            exit_code,
        })
    }
}

/// Select the right backend based on an environment variable. Generated
/// packages call this in `main` so that tests default to the dry-run
/// adapter while operators can flip `CLI_ANYTHING_BACKEND=system` to hit
/// the real GUI.
pub const BACKEND_MODE_ENV: &str = "CLI_ANYTHING_BACKEND";

pub fn backend_from_env() -> Arc<dyn Backend> {
    match std::env::var(BACKEND_MODE_ENV).as_deref() {
        Ok("system") => Arc::new(SystemBackend::new()),
        _ => Arc::new(DryRunBackend::new()),
    }
}

/// Helper used by adapters that need to fail if the backend command
/// ended with a non-zero status.
pub fn ensure_success(outcome: &BackendOutcome) -> Result<()> {
    match outcome.status {
        BackendStatus::Success | BackendStatus::DryRun => Ok(()),
        BackendStatus::Failed => Err(BackendError::NonZeroExit {
            program: outcome.invocation.program.clone(),
            code: outcome.exit_code,
            stderr: outcome.stderr.clone(),
        })
        .context("backend command failed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dry_run_backend_records_invocations_without_running() {
        let backend = DryRunBackend::new();
        let invocation = BackendInvocation::new(
            "gimp",
            vec![
                "-i".to_string(),
                "-b".to_string(),
                "(plug-in-blur 1 1 1)".to_string(),
            ],
            "blur-default-layer",
        );

        let outcome = backend
            .execute(invocation.clone())
            .expect("dry run should succeed");

        assert_eq!(outcome.status, BackendStatus::DryRun);
        assert!(outcome.stdout.contains("gimp -i -b"));
        assert_eq!(backend.recorded().len(), 1);
        assert_eq!(backend.recorded()[0], invocation);
    }

    #[test]
    fn ensure_success_accepts_dry_run_and_success() {
        let outcome = BackendOutcome {
            invocation: BackendInvocation::new("gimp", vec![], "noop"),
            status: BackendStatus::DryRun,
            stdout: String::new(),
            stderr: String::new(),
            exit_code: None,
        };
        ensure_success(&outcome).expect("dry run should be accepted");
    }

    #[test]
    fn ensure_success_rejects_failure() {
        let outcome = BackendOutcome {
            invocation: BackendInvocation::new("gimp", vec![], "noop"),
            status: BackendStatus::Failed,
            stdout: String::new(),
            stderr: "boom".to_string(),
            exit_code: Some(2),
        };
        assert!(ensure_success(&outcome).is_err());
    }

    #[test]
    fn system_backend_runs_a_command_and_captures_stdout() {
        let backend = SystemBackend::new();
        // `true` on Unix and `cmd /C exit 0` on Windows both exit 0 with
        // no output; use `true` which is available in our CI image.
        let outcome = backend
            .execute(BackendInvocation::new("true", Vec::new(), "noop-success"))
            .expect("system backend should run 'true'");

        assert_eq!(outcome.status, BackendStatus::Success);
        assert_eq!(outcome.exit_code, Some(0));
    }

    #[test]
    fn backend_from_env_defaults_to_dry_run() {
        let key = BACKEND_MODE_ENV;
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::remove_var(key);
        }
        let backend = backend_from_env();
        assert_eq!(backend.name(), "dry-run");
        if let Some(prev) = previous {
            unsafe {
                std::env::set_var(key, prev);
            }
        }
    }

    #[test]
    fn backend_from_env_switches_to_system_when_requested() {
        let key = BACKEND_MODE_ENV;
        let previous = std::env::var(key).ok();
        unsafe {
            std::env::set_var(key, "system");
        }
        let backend = backend_from_env();
        assert_eq!(backend.name(), "system");
        unsafe {
            if let Some(prev) = previous {
                std::env::set_var(key, prev);
            } else {
                std::env::remove_var(key);
            }
        }
    }

    #[test]
    fn display_command_includes_program_and_args() {
        let invocation = BackendInvocation::new(
            "blender",
            vec![
                "--background".to_string(),
                "--python".to_string(),
                "render.py".to_string(),
            ],
            "render-frame",
        );
        assert_eq!(
            invocation.display_command(),
            "blender --background --python render.py"
        );
    }
}
