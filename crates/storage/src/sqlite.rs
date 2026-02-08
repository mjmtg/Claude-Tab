//! SQLite storage backend implementation.
//!
//! This backend reads sessions directly from Claude's sessions-index.json files
//! and only uses SQLite for storing user preferences (pinned/hidden directories).

use async_trait::async_trait;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::debug;

use crate::migrations::run_migrations;
use crate::models::{ClaudeSession, DirectoryPreference, SessionFilter, SessionMessage};
use crate::reader::SessionReader;
use crate::scanner::SessionScanner;
use crate::traits::{StorageBackend, StorageResult};
use crate::StorageError;

pub struct SqliteBackend {
    conn: Mutex<Connection>,
    scanner: SessionScanner,
}

impl SqliteBackend {
    pub fn new(path: &str) -> Result<Self, StorageError> {
        let expanded = shellexpand(path);
        if let Some(parent) = PathBuf::from(&expanded).parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| StorageError::Init(format!("Failed to create db directory: {}", e)))?;
        }

        let conn = Connection::open(&expanded)
            .map_err(|e| StorageError::Init(format!("Failed to open database: {}", e)))?;

        conn.execute_batch("PRAGMA journal_mode=WAL;")
            .map_err(|e| StorageError::Init(format!("Failed to set pragmas: {}", e)))?;

        run_migrations(&conn)
            .map_err(|e| StorageError::Init(format!("Migration failed: {}", e)))?;

        debug!(path = %expanded, "Storage database initialized");

        Ok(Self {
            conn: Mutex::new(conn),
            scanner: SessionScanner::new(),
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn list_sessions(&self, filter: SessionFilter) -> StorageResult<Vec<ClaudeSession>> {
        // Read all sessions directly from Claude's files
        let mut sessions = self.scanner.list_all_sessions();

        // Get preferences for filtering
        let preferences = self.get_directory_preferences().await?;
        let hidden_projects: std::collections::HashSet<_> = preferences
            .iter()
            .filter(|p| p.hidden)
            .map(|p| p.project_path.as_str())
            .collect();
        let pinned_projects: std::collections::HashSet<_> = preferences
            .iter()
            .filter(|p| p.pinned)
            .map(|p| p.project_path.as_str())
            .collect();

        // Apply filters
        if let Some(ref project_path) = filter.project_path {
            sessions.retain(|s| &s.project_path == project_path);
        }

        if filter.pinned_only {
            sessions.retain(|s| pinned_projects.contains(s.project_path.as_str()));
        }

        if !filter.include_hidden {
            sessions.retain(|s| !hidden_projects.contains(s.project_path.as_str()));
        }

        if let Some(ref query) = filter.search_query {
            let query_lower = query.to_lowercase();
            sessions.retain(|s| {
                s.first_prompt.as_ref().map(|p| p.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || s.summary.as_ref().map(|p| p.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || s.project_path.to_lowercase().contains(&query_lower)
            });
        }

        // Apply pagination
        let offset = filter.offset;
        let limit = if filter.limit > 0 { filter.limit } else { 100 };

        if offset > 0 {
            sessions = sessions.into_iter().skip(offset).collect();
        }
        sessions.truncate(limit);

        Ok(sessions)
    }

    async fn get_session(&self, session_id: &str) -> StorageResult<Option<ClaudeSession>> {
        Ok(self.scanner.find_session(session_id))
    }

    async fn get_session_content(&self, session_id: &str) -> StorageResult<Vec<SessionMessage>> {
        let session = self.get_session(session_id).await?
            .ok_or_else(|| StorageError::Query(format!("Session not found: {}", session_id)))?;

        let path = Path::new(&session.jsonl_path);
        SessionReader::read_session(path)
            .map_err(|e| StorageError::Query(format!("Failed to read session file: {}", e)))
    }

    async fn set_directory_preference(&self, pref: DirectoryPreference) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT OR REPLACE INTO directory_preferences (project_path, pinned, hidden, display_name)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                pref.project_path,
                pref.pinned as i32,
                pref.hidden as i32,
                pref.display_name,
            ],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn get_directory_preferences(&self) -> StorageResult<Vec<DirectoryPreference>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        let mut stmt = conn
            .prepare("SELECT project_path, pinned, hidden, display_name FROM directory_preferences")
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(DirectoryPreference {
                    project_path: row.get(0)?,
                    pinned: row.get::<_, i32>(1)? != 0,
                    hidden: row.get::<_, i32>(2)? != 0,
                    display_name: row.get(3)?,
                })
            })
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut prefs = Vec::new();
        for row in rows {
            prefs.push(row.map_err(|e| StorageError::Query(e.to_string()))?);
        }
        Ok(prefs)
    }

    async fn remove_directory_preference(&self, project_path: &str) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "DELETE FROM directory_preferences WHERE project_path = ?1",
            rusqlite::params![project_path],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }
}

fn shellexpand(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    }
    path.to_string()
}
