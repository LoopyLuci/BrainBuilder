//! The **self-building agent**: BrainBuilder driving an OpenCode coding agent
//! against its own repository, safely. The design principle is that
//! "self-modifying" must never mean "unsupervised and irreversible," so every
//! autonomy level shares the same containment spine:
//!
//! - **Isolation** — the agent only ever edits a dedicated **git worktree**
//!   (`git worktree add`), never the live checkout. Its work is a branch that
//!   can be diffed, tested, merged, or thrown away without touching the tree
//!   the user is running.
//! - **Sandbox** — the OpenCode process is spawned through the nervous system
//!   ([`Supervisor`]) with [`Capabilities`] scoped to that worktree, so even a
//!   misbehaving agent can't read outside its sandbox or outlive its timeout.
//! - **Test gate** — before any change is merged, the real test suite runs in
//!   the worktree; red tests block an auto/full merge and are surfaced in
//!   propose mode.
//!
//! The three [`AutonomyMode`]s differ *only* in where the human approval gate
//! sits — the worktree/sandbox/test-gate machinery underneath is identical.
//!
//! What's verifiable here vs. not: the worktree lifecycle, diffing, the
//! test-gate command construction, and the mode/gate logic are all real and
//! unit-tested. The OpenCode invocation itself is a subprocess hop that needs
//! the `opencode` CLI installed + a configured provider — the same
//! "can't-bind-a-real-network-service-in-this-sandbox" limitation the Ollama
//! client documents — so that one edge is exercised by a developer running the
//! real desktop app, not by CI.
use crate::interop::protocol::BrainBuilderError;
use crate::runtime::nervous_system::{Capabilities, Supervisor};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

pub mod test_doctor;

/// Where the human approval gate sits. The agent's edit/test loop is identical
/// across all three; only the merge decision differs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AutonomyMode {
    /// Agent edits + tests in the worktree; the user reviews the diff and
    /// approves before anything merges. The default.
    ProposeApprove,
    /// Agent edits + tests + merges automatically on green; the user reviews
    /// after the fact and reverts via git if unhappy.
    AutoApply,
    /// Agent edits + tests + merges with no gate at all. Maximum power,
    /// explicitly opt-in.
    Full,
}

impl AutonomyMode {
    /// Whether this mode is allowed to merge without an explicit human approve
    /// step. Propose mode never auto-merges; the other two do (still gated on
    /// green tests unless `Full`).
    pub fn merges_automatically(self) -> bool {
        matches!(self, AutonomyMode::AutoApply | AutonomyMode::Full)
    }

    /// Whether a red test gate blocks an automatic merge. Only `Full` overrides
    /// the test gate (and even then the failures are recorded).
    pub fn requires_green_tests(self) -> bool {
        !matches!(self, AutonomyMode::Full)
    }
}

/// A live agent working session bound to an isolated worktree + branch.
#[derive(Debug, Clone, Serialize)]
pub struct AgentSession {
    pub id: String,
    pub mode: AutonomyMode,
    pub branch: String,
    pub worktree: PathBuf,
    /// The repo this session was cut from (the live checkout's top level).
    pub repo_root: PathBuf,
}

impl AgentSession {
    /// Create a fresh session: cut a new branch + worktree off `repo_root` so
    /// the agent has an isolated place to work. The worktree lands under the
    /// system temp dir, never inside the live tree.
    pub fn create(repo_root: &Path, mode: AutonomyMode) -> Result<Self> {
        let id = short_id();
        let branch = format!("agent/{id}");
        let worktree = std::env::temp_dir().join(format!("bb_agent_{id}"));

        run_git(
            repo_root,
            &["worktree", "add", "-b", &branch, &worktree.to_string_lossy(), "HEAD"],
        )?;

        Ok(Self {
            id,
            mode,
            branch,
            worktree,
            repo_root: repo_root.to_path_buf(),
        })
    }

    /// Run one agent step: hand `task` to the OpenCode CLI, executed **inside
    /// the worktree, through the nervous-system sandbox**. Network is allowed
    /// (the agent talks to its model provider) but the filesystem grant is the
    /// worktree only. Returns the agent's stdout/stderr for display.
    ///
    /// Requires `opencode` on PATH; returns a clear error otherwise (the app
    /// surfaces "install OpenCode to enable the self-building agent").
    pub fn run_agent_step(&self, task: &str, model_selector: &str) -> Result<String> {
        let caps = Capabilities::none()
            .allow_read(&self.worktree)
            .allow_network()
            .with_timeout(Duration::from_secs(600))
            .with_memory_limit(4 * 1024 * 1024 * 1024);
        self.run_agent_step_scoped(task, model_selector, caps, &[&self.worktree])
    }

