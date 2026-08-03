//! Persistent conversation memory backed by SQLite.
//!
//! Schema stores role, content, timestamp, optional tool_call_id, and optional
//! tool_calls JSON so that multi-turn tool conversations can be reconstructed
//! correctly across process restarts.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};

use crate::error::ConciergeError;
use crate::llm_client::ToolCall;

/// A single turn in the conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String, // "user" | "assistant" | "system" | "tool"
    pub content: String,
    pub timestamp: DateTime<Utc>,
    /// Present when role == "tool"; matches the id of the assistant's tool_call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Present when role == "assistant" and the model requested tools.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        }
    }

    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            timestamp: Utc::now(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// SQLite-backed conversation store.
#[derive(Clone)]
pub struct ConversationMemory {
    pool: Pool<Sqlite>,
}

impl ConversationMemory {
    /// Open (or create) a SQLite database at the given URL.
    /// Example URLs: `"sqlite://concierge.db"`, `"sqlite::memory:"`.
    pub async fn new(database_url: &str) -> Result<Self, ConciergeError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Failed to connect to SQLite: {e}")))?;

        // Create base table (compatible with older DBs that lack the new columns)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                tool_call_id TEXT,
                tool_calls_json TEXT
            )
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Failed to create schema: {e}")))?;

        // Migrate older schemas that may lack the new columns
        let _ = sqlx::query("ALTER TABLE messages ADD COLUMN tool_call_id TEXT")
            .execute(&pool)
            .await;
        let _ = sqlx::query("ALTER TABLE messages ADD COLUMN tool_calls_json TEXT")
            .execute(&pool)
            .await;

        Ok(Self { pool })
    }

    /// Append a message to the conversation history.
    pub async fn add_message(&self, msg: Message) -> Result<(), ConciergeError> {
        let tool_calls_json = msg
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc).unwrap_or_default());

        sqlx::query(
            r#"
            INSERT INTO messages (role, content, timestamp, tool_call_id, tool_calls_json)
            VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(msg.timestamp.to_rfc3339())
        .bind(&msg.tool_call_id)
        .bind(&tool_calls_json)
        .execute(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Failed to insert message: {e}")))?;
        Ok(())
    }

    /// Retrieve the most recent `limit` messages in chronological order.
    pub async fn get_history(&self, limit: usize) -> Result<Vec<Message>, ConciergeError> {
        let rows = sqlx::query_as::<_, (String, String, String, Option<String>, Option<String>)>(
            r#"
            SELECT role, content, timestamp, tool_call_id, tool_calls_json
            FROM messages
            ORDER BY id DESC
            LIMIT ?
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Failed to fetch history: {e}")))?;

        let messages = rows
            .into_iter()
            .rev() // chronological
            .map(|(role, content, ts, tool_call_id, tool_calls_json)| {
                let timestamp = DateTime::parse_from_rfc3339(&ts)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                let tool_calls = tool_calls_json.and_then(|j| {
                    serde_json::from_str::<Vec<ToolCall>>(&j).ok()
                });
                Message {
                    role,
                    content,
                    timestamp,
                    tool_call_id,
                    tool_calls,
                }
            })
            .collect();

        Ok(messages)
    }

    /// Delete all stored messages (useful for testing or “new conversation”).
    pub async fn clear(&self) -> Result<(), ConciergeError> {
        sqlx::query("DELETE FROM messages")
            .execute(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Failed to clear history: {e}")))?;
        Ok(())
    }

    /// Return the total number of stored messages.
    pub async fn count(&self) -> Result<i64, ConciergeError> {
        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM messages")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Failed to count messages: {e}")))?;
        Ok(count)
    }
}
