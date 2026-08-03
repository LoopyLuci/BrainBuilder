//! Core ReAct agent loop that ties LLM, tools, memory and the code agent together.
//!
//! Correctly emits a single assistant message when both content and tool_calls
//! are present, persists tool_call_id, and recovers gracefully from tool failures.

use std::sync::Arc;

use chrono::Utc;
use tracing::{debug, info, warn};

use crate::code_agent::{CodeAgent, CodeAgentMode};
use crate::error::ConciergeError;
use crate::llm_client::{LlmClient, LlmMessage, LlmRequest, ToolCall};
use crate::memory::{ConversationMemory, Message};
use crate::tools::implementations::{MockPlatformHost, PlatformHost, ToolExecutor};
use crate::tools::registry::ToolResult;
use crate::tools::schemas::get_all_tools;

/// Well-known free OpenRouter models that require no paid key.
pub const FREE_MODELS: &[&str] = &[
    "mistralai/mistral-7b-instruct:free",
    "google/gemma-2-9b-it:free",
    "meta-llama/llama-3.2-3b-instruct:free",
    "microsoft/phi-3-mini-128k-instruct:free",
    "qwen/qwen-2-7b-instruct:free",
    "openchat/openchat-7b:free",
];

/// The OmniForge Concierge – an agentic personal assistant that can drive the
/// entire platform through natural language.
pub struct ConciergeAgent {
    llm_client: Arc<dyn LlmClient>,
    tool_executor: ToolExecutor,
    memory: ConversationMemory,
    code_agent: CodeAgent,
    max_turns: usize,
    system_prompt: String,
    /// Default model id used when the caller does not override it.
    default_model: String,
}

impl ConciergeAgent {
    /// Create a new Concierge with the given LLM client and SQLite memory path.
    /// Uses the mock platform host (replace in production).
    pub async fn new(
        llm_client: Arc<dyn LlmClient>,
        memory_path: &str,
    ) -> Result<Self, ConciergeError> {
        Self::with_host(llm_client, memory_path, Box::new(MockPlatformHost)).await
    }

    /// Create a Concierge with a custom PlatformHost implementation.
    pub async fn with_host(
        llm_client: Arc<dyn LlmClient>,
        memory_path: &str,
        host: Box<dyn PlatformHost>,
    ) -> Result<Self, ConciergeError> {
        let memory = ConversationMemory::new(memory_path).await?;
        let tool_executor = ToolExecutor::new(host);
        let code_agent = CodeAgent::new(CodeAgentMode::Go);

        let system_prompt = build_system_prompt();

        Ok(Self {
            llm_client,
            tool_executor,
            memory,
            code_agent,
            max_turns: 12,
            system_prompt,
            default_model: FREE_MODELS[0].to_string(),
        })
    }

