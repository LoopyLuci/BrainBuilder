use async_trait::async_trait;
use crate::types::{BotConfig, OutgoingMessage};
use crate::adapter::{AdapterResult, AdapterError, PlatformAdapter};

pub struct EmailAdapter;

impl Default for EmailAdapter { fn default() -> Self { Self } }

#[async_trait]
impl PlatformAdapter for EmailAdapter {
    fn platform(&self) -> &'static str { "email" }

    async fn start(&self, _config: BotConfig) -> Result<(), AdapterError> { Ok(()) }
    async fn stop(&self) -> Result<(), AdapterError> { Ok(()) }

    async fn send(&self, _message: OutgoingMessage) -> Result<(), AdapterError> { Ok(()) }

    async fn on_message(&self) -> AdapterResult<crate::types::IncomingMessage> {
        Err(AdapterError::Unsupported("IMAP polling not wired".into()))
    }
}
