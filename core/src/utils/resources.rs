//! Where the bundled `components/` folder lives at run time.
//!
//! Development builds find it next to the crate (`CARGO_MANIFEST_DIR/../components`),
//! but that is a compile-time path: a packaged build copied to another machine
//! or launched from another working directory cannot rely on it. The lookup
//! order here is:
//!
//! 1. `BRAINBUILDER_COMPONENTS_DIR`, if set (must exist);
//! 2. `components/` next to the running executable, then one and two levels up
//!    (Tauri bundles resources beside the binary; `target/<profile>/` builds sit
//!    two levels below the workspace);
//! 3. the compile-time development path.
//!
//! Each candidate must contain `python/_bb_worker.py` to count, so an unrelated
//! folder called `components` is never picked.
use std::path::{Path, PathBuf};

pub const COMPONENTS_ENV: &str = "BRAINBUILDER_COMPONENTS_DIR";
const MARKER: &str = "python/_bb_worker.py";

fn is_components_dir(p: &Path) -> bool {
    p.join(MARKER).is_file()
}

/// Every place `components_dir` looks, in order.
pub fn candidate_components_dirs() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = std::env::current_exe() {
        if let Some(exe_dir) = dir.parent() {
            for up in [exe_dir.to_path_buf(), exe_dir.join(".."), exe_dir.join("../..")] {
                out.push(up.join("components"));
            }
        }
    }
    out.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("../components"));
    out
}

/// The components folder, or a message naming every place that was tried.
pub fn components_dir() -> Result<PathBuf, String> {
    resolve_components_dir(std::env::var_os(COMPONENTS_ENV).map(PathBuf::from), &candidate_components_dirs())
}

/// The lookup itself, with its inputs injected so it can be tested.
pub fn resolve_components_dir(env_override: Option<PathBuf>, candidates: &[PathBuf]) -> Result<PathBuf, String> {
    if let Some(dir) = env_override {
        return if is_components_dir(&dir) {
            Ok(dir)
        } else {
            Err(format!("{COMPONENTS_ENV}={} does not contain {MARKER}", dir.display()))
        };
    }
    for c in candidates {
        if is_components_dir(c) {
            return Ok(c.clone());
        }
    }
    let tried: Vec<String> = candidates.iter().map(|c| c.display().to_string()).collect();
    Err(format!(
        "components folder not found (looked for {MARKER} in: {}); set {COMPONENTS_ENV}",
        tried.join(", ")
    ))
}

/// Path of the long-lived Python worker script.
pub fn python_worker_script() -> Result<PathBuf, String> {
    components_dir().map(|d| d.join(MARKER))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_components(root: &Path) -> PathBuf {
        let dir = root.join("components");
        std::fs::create_dir_all(dir.join("python")).unwrap();
        std::fs::write(dir.join(MARKER), "# worker\n").unwrap();
        dir
    }

    #[test]
    fn development_layout_is_found() {
        let dir = components_dir().expect("the repository's components folder");
        assert!(dir.join(MARKER).is_file());
    }

    #[test]
    fn override_wins_and_must_be_valid() {
        let tmp = std::env::temp_dir().join(format!("bb-res-{}", std::process::id()));
        let good = fake_components(&tmp);
        assert_eq!(resolve_components_dir(Some(good.clone()), &[]), Ok(good));
        let bad = tmp.join("nope");
        assert!(resolve_components_dir(Some(bad), &[]).unwrap_err().contains(COMPONENTS_ENV));
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn first_valid_candidate_is_used_and_failures_name_every_path() {
        let tmp = std::env::temp_dir().join(format!("bb-res2-{}", std::process::id()));
        let empty = tmp.join("empty/components");
        std::fs::create_dir_all(&empty).unwrap(); // a folder called components without the worker
        let real = fake_components(&tmp.join("bundle"));
        assert_eq!(resolve_components_dir(None, &[empty.clone(), real.clone()]), Ok(real));
        let err = resolve_components_dir(None, &[empty.clone()]).unwrap_err();
        assert!(err.contains(&empty.display().to_string()));
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
