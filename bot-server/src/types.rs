use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IncomingMessage {
    Text {
        conversation: ConversationId,
        user: UserId,
        text: String,
        platform: String,
        timestamp: i64,
        metadata: HashMap<String, String>,
    },
    Media {
        conversation: ConversationId,
        user: UserId,
        media_type: String,
        url: Option<String>,
        bytes: Option<Vec<u8>>,
        caption: Option<String>,
        platform: String,
        timestamp: i64,
    },
    CallStart {
        conversation: ConversationId,
        user: UserId,
        platform: String,
        timestamp: i64,
    },
    CallEnd {
        conversation: ConversationId,
        user: UserId,
        duration_secs: u64,
        platform: String,
        timestamp: i64,
    },
    Command {
        conversation: ConversationId,
        user: UserId,
        command: String,
        args: Vec<String>,
        platform: String,
        timestamp: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMessage {
    pub conversation: ConversationId,
    pub text: String,
    pub media: Option<OutgoingMedia>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutgoingMedia {
    pub media_type: String,
    pub url: Option<String>,
    pub bytes: Option<Vec<u8>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciResponse {
    pub text: String,
    pub actions: Vec<LuciAction>,
    pub mood: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LuciAction {
    SendMessage { conversation: ConversationId, text: String },
    SendMedia { media_type: String, url: Option<String>, bytes: Option<Vec<u8>> },
    StartCall { platform: String },
    EndCall { reason: String },
    SetMood { mood: String },
    Remember { content: String, tags: Vec<String> },
    Plan { title: String, description: String, steps: Vec<String> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub bot_id: String,
    pub platform: String,
    pub enabled: bool,
    pub credentials: HashMap<String, String>,
}

impl Default for BotConfig {
    fn default() -> Self {
        Self {
            bot_id: Uuid::new_v4().to_string(),
            platform: String::new(),
            enabled: true,
            credentials: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallSession {
    pub id: String,
    pub conversation: ConversationId,
    pub user: UserId,
    pub platform: String,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub transcript: Vec<TranscriptEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub speaker: String,
    pub text: String,
    pub timestamp: i64,
}
