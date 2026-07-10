"""Persistent worker process for core/src/interop/python.rs. Reads one JSON
request per line from stdin, writes one JSON response per line to stdout.
Tensor payloads are exchanged as raw float32 files (paths chosen by the Rust
side), memory-mapped on both ends (see `read_tensor`/`write_tensor`) rather
than inlined as JSON or read/written wholesale — real shared memory between
this process and the Rust side, scaling to real tensor sizes without an
extra copy through a `bytes` object. Runs under
core/src/runtime/nervous_system's Supervisor with a scoped filesystem
capability grant (scratch dir + components/python) and no network — this is
what replaced in-process pyo3 (which ran arbitrary component code with full
host-process privileges). Every tensor is moved onto `_detect_device()`'s
real hardware (CUDA/MPS if present, else CPU) the moment it's read, and
back to CPU only when written back out — this, not the standalone Rust
`wgpu`/`cuda` `Device` backends, is what actually accelerates training when
a real GPU is installed, since every component runs as ordinary PyTorch ops
here.
"""
import sys
import os
import json
import mmap
import struct
import time
import inspect
import torch


def _detect_device():
    """Real hardware acceleration for the actual training path (the
    standalone Rust `wgpu`/`cuda` `Device` backends in
    core/src/runtime/{wgpu_backend,cuda_backend}.rs exist but were never
    wired into it — every tensor PythonBridge exchanges is CPU-only
    regardless of what GPU is installed). Auto-detects the best real device
    PyTorch itself can see: CUDA, then Apple Silicon's MPS, else CPU.
    Override with BRAINBUILDER_DEVICE for testing/debugging. Unverified on
    real GPU hardware in this all-CPU development sandbox — the CPU path
    (the `torch.device("cpu")` branch, and the override) is what's actually
    exercised by this repo's test suite; treat the CUDA/MPS branches as real
    code following PyTorch's own documented device API, not yet proven on a
    real GPU by this project."""
    def _cuda_ok():
        return torch.cuda.is_available()

    def _mps_ok():
        return getattr(torch.backends, "mps", None) is not None and torch.backends.mps.is_available()

    override = os.environ.get("BRAINBUILDER_DEVICE")
    if override:
        # Validate the override so forcing an accelerator that isn't actually
        # present (e.g. BRAINBUILDER_DEVICE=cuda on a box torch can't see a GPU
        # on) degrades to auto-detect instead of crashing at first tensor use.
        dev = torch.device(override)
        if dev.type == "cuda" and not _cuda_ok():
            pass  # fall through to auto-detect
        elif dev.type == "mps" and not _mps_ok():
            pass
        else:
            return dev
    if _cuda_ok():
        return torch.device("cuda")
    if _mps_ok():
        return torch.device("mps")
    return torch.device("cpu")


_DEVICE = _detect_device()


def _apply_seed():
    """Real reproducibility: if BRAINBUILDER_SEED is set, seed PyTorch's RNG at
    worker startup so every `torch.randn` weight initialization (see
    `random_tensor`) is deterministic across runs. Without this, a graph's
    initial weights differ every run, so training *speed* — and, for a
    borderline architecture, whether a fixed step budget converges at all —
    varies run to run (the root cause of the historically-flaky
    branching_graph test). This is the mechanism behind BBIR's
    `ReproducibilityConfig.seed`; the Rust side sets the env var before
    spawning this worker."""
    raw = os.environ.get("BRAINBUILDER_SEED")
    if raw is None or raw == "":
        return
    try:
        torch.manual_seed(int(raw))
    except ValueError:
        # A non-integer seed is a caller error, not something to crash the
        # whole worker over — leave the RNG unseeded (matching the no-seed
        # default) rather than take down every subsequent request.
        pass


_apply_seed()


def call_with_hyperparams(fn, positional, hyperparams):
    """Calls `fn(*positional, **kwargs)`, where kwargs is `hyperparams`
    filtered down to only the names `fn` actually declares — so a node's
    hyperparameters (e.g. `layernorm`'s `eps`, attention's `num_heads`) reach
    the component that wants them, while components with no use for any
    hyperparameter (`linear`, `relu`) are unaffected and never see an
    unexpected-keyword-argument error."""
    hp = hyperparams or {}
    accepted = inspect.signature(fn).parameters
    kwargs = {k: v for k, v in hp.items() if k in accepted}
    return fn(*positional, **kwargs)