    /// Override the default model (any OpenRouter / Ollama / compatible id).
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.default_model = model.into();
        self
    }

    /// Change the maximum number of ReAct turns.
    pub fn with_max_turns(mut self, turns: usize) -> Self {
        self.max_turns = turns;
        self
    }

    /// Access the underlying conversation memory (for UI history, export, …).
    pub fn memory(&self) -> &ConversationMemory {
        &self.memory
    }

    /// Current default model id.
    pub fn model(&self) -> &str {
        &self.default_model
    }

    /// Switch the active model at runtime.
    pub fn set_model(&mut self, model: impl Into<String>) {
        self.default_model = model.into();
    }

    /// List known free OpenRouter models.
    pub fn free_models() -> &'static [&'static str] {
        FREE_MODELS
    }

    /// Switch code-generation backend.
    pub fn set_code_mode(&mut self, mode: CodeAgentMode) {
        self.code_agent.set_mode(mode);
    }

    pub fn code_mode(&self) -> CodeAgentMode {
        self.code_agent.mode()
    }

    /// Main entry point: send a user message and receive the final assistant reply.
    pub async fn chat(&mut self, user_input: &str) -> Result<String, ConciergeError> {
        info!(user_input = %user_input, model = %self.default_model, "Concierge chat started");

        // Persist user turn
        self.memory
            .add_message(Message::user(user_input.to_string()))
            .await?;

        // Build the message list for the LLM from persisted history
        let mut messages = self.build_llm_messages().await?;

        let tools = get_all_tools();
        let mut final_response = String::new();

        for turn in 0..self.max_turns {
            debug!(turn, "Agent turn");

            let request = LlmRequest {
                model: self.default_model.clone(),
                messages: messages.clone(),
                temperature: 0.4,
                max_tokens: 2048,
                tools: Some(tools.clone()),
                tool_choice: Some(serde_json::json!("auto")),
            };

            let response = self.llm_client.chat(request).await?;
            let choice = &response.choices[0];
            let finish = choice.finish_reason.as_deref().unwrap_or("");

            let content = choice.message.content.clone().unwrap_or_default();
            let tool_calls = choice.message.tool_calls.clone();

            // ── Single assistant message (content ± tool_calls) ──────────
            // Never emit two consecutive assistant messages for the same turn.
            if tool_calls.as_ref().map(|t| !t.is_empty()).unwrap_or(false) {
                let tcs = tool_calls.clone().unwrap_or_default();

                // Persist once with both fields
                self.memory
                    .add_message(Message::assistant_with_tools(content.clone(), tcs.clone()))
                    .await?;

                // Live context: one assistant message carrying tool_calls
                messages.push(LlmMessage {
                    role: "assistant".into(),
                    content: content.clone(),
                    tool_call_id: None,
                    tool_calls: Some(tcs.clone()),
                });

                if !content.trim().is_empty() {
                    final_response = content.clone();
                }

                // Execute each tool; failures become ToolResult::err so the
                // model can recover instead of aborting the whole conversation.
                for tc in &tcs {
                    let tool_name = &tc.function.name;
                    let arguments: serde_json::Value =
                        serde_json::from_str(&tc.function.arguments)
                            .unwrap_or_else(|_| serde_json::json!({}));

                    info!(tool = %tool_name, args = %arguments, "Executing tool");

                    let result = self.dispatch_tool(tool_name, &arguments).await;

                    let result_json = serde_json::to_string(&result)
                        .unwrap_or_else(|_| r#"{"success":false,"data":"","error":"serialize"}"#.into());

                    // Live context for the next LLM turn
                    messages.push(LlmMessage::tool(&tc.id, &result_json));

                    // Persist with the matching tool_call_id
                    self.memory
                        .add_message(Message::tool(
                            tc.id.clone(),
                            format!("{} → {}", tool_name, result.data),
                        ))
                        .await?;
                }

                // Continue the loop so the model can react to tool results
                continue;
            }

            // Plain text reply (no tool calls)
            if !content.trim().is_empty() {
                self.memory
                    .add_message(Message::assistant(content.clone()))
                    .await?;
                messages.push(LlmMessage::assistant(content.clone()));
                final_response = content;
            }

            // Terminal conditions
            if finish == "stop" || finish == "end_turn" || tool_calls.is_none() {
                break;
            }

            if final_response.is_empty() {
                warn!("Empty response with no tool calls – stopping");
                break;
            }
        }

        if final_response.is_empty() {
            final_response =
                "I completed the requested actions. Let me know if you need anything else."
                    .to_string();
        }

        Ok(final_response)
    }

    /// Execute user-approved generated code.
    pub async fn execute_generated_code(&self, code: &str) -> Result<String, ConciergeError> {
        self.code_agent.execute_sandboxed(code).await
    }

    /// Generate Python code without executing it.
    pub async fn generate_code(&self, task: &str) -> Result<String, ConciergeError> {
        self.code_agent.generate_python(task).await
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Reconstruct the full LLM message list from SQLite history, correctly
    /// attaching tool_call_id and tool_calls so providers accept the payload.
    async fn build_llm_messages(&self) -> Result<Vec<LlmMessage>, ConciergeError> {
        let mut messages = vec![LlmMessage::system(&self.system_prompt)];

        let history = self.memory.get_history(40).await?;
        for msg in history {
            if msg.role == "system" {
                continue; // already injected
            }
            messages.push(LlmMessage {
                role: msg.role,
                content: msg.content,
                tool_call_id: msg.tool_call_id,
                tool_calls: msg.tool_calls,
            });
        }
        Ok(messages)
    }

    /// Route a tool call, converting host errors into ToolResult::err so the
    /// ReAct loop can continue.
    async fn dispatch_tool(
        &mut self,
        tool_name: &str,
        arguments: &serde_json::Value,
    ) -> ToolResult {
        match tool_name {
            "set_code_agent_mode" => {
                let mode_str = arguments["mode"].as_str().unwrap_or("go");
                let mode = if mode_str.eq_ignore_ascii_case("zen") {
                    CodeAgentMode::Zen
                } else {
                    CodeAgentMode::Go
                };
                self.code_agent.set_mode(mode);
                ToolResult::ok(format!("Code agent mode set to {:?}", mode))
            }
            "execute_python" => match self.handle_execute_python(arguments).await {
                Ok(r) => r,
                Err(e) => ToolResult::err(e.to_string()),
            },
            "set_model" => {
                // Optional meta-tool for dynamic model switching
                if let Some(m) = arguments["model"].as_str() {
                    self.default_model = m.to_string();
                    ToolResult::ok(format!("Model switched to {}", m))
                } else {
                    ToolResult::err("Missing 'model' argument".into())
                }
            }
            _ => match self.tool_executor.execute(tool_name, arguments.clone()).await {
                Ok(r) => r,
                Err(e) => {
                    warn!(tool = %tool_name, error = %e, "Tool execution failed");
                    ToolResult::err(e.to_string())
                }
            },
        }
    }

    async fn handle_execute_python(
        &self,
        arguments: &serde_json::Value,
    ) -> Result<ToolResult, ConciergeError> {
        if let Some(code) = arguments["code"].as_str() {
            let output = self.code_agent.execute_sandboxed(code).await?;
            return Ok(ToolResult::ok(output));
        }

        let task = arguments["task"].as_str().unwrap_or("print('hello from Concierge')");
        let code = self.code_agent.generate_python(task).await?;
        // Safety: generate only; show to user for approval before execution.
        Ok(ToolResult::ok(format!(
            "Generated code (not yet executed – present to user for approval):\n\n```python\n{}\n```",
            code
        )))
    }
}

fn build_system_prompt() -> String {
    let tools_summary = get_all_tools()
        .iter()
        .filter_map(|t| t["function"]["name"].as_str())
        .collect::<Vec<_>>()
        .join(", ");

    format!(
        r#"You are the OmniForge Concierge – a highly capable agentic personal assistant that helps individuals and small teams build, train, compose and deploy AI models.

You have full access to the OmniForge platform through the following tools:
{tools_summary}

Guidelines:
- Prefer using tools over pure text when the user asks to search, import, add nodes, train, merge, etc.
- When generating or executing Python code, prefer the write_plugin / execute_python tools and always show the generated code to the user before executing it.
- Be concise, precise and helpful. Explain what you are about to do in one short sentence before calling tools.
- If a tool returns an error or a mock result, acknowledge it and continue the conversational flow; never invent IDs.
- Never invent model IDs or node IDs; always use the values returned by previous tool calls.
- You can switch between OpenCode Go (fast) and OpenCode Zen (deep) via set_code_agent_mode.
- For free-tier OpenRouter models, keep responses short and focused.

You are the single conversational interface for the entire platform. Make complex multi-step workflows feel effortless."#
    )
}
