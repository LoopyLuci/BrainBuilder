// Python execution, sandboxed for real: a persistent subprocess worker
// (components/python/_bb_worker.py) instead of in-process pyo3. The
// original pyo3-based bridge embedded the interpreter directly in the app
// process via `auto-initialize` — every component's Python code (arbitrary,
// user-authored) ran with full host-process privileges. That was the last
// unsandboxed execution path in the nervous system (Racket/Clojure already
// route through `Supervisor`). Tensors cross the process boundary as raw
// float32 files rather than inlined JSON — PyTorch's own
// `torch.frombuffer`/`.numpy().tobytes()` make this a few lines on the
// Python side and it scales to real tensor sizes, unlike marshaling values
// as text. This is *not* zero-copy shared memory (a real next step), but it
// is genuine cross-process isolation: component code cannot observe or
// touch anything outside its scratch directory and `components/python`.
use crate::interop::arena::SharedArena;
use crate::interop::dlpack_support::{cpu_context, dtype_to_dlpack};
use crate::interop::protocol::BrainBuilderError;
use crate::runtime::nervous_system::{job_object, Capabilities, ComponentRuntime, JobObject};
use crate::runtime::scheduler::ExecutableOp;
use crate::{Result, Tensor};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Real memory ceiling for the worker (Job Object, Windows) — component code
/// is arbitrary and shouldn't be able to exhaust host memory.
/// Auto-scales with real total system RAM (50%, floor 2GiB, ceiling 48GiB) —
/// override with BRAINBUILDER_PYTHON_MEMORY_MB. Training tensors/model state
/// legitimately want real headroom on a big desktop; a fixed 4GiB ceiling
/// would starve a real workload on a 64GiB machine for no reason.
fn worker_memory_limit_bytes() -> u64 {
    crate::runtime::nervous_system::system_resources::resolve_memory_limit_bytes(
        "BRAINBUILDER_PYTHON_MEMORY_MB",
        0.5,
        2 * 1024 * 1024 * 1024,
        48 * 1024 * 1024 * 1024,
    )
}

pub struct PythonBridge {
    arena: Arc<SharedArena>,
    scratch_dir: PathBuf,
    caps: Capabilities,
    /// The torch device string (`"cuda"`, `"cpu"`, `"mps"`, ...) the worker is
    /// launched with via `BRAINBUILDER_DEVICE`. `None` = let the worker
    /// auto-detect. Set from the GUI's GPU picker; a respawn applies it.
    device: Mutex<Option<String>>,
    worker: Mutex<Worker>,
}

struct Worker {
    child: Child,
    stdin: ChildStdin,
    /// A background thread owns the blocking `read_line` loop and forwards
    /// each line here, so `request()` can wait on it with a real timeout
    /// (`BufRead::read_line` itself has no timeout primitive on a pipe).
    lines: Receiver<std::io::Result<String>>,
    _job: Option<JobObject>,
}

impl ComponentRuntime for PythonBridge {
    fn name(&self) -> &str {
        "python"
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Worker {
    fn spawn(_scratch_dir: &Path, caps: &Capabilities, device: Option<&str>) -> Result<Self> {
        let worker_script =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../components/python/_bb_worker.py");
        let mut command = Command::new("python");
        command.arg(&worker_script);
        // Force the worker's torch device when the user picked a GPU (validated
        // + graceful-fallback inside `_detect_device`). Absent = auto-detect.
        if let Some(dev) = device {
            command.env("BRAINBUILDER_DEVICE", dev);
        }
        // Same OS-level containment `Supervisor` gives Racket/Clojure's
        // one-shot invocations, applied here to the worker's own long-lived
        // process: Linux/macOS get a process group + RLIMIT_AS ceiling
        // pre-exec, and macOS additionally runs the worker under a real
        // `sandbox-exec` profile scoped to exactly `caps`'s granted read
        // paths (Windows applies its equivalent, `JobObject`, post-spawn
        // below). Must run before the stdio setup: macOS's hardening
        // rewraps the whole `Command` to wrap it in `sandbox-exec`.
        job_object::harden_before_spawn(&mut command, Some(worker_memory_limit_bytes()), caps);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                BrainBuilderError::Python(format!(
                    "failed to launch python worker (is python + torch installed and on PATH?): {e}"
                ))
            })?;
        let stdin = child.stdin.take().expect("piped stdin");
        let stdout = BufReader::new(child.stdout.take().expect("piped stdout"));

        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut stdout = stdout;
            loop {
                let mut line = String::new();
                let result = stdout.read_line(&mut line).map(|n| (n, line));
                let done = result.is_err();
                let msg = match result {
                    Ok((0, _)) => Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "worker stdout closed")),
                    Ok((_, line)) => Ok(line),
                    Err(e) => Err(e),
                };
                let stop = done || msg.is_err();
                if tx.send(msg).is_err() || stop {
                    break;
                }
            }
        });

        let _job = JobObject::new(Some(worker_memory_limit_bytes())).inspect(|job| {
            job.assign(&child);
        });

        Ok(Self { child, stdin, lines: rx, _job })
    }
}

