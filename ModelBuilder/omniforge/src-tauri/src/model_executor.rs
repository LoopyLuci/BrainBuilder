//! Model execution backend with structured error handling.
//!
//! - **GGUF**: probes for llama-server / llama-cpp binaries, validates files,
//!   health-checks the HTTP endpoint, tracks process status.
//! - **ONNX**: uses the `ort` crate when built with `--features onnx`.
//!
//! Build with ONNX:
//!   cargo build --features onnx

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{info, warn};
use uuid::Uuid;

// ── Structured errors ───────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExecError {
    FileNotFound { path: String },
    InvalidFormat { path: String, expected: String, detail: String },
    BinaryNotFound { names_tried: Vec<String> },
    ProcessSpawn { detail: String },
    ProcessExited { process_id: String, code: Option<i32> },
    HealthCheckFailed { endpoint: String, detail: String },
    SessionNotFound { session_id: String },
    OnnxNotCompiled,
    OnnxInit { detail: String },
    OnnxLoad { path: String, detail: String },
    OnnxInfer { session_id: String, detail: String },
    InvalidInput { detail: String },
    Internal { detail: String },
}

impl std::fmt::Display for ExecError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExecError::FileNotFound { path } => write!(f, "File not found: {path}"),
            ExecError::InvalidFormat { path, expected, detail } => {
                write!(f, "Invalid {expected} file '{path}': {detail}")
            }
            ExecError::BinaryNotFound { names_tried } => {
                write!(
                    f,
                    "No GGUF runner found. Tried: {}. Install llama.cpp and ensure it is on PATH.",
                    names_tried.join(", ")
                )
            }
            ExecError::ProcessSpawn { detail } => write!(f, "Failed to spawn process: {detail}"),
            ExecError::ProcessExited { process_id, code } => {
                write!(f, "Process {process_id} exited early (code={code:?})")
            }
            ExecError::HealthCheckFailed { endpoint, detail } => {
                write!(f, "Health check failed for {endpoint}: {detail}")
            }
            ExecError::SessionNotFound { session_id } => {
                write!(f, "Unknown session: {session_id}")
            }
            ExecError::OnnxNotCompiled => write!(
                f,
                "ONNX Runtime support is not compiled in. Rebuild with `--features onnx`."
            ),
            ExecError::OnnxInit { detail } => write!(f, "ONNX Runtime init failed: {detail}"),
            ExecError::OnnxLoad { path, detail } => {
                write!(f, "Failed to load ONNX model '{path}': {detail}")
            }
            ExecError::OnnxInfer { session_id, detail } => {
                write!(f, "ONNX inference failed ({session_id}): {detail}")
            }
            ExecError::InvalidInput { detail } => write!(f, "Invalid input: {detail}"),
            ExecError::Internal { detail } => write!(f, "Internal error: {detail}"),
        }
    }
}

impl From<ExecError> for String {
    fn from(e: ExecError) -> String {
        e.to_string()
    }
}

