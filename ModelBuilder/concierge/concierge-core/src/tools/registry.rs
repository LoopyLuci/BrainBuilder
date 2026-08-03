//! Shared types for tool results.

use serde::{Deserialize, Serialize};

/// Result returned by every tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub success: bool,
    pub data: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl ToolResult {
    pub fn ok(data: impl Into<String>) -> Self {
        Self {
            success: true,
            data: data.into(),
            error: None,
        }
    }

    pub fn err(msg: impl Into<String>) -> Self {
        let m = msg.into();
        Self {
            success: false,
            data: String::new(),
            error: Some(m),
        }
    }
}
