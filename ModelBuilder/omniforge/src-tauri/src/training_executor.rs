//! Launches Python LoRA/QLoRA jobs with structured validation and error handling.
//! Streams progress via Tauri events.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tauri::Manager;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub base_model: String,
    pub dataset_path: String,
    pub output_km_path: String,
    pub recipe: String, // "lora" | "qlora"
    pub rank: u32,
    pub alpha: f32,
    pub learning_rate: f64,
    pub epochs: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TrainingError {
    InvalidConfig { field: String, detail: String },
    DatasetNotFound { path: String },
    DatasetEmpty { path: String },
    ScriptNotFound { path: String },
    PythonNotFound,
    SpawnFailed { detail: String },
    ProcessFailed { job_id: String, exit_code: Option<i32>, stderr_tail: String },
    Cancelled { job_id: String },
    Internal { detail: String },
}

impl std::fmt::Display for TrainingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TrainingError::InvalidConfig { field, detail } => {
                write!(f, "Invalid config ({field}): {detail}")
            }
            TrainingError::DatasetNotFound { path } => {
                write!(f, "Dataset not found: {path}")
            }
            TrainingError::DatasetEmpty { path } => {
                write!(f, "Dataset is empty: {path}")
            }
            TrainingError::ScriptNotFound { path } => {
                write!(f, "Training script not found: {path}")
            }
            TrainingError::PythonNotFound => {
                write!(f, "Python not found on PATH (tried python / python3). Install Python 3.10+ and ensure it is on PATH.")
            }
            TrainingError::SpawnFailed { detail } => {
                write!(f, "Failed to start training process: {detail}")
            }
            TrainingError::ProcessFailed {
                job_id,
                exit_code,
                stderr_tail,
            } => write!(
                f,
                "Training job {job_id} failed (exit={exit_code:?}): {stderr_tail}"
            ),
            TrainingError::Cancelled { job_id } => {
                write!(f, "Training job {job_id} was cancelled")
            }
            TrainingError::Internal { detail } => write!(f, "Internal training error: {detail}"),
        }
    }
}

impl From<TrainingError> for String {
    fn from(e: TrainingError) -> String {
        e.to_string()
    }
}

fn validate_config(config: &TrainingConfig) -> Result<(), TrainingError> {
    if config.base_model.trim().is_empty() {
        return Err(TrainingError::InvalidConfig {
            field: "base_model".into(),
            detail: "must not be empty".into(),
        });
    }
    if !matches!(config.recipe.as_str(), "lora" | "qlora") {
        return Err(TrainingError::InvalidConfig {
            field: "recipe".into(),
            detail: format!("expected 'lora' or 'qlora', got '{}'", config.recipe),
        });
    }
    if config.rank == 0 || config.rank > 256 {
        return Err(TrainingError::InvalidConfig {
            field: "rank".into(),
            detail: "must be in 1..=256".into(),
        });
    }
    if config.alpha <= 0.0 {
        return Err(TrainingError::InvalidConfig {
            field: "alpha".into(),
            detail: "must be positive".into(),
        });
    }
    if config.learning_rate <= 0.0 || config.learning_rate > 1.0 {
        return Err(TrainingError::InvalidConfig {
            field: "learning_rate".into(),
            detail: "must be in (0, 1]".into(),
        });
    }
    if config.epochs == 0 || config.epochs > 100 {
        return Err(TrainingError::InvalidConfig {
            field: "epochs".into(),
            detail: "must be in 1..=100".into(),
        });
    }
    if config.output_km_path.trim().is_empty() {
        return Err(TrainingError::InvalidConfig {
            field: "output_km_path".into(),
            detail: "must not be empty".into(),
        });
    }

    let ds = Path::new(&config.dataset_path);
    if !ds.exists() {
        // Allow missing dataset only for pure demo runs – warn via structured error
        // so the UI can decide to continue with synthetic data.
        return Err(TrainingError::DatasetNotFound {
            path: config.dataset_path.clone(),
        });
    }
    if ds.is_file() {
        let meta = std::fs::metadata(ds).map_err(|e| TrainingError::Internal {
            detail: e.to_string(),
        })?;
        if meta.len() == 0 {
            return Err(TrainingError::DatasetEmpty {
                path: config.dataset_path.clone(),
            });
        }
    }
    Ok(())
}

fn ensure_python() -> Result<(), TrainingError> {
    match std::process::Command::new(crate::paths::python_cmd())
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
    {
        Ok(s) if s.success() => Ok(()),
        _ => Err(TrainingError::PythonNotFound),
    }
}

