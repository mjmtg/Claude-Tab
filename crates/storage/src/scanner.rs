//! Session Scanner
//!
//! Reads Claude Code sessions directly from ~/.claude/projects/*/sessions-index.json files.

use crate::models::ClaudeSession;
use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

/// Scanner for Claude Code session files.
pub struct SessionScanner {
    claude_dir: PathBuf,
}

impl SessionScanner {
    /// Create a new scanner with the default Claude directory (~/.claude).
    pub fn new() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        Self {
            claude_dir: PathBuf::from(home).join(".claude"),
        }
    }

    /// Create a scanner with a custom Claude directory (for testing).
    pub fn with_dir(claude_dir: PathBuf) -> Self {
        Self { claude_dir }
    }

    /// Get the projects directory path.
    pub fn projects_dir(&self) -> PathBuf {
        self.claude_dir.join("projects")
    }

    /// Read all sessions from all projects' sessions-index.json files.
    pub fn list_all_sessions(&self) -> Vec<ClaudeSession> {
        let projects_dir = self.projects_dir();
        if !projects_dir.exists() {
            debug!("Projects directory does not exist: {:?}", projects_dir);
            return Vec::new();
        }

        let mut sessions = Vec::new();
        match fs::read_dir(&projects_dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        let encoded_path = path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string();
                        sessions.extend(self.read_project_sessions(&encoded_path));
                    }
                }
            }
            Err(e) => {
                warn!("Failed to read projects directory: {}", e);
            }
        }

        // Sort by modified_at descending
        sessions.sort_by(|a, b| b.modified_at.cmp(&a.modified_at));
        sessions
    }

    /// Read sessions from a specific project's sessions-index.json.
    fn read_project_sessions(&self, encoded_path: &str) -> Vec<ClaudeSession> {
        let project_dir = self.projects_dir().join(encoded_path);
        let index_path = project_dir.join("sessions-index.json");

        if !index_path.exists() {
            return Vec::new();
        }

        let project_path = decode_path(encoded_path);
        self.parse_sessions_index(&index_path, &project_path, encoded_path)
            .unwrap_or_default()
    }

    /// Parse sessions-index.json file.
    fn parse_sessions_index(
        &self,
        index_path: &Path,
        project_path: &str,
        encoded_path: &str,
    ) -> Option<Vec<ClaudeSession>> {
        let content = fs::read_to_string(index_path).ok()?;
        let index: serde_json::Value = serde_json::from_str(&content).ok()?;

        // sessions-index.json has format: { "version": 1, "entries": [...] }
        let entries = index.get("entries")?.as_array()?;

        let mut sessions = Vec::new();
        let now = Utc::now().to_rfc3339();

        for entry in entries {
            let session_id = entry.get("sessionId")?.as_str()?;

            // Use fullPath from index if available, otherwise construct it
            let jsonl_path = entry.get("fullPath")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    self.projects_dir()
                        .join(encoded_path)
                        .join(format!("{}.jsonl", session_id))
                });

            if !jsonl_path.exists() {
                continue;
            }

            // Get git branch from index (it's camelCase: gitBranch)
            let git_branch = entry.get("gitBranch")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from);

            // Use projectPath from index if available
            let actual_project_path = entry.get("projectPath")
                .and_then(|v| v.as_str())
                .unwrap_or(project_path);

            sessions.push(ClaudeSession {
                session_id: session_id.to_string(),
                project_path: actual_project_path.to_string(),
                jsonl_path: jsonl_path.to_string_lossy().to_string(),
                first_prompt: entry.get("firstPrompt").and_then(|v| v.as_str()).map(String::from),
                summary: entry.get("summary").and_then(|v| v.as_str()).map(String::from),
                message_count: entry.get("messageCount").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
                created_at: entry.get("created").and_then(|v| v.as_str()).unwrap_or(&now).to_string(),
                modified_at: entry.get("modified").and_then(|v| v.as_str()).unwrap_or(&now).to_string(),
                git_branch,
            });
        }

        Some(sessions)
    }

    /// Find a session by ID across all projects.
    pub fn find_session(&self, session_id: &str) -> Option<ClaudeSession> {
        let projects_dir = self.projects_dir();
        if !projects_dir.exists() {
            return None;
        }

        if let Ok(entries) = fs::read_dir(&projects_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let encoded_path = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("")
                        .to_string();

                    let sessions = self.read_project_sessions(&encoded_path);
                    if let Some(session) = sessions.into_iter().find(|s| s.session_id == session_id) {
                        return Some(session);
                    }
                }
            }
        }

        None
    }
}

impl Default for SessionScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Decode Claude's path encoding (- → /).
fn decode_path(encoded: &str) -> String {
    // Claude encodes paths by replacing / with -
    // e.g., "-Users-mjmoshiri-project" → "/Users/mjmoshiri/project"
    if encoded.starts_with('-') {
        encoded.replacen('-', "/", 1).replace('-', "/")
    } else {
        encoded.replace('-', "/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_path() {
        assert_eq!(decode_path("-Users-name-project"), "/Users/name/project");
        assert_eq!(decode_path("Users-name-project"), "Users/name/project");
    }
}
