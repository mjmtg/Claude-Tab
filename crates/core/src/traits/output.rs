//! Output Reader Trait
//!
//! Defines the contract for reading and distributing PTY output.
//! This trait abstracts the output streaming mechanism from consumers.

use async_trait::async_trait;
use std::io::Read;
use tokio::sync::broadcast;

/// A chunk of output data from a session.
#[derive(Debug, Clone)]
pub struct OutputChunk {
    /// Session ID this output belongs to
    pub session_id: String,

    /// The raw output data
    pub data: Vec<u8>,

    /// Timestamp in milliseconds since epoch
    pub timestamp: u64,
}

/// Error types for output operations.
#[derive(Debug, thiserror::Error)]
pub enum OutputError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Reader error: {0}")]
    ReaderError(String),

    #[error("Channel error: {0}")]
    ChannelError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Trait for reading and distributing PTY output.
///
/// Implementations should:
/// - Accept a reader from the PTY
/// - Stream output to subscribers
/// - Buffer recent output for late joiners (optional)
///
/// # Example Implementation
/// ```ignore
/// struct BroadcastOutputStream {
///     sender: broadcast::Sender<OutputChunk>,
/// }
///
/// #[async_trait]
/// impl OutputReader for BroadcastOutputStream {
///     async fn start_reading(&self, session_id: String, reader: Box<dyn Read + Send>) {
///         // Spawn task to read from PTY and broadcast
///     }
///
///     fn subscribe(&self) -> broadcast::Receiver<OutputChunk> {
///         self.sender.subscribe()
///     }
/// }
/// ```
#[async_trait]
pub trait OutputReader: Send + Sync + 'static {
    /// Start reading output from a PTY reader.
    ///
    /// This should spawn a background task that reads from the reader
    /// and broadcasts chunks to subscribers.
    fn start_reading(&self, session_id: String, reader: Box<dyn Read + Send>);

    /// Stop reading output for a session.
    fn stop_reading(&self, session_id: &str);

    /// Subscribe to output from all sessions.
    fn subscribe(&self) -> broadcast::Receiver<OutputChunk>;

    /// Get the broadcast sender for testing/mocking purposes.
    fn sender(&self) -> broadcast::Sender<OutputChunk>;
}

/// Trait for subscribing to output for a specific session.
///
/// This is a filtered version of OutputReader that only
/// delivers output for a single session.
pub trait SessionOutputSubscriber: Send + Sync {
    /// Subscribe to output for a specific session.
    fn subscribe_session(&self, session_id: &str) -> Box<dyn SessionOutputStream>;
}

/// A stream of output for a single session.
#[async_trait]
pub trait SessionOutputStream: Send {
    /// Receive the next chunk of output.
    ///
    /// Returns None if the session has ended or the stream is closed.
    async fn recv(&mut self) -> Option<Vec<u8>>;

    /// Close the output stream.
    fn close(&mut self);
}
