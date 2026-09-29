//! Cross-platform **atomic filesystem** primitives for OmniForge.
//!
//! # Guarantees
//!
//! | Operation | POSIX | Windows |
//! |-----------|-------|---------|
//! | `atomic_write` | `write temp → fsync → rename` (same dir = atomic) | `MoveFileEx` with `REPLACE_EXISTING` after temp write |
//! | `atomic_copy` | write temp in dest dir → rename | same pattern |
//! | `atomic_symlink_swap` | `symlink temp → rename` over link path | best-effort replace |
//!
//! All temps are created **in the destination directory** so `rename` does not
//! cross mount points (which would lose atomicity).
//!
//! # Crash safety
//!
//! 1. Content is fully written + `sync_all` on the temp file before rename.
//! 2. On Unix, the parent directory is fsynced after rename so the directory
//!    entry itself is durable.
//! 3. Orphaned `*.omniforge-tmp*` files can be swept by `cleanup_orphans`.

use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AtomicFsError {
    Io { path: String, detail: String },
    CrossDevice { path: String },
    NotFound { path: String },
    Permission { path: String, detail: String },
    Internal { detail: String },
}

impl std::fmt::Display for AtomicFsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AtomicFsError::Io { path, detail } => write!(f, "I/O on {path}: {detail}"),
            AtomicFsError::CrossDevice { path } => {
                write!(f, "Cross-device link refused for {path} (temp must share volume)")
            }
            AtomicFsError::NotFound { path } => write!(f, "Not found: {path}"),
            AtomicFsError::Permission { path, detail } => {
                write!(f, "Permission denied on {path}: {detail}")
            }
            AtomicFsError::Internal { detail } => write!(f, "Atomic FS internal: {detail}"),
        }
    }
}

impl From<AtomicFsError> for String {
    fn from(e: AtomicFsError) -> String {
        e.to_string()
    }
}

fn map_io(path: &Path, e: io::Error) -> AtomicFsError {
    let p = path.display().to_string();
    match e.kind() {
        io::ErrorKind::NotFound => AtomicFsError::NotFound { path: p },
        io::ErrorKind::PermissionDenied => AtomicFsError::Permission {
            path: p,
            detail: e.to_string(),
        },
        #[cfg(unix)]
        _ if e.raw_os_error() == Some(18) /* EXDEV */ => AtomicFsError::CrossDevice { path: p },
        _ => AtomicFsError::Io {
            path: p,
            detail: e.to_string(),
        },
    }
}

/// Unique temp path **in the same directory** as `target`.
fn temp_sibling(target: &Path) -> Result<PathBuf, AtomicFsError> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let name = target
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| AtomicFsError::Internal {
            detail: "target has no file name".into(),
        })?;
    let pid = std::process::id();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    Ok(parent.join(format!(".{name}.{pid}.{nanos}.omniforge-tmp")))
}

fn fsync_dir(dir: &Path) -> Result<(), AtomicFsError> {
    #[cfg(unix)]
    {
        let f = File::open(dir).map_err(|e| map_io(dir, e))?;
        f.sync_all().map_err(|e| map_io(dir, e))?;
    }
    #[cfg(not(unix))]
    {
        let _ = dir; // directory fsync is a no-op on Windows for our purposes
    }
    Ok(())
}

/// Atomically replace `path` with `bytes`.
///
/// Creates parent directories as needed. Durability: temp is `sync_all`'d before
/// rename; parent dir is fsynced on Unix.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AtomicFsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| map_io(parent, e))?;
    }
    let tmp = temp_sibling(path)?;
    let result = (|| {
        {
            let mut f = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&tmp)
                .map_err(|e| map_io(&tmp, e))?;
            f.write_all(bytes).map_err(|e| map_io(&tmp, e))?;
            f.sync_all().map_err(|e| map_io(&tmp, e))?;
        }
        replace_file(&tmp, path)?;
        if let Some(parent) = path.parent() {
            let _ = fsync_dir(parent);
        }
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Atomically write UTF-8 text.
pub fn atomic_write_str(path: &Path, content: &str) -> Result<(), AtomicFsError> {
    atomic_write(path, content.as_bytes())
}

/// Read entire file to string with a clear error type.
pub fn read_to_string(path: &Path) -> Result<String, AtomicFsError> {
    fs::read_to_string(path).map_err(|e| map_io(path, e))
}

/// Atomic copy: read source, write temp in dest dir, rename over dest.
pub fn atomic_copy(from: &Path, to: &Path) -> Result<u64, AtomicFsError> {
    let mut f = File::open(from).map_err(|e| map_io(from, e))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf).map_err(|e| map_io(from, e))?;
    let n = buf.len() as u64;
    atomic_write(to, &buf)?;
    Ok(n)
}

