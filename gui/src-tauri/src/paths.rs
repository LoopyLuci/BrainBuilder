//! Cross-platform application paths (Windows 10 / Linux / macOS).

use std::path::PathBuf;

/// Resolve the OmniForge application data directory.
///
/// | OS | Location |
/// |----|----------|
/// | Windows | `%APPDATA%\OmniForge` |
/// | Linux | `~/.local/share/omniforge` |
/// | macOS | `~/Library/Application Support/OmniForge` |
///
/// Override with `OMNIFORGE_DATA_DIR`.
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("OMNIFORGE_DATA_DIR") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "windows")]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return PathBuf::from(appdata).join("OmniForge");
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(home) = home_dir() {
            return home.join("Library/Application Support/OmniForge");
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(home) = home_dir() {
            return home.join(".local/share/omniforge");
        }
    }
    // Fallback: cwd/data
    PathBuf::from("data")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// Ensure data dir exists and return it.
pub fn ensure_data_dir() -> Result<PathBuf, String> {
    let d = data_dir();
    if std::fs::create_dir_all(&d).is_err() {
        let fallback = PathBuf::from("data");
        std::fs::create_dir_all(&fallback).map_err(|e| format!("Create fallback data dir {}: {e}", fallback.display()))?;
        return Ok(fallback);
    }
    std::fs::create_dir_all(d.join("models")).ok();
    std::fs::create_dir_all(d.join("kms")).ok();
    std::fs::create_dir_all(d.join("datasets")).ok();
    std::fs::create_dir_all(d.join("output")).ok();
    Ok(d)
}

/// Filesystem path for a SQLite file under the data dir.
/// This is the correct argument for `rusqlite::Connection::open()`.
pub fn sqlite_path(filename: &str) -> Result<PathBuf, String> {
    let dir = ensure_data_dir()?;
    Ok(dir.join(filename))
}

/// Project root for self-edit allow-list (dev mode).
pub fn project_root() -> PathBuf {
    if let Ok(p) = std::env::var("OMNIFORGE_ROOT") {
        return PathBuf::from(p);
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Prefer `python` on Windows, `python3` elsewhere; honor `OMNIFORGE_PYTHON`.
pub fn python_cmd() -> String {
    if let Ok(p) = std::env::var("OMNIFORGE_PYTHON") {
        return p;
    }
    #[cfg(target_os = "windows")]
    {
        // Try py launcher first via env; default to python
        "python".into()
    }
    #[cfg(not(target_os = "windows"))]
    {
        "python3".into()
    }
}
