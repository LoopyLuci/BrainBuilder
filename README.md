# BrainBuilder — built, running, and verified end-to-end

This tree was originally scaffolded from a design blueprint that had never been
compiled. It has since been built for real: the whole Rust workspace compiles,
the Tauri desktop app launches, and every polyglot interop path (Racket,
Clojure, Python/PyTorch DLPack) has been exercised against real, installed
toolchains — not just read for plausibility.

## Phase 0: a human can actually build, train, and predict from the GUI

Earlier passes made the *backend* real (autograd training that provably
converges, real EDN/DLPack, real polyglot bridges). But the GUI itself
couldn't produce a graph the backend would accept — `convertToBBIR` hardcoded
every node's ports to `input`/`output`, while real components use ports like
`x`/`weight`/`y`. This pass closed that loop, and along the way surfaced
several real bugs that only show up when you actually exercise the full
journey end to end (see `core/tests/gui_flow_end_to_end.rs`, which runs the
identical `Orchestrator` code path the GUI's Tauri commands call):

- **No shipped component had a real Python implementation.** `linear.edn`
  pointed at `linear.py` since the original blueprint, but that file never
  existed anywhere in the repo — training any real graph would have failed
  with `ModuleNotFoundError`. Added real, tested implementations
  (`components/python/linear.py`, `relu.py`, `scale.py`) and wired
  `PYTHONPATH` at startup so they're actually importable.
- **`edn_rs::json_to_edn` (the crate's own helper) is broken for numbers** —
  `{"lr": 0.01, "epochs": 30}` serialized to the malformed `{:epochs30,
  :lr0.01}` (key and value glued together). Replaced with a hand-written,
  tested `serde_json::Value -> Edn` converter
  (`interop/edn_value.rs::json_to_edn`).
- **Component descriptors had no way to distinguish data ports from learnable
  parameters** (e.g. `linear`'s `weight`) — added a real `PortRole` field,
  used throughout scheduling/training to auto-initialize and optimize
  parameter ports without the GUI needing to wire them by hand.
- **DataFusion's CSV reader infers decimal columns as `Float64`** — the Arrow
  → DLPack bridge only handled Float32/Int32/Int64/Boolean, so *every* real
  CSV with decimal values would fail. This isn't an edge case; it's the
  common case. Fixed.
- **Weight auto-init sized parameters to the training batch's length**, which
  silently broke inference the moment Predict ran on a different number of
  rows. Fixed to initialize parameters as broadcastable scalars.
- **`graph_id` was regenerated randomly on every export**, meaning a
  checkpoint saved after training could never be found again by Predict.
  Moved `graph_id` into the GUI's graph store as a stable, per-document id.
- **`BBIRNode` had no position field** — saving and reloading a graph would
  have silently scattered every node back to a default layout. Added
  `NodePosition`, round-tripped through EDN.

What's now real and working, verified via the app itself (not just unit
tests): descriptor-driven node ports and handles, a dataset picker with live
preview, editable loss/optimizer/lr/epochs, a real loss chart fed by the
existing metrics stream, graph save/load/autosave with undo/redo, checkpoint
save/load (real `torch.save`/`torch.load`) with a Predict panel, and a bundled
first-run example (`gui/examples/`) that trains a real, tiny linear-regression
problem to convergence with zero setup.

## Verified working

- **`cargo build --workspace`** (core + `gui/src-tauri`, `cpu`+`gui` features)
  compiles clean with zero errors.
- **`npx tauri build --debug`** produces a real `BrainBuilder.exe` that
  launches, opens its window, and streams the training-metrics event channel
  without panicking.
- **EDN parsing is real**, via the actual published `edn-rs`/`edn-derive`
  crates (not the fictional `serde_edn` from the original blueprint):
  - `ComponentRegistry::load_from_dir` parses every shipped `.edn` file in
    `components/` (verified in `core/tests/component_registry.rs`).
  - `BBIRGraph::from_edn`/`to_edn` round-trip (verified in
    `core/tests/bbir_roundtrip.rs`).
- **DLPack tensors are real**, via the actual published `dlpack` 0.2 crate
  (a bare `#[repr(C)]` binding to the DLPack C ABI, not the fictional
  `ManagedTensor::new(...)` API the blueprint assumed). Allocation, the
  CPU backend, and the deleter-based cleanup path are hand-rolled in
  `core/src/interop/dlpack_support.rs` and `core/src/lib.rs`'s `TensorHandle`.
- **The Python/PyTorch DLPack bridge is real and tested** against a live
  Python 3.11 + PyTorch 2.0 install (`core/tests/python_dlpack.rs`,
  `cargo test -- --ignored`): a tensor is allocated, written to, passed
  through `torch.utils.dlpack.from_dlpack`, round-tripped through an
  identity `torch` function, and the values are verified byte-for-byte.
  This required bypassing `pyo3::types::PyCapsule`'s safe wrapper (its
  destructor doesn't understand DLPack's "renamed capsule means the
  consumer took ownership" protocol) in favor of raw `pyo3::ffi` capsule
  calls — see `core/src/interop/python.rs` for the full explanation.
- **Racket and Clojure bridges are real subprocess implementations**
  (`core/src/interop/racket.rs`, `core/src/interop/clojure.rs`), tested
  against actually-installed Racket 9.2 and Leiningen/Clojure 1.12
  (`core/tests/interop_smoke.rs`, `cargo test -- --ignored`):
  - Racket: shells out to `racket languages/racket/bridge.rkt`, a two-line
    stdin/stdout protocol around `symbolic-diff`.
  - Clojure: shells out to `java -cp <classpath> clojure.main
    languages/clojure/bridge.clj`, resolving the classpath from
    `~/.m2/repository/org/clojure/*` (populated by Leiningen) since
    `brainbuilder_config.clj`'s filename doesn't follow Clojure's
    namespace→path convention and is loaded via `load-file` instead of
    `require`.

## What was fixed getting here (real bugs, not style)

- **`ExecutionPlan` import path**, **`AppContext.arena` type mismatch**,
  **`ComponentRegistry` behind `RwLock`**, **`load_dataset` needing
  `async`**, **`scheduler::ExecutionPlan::forward`'s duplicate-insert bug**,
  **`cpu_backend.rs`'s missing `ManagedTensor` import**, **GUI
  `NodeComponent`/`graphStore`/`package.json` issues** — all as originally
  documented and fixed.
- **`serde_edn`/`dlpack` were fictional APIs** — replaced with the real
  `edn-rs`+`edn-derive` and `dlpack` 0.2 crates; every struct that round-trips
  through EDN got real (derived or hand-written) `Deserialize`/`Serialize`
  impls matching the real crate APIs, not assumed ones.
- **`arrow`'s `compute`/`pyarrow` features don't exist at 51.0.0** as
  configured (`compute` isn't a feature at all; `pyarrow` drags in a `pyo3`
  version that conflicts with our own pin) — trimmed to `["prettyprint"]`.
- **`libp2p`'s `mplex` feature is gone in 0.54** (protocol deprecated) —
  dropped, `mdns` added explicitly since `distributed.rs` needs it.
- **`arrow-arith` 51.0.0 doesn't compile against chrono ≥0.4.39** (both define
  a `quarter` method, now ambiguous) — pinned `chrono = "=0.4.38"`.
- **`cuda_backend.rs` was compiled unconditionally** even though it only
  makes sense behind the `gpu` feature (it references the optional `cudarc`
  dependency) — now `#[cfg(feature = "gpu")]` in `runtime/mod.rs`.
- **Tauri 1.x doesn't have `tauri::Emitter`/`AppHandle::emit`** (that's a
  Tauri 2.x API) — uses `Manager::emit_all` instead.