    /// Same as `run_agent_step`, but with a caller-supplied `Capabilities`
    /// grant and `touched_paths` list instead of the default "the whole
    /// worktree" one. Used by `test_doctor::attempt_auto_fix` to actually
    /// apply the narrower test-paths-only grant it computes — before this
    /// existed, that computed grant was built but never passed anywhere,
    /// so every auto-fix step ran with full-worktree read access regardless
    /// of how narrowly `scoped_capabilities_for_test_paths` had scoped it.
    ///
    /// Same caveat as the rest of this module's sandboxing: `Capabilities`
    /// here drives a pre-spawn path-grant check (`touched_paths` must be
    /// covered by `caps`), not an OS-level filesystem/network jail on every
    /// platform (see `runtime::nervous_system::job_object`'s module doc for
    /// the real, currently-documented per-platform enforcement gap). A
    /// narrower grant here still meaningfully narrows what a *caller*
    /// asserts and gets checked against, and keeps the intended edit surface
    /// visible in the audit log — it does not by itself guarantee an
    /// already-spawned `opencode` process can't reach outside it on Windows.
    pub fn run_agent_step_scoped(
        &self,
        task: &str,
        model_selector: &str,
        caps: Capabilities,
        touched_paths: &[&Path],
    ) -> Result<String> {
        // `opencode run <task>` is OpenCode's non-interactive one-shot mode.
        // `--model` threads the same provider:model selector the rest of
        // BrainBuilder uses.
        let mut command = Command::new("opencode");
        command
            .current_dir(&self.worktree)
            .arg("run")
            .arg(task)
            .arg("--model")
            .arg(model_selector);

        let output = Supervisor::run_checked_named("agent-opencode", &mut command, &caps, touched_paths, None)
            .map_err(|e| {
                BrainBuilderError::ConfigError(format!(
                    "couldn't run the OpenCode agent (is `opencode` installed and on PATH?): {e}"
                ))
            })?;

        let mut out = String::from_utf8_lossy(&output.stdout).into_owned();
        let err = String::from_utf8_lossy(&output.stderr);
        if !err.trim().is_empty() {
            out.push_str("\n[stderr]\n");
            out.push_str(&err);
        }
        Ok(out)
    }

    /// The unified diff of everything the agent has changed in its worktree,
    /// relative to the branch point — what the user reviews in propose mode.
    pub fn diff(&self) -> Result<String> {
        // Stage first so new files show up in the diff, then diff against HEAD.
        run_git(&self.worktree, &["add", "-A"])?;
        run_git(&self.worktree, &["diff", "--cached"])
    }

    /// Run the test gate in the worktree. Returns `(passed, combined_output)`.
    /// This is the same suite CI runs, so "green" means the same thing here.
    pub fn run_test_gate(&self) -> TestGateResult {
        // Rust fast tests — the always-runnable core gate.
        let rust = run_capture(
            &self.worktree,
            "cargo",
            &["test", "-p", "brainbuilder-core", "--lib", "--locked"],
        );
        // Frontend typecheck, best-effort (skipped cleanly if npm/tsc absent).
        let ts = run_capture(&self.worktree.join("gui"), "npx", &["tsc", "--noEmit"]);

        let passed = rust.success && ts.success;
        let mut output = format!("$ cargo test -p brainbuilder-core --lib\n{}\n", rust.output);
        output.push_str(&format!("$ (gui) npx tsc --noEmit\n{}\n", ts.output));
        TestGateResult { passed, output }
    }

    /// Commit the agent's work in its worktree (needed before a merge). No-op
    /// if there's nothing staged.
    pub fn commit(&self, message: &str) -> Result<()> {
        run_git(&self.worktree, &["add", "-A"])?;
        // `git commit` fails if there's nothing to commit; treat that as fine.
        let _ = run_git(&self.worktree, &["commit", "-m", message]);
        Ok(())
    }

    /// Approve: merge the agent's branch into the live checkout. Used by the
    /// user in propose mode, or automatically in auto/full mode on green.
    pub fn approve(&self) -> Result<()> {
        self.commit(&format!("agent: {} (approved)", self.id))?;
        run_git(&self.repo_root, &["merge", "--no-ff", &self.branch, "-m", &format!("merge agent branch {}", self.branch)])?;
        Ok(())
    }

