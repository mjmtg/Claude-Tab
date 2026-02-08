//! Storage traits for Claude Code session indexing.

use async_trait::async_trait;

use crate::models::{ClaudeSession, DirectoryPreference, SessionFilter, SessionMessage};

pub type StorageResult<T> = Result<T, crate::StorageError>;

/// Storage backend for Claude Code session metadata.
#[async_trait]
pub trait StorageBackend: Send + Sync + 'static {
    // -------------------------------------------------------------------------
    // Session reading (from Claude's sessions-index.json files)
    // -------------------------------------------------------------------------

    /// List sessions with optional filtering.
    /// Reads directly from Claude's sessions-index.json files.
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
}
