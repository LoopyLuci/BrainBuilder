# Scripts & Testing

## `scripts/verify.ps1`

Run via `pwsh scripts/verify.ps1`. Six automated steps, each wrapped in a `Step`
helper that prints PASS/FAIL and increments a failure counter (exits 1 if anything
failed):

1. `cargo test -p brainbuilder-core --lib --quiet` — core lib tests.
2. `cargo test -p brainbuilder-core --lib --features wgpu,pollster adapter_tests
   --quiet` — wgpu adapter tests.
3. `npx tsc --noEmit` in `gui/` — frontend typecheck.
4. `npx vitest run` in `gui/` — frontend unit tests.
5. `python dspark_system/test_smoke.py` (with `PYTHONPATH` set to the repo root) —
   [DSpark](dspark-system.md) engine smoke test.
6. `python -m dspark_system.serve --new-tokens 4` — confirms the DSpark serve harness
   actually runs.

It then prints a "MANUAL (native app only)" checklist of 5 items requiring the real
`BrainBuilder.exe` (OpenCode inference, Agent propose-mode, Nervous System audit, a
real GPU test, Auto-tune) — see the
[Live Verification Checklist](VERIFICATION.md) for the full manual steps.

## `scripts/ship.ps1`

Runs everything `verify.ps1` runs, plus optional gate steps (skipped with a warning,
not a failure, if the target file doesn't exist yet, via an `OptionalStep` helper):

- `testing/contracts/check_worker_protocol.py` — see [Contracts](#contracts) below.
- `testing/bench/dspark_acceptance.py` — the
  [DSpark acceptance-rate regression bench](dspark-system.md#acceptance-benchmark-testingbenchdspark_acceptancepy).
- `gui/e2e` (`npm run e2e`, Playwright).

Only if every step passes does it proceed to `npm run tauri build` inside `gui/` to
produce a packaged binary under `gui/src-tauri/target/release/bundle/`. If the gate
fails, it exits 1 without building; if the gate passes but the build fails, it exits 1
with a distinct "gate passed but the packaged build failed" message — so a failure
message always tells you which side broke.

## `scripts/tauri-dev-debug.cmd`

Launches the real Tauri dev app with
`WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` set, then runs
`npm --prefix gui run tauri -- dev`. This opens a CDP port on the actual native
WebView2 window content (not just a plain browser tab), which is exactly the port
[universal-harness's](universal-harness.md) GUI introspection/execution connects to.
Debug-only — never used in production.

## `scripts/ci/pipeline.mjs`

BrainBuilder's self-described "local CI/CD pipeline — runs entirely on this machine,
no cloud runner, no GitHub dependency," explicitly meant to be the cross-platform,
zero-network, zero-third-party-account CI story. Stages are defined as data
(`name`/`cwd`/`cmd`/`cmdArgs`/`when`): `rust-build` (`cargo build --workspace
--locked`), `rust-test-fast` (`cargo test -p brainbuilder-core --lib --locked`),
`frontend-typecheck`, `frontend-build`, and `rust-test-full` (the full
~30-integration-test-binary suite, gated behind `--full` since it needs Python + torch
on PATH). CLI flags: `--full`, `--package`, `--install-hooks` (installs a git pre-push
hook that runs this pipeline), `--help`/`-h`. This is the same script
[universal-harness's BrainBuilder connector](universal-harness.md#the-brainbuilder-connector-universal-harnessconnectorsbrainbuilder)
wraps as its one `pipeline` operation.

### GitHub Actions status

`.github/workflows/` currently does not exist on disk — `ci.yml` and `release.yml`
were deleted from the working tree. As of this documentation pass that deletion is
**uncommitted** (no matching commit removing them was found in history), so treat this
as a pending, in-progress change rather than settled project history. `pipeline.mjs`'s
own comment treats it as the canonical CI story regardless, with any GitHub Actions
workflow as merely "an optional cloud mirror."

## Contracts

**`testing/contracts/check_worker_protocol.py`** is a stdlib-only, no-build-step drift
check between the Rust and Python halves of the
[worker protocol](interop.md#python-worker-protocol-componentspython_bb_workerpy). It
regex-parses `"op": "..."` literals out of `core/src/interop/python.rs` and the keys of
the `HANDLERS` dict out of `components/python/_bb_worker.py`, then diffs the two sets —
reporting "Rust-only ops" (a request the worker's dispatch loop would reject as
unknown) and "Python-only ops" (dead handler code or stale naming). Exit 0 with an "OK:
N ops in sync" message when they match; exit 1 with a detailed listing otherwise. This
is the *only* thing catching a renamed/removed op between the two languages — nothing
compiler-level checks this wire contract. Wired into `ship.ps1` as an optional gate.

## `core/tests/` — the Rust integration test suite

36 test files plus a `fixtures/` subdirectory, each its own linked binary (this is the
"~30 separate integration-test binaries" `pipeline.mjs`'s `rust-test-full` stage
comments about): `batch_predict`, `bbir_roundtrip`, `bbir_roundtrip_proptest`,
`branching_graph`, `chaos`, `checkpoint`, `cluster_device_profile`, `cluster_pairing`,
`component_registry`, `diagnostics_integration`, `distributed_gradient_network`,
`distributed_gradients`, `distributed_job_protocol`, `distributed_training_live`,
`dropout_eval_mode`, `early_stopping`, `feature_importance`, `gen_first_run_example`,
`gen_sentiment_example`, `gen_story_example`, `grad_clip`, `gui_flow_end_to_end`,
`image_folder_training`, `intent_end_to_end`, `interop_smoke`, `label_smoothing`,
`lr_decay`, `momentum`, `multi_feature_linear`, `python_dlpack`,
`python_worker_hardening`, `real_components`, `sentiment_classifier`,
`sequence_training`, `story_teller`, `text_column_ingestion`, `train_step`,
`transfer_weight_seeding`, `transformer_components`, `weight_decay`, `wgpu_backend`.

## `gui/` test setup

- **Unit tests** (`gui/vitest.config.ts`) use `environment: 'jsdom'` — needed to give
  pure Zustand stores a `localStorage`/`window` for persistence-path testing. Run via
  `npm run test` (= `vitest run`), covering `src/**/*.test.ts`.
- **End-to-end tests** (`gui/playwright.config.ts`) target `http://localhost:4173` (a
  `vite preview` static build, **not** a packaged Tauri binary). The config's own
  comment declares "HONEST SCOPE": there is no real Tauri runtime in this test — no
  `tauri-driver`/WebKitWebDriver/WinAppDriver — any `invoke()` call is stubbed via
  `window.__TAURI_IPC__` mocked in `gui/e2e/tauriMock.ts`. This proves the
  React/Zustand/reactflow frontend wiring works in a browser but explicitly does
  **not** prove real Tauri IPC or the Rust core — that's what
  [universal-harness's live GUI manifest](universal-harness.md#the-brainbuilder-connector-universal-harnessconnectorsbrainbuilder)
  is for. Spec files: `app.spec.ts`, `emptyState.spec.ts`, `templates.spec.ts`. Run via
  `npm run e2e`, invoked from `ship.ps1`'s optional gate.
