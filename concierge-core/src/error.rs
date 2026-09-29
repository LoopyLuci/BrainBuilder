use thiserror::Error;

/// Unified error type for the entire Concierge stack.
#[derive(Error, Debug)]
pub enum ConciergeError {
    #[error("LLM client error: {0}")]
    LlmClient(String),

    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    #[error("Memory error: {0}")]
    Memory(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("Agent planning error: {0}")]
    AgentPlanning(String),

    #[error("Sandbox error: {0}")]
    Sandbox(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Timeout: {0}")]
    Timeout(String),
}

impl ConciergeError {
    pub fn llm(msg: impl Into<String>) -> Self {
        ConciergeError::LlmClient(msg.into())
    }

    pub fn tool(msg: impl Into<String>) -> Self {
        ConciergeError::ToolExecution(msg.into())
    }

    pub fn sandbox(msg: impl Into<String>) -> Self {
        ConciergeError::Sandbox(msg.into())
    }
}