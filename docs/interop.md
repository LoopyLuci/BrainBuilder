# Interop

BrainBuilder is polyglot: real training math runs in Python/PyTorch, some symbolic
differentiation and config DSLs run in Racket and Clojure, and all of it is
orchestrated from Rust. This doc covers `core/src/interop/`, the persistent Python
worker protocol, and the two subprocess bridges.

## Architectural note: no embedded Python anymore

An earlier version of BrainBuilder embedded Python in-process via PyO3
(`auto-initialize`), giving arbitrary component code the full privileges of the host
process — "the last unsandboxed execution path" (`core/src/interop/python.rs:1-13`).
That's been replaced entirely by a **persistent subprocess worker**
(`components/python/_bb_worker.py`) talking JSON-over-stdio plus mmap'd tensor files.
There is no PyO3 code left in `core/src/interop/`.

## `PythonBridge` (`core/src/interop/python.rs`)

`PythonBridge` holds a `SharedArena`, a scratch directory, `Capabilities` (see
[the nervous-system sandbox](runtime-and-devices.md#the-nervous-system-sandbox)), the
current torch device preference (`Mutex<Option<String>>`), and a `Mutex<Worker>`.

- **`Worker::spawn`** launches `python components/python/_bb_worker.py`, setting
  `BRAINBUILDER_DEVICE` from the requested device when present. It calls
  `job_object::harden_before_spawn` **before** creating the child so Linux/macOS
  pre-exec hardening applies to the worker itself, wires stdin/stdout/stderr pipes, and
  spins a background thread forwarding each `read_line` over an `mpsc::channel` so
  `request()` can apply a real timeout (`BufRead::read_line` has none natively). A
  Windows `JobObject` is attached post-spawn.
- **`PythonBridge::new`** creates a scratch dir under
  `temp_dir()/brainbuilder_ipc_<pid>`, builds `Capabilities` scoped to that dir plus
  every `PYTHONPATH` entry, no network, a 60s timeout, then spawns the first worker
  with `device=None`.
- **`set_device`** normalizes the requested device string, no-ops if unchanged, else
  stores it and **respawns the worker** with the new device — this is how a GUI GPU
  pick actually reaches PyTorch (see
  [device propagation](runtime-and-devices.md#device-propagation-gui-pick--training)).
- **`request`** writes one JSON line to the worker's stdin and blocks on
  `worker.lines.recv_timeout(caps.timeout())`. Any failure mode — write fails, worker
  crashed, timeout, reader thread disconnected — **kills and respawns the worker** so
  the *next* call succeeds, while the in-flight call still surfaces an `Err`. Every
  outcome is recorded via `audit::record` with an outcome tag (`worker_crashed`,
  `timeout_killed`, `allowed_but_failed`, `allowed`) — this is what feeds the
  [Nervous System audit log](gui-panels.md#console-panel).

### Tensor transfer: mmap, not inline JSON

Tensors move through **memory-mapped scratch files**, not JSON payloads:

- `write_tensor` creates/sizes a file and `memmap2::MmapMut`s it, copying the tensor's
  `f32` bytes straight into mapped pages.
- `read_tensor` mmaps the worker's result file and copies into a freshly
  arena-allocated tensor — defensively: rejects negative dims, checked-multiplies for
  element/byte-count overflow, and checks the mapped file is at least the expected byte
  length before copying. These guard against a crashed or malicious worker and are
  covered by a `chaos_tests` module in `python.rs`.

### Public operations

`call_component`, `train_step` (forward→loss→backward→optimizer-step in one worker
call, keeping the autograd graph intact — see [why below](#why-training-is-one-worker-call)),
`compute_gradients` (forward+backward only, no optimizer step, for distributed
data-parallel training), `apply_averaged_gradients` (applies one optimizer step given
externally-averaged gradients — see
[Cluster & Distributed Training](cluster-and-distributed.md)), `random_tensor` /
`zero_tensor` (real `torch.randn`/`torch.zeros`, run worker-side), `save_state_dict` /
`load_state_dict` (real `torch.save`/`torch.load`).

### Why training is one worker call

DLPack's `__dlpack__`/`from_dlpack` carry no `grad_fn`/autograd history. Round-tripping
each op's output back into Rust between calls (fine for pure inference, which
`ExecutionPlan::forward` still does op-by-op) would silently produce wrong (zero)
gradients across every op boundary. So `train_step` runs the entire
forward→loss→backward→optimizer.step() chain as one live sequence of `torch.Tensor`
objects inside a single worker call.

## DLPack tensor support (`core/src/interop/dlpack_support.rs`)

A hand-rolled DLPack C-ABI allocator — the `dlpack` 0.2 crate is a bare `#[repr(C)]`
binding with no safe constructor.

- `dtype_to_dlpack` maps `component::descriptor::DataType` → DLPack's `(code, bits,
  lanes)` triple.
- `tensor_to_vec_f32` / `tensor_from_vec_f32` convert between DLPack tensors and plain
  `Vec<f32>` at non-DLPack-aware boundaries (GUI JSON, the cluster wire protocol).
- **`ManagerCtx`** holds the `Layout` and owning shape so the deleter frees exactly
  what was allocated; **`managed_tensor_deleter`** frees the data buffer, side
  allocations, and the `ManagedTensor` struct itself, per DLPack's contract.
- **`alloc_managed_tensor`** allocates a zeroed, 64-byte-aligned CPU buffer.

This allocator backs both `CpuDevice::alloc` and, notably, `WgpuDevice::alloc` — GPU
tensors are DLPack-wrapped over CPU memory, not GPU-resident. See
[Runtime & Devices](runtime-and-devices.md#wgpu-backend).

## Python worker protocol (`components/python/_bb_worker.py`)

Reads one JSON request per stdin line, writes one JSON response per stdout line,
dispatching via a `HANDLERS` dict keyed by `req["op"]`: `call_component`, `train_step`,
`compute_gradients`, `apply_averaged_gradients`, `random_tensor`, `zero_tensor`,
`save_state_dict`, `load_state_dict`, `sleep`. Unknown ops and missing-field
`KeyError`s convert to `{"ok": false, "error": ...}` rather than crashing the worker —
one bad request doesn't take the process down.

**Device detection** (`_detect_device`): auto-detects CUDA → MPS → CPU, overridable via
`BRAINBUILDER_DEVICE`, gracefully falling back to auto-detect if the override names an
accelerator torch can't actually see. Importantly: **this Python/torch device
selection, not the standalone Rust wgpu/CUDA backends, is what actually accelerates
real component training** — every tensor is moved to the detected device on read and
back to CPU on write.

**Tensor I/O**: `read_tensor` mmaps the Rust-written file and hands it to
`torch.frombuffer` (near-zero-copy, apart from `.clone()` to detach from the mmap and
`.to(device)`, a real PCIe copy only if GPU-resident). `write_tensor` deliberately
avoids `.numpy()` — the installed torch build predates the installed numpy's ABI,
which would raise "Numpy is not available" — using `struct.pack` instead.

`handle_train_step` marks only `trainable_ports` as `requires_grad_`, runs each
`operations` entry via dynamic `__import__`, computes loss (casting the target to
`.long()` only for `cross_entropy`), builds an optimizer (`sgd` supports `momentum`,
`adam` doesn't — passing one would raise `TypeError`), applies optional `grad_clip`,
steps, and returns updated weights. `compute_gradients`/`apply_averaged_gradients`
split that atomic sequence for distributed training — the latter deliberately does
*not* thread `momentum` through, since a fresh optimizer per round would discard any
velocity anyway.

### Protocol drift is caught mechanically

`testing/contracts/check_worker_protocol.py` regex-parses `"op": "..."` literals out
of `python.rs` and the `HANDLERS` dict keys out of `_bb_worker.py`, diffing the two op
sets and failing (exit 1) on any drift in either direction. This is the only thing
catching a renamed/removed op between the two languages, since nothing compiler-level
checks this wire contract. See [Scripts & Testing](scripts-and-testing.md#contracts).

## Racket bridge (`core/src/interop/racket.rs`)

Bridges to `languages/racket/symbolic_ad.rkt` by shelling `racket
languages/racket/bridge.rkt` through `Supervisor::run_checked_named`. `Default`
capabilities scope reads to `languages/racket/` only, no network, a 10s timeout, and an
auto-scaled memory limit (25% of RAM, 512MiB floor, 16GiB ceiling, overridable via
`BRAINBUILDER_SUBPROCESS_MEMORY_MB`).

**Protocol**: two stdin lines (an expression, then a variable name), one stdout line —
either the differentiated expression's printed form or `ERROR: <message>`.

`languages/racket/bridge.rkt` deliberately avoids the `json` racket package (this
Racket install, via scoop, can't fetch it) — the protocol is plain lines, not JSON.
`symbolic_ad.rkt`'s `symbolic-diff` is a small pattern-matched symbolic differentiator
over `+ - * / exp log sin cos` and constants/symbols; it needed full `#lang racket`
rather than `#lang racket/base`, which doesn't provide `match`.
`languages/racket/dsl.rkt` is a minimal, apparently unfinished `defcomponent` macro
that just prints a registration message — not a real component-registration mechanism.

## Clojure bridge (`core/src/interop/clojure.rs`)

Shells `java -cp <classpath> clojure.main languages/clojure/bridge.clj` rather than
embedding the JVM via JNI. `Default` capabilities grant read access to
`languages/clojure/` plus the resolved `~/.m2` classpath, a 15s timeout (JVM startup is
slower), and auto-scaled memory (35%, 2GiB floor, 16GiB ceiling).

**Classpath resolution** (`clojure_classpath`) walks
`~/.m2/repository/org/clojure/{clojure,spec.alpha,core.specs.alpha}` for the newest jar
under each, erroring with an actionable "run `lein repl` once..." message if a jar is
missing. Reads `USERPROFILE` then `HOME` for the `.m2` location.

**Protocol**: one stdin line (a Clojure form as text), one stdout line — `pr-str` of
the eval result, or `ERROR: <message>`.

`languages/clojure/bridge.clj` `load-file`s `brainbuilder_config.clj` directly (its
filename doesn't match Clojure's namespace→path convention, so `require` can't find
it). `brainbuilder_config.clj` defines one macro, `defgraph`, which captures a name and
freeform specs into a map — a thin DSL surface, not a full graph builder.

## EDN/JSON conversion (`core/src/interop/edn_value.rs`)

`json_to_edn` is explicitly **hand-written**, not delegated to `edn_rs::json_to_edn`,
because that function is broken for numerics in edn-rs 0.18 — `{"lr": 0.01, "epochs":
30}` serializes as `{:epochs30,:lr0.01}` with the key and value glued together.
`edn_to_json` does round-trip fine through `Edn::to_json`.

## Known gaps and platform notes

- Both `racket.rs`/`clojure.rs` are one-process-per-call. Fine for the current
  config/symbolic-diff volume; if call volume ever makes process-spawn overhead a
  problem, a persistent request/response loop (like the Python worker already has)
  would be the next step.
- All subprocess execution here shares the sandboxing model described in
  [Runtime & Devices](runtime-and-devices.md#the-nervous-system-sandbox), including its
  documented per-platform limits (no memory ceiling enforcement on macOS).
