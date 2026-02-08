use rusqlite::Connection;
use tracing::info;

pub fn run_migrations(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version = get_schema_version(conn);

    // Migration to v4: Drop session indexing tables, keep only preferences
    if version < 4 {
        migrate_v4(conn)?;
        set_schema_version(conn, 4)?;
        info!("Storage schema migrated to v4 (simplified: preferences only)");
    }

    Ok(())
}

fn get_schema_version(conn: &Connection) -> i32 {
    conn.pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap_or(0)
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<(), rusqlite::Error> {
    conn.pragma_update(None, "user_version", version)?;
    Ok(())
}

fn migrate_v4(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        -- Drop old session indexing tables (now read from Claude's files directly)
        DROP TABLE IF EXISTS claude_sessions;
        DROP TABLE IF EXISTS scan_state;
        DROP TABLE IF EXISTS archived_sessions;

        -- User preferences for directory filtering/pinning (the only table we need)
        CREATE TABLE IF NOT EXISTS directory_preferences (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            project_path TEXT NOT NULL UNIQUE,
            pinned INTEGER DEFAULT 0,
            hidden INTEGER DEFAULT 0,
            display_name TEXT
        );
        ",
    )?;
    Ok(())
}
