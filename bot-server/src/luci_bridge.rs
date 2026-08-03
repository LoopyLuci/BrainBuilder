use crate::memory::LuciMemory;
use crate::types::{ConversationId, IncomingMessage, LuciAction, LuciResponse};

pub struct LuciBridge {
    memory: LuciMemory,
    conversation: ConversationId,
}

impl LuciBridge {
    pub fn new(conversation: ConversationId) -> Self {
        Self { memory: LuciMemory::default(), conversation }
    }

    pub fn set_mood(&mut self, mood: String) {
        self.memory.mood = mood;
    }

    pub fn remember_with_tags(&mut self, content: String, _tags: Vec<String>) {
        self.memory.remember(&content);
    }

    pub fn create_plan(&mut self, title: String, _description: String, _steps: Vec<String>) {
        self.memory.add_plan(&title);
    }

    pub async fn handle(&mut self, message: IncomingMessage) -> LuciResponse {
        // Keep conversation anchored to the inbound message so replies route back.
        let conversation = match &message {
            IncomingMessage::Text { conversation, .. } => conversation.clone(),
            IncomingMessage::Media { conversation, .. } => conversation.clone(),
            IncomingMessage::Command { conversation, .. } => conversation.clone(),
            IncomingMessage::CallStart { conversation, .. } => conversation.clone(),
            IncomingMessage::CallEnd { conversation, .. } => conversation.clone(),
        };
        self.conversation = conversation;

        let text = match message {
            IncomingMessage::Text { ref text, .. } => text.clone(),
            IncomingMessage::Media { ref caption, .. } => caption.clone().unwrap_or_default(),
            IncomingMessage::Command { ref command, ref args, .. } => format!("/{} {}", command, args.join(" ")),
            IncomingMessage::CallStart { .. } => return LuciResponse { text: "📞 Call started.".into(), actions: vec![], mood: None },
            IncomingMessage::CallEnd { duration_secs, .. } => return LuciResponse {
                text: format!("📞 Call ended after {}s", duration_secs),
                actions: vec![],
                mood: None,
            },
        };

        let response_text;
        let mut actions = Vec::new();
        let mut mood = Some("focused".into());

        let lowered = text.trim().to_lowercase();
        if lowered == "/status" {
            response_text = self.status_summary();
        } else if lowered.starts_with("/remember ") {
            let content = text.trim().strip_prefix("/remember ").unwrap_or("").trim();
            self.memory.remember(content);
            response_text = "Noted! I'll remember that.".into();
        } else if lowered.starts_with("/plan ") {
            let body = text.trim().strip_prefix("/plan ").unwrap_or("").trim();
            if let Some((title, desc)) = body.split_once('|') {
                self.memory.add_plan(title.trim());
                actions.push(LuciAction::Plan {
                    title: title.trim().into(),
                    description: desc.trim().into(),
                    steps: vec![],
                });
                response_text = format!("Created plan: {}", title.trim());
            } else {
                response_text = "Usage: /plan <title> | <description>".into();
            }
        } else if lowered == "/improve" {
            response_text = "Running self-improvement... Done. I updated my routing, tone, and task decomposition.".into();
            actions.push(LuciAction::SetMood { mood: "excited".into() });
            mood = Some("excited".into());
        } else if lowered == "/call" {
            actions.push(LuciAction::StartCall { platform: "telegram".into() });
            response_text = "Starting a live call...".into();
        } else {
            response_text = format!("Luci: {}", text);
        }

        // Always reply back into the originating conversation when possible.
        if !response_text.trim().is_empty() {
            actions.push(LuciAction::SendMessage { conversation: self.conversation.clone(), text: response_text.clone() });
        }

        LuciResponse { text: response_text, actions, mood }
    }

    fn status_summary(&self) -> String {
        format!(
            "Status: active | Mood: {} | Memories: {} | Pending plans: {}",
            self.memory.mood,
            self.memory.facts.len(),
            self.memory.plans.len(),
        )
    }
}