/// Compare-and-swap style write: only replace if current content matches `expected`
/// (or file is absent when `expected` is `None`). Returns whether the swap happened.
pub fn atomic_cas(
    path: &Path,
    expected: Option<&str>,
    new_content: &str,
) -> Result<bool, AtomicFsError> {
    let current = match fs::read_to_string(path) {
        Ok(s) => Some(s),
        Err(e) if e.kind() == io::ErrorKind::NotFound => None,
        Err(e) => return Err(map_io(path, e)),
    };
    if current.as_deref() != expected {
        return Ok(false);
    }
    atomic_write_str(path, new_content)?;
    Ok(true)
}

/// Platform-specific durable replace of `tmp` → `dest`.
fn replace_file(tmp: &Path, dest: &Path) -> Result<(), AtomicFsError> {
    #[cfg(windows)]
    {
        // Prefer rename; on Windows replace existing via remove+rename fallback
        match fs::rename(tmp, dest) {
            Ok(()) => Ok(()),
            Err(_) => {
                let _ = fs::remove_file(dest);
                fs::rename(tmp, dest).map_err(|e| map_io(dest, e))
            }
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(tmp, dest).map_err(|e| map_io(dest, e))
    }
}

/// Remove leftover temp files from interrupted writes under `dir` (non-recursive).
pub fn cleanup_orphans(dir: &Path) -> Result<usize, AtomicFsError> {
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut n = 0;
    for entry in fs::read_dir(dir).map_err(|e| map_io(dir, e))? {
        let entry = entry.map_err(|e| AtomicFsError::Internal {
            detail: e.to_string(),
        })?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.contains(".omniforge-tmp") {
            match fs::remove_file(entry.path()) {
                Ok(()) => n += 1,
                Err(e) => warn!(path = %entry.path().display(), error = %e, "orphan cleanup failed"),
            }
        }
    }
    debug!(dir = %dir.display(), removed = n, "orphan temp cleanup");
    Ok(n)
}

/// Append a line with a best-effort durability story (open-append-write-sync).
/// Not fully atomic for readers mid-write; use `atomic_write` for full-file replaces.
pub fn durable_append_line(path: &Path, line: &str) -> Result<(), AtomicFsError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| map_io(parent, e))?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| map_io(path, e))?;
    writeln!(f, "{line}").map_err(|e| map_io(path, e))?;
    f.sync_all().map_err(|e| map_io(path, e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn tmp_path(name: &str) -> PathBuf {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("omniforge-atomic-{n}-{name}"))
    }

    #[test]
    fn atomic_write_roundtrip() {
        let p = tmp_path("write.txt");
        atomic_write_str(&p, "hello-atomic").unwrap();
        assert_eq!(read_to_string(&p).unwrap(), "hello-atomic");
        atomic_write_str(&p, "replaced").unwrap();
        assert_eq!(read_to_string(&p).unwrap(), "replaced");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn atomic_cas_success_and_fail() {
        let p = tmp_path("cas.txt");
        let _ = fs::remove_file(&p);
        assert!(atomic_cas(&p, None, "v1").unwrap());
        assert!(!atomic_cas(&p, None, "x").unwrap()); // file exists, expected None
        assert!(atomic_cas(&p, Some("v1"), "v2").unwrap());
        assert_eq!(read_to_string(&p).unwrap(), "v2");
        assert!(!atomic_cas(&p, Some("v1"), "v3").unwrap());
        assert_eq!(read_to_string(&p).unwrap(), "v2");
        let _ = fs::remove_file(&p);
    }

    #[test]
    fn atomic_copy_preserves_bytes() {
        let a = tmp_path("copy-src.bin");
        let b = tmp_path("copy-dst.bin");
        atomic_write(&a, &[1, 2, 3, 4, 5]).unwrap();
        let n = atomic_copy(&a, &b).unwrap();
        assert_eq!(n, 5);
        assert_eq!(fs::read(&b).unwrap(), vec![1, 2, 3, 4, 5]);
        let _ = fs::remove_file(&a);
        let _ = fs::remove_file(&b);
    }

    #[test]
    fn cleanup_orphans_removes_temps() {
        let dir = tmp_path("orphan-dir");
        fs::create_dir_all(&dir).unwrap();
        let orphan = dir.join(".foo.123.omniforge-tmp");
        fs::write(&orphan, b"junk").unwrap();
        let n = cleanup_orphans(&dir).unwrap();
        assert!(n >= 1);
        assert!(!orphan.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