pub async fn start_training_job(
    config: TrainingConfig,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    start_training_job_inner(config, app_handle)
        .await
        .map_err(Into::into)
}

async fn start_training_job_inner(
    config: TrainingConfig,
    app_handle: tauri::AppHandle,
) -> Result<String, TrainingError> {
    validate_config(&config)?;
    ensure_python()?;

    let script = resolve_python_script("train_lora.py")?;
    if !script.exists() {
        return Err(TrainingError::ScriptNotFound {
            path: script.display().to_string(),
        });
    }

    // Ensure output directory exists
    if let Some(parent) = Path::new(&config.output_km_path).parent() {
        std::fs::create_dir_all(parent).map_err(|e| TrainingError::Internal {
            detail: format!("create output dir: {e}"),
        })?;
    }

    let job_id = Uuid::new_v4().to_string();
    info!(
        job_id = %job_id,
        recipe = %config.recipe,
        base = %config.base_model,
        "Starting training job"
    );

    let _ = app_handle.emit_all(
        "training-start",
        serde_json::json!({
            "jobId": &job_id,
            "config": {
                "recipe": config.recipe,
                "base_model": config.base_model,
                "epochs": config.epochs,
                "rank": config.rank
            }
        }),
    );

    let sandbox = std::env::var("CONCIERGE_SANDBOX").ok();
    let mut cmd = Command::new(crate::paths::python_cmd());
    if let Some(ref sb) = sandbox {
        cmd.arg(sb);
    }
    cmd.arg(&script)
        .args([
            "--base_model",
            &config.base_model,
            "--dataset",
            &config.dataset_path,
            "--output_km",
            &config.output_km_path,
            "--recipe",
            &config.recipe,
            "--rank",
            &config.rank.to_string(),
            "--alpha",
            &config.alpha.to_string(),
            "--lr",
            &config.learning_rate.to_string(),
            "--epochs",
            &config.epochs.to_string(),
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|e| TrainingError::SpawnFailed {
        detail: e.to_string(),
    })?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();
    let jid = job_id.clone();
    let handle = app_handle.clone();

    if let Some(stdout) = stdout {
        let jid = jid.clone();
        let handle = handle.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let _ = handle.emit_all(
                    "training-progress",
                    serde_json::json!({ "jobId": &jid, "line": line }),
                );
            }
        });
    }

    // Collect stderr tail for final error reporting
    let stderr_buf = std::sync::Arc::new(tokio::sync::Mutex::new(String::new()));
    if let Some(stderr) = stderr {
        let jid = job_id.clone();
        let handle = app_handle.clone();
        let buf = stderr_buf.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                warn!(job = %jid, %line, "training stderr");
                {
                    let mut b = buf.lock().await;
                    if b.len() < 4000 {
                        b.push_str(&line);
                        b.push('\n');
                    }
                }
                let _ = handle.emit_all(
                    "training-error",
                    serde_json::json!({ "jobId": &jid, "error": line }),
                );
            }
        });
    }

    let jid_done = job_id.clone();
    let handle_done = app_handle.clone();
    let stderr_buf_done = stderr_buf.clone();
    tokio::spawn(async move {
        match child.wait().await {
            Ok(status) => {
                let success = status.success();
                if !success {
                    let tail = stderr_buf_done.lock().await.clone();
                    error!(
                        job = %jid_done,
                        code = ?status.code(),
                        "training process failed"
                    );
                    let _ = handle_done.emit_all(
                        "training-error",
                        serde_json::json!({
                            "jobId": &jid_done,
                            "error": format!(
                                "Process exited with {:?}: {}",
                                status.code(),
                                tail.chars().take(500).collect::<String>()
                            )
                        }),
                    );
                }
                let _ = handle_done.emit_all(
                    "training-complete",
                    serde_json::json!({
                        "jobId": jid_done,
                        "success": success,
                        "exitCode": status.code()
                    }),
                );
            }
            Err(e) => {
                error!(job = %jid_done, error = %e, "wait failed");
                let _ = handle_done.emit_all(
                    "training-error",
                    serde_json::json!({ "jobId": &jid_done, "error": e.to_string() }),
                );
                let _ = handle_done.emit_all(
                    "training-complete",
                    serde_json::json!({ "jobId": jid_done, "success": false }),
                );
            }
        }
    });

    Ok(job_id)
}

