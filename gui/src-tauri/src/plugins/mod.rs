//! OmniForge **Tauri plugin architecture**.
//!
//! # Model
//!
//! External capabilities register as [`OmniPlugin`] implementations. Each plugin:
//! - Declares a stable `name` + `version`
//! - Exposes invoke handlers via [`OmniPlugin::commands`]
//! - Optionally runs setup on app start
//! - Can subscribe to lifecycle hooks (ready, exit)
//!
//! Built-in plugins ship in-tree; community Python plugins remain under
//! `crate::plugin_system` (filesystem manifests). This module is the **Rust/Tauri**
//! side — compile-time safe, shareable as crates later (`tauri-plugin-omniforge-*`).
//!
//! # Anatomy of a Tauri plugin (v1/v2 aligned)
//!
//! ```ignore
//! pub fn init<R: Runtime>() -> TauriPlugin<R> {
//!     PluginBuilder::new("omniforge-fs")
//!         .invoke_handler(tauri::generate_handler![...])
//!         .setup(|app, _api| { /* state */ Ok(()) })
//!         .build()
//! }
//! ```
//!
//! Until we split crates, [`PluginRegistry`] mirrors that pattern in-process.

pub mod fs_plugin;
pub mod registry;

#[allow(unused_imports)]
pub use registry::{OmniPlugin, PluginContext, PluginMeta, PluginRegistry};
