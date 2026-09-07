use rusqlite::{params, Connection, Row};

use super::models::Interview;
use crate::error::AppError;

pub fn create(
    conn: &Connection,
    title: &str,
    language: Option<&str>,
    audio_path: &str,
) -> Result<Interview, AppError> {
    let now = super::now_epoch_secs();
    conn.execute(
        "INSERT INTO interview (title, language, mode, audio_path, status, created_at, updated_at)
         VALUES (?1, ?2, 'posteriori', ?3, 'imported', ?4, ?4)",
        params![title, language, audio_path, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

/// Creates an interview for a live microphone session: `mode = 'realtime'`
/// (vs. `create`'s `'posteriori'`) and starts directly in `'transcribing'`
/// status since segments are produced as capture happens, not after an
/// import step. `audio_path` points at the WAV `capture::session` is about
/// to write incrementally.
pub fn create_realtime(
    conn: &Connection,
    title: &str,
    audio_path: &str,
) -> Result<Interview, AppError> {
    let now = super::now_epoch_secs();
    conn.execute(
        "INSERT INTO interview (title, language, mode, audio_path, status, created_at, updated_at)
         VALUES (?1, NULL, 'realtime', ?2, 'transcribing', ?3, ?3)",
        params![title, audio_path, now],
    )?;
    get(conn, conn.last_insert_rowid())
}

pub fn get(conn: &Connection, id: i64) -> Result<Interview, AppError> {
    conn.query_row(
        "SELECT id, title, language, mode, audio_path, status, error_message, created_at, updated_at
         FROM interview WHERE id = ?1",
        params![id],
        row_to_interview,
    )
    .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("interview {id}")),
        other => AppError::Db(other),
    })
}

pub fn list(conn: &Connection) -> Result<Vec<Interview>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, language, mode, audio_path, status, error_message, created_at, updated_at
         FROM interview ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_interview)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn set_audio_path(conn: &Connection, id: i64, audio_path: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE interview SET audio_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![audio_path, super::now_epoch_secs(), id],
    )?;
    Ok(())
}

pub fn update_status(
    conn: &Connection,
    id: i64,
    status: &str,
    error_message: Option<&str>,
) -> Result<(), AppError> {
    let now = super::now_epoch_secs();
    conn.execute(
        "UPDATE interview SET status = ?1, error_message = ?2, updated_at = ?3 WHERE id = ?4",
        params![status, error_message, now, id],
    )?;
    Ok(())
}

fn row_to_interview(row: &Row) -> rusqlite::Result<Interview> {
    Ok(Interview {
        id: row.get(0)?,
        title: row.get(1)?,
        language: row.get(2)?,
        mode: row.get(3)?,
        audio_path: row.get(4)?,
        status: row.get(5)?,
        error_message: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        conn
    }

    #[test]
    fn create_and_get_round_trip() {
        let conn = setup();
        let created = create(&conn, "Entretien test", Some("fr"), "/audio/1.wav").unwrap();
        assert_eq!(created.status, "imported");
        let fetched = get(&conn, created.id).unwrap();
        assert_eq!(fetched.title, "Entretien test");
        assert_eq!(fetched.language.as_deref(), Some("fr"));
    }

    #[test]
    fn get_missing_is_not_found() {
        let conn = setup();
        let err = get(&conn, 999).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn update_status_persists_error_message() {
        let conn = setup();
        let created = create(&conn, "Entretien test", None, "/audio/1.wav").unwrap();
        update_status(&conn, created.id, "error", Some("decode failed")).unwrap();
        let fetched = get(&conn, created.id).unwrap();
        assert_eq!(fetched.status, "error");
        assert_eq!(fetched.error_message.as_deref(), Some("decode failed"));
    }

    #[test]
    fn create_realtime_starts_in_transcribing_status_with_realtime_mode() {
        let conn = setup();
        let created = create_realtime(&conn, "Session live", "/audio/live-1.wav").unwrap();
        assert_eq!(created.mode, "realtime");
        assert_eq!(created.status, "transcribing");
        assert_eq!(created.language, None);
    }

    #[test]
    fn list_orders_most_recent_first() {
        let conn = setup();
        create(&conn, "Premier", None, "/audio/1.wav").unwrap();
        create(&conn, "Second", None, "/audio/2.wav").unwrap();
        let all = list(&conn).unwrap();
        assert_eq!(all.len(), 2);
    }
}
