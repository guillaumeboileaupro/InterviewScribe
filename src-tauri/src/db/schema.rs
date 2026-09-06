use rusqlite::Connection;

use crate::error::AppError;

pub fn init(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS interview (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            language TEXT,
            mode TEXT NOT NULL CHECK (mode IN ('posteriori','realtime')),
            audio_path TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN ('imported','transcribing','transcribed','error')),
            error_message TEXT,
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
            created_at TEXT NOT NULL
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
}
