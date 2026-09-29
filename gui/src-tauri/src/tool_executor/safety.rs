use super::super::tool_executor::executor::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use crate::tool_executor::world::Workspace;

#[derive(Clone)]
pub struct SafetyStore {
    workspace: Arc<Workspace>,
    path: &'static str,
}

impl SafetyStore {
    pub fn new(workspace: Arc<Workspace>) -> Self {
        Self { workspace, path: "safety/rules.json" }
    }

    pub fn deny_paths(&self) -> Vec<String> {
        if let Ok(contents) = self.workspace.read_file(self.path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                return json.get("deny_paths").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            }
        }
        Vec::new()
    }

    pub fn allow_shell(&self) -> Vec<String> {
        if let Ok(contents) = self.workspace.read_file(self.path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                return json.get("allow_shell").and_then(|v| v.as_array()).map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect()).unwrap_or_default();
            }
        }
        Vec::new()
    }

    pub fn max_http_size(&self) -> usize {
        if let Ok(contents) = self.workspace.read_file(self.path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                return json.get("max_http_size_bytes").and_then(|v| v.as_u64()).unwrap_or(2 * 1024 * 1024) as usize;
            }
        }
        2 * 1024 * 1024
    }
}

pub struct ApprovalGateTool;
#[async_trait]
impl Tool for ApprovalGateTool {
    fn name(&self) -> &'static str { "safety.approve" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let action = args.get("action").and_then(|v| v.as_str()).ok_or("action required")?;
        Ok(serde_json::json!({"action": action, "approved": false, "reason": "approval required"}))
    }
}

pub struct BudgetTool;
#[async_trait]
impl Tool for BudgetTool {
    fn name(&self) -> &'static str { "safety.budget" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let action = args.get("action").and_then(|v| v.as_str()).ok_or("action required")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(0);
        Ok(serde_json::json!({"action": action, "limit": limit, "remaining": limit}))
    }
}

pub struct RedTeamTool;
#[async_trait]
impl Tool for RedTeamTool {
    fn name(&self) -> &'static str { "safety.redteam" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let target = args.get("target").and_then(|v| v.as_str()).ok_or("target required")?;
        Ok(serde_json::json!({"target": target, "findings": Vec::<Value>::new()}))
    }
}
