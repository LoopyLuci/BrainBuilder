//! In-process plugin registry (Tauri-plugin-shaped).

use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

#[derive(Debug, Clone, Serialize)]
pub struct PluginMeta {
    pub name: String,
    pub version: String,
    pub description: String,
    pub commands: Vec<String>,
}

/// Shared handles plugins may need during setup / command handling.
pub struct PluginContext {
    pub app: Option<tauri::AppHandle>,
}

pub trait OmniPlugin: Send + Sync {
    fn meta(&self) -> PluginMeta;
    /// Called once when the registry is bound to a running app.
    fn setup(&self, _ctx: &PluginContext) -> Result<(), String> {
        Ok(())
    }
}

pub struct PluginRegistry {
    plugins: HashMap<String, Arc<dyn OmniPlugin>>,
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    pub fn register(&mut self, plugin: Arc<dyn OmniPlugin>) {
        let meta = plugin.meta();
        info!(name = %meta.name, version = %meta.version, "plugin registered");
        self.plugins.insert(meta.name.clone(), plugin);
    }

    pub fn list(&self) -> Vec<PluginMeta> {
        self.plugins.values().map(|p| p.meta()).collect()
    }

    pub fn setup_all(&self, ctx: &PluginContext) -> Result<(), String> {
        for p in self.plugins.values() {
            p.setup(ctx)?;
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn OmniPlugin>> {
        self.plugins.get(name).cloned()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}
