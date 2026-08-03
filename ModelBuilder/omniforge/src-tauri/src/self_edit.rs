//! Allow-listed source edits with atomic durability + hot-reload signaling.
//!
//! Builds on [`crate::atomic_fs`] for crash-safe writes.

use crate::atomic_fs::{self, AtomicFsError};
use serde::Serialize;
use std::fs;
use std::path::{Component, Path, PathBuf};
use tauri::Manager;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
}

fn project_root() -> PathBuf {
    crate::paths::project_root()
}

fn allowed_prefixes() -> [&'static str; 4] {
    ["src/", "src-tauri/src/", "python/", "plugins/"]
}

fn is_allowed(rel: &str) -> bool {
    let rel = rel.replace('\\', "/");
    allowed_prefixes().iter().any(|p| rel.starts_with(p)) && !rel.contains("..")
}

fn resolve_safe(rel: &str) -> Result<PathBuf, String> {
    let rel = rel.replace('\\', "/");
    if !is_allowed(&rel) {
        return Err(format!(
            "Path not in allow-list (src/, src-tauri/src/, python/, plugins/): {rel}"
        ));
    }
    let path = Path::new(&rel);
    if path
        .components()
        .any(|c| matches!(c, Component::ParentDir | Component::RootDir))
    {
        return Err("Invalid path components".into());
    }
    let full = project_root().join(&rel);
    let root = project_root()
        .canonicalize()
        .unwrap_or_else(|_| project_root());
    match full.canonicalize() {
        Ok(c) if c.starts_with(&root) => Ok(c),
        Ok(_) => Err("Path escapes project root".into()),
        Err(_) => {
            if let Some(parent) = full.parent() {
                if let Ok(pc) = parent.canonicalize() {
                    if !pc.starts_with(&root) {
                        return Err("Parent escapes project root".into());
                    }
                }
            }
            Ok(full)
        }
    }
}

pub fn list_editable_sources() -> Result<Vec<FileEntry>, String> {
    let root = project_root();
    let mut out = Vec::new();
    for prefix in allowed_prefixes() {
        let dir = root.join(prefix);
        if dir.exists() {
            walk(&dir, &root, &mut out)?;
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(dir: &Path, root: &Path, out: &mut Vec<FileEntry>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if matches!(name, "node_modules" | "target" | "dist" | ".git") {
                continue;
            }
            walk(&path, root, out)?;
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            if matches!(
                ext.as_str(),
                "tsx" | "ts" | "jsx" | "js" | "css" | "rs" | "py" | "json" | "toml" | "md"
            ) {
                let rel = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(FileEntry { path: rel, size });
            }
        }
    }
    Ok(())
}

pub fn read_source_file(rel: &str) -> Result<String, String> {
    let path = resolve_safe(rel)?;
    atomic_fs::read_to_string(&path).map_err(Into::into)
}

pub fn atomic_write_source(
    rel: &str,
    content: &str,
    app: Option<&tauri::AppHandle>,
    trigger_reload: bool,
) -> Result<(), String> {
    let path = resolve_safe(rel)?;
    atomic_fs::atomic_write_str(&path, content).map_err(|e: AtomicFsError| e.to_string())?;

    // Sweep orphans in the same directory
    if let Some(parent) = path.parent() {
        let _ = atomic_fs::cleanup_orphans(parent);
    }

    info!(path = %rel, bytes = content.len(), "Atomic source write complete");

    if trigger_reload {
        if let Some(app) = app {
            let _ = app.emit_all(
                "source-reloaded",
                serde_json::json!({ "path": rel, "bytes": content.len() }),
            );
        }
        let sentinel = project_root().join("src/.omniforge-hmr");
        if let Err(e) = atomic_fs::atomic_write_str(
            &sentinel,
            &format!("{}\n{}", rel, chrono::Utc::now().to_rfc3339()),
        ) {
            warn!(error = %e, "HMR sentinel write failed");
        }
    }
    Ok(())
}

/// CAS write for collaborative / concurrent editors.
pub fn cas_write_source(
    rel: &str,
    expected: Option<&str>,
    content: &str,
    app: Option<&tauri::AppHandle>,
) -> Result<bool, String> {
    let path = resolve_safe(rel)?;
    let ok = atomic_fs::atomic_cas(&path, expected, content).map_err(|e| e.to_string())?;
    if ok {
        if let Some(app) = app {
            let _ = app.emit_all(
                "source-reloaded",
                serde_json::json!({ "path": rel, "cas": true }),
            );
        }
    }
    Ok(ok)
}
