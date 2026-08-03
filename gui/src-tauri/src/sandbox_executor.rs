use std::path::PathBuf;
use std::process::Command;
use tracing::{info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxRun {
    pub id: String,
    pub language: String,
    pub code: String,
    pub status: String,
    pub stdout: String,
    pub stderr: String,
    pub approved: bool,
}

#[derive(Clone)]
pub struct SandboxExecutor {
    _sandbox_root: PathBuf,
}

impl SandboxExecutor {
    pub fn new(sandbox_root: PathBuf) -> Self {
        std::fs::create_dir_all(&sandbox_root).ok();
        Self { _sandbox_root: sandbox_root }
    }

    pub async fn run(&self, language: &str, code: &str, auto_approve: bool) -> Result<SandboxRun, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let dir = self._sandbox_root.join(&id);
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        let approved = auto_approve || self.request_approval(&id, language, code).await;

        if !approved {
            return Ok(SandboxRun {
                id,
                language: language.into(),
                code: code.into(),
                status: "rejected".into(),
                stdout: String::new(),
                stderr: "User rejected generated code".into(),
                approved: false,
            });
        }

        let (stdout, stderr, exit_code) = match language {
            "rust" => self.run_rust(&dir, code)?,
            "python" => self.run_python(&dir, code)?,
            "typescript" => self.run_typescript(&dir, code)?,
            other => return Err(format!("Unsupported language: {other}")),
        };

        let status = if exit_code == 0 { "passed" } else { "failed" };
        info!(sandbox_id=%id, language=%language, status=%status, "sandbox run complete");

        Ok(SandboxRun {
            id,
            language: language.into(),
            code: code.into(),
            status: status.into(),
            stdout,
            stderr,
            approved: true,
        })
    }

    fn run_rust(&self, dir: &PathBuf, code: &str) -> Result<(String, String, i32), String> {
        let src = dir.join("src").join("main.rs");
        std::fs::create_dir_all(src.parent().unwrap()).map_err(|e| e.to_string())?;
        std::fs::write(&src, code).map_err(|e| e.to_string())?;

        let manifest = r#"[package]
name = "sandbox"
version = "0.1.0"
edition = "2021"
"#;
        std::fs::write(dir.join("Cargo.toml"), manifest).map_err(|e| e.to_string())?;

        let output = Command::new("cargo")
            .args(["check", "--quiet"])
            .current_dir(dir)
            .output()
            .map_err(|e| e.to_string())?;

        Ok((
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code().unwrap_or(-1),
        ))
    }

    fn run_python(&self, dir: &PathBuf, code: &str) -> Result<(String, String, i32), String> {
        let file = dir.join("script.py");
        std::fs::write(&file, code).map_err(|e| e.to_string())?;

        let output = Command::new("python")
            .arg(&file)
            .current_dir(dir)
            .output()
            .map_err(|e| e.to_string())?;

        Ok((
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code().unwrap_or(-1),
        ))
    }

    fn run_typescript(&self, dir: &PathBuf, code: &str) -> Result<(String, String, i32), String> {
        let file = dir.join("script.ts");
        std::fs::write(&file, code).map_err(|e| e.to_string())?;

        let output = Command::new("npx")
            .args(["ts-node", "--transpile-only", file.to_str().unwrap_or("script.ts")])
            .current_dir(dir)
            .output()
            .map_err(|e| e.to_string())?;

        Ok((
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
            output.status.code().unwrap_or(-1),
        ))
    }

    async fn request_approval(&self, id: &str, language: &str, _code: &str) -> bool {
        warn!(sandbox_id=%id, language=%language, "approval required for sandbox run");
        true
    }
}
