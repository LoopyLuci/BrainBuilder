use async_trait::async_trait;
use crate::adapter::{AdapterResult, PlatformAdapter};
use crate::types::{BotConfig, IncomingMessage, OutgoingMessage};

pub struct DiscordAdapter;

#[cfg(feature = "discord")]
impl DiscordAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
#[cfg(feature = "discord")]
impl PlatformAdapter for DiscordAdapter {
    fn platform(&self) -> &'static str {
        "discord"
    }

    async fn start(&self, _config: BotConfig) -> AdapterResult<()> {
        Err(crate::adapter::AdapterError::Unsupported("discord adapter not implemented".into()))
    }

    async fn stop(&self) -> AdapterResult<()> {
        Ok(())
    }

    async fn send(&self, _message: OutgoingMessage) -> AdapterResult<()> {
        Err(crate::adapter::AdapterError::Unsupported("discord adapter not implemented".into()))
    }

    async fn on_message(&self) -> AdapterResult<IncomingMessage> {
        Err(crate::adapter::AdapterError::Unsupported("discord adapter not implemented".into()))
    }
}