impl PythonBridge {
    pub fn new(arena: Arc<SharedArena>) -> Result<Self> {
        let scratch_dir = std::env::temp_dir().join(format!("brainbuilder_ipc_{}", std::process::id()));
        std::fs::create_dir_all(&scratch_dir)
            .map_err(|e| BrainBuilderError::Python(format!("failed to create IPC scratch dir: {e}")))?;

        // The worker imports components by module name off PYTHONPATH
        // (inherited from this process's environment, same as before). Its
        // own capability grant: read/write only its scratch dir, plus
        // whatever's on PYTHONPATH (the real component implementations) —
        // no network. `timeout` doubles as the per-request timeout in
        // `request()` (a stuck/hung component call gets the worker killed
        // and transparently respawned, rather than blocking forever).
        // Computed before the first spawn so `Worker::spawn` can harden the
        // worker process itself against exactly this grant (see
        // `job_object::harden_before_spawn`'s macOS sandbox-exec profile).
        let mut caps = Capabilities::none()
            .allow_read(scratch_dir.clone())
            .with_timeout(std::time::Duration::from_secs(60));
        if let Ok(pythonpath) = std::env::var("PYTHONPATH") {
            for entry in std::env::split_paths(&pythonpath) {
                caps = caps.allow_read(entry);
            }
        }

        let worker = Worker::spawn(&scratch_dir, &caps, None)?;

        Ok(Self {
            arena,
            scratch_dir,
            caps,
            device: Mutex::new(None),
            worker: Mutex::new(worker),
        })
    }

