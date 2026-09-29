//! Built-in **omniforge-fs** plugin — atomic FS surface exposed as a coherent unit.
//!
//! Command names follow Tauri plugin conventions: `plugin:omniforge-fs|atomic_write`,
//! but are registered as flat invoke handlers until crate-split.

use super::registry::{OmniPlugin, PluginContext, PluginMeta};
use crate::atomic_fs;
use std::path::PathBuf;

pub struct FsPlugin;

impl OmniPlugin for FsPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            name: "omniforge-fs".into(),
            version: "0.1.0".into(),
            description: "Atomic filesystem operations (write, copy, CAS, orphan cleanup)"
                .into(),
            commands: vec![
                "fs_atomic_write".into(),
                "fs_atomic_copy".into(),
                "fs_cas".into(),
                "fs_cleanup_orphans".into(),
            ],
        }
    }

    fn setup(&self, _ctx: &PluginContext) -> Result<(), String> {
        // Could create app-data directories here
        Ok(())
    }
}

impl FsPlugin {
    pub fn atomic_write(path: String, content: String) -> Result<(), String> {
        atomic_fs::atomic_write_str(PathBuf::from(path).as_path(), &content).map_err(Into::into)
    }

    pub fn atomic_copy(from: String, to: String) -> Result<u64, String> {
        atomic_fs::atomic_copy(PathBuf::from(from).as_path(), PathBuf::from(to).as_path())
            .map_err(Into::into)
    }

    pub fn cas(path: String, expected: Option<String>, content: String) -> Result<bool, String> {
        atomic_fs::atomic_cas(
            PathBuf::from(path).as_path(),
            expected.as_deref(),
            &content,
        )
        .map_err(Into::into)
    }

    pub fn cleanup_orphans(dir: String) -> Result<usize, String> {
        atomic_fs::cleanup_orphans(PathBuf::from(dir).as_path()).map_err(Into::into)
    }
}
