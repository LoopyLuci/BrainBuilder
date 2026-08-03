//! OmniForge Concierge – the agentic personal assistant that controls the entire platform
//! through natural language.
//!
//! # Features
//! - ReAct agent loop with correct single-assistant-message semantics
//! - Tool registry covering model import, canvas ops, training, knowledge modules, etc.
//! - Generic LLM client: OpenRouter (free models with automatic fallback), Ollama, any OpenAI-compatible API
//! - Task-aware local code generation (OpenCode Go/Zen style) + mandatory sandbox
//! - SQLite conversation memory that persists tool_call_id and tool_calls
//! - Graceful tool-error recovery
//! - Production-ready error handling and structured logging

pub mod agent;
pub mod tools;
pub mod llm_client;
pub mod openrouter;
pub mod code_agent;
pub mod memory;
pub mod error;

pub use agent::{ConciergeAgent, FREE_MODELS};
pub use error::ConciergeError;
pub use llm_client::{LlmClient, LlmMessage, LlmRequest, LlmResponse, ToolCall, FunctionCall};
pub use openrouter::OpenRouterClient;
pub use code_agent::{CodeAgent, CodeAgentMode};
pub use memory::{ConversationMemory, Message};
pub use tools::implementations::{PlatformHost, MockPlatformHost, ToolExecutor};
pub use tools::registry::ToolResult;
pub use tools::schemas::get_all_tools;
