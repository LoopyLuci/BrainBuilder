//! Community plugin discovery and execution.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub main: String,
    pub node_definitions: Vec<NodeDefinition>,
    pub tools: Option<Vec<ToolDef>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub id: String,
    pub name: String,
    pub category: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

fn plugin_root() -> PathBuf {
    std::env::var("OMNIFORGE_PLUGINS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("plugins"))
}

pub fn discover_plugins() -> Vec<PluginManifest> {
    let mut plugins = Vec::new();
    let dir = plugin_root();
    if !dir.exists() {
        return plugins;
    }
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("manifest.json");
            if let Ok(content) = fs::read_to_string(&manifest_path) {
                if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                    plugins.push(manifest);
                }
            }
        }
    }
    plugins
}

pub async fn execute_plugin_tool(
    plugin_name: &str,
    tool_name: &str,
    arguments: &str,
) -> Result<String, String> {
    let plugins = discover_plugins();
    let manifest = plugins
        .iter()
        .find(|p| p.name == plugin_name)
        .ok_or_else(|| format!("Plugin not found: {plugin_name}"))?;

    let entry = plugin_root().join(&manifest.name).join(&manifest.main);
    if !entry.exists() {
        return Err(format!("Plugin entry missing: {}", entry.display()));
    }

    let output = tokio::process::Command::new(crate::paths::python_cmd())
        .arg(&entry)
        .arg("--tool")
        .arg(tool_name)
        .arg("--args")
        .arg(arguments)
        .output()
        .await
        .map_err(|e| e.to_string())?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).to_string())
    }
}
