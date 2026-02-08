//! Data models for Claude Code session storage.

use serde::{Deserialize, Serialize};

/// Metadata for a Claude Code session (read from Claude's sessions-index.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeSession {
    pub session_id: String,
    pub project_path: String,
    pub jsonl_path: String,
    pub first_prompt: Option<String>,
    pub summary: Option<String>,  // Claude's auto-generated title
    pub message_count: i32,
    pub created_at: String,
    pub modified_at: String,
    pub git_branch: Option<String>,
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

/// User preferences for a project directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryPreference {
    pub project_path: String,
    pub pinned: bool,
    pub hidden: bool,
    pub display_name: Option<String>,
}

/// A message from a Claude Code session JSONL file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: Option<String>,
    pub content: Option<serde_json::Value>,
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
