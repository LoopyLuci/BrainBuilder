# Runtime & Devices

`core/src/runtime/` is where compiled graphs actually execute, where CPU/GPU device
backends live, and where "the nervous system" — BrainBuilder's capability-based
sandbox for every subprocess it spawns — is implemented.

## Native device selection (`core/src/runtime/device_select.rs`)

`resolve_device(preferred)` is scoped narrowly: it only resolves the device for
BrainBuilder's *native* `rust`-language kernel ops (`Device::exec` inside
`ExecutionPlan::forward`). Python/torch components have an entirely separate device
story through `PythonBridge` — see [Interop](interop.md#python-worker-componentspython_bb_workerpy).
It never fails: with the `wgpu` feature enabled and a matching adapter (case-insensitive
substring match on adapter name), it returns a `WgpuDevice`; otherwise it falls back to
`CpuDevice`.

## Device propagation: GUI pick → training

One GPU pick in the [GPU Picker](gui-panels.md#models-directory) fans out to **two
independent device stories**, each with its own fallback:

1. `Orchestrator::set_preferred_gpu` stores the adapter name in
   `preferred_gpu: Mutex<Option<String>>` and feeds it to
   `device_select::resolve_device` for the native-`rust`-kernel path.
2. The same call maps any non-empty pick to the literal torch device string `"cuda"`
   (wgpu adapters are named — "Radeon RX 7900 XTX" — but torch selects by device
   *type*, and `"cuda"` is the namespace both ROCm/AMD and NVIDIA use) and forwards it
   to `PythonBridge::set_device`, which **respawns the worker** with
   `BRAINBUILDER_DEVICE` set. `_bb_worker.py`'s `_detect_device` validates the request
   and falls back to auto-detect if torch can't actually see that accelerator.

This split exists because native `rust` kernels and the Python/torch training path are
genuinely separate execution engines with separate device APIs — see
[Component System](component-system.md#implementations) for why almost every shipped
component runs through the Python path today.

## CPU backend (`core/src/runtime/cpu_backend.rs`)

`CpuDevice` is the trivial `Device` implementation: `alloc` calls
`dlpack_support::alloc_managed_tensor`, `exec` delegates to `ops::dispatch`, `sync` is
a no-op.

## Native kernel dispatch (`core/src/runtime/ops.rs`)

`dispatch(kernel, inputs)` is a small CPU kernel table — currently only `identity` and
`relu` — with anything else returning a real `UnsupportedLanguage` error rather than
panicking. The catalog is intentionally small: no shipped `.edn` component currently
declares `:language "rust"` (they're all Python — see the
[Component Library](component-library.md)), so there's no real consumer for more
kernels yet. The dispatch *mechanism* is complete even though the catalog is minimal.

## wgpu backend (`core/src/runtime/wgpu_backend.rs`)

Enabled via core features `wgpu,pollster` (**not** the `gpu` feature, which pulls in
`cudarc`/the CUDA toolkit).

- `list_adapters()` enumerates every wgpu adapter across `Backends::all()`
  (Vulkan/DX12/Metal/GL), returning an empty vec rather than erroring on a machine with
  no GPU — this backs the [GPU Picker](gui-panels.md#models-directory) UI.
- `WgpuDevice::with_preferred(Option<&str>)` does a case-insensitive substring match
  against enumerated adapters, falling back to `request_adapter` with
  `PowerPreference::HighPerformance` if no match or `None` is given.
- **Tensors stay CPU-resident even on this backend.** `exec()` uploads bytes to a
  `wgpu::Buffer`, runs a real WGSL compute shader (`RELU_SHADER`/`IDENTITY_SHADER` are
  the two wired kernels), and downloads the result back to a CPU-allocated DLPack
  tensor. Making tensors persistently GPU-resident would need a genuine redesign of the
  DLPack tensor model — persistently-mapped wgpu buffers conflict with wgpu's
  per-submission map/unmap model. This is a documented, deliberate scope limitation,
  not an oversight.

Commands: `list_gpu_adapters`, `probe_gpu_adapter`, `set_preferred_gpu` (all in
`gui/src-tauri/src/main.rs`).

## CUDA backend (`gpu` feature)

Gated on `feature = "gpu"`, and genuinely stubbed. `CudaDevice::new()` does construct a
real `CudaContext::new(0)` via `cudarc` 0.12.1, but `alloc()`/`exec()` are `todo!()`,
with detailed comments describing the intended implementation (`alloc_zeros`, wrapping
the `CudaSlice`'s device pointer in a DLPack tensor with `device_type=GPU`). This
genuinely cannot be finished or verified without real NVIDIA hardware and a CUDA
toolkit — `cudarc`'s build script refuses to configure without `nvcc`, so `cargo build
--features gpu` fails before rustc ever reaches this file on a machine without one.
`cudarc` is configured with `features = ["cuda-version-from-build-system"]` specifically
so it probes `nvcc`/`CUDA_ROOT` at build time instead of hardcoding a CUDA version.

## The `Device` trait (`core/src/runtime/device.rs`)

The common interface all backends implement: `name`, `alloc(shape, dtype) -> Tensor`,
`exec(kernel, inputs) -> Result<Tensor>`, `sync() -> Result<()>`.

## The nervous-system sandbox (`core/src/runtime/nervous_system/`)

"The Sandboxing Nervous System": a capability-based process fabric for Racket/Clojure
(via `Supervisor::spawn_checked`) and the persistent Python worker (via
`job_object::harden_before_spawn`). What it sandboxes: **deny-by-default filesystem
reads** (only explicitly `allow_read`ed paths), **no network** unless
`allow_network()`, a **hard wall-clock timeout**, and an **optional memory ceiling**.

`Capabilities` (`nervous_system/capability.rs`) is the portable model:
`read_paths`, `network`, `timeout`, `memory_limit_bytes` — all deny/off by default via
`Capabilities::none()`. `check_read` canonicalizes both the target and each granted
path so `..`/symlink tricks can't escape a grant.

### Per-platform enforcement (`nervous_system/job_object.rs`)

Enforcement genuinely differs by platform, and the differences are documented rather
than hidden:

- **Windows** — a real Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (kills the
  whole process tree, including grandchildren a plain `Child::kill()` would miss) plus
  an optional `JOB_OBJECT_LIMIT_JOB_MEMORY` ceiling, both applied *post-spawn*.
- **Linux/other Unix** — `harden_before_spawn` sets `command.process_group(0)` and, if
  a memory limit is given, installs a `pre_exec` hook calling `setrlimit(RLIMIT_AS,
  ...)` (async-signal-safe, no allocation) — must run *pre-spawn* since rlimits can't
  be applied after `exec`. `kill_process_tree` uses `killpg`. Verified on a real Linux
  CI runner.
- **macOS** — the same rlimit + process-group approach is **not** applied: forcing
  Rust off the `posix_spawn` fast path (which `pre_exec` requires) onto real
  `fork()+exec()` was found, on real hardware, to corrupt multi-threaded processes
  mid-fork. Instead, `harden_before_spawn` writes a real Seatbelt (`sandbox-exec`)
  profile to a temp file and rewraps the command as `sandbox-exec -f <profile> --
  <program> <args>`. The profile denies everything by default, imports Apple's `bsd.sb`
  baseline (needed for dyld/mach bootstrap — hand-enumerating those paths was tried and
  broke dyld outright on real hardware), then allows exactly the granted `read_paths`
  and network only if requested.
  **Documented gap: macOS has no memory-ceiling enforcement at all** — Seatbelt has no
  native primitive for it. Windows and Linux both enforce one; macOS doesn't.

`Supervisor::run_checked_named` (used by the Racket/Clojure bridges and by
[component synthesis](synthesis.md#gate-3--sandboxed-smoke-test)) is the one-shot
run-to-completion path; the persistent Python worker instead calls
`job_object::harden_before_spawn` directly since it isn't a one-shot subprocess.

Every sandboxed invocation is recorded to an audit log surfaced in the GUI's
[Console panel's Nervous System tab](gui-panels.md#console-panel).

## Cluster & distributed runtime

Peer discovery, gossip, and gradient-averaging transport (libp2p) is a large enough
subsystem to have its own doc — see
[Cluster & Distributed Training](cluster-and-distributed.md).
