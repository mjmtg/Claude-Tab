//! Data models for Claude Code session storage.

use serde::{Deserialize, Serialize};

/// Metadata for a Claude Code session (read from JSONL files).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub session_id: String,
    pub project_path: String,
    pub jsonl_path: String,
    pub first_prompt: Option<String>,
    pub summary: Option<String>,
    pub message_count: i32,
    pub created_at: String,
    pub modified_at: String,
    pub git_branch: Option<String>,
}

/// User preferences for a project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryPreference {
    pub project_path: String,
    pub pinned: bool,
    pub hidden: bool,
    pub display_name: Option<String>,
}

/// Persisted metadata for a Claude session, keyed by `claude_session_id`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub claude_session_id: String,
    pub project_path: String,
    pub custom_title: Option<String>,
    pub user_set_title: bool,
    pub generated_title: Option<String>,
    pub hidden: bool,
    pub previous_session_id: Option<String>,
    pub last_known_state: Option<String>,
    pub last_state_change_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Filter options for listing sessions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SessionFilter {
    pub project_path: Option<String>,
    pub pinned_only: bool,
    pub include_hidden: bool,
    pub search_query: Option<String>,
    pub limit: usize,
    pub offset: usize,
}

/// Inner message payload (the API response or user message).
///
/// In Claude Code JSONL files, user/assistant entries wrap the actual
/// role + content inside a `message` field:
/// ```json
/// { "type": "user", "message": { "role": "user", "content": "..." }, ... }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagePayload {
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
    #[serde(default)]
    pub model: Option<String>,
}

/// A message from a Claude Code session JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    /// The nested message payload (contains role + content for user/assistant).
    #[serde(default)]
    pub message: Option<MessagePayload>,
    pub timestamp: Option<String>,
    pub cwd: Option<String>,
    #[serde(rename = "gitBranch")]
    pub git_branch: Option<String>,
    #[serde(rename = "sessionId")]
    pub session_id: Option<String>,
    pub uuid: Option<String>,
    #[serde(rename = "parentUuid")]
    pub parent_uuid: Option<String>,
    pub summary: Option<String>,
}

impl SessionMessage {
    /// Get the role, extracting from the nested message payload.
    pub fn role(&self) -> Option<&str> {
        self.message.as_ref().and_then(|m| m.role.as_deref())
    }

    /// Get the content, extracting from the nested message payload.
    pub fn content(&self) -> Option<&serde_json::Value> {
        self.message.as_ref().and_then(|m| m.content.as_ref())
    }
}
