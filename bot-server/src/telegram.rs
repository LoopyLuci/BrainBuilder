use async_trait::async_trait;
use crate::adapter::{AdapterResult, AdapterError, PlatformAdapter};
use crate::types::{BotConfig, IncomingMessage, OutgoingMessage, ConversationId, UserId};
use std::collections::HashMap;
use std::sync::Arc;
use teloxide::prelude::*;
use tokio::sync::{mpsc, Mutex};

pub struct TelegramAdapter {
    tx: mpsc::UnboundedSender<IncomingMessage>,
    bot: Arc<Mutex<Option<Bot>>>,
    shutdown: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl TelegramAdapter {
    pub fn new() -> Self {
        let (tx, _rx) = mpsc::unbounded_channel();
        Self { tx, bot: Arc::new(Mutex::new(None)), shutdown: Arc::new(Mutex::new(None)) }
    }

    pub fn new_with_sender(tx: Option<mpsc::UnboundedSender<IncomingMessage>>) -> Self {
        let (ch_tx, _rx) = mpsc::unbounded_channel();
        let tx = tx.unwrap_or(ch_tx);
        Self { tx, bot: Arc::new(Mutex::new(None)), shutdown: Arc::new(Mutex::new(None)) }
    }
}

#[async_trait]
impl PlatformAdapter for TelegramAdapter {
    fn platform(&self) -> &'static str { "telegram" }

    async fn start(&self, config: BotConfig) -> Result<(), AdapterError> {
        let token = config.credentials.get("token").ok_or_else(|| AdapterError::Upstream("missing telegram token".into()))?.clone();
        let bot = Bot::new(token);
        let tx = self.tx.clone();
        let _bot_arc = Arc::new(Mutex::new(Some(bot.clone())));
        *self.bot.lock().await = Some(bot.clone());

        let handle = tokio::spawn(async move {
            let mut offset: i32 = 0;
            loop {
                let updates = match bot.get_updates().limit(100).offset(offset).send().await {
                    Ok(u) => u,
                    Err(_) => {
                        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                        continue;
                    }
                };
                for update in updates {
                    offset = update.id.0 as i32;
                    if let teloxide::types::UpdateKind::Message(message) = &update.kind {
                        let user = message.from.as_ref().map(|u| UserId(u.id.to_string())).unwrap_or(UserId("unknown".into()));
                        let text = message.text().unwrap_or("").to_string();
                        let incoming = IncomingMessage::Text {
                            conversation: ConversationId(message.chat.id.to_string()),
                            user,
                            text,
                            platform: "telegram".into(),
                            timestamp: message.date.timestamp(),
                            metadata: HashMap::new(),
                        };
                        let _ = tx.send(incoming);
                    }
                }
            }
        });
        *self.shutdown.lock().await = Some(handle);
        Ok(())
    }

    async fn stop(&self) -> Result<(), AdapterError> {
        let mut bot_lock = self.bot.lock().await;
        if let Some(bot) = bot_lock.take() {
            let _ = bot.send_message(ChatId(0), "stopping").await.map_err(|e| AdapterError::Upstream(e.to_string()));
        }
        let mut shutdown_lock = self.shutdown.lock().await;
        if let Some(handle) = shutdown_lock.take() {
            handle.abort();
        }
        Ok(())
    }

    async fn send(&self, message: OutgoingMessage) -> Result<(), AdapterError> {
        let bot_lock = self.bot.lock().await;
        if let Some(bot) = bot_lock.as_ref() {
            let chat_id = message.conversation.0.parse::<i64>().map(ChatId).or_else(|_| message.conversation.0.parse::<String>().map(|s| ChatId(s.parse::<i64>().unwrap_or(0))));
            let chat_id = match chat_id { Ok(v) => v, Err(_) => return Ok(()) };
            if chat_id.0 != 0 {
                bot.send_message(chat_id, message.text).await.map_err(|e| AdapterError::Upstream(e.to_string()))?;
            }
        }
        Ok(())
    }

    async fn on_message(&self) -> AdapterResult<IncomingMessage> {
        Err(AdapterError::Unsupported("use polling/event loop".into()))
    }
}
