use rusqlite::Connection;

use crate::error::AppError;

const SCHEMA_VERSION: i64 = 2;

pub fn init(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS interview (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            notes TEXT,
            language TEXT,
            mode TEXT NOT NULL CHECK (mode IN ('posteriori','realtime')),
            audio_path TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('imported','transcribing','transcribed','error')),
            error_message TEXT,
            transcription_cursor_ms INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS speaker (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            interview_id INTEGER NOT NULL REFERENCES interview(id) ON DELETE CASCADE,
            label TEXT NOT NULL,
            color TEXT NOT NULL,
            display_name TEXT
        );

        CREATE TABLE IF NOT EXISTS segment (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            interview_id INTEGER NOT NULL REFERENCES interview(id) ON DELETE CASCADE,
            speaker_id INTEGER REFERENCES speaker(id) ON DELETE SET NULL,
            start_ms INTEGER NOT NULL,
            end_ms INTEGER NOT NULL,
            raw_text TEXT NOT NULL,
            confidence REAL,
            status TEXT NOT NULL DEFAULT 'raw' CHECK (status IN ('raw','uncertain'))
        );

        CREATE TABLE IF NOT EXISTS edit (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            segment_id INTEGER NOT NULL REFERENCES segment(id) ON DELETE CASCADE,
            operation TEXT NOT NULL,
            before_text TEXT NOT NULL,
            after_text TEXT NOT NULL,
            created_at TEXT NOT NULL,
            reverted_at TEXT
        );

        CREATE TABLE IF NOT EXISTS setting (
            interview_id INTEGER PRIMARY KEY REFERENCES interview(id) ON DELETE CASCADE,
            show_timestamps INTEGER NOT NULL DEFAULT 1,
            model_path TEXT NOT NULL,
            language TEXT,
            cleanup_enabled INTEGER NOT NULL DEFAULT 0,
            device TEXT NOT NULL DEFAULT 'cpu'
        );
        ",
    )?;
    migrate(conn)?;
    Ok(())
}

/// Applies additive schema changes for databases created before this version,
/// idempotently. New databases already get the current shape from `init`
/// above (the `ensure_column` calls below are then no-ops), but the index
/// still has to be created *after* the column is guaranteed to exist, since
/// an existing on-disk database predating `reverted_at` would otherwise fail
/// to open at all (`CREATE TABLE IF NOT EXISTS` is a no-op on an existing
/// table, so the column would be missing when the index tried to reference
/// it — this was caught by a real crash against a Phase 1 database, not by
/// the in-memory tests below, which always started from a fresh connection).
fn migrate(conn: &Connection) -> Result<(), AppError> {
    ensure_column(conn, "edit", "reverted_at", "TEXT")?;
    ensure_column(conn, "interview", "notes", "TEXT")?;
    ensure_column(
        conn,
        "interview",
        "transcription_cursor_ms",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_edit_segment ON edit(segment_id, reverted_at)",
        [],
    )?;
    let version: i64 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version < SCHEMA_VERSION {
        conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    }
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, decl: &str) -> Result<(), AppError> {
    let exists = conn
        .prepare(&format!("PRAGMA table_info({table})"))?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == column);
    if !exists {
        conn.execute(
            &format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"),
            [],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_is_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        init(&conn).unwrap();
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        let enabled: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
    }

    #[test]
    fn migrate_adds_reverted_at_to_a_pre_existing_edit_table() {
        let conn = Connection::open_in_memory().unwrap();
        // Simulate a database created before `reverted_at` existed.
        conn.execute_batch(
            "CREATE TABLE edit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                segment_id INTEGER NOT NULL,
                operation TEXT NOT NULL,
                before_text TEXT NOT NULL,
                after_text TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
        .unwrap();

        init(&conn).unwrap();

        let has_column: bool = conn
            .prepare("PRAGMA table_info(edit)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .any(|name| name == "reverted_at");
        assert!(has_column);

        // Running it again must not error (idempotent).
        init(&conn).unwrap();
    }

    #[test]
    fn migrate_adds_notes_to_a_pre_existing_interview_table() {
        let conn = Connection::open_in_memory().unwrap();
        // Simulate a database created before `notes` existed.
        conn.execute_batch(
            "CREATE TABLE interview (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                language TEXT,
                mode TEXT NOT NULL,
                audio_path TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .unwrap();

        init(&conn).unwrap();

        let has_column: bool = conn
            .prepare("PRAGMA table_info(interview)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .any(|name| name == "notes");
        assert!(has_column);

        // Running it again must not error (idempotent).
        init(&conn).unwrap();
    }

    #[test]
    fn migrate_adds_transcription_cursor_to_a_pre_existing_database() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE interview (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                notes TEXT,
                language TEXT,
                mode TEXT NOT NULL,
                audio_path TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )
        .unwrap();

        init(&conn).unwrap();

        let has_cursor = conn
            .prepare("PRAGMA table_info(interview)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .any(|name| name == "transcription_cursor_ms");
        assert!(has_cursor);
    }

    #[test]
    fn init_succeeds_against_a_pre_existing_phase1_database() {
        // Regression test: reproduces the exact shape of a database created
        // by Phase 1 (before `reverted_at` existed), then calls the real
        // `init()` entry point end-to-end, the same way `db::open` does when
        // the app starts against an existing on-disk file. This previously
        // crashed the whole app on startup because the index on `edit`
        // was created before the migration added the missing column.
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "
            CREATE TABLE interview (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                title TEXT NOT NULL,
                language TEXT,
                mode TEXT NOT NULL,
                audio_path TEXT NOT NULL,
                status TEXT NOT NULL,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE segment (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                interview_id INTEGER NOT NULL,
                speaker_id INTEGER,
                start_ms INTEGER NOT NULL,
                end_ms INTEGER NOT NULL,
                raw_text TEXT NOT NULL,
                confidence REAL,
                status TEXT NOT NULL DEFAULT 'raw'
            );
            CREATE TABLE edit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                segment_id INTEGER NOT NULL,
                operation TEXT NOT NULL,
                before_text TEXT NOT NULL,
                after_text TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            ",
        )
        .unwrap();

        init(&conn).unwrap();

        let has_column: bool = conn
            .prepare("PRAGMA table_info(edit)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .filter_map(Result::ok)
            .any(|name| name == "reverted_at");
        assert!(has_column);
    }

    #[test]
    fn user_version_is_set_after_init() {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        let version: i64 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, SCHEMA_VERSION);
    }
}
