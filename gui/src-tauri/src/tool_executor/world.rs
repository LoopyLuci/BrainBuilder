use super::super::tool_executor::executor::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::process::Command;
use reqwest::Client;
use std::time::Duration;
use rusqlite::Connection;

#[derive(Clone)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(root: P) -> Self {
        let root = root.as_ref().to_path_buf();
        let _ = std::fs::create_dir_all(&root);
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolve a user-provided path against the workspace, preventing escapes.
    pub fn resolve(&self, path: &str) -> Result<PathBuf, String> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.root.join(path)
        };
        let canonical = std::fs::canonicalize(&candidate)
            .or_else(|_| std::fs::canonicalize(self.root.join(&candidate)))
            .map_err(|e| format!("invalid path: {}", e))?;
        let root_canonical = std::fs::canonicalize(&self.root)
            .map_err(|e| format!("workspace unavailable: {}", e))?;
        if !canonical.starts_with(&root_canonical) {
            return Err(format!("path escapes workspace: {}", path));
        }
        Ok(canonical)
    }

    pub fn create_file(&self, path: &str, content: &str) -> Result<PathBuf, String> {
        let resolved = self.resolve(path)?;
        if let Some(parent) = resolved.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&resolved, content).map_err(|e| e.to_string())?;
        Ok(resolved)
    }

    pub fn read_file(&self, path: &str) -> Result<String, String> {
        let resolved = self.resolve(path)?;
        std::fs::read_to_string(&resolved).map_err(|e| e.to_string())
    }

    pub fn write_file(&self, path: &str, content: &str) -> Result<PathBuf, String> {
        self.create_file(path, content)
    }

    pub fn delete_file(&self, path: &str) -> Result<(), String> {
        let resolved = self.resolve(path)?;
        std::fs::remove_file(&resolved).map_err(|e| e.to_string())
    }

    pub fn list_dir(&self, path: &str) -> Result<Vec<String>, String> {
        let resolved = self.resolve(path)?;
        let mut entries = Vec::new();
        for entry in std::fs::read_dir(&resolved).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            entries.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(entries)
    }
}

#[derive(Clone)]
pub struct JailedShell {
    workspace: Arc<Workspace>,
    allowed_commands: Vec<String>,
}

impl JailedShell {
    pub fn new(workspace: Arc<Workspace>, allowed_commands: Vec<String>) -> Self {
        Self { workspace, allowed_commands }
    }

    async fn exec(&self, cmd: &str, args: &[&str]) -> Result<(String, String, i32), String> {
        let output = Command::new(cmd)
            .args(args)
            .current_dir(self.workspace.root())
            .output()
            .await
            .map_err(|e| format!("shell execution failed: {}", e))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let status = output.status.code().unwrap_or(-1);
        Ok((stdout, stderr, status))
    }

    pub async fn execute(&self, cmd: &str) -> Result<(String, String, i32), String> {
        if !self.allowed_commands.contains(&cmd.to_string()) {
            return Err(format!("command not allowed: {}", cmd));
        }
        self.exec(cmd, &[]).await
    }
}

pub struct FsReadTool;
#[async_trait]
impl Tool for FsReadTool {
    fn name(&self) -> &'static str { "fs.read" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
        let workspace = Workspace::new("./workspace");
        let content = workspace.read_file(path)?;
        Ok(Value::String(content))
    }
}

pub struct FsWriteTool;
#[async_trait]
impl Tool for FsWriteTool {
    fn name(&self) -> &'static str { "fs.write" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
        let content = args.get("content").and_then(|v| v.as_str()).ok_or("content required")?;
        let workspace = Workspace::new("./workspace");
        let resolved = workspace.write_file(path, content)?;
        Ok(serde_json::json!({"path": resolved.to_string_lossy(), "bytes": content.len()}))
    }
}

pub struct FsDeleteTool;
#[async_trait]
impl Tool for FsDeleteTool {
    fn name(&self) -> &'static str { "fs.delete" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
        let workspace = Workspace::new("./workspace");
        workspace.delete_file(path)?;
        Ok(serde_json::json!({"deleted": path}))
    }
}

pub struct FsListTool;
#[async_trait]
impl Tool for FsListTool {
    fn name(&self) -> &'static str { "fs.list" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
        let workspace = Workspace::new("./workspace");
        let entries = workspace.list_dir(path)?;
        Ok(serde_json::json!(entries))
    }
}

pub struct HttpFetchTool;
#[async_trait]
impl Tool for HttpFetchTool {
    fn name(&self) -> &'static str { "http.fetch" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let url = args.get("url").and_then(|v| v.as_str()).ok_or("url required")?;
        let client = Client::builder()
            .timeout(Duration::from_secs(15))
            .user_agent("Luci/1.0")
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client.get(url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status().as_u16();
        let body = resp.text().await.map_err(|e| e.to_string())?;
        if body.len() > 2 * 1024 * 1024 {
            return Err("response body too large".into());
        }
        Ok(serde_json::json!({"url": url, "status": status, "body": body}))
    }
}

pub struct ShellExecTool;
#[async_trait]
impl Tool for ShellExecTool {
    fn name(&self) -> &'static str { "shell.exec" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let cmd = args.get("cmd").and_then(|v| v.as_str()).ok_or("cmd required")?;
        let workspace = Arc::new(Workspace::new("./workspace"));
        let shell = JailedShell::new(workspace, vec![
            "ls".into(), "cat".into(), "echo".into(), "pwd".into(),
            "date".into(), "whoami".into(), "python3".into(), "node".into(),
            "cargo".into(), "git".into(),
        ]);
        let (stdout, stderr, status) = shell.execute(cmd).await?;
        Ok(serde_json::json!({"stdout": stdout, "stderr": stderr, "status": status}))
    }
}

pub struct SqliteQueryTool;
#[async_trait]
impl Tool for SqliteQueryTool {
    fn name(&self) -> &'static str { "sqlite.query" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let path = args.get("path").and_then(|v| v.as_str()).ok_or("path required")?;
        let sql = args.get("sql").and_then(|v| v.as_str()).ok_or("sql required")?;
        let workspace = Workspace::new("./workspace");
        let resolved = workspace.resolve(path)?;
        let conn = Connection::open(&resolved).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let column_names: Vec<String> = stmt.column_names().into_iter().map(|s| s.to_string()).collect();
        let mut rows: Vec<serde_json::Map<String, Value>> = Vec::new();
        let iter = stmt.query_map([], |row| {
            let mut map = serde_json::Map::new();
            for (idx, name) in column_names.iter().enumerate() {
                let value: Value = match row.get_ref(idx) {
                    Ok(rusqlite::types::ValueRef::Null) => Value::Null,
                    Ok(rusqlite::types::ValueRef::Integer(i)) => Value::Number(i.into()),
                    Ok(rusqlite::types::ValueRef::Real(f)) => Value::Number(serde_json::Number::from_f64(f).unwrap_or_else(|| 0.into())),
                    Ok(rusqlite::types::ValueRef::Text(s)) => Value::String(String::from_utf8_lossy(s).to_string()),
                    Ok(rusqlite::types::ValueRef::Blob(b)) => Value::String(format!("<blob {} bytes>", b.len())),
                    Err(_) => Value::Null,
                };
                map.insert(name.clone(), value);
            }
            Ok(map)
        }).map_err(|e| e.to_string())?;
        for r in iter {
            match r {
                Ok(map) => rows.push(map),
                Err(e) => return Err(e.to_string()),
            }
        }
        Ok(serde_json::json!({"rows": rows}))
    }
}