// ── Result types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
pub struct InferenceResult {
    pub process_id: String,
    pub outputs: serde_json::Value,
    pub backend: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct GgufProcessInfo {
    pub id: String,
    pub model_path: String,
    pub port: u16,
    pub endpoint: String,
    pub status: String,
}

#[derive(Debug, Clone)]
struct GgufHandle {
    child: Child,
    model_path: String,
    port: u16,
}

// ── Executor ────────────────────────────────────────────────────────────

pub struct ModelExecutor {
    processes: Arc<Mutex<HashMap<String, GgufHandle>>>,
    onnx_sessions: Arc<Mutex<HashMap<String, OnnxSessionHandle>>>,
    /// Candidate binary names for GGUF runners (first match wins)
    gguf_binaries: Vec<String>,
}

enum OnnxSessionHandle {
    #[cfg(feature = "onnx")]
    Live { session: ort::session::Session, path: String },
    #[cfg(not(feature = "onnx"))]
    Stub { model_path: String },
}

impl ModelExecutor {
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            onnx_sessions: Arc::new(Mutex::new(HashMap::new())),
            gguf_binaries: vec![
                "llama-server".into(),
                "llama-cpp-server".into(),
                "server".into(), // some llama.cpp builds ship as just `server`
            ],
        }
    }

    /// Override the list of GGUF runner binaries to probe.
    pub fn with_gguf_binaries(mut self, names: Vec<String>) -> Self {
        if !names.is_empty() {
            self.gguf_binaries = names;
        }
        self
    }

    // ── Validation helpers ──────────────────────────────────────────────

    fn validate_file(path: &str, expected_exts: &[&str]) -> Result<PathBuf, ExecError> {
        let p = PathBuf::from(path);
        if !p.exists() {
            return Err(ExecError::FileNotFound {
                path: path.to_string(),
            });
        }
        if !p.is_file() {
            return Err(ExecError::InvalidFormat {
                path: path.to_string(),
                expected: expected_exts.join("/"),
                detail: "path is not a regular file".into(),
            });
        }
        let ext = p
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if !expected_exts.is_empty() && !expected_exts.iter().any(|e| *e == ext) {
            return Err(ExecError::InvalidFormat {
                path: path.to_string(),
                expected: expected_exts.join("/"),
                detail: format!("got extension '.{ext}'"),
            });
        }
        // Basic size sanity check
        if let Ok(meta) = std::fs::metadata(&p) {
            if meta.len() < 64 {
                return Err(ExecError::InvalidFormat {
                    path: path.to_string(),
                    expected: expected_exts.join("/"),
                    detail: "file is suspiciously small (< 64 bytes)".into(),
                });
            }
        }
        Ok(p)
    }

    fn find_gguf_binary(&self) -> Result<String, ExecError> {
        for name in &self.gguf_binaries {
            // `which`-style probe
            if Command::new(name)
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false)
            {
                return Ok(name.clone());
            }
            // Also accept if the binary exists even without --version support
            if Command::new(name)
                .arg("-h")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok()
            {
                return Ok(name.clone());
            }
        }
        Err(ExecError::BinaryNotFound {
            names_tried: self.gguf_binaries.clone(),
        })
    }

    // ── GGUF ────────────────────────────────────────────────────────────

    /// Start a GGUF model server.
    ///
    /// Options (via env or defaults):
    /// - host: 127.0.0.1
    /// - ctx size: 2048
    /// - n_gpu_layers: 0 (CPU) – set `OMNIFORGE_N_GPU_LAYERS` to offload
    pub async fn start_gguf(&self, model_path: &str, port: u16) -> Result<String, String> {
        self.start_gguf_inner(model_path, port)
            .await
            .map_err(Into::into)
    }

    async fn start_gguf_inner(&self, model_path: &str, port: u16) -> Result<String, ExecError> {
        let path = Self::validate_file(model_path, &["gguf"])?;
        let binary = self.find_gguf_binary()?;

        // Refuse if port already in use by us
        {
            let procs = self.processes.lock().await;
            if procs.values().any(|h| h.port == port) {
                return Err(ExecError::ProcessSpawn {
                    detail: format!("port {port} is already used by another GGUF process"),
                });
            }
        }

        let n_gpu = std::env::var("OMNIFORGE_N_GPU_LAYERS")
            .ok()
            .and_then(|v| v.parse::<i32>().ok())
            .unwrap_or(0);
        let ctx = std::env::var("OMNIFORGE_CTX_SIZE")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(2048);

        let id = format!("gguf-{}", Uuid::new_v4());
        info!(
            id = %id,
            model = %path.display(),
            port,
            binary = %binary,
            n_gpu,
            ctx,
            "Starting GGUF model"
        );

        let mut cmd = Command::new(&binary);
        cmd.args([
            "-m",
            path.to_str().unwrap_or(model_path),
            "--port",
            &port.to_string(),
            "--host",
            "127.0.0.1",
            "-c",
            &ctx.to_string(),
        ]);
        if n_gpu != 0 {
            cmd.args(["-ngl", &n_gpu.to_string()]);
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| ExecError::ProcessSpawn {
            detail: e.to_string(),
        })?;

        // Brief wait then check the process didn't die immediately
        tokio::time::sleep(Duration::from_millis(400)).await;
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code();
                let stderr = child
                    .stderr
                    .take()
                    .map(|mut s| {
                        let mut buf = String::new();
                        let _ = std::io::Read::read_to_string(&mut s, &mut buf);
                        buf
                    })
                    .unwrap_or_default();
                return Err(ExecError::ProcessExited {
                    process_id: id,
                    code,
                }
                .pipe_detail(stderr));
            }
            Ok(None) => {} // still running – good
            Err(e) => {
                return Err(ExecError::ProcessSpawn {
                    detail: format!("try_wait failed: {e}"),
                });
            }
        }

        // Optional health check against llama-server HTTP API
        let endpoint = format!("http://127.0.0.1:{port}/health");
        if let Err(e) = self.health_check(&endpoint, 8).await {
            warn!(%endpoint, error = %e, "GGUF health check failed – process may still be starting");
            // Non-fatal: some builds don't expose /health; keep the process
        }

        self.processes.lock().await.insert(
            id.clone(),
            GgufHandle {
                child,
                model_path: path.display().to_string(),
                port,
            },
        );
        Ok(id)
    }

    async fn health_check(&self, endpoint: &str, attempts: u32) -> Result<(), ExecError> {
        for i in 0..attempts {
            match reqwest_get(endpoint).await {
                Ok(status) if status >= 200 && status < 500 => return Ok(()),
                Ok(status) => {
                    if i + 1 == attempts {
                        return Err(ExecError::HealthCheckFailed {
                            endpoint: endpoint.into(),
                            detail: format!("HTTP {status}"),
                        });
                    }
                }
                Err(e) => {
                    if i + 1 == attempts {
                        return Err(ExecError::HealthCheckFailed {
                            endpoint: endpoint.into(),
                            detail: e,
                        });
                    }
                }
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Ok(())
    }

    /// List running GGUF processes with metadata.
    pub async fn list_gguf(&self) -> Vec<GgufProcessInfo> {
        let mut procs = self.processes.lock().await;
        let mut out = Vec::new();
        let mut dead = Vec::new();

        for (id, handle) in procs.iter_mut() {
            let status = match handle.child.try_wait() {
                Ok(Some(s)) => {
                    dead.push(id.clone());
                    format!("exited:{:?}", s.code())
                }
                Ok(None) => "running".into(),
                Err(e) => format!("error:{e}"),
            };
            out.push(GgufProcessInfo {
                id: id.clone(),
                model_path: handle.model_path.clone(),
                port: handle.port,
                endpoint: format!("http://127.0.0.1:{}", handle.port),
                status,
            });
        }
        for id in dead {
            procs.remove(&id);
        }
        out
    }

    // ── ONNX ────────────────────────────────────────────────────────────

    pub async fn load_onnx(&self, model_path: &str) -> Result<String, String> {
        self.load_onnx_inner(model_path).await.map_err(Into::into)
    }

    async fn load_onnx_inner(&self, model_path: &str) -> Result<String, ExecError> {
        let path = Self::validate_file(model_path, &["onnx"])?;
        let id = format!("onnx-{}", Uuid::new_v4());
        info!(id = %id, model = %path.display(), "Loading ONNX model");

        #[cfg(feature = "onnx")]
        {
            ort::init().commit().map_err(|e| ExecError::OnnxInit {
                detail: e.to_string(),
            })?;

            let session = ort::session::Session::builder()
                .map_err(|e| ExecError::OnnxInit {
                    detail: e.to_string(),
                })?
                .with_optimization_level(ort::session::builder::GraphOptimizationLevel::Level3)
                .map_err(|e| ExecError::OnnxInit {
                    detail: e.to_string(),
                })?
                .commit_from_file(&path)
                .map_err(|e| ExecError::OnnxLoad {
                    path: path.display().to_string(),
                    detail: e.to_string(),
                })?;

            let inputs: Vec<_> = session.inputs.iter().map(|i| i.name.clone()).collect();
            let outputs: Vec<_> = session.outputs.iter().map(|o| o.name.clone()).collect();
            info!(?inputs, ?outputs, "ONNX model loaded");

            self.onnx_sessions.lock().await.insert(
                id.clone(),
                OnnxSessionHandle::Live {
                    session,
                    path: path.display().to_string(),
                },
            );
            Ok(id)
        }

        #[cfg(not(feature = "onnx"))]
        {
            let _ = path;
            self.onnx_sessions.lock().await.insert(
                id.clone(),
                OnnxSessionHandle::Stub {
                    model_path: model_path.to_string(),
                },
            );
            // Return id but inference will fail with OnnxNotCompiled
            warn!("ONNX feature not enabled – stub session stored");
            Ok(id)
        }
    }

    pub async fn infer_onnx(
        &self,
        session_id: &str,
        inputs: serde_json::Value,
    ) -> Result<InferenceResult, String> {
        self.infer_onnx_inner(session_id, inputs)
            .await
            .map_err(Into::into)
    }

    async fn infer_onnx_inner(
        &self,
        session_id: &str,
        inputs: serde_json::Value,
    ) -> Result<InferenceResult, ExecError> {
        let mut sessions = self.onnx_sessions.lock().await;
        let handle = sessions
            .get_mut(session_id)
            .ok_or_else(|| ExecError::SessionNotFound {
                session_id: session_id.to_string(),
            })?;

        #[cfg(feature = "onnx")]
        {
            match handle {
                OnnxSessionHandle::Live { session, .. } => {
                    use ndarray::Array;
                    use ort::value::TensorRef;

                    let mut ort_inputs: Vec<(String, ort::value::Value)> = Vec::new();
                    let shapes = inputs
                        .get("__shapes__")
                        .and_then(|s| s.as_object())
                        .cloned()
                        .unwrap_or_default();

                    let obj = inputs.as_object().ok_or_else(|| ExecError::InvalidInput {
                        detail: "inputs must be a JSON object".into(),
                    })?;

                    for (name, val) in obj {
                        if name == "__shapes__" {
                            continue;
                        }
                        let flat: Vec<f32> = val
                            .as_array()
                            .ok_or_else(|| ExecError::InvalidInput {
                                detail: format!("input '{name}' must be a numeric array"),
                            })?
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        if flat.is_empty() {
                            return Err(ExecError::InvalidInput {
                                detail: format!("input '{name}' is empty"),
                            });
                        }

                        let shape: Vec<usize> = shapes
                            .get(name)
                            .and_then(|s| s.as_array())
                            .map(|a| {
                                a.iter()
                                    .filter_map(|v| v.as_u64().map(|u| u as usize))
                                    .collect()
                            })
                            .unwrap_or_else(|| vec![1, flat.len()]);

                        let arr = Array::from_shape_vec(shape, flat).map_err(|e| {
                            ExecError::InvalidInput {
                                detail: format!("shape mismatch for '{name}': {e}"),
                            }
                        })?;
                        let tensor = TensorRef::from_array_view(arr.view())
                            .map_err(|e| ExecError::OnnxInfer {
                                session_id: session_id.into(),
                                detail: format!("tensor '{name}': {e}"),
                            })?
                            .into();
                        ort_inputs.push((name.clone(), tensor));
                    }

                    let outputs =
                        session
                            .run(ort_inputs)
                            .map_err(|e| ExecError::OnnxInfer {
                                session_id: session_id.into(),
                                detail: e.to_string(),
                            })?;

                    let mut out_map = serde_json::Map::new();
                    for (name, value) in outputs.iter() {
                        if let Ok(tensor) = value.try_extract_tensor::<f32>() {
                            let (_, data) = tensor;
                            let preview: Vec<f32> = data.iter().take(64).copied().collect();
                            out_map.insert(
                                name.to_string(),
                                serde_json::json!({ "preview": preview, "len": data.len() }),
                            );
                        }
                    }

                    Ok(InferenceResult {
                        process_id: session_id.to_string(),
                        outputs: serde_json::Value::Object(out_map),
                        backend: "onnxruntime".into(),
                    })
                }
                #[cfg(not(feature = "onnx"))]
                OnnxSessionHandle::Stub { .. } => unreachable!(),
            }
        }

        #[cfg(not(feature = "onnx"))]
        {
            let _ = handle;
            Err(ExecError::OnnxNotCompiled)
        }
    }

    pub async fn unload_onnx(&self, session_id: &str) -> Result<(), String> {
        let mut sessions = self.onnx_sessions.lock().await;
        if sessions.remove(session_id).is_some() {
            info!(session_id, "ONNX session unloaded");
            Ok(())
        } else {
            Err(ExecError::SessionNotFound {
                session_id: session_id.to_string(),
            }
            .into())
        }
    }

    // ── Lifecycle ───────────────────────────────────────────────────────

    pub async fn stop(&self, process_id: &str) -> Result<(), String> {
        {
            let mut procs = self.processes.lock().await;
            if let Some(mut handle) = procs.remove(process_id) {
                let _ = handle.child.kill();
                let _ = handle.child.wait();
                info!(process_id, "GGUF process stopped");
                return Ok(());
            }
        }
        self.unload_onnx(process_id).await
    }

    pub async fn list(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.processes.lock().await.keys().cloned().collect();
        ids.extend(self.onnx_sessions.lock().await.keys().cloned());
        ids
    }
}

// Tiny helper so ProcessExited can attach stderr without a second match arm
trait PipeDetail {
    fn pipe_detail(self, stderr: String) -> Self;
}
impl PipeDetail for ExecError {
    fn pipe_detail(self, stderr: String) -> Self {
        match self {
            ExecError::ProcessExited { process_id, code } if !stderr.is_empty() => {
                ExecError::ProcessSpawn {
                    detail: format!(
                        "process {process_id} exited (code={code:?}): {}",
                        stderr.chars().take(400).collect::<String>()
                    ),
                }
            }
            other => other,
        }
    }
}

/// Minimal HTTP GET without adding reqwest as a hard dep when unused.
/// Uses std only via a blocking call on a worker thread; returns status code.
async fn reqwest_get(url: &str) -> Result<u16, String> {
    // Prefer reqwest if the host crate already pulls it transitively via concierge-core
    let url = url.to_string();
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(&url).send().map_err(|e| e.to_string())?;
        Ok(resp.status().as_u16())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_not_found_error() {
        let err = ModelExecutor::validate_file("/tmp/no-such-omniforge-model.gguf", &["gguf"]);
        assert!(matches!(err, Err(ExecError::FileNotFound { .. })));
    }

    #[test]
    fn invalid_extension() {
        let path = "/tmp/omniforge-test-model.bin";
        std::fs::write(path, vec![0u8; 128]).unwrap();
        let err = ModelExecutor::validate_file(path, &["gguf"]);
        assert!(matches!(err, Err(ExecError::InvalidFormat { .. })));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn accepts_valid_gguf_extension() {
        let path = "/tmp/omniforge-test-model.gguf";
        std::fs::write(path, vec![0u8; 128]).unwrap();
        assert!(ModelExecutor::validate_file(path, &["gguf"]).is_ok());
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn rejects_tiny_file() {
        let path = "/tmp/omniforge-tiny.onnx";
        std::fs::write(path, b"xx").unwrap();
        let err = ModelExecutor::validate_file(path, &["onnx"]);
        assert!(matches!(err, Err(ExecError::InvalidFormat { .. })));
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn exec_error_display_messages() {
        let e = ExecError::BinaryNotFound {
            names_tried: vec!["llama-server".into(), "server".into()],
        };
        let s = e.to_string();
        assert!(s.contains("llama-server"));
        assert!(s.contains("PATH"));

        let e = ExecError::OnnxNotCompiled;
        assert!(e.to_string().contains("features onnx"));

        let e = ExecError::SessionNotFound {
            session_id: "onnx-123".into(),
        };
        assert!(e.to_string().contains("onnx-123"));
    }

    #[test]
    fn find_binary_reports_tried_names() {
        let exec = ModelExecutor::new().with_gguf_binaries(vec![
            "definitely-not-a-real-binary-xyz".into(),
        ]);
        let err = exec.find_gguf_binary().unwrap_err();
        match err {
            ExecError::BinaryNotFound { names_tried } => {
                assert!(names_tried[0].contains("definitely-not"));
            }
            other => panic!("unexpected: {other}"),
        }
    }
}
