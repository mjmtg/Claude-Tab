//! SQLite storage backend implementation.
//!
//! Sessions are read directly from JSONL files on disk.
//! SQLite is used only for user preferences and session metadata.

use async_trait::async_trait;
use rusqlite::Connection;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tracing::debug;

use crate::migrations::run_migrations;
use crate::models::{ClaudeSession, DirectoryPreference, SessionFilter, SessionMessage, SessionMetadata};
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

        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;",
        )
        .map_err(|e| StorageError::Init(format!("Failed to set pragmas: {}", e)))?;

        run_migrations(&conn)
            .map_err(|e| StorageError::Init(format!("Migration failed: {}", e)))?;

        let scanner = SessionScanner::new()
            .map_err(|e| StorageError::Init(e))?;

        debug!(path = %expanded, "Storage database initialized");

        Ok(Self {
            conn: Mutex::new(conn),
            scanner,
        })
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn list_sessions(&self, filter: SessionFilter) -> StorageResult<Vec<ClaudeSession>> {
        // Read all sessions directly from JSONL files
        let mut sessions = self.scanner.list_all_sessions();

        // Apply filters
        if let Some(ref project_path) = filter.project_path {
            sessions.retain(|s| s.project_path == *project_path);
        }

        if let Some(ref query) = filter.search_query {
            let query_lower = query.to_lowercase();
            sessions.retain(|s| {
                s.first_prompt.as_ref().map(|p| p.to_lowercase().contains(&query_lower)).unwrap_or(false)
                    || s.project_path.to_lowercase().contains(&query_lower)
            });
        }

        // Enrich with DB metadata (custom titles, hidden status)
        if !filter.include_hidden {
            let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
            let hidden_ids: std::collections::HashSet<String> = {
                let mut stmt = conn.prepare(
                    "SELECT claude_session_id FROM session_metadata WHERE hidden = 1"
                ).map_err(|e| StorageError::Query(e.to_string()))?;
                let rows = stmt.query_map([], |row| row.get::<_, String>(0))
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                rows.filter_map(|r| r.ok()).collect()
            };
            sessions.retain(|s| !hidden_ids.contains(&s.session_id));
        }

        // Apply limit
        let limit = if filter.limit > 0 { filter.limit } else { 100 };
        if sessions.len() > limit {
            sessions.truncate(limit);
        }

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
        let tx = conn.unchecked_transaction()
            .map_err(|e| StorageError::Query(e.to_string()))?;
        tx.execute(
            "INSERT OR REPLACE INTO directory_preferences (project_path, pinned, hidden, display_name)
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![
                pref.project_path,
                pref.pinned as i32,
                pref.hidden as i32,
                pref.display_name,
            ],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        tx.commit().map_err(|e| StorageError::Query(e.to_string()))?;
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

    async fn get_session_metadata(&self, claude_session_id: &str) -> StorageResult<Option<SessionMetadata>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        let mut stmt = conn
            .prepare(
                "SELECT claude_session_id, project_path, custom_title, user_set_title,
                        generated_title, hidden, previous_session_id,
                        last_known_state, last_state_change_at,
                        created_at, updated_at
                 FROM session_metadata WHERE claude_session_id = ?1",
            )
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let result = stmt.query_row(rusqlite::params![claude_session_id], |row| {
            Ok(row_to_session_metadata(row))
        });

        match result {
            Ok(meta) => Ok(Some(meta)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(StorageError::Query(e.to_string())),
        }
    }

    async fn upsert_session_metadata(&self, metadata: &SessionMetadata) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT INTO session_metadata
             (claude_session_id, project_path, custom_title, user_set_title,
              generated_title, hidden, previous_session_id,
              last_known_state, last_state_change_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, datetime('now'))
             ON CONFLICT(claude_session_id) DO UPDATE SET
                project_path = excluded.project_path,
                custom_title = excluded.custom_title,
                user_set_title = excluded.user_set_title,
                generated_title = excluded.generated_title,
                hidden = excluded.hidden,
                previous_session_id = excluded.previous_session_id,
                last_known_state = excluded.last_known_state,
                last_state_change_at = excluded.last_state_change_at,
                updated_at = datetime('now')",
            rusqlite::params![
                metadata.claude_session_id,
                metadata.project_path,
                metadata.custom_title,
                metadata.user_set_title as i32,
                metadata.generated_title,
                metadata.hidden as i32,
                metadata.previous_session_id,
                metadata.last_known_state,
                metadata.last_state_change_at,
                metadata.created_at,
            ],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn set_session_hidden(&self, claude_session_id: &str, hidden: bool) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT INTO session_metadata (claude_session_id, hidden)
             VALUES (?1, ?2)
             ON CONFLICT(claude_session_id) DO UPDATE SET
                hidden = excluded.hidden,
                updated_at = datetime('now')",
            rusqlite::params![claude_session_id, hidden as i32],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn set_generated_title(&self, claude_session_id: &str, title: &str) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT INTO session_metadata (claude_session_id, generated_title)
             VALUES (?1, ?2)
             ON CONFLICT(claude_session_id) DO UPDATE SET
                generated_title = excluded.generated_title,
                updated_at = datetime('now')",
            rusqlite::params![claude_session_id, title],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn link_session(&self, new_session_id: &str, previous_session_id: &str) -> StorageResult<()> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;
        conn.execute(
            "INSERT INTO session_metadata (claude_session_id, previous_session_id)
             VALUES (?1, ?2)
             ON CONFLICT(claude_session_id) DO UPDATE SET
                previous_session_id = excluded.previous_session_id,
                updated_at = datetime('now')",
            rusqlite::params![new_session_id, previous_session_id],
        ).map_err(|e| StorageError::Query(e.to_string()))?;
        Ok(())
    }

    async fn get_session_chain(&self, claude_session_id: &str) -> StorageResult<Vec<SessionMetadata>> {
        let conn = self.conn.lock().map_err(|e| StorageError::Lock(e.to_string()))?;

        // Walk backward from current session through previous_session_id links
        let mut chain = Vec::new();
        let mut current_id = claude_session_id.to_string();

        loop {
            let mut stmt = conn
                .prepare(
                    "SELECT claude_session_id, project_path, custom_title, user_set_title,
                            generated_title, hidden, previous_session_id,
                            last_known_state, last_state_change_at,
                            created_at, updated_at
                     FROM session_metadata WHERE claude_session_id = ?1",
                )
                .map_err(|e| StorageError::Query(e.to_string()))?;

            let result = stmt.query_row(rusqlite::params![current_id], |row| {
                Ok(row_to_session_metadata(row))
            });

            match result {
                Ok(meta) => {
                    let prev = meta.previous_session_id.clone();
                    chain.push(meta);
                    match prev {
                        Some(prev_id) if !prev_id.is_empty() => {
                            current_id = prev_id;
                        }
                        _ => break,
                    }
                }
                Err(rusqlite::Error::QueryReturnedNoRows) => break,
                Err(e) => return Err(StorageError::Query(e.to_string())),
            }
        }

        // Reverse so oldest is first
        chain.reverse();
        Ok(chain)
    }
}

fn row_to_session_metadata(row: &rusqlite::Row) -> SessionMetadata {
    SessionMetadata {
        claude_session_id: row.get(0).unwrap_or_default(),
        project_path: row.get(1).unwrap_or_default(),
        custom_title: row.get(2).ok().and_then(|v: Option<String>| v),
        user_set_title: row.get::<_, i32>(3).unwrap_or(0) != 0,
        generated_title: row.get(4).ok().and_then(|v: Option<String>| v),
        hidden: row.get::<_, i32>(5).unwrap_or(0) != 0,
        previous_session_id: row.get(6).ok().and_then(|v: Option<String>| v),
        last_known_state: row.get(7).ok().and_then(|v: Option<String>| v),
        last_state_change_at: row.get(8).ok().and_then(|v: Option<String>| v),
        created_at: row.get(9).unwrap_or_default(),
        updated_at: row.get(10).unwrap_or_default(),
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
