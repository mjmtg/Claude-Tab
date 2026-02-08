use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChannelType {
    UnixSocket,
    Pipe,
    File,
}

#[derive(Debug, thiserror::Error)]
pub enum ChannelError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
}

#[async_trait]
pub trait NotificationChannel: Send + Sync + 'static {
    fn channel_type(&self) -> ChannelType;
    async fn connect(&mut self) -> Result<(), ChannelError>;
    async fn disconnect(&mut self) -> Result<(), ChannelError>;
    async fn send(&self, data: &[u8]) -> Result<(), ChannelError>;
    async fn receive(&self) -> Result<Vec<u8>, ChannelError>;
    fn is_connected(&self) -> bool;
}