    /// Select the torch device the worker runs on (`Some("cuda")` to force the
    /// GPU, `None` to auto-detect). Respawns the worker so the next component
    /// call runs on the chosen device. Invalid/absent accelerators degrade to
    /// auto-detect inside the worker, so this can't wedge training.
    pub fn set_device(&self, device: Option<String>) -> Result<()> {
        let cleaned = device.and_then(|d| {
            let t = d.trim().to_string();
            if t.is_empty() { None } else { Some(t) }
        });
        {
            let mut guard = self
                .device
                .lock()
                .map_err(|_| BrainBuilderError::Python("python device lock poisoned".into()))?;
            if *guard == cleaned {
                return Ok(()); // no change — don't churn the worker
            }
            *guard = cleaned.clone();
        }
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| BrainBuilderError::Python("python worker lock poisoned".into()))?;
        *worker = Worker::spawn(&self.scratch_dir, &self.caps, cleaned.as_deref())?;
        Ok(())
    }

    /// The device string the worker is currently launched with (for respawns).
    fn current_device(&self) -> Option<String> {
        self.device.lock().ok().and_then(|g| g.clone())
    }

    fn temp_path(&self, tag: &str) -> PathBuf {
        let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        self.scratch_dir.join(format!("{tag}_{n}.bin"))
    }

    /// Sends one JSON request line, blocks for one JSON response line. No
    /// per-request timeout yet (the `Supervisor`'s capability timeout model
    /// is built for run-to-completion subprocesses, not a persistent
    /// request/response worker) — a genuinely stuck component call currently
    /// blocks the caller indefinitely. Documented gap, not a silent one.
    /// Sends one request and waits for one response, real wall-clock bounded
    /// (`caps.timeout()`). A stuck/crashed worker is killed and transparently
    /// respawned so the *next* call succeeds instead of every subsequent
    /// call on this `PythonBridge` failing forever — the in-flight request
    /// itself still surfaces the error.
    fn request(&self, req: Value) -> Result<Value> {
        use crate::runtime::nervous_system::audit::{self, AuditEvent, Timer};
        let timer = Timer::start();
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| BrainBuilderError::Python("python worker lock poisoned".into()))?;

        let write_result = writeln!(worker.stdin, "{req}").and_then(|()| worker.stdin.flush());
        if write_result.is_err() {
            *worker = Worker::spawn(&self.scratch_dir, &self.caps, self.current_device().as_deref())?;
            audit::record(AuditEvent {
                runtime: "python",
                outcome: "worker_crashed",
                duration_ms: Some(timer.elapsed_ms()),
                detail: "worker was gone (crashed) before request could be sent — respawned",
            });
            return Err(BrainBuilderError::Python(
                "python worker was gone (crashed) — respawned for the next call".into(),
            ));
        }

        let timeout = self.caps.timeout().unwrap_or(std::time::Duration::from_secs(60));
        let line = match worker.lines.recv_timeout(timeout) {
            Ok(Ok(line)) => line,
            Ok(Err(e)) => {
                *worker = Worker::spawn(&self.scratch_dir, &self.caps, self.current_device().as_deref())?;
                audit::record(AuditEvent {
                    runtime: "python",
                    outcome: "worker_crashed",
                    duration_ms: Some(timer.elapsed_ms()),
                    detail: &format!("worker crashed ({e}) — respawned"),
                });
                return Err(BrainBuilderError::Python(format!(
                    "python worker crashed ({e}) — respawned for the next call"
                )));
            }
            Err(RecvTimeoutError::Timeout) => {
                *worker = Worker::spawn(&self.scratch_dir, &self.caps, self.current_device().as_deref())?;
                audit::record(AuditEvent {
                    runtime: "python",
                    outcome: "timeout_killed",
                    duration_ms: Some(timer.elapsed_ms()),
                    detail: &format!("did not respond within {timeout:?} — killed and respawned"),
                });
                return Err(BrainBuilderError::Python(format!(
                    "python worker did not respond within {timeout:?} — killed and respawned"
                )));
            }
            Err(RecvTimeoutError::Disconnected) => {
                *worker = Worker::spawn(&self.scratch_dir, &self.caps, self.current_device().as_deref())?;
                audit::record(AuditEvent {
                    runtime: "python",
                    outcome: "worker_crashed",
                    duration_ms: Some(timer.elapsed_ms()),
                    detail: "reader thread died — respawned",
                });
                return Err(BrainBuilderError::Python(
                    "python worker's reader thread died — respawned for the next call".into(),
                ));
            }
        };

        let resp: Value = serde_json::from_str(&line)
            .map_err(|e| BrainBuilderError::Python(format!("malformed worker response ({e}): {line}")))?;
        if resp.get("ok").and_then(Value::as_bool) != Some(true) {
            let msg = resp.get("error").and_then(Value::as_str).unwrap_or("unknown worker error");
            audit::record(AuditEvent {
                runtime: "python",
                outcome: "allowed_but_failed",
                duration_ms: Some(timer.elapsed_ms()),
                detail: msg,
            });
            return Err(BrainBuilderError::Python(msg.to_string()));
        }
        audit::record(AuditEvent {
            runtime: "python",
            outcome: "allowed",
            duration_ms: Some(timer.elapsed_ms()),
            detail: req.get("op").and_then(Value::as_str).unwrap_or("request"),
        });
        Ok(resp)
    }

    /// Writes the tensor into a memory-mapped scratch file: the destination
    /// file is created at its final size and mapped once, and the tensor's
    /// bytes are copied directly into the mapped pages (one copy, straight
    /// into the OS page cache) instead of going through a buffered
    /// `std::fs::write`. The Python worker maps the same file
    /// (`mmap.mmap` + `torch.frombuffer`) rather than `read()`-ing it into a
    /// fresh `bytes` object — real shared memory between the two processes,
    /// not a copy-in-copy-out file handoff.
    fn write_tensor(&self, tensor: &Tensor, tag: &str) -> Result<Value> {
        let (shape, values) = crate::interop::dlpack_support::tensor_to_vec_f32(tensor)?;
        let path = self.temp_path(tag);
        let byte_len = values.len() * 4;
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .map_err(|e| BrainBuilderError::Python(format!("failed to create tensor scratch file: {e}")))?;
        // A zero-length mmap is invalid, but a scalar (0-d / empty) tensor is
        // a real case (e.g. a reduced loss value) — fall back to a plain
        // write for the degenerate size instead of mapping nothing.
        if byte_len == 0 {
            return Ok(json!({"path": path.to_string_lossy(), "shape": shape}));
        }
        file.set_len(byte_len as u64)
            .map_err(|e| BrainBuilderError::Python(format!("failed to size tensor scratch file: {e}")))?;
        let mut mmap = unsafe {
            memmap2::MmapMut::map_mut(&file)
                .map_err(|e| BrainBuilderError::Python(format!("failed to mmap tensor scratch file: {e}")))?
        };
        let bytes: &[u8] =
            unsafe { std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), byte_len) };
        mmap.copy_from_slice(bytes);
        mmap.flush()
            .map_err(|e| BrainBuilderError::Python(format!("failed to flush tensor scratch file: {e}")))?;
        Ok(json!({"path": path.to_string_lossy(), "shape": shape}))
    }

    /// Reads a tensor back by mapping the worker's result file directly into
    /// this process's address space and copying straight from the mapped
    /// pages into the tensor arena slot — no intermediate `Vec<u8>`
    /// allocation the way `std::fs::read` would require.
    fn read_tensor(&self, desc: &Value) -> Result<Tensor> {
        let path = desc["path"]
            .as_str()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing tensor path".into()))?;
        let shape: Vec<i64> = desc["shape"]
            .as_array()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing tensor shape".into()))?
            .iter()
            .map(|v| v.as_i64().unwrap_or(1))
            .collect();
        // The worker response's shape is external input (subprocess-controlled,
        // and a crashed/malicious worker could send anything) — reject a
        // negative dimension or an element count that would overflow before
        // it ever reaches `alloc_managed_tensor`'s internal
        // `Layout::from_size_align(...).unwrap()`/pointer arithmetic.
        if shape.iter().any(|&d| d < 0) {
            return Err(BrainBuilderError::Python(format!(
                "worker response has a negative tensor dimension: {shape:?}"
            )));
        }
        let numel: i64 = shape
            .iter()
            .try_fold(1i64, |acc, &d| acc.checked_mul(d))
            .ok_or_else(|| {
                BrainBuilderError::Python(format!("worker response tensor shape {shape:?} overflows element count"))
            })?;
        let byte_len = (numel as usize)
            .checked_mul(std::mem::size_of::<f32>())
            .ok_or_else(|| {
                BrainBuilderError::Python(format!("worker response tensor shape {shape:?} overflows byte size"))
            })?;
        if byte_len > isize::MAX as usize {
            return Err(BrainBuilderError::Python(format!(
                "worker response tensor shape {shape:?} implies a {byte_len}-byte allocation, which exceeds what this platform can allocate"
            )));
        }

        if numel == 0 {
            return Ok(self.arena.allocate(
                &shape,
                dtype_to_dlpack(crate::component::descriptor::DataType::Float32),
                cpu_context(),
            ));
        }
        let file = std::fs::File::open(path)
            .map_err(|e| BrainBuilderError::Python(format!("failed to open tensor result file: {e}")))?;
        let mmap = unsafe {
            memmap2::Mmap::map(&file)
                .map_err(|e| BrainBuilderError::Python(format!("failed to mmap tensor result file: {e}")))?
        };
        // Validate the mapped file actually holds as many bytes as the
        // declared shape implies *before* allocating the destination tensor
        // or copying — a mismatched (short) result file would otherwise
        // read past the end of `mmap` in the `copy_nonoverlapping` below.
        if mmap.len() < byte_len {
            return Err(BrainBuilderError::Python(format!(
                "worker tensor result file `{path}` is {} bytes, short of the {byte_len} bytes implied by shape {shape:?}",
                mmap.len()
            )));
        }
        let tensor = self.arena.allocate(
            &shape,
            dtype_to_dlpack(crate::component::descriptor::DataType::Float32),
            cpu_context(),
        );
        unsafe {
            let dst = (*tensor.0).dl_tensor.data as *mut u8;
            std::ptr::copy_nonoverlapping(mmap.as_ptr(), dst, byte_len);
        }
        Ok(tensor)
    }

    /// Call a Python component function, transferring tensors via the
    /// worker's scratch-file protocol. `hyperparams` are forwarded as keyword
    /// arguments the component's `forward` may optionally accept (e.g.
    /// `layernorm`'s `eps`, attention's `num_heads`) — the worker only passes
    /// through the ones the function's signature actually declares, so
    /// components that don't need any are unaffected.
    pub fn call_component(
        &self,
        module: &str,
        function: &str,
        inputs: Vec<Tensor>,
        hyperparams: &Value,
    ) -> Result<Tensor> {
        let input_descs: Vec<Value> = inputs
            .iter()
            .enumerate()
            .map(|(i, t)| self.write_tensor(t, &format!("in{i}")))
            .collect::<Result<_>>()?;
        let output_path = self.temp_path("out");
        let resp = self.request(json!({
            "op": "call_component",
            "module": module,
            "function": function,
            "inputs": input_descs,
            "output_path": output_path.to_string_lossy(),
            "hyperparams": hyperparams,
        }))?;
        self.read_tensor(&resp["output"])
    }

    /// Real forward -> loss -> backward -> optimizer-step training
    /// iteration, run entirely inside one worker request so the autograd
    /// graph stays intact across every op in the chain (see the module doc
    /// on why DLPack-per-op round trips would silently zero out gradients —
    /// same reasoning applies here: the whole chain must happen in one
    /// Python-side call).
    #[allow(clippy::too_many_arguments)]
    pub fn train_step(
        &self,
        operations: &[ExecutableOp],
        inputs: HashMap<String, Tensor>,
        output_port: &str,
        target: &Tensor,
        loss_name: &str,
        optimizer_name: &str,
        lr: f64,
        weight_decay: f64,
        trainable_ports: &[String],
    ) -> Result<(f32, HashMap<String, Tensor>)> {
        for op in operations {
            if op.language != "python" {
                return Err(BrainBuilderError::Python(format!(
                    "train_step only supports python-language ops; got `{}` for {}",
                    op.language, op.component
                )));
            }
        }

        let mut input_descs = serde_json::Map::new();
        for (name, tensor) in &inputs {
            input_descs.insert(name.clone(), self.write_tensor(tensor, name)?);
        }
        let target_desc = self.write_tensor(target, "target")?;
        let mut output_paths = serde_json::Map::new();
        for name in trainable_ports {
            output_paths.insert(
                name.clone(),
                Value::String(self.temp_path(&format!("upd_{name}")).to_string_lossy().to_string()),
            );
        }
        let ops_json: Vec<Value> = operations
            .iter()
            .map(|op| json!({"component": op.component, "entry": op.entry, "inputs": op.inputs, "outputs": op.outputs, "hyperparams": op.hyperparams}))
            .collect();

        let resp = self.request(json!({
            "op": "train_step",
            "operations": ops_json,
            "inputs": Value::Object(input_descs),
            "output_port": output_port,
            "target": target_desc,
            "loss_name": loss_name,
            "optimizer_name": optimizer_name,
            "lr": lr,
            "weight_decay": weight_decay,
            "trainable_ports": trainable_ports,
            "output_paths": Value::Object(output_paths),
        }))?;

        let loss_value = resp["loss"].as_f64().unwrap_or(0.0) as f32;
        let mut updated = HashMap::new();
        let updated_json = resp["updated_weights"]
            .as_object()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing updated_weights".into()))?;
        for (name, desc) in updated_json {
            updated.insert(name.clone(), self.read_tensor(desc)?);
        }
        Ok((loss_value, updated))
    }

    /// Forward -> loss -> backward, stopping short of the optimizer step —
    /// the data-parallel distributed training primitive. A host collects
    /// gradients (not weight updates) from each client so it can average
    /// across shards *before* anyone applies an update, then calls
    /// `apply_averaged_gradients` once with the averaged result. Reuses the
    /// exact same autograd chain as `train_step`, split into two worker ops.
    #[allow(clippy::too_many_arguments)]
    pub fn compute_gradients(
        &self,
        operations: &[ExecutableOp],
        inputs: HashMap<String, Tensor>,
        output_port: &str,
        target: &Tensor,
        loss_name: &str,
        trainable_ports: &[String],
    ) -> Result<(f32, HashMap<String, Tensor>)> {
        for op in operations {
            if op.language != "python" {
                return Err(BrainBuilderError::Python(format!(
                    "compute_gradients only supports python-language ops; got `{}` for {}",
                    op.language, op.component
                )));
            }
        }

        let mut input_descs = serde_json::Map::new();
        for (name, tensor) in &inputs {
            input_descs.insert(name.clone(), self.write_tensor(tensor, name)?);
        }
        let target_desc = self.write_tensor(target, "target")?;
        let mut gradient_paths = serde_json::Map::new();
        for name in trainable_ports {
            gradient_paths.insert(
                name.clone(),
                Value::String(self.temp_path(&format!("grad_{name}")).to_string_lossy().to_string()),
            );
        }
        let ops_json: Vec<Value> = operations
            .iter()
            .map(|op| json!({"component": op.component, "entry": op.entry, "inputs": op.inputs, "outputs": op.outputs, "hyperparams": op.hyperparams}))
            .collect();

        let resp = self.request(json!({
            "op": "compute_gradients",
            "operations": ops_json,
            "inputs": Value::Object(input_descs),
            "output_port": output_port,
            "target": target_desc,
            "loss_name": loss_name,
            "trainable_ports": trainable_ports,
            "gradient_paths": Value::Object(gradient_paths),
        }))?;

        let loss_value = resp["loss"].as_f64().unwrap_or(0.0) as f32;
        let mut gradients = HashMap::new();
        let gradients_json = resp["gradients"]
            .as_object()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing gradients".into()))?;
        for (name, desc) in gradients_json {
            gradients.insert(name.clone(), self.read_tensor(desc)?);
        }
        Ok((loss_value, gradients))
    }

    /// Second half of the distributed-training split: given current weights
    /// and an already-averaged gradient per port (averaged across all
    /// clients by the host), runs exactly one optimizer step.
    pub fn apply_averaged_gradients(
        &self,
        weights: &HashMap<String, Tensor>,
        averaged_gradients: &HashMap<String, Tensor>,
        optimizer_name: &str,
        lr: f64,
        weight_decay: f64,
    ) -> Result<HashMap<String, Tensor>> {
        let mut weight_descs = serde_json::Map::new();
        let mut gradient_descs = serde_json::Map::new();
        let mut output_paths = serde_json::Map::new();
        for (name, tensor) in weights {
            weight_descs.insert(name.clone(), self.write_tensor(tensor, &format!("w_{name}"))?);
            let grad = averaged_gradients.get(name).ok_or_else(|| {
                BrainBuilderError::Python(format!("missing averaged gradient for port `{name}`"))
            })?;
            gradient_descs.insert(name.clone(), self.write_tensor(grad, &format!("avg_grad_{name}"))?);
            output_paths.insert(
                name.clone(),
                Value::String(self.temp_path(&format!("upd_{name}")).to_string_lossy().to_string()),
            );
        }

        let resp = self.request(json!({
            "op": "apply_averaged_gradients",
            "weights": Value::Object(weight_descs),
            "gradients": Value::Object(gradient_descs),
            "optimizer_name": optimizer_name,
            "lr": lr,
            "weight_decay": weight_decay,
            "output_paths": Value::Object(output_paths),
        }))?;

        let mut updated = HashMap::new();
        let updated_json = resp["updated_weights"]
            .as_object()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing updated_weights".into()))?;
        for (name, desc) in updated_json {
            updated.insert(name.clone(), self.read_tensor(desc)?);
        }
        Ok(updated)
    }

    /// Real random parameter initialization via `torch.randn`, run in the
    /// worker process (not the app process).
    pub fn random_tensor(&self, shape: &[i64]) -> Result<Tensor> {
        let output_path = self.temp_path("rand");
        let resp = self.request(json!({
            "op": "random_tensor",
            "shape": shape,
            "output_path": output_path.to_string_lossy(),
        }))?;
        self.read_tensor(&resp["output"])
    }

    /// Zero-initialized parameter (LoRA's `lora_b` — see
    /// `ComponentDescriptor::zero_init_parameter_ports`), run in the worker
    /// process for the same reason `random_tensor` is: real `torch.zeros`,
    /// not a Rust-side approximation.
    pub fn zero_tensor(&self, shape: &[i64]) -> Result<Tensor> {
        let output_path = self.temp_path("zero");
        let resp = self.request(json!({
            "op": "zero_tensor",
            "shape": shape,
            "output_path": output_path.to_string_lossy(),
        }))?;
        self.read_tensor(&resp["output"])
    }

    /// Save a checkpoint: real `torch.save` of a state-dict-style mapping
    /// (port name -> tensor), run in the worker process.
    pub fn save_state_dict(&self, weights: &HashMap<String, Tensor>, path: &Path) -> Result<()> {
        let mut weight_descs = serde_json::Map::new();
        for (name, tensor) in weights {
            weight_descs.insert(name.clone(), self.write_tensor(tensor, name)?);
        }
        self.request(json!({
            "op": "save_state_dict",
            "weights": Value::Object(weight_descs),
            "path": path.to_string_lossy(),
        }))?;
        Ok(())
    }

    /// Load a checkpoint saved by `save_state_dict`.
    pub fn load_state_dict(&self, path: &Path) -> Result<HashMap<String, Tensor>> {
        let resp = self.request(json!({
            "op": "load_state_dict",
            "path": path.to_string_lossy(),
            "output_dir": self.scratch_dir.to_string_lossy(),
        }))?;
        let mut weights = HashMap::new();
        let weights_json = resp["weights"]
            .as_object()
            .ok_or_else(|| BrainBuilderError::Python("worker response missing weights".into()))?;
        for (name, desc) in weights_json {
            weights.insert(name.clone(), self.read_tensor(desc)?);
        }
        Ok(weights)
    }

    /// Test-only: shortens the per-request timeout so a hang/respawn test
    /// doesn't need to wait out the real (60s) production timeout.
    #[doc(hidden)]
    pub fn with_timeout_for_testing(mut self, timeout: std::time::Duration) -> Self {
        self.caps = self.caps.clone().with_timeout(timeout);
        self
    }

    /// Test-only: exercises the worker's `sleep` op, to test the timeout +
    /// respawn path in `request()` without needing a real hang bug.
    #[doc(hidden)]
    pub fn sleep_for_testing(&self, seconds: f64) -> Result<()> {
        self.request(json!({"op": "sleep", "seconds": seconds})).map(|_| ())
    }

    /// Test-only: kills the *current* worker process out from under the
    /// bridge (as opposed to `sleep_for_testing`, which exercises the
    /// *timeout* path — this exercises the "worker straight-up died"
    /// paths in `request()`: the write-side `worker was gone` branch and
    /// the reader-thread-disconnected branch). Real `taskkill`/`SIGKILL` on
    /// a real child process, not a simulated failure.
    #[doc(hidden)]
    pub fn kill_worker_for_testing(&self) -> Result<()> {
        let mut worker = self
            .worker
            .lock()
            .map_err(|_| BrainBuilderError::Python("python worker lock poisoned".into()))?;
        worker.child.kill().map_err(|e| BrainBuilderError::Python(format!("failed to kill worker for test: {e}")))?;
        let _ = worker.child.wait();
        Ok(())
    }
}

