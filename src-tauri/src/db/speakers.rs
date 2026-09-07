use rusqlite::{params, Connection, Row};

use super::models::Speaker;
use crate::error::AppError;

/// Test-only convenience: a single default speaker, equivalent to
/// `create_numbered(conn, id, 1)`. Production code always goes through
/// diarization's `create_numbered` now.
#[cfg(test)]
pub fn create_default(conn: &Connection, interview_id: i64) -> Result<Speaker, AppError> {
    create_numbered(conn, interview_id, 1)
}

pub fn list_for_interview(conn: &Connection, interview_id: i64) -> Result<Vec<Speaker>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, interview_id, label, color, display_name FROM speaker WHERE interview_id = ?1",
    )?;
    let rows = stmt.query_map(params![interview_id], row_to_speaker)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

const PALETTE: &[&str] = &[
    "#3156a3", "#a33131", "#2f8f5b", "#a37a13", "#6b3fa0", "#1b7f8f",
];

/// Creates one of several speakers diarization detected for an interview.
/// `index` is 1-based ("Intervenant 1", "Intervenant 2", ...) - never a
/// human name, same invariant as `create_default`.
pub fn create_numbered(
    conn: &Connection,
    interview_id: i64,
    index: usize,
) -> Result<Speaker, AppError> {
    let label = format!("Intervenant {index}");
    let color = PALETTE[(index.saturating_sub(1)) % PALETTE.len()];
    create_with_label(conn, interview_id, &label, color)
}

/// Creates a speaker the user adds by hand (e.g. auto-detection missed a
/// third voice). Still never invents a display name.
pub fn create_speaker(
    conn: &Connection,
    interview_id: i64,
    label: &str,
) -> Result<Speaker, AppError> {
    if label.trim().is_empty() {
        return Err(AppError::Edit(
            "le nom du locuteur ne peut pas etre vide".into(),
        ));
    }
    let existing = list_for_interview(conn, interview_id)?.len();
    let color = PALETTE[existing % PALETTE.len()];
    create_with_label(conn, interview_id, label, color)
}

fn create_with_label(
    conn: &Connection,
    interview_id: i64,
    label: &str,
    color: &str,
) -> Result<Speaker, AppError> {
    conn.execute(
        "INSERT INTO speaker (interview_id, label, color, display_name) VALUES (?1, ?2, ?3, NULL)",
        params![interview_id, label, color],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, label, color, display_name FROM speaker WHERE id = ?1",
        params![id],
        row_to_speaker,
    )
    .map_err(AppError::from)
}

/// Sets the human-chosen name for a speaker. Rejects blank input rather than
/// silently storing an empty display name.
pub fn rename(conn: &Connection, speaker_id: i64, display_name: &str) -> Result<Speaker, AppError> {
    let trimmed = display_name.trim();
    if trimmed.is_empty() {
        return Err(AppError::Edit(
            "le nom du locuteur ne peut pas etre vide".into(),
        ));
    }
    conn.execute(
        "UPDATE speaker SET display_name = ?1 WHERE id = ?2",
        params![trimmed, speaker_id],
    )?;
    conn.query_row(
        "SELECT id, interview_id, label, color, display_name FROM speaker WHERE id = ?1",
        params![speaker_id],
        row_to_speaker,
    )
    .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("speaker {speaker_id}")),
        other => AppError::Db(other),
    })
}

/// Merges `remove_id` into `keep_id`: every segment attributed to `remove_id`
/// is reassigned, then `remove_id` is deleted. Single transaction so a
/// half-applied merge can never be observed.
pub fn merge(
    conn: &Connection,
    interview_id: i64,
    keep_id: i64,
    remove_id: i64,
) -> Result<Vec<Speaker>, AppError> {
    if keep_id == remove_id {
        return Err(AppError::Edit(
            "impossible de fusionner un locuteur avec lui-meme".into(),
        ));
    }
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE segment SET speaker_id = ?1 WHERE speaker_id = ?2",
        params![keep_id, remove_id],
    )?;
    let deleted = tx.execute("DELETE FROM speaker WHERE id = ?1", params![remove_id])?;
    if deleted == 0 {
        return Err(AppError::NotFound(format!("speaker {remove_id}")));
    }
    tx.commit()?;
    list_for_interview(conn, interview_id)
}

