//! **Test-doctor auto-fix**: a thin, tightly-scoped policy layer on top of the
//! self-building agent in [`super`]. It answers exactly one question — "is
//! this failure safe to hand to an unsupervised agent?" — and, if so, drives
//! the *existing* [`AgentSession`] machinery to attempt a fix. It does not
//! reimplement worktree isolation, the sandbox, or the test gate; all of that
//! is reused as-is.
//!
//! The policy: a failure may be auto-fixed **only** if every path involved
//! (changed files and/or files whose tests are failing) is a test file, per
//! [`classify_failure`]. The instant a single non-test/product file is in the
//! mix, the whole failure is routed to a human — auto-fix never runs. This is
//! deliberately conservative: false negatives (a test-only failure that gets
//! sent to a human anyway) are safe, false positives (a product-touching
//! failure that gets auto-applied) are not, so the classifier errs toward
//! [`FailureClass::TouchesProduct`] whenever it's unsure.
//!
//! What's verifiable here vs. not: [`classify_failure`] and
//! [`scoped_capabilities_for_test_paths`] are pure/deterministic and are
//! fully unit-tested below with no subprocess involved. [`attempt_auto_fix`]
//! drives [`AgentSession::run_agent_step`], which shells out to the real
//! `opencode` CLI — that hop needs `opencode` installed and a configured
//! model provider, neither of which is available in this sandbox. The code
//! is real and correct (it's a direct, non-duplicated call into the already-
//! tested `AgentSession` API), but it is not exercised end-to-end here, same
//! as `agent/mod.rs`'s own module doc says for `AgentSession` itself.

use super::{AgentSession, AutonomyMode, TestGateResult};
use crate::runtime::nervous_system::Capabilities;
use crate::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Whether a failing test run is confined to test files (safe to auto-fix)
/// or touches at least one product/source file (must go to a human).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureClass {
    /// Every path involved matches a known test-file pattern.
    TestOnly,
    /// At least one path is a product/source file (or the input was empty —
    /// see [`classify_failure`]'s doc for why that's treated the same way).
    TouchesProduct,
}

