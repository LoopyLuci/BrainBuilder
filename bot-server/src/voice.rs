use crate::types::{CallSession, ConversationId, TranscriptEntry, UserId};

pub struct VoiceServer {
    _sessions: Vec<CallSession>,
}

impl Default for VoiceServer {
    fn default() -> Self {
        Self { _sessions: Vec::new() }
    }
}

impl VoiceServer {
    pub async fn start_call(&self, conversation: ConversationId, user: UserId, platform: String) -> CallSession {
        let session = CallSession {
            id: uuid::Uuid::new_v4().to_string(),
            conversation: conversation.clone(),
            user: user.clone(),
            platform,
            started_at: chrono::Utc::now().timestamp(),
            ended_at: None,
            transcript: Vec::new(),
        };
        CallSession {
            id: session.id.clone(),
            conversation: session.conversation.clone(),
            user: session.user.clone(),
            platform: session.platform.clone(),
            started_at: session.started_at,
            ended_at: None,
            transcript: Vec::new(),
        }
    }

    pub async fn transcript(&self, _session_id: &str) -> Vec<TranscriptEntry> {
        Vec::new()
    }

    pub async fn create_peer_connection(&self) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
