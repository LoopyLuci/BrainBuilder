# Live Verification Checklist

Most of BrainBuilder is exercised by the automated suite
(`cargo test -p brainbuilder-core --lib`, `tsc`, frontend checks). A few paths,
however, need **live network** and the external **`opencode` CLI**, so they
cannot be proven inside the sandbox/CI. This checklist covers exactly those
hops. Run it once on a machine with an OpenCode key and the CLI installed.

Everything here is designed to fail *safely* if a step is skipped: the app
falls back to local Ollama, the agent works only in a throwaway git worktree,
and generated code never runs unsandboxed.

---

## 0. Prerequisites

- [ ] Ollama running locally (fallback provider) — `ollama list` succeeds.
- [ ] An OpenCode Go API key.
- [ ] `opencode` CLI on `PATH` (only needed for the agent, section 3) —
      `opencode --version` succeeds.
- [ ] A clean git checkout of this repo (the agent needs a git repo to branch
      from). `git status` is clean.

---

## 1. OpenCode inference provider (Phase A1)

1. [ ] Launch the app. Open **Models / Providers**.
2. [ ] Enter the OpenCode API key and save it.
       - Confirm the key is stored in the OS keychain, **not** in any file:
         search the repo + config dir for the key's first 6 chars — expect zero
         hits. (macOS: Keychain Access → "brainbuilder"; Windows: Credential
         Manager → "brainbuilder"; Linux: `secret-tool lookup service brainbuilder`.)
3. [ ] `has_provider_credentials` shows OpenCode as configured; the key is
       **never** echoed back to the UI.
4. [ ] Select provider **OpenCode Go**, pick a model from the fetched list
       (`opencode-go/*`).
5. [ ] In the **Author** panel, describe a small graph and generate it.
       Expect a valid graph to appear on the canvas.
6. [ ] Switch the provider back to **Ollama**, regenerate — confirm both
       providers work through the same selector.

**Pass criteria:** a graph authored end-to-end via OpenCode; key absent from
disk; Ollama still works as the local fallback.

---

## 2. Component synthesis → train (Phase B)

1. [ ] Open **Synthesize a Component**. Describe a simple layer, e.g.
       *"a swish activation: x * sigmoid(x), same shape in and out"*.
2. [ ] Click **Synthesize**. Watch for the smoke-test result chip.
       - Green: the sandboxed smoke test ran the kernel on tiny tensors and the
         output shape matched the declared interface.
       - Red: it is **not** installable (button stays disabled) — this is the
         gauntlet working, not a bug.
3. [ ] On green, click **Add to canvas**. Confirm the new component both
       appears in the Components palette **and** is dropped onto the canvas as a
       node — no restart.
4. [ ] Wire it into a tiny graph and run a short training pass; confirm loss
       streams to the dashboard.

**Pass criteria:** describe → smoke-green → placed on canvas → trains, without
touching EDN or restarting.

---

## 3. Self-building agent (Phase C)

> The agent only ever edits an **isolated git worktree**, runs through the
> nervous-system sandbox, and gates merges on the test suite. Start in
> **propose-approve** mode.

1. [ ] Open the **Agent** panel. Select mode **Propose + approve**, an OpenCode
       provider/model, and give a small task (e.g. *"add a docstring to
       core/src/autotune/mod.rs"*).
2. [ ] Start. Confirm a new worktree + branch are created (`git worktree list`
       shows a `.worktrees/…` entry) — the live checkout is untouched
       (`git status` on the main tree stays clean).
3. [ ] Review the produced **diff** in the panel.
4. [ ] Confirm the **test gate** ran; a red suite blocks the merge and is shown.
5. [ ] Approve — confirm a `--no-ff` merge lands the change.
6. [ ] Repeat the task in **Auto-apply** mode; confirm it merges automatically
       on green and that **Revert** removes the worktree + deletes the branch.
7. [ ] (Optional, opt-in) Try **Full autonomy** on a throwaway task; note that
       this mode explicitly trades safety for power.
8. [ ] Sandbox scoping: confirm via the **Console / audit log** that the
       agent's subprocess was capability-gated (FS scoped to the worktree,
       network to the provider egress only).

**Pass criteria:** work happens in a worktree, is reviewable + revertible, and
red tests block auto/full merges.

---

## 4. Auto-tune (Phase D)

1. [ ] Build any small trainable graph and pick a dataset.
2. [ ] Click **Auto-tune** in the Data & Training panel.
3. [ ] Confirm trials stream to the metrics dashboard and the leaderboard shows
       them ranked best-first, with the winner marked.
4. [ ] Confirm the winning `lr`/`optimizer`/`batch_size` is applied to the
       training config automatically, and a normal training run with it works.

**Pass criteria:** trials run under time/memory ceilings, rank, and the best
config is applied with one click.

---

## 5. GPU device picker — AMD RX 7900 XTX (Phase: hardware)

Enumeration and binding are code-complete and unit-tested; confirming a
*specific* card needs that card.

1. [ ] Open **Models** → the **GPU** panel. Confirm the RX 7900 XTX appears in
       the list (typically under **Vulkan** and/or **DX12**).
2. [ ] Select it as the preferred device; confirm the choice persists across an
       app restart.
3. [ ] Click **Test this GPU** — confirm it reports `bound: …7900 XTX…` (the
       `probe_gpu_adapter` command actually acquires the adapter).
4. [ ] Build a small graph and run a training/exec pass on the wgpu backend;
       confirm the WGSL compute path runs (see `core/tests/wgpu_backend.rs` for
       the arithmetic-on-GPU proof against a real adapter).

**Known limit:** tensors are CPU-resident between calls; a persistent
GPU-buffer tensor model is a deliberate future redesign (documented in
[wgpu_backend.rs](../core/src/runtime/wgpu_backend.rs)).

## 6. Model inference & GGUF (hardware / external)

- **ONNX inference** is real via the `ort` crate but needs a model file present;
  load one in the Model Hub and run it on your machine.
- **GGUF** is intentionally delegated to a local `ollama` (llama.cpp) install
  rather than a from-scratch quantized engine — register a `.gguf` via the Model
  Hub, then chat/generate. See
  [gguf_router.rs](../core/src/models/gguf_router.rs) for the rationale.

## Honest limits

- "Zero failures / zero downtime" is approached via **containment +
  reviewability** (sandbox, worktree, test gate, per-widget error boundaries),
  not an absolute guarantee. Full-autonomy agent mode is opt-in and less safe.
- OpenCode Go is a paid hosted API; Ollama remains the always-local fallback so
  the app never *requires* a subscription.
- On macOS there is no hard RSS ceiling via rlimit; the memory cap is
  best-effort there (see the nervous-system notes).
