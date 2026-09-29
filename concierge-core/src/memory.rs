//! Persistent conversation memory backed by SQLite via rusqlite.
//!
//! Schema stores role, content, timestamp, optional tool_call_id, and optional
//! tool_calls JSON so that multi-turn tool conversations can be reconstructed
//! correctly across process restarts.

use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use rusqlite::{params, Connection, OptionalExtension, Row};

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

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        let ts = row.get::<_, String>(2)?;
        let timestamp = DateTime::parse_from_rfc3339(&ts)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now());
        let tool_calls_json: Option<String> = row.get(4)?;
        let tool_calls = tool_calls_json.and_then(|j| serde_json::from_str::<Vec<ToolCall>>(&j).ok());
        Ok(Message {
            role: row.get(0)?,
            content: row.get(1)?,
            timestamp,
            tool_call_id: row.get(3)?,
            tool_calls,
        })
    }
}

/// SQLite-backed conversation store.
#[derive(Clone)]
pub struct ConversationMemory {
    pool: Arc<Mutex<Connection>>,
}

impl ConversationMemory {
    /// Open (or create) a SQLite database at the given path.
    pub async fn new(db_path: &str) -> Result<Self, ConciergeError> {
        let db_path = std::path::Path::new(db_path);
        let conn = rusqlite::Connection::open(db_path)
            .map_err(|e| ConciergeError::Memory(format!("Failed to open SQLite: {e}")))?;
        // WAL mode for concurrent readers
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA foreign_keys=ON;",
        )
        .map_err(|e| ConciergeError::Memory(format!("PRAGMA failed: {e}")))?;

        conn.execute(
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
            [],
        )
        .map_err(|e| ConciergeError::Memory(format!("Failed to create schema: {e}")))?;

        // Migrate older schemas that may lack the new columns
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN tool_call_id TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE messages ADD COLUMN tool_calls_json TEXT",
            [],
        );

        Ok(Self {
            pool: Arc::new(Mutex::new(conn)),
        })
    }

    /// Append a message to the conversation history.
    pub async fn add_message(&self, msg: Message) -> Result<(), ConciergeError> {
        let tool_calls_json = msg
            .tool_calls
            .as_ref()
            .map(|tc| serde_json::to_string(tc).unwrap_or_default());

        let conn = self.pool.lock().await;
        conn.execute(
            r#"
            INSERT INTO messages (role, content, timestamp, tool_call_id, tool_calls_json)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
            params![
                msg.role,
                msg.content,
                msg.timestamp.to_rfc3339(),
                msg.tool_call_id,
                tool_calls_json,
            ],
        )
        .map_err(|e| ConciergeError::Memory(format!("Failed to insert message: {e}")))?;
        Ok(())
    }

    /// Retrieve the most recent `limit` messages in chronological order.
    pub async fn get_history(&self, limit: usize) -> Result<Vec<Message>, ConciergeError> {
        let conn = self.pool.lock().await;
        let mut stmt = conn.prepare(
            r#"
            SELECT role, content, timestamp, tool_call_id, tool_calls_json
            FROM messages
            ORDER BY id DESC
            LIMIT ?1
            "#,
        )
        .map_err(|e| ConciergeError::Memory(format!("Prepare failed: {e}")))?;

        let rows = stmt
            .query_map(params![limit as i64], |row| Message::from_row(row))
            .map_err(|e| ConciergeError::Memory(format!("Query failed: {e}")))?;

        let mut messages: Vec<Message> = Vec::new();
        for row in rows {
            messages.push(row.map_err(|e| ConciergeError::Memory(format!("Row failed: {e}")))?);
        }
        messages.reverse(); // chronological
        Ok(messages)
    }

    /// Delete all stored messages (useful for testing or new conversation).
    pub async fn clear(&self) -> Result<(), ConciergeError> {
        let conn = self.pool.lock().await;
        conn.execute("DELETE FROM messages", [])
            .map_err(|e| ConciergeError::Memory(format!("Failed to clear history: {e}")))?;
        Ok(())
    }

    /// Return the total number of stored messages.
    pub async fn count(&self) -> Result<i64, ConciergeError> {
        let conn = self.pool.lock().await;
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM messages", [], |row| row.get(0))
            .optional()
            .map_err(|e| ConciergeError::Memory(format!("Count failed: {e}")))?
            .unwrap_or(0);
        Ok(count)
    }
}