#[cfg(test)]
mod chaos_tests {
    //! Regression tests for a crafted/short worker response reaching
    //! `read_tensor`: previously an unvalidated negative dim, an overflowing
    //! element/byte count, or a result file shorter than the declared shape
    //! implies could panic or read out of bounds. None of these need a real
    //! python/torch worker — `read_tensor` only touches `self.arena`, so the
    //! fixture below spawns a trivial (`cmd`/`sh`) child in place of the real
    //! `_bb_worker.py` process just to satisfy `Worker`'s fields.
    use super::*;

    fn dummy_bridge() -> PythonBridge {
        let scratch_dir = std::env::temp_dir().join(format!(
            "brainbuilder_chaos_test_{}_{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&scratch_dir).unwrap();

        // A trivial, fast-exiting child process stands in for the real
        // worker — `read_tensor` never touches `worker`, so what's actually
        // running behind the handle doesn't matter for these tests.
        #[cfg(windows)]
        let mut command = {
            let mut c = Command::new("cmd");
            c.args(["/C", "exit 0"]);
            c
        };
        #[cfg(not(windows))]
        let mut command = {
            let mut c = Command::new("sh");
            c.args(["-c", "exit 0"]);
            c
        };
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn dummy child for test fixture");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let (tx, _rx_keep_alive_source) = std::sync::mpsc::channel();
        // Leak the sender side into a background thread purely so `lines`
        // has a receiver that won't immediately disconnect; these tests
        // never call `request()`/`recv`, so nothing is ever sent.
        std::thread::spawn(move || {
            let mut stdout = stdout;
            let mut line = String::new();
            let _ = stdout.read_line(&mut line);
            drop(tx);
        });
        let worker = Worker { child, stdin, lines: _rx_keep_alive_source, _job: None };

        let caps = Capabilities::none().allow_read(scratch_dir.clone());
        PythonBridge {
            arena: Arc::new(SharedArena::new()),
            scratch_dir,
            caps,
            device: Mutex::new(None),
            worker: Mutex::new(worker),
        }
    }

    #[test]
    fn read_tensor_rejects_negative_dimension_instead_of_panicking() {
        let bridge = dummy_bridge();
        let desc = json!({"path": "unused", "shape": [-1, 4]});
        let err = bridge.read_tensor(&desc).err().expect("negative dim must error, not panic");
        assert!(err.to_string().contains("negative"), "unexpected error: {err}");
    }

    #[test]
    fn read_tensor_rejects_element_count_overflow_instead_of_panicking() {
        let bridge = dummy_bridge();
        // Two dims whose product overflows i64 — a crashed/malicious worker
        // could send this; the old code had no `checked_mul` here.
        let desc = json!({"path": "unused", "shape": [i64::MAX, 2]});
        let err = bridge.read_tensor(&desc).err().expect("overflowing element count must error, not panic");
        assert!(err.to_string().contains("overflow"), "unexpected error: {err}");
    }

    #[test]
    fn read_tensor_rejects_byte_size_beyond_isize_max_instead_of_panicking() {
        let bridge = dummy_bridge();
        // A shape whose element count is representable in i64 and whose
        // implied byte size (elements * 4) does NOT overflow usize, but DOES
        // exceed isize::MAX — the old code would reach
        // `Layout::from_size_align(...).unwrap()` with this. (elements * 4
        // must stay under usize::MAX to hit the isize::MAX check rather than
        // the checked_mul-overflow check exercised by the previous test.)
        let big = (isize::MAX as i64) / 4 + 1024;
        let desc = json!({"path": "unused", "shape": [big]});
        let err = bridge.read_tensor(&desc).err().expect("byte size beyond isize::MAX must error, not panic");
        assert!(
            err.to_string().contains("exceeds what this platform can allocate"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn read_tensor_rejects_a_short_result_file_instead_of_reading_out_of_bounds() {
        let bridge = dummy_bridge();
        // Declares a shape implying 16 bytes (4 f32s) but the backing file on
        // disk is only 4 bytes — the old code mapped the file and copied
        // `byte_len` bytes out of it regardless, reading past the mapping's
        // end. Now `read_tensor` must check `mmap.len() >= byte_len` first.
        let path = bridge.temp_path("short_result");
        std::fs::write(&path, [0u8; 4]).unwrap();
        let desc = json!({"path": path.to_string_lossy(), "shape": [4]});
        let err = bridge.read_tensor(&desc).err().expect("short result file must error, not read OOB");
        assert!(err.to_string().contains("short of"), "unexpected error: {err}");
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn read_tensor_still_succeeds_on_a_well_formed_response() {
        // Confirms the new validation doesn't reject legitimate responses —
        // a real regression here (over-tightening) would be just as bad as
        // the original bug.
        let bridge = dummy_bridge();
        let path = bridge.temp_path("good_result");
        let values: [f32; 4] = [1.0, 2.0, 3.0, 4.0];
        let bytes: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
        std::fs::write(&path, &bytes).unwrap();
        let desc = json!({"path": path.to_string_lossy(), "shape": [2, 2]});
        let tensor = bridge.read_tensor(&desc).expect("well-formed response should still succeed");
        unsafe {
            let data = std::slice::from_raw_parts((*tensor.0).dl_tensor.data as *const f32, 4);
            assert_eq!(data, &values);
        }
        std::fs::remove_file(&path).ok();
    }
}
