//! Session Buffer Provider Trait
//!
//! Defines the contract for per-session output buffering.
//! This trait abstracts the buffer management from consumers.

use async_trait::async_trait;

/// Error types for buffer operations.
#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Buffer overflow")]
    Overflow,

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Trait for managing per-session output buffers.
///
/// Implementations should:
/// - Create/destroy buffers per session
/// - Append output data to session buffers
/// - Track user input to determine archivability
/// - Provide access to buffer contents
///
/// # Example Implementation
/// ```ignore
/// struct InMemoryBufferProvider {
///     buffers: Arc<RwLock<HashMap<String, SessionBuffer>>>,
/// }
///
/// #[async_trait]
/// impl SessionBufferProvider for InMemoryBufferProvider {
///     async fn create(&self, session_id: &str) {
///         // Create new buffer entry
///     }
///
///     async fn append(&self, session_id: &str, data: &[u8]) {
///         // Append to buffer
///     }
/// }
/// ```
#[async_trait]
pub trait SessionBufferProvider: Send + Sync + 'static {
    /// Create a new buffer for a session.
    async fn create(&self, session_id: &str);

    /// Remove a session's buffer.
    async fn remove(&self, session_id: &str);

    /// Append data to a session's buffer.
    async fn append(&self, session_id: &str, data: &[u8]);

    /// Mark that the session has received user input.
    ///
    /// This affects whether the session should be archived on close.
    async fn mark_user_input(&self, session_id: &str);

    /// Check if the session has received user input.
    async fn had_user_input(&self, session_id: &str) -> bool;

    /// Get the buffer contents for a session.
    async fn get_buffer(&self, session_id: &str) -> Option<Vec<u8>>;

    /// Get the buffer size for a session.
    async fn get_size(&self, session_id: &str) -> Option<usize>;

    /// Clear a session's buffer (but don't remove it).
    async fn clear(&self, session_id: &str);
}