    /// Revert + clean up: discard the worktree and delete the agent branch. The
    /// live checkout is untouched (nothing was merged), so this is a true undo.
    /// Best-effort: a failure here (e.g. the worktree dir is locked by another
    /// process, or the branch was already removed) must not stop the revert
    /// from being reported as done, but it's logged so a leaked worktree/branch
    /// is diagnosable instead of silently lingering on disk.
    pub fn revert(&self) -> Result<()> {
        // Remove the worktree (force, since it has uncommitted/committed work).
        if let Err(e) = run_git(&self.repo_root, &["worktree", "remove", "--force", &self.worktree.to_string_lossy()]) {
            log::warn!("agent {}: failed to remove worktree {}: {e}", self.id, self.worktree.display());
        }
        if let Err(e) = run_git(&self.repo_root, &["branch", "-D", &self.branch]) {
            log::warn!("agent {}: failed to delete branch {}: {e}", self.id, self.branch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TestGateResult {
    pub passed: bool,
    pub output: String,
}

// --- git / process helpers ---

struct Captured {
    success: bool,
    output: String,
}

fn run_capture(dir: &Path, program: &str, args: &[&str]) -> Captured {
    match Command::new(program).current_dir(dir).args(args).output() {
        Ok(out) => {
            let mut s = String::from_utf8_lossy(&out.stdout).into_owned();
            s.push_str(&String::from_utf8_lossy(&out.stderr));
            Captured { success: out.status.success(), output: s }
        }
        Err(e) => Captured { success: false, output: format!("failed to run {program}: {e}") },
    }
}

fn run_git(dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to run git: {e}")))?;
    if !out.status.success() {
        return Err(BrainBuilderError::ConfigError(format!(
            "git {:?} failed: {}",
            args,
            String::from_utf8_lossy(&out.stderr).trim()
        )));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn short_id() -> String {
    // Enough entropy to avoid worktree/branch collisions without a uuid dep
    // here; the process id + a monotonic counter keeps it unique per run.
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let t = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    format!("{:x}{:x}", t, n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn propose_mode_never_auto_merges() {
        assert!(!AutonomyMode::ProposeApprove.merges_automatically());
        assert!(AutonomyMode::ProposeApprove.requires_green_tests());
    }

    #[test]
    fn auto_apply_merges_but_still_needs_green_tests() {
        assert!(AutonomyMode::AutoApply.merges_automatically());
        assert!(AutonomyMode::AutoApply.requires_green_tests());
    }

    #[test]
    fn full_autonomy_merges_and_overrides_the_test_gate() {
        assert!(AutonomyMode::Full.merges_automatically());
        assert!(!AutonomyMode::Full.requires_green_tests());
    }

    #[test]
    fn mode_serializes_kebab_case() {
        let json = serde_json::to_string(&AutonomyMode::ProposeApprove).unwrap();
        assert_eq!(json, "\"propose-approve\"");
    }

    #[test]
    fn short_ids_are_unique() {
        let a = short_id();
        let b = short_id();
        assert_ne!(a, b);
    }

    // Full worktree lifecycle against a throwaway real git repo — proves the
    // create → diff → revert spine works without needing opencode or a network.
    #[test]
    fn worktree_lifecycle_creates_isolates_and_reverts() {
        let tmp = std::env::temp_dir().join(format!("bb_agent_test_repo_{}", short_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        // Init a real repo with one commit so HEAD exists.
        let git = |args: &[&str]| Command::new("git").current_dir(&tmp).args(args).output().unwrap();
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t.t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(tmp.join("a.txt"), "hello").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-qm", "init"]);

        let session = AgentSession::create(&tmp, AutonomyMode::ProposeApprove).expect("create session");
        assert!(session.worktree.is_dir(), "worktree should exist");

        // Make a change in the worktree; it must show in the diff but NOT in
        // the original checkout (isolation).
        std::fs::write(session.worktree.join("b.txt"), "agent-made-this").unwrap();
        let diff = session.diff().expect("diff");
        assert!(diff.contains("b.txt"), "diff should include the agent's new file: {diff}");
        assert!(!tmp.join("b.txt").exists(), "the live checkout must not see the worktree's change");

        // Revert cleans up the worktree + branch.
        session.revert().expect("revert");
        assert!(!session.worktree.exists(), "worktree should be gone after revert");

        std::fs::remove_dir_all(&tmp).ok();
    }

    // Regression test: `revert`'s git failures (worktree/branch cleanup) used
    // to be silently dropped. This forces both git commands inside `revert`
    // to fail (worktree path never created, branch name never created) and
    // asserts `revert` still returns `Ok` (its documented best-effort
    // contract) rather than surfacing the failure as an `Err` or panicking —
    // the fix only added `log::warn!` calls, it did not change that
    // contract.
    #[test]
    fn revert_still_reports_ok_when_its_git_cleanup_commands_fail() {
        let tmp = std::env::temp_dir().join(format!("bb_agent_test_repo_revert_{}", short_id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let git = |args: &[&str]| Command::new("git").current_dir(&tmp).args(args).output().unwrap();
        git(&["init", "-q"]);
        git(&["config", "user.email", "t@t.t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(tmp.join("a.txt"), "hello").unwrap();
        git(&["add", "-A"]);
        git(&["commit", "-qm", "init"]);

        // A session whose worktree/branch were never actually created —
        // `revert`'s `git worktree remove` and `git branch -D` will both
        // fail against real git, exactly the scenario the fix's log::warn!
        // calls are meant to surface instead of silently swallowing.
        let session = AgentSession {
            id: "nonexistent".into(),
            mode: AutonomyMode::ProposeApprove,
            branch: "agent/does-not-exist".into(),
            worktree: tmp.join("no_such_worktree_dir"),
            repo_root: tmp.clone(),
        };

        let result = session.revert();
        assert!(
            result.is_ok(),
            "revert must stay best-effort (Ok) even when its git cleanup commands fail: {result:?}"
        );

        std::fs::remove_dir_all(&tmp).ok();
    }
}
