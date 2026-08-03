use std::sync::Arc;
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::RwLock;
use crate::tool_executor::types::ToolCall;

#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &'static str;
    async fn execute(&self, args: &serde_json::Value) -> Result<serde_json::Value, String>;
}

pub struct ToolExecutor {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolExecutor {
    pub fn new() -> Self {
        Self { tools: Arc::new(RwLock::new(HashMap::new())) }
    }

    pub async fn register(&self, tool: Arc<dyn Tool>) {
        self.tools.write().await.insert(tool.name().to_string(), tool);
    }

    pub async fn execute(&self, call: ToolCall) -> ToolCall {
        let mut result = call;
        let tools = self.tools.read().await;
        if let Some(tool) = tools.get(&result.tool_name) {
            match tool.execute(&result.args).await {
                Ok(val) => {
                    result.status = "succeeded".into();
                    result.result = Some(val);
                }
                Err(err) => {
                    result.status = "failed".into();
                    result.error = Some(err);
                }
            }
        } else {
            result.status = "failed".into();
            result.error = Some(format!("tool not found: {}", result.tool_name));
        }
        result
    }

    pub async fn list_tools(&self) -> Vec<String> {
        self.tools.read().await.keys().cloned().collect()
    }
}