- **`tokio::spawn` inside `.setup()` panicked at runtime** ("no reactor
  running") because Tauri's `.setup()` callback doesn't run inside an
  ambient Tokio context — switched to `tauri::async_runtime::spawn`, which
  lazily owns/drives its own runtime.
- **`brainbuilder-gui` was missing the `custom-protocol` feature** that
  `tauri build` requires for production bundling, and had no app icon
  (`tauri-build`'s Windows resource step needs `icons/icon.ico`) — both
  added.

## Dependency audit (every declared dependency is real *and* used)

Every crate/package version in `core/Cargo.toml`, `gui/src-tauri/Cargo.toml`
and `gui/package.json` is a real, published, resolvable dependency — verified
by `cargo build`/`npm install` actually fetching and compiling them, not by
reading version numbers. Beyond that, this pass also checked that nothing is
declared-but-dead:

- Removed **`ndarray`, `rand`, `jni`, `nix`, `url`, `wgpu`, and `tauri` (from
  `core`)** — all real crates, but grepped and confirmed zero references
  anywhere in `core/src`. `jni`/`tauri`-in-`core` were leftovers from before
  the Racket/Clojure bridges became subprocess-based and before it was clear
  `gui/src-tauri` (which has its own `tauri` dependency) needs nothing
  Tauri-specific from `core`. `wgpu` was declared under the `gpu` feature
  alongside `cudarc` but never actually referenced by any GPU backend code.
  Removing dead dependencies isn't just tidiness — an unused-but-declared
  dependency is exactly the kind of thing that *looks* wired up but isn't.
- **`cudarc` was misconfigured independent of hardware availability**: its
  build script hard-panics unless one of its CUDA-version features (or
  `cuda-version-from-build-system`) is selected — so `--features gpu` would
  have failed to even configure on a machine with a real GPU and CUDA
  toolkit, before ever touching hardware. Fixed by selecting
  `cuda-version-from-build-system`; confirmed the failure mode changed from
  a config error to the expected `nvcc: not found` (no CUDA toolkit here).
- **`log`/`env_logger` were declared but never called** — wired up for real:
  `env_logger::init()` in `gui/src-tauri/src/main.rs`'s `main()`, with
  `log::info!`/`debug!`/`error!` calls in `component/registry.rs` and
  `orchestrator.rs`. Verified by running `BrainBuilder.exe` with
  `RUST_LOG=info` and observing real log output (`loaded 5 component(s)
  from ...`).
- Removed **`serde` as a direct dependency of `gui/src-tauri`** — only
  `serde_json` is actually used there (`serde`'s derive macros are pulled in
  transitively through `brainbuilder-core`).
- npm side: every dependency in `gui/package.json` (`react`, `react-dnd`,
  `reactflow`, `zustand`, `uuid`, `@tauri-apps/api`, etc.) was grepped against
  actual imports in `gui/src` — all in active use, nothing dead.

## Real training, not just plumbing that compiles

The rest of the previously-stubbed execution path is now implemented and
tested end to end, not just typed to compile:

- **Arrow -> DLPack tensor conversion** (`runtime/arrow_bridge.rs`): each
  `RecordBatch` column becomes a real zero-copy CPU tensor (float32/int32/
  int64/bool, bool bit-unpacked since DLPack has no packed-bit dtype); columns
  with nulls are rejected with a clear error rather than silently producing
  garbage.
- **Real training via `PythonBridge::train_step`**: proven in
  `core/tests/train_step.rs` (`cargo test -- --ignored`) to actually learn —
  200 real SGD steps against real PyTorch autograd collapse the loss by
  >99% and converge a weight to within 0.05 of its true value. This *had* to
  be one `Python::with_gil` session running the whole forward->loss->
  backward->optimizer.step() chain as live `torch.Tensor` objects: DLPack's
  `__dlpack__`/`from_dlpack` carry no `grad_fn`/autograd history, so
  round-tripping each op's output back into Rust between calls (fine for
  pure inference, which `ExecutionPlan::forward` still does) would silently
  produce wrong (zero) gradients across every op boundary — worse than
  refusing to run. Learnable parameters (e.g. `linear`'s `weight` port) are
  detected as input ports no op produces, lazily initialized via real
  `torch.randn`, and threaded across the whole epoch×batch loop so they
  actually converge instead of resetting every step.
- **CPU kernel dispatch** (`runtime/ops.rs`): `Device::exec` now returns
  `Result<Tensor>` (was `todo!()`) with a real, unit-tested `identity`/`relu`
  kernel table. None of the shipped `components/*.edn` currently declare
  `:language "rust"` (they're all Python), so there's no real component
  routing to a specific kernel name yet — rather than invent one, unknown
  kernels get a real error instead of a panic.
- **Data preprocessing** (`data/etl.rs`): `normalize` (real z-score, verified
  to produce ~0 mean) and `cast` (via `arrow::compute::cast`) are real and
  wired into `data::source::load_dataset`. Ops needing a real dependency this
  crate doesn't have yet (image resize, tokenization) return a clear error
  rather than a fake no-op.
- **Provenance persistence** (`utils/provenance.rs`): a real append-only
  SQLite table (`rusqlite`, bundled — no system libsqlite3 needed), opened
  once in `AppContext` and written to after every real training step.
- **Nix environment capture** (`utils/nix.rs`): reads real version data —
  `Cargo.lock` (hand-parsed; it's a stable, simple format, not worth a TOML
  dependency) and a real `pip freeze` subprocess — into a syntactically
  valid Nix attribute set. Doesn't pretend to produce a buildable `mkShell`
  (mapping arbitrary pip/cargo packages to nixpkgs derivation names isn't
  generically correct); what's captured is the exact version data itself.
- **`CudaContext::new()` returns `Arc<Self>`, not `Self`** (verified against
  cudarc 0.12.1's real source) — the CUDA backend's field type was wrong and
  wouldn't have compiled even with a GPU present. Fixed. `alloc`/`exec`
  remain `todo!()`: cudarc's own build script refuses to configure without a
  real CUDA toolkit (`nvcc`), so nothing past that point can be compile-checked
  in this environment — writing further speculative kernel code against an
  unverifiable API would repeat the original blueprint's mistake in a
  different crate.

## Known remaining gaps

- **CUDA backend** — see above; needs real GPU hardware to finish and verify.
- **Racket/Clojure bridges are one-process-per-call.** Fine for the config/
  symbolic-diff use cases they currently serve; if call volume ever makes
  process-spawn overhead a problem, the next step is a persistent
  request/response loop instead of spawn-per-call.
- **`train_step` assumes the batch's last column is the training target.**
  `DataSourceConfig` has no formal "label column" field yet, so this is a
  documented convention, not something the schema makes explicit.
- **The Arrow bridge produces one 1-D (length = batch size) tensor per
  dataset column** — real and correct for elementwise components like
  `scale`, but `linear`/`conv2d` need a genuine `(batch, features)` 2-D
  matrix, which nothing currently assembles from multiple columns. The
  bundled first-run example deliberately uses `scale` to stay honest about
  this; multi-feature `linear`/`conv2d` graphs need that reshape step built
  first (a real Phase 2 design question, not a quick fix).
- **`conv2d.py`, `adam.py`, and the image-classifier pipeline's Python
  implementation still don't exist** — only `linear.py`, `relu.py`, and
  `scale.py` do. Their descriptors would still fail with
  `ModuleNotFoundError` if used in a real graph today.
- **Resource paths (`components/`, `gui/examples/`) resolve relative to the
  Tauri backend's working directory** (`main.rs`'s `current_dir().join(
  "../components")` convention), which is fragile across how the app gets
  launched (`tauri dev` vs. a packaged build vs. running the built `.exe`
  directly from an arbitrary directory — verified this actually breaks if
  launched from the wrong cwd). Fine for development; a packaged build should
  resolve these via Tauri's bundled-resource API instead.

## Build instructions (now actually verified in this environment)

```bash
cargo build --workspace --no-default-features --features brainbuilder-core/cpu,brainbuilder-core/gui
cd gui && npm install && npx tauri build --debug   # or `npx tauri dev`
```

Running the full interop test suite (needs Racket, a JDK + Leiningen-resolved
Clojure jars, and Python + PyTorch on PATH):

```bash
cargo test --workspace -- --include-ignored
```