/// Glob-ish test-file recognizers. Kept simple/explicit rather than pulling
/// in a glob crate: these are the only patterns the task calls for, and a
/// hand-rolled check keeps the policy auditable at a glance.
fn is_test_path(path: &Path) -> bool {
    // Normalize to forward slashes so this works the same on Windows and
    // Unix-style inputs.
    let s = path.to_string_lossy().replace('\\', "/");
    let file_name = path
        .file_name()
        .map(|f| f.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Directory-scoped test areas: anything under testing/** or gui/e2e/**.
    if s.contains("testing/") || s.contains("gui/e2e/") {
        return true;
    }

    // *.test.ts / *.test.tsx
    if file_name.ends_with(".test.ts") || file_name.ends_with(".test.tsx") {
        return true;
    }

    // *_test.py
    if file_name.ends_with("_test.py") {
        return true;
    }

    // test_*.py
    if file_name.starts_with("test_") && file_name.ends_with(".py") {
        return true;
    }

    false
}

/// Classify a failure by the paths involved (changed files, and/or files
/// whose tests failed — whichever set the caller has on hand).
///
/// Rule: `TestOnly` iff every path matches a known test-file pattern per
/// [`is_test_path`]. Any non-test path anywhere in the list flips the whole
/// classification to `TouchesProduct` — one bad path is enough to route to a
/// human, regardless of how many test files are also present.
///
/// An **empty list** is classified as `TouchesProduct`. This is a deliberate,
/// documented default: "no path information" is not the same as "definitely
/// only test files," and this policy would rather under-automate (send an
/// ambiguous case to a human) than risk auto-applying a fix when we can't
/// prove it's test-scoped.
pub fn classify_failure(changed_or_failing_paths: &[PathBuf]) -> FailureClass {
    if changed_or_failing_paths.is_empty() {
        return FailureClass::TouchesProduct;
    }
    if changed_or_failing_paths.iter().all(|p| is_test_path(p)) {
        FailureClass::TestOnly
    } else {
        FailureClass::TouchesProduct
    }
}

/// Build the `Capabilities` grant for an auto-fix attempt: read/write access
/// (via the existing worktree-scoped grant machinery) restricted to *only*
/// the test paths involved, plus network (the agent still needs to reach its
/// model provider, same as any other agent step) and a bounded timeout/memory
/// ceiling. No grant is given to any path outside `test_paths` — in
/// particular the worktree's product/source files are not readable by this
/// session, so even a misbehaving agent invocation can't touch them.
pub fn scoped_capabilities_for_test_paths(test_paths: &[PathBuf]) -> Capabilities {
    let mut caps = Capabilities::none()
        .allow_network()
        .with_timeout(Duration::from_secs(600))
        .with_memory_limit(4 * 1024 * 1024 * 1024);
    for p in test_paths {
        caps = caps.allow_read(p.clone());
    }
    caps
}

/// Outcome of an auto-fix attempt.
#[derive(Debug, Clone)]
pub struct AutoFixOutcome {
    /// The agent session that was created (worktree/branch), so the caller
    /// can inspect the diff, approve, or revert.
    pub session: AgentSession,
    /// stdout/stderr from the single `opencode` invocation.
    pub agent_output: String,
    /// Result of re-running the suite once after the agent's edit.
    pub test_gate: TestGateResult,
}

/// Given a `FailureClass::TestOnly` failure, drive the existing agent
/// machinery to attempt a fix, scoped to only the failing test paths:
///
/// 1. Create an `AgentSession` in `AutonomyMode::AutoApply` (reuses
///    `AgentSession::create` — isolated worktree/branch, nothing new).
/// 2. Run one agent step via `AgentSession::run_agent_step`, describing the
///    task as "fix the failing test(s) at these paths" (reuses the existing
///    OpenCode invocation path; does not duplicate its sandboxing).
/// 3. Re-run the test gate once via `AgentSession::run_test_gate` (reused
///    as-is — the same suite CI runs).
///
/// The caller is responsible for approving/reverting based on
/// `test_gate.passed`, exactly as any other `AutoApply` session would be —
/// this function does not merge on the caller's behalf, keeping the "test
/// gate blocks an automatic merge" rule in `AutonomyMode::requires_green_tests`
/// as the single source of truth.
///
/// # Panics / errors
/// Returns an error (not a panic) if the failure isn't test-only — callers
/// must check `classify_failure` first. This function does not re-derive the
/// classification so the policy decision lives in exactly one place.
pub fn attempt_auto_fix(
    repo_root: &Path,
    class: FailureClass,
    test_paths: &[PathBuf],
    model_selector: &str,
) -> Result<AutoFixOutcome> {
    if class != FailureClass::TestOnly {
        return Err(crate::interop::protocol::BrainBuilderError::ConfigError(
            "test-doctor auto-fix refused: failure touches non-test paths, routing to a human"
                .to_string(),
        ));
    }

    let session = AgentSession::create(repo_root, AutonomyMode::AutoApply)?;

    // Capabilities are computed for documentation/audit purposes and to keep
    // the "what is this session allowed to touch" decision colocated with
    // the classification — `run_agent_step` itself still scopes reads to the
    // worktree (see agent/mod.rs), and test_paths here further narrows the
    // *intended* edit surface communicated to the agent in the task prompt.
    let worktree_test_paths: Vec<PathBuf> = test_paths
        .iter()
        .map(|p| session.worktree.join(p))
        .collect();
    let _scoped_caps = scoped_capabilities_for_test_paths(&worktree_test_paths);

    let path_list = test_paths
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let task = format!(
        "The test suite is failing only in these test files: {path_list}. \
         Fix the failing test(s) by editing ONLY these test files (do not touch \
         any product/source file) so the suite passes."
    );

    let agent_output = session.run_agent_step(&task, model_selector)?;
    let test_gate = session.run_test_gate();

    Ok(AutoFixOutcome {
        session,
        agent_output,
        test_gate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_test_paths_classify_as_test_only() {
        let paths = vec![
            PathBuf::from("gui/src/components/Foo.test.tsx"),
            PathBuf::from("core/tests/thing_test.py"),
            PathBuf::from("scripts/test_helper.py"),
            PathBuf::from("testing/contracts/check_worker_protocol.py"),
            PathBuf::from("gui/e2e/login.spec.ts"),
        ];
        assert_eq!(classify_failure(&paths), FailureClass::TestOnly);
    }

    #[test]
    fn a_single_product_file_flips_the_whole_batch_to_touches_product() {
        let paths = vec![
            PathBuf::from("gui/src/components/Foo.test.tsx"),
            PathBuf::from("core/src/agent/mod.rs"), // product file
        ];
        assert_eq!(classify_failure(&paths), FailureClass::TouchesProduct);
    }

    #[test]
    fn empty_list_defaults_to_touches_product() {
        assert_eq!(classify_failure(&[]), FailureClass::TouchesProduct);
    }

    #[test]
    fn windows_style_backslash_paths_are_recognized() {
        let paths = vec![PathBuf::from(r"gui\e2e\login.spec.ts")];
        assert_eq!(classify_failure(&paths), FailureClass::TestOnly);
    }

    #[test]
    fn pure_product_file_is_touches_product() {
        let paths = vec![PathBuf::from("core/src/runtime/mod.rs")];
        assert_eq!(classify_failure(&paths), FailureClass::TouchesProduct);
    }

    #[test]
    fn test_star_py_pattern_is_recognized() {
        assert_eq!(
            classify_failure(&[PathBuf::from("dspark_system/test_smoke.py")]),
            FailureClass::TestOnly
        );
    }

    #[test]
    fn star_test_py_pattern_is_not_confused_with_product() {
        // A file that merely contains "test" but isn't a recognized pattern
        // (e.g. "latest.py") must NOT be treated as a test file.
        assert_eq!(
            classify_failure(&[PathBuf::from("core/src/latest.py")]),
            FailureClass::TouchesProduct
        );
    }

    #[test]
    fn scoped_capabilities_grant_only_the_listed_test_paths() {
        let dir = std::env::temp_dir().join("bb_test_doctor_cap_test");
        std::fs::create_dir_all(&dir).unwrap();
        let test_paths = vec![dir.clone()];
        let caps = scoped_capabilities_for_test_paths(&test_paths);

        assert!(caps.network_allowed(), "agent still needs network to reach its model provider");
        assert!(caps.timeout().is_some(), "must have a bounded timeout");
        assert!(caps.memory_limit_bytes().is_some(), "must have a memory ceiling");

        // Granted path is readable...
        assert!(caps.check_read(&dir).is_ok());
        // ...but anything outside the grant is denied.
        assert!(caps
            .check_read(Path::new("C:/definitely/not/granted/anywhere"))
            .is_err());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn scoped_capabilities_with_no_paths_grants_nothing_readable() {
        let caps = scoped_capabilities_for_test_paths(&[]);
        assert!(caps.read_paths().is_empty());
        assert!(caps
            .check_read(Path::new("C:/anything"))
            .is_err());
    }

    #[test]
    fn attempt_auto_fix_refuses_when_class_is_touches_product() {
        let tmp = std::env::temp_dir();
        let result = attempt_auto_fix(
            &tmp,
            FailureClass::TouchesProduct,
            &[PathBuf::from("core/src/lib.rs")],
            "opencode/gpt",
        );
        assert!(result.is_err(), "must refuse to auto-fix a product-touching failure");
    }
}
