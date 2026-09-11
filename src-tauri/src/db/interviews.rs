use std::path::Path;

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
        "SELECT id, title, notes, language, mode, audio_path, status, error_message, created_at, updated_at
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
        "SELECT id, title, notes, language, mode, audio_path, status, error_message, created_at, updated_at
         FROM interview ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], row_to_interview)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Returns realtime sessions whose persisted status still says
/// `transcribing` but which are not the recording currently owned by this
/// process. This derived state distinguishes an interrupted session after a
/// restart without rewriting its evidence or pretending it completed.
pub fn list_recovery_candidates(
    conn: &Connection,
    active_interview_id: Option<i64>,
) -> Result<Vec<Interview>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, notes, language, mode, audio_path, status, error_message, created_at, updated_at
         FROM interview
         WHERE mode = 'realtime' AND status = 'transcribing'
           AND (?1 IS NULL OR id != ?1)
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![active_interview_id], row_to_interview)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Posteriori counterpart to `list_recovery_candidates`: imported files
/// whose transcription was stopped partway through (or crashed) rather than
/// completed, and are therefore safe to resume. `active_interview_id`
/// excludes whichever transcription (if any) is running right now, so its
/// own library row never shows a stale "Reprendre" badge while it's already
/// in flight.
pub fn list_resumable_posteriori(
    conn: &Connection,
    active_interview_id: Option<i64>,
) -> Result<Vec<Interview>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title, notes, language, mode, audio_path, status, error_message, created_at, updated_at
         FROM interview
         WHERE mode = 'posteriori' AND status IN ('transcribing', 'error')
           AND (?1 IS NULL OR id != ?1)
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![active_interview_id], row_to_interview)?;
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

pub fn transcription_cursor_ms(conn: &Connection, id: i64) -> Result<i64, AppError> {
    conn.query_row(
        "SELECT transcription_cursor_ms FROM interview WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )
    .map_err(AppError::from)
}

/// Sets the free-text note attached to an interview (distinct from `title`
/// and never touched by transcription/cleanup - purely a user-owned memo).
/// `None`/empty clears it. Editable at any time, not just at creation, since
/// the field is optional and often filled in after the fact.
pub fn update_notes(conn: &Connection, id: i64, notes: Option<&str>) -> Result<(), AppError> {
    let notes = notes.filter(|value| !value.trim().is_empty());
    conn.execute(
        "UPDATE interview SET notes = ?1, updated_at = ?2 WHERE id = ?3",
        params![notes, super::now_epoch_secs(), id],
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

/// Deletes an interview and its private audio copy as one coordinated
/// operation. The database transaction is rolled back if the file cannot be
/// removed. A path outside `audio_dir`, or not named after the interview id,
/// is rejected so a corrupt/legacy row can never delete a user-owned source.
pub fn delete_with_managed_audio(
    conn: &mut Connection,
    id: i64,
    audio_dir: &Path,
) -> Result<(), AppError> {
    let interview = get(conn, id)?;
    let audio_path = Path::new(&interview.audio_path);
    if !is_managed_audio_path(audio_path, audio_dir, id) {
        return Err(AppError::Audio(
            "suppression refusee: le fichier audio n'appartient pas au stockage prive".into(),
        ));
    }

    let tx = conn.transaction()?;
    let deleted = tx.execute("DELETE FROM interview WHERE id = ?1", params![id])?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("interview {id}")));
    }
    if audio_path.exists() {
        std::fs::remove_file(audio_path)?;
    }
    tx.commit()?;
    Ok(())
}

/// Checks that an audio path belongs to this application's private storage
/// and is named after its interview. Callers must use this before reading or
/// deleting a path persisted in SQLite: legacy or corrupt rows may otherwise
/// point at an arbitrary user-owned file.
pub(crate) fn is_managed_audio_path(audio_path: &Path, audio_dir: &Path, id: i64) -> bool {
    let expected_stem = id.to_string();
    audio_path.parent() == Some(audio_dir)
        && audio_path.file_stem().and_then(|stem| stem.to_str()) == Some(expected_stem.as_str())
}

