use async_trait::async_trait;
use crate::types::{BotConfig, OutgoingMessage};
use crate::adapter::{AdapterResult, AdapterError, PlatformAdapter};

pub struct SmsAdapter { _client: Option<twilio::Client> }

impl SmsAdapter { pub fn new() -> Self { Self { _client: None } } }

#[async_trait]
impl PlatformAdapter for SmsAdapter {
    fn platform(&self) -> &'static str { "sms" }

    async fn start(&self, _config: BotConfig) -> Result<(), AdapterError> { Ok(()) }
    async fn stop(&self) -> Result<(), AdapterError> { Ok(()) }

    async fn send(&self, _message: OutgoingMessage) -> Result<(), AdapterError> { Ok(()) }

    async fn on_message(&self) -> AdapterResult<crate::types::IncomingMessage> {
        Err(AdapterError::Unsupported("webhook polling not wired".into()))
    }
}
