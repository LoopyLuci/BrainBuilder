use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub params: HashMap<String, String>,
    pub implementation: String,
    pub source: String,
    pub confidence: f64,
    pub success_count: u64,
    pub failure_count: u64,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool_name: String,
    pub args: serde_json::Value,
    pub caller: String,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}
