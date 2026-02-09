use rusqlite::Connection;
use tracing::info;

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    // Drop old tables to rebuild from scratch
    conn.execute_batch(
        "
        DROP TABLE IF EXISTS session_metadata;
        DROP TABLE IF EXISTS directory_preferences;
        ",
    )?;

    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS directory_preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_path TEXT NOT NULL UNIQUE,
            pinned INTEGER DEFAULT 0,
            hidden INTEGER DEFAULT 0,
            display_name TEXT
        );

        CREATE TABLE IF NOT EXISTS session_metadata (
            claude_session_id TEXT PRIMARY KEY,
            project_path TEXT NOT NULL DEFAULT '',
            custom_title TEXT,
            user_set_title INTEGER DEFAULT 0,
            generated_title TEXT,
            hidden INTEGER DEFAULT 0,
            previous_session_id TEXT,
            last_known_state TEXT,
            last_state_change_at TEXT,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE INDEX IF NOT EXISTS idx_session_metadata_updated ON session_metadata(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_session_metadata_project ON session_metadata(project_path);
        CREATE INDEX IF NOT EXISTS idx_session_previous ON session_metadata(previous_session_id);
        ",
    )?;

    info!("Storage schema initialized");
    Ok(())
}
