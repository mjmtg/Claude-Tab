//! Session Archiver Trait
//!
//! Defines the contract for archiving terminated sessions.
//! This trait abstracts the persistence mechanism from the session lifecycle.

use crate::session::Session;
use async_trait::async_trait;

/// Archived session data structure.
///
/// This is a minimal representation for the trait interface.
/// Implementations may extend this with additional fields.
#[derive(Debug, Clone)]
pub struct ArchivedSessionData {
    /// Unique identifier for the archived session
    pub id: String,

    /// Optional Claude session ID for resume functionality
    pub claude_session_id: Option<String>,

    /// Provider that created this session
    pub provider_id: String,

    /// Session title
    pub title: String,

    /// Working directory
    pub working_directory: Option<String>,

    /// ISO-8601 timestamp when session was created
    pub created_at: String,

    /// ISO-8601 timestamp when session ended
    pub ended_at: Option<String>,

    /// Transcript content (ANSI stripped)
    pub transcript: String,
}

/// Search result for history queries.
#[derive(Debug, Clone)]
pub struct SearchResultData {
    /// Session ID containing the match
    pub session_id: String,

    /// Session title
    pub session_title: String,

    /// Snippet of matching text with context
    pub snippet: String,
}

/// Error types for archiver operations.
#[derive(Debug, thiserror::Error)]
pub enum ArchiverError {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Trait for archiving and retrieving terminated sessions.
///
/// Implementations should:
/// - Store session content persistently
/// - Strip ANSI escape codes from transcripts
/// - Support full-text search on transcripts
/// - Handle session metadata (claude_session_id, etc.)
///
/// # Example Implementation
/// ```ignore
/// struct SqliteArchiver {
///     db: Pool<Sqlite>,
/// }
///
/// #[async_trait]
/// impl SessionArchiver for SqliteArchiver {
///     async fn archive(&self, session: &Session, transcript: &[u8]) -> Result<(), ArchiverError> {
///         // Strip ANSI, store in SQLite
///     }
///
///     async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResultData>, ArchiverError> {
///         // Full-text search
///     }
/// }
/// ```
#[async_trait]
pub trait SessionArchiver: Send + Sync + 'static {
    /// Archive a terminated session with its transcript.
    ///
    /// The transcript should be the raw PTY output buffer.
    /// Implementations should strip ANSI codes before storing.
    async fn archive(&self, session: &Session, transcript: &[u8]) -> Result<(), ArchiverError>;

    /// Archive a session using a pre-built archived session data structure.
    /// This is useful when the transcript has already been processed.
    async fn archive_session(&self, session: &Session);

    /// Search archived session transcripts.
    async fn search(&self, query: &str, limit: usize) -> Result<Vec<SearchResultData>, ArchiverError>;

    /// List archived sessions with pagination.
    async fn list(&self, limit: usize, offset: usize) -> Result<Vec<ArchivedSessionData>, ArchiverError>;

    /// Get a specific archived session by ID.
    async fn get(&self, session_id: &str) -> Result<Option<ArchivedSessionData>, ArchiverError>;

    /// Delete an archived session.
    async fn delete(&self, session_id: &str) -> Result<(), ArchiverError>;

    /// Check if a session exists in the archive.
    async fn exists(&self, session_id: &str) -> Result<bool, ArchiverError>;
}