def read_tensor(desc):
    numel = 1
    for s in desc["shape"]:
        numel *= s
    if numel == 0:
        return torch.zeros(desc["shape"], dtype=torch.float32, device=_DEVICE)
    # Memory-maps the Rust side's scratch file and hands the mapped buffer
    # straight to `torch.frombuffer` — no `f.read()` copy into a Python
    # `bytes` object first. `.clone()` still makes one real copy into a
    # tensor the caller can safely mutate/hold after the mmap is closed.
    # `.to(_DEVICE)` is a real (PCIe) copy when a GPU is present, and a
    # genuine no-op (returns the same tensor) when `_DEVICE` is CPU — so the
    # zero-copy CPU path this mmap scheme exists for is unaffected when no
    # GPU is installed, which is this project's own real dev/CI environment.
    with open(desc["path"], "rb") as f:
        mm = mmap.mmap(f.fileno(), numel * 4, access=mmap.ACCESS_READ)
        try:
            return torch.frombuffer(mm, dtype=torch.float32, count=numel).clone().reshape(desc["shape"]).to(_DEVICE)
        finally:
            mm.close()


def write_tensor(t, path):
    # Deliberately not `.numpy()`: this torch build predates the installed
    # numpy's major version (ABI mismatch -> "Numpy is not available" at
    # runtime, seen throughout this environment). `struct.pack` over
    # `.tolist()` is pure Python/torch, no numpy involved. The packed bytes
    # are copied into a memory-mapped file (shared with the Rust side)
    # rather than written through buffered file I/O. `.cpu()` brings a
    # GPU-resident result back before serializing — a no-op when `_DEVICE`
    # is already CPU.
    t = t.detach().contiguous().cpu().to(torch.float32)
    shape = list(t.shape)
    values = t.flatten().tolist()
    packed = struct.pack(f"<{len(values)}f", *values)
    if not packed:
        open(path, "wb").close()
        return {"path": path, "shape": shape, "dtype": "float32"}
    with open(path, "wb") as f:
        f.truncate(len(packed))
    with open(path, "r+b") as f:
        mm = mmap.mmap(f.fileno(), len(packed))
        try:
            mm[:] = packed
            mm.flush()
        finally:
            mm.close()
    return {"path": path, "shape": shape, "dtype": "float32"}


LOSS_FNS = {"mse": torch.nn.functional.mse_loss, "cross_entropy": torch.nn.functional.cross_entropy}
OPTIMIZERS = {"sgd": torch.optim.SGD, "adam": torch.optim.Adam}


def compute_loss(loss_fn, loss_name, prediction, target):
    """BrainBuilder's tensor exchange is float32 end to end (see the module
    docstring), but `torch.nn.functional.cross_entropy` requires an integer
    (`long`) class-index target — passing it a float tensor raises at
    runtime. Real classification/language-modeling losses need this cast;
    `mse` (and anything else regression-flavored) leaves the target alone."""
    if loss_name == "cross_entropy":
        target = target.long().flatten()
    return loss_fn(prediction, target)


def handle_call_component(req):
    mod = __import__(req["module"])
    try:
        fn = getattr(mod, req["function"])
    except AttributeError:
        raise ValueError(f"call_component: module `{req['module']}` has no function `{req['function']}`")
    result = call_with_hyperparams(fn, [read_tensor(d) for d in req["inputs"]], req.get("hyperparams"))
    return {"ok": True, "output": write_tensor(result, req["output_path"])}


def handle_train_step(req):
    live = {}
    for name, desc in req["inputs"].items():
        t = read_tensor(desc)
        if name in req["trainable_ports"]:
            t = t.clone().detach().requires_grad_(True)
        live[name] = t

    for op in req["operations"]:
        mod = __import__(op["component"])
        fn = getattr(mod, op["entry"])
        result = call_with_hyperparams(fn, [live[p] for p in op["inputs"]], op.get("hyperparams"))
        for out_name in op["outputs"]:
            live[out_name] = result

    loss_fn = LOSS_FNS.get(req["loss_name"])
    if loss_fn is None:
        return {"ok": False, "error": f"unknown loss `{req['loss_name']}`"}
    optim_cls = OPTIMIZERS.get(req["optimizer_name"])
    if optim_cls is None:
        return {"ok": False, "error": f"unknown optimizer `{req['optimizer_name']}`"}

    loss = compute_loss(loss_fn, req["loss_name"], live[req["output_port"]], read_tensor(req["target"]))
    params = [live[p] for p in req["trainable_ports"]]
    # weight_decay defaults to 0.0 (off) via .get so a request from before
    # this key existed still behaves identically.
    optimizer = optim_cls(params, lr=req["lr"], weight_decay=req.get("weight_decay", 0.0))
    optimizer.zero_grad()
    loss.backward()
    # grad_clip defaults to 0.0 (off): gradients pass through unmodified,
    # same as before this key existed. A positive value caps the combined
    # L2 norm of every trainable parameter's gradient at that value, in
    # place, before the optimizer reads them.
    grad_clip = req.get("grad_clip", 0.0)
    if grad_clip > 0.0:
        torch.nn.utils.clip_grad_norm_(params, grad_clip)
    optimizer.step()

    updated = {name: write_tensor(live[name], req["output_paths"][name]) for name in req["trainable_ports"]}
    return {"ok": True, "loss": loss.item(), "updated_weights": updated}


