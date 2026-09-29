use crate::tool_executor::registry::ToolRegistry;
use crate::tool_executor::types::ToolCall;
use tauri::State;
use serde_json::Value;

pub struct ToolExecutorState {
    pub registry: ToolRegistry,
}

#[tauri::command]
pub async fn tool_executor_run(
    state: State<'_, ToolExecutorState>,
    tool_name: String,
    args: Value,
) -> Result<Value, String> {
    let call = ToolCall {
        id: uuid::Uuid::new_v4().to_string(),
        tool_name,
        args,
        caller: "tauri".into(),
        status: "pending".into(),
        result: None,
        error: None,
        started_at: chrono::Utc::now().to_rfc3339(),
        finished_at: None,
    };
    let result = state.registry.executor.execute(call).await;
    Ok(Value::Object({
        let mut map = serde_json::Map::new();
        map.insert("id".into(), Value::String(result.id));
        map.insert("tool_name".into(), Value::String(result.tool_name));
        map.insert("status".into(), Value::String(result.status));
        map.insert("result".into(), result.result.unwrap_or(Value::Null));
        map.insert("error".into(), result.error.map(Value::String).unwrap_or(Value::Null));
        map.insert("started_at".into(), Value::String(result.started_at));
        map.insert("finished_at".into(), result.finished_at.map(Value::String).unwrap_or(Value::Null));
        map
    }))
}

#[tauri::command]
pub async fn tool_executor_list(
    state: State<'_, ToolExecutorState>,
) -> Result<Vec<String>, String> {
    let tools = state.registry.executor.list_tools().await;
    Ok(tools)
}