fn row_to_speaker(row: &Row) -> rusqlite::Result<Speaker> {
    Ok(Speaker {
        id: row.get(0)?,
        interview_id: row.get(1)?,
        label: row.get(2)?,
        color: row.get(3)?,
        display_name: row.get(4)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{interviews, schema};

    fn setup_interview() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let interview = interviews::create(&conn, "Entretien", None, "/audio/1.wav").unwrap();
        (conn, interview.id)
    }

    #[test]
    fn default_speaker_never_invents_a_name() {
        let (conn, interview_id) = setup_interview();
        let speaker = create_default(&conn, interview_id).unwrap();
        assert_eq!(speaker.label, "Intervenant 1");
        assert_eq!(speaker.display_name, None);
    }

    #[test]
    fn deleting_interview_cascades_to_speaker() {
        let (conn, interview_id) = setup_interview();
        create_default(&conn, interview_id).unwrap();
        conn.execute("DELETE FROM interview WHERE id = ?1", params![interview_id])
            .unwrap();
        let remaining = list_for_interview(&conn, interview_id).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn create_numbered_never_invents_a_name() {
        let (conn, interview_id) = setup_interview();
        let first = create_numbered(&conn, interview_id, 1).unwrap();
        let second = create_numbered(&conn, interview_id, 2).unwrap();
        assert_eq!(first.label, "Intervenant 1");
        assert_eq!(second.label, "Intervenant 2");
        assert_eq!(first.display_name, None);
        assert_ne!(first.color, second.color);
    }

    #[test]
    fn create_speaker_rejects_a_blank_label() {
        let (conn, interview_id) = setup_interview();
        let err = create_speaker(&conn, interview_id, "   ").unwrap_err();
        assert!(matches!(err, AppError::Edit(_)));
    }

    #[test]
    fn rename_sets_display_name_and_rejects_blank() {
        let (conn, interview_id) = setup_interview();
        let speaker = create_default(&conn, interview_id).unwrap();
        let renamed = rename(&conn, speaker.id, "Marie").unwrap();
        assert_eq!(renamed.display_name, Some("Marie".to_string()));
        // The placeholder label is untouched by a rename.
        assert_eq!(renamed.label, "Intervenant 1");

        let err = rename(&conn, speaker.id, "   ").unwrap_err();
        assert!(matches!(err, AppError::Edit(_)));
    }

    #[test]
    fn rename_missing_speaker_is_not_found() {
        let (conn, _interview_id) = setup_interview();
        let err = rename(&conn, 999, "Marie").unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn merge_reassigns_segments_and_deletes_the_removed_speaker() {
        let (conn, interview_id) = setup_interview();
        let keep = create_numbered(&conn, interview_id, 1).unwrap();
        let remove = create_numbered(&conn, interview_id, 2).unwrap();
        crate::db::segments::insert_batch(
            &conn,
            interview_id,
            &[crate::db::segments::NewSegment {
                speaker_id: Some(remove.id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour".into(),
                confidence: None,
            }],
        )
        .unwrap();

        let remaining = merge(&conn, interview_id, keep.id, remove.id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, keep.id);

        let segments = crate::db::segments::list_for_interview(&conn, interview_id).unwrap();
        assert_eq!(segments[0].speaker_id, Some(keep.id));
    }

    #[test]
    fn merge_rejects_merging_a_speaker_with_itself() {
        let (conn, interview_id) = setup_interview();
        let speaker = create_default(&conn, interview_id).unwrap();
        let err = merge(&conn, interview_id, speaker.id, speaker.id).unwrap_err();
        assert!(matches!(err, AppError::Edit(_)));
    }

    #[test]
    fn merge_missing_speaker_is_not_found() {
        let (conn, interview_id) = setup_interview();
        let keep = create_default(&conn, interview_id).unwrap();
        let err = merge(&conn, interview_id, keep.id, 999).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }
}
