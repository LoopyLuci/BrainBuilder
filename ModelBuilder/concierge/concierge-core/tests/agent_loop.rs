//! Basic offline tests that do not require a live LLM.
//! Run with: cargo test --test agent_loop

use concierge_core::memory::{ConversationMemory, Message};
use concierge_core::tools::registry::ToolResult;
use concierge_core::llm_client::ToolCall;
use concierge_core::code_agent::{CodeAgent, CodeAgentMode};

#[tokio::test]
async fn memory_persists_tool_call_id() {
    let mem = ConversationMemory::new("sqlite::memory:").await.expect("db");
    let tc = ToolCall {
        id: "call_abc".into(),
        call_type: "function".into(),
        function: concierge_core::llm_client::FunctionCall {
            name: "search_models".into(),
            arguments: r#"{"query":"llama"}"#.into(),
        },
    };
    mem.add_message(Message::assistant_with_tools("searching…", vec![tc.clone()]))
        .await
        .unwrap();
    mem.add_message(Message::tool("call_abc", "Found 0 models"))
        .await
        .unwrap();

    let history = mem.get_history(10).await.unwrap();
    assert_eq!(history.len(), 2);
    assert!(history[0].tool_calls.is_some());
    assert_eq!(history[0].tool_calls.as_ref().unwrap()[0].id, "call_abc");
    assert_eq!(history[1].tool_call_id.as_deref(), Some("call_abc"));
}

#[tokio::test]
async fn code_agent_generates_training_scaffold() {
    let agent = CodeAgent::new(CodeAgentMode::Go);
    let code = agent
        .generate_python("Fine-tune a model with LoRA on my dataset")
        .await
        .unwrap();
    assert!(code.contains("build_lora_config") || code.contains("train_loop"));
    assert!(code.contains("def main"));
}

#[tokio::test]
async fn code_agent_refuses_without_sandbox() {
    // Force a non-existent sandbox path
    std::env::set_var("CONCIERGE_SANDBOX", "/nonexistent/sandbox.py");
    let agent = CodeAgent::new(CodeAgentMode::Go);
    let err = agent
        .execute_sandboxed("print('hi')")
        .await
        .expect_err("must fail when sandbox missing");
    let msg = err.to_string();
    assert!(msg.contains("Sandbox script not found") || msg.contains("Refusing"));
    std::env::remove_var("CONCIERGE_SANDBOX");
}

#[test]
fn tool_result_err_serializes() {
    let r = ToolResult::err("boom");
    let j = serde_json::to_string(&r).unwrap();
    assert!(j.contains("\"success\":false"));
    assert!(j.contains("boom"));
}