def handle_compute_gradients(req):
    # Split half of train_step: runs forward + backward and returns each
    # trainable port's gradient, but does NOT step the optimizer. This is
    # what distributed data-parallel training needs — a host averages
    # gradients from multiple clients (each computed on a different data
    # shard) before anyone applies an update, so no single client's
    # optimizer.step() can run ahead of the others.
    live = {}
    for name, desc in req["inputs"].items():
        t = read_tensor(desc)
        if name in req["trainable_ports"]:
            t = t.clone().detach().requires_grad_(True)
        live[name] = t

    for op in req["operations"]:
        mod = __import__(op["component"])
        fn = getattr(mod, op["entry"])
        result = call_with_hyperparams(fn, [live[p] for p in op["inputs"]], op.get("hyperparams"))
        for out_name in op["outputs"]:
            live[out_name] = result

    loss_fn = LOSS_FNS.get(req["loss_name"])
    if loss_fn is None:
        return {"ok": False, "error": f"unknown loss `{req['loss_name']}`"}

    loss = compute_loss(loss_fn, req["loss_name"], live[req["output_port"]], read_tensor(req["target"]))
    loss.backward()

    gradients = {}
    for name in req["trainable_ports"]:
        grad = live[name].grad
        if grad is None:
            grad = torch.zeros_like(live[name])
        gradients[name] = write_tensor(grad, req["gradient_paths"][name])

    return {"ok": True, "loss": loss.item(), "gradients": gradients}


def handle_apply_averaged_gradients(req):
    # Second half: given current weights and an already-averaged gradient
    # per trainable port (averaged across all clients by the host), run
    # exactly one optimizer step and return the updated weights. Reuses the
    # same LOSS_FNS/OPTIMIZERS-backed optimizer construction as train_step so
    # optimizer state (e.g. Adam's moments) stays consistent with the
    # non-distributed path.
    optim_cls = OPTIMIZERS.get(req["optimizer_name"])
    if optim_cls is None:
        return {"ok": False, "error": f"unknown optimizer `{req['optimizer_name']}`"}

    names = list(req["weights"].keys())
    params = []
    for name in names:
        w = read_tensor(req["weights"][name]).clone().detach().requires_grad_(True)
        w.grad = read_tensor(req["gradients"][name])
        params.append(w)

    optimizer = optim_cls(params, lr=req["lr"], weight_decay=req.get("weight_decay", 0.0))
    grad_clip = req.get("grad_clip", 0.0)
    if grad_clip > 0.0:
        torch.nn.utils.clip_grad_norm_(params, grad_clip)
    optimizer.step()

    updated = {name: write_tensor(p, req["output_paths"][name]) for name, p in zip(names, params)}
    return {"ok": True, "updated_weights": updated}


def handle_random_tensor(req):
    return {"ok": True, "output": write_tensor(torch.randn(req["shape"]), req["output_path"])}


def handle_zero_tensor(req):
    return {"ok": True, "output": write_tensor(torch.zeros(req["shape"]), req["output_path"])}


def handle_save_state_dict(req):
    torch.save({name: read_tensor(d) for name, d in req["weights"].items()}, req["path"])
    return {"ok": True}


def handle_load_state_dict(req):
    # Weight names aren't known to the caller until after loading, so paths
    # are derived here (output_dir/<name>.bin) rather than pre-assigned.
    state = torch.load(req["path"], weights_only=True)
    return {
        "ok": True,
        "weights": {
            name: write_tensor(t, os.path.join(req["output_dir"], name + ".bin"))
            for name, t in state.items()
        },
    }


def handle_sleep(req):
    # Test-support only: lets core/tests exercise the Rust-side per-request
    # timeout + respawn path without needing a real hang bug.
    time.sleep(req["seconds"])
    return {"ok": True}


HANDLERS = {
    "call_component": handle_call_component,
    "train_step": handle_train_step,
    "compute_gradients": handle_compute_gradients,
    "apply_averaged_gradients": handle_apply_averaged_gradients,
    "random_tensor": handle_random_tensor,
    "zero_tensor": handle_zero_tensor,
    "save_state_dict": handle_save_state_dict,
    "load_state_dict": handle_load_state_dict,
    "sleep": handle_sleep,
}


def main():
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line)
            handler = HANDLERS.get(req.get("op"))
            resp = handler(req) if handler else {"ok": False, "error": f"unknown op `{req.get('op')}`"}
        except KeyError as e:
            # A required field is missing from the Rust side's request (protocol
            # mismatch, not a tensor-math failure) — `str(KeyError)` alone is just
            # the bare key name (e.g. `'module'`), which is unhelpful without the
            # op it was missing from.
            resp = {"ok": False, "error": f"request for op `{req.get('op')}` is missing required field {e}"}
        except Exception as e:  # noqa: BLE001 - report any failure back over the protocol
            resp = {"ok": False, "error": str(e)}
        sys.stdout.write(json.dumps(resp) + "\n")
        sys.stdout.flush()


if __name__ == "__main__":
    main()
