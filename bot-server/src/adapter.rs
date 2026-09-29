use async_trait::async_trait;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("upstream adapter error: {0}")]
    Upstream(String),
    #[error("unsupported operation: {0}")]
    Unsupported(String),
}

pub type AdapterResult<T> = Result<T, AdapterError>;

#[async_trait]
pub trait PlatformAdapter: Send + Sync {
    fn platform(&self) -> &'static str;
    async fn start(&self, config: super::types::BotConfig) -> AdapterResult<()>;
    async fn stop(&self) -> AdapterResult<()>;
    async fn send(&self, message: super::types::OutgoingMessage) -> AdapterResult<()>;
    async fn on_message(&self) -> AdapterResult<super::types::IncomingMessage>;
}
