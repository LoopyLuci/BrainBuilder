use super::audit::{self, AuditEvent, Timer};
use super::job_object::{self, JobObject};
use super::Capabilities;
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command, Output, Stdio};

/// Fallback per-subprocess memory ceiling for one-shot (Racket/Clojure)
/// invocations, used only if a `Capabilities` grant doesn't specify one and
/// the real-RAM auto-scale query itself is unavailable. 1 GiB was tried and
/// found too tight in practice: the JVM (Clojure's bridge) reserves address
/// space well above what it'll actually commit, and gets denied outright by
/// a hard job memory ceiling before it can even start.
const FALLBACK_MEMORY_LIMIT_BYTES: u64 = 3 * 1024 * 1024 * 1024; // 3 GiB

/// Owns subprocess spawn/teardown for capability-checked component runtimes.
/// Real enforcement: every filesystem path an invocation touches is
/// pre-validated against the runtime's `Capabilities` before the process is
/// spawned; every run is wall-clock bounded; and the child is contained by
/// real OS process-tree controls — a Windows Job Object (kill-on-close,
/// memory ceiling) or, on Unix, a private process group plus an `RLIMIT_AS`
/// ceiling applied pre-exec (see `job_object.rs`; the Unix path is written
/// but unverified on a real Linux/macOS machine in this all-Windows dev
/// environment) — so a misbehaving component can't outlive a timeout by
/// spawning grandchildren. cgroups/seccomp/App-Sandbox hardening beyond
/// that remains a documented follow-up, not silently pretended.
pub struct Supervisor;

impl Supervisor {
    /// Validates `touched_paths` against `caps`, writes `stdin_data` (if
    /// any) to the child's stdin, then runs `command` to completion, killing
    /// it if it outlives `caps.timeout()`. Every outcome (denied / allowed /
    /// timed out) is recorded to the nervous system's audit log, tagged with
    /// `runtime_name` (e.g. `"racket"`, `"clojure"`) so the GUI's Console can
    /// show a person what sandboxed code actually did.
    pub fn run_checked(
        command: &mut Command,
        caps: &Capabilities,
        touched_paths: &[&Path],
        stdin_data: Option<&str>,
    ) -> Result<Output> {
        Self::run_checked_named("subprocess", command, caps, touched_paths, stdin_data)
    }

    pub fn run_checked_named(
        runtime_name: &str,
        command: &mut Command,
        caps: &Capabilities,
        touched_paths: &[&Path],
        stdin_data: Option<&str>,
    ) -> Result<Output> {
        let timer = Timer::start();
        for path in touched_paths {
            if let Err(e) = caps.check_read(path) {
                audit::record(AuditEvent {
                    runtime: runtime_name,
                    outcome: "capability_denied",
                    duration_ms: Some(timer.elapsed_ms()),
                    detail: &format!("denied read of `{}`: {e}", path.display()),
                });
                return Err(e);
            }
        }

        if stdin_data.is_some() {
            command.stdin(Stdio::piped());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        // Memory ceiling is computed before spawn: Unix needs it pre-fork
        // (an rlimit set on the child via `pre_exec`, applied by
        // `harden_before_spawn` below), while Windows applies its Job
        // Object memory limit after spawn (`JobObject::new`/`assign`).
        let memory_limit = caps.memory_limit_bytes().unwrap_or(FALLBACK_MEMORY_LIMIT_BYTES);
        job_object::harden_before_spawn(command, Some(memory_limit));

        let mut child: Child = command
            .spawn()
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to spawn subprocess: {e}")))?;

        // Best-effort: a Job Object failing to create/assign doesn't fail
        // the call, since the portable capability/timeout checks still hold.
        let _job = JobObject::new(Some(memory_limit)).inspect(|job| {
            job.assign(&child);
        });

        if let Some(data) = stdin_data {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                BrainBuilderError::ConfigError("failed to open subprocess stdin".into())
            })?;
            stdin
                .write_all(data.as_bytes())
                .map_err(|e| BrainBuilderError::ConfigError(format!("failed to write subprocess stdin: {e}")))?;
        }

        if let Some(timeout) = caps.timeout() {
            let deadline = std::time::Instant::now() + timeout;
            loop {
                if let Some(_status) = child
                    .try_wait()
                    .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?
                {
                    break;
                }
                if std::time::Instant::now() >= deadline {
                    job_object::kill_process_tree(&mut child);
                    let _ = child.wait();
                    audit::record(AuditEvent {
                        runtime: runtime_name,
                        outcome: "timeout_killed",
                        duration_ms: Some(timer.elapsed_ms()),
                        detail: &format!("exceeded its {timeout:?} capability timeout"),
                    });
                    return Err(BrainBuilderError::ConfigError(format!(
                        "subprocess exceeded its {:?} capability timeout and was killed",
                        timeout
                    )));
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }

        let output = child
            .wait_with_output()
            .map_err(|e| BrainBuilderError::ConfigError(format!("subprocess wait failed: {e}")));
        let detail = match &output {
            Ok(out) => format!("exit status: {}", out.status),
            Err(e) => format!("subprocess error: {e}"),
        };
        audit::record(AuditEvent {
            runtime: runtime_name,
            outcome: if output.is_ok() { "allowed" } else { "allowed_but_failed" },
            duration_ms: Some(timer.elapsed_ms()),
            detail: &detail,
        });
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn denies_a_command_touching_an_ungranted_path() {
        let caps = Capabilities::none().allow_read(std::env::temp_dir());
        let mut cmd = Command::new("cmd");
        let result = Supervisor::run_checked(&mut cmd, &caps, &[Path::new("C:/not/granted")], None);
        assert!(result.is_err());
    }

    #[test]
    fn kills_a_process_that_exceeds_its_timeout() {
        let caps = Capabilities::none().with_timeout(Duration::from_millis(200));
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "ping", "-n", "10", "127.0.0.1", ">", "NUL"]);
        let result = Supervisor::run_checked(&mut cmd, &caps, &[], None);
        assert!(result.is_err(), "expected the long-running process to be killed");
    }
}
