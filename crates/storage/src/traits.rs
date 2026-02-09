//! Storage traits for Claude Code session indexing.

use async_trait::async_trait;

use crate::models::{ClaudeSession, DirectoryPreference, SessionFilter, SessionMessage, SessionMetadata};

pub type StorageResult<T> = Result<T, crate::StorageError>;

/// Storage backend for Claude Code session metadata.
#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    // -------------------------------------------------------------------------
    // Session reading (from JSONL files)
    // -------------------------------------------------------------------------

    /// List sessions with optional filtering.
    async fn list_sessions(&self, filter: SessionFilter) -> StorageResult<Vec<ClaudeSession>>;

    /// Get a specific session by ID.
    async fn get_session(&self, session_id: &str) -> StorageResult<Option<ClaudeSession>>;

    /// Read full conversation content from a session's JSONL file.
    async fn get_session_content(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>>;

    // -------------------------------------------------------------------------
    // Directory preferences (persisted in SQLite)
    // -------------------------------------------------------------------------

    /// Set preferences for a project directory (pin/hide/rename).
    async fn set_directory_preference(&self, pref: DirectoryPreference) -> StorageResult<()>;

    /// Get all directory preferences.
    async fn get_directory_preferences(&self) -> StorageResult<Vec<DirectoryPreference>>;

    /// Remove preferences for a directory.
    async fn remove_directory_preference(&self, project_path: &str) -> StorageResult<()>;

    // -------------------------------------------------------------------------
    // Session metadata (persisted in SQLite)
    // -------------------------------------------------------------------------

    /// Get metadata for a specific Claude session.
    async fn get_session_metadata(&self, claude_session_id: &str) -> StorageResult<Option<SessionMetadata>>;

    /// Insert or update session metadata.
    async fn upsert_session_metadata(&self, metadata: &SessionMetadata) -> StorageResult<()>;

    /// Set or clear hidden flag for a session.
    async fn set_session_hidden(&self, claude_session_id: &str, hidden: bool) -> StorageResult<()>;

    /// Set the Haiku-generated title for a session.
    async fn set_generated_title(&self, claude_session_id: &str, title: &str) -> StorageResult<()>;

    /// Link a new session to a previous one (for /clear chaining).
    async fn link_session(&self, new_session_id: &str, previous_session_id: &str) -> StorageResult<()>;

    /// Get the full chain of linked sessions.
    async fn get_session_chain(&self, claude_session_id: &str) -> StorageResult<Vec<SessionMetadata>>;
}