fn row_to_interview(row: &Row) -> rusqlite::Result<Interview> {
    Ok(Interview {
        id: row.get(0)?,
        title: row.get(1)?,
        notes: row.get(2)?,
        language: row.get(3)?,
        mode: row.get(4)?,
        audio_path: row.get(5)?,
        status: row.get(6)?,
        error_message: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
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
    fn update_notes_persists_and_clears() {
        let conn = setup();
        let created = create(&conn, "Entretien test", None, "/audio/1.wav").unwrap();
        assert_eq!(get(&conn, created.id).unwrap().notes, None);

        update_notes(&conn, created.id, Some("Rappeler le contexte")).unwrap();
        assert_eq!(
            get(&conn, created.id).unwrap().notes.as_deref(),
            Some("Rappeler le contexte")
        );

        update_notes(&conn, created.id, Some("   ")).unwrap();
        assert_eq!(get(&conn, created.id).unwrap().notes, None);
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

    #[test]
    fn recovery_candidates_only_include_inactive_interrupted_realtime_sessions() {
        let conn = setup();
        let interrupted = create_realtime(&conn, "Interrompu", "/audio/1.wav").unwrap();
        let active = create_realtime(&conn, "Actif", "/audio/2.wav").unwrap();
        create(&conn, "Import normal", None, "/audio/3.wav").unwrap();
        let finished = create_realtime(&conn, "Termine", "/audio/4.wav").unwrap();
        update_status(&conn, finished.id, "transcribed", None).unwrap();

        let candidates = list_recovery_candidates(&conn, Some(active.id)).unwrap();

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].id, interrupted.id);
    }

    #[test]
    fn resumable_posteriori_excludes_realtime_and_the_active_transcription() {
        let conn = setup();
        let stopped = create(&conn, "Import arrete", None, "/audio/1.wav").unwrap();
        update_status(&conn, stopped.id, "transcribing", None).unwrap();
        let currently_running = create(&conn, "Import en cours", None, "/audio/2.wav").unwrap();
        update_status(&conn, currently_running.id, "transcribing", None).unwrap();
        create_realtime(&conn, "Session live interrompue", "/audio/3.wav").unwrap();
        let finished = create(&conn, "Import termine", None, "/audio/4.wav").unwrap();
        update_status(&conn, finished.id, "transcribed", None).unwrap();
        let retryable_error = create(&conn, "Import en erreur", None, "/audio/5.wav").unwrap();
        update_status(&conn, retryable_error.id, "error", Some("erreur technique")).unwrap();

        let resumable = list_resumable_posteriori(&conn, Some(currently_running.id)).unwrap();

        assert_eq!(resumable.len(), 2);
        assert!(resumable.iter().any(|item| item.id == stopped.id));
        assert!(resumable.iter().any(|item| item.id == retryable_error.id));
    }

    #[test]
    fn every_transcribing_realtime_session_is_recoverable_after_restart() {
        let conn = setup();
        let first = create_realtime(&conn, "Premier", "/audio/1.wav").unwrap();
        let second = create_realtime(&conn, "Second", "/audio/2.wav").unwrap();

        let candidates = list_recovery_candidates(&conn, None).unwrap();
        let ids: Vec<i64> = candidates.iter().map(|interview| interview.id).collect();

        assert!(ids.contains(&first.id));
        assert!(ids.contains(&second.id));
    }

    #[test]
    fn delete_with_managed_audio_removes_file_and_database_row() {
        let mut conn = setup();
        let root = std::env::temp_dir().join(format!(
            "interviewscribe-delete-managed-{}",
            std::process::id()
        ));
        let audio_dir = root.join("audio");
        std::fs::create_dir_all(&audio_dir).unwrap();
        let interview = create(&conn, "A supprimer", None, "placeholder").unwrap();
        let audio_path = audio_dir.join(format!("{}.wav", interview.id));
        std::fs::write(&audio_path, b"private audio sentinel").unwrap();
        set_audio_path(&conn, interview.id, &audio_path.to_string_lossy()).unwrap();

        delete_with_managed_audio(&mut conn, interview.id, &audio_dir).unwrap();

        assert!(!audio_path.exists());
        assert!(matches!(
            get(&conn, interview.id),
            Err(AppError::NotFound(_))
        ));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn delete_with_managed_audio_refuses_external_file_without_touching_anything() {
        let mut conn = setup();
        let root = std::env::temp_dir().join(format!(
            "interviewscribe-delete-external-{}",
            std::process::id()
        ));
        let audio_dir = root.join("private-audio");
        std::fs::create_dir_all(&audio_dir).unwrap();
        let external = root.join("user-owned.wav");
        std::fs::write(&external, b"must survive").unwrap();
        let interview = create(&conn, "A conserver", None, &external.to_string_lossy()).unwrap();

        let error = delete_with_managed_audio(&mut conn, interview.id, &audio_dir).unwrap_err();

        assert!(matches!(error, AppError::Audio(_)));
        assert_eq!(std::fs::read(&external).unwrap(), b"must survive");
        assert_eq!(get(&conn, interview.id).unwrap().title, "A conserver");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn delete_with_managed_audio_allows_an_already_missing_private_file() {
        let mut conn = setup();
        let audio_dir = std::env::temp_dir().join(format!(
            "interviewscribe-delete-missing-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&audio_dir).unwrap();
        let interview = create(&conn, "Audio absent", None, "placeholder").unwrap();
        let audio_path = audio_dir.join(format!("{}.wav", interview.id));
        set_audio_path(&conn, interview.id, &audio_path.to_string_lossy()).unwrap();

        delete_with_managed_audio(&mut conn, interview.id, &audio_dir).unwrap();

        assert!(matches!(
            get(&conn, interview.id),
            Err(AppError::NotFound(_))
        ));
        std::fs::remove_dir_all(audio_dir).ok();
    }

    #[test]
    fn delete_with_managed_audio_rolls_back_database_when_file_removal_fails() {
        let mut conn = setup();
        let root = std::env::temp_dir().join(format!(
            "interviewscribe-delete-rollback-{}",
            std::process::id()
        ));
        let audio_dir = root.join("audio");
        std::fs::create_dir_all(&audio_dir).unwrap();
        let interview = create(&conn, "Suppression impossible", None, "placeholder").unwrap();
        let audio_path = audio_dir.join(format!("{}.wav", interview.id));
        // A directory at the managed file path makes remove_file fail on every
        // supported platform without relying on Unix-only permissions.
        std::fs::create_dir(&audio_path).unwrap();
        set_audio_path(&conn, interview.id, &audio_path.to_string_lossy()).unwrap();

        let error = delete_with_managed_audio(&mut conn, interview.id, &audio_dir).unwrap_err();

        assert!(matches!(error, AppError::Io(_)));
        assert!(audio_path.is_dir());
        assert_eq!(
            get(&conn, interview.id).unwrap().title,
            "Suppression impossible"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