/// Soft-start: if dataset is missing, create a tiny synthetic JSONL so demo
/// training can still run. Returns the (possibly rewritten) config.
pub async fn start_training_job_lenient(
    mut config: TrainingConfig,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    if let Err(TrainingError::DatasetNotFound { path }) = validate_config(&config) {
        warn!(%path, "Dataset missing – writing synthetic demo dataset");
        if let Some(parent) = Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let demo = concat!(
            r#"{"text":"OmniForge trains adapters on local data."}"#,
            "\n",
            r#"{"text":"Knowledge modules package LoRA weights for reuse."}"#,
            "\n",
            r#"{"text":"Streaming inference updates the UI token by token."}"#,
            "\n"
        );
        std::fs::write(&path, demo).map_err(|e| e.to_string())?;
        let _ = app_handle.emit_all(
            "training-progress",
            serde_json::json!({
                "jobId": "preflight",
                "line": format!("Created synthetic dataset at {path}")
            }),
        );
    }
    // Re-validate other fields (dataset now exists)
    match validate_config(&config) {
        Ok(()) => {}
        Err(TrainingError::DatasetNotFound { .. }) => {}
        Err(e) => return Err(e.into()),
    }
    start_training_job(config, app_handle).await
}

fn resolve_python_script(name: &str) -> Result<PathBuf, TrainingError> {
    let candidates = [
        PathBuf::from("python").join(name),
        PathBuf::from("../python").join(name),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("python").join(name)))
            .unwrap_or_default(),
    ];
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Ok(PathBuf::from("python").join(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_config(ds: &str) -> TrainingConfig {
        // Ensure dataset exists for positive path
        if !Path::new(ds).exists() {
            if let Some(parent) = Path::new(ds).parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(ds, "{\"text\":\"hello\"}\n");
        }
        TrainingConfig {
            base_model: "gpt2".into(),
            dataset_path: ds.into(),
            output_km_path: "/tmp/omniforge-test-km".into(),
            recipe: "lora".into(),
            rank: 8,
            alpha: 16.0,
            learning_rate: 2e-4,
            epochs: 1,
        }
    }

    #[test]
    fn rejects_empty_base_model() {
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.base_model = "  ".into();
        let err = validate_config(&c).unwrap_err();
        assert!(matches!(err, TrainingError::InvalidConfig { field, .. } if field == "base_model"));
    }

    #[test]
    fn rejects_bad_recipe() {
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.recipe = "full-ft".into();
        let err = validate_config(&c).unwrap_err();
        assert!(matches!(err, TrainingError::InvalidConfig { field, .. } if field == "recipe"));
    }

    #[test]
    fn rejects_zero_rank() {
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.rank = 0;
        assert!(matches!(
            validate_config(&c),
            Err(TrainingError::InvalidConfig { field, .. }) if field == "rank"
        ));
    }

    #[test]
    fn rejects_bad_lr() {
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.learning_rate = 0.0;
        assert!(matches!(
            validate_config(&c),
            Err(TrainingError::InvalidConfig { field, .. }) if field == "learning_rate"
        ));
    }

    #[test]
    fn rejects_missing_dataset() {
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.dataset_path = "/tmp/definitely-missing-omniforge-ds-xyz.jsonl".into();
        let err = validate_config(&c).unwrap_err();
        assert!(matches!(err, TrainingError::DatasetNotFound { .. }));
    }

    #[test]
    fn rejects_empty_dataset_file() {
        let path = "/tmp/omniforge-empty-ds.jsonl";
        let _ = std::fs::write(path, "");
        let mut c = valid_config("/tmp/omniforge-ds.jsonl");
        c.dataset_path = path.into();
        let err = validate_config(&c).unwrap_err();
        assert!(matches!(err, TrainingError::DatasetEmpty { .. }));
    }

    #[test]
    fn accepts_valid_config() {
        let c = valid_config("/tmp/omniforge-ds.jsonl");
        assert!(validate_config(&c).is_ok());
    }

    #[test]
    fn error_display_is_informative() {
        let e = TrainingError::PythonNotFound;
        assert!(e.to_string().contains("python3"));
        let e = TrainingError::SpawnFailed {
            detail: "permission denied".into(),
        };
        assert!(e.to_string().contains("permission denied"));
        let e = TrainingError::ProcessFailed {
            job_id: "abc".into(),
            exit_code: Some(1),
            stderr_tail: "CUDA OOM".into(),
        };
        let s = e.to_string();
        assert!(s.contains("abc"));
        assert!(s.contains("CUDA OOM"));
    }
}
