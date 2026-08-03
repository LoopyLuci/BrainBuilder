# Self-Building Agent

`core/src/agent/mod.rs` drives an external OpenCode coding agent against
BrainBuilder's **own repository** — self-modification, made safe by construction
rather than by trust. Three containment layers apply to every autonomy level: git
worktree isolation, sandboxed subprocess execution, and a test gate before merge.

## `AgentSession`

Fields: `id, mode, branch, worktree, repo_root`.

- **`create(repo_root, mode)`** generates a short id (PID-time plus an atomic counter),
  names the branch `agent/{id}` and the worktree `$TMP/bb_agent_{id}`, and runs `git
  worktree add -b {branch} {worktree} HEAD`. The worktree always lands under the
  system temp directory — never inside the live checkout.
- **`run_agent_step(task, model_selector)`** builds
  `Capabilities::none().allow_read(&worktree).allow_network().with_timeout(600s).with_memory_limit(4 GiB)`
  and delegates to `run_agent_step_scoped`, which spawns `opencode run <task> --model
  <selector>` with `current_dir(&self.worktree)` through
  `Supervisor::run_checked_named("agent-opencode", …)`, mapping a spawn failure to "is
  `opencode` installed and on PATH?" — the one documented external dependency this
  system needs. This scoped variant exists specifically so a narrower,
  computed capability grant (e.g. test-paths-only during auto-fix) is actually applied
  rather than silently discarded in favor of full-worktree access — a real prior bug
  the code comment documents as fixed.
- **`diff()`** runs `git add -A` then `git diff --cached` in the worktree, so newly
  created files show up in the diff, not just edits to existing ones.
- **`run_test_gate()`** runs `cargo test -p brainbuilder-core --lib --locked` and
  (best-effort) `npx tsc --noEmit` under `gui/`; passes only if both succeed.
- **`commit(message)`** stages and commits (a no-op is tolerated if nothing is
  staged).
- **`approve()`** commits, then `git merge --no-ff {branch}` into `repo_root` — the
  actual merge into the live checkout.
- **`revert()`** removes the worktree (`git worktree remove --force`) and deletes the
  branch (`git branch -D`) — best-effort, failures are logged as warnings rather than
  surfaced as errors, so revert is a true, always-succeeding undo path.

## Autonomy modes

Three `AutonomyMode` variants, serialized kebab-case:

| Mode | Auto-merges? | Requires green tests? |
|---|---|---|
| **`ProposeApprove`** (default) | No — user reviews the diff and clicks approve | Yes |
| **`AutoApply`** | Yes, but only on a green test gate | Yes |
| **`Full`** | Yes, unconditionally | No — the only mode where the test gate is overridden (failures are still recorded, never hidden) |

## Sandbox caveat, documented honestly

`run_agent_step_scoped`'s doc comment is explicit that `Capabilities` here is a
pre-spawn path-grant *check*, not an OS-level filesystem/network jail on every
platform — see [the nervous-system sandbox's per-platform notes](runtime-and-devices.md#per-platform-enforcement-nervous_systemjob_objectrs)
for what's actually enforced where. A narrower grant meaningfully narrows what's
*asserted*, but doesn't by itself guarantee an already-spawned `opencode` process can't
reach outside it on every platform (macOS in particular has no memory ceiling and, per
the current implementation, network is granted wholesale rather than scoped per-host).

## Tauri commands (`gui/src-tauri/src/main.rs`)

- **`agent_start(mode, state)`** parses the mode string, resolves the repo root via
  `git rev-parse --show-toplevel` (erroring "not inside a git repository" otherwise),
  and calls `AgentSession::create`.
- **`agent_status(state)`** returns the active session JSON, or `None`.
- **`agent_run(task, selector, window, state)`** runs off the async runtime via
  `spawn_blocking`; emits live `agent-progress` window events per phase (`agent`,
  `agent-output`, `diff`, `tests`, `gate`, `done`) so the UI shows the diff before the
  slower test gate finishes. Applies the merge policy: auto-merges only if
  `mode.merges_automatically() && (gate.passed || !mode.requires_green_tests())`.
- **`agent_approve(state)`** calls `session.approve()` off-thread — the human gate for
  propose-approve mode.
- **`agent_revert(state)`** clears the session and calls `session.revert()`.

## GUI

The [Agent panel](gui-panels.md#llm--synthesis--agent-gui-side-only) lets a user pick
an autonomy mode, start a session, submit a task, and watch the streamed
`agent-progress` events update a live status line, diff view, and test-gate chip. It
also shows a filtered [Nervous System audit trace](gui-panels.md#console-panel)
(filtered to `runtime.includes('agent')`) as visible proof of capability-gated
sandboxing.

## Verifying this end to end

Because this needs a real network hop and the external `opencode` CLI, it can't be
proven in an automated sandbox — see
[section 3 of the Live Verification Checklist](VERIFICATION.md#3-self-building-agent-phase-c)
for the manual pass/fail steps (worktree isolation, diff review, test-gate blocking,
auto-apply, and revert).
