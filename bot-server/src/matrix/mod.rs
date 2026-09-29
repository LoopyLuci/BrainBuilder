use async_trait::async_trait;
use crate::adapter::{AdapterResult, PlatformAdapter};
use crate::types::{BotConfig, IncomingMessage, OutgoingMessage};

pub struct MatrixAdapter;

#[cfg(feature = "matrix")]
impl MatrixAdapter {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
#[cfg(feature = "matrix")]
impl PlatformAdapter for MatrixAdapter {
    fn platform(&self) -> &'static str {
        "matrix"
    }

    async fn start(&self, _config: BotConfig) -> AdapterResult<()> {
        Err(crate::adapter::AdapterError::Unsupported("matrix adapter not implemented".into()))
    }

    async fn stop(&self) -> AdapterResult<()> {
        Ok(())
    }

    async fn send(&self, _message: OutgoingMessage) -> AdapterResult<()> {
        Err(crate::adapter::AdapterError::Unsupported("matrix adapter not implemented".into()))
    }

    async fn on_message(&self) -> AdapterResult<IncomingMessage> {
        Err(crate::adapter::AdapterError::Unsupported("matrix adapter not implemented".into()))
    }
}
