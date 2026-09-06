use rusqlite::{params, Connection, OptionalExtension, Row};

use super::models::Edit;
use crate::error::AppError;

/// Records one user action (one "Nettoyer" click, one saved manual edit) as a
/// new, non-reverted edit row. Never mutates `segment.raw_text`.
pub fn record(
    conn: &Connection,
    segment_id: i64,
    operation: &str,
    before_text: &str,
    after_text: &str,
) -> Result<Edit, AppError> {
    let now = super::now_epoch_secs();
    conn.execute(
        "INSERT INTO edit (segment_id, operation, before_text, after_text, created_at, reverted_at)
         VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![segment_id, operation, before_text, after_text, now],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, segment_id, operation, before_text, after_text, created_at, reverted_at
         FROM edit WHERE id = ?1",
        params![id],
        row_to_edit,
    )
    .map_err(AppError::from)
}

pub fn list_for_segment(conn: &Connection, segment_id: i64) -> Result<Vec<Edit>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, segment_id, operation, before_text, after_text, created_at, reverted_at
         FROM edit WHERE segment_id = ?1 ORDER BY id DESC",
    )?;
    let rows = stmt.query_map(params![segment_id], row_to_edit)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// The text a segment currently shows: the latest non-reverted edit's
/// `after_text`, or `raw_text` if it has never been edited (or every edit has
/// been undone).
pub fn current_text(
    conn: &Connection,
    segment_id: i64,
    raw_text: &str,
) -> Result<String, AppError> {
    let latest: Option<String> = conn
        .query_row(
            "SELECT after_text FROM edit WHERE segment_id = ?1 AND reverted_at IS NULL
             ORDER BY id DESC LIMIT 1",
            params![segment_id],
            |row| row.get(0),
        )
        .optional()?;
    Ok(latest.unwrap_or_else(|| raw_text.to_string()))
}

/// Undoes the most recent non-reverted edit for this segment (a simple LIFO
/// stack, no redo). Returns `None`, not an error, if there was nothing to
/// undo — callers/UI are expected to disable the action in that case rather
/// than treat it as a failure.
pub fn revert_latest(conn: &Connection, segment_id: i64) -> Result<Option<Edit>, AppError> {
    let target_id: Option<i64> = conn
        .query_row(
            "SELECT id FROM edit WHERE segment_id = ?1 AND reverted_at IS NULL
             ORDER BY id DESC LIMIT 1",
            params![segment_id],
            |row| row.get(0),
        )
        .optional()?;
    let Some(id) = target_id else {
        return Ok(None);
    };
    let now = super::now_epoch_secs();
    conn.execute(
        "UPDATE edit SET reverted_at = ?1 WHERE id = ?2",
        params![now, id],
    )?;
    conn.query_row(
        "SELECT id, segment_id, operation, before_text, after_text, created_at, reverted_at
         FROM edit WHERE id = ?1",
        params![id],
        row_to_edit,
    )
    .map(Some)
    .map_err(AppError::from)
}

fn row_to_edit(row: &Row) -> rusqlite::Result<Edit> {
    Ok(Edit {
        id: row.get(0)?,
        segment_id: row.get(1)?,
        operation: row.get(2)?,
        before_text: row.get(3)?,
        after_text: row.get(4)?,
        created_at: row.get(5)?,
        reverted_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{interviews, schema, segments, speakers};

    fn setup_segment() -> (Connection, i64) {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let interview = interviews::create(&conn, "Entretien", None, "/audio/1.wav").unwrap();
        let speaker = speakers::create_default(&conn, interview.id).unwrap();
        segments::insert_batch(
            &conn,
            interview.id,
            &[segments::NewSegment {
                speaker_id: Some(speaker.id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour euh le monde".into(),
                confidence: None,
            }],
        )
        .unwrap();
        let segment_id = segments::list_for_interview(&conn, interview.id).unwrap()[0].id;
        (conn, segment_id)
    }

    #[test]
    fn current_text_falls_back_to_raw_text_with_no_edits() {
        let (conn, segment_id) = setup_segment();
        let text = current_text(&conn, segment_id, "Bonjour euh le monde").unwrap();
        assert_eq!(text, "Bonjour euh le monde");
    }

    #[test]
    fn record_updates_current_text() {
        let (conn, segment_id) = setup_segment();
        record(
            &conn,
            segment_id,
            "cleanup",
            "Bonjour euh le monde",
            "Bonjour le monde",
        )
        .unwrap();
        let text = current_text(&conn, segment_id, "Bonjour euh le monde").unwrap();
        assert_eq!(text, "Bonjour le monde");
    }

    #[test]
    fn revert_latest_falls_back_one_level_then_to_raw_text() {
        let (conn, segment_id) = setup_segment();
        record(
            &conn,
            segment_id,
            "cleanup",
            "Bonjour euh le monde",
            "Bonjour le monde",
        )
        .unwrap();
        record(
            &conn,
            segment_id,
            "manual",
            "Bonjour le monde",
            "Salut le monde",
        )
        .unwrap();
        assert_eq!(
            current_text(&conn, segment_id, "Bonjour euh le monde").unwrap(),
            "Salut le monde"
        );

        let reverted = revert_latest(&conn, segment_id).unwrap();
        assert!(reverted.is_some());
        assert_eq!(
            current_text(&conn, segment_id, "Bonjour euh le monde").unwrap(),
            "Bonjour le monde"
        );

        revert_latest(&conn, segment_id).unwrap();
        assert_eq!(
            current_text(&conn, segment_id, "Bonjour euh le monde").unwrap(),
            "Bonjour euh le monde"
        );
    }

    #[test]
    fn revert_latest_with_nothing_pending_returns_none() {
        let (conn, segment_id) = setup_segment();
        assert!(revert_latest(&conn, segment_id).unwrap().is_none());
    }

    #[test]
    fn a_fresh_edit_after_undo_does_not_resurrect_the_undone_one() {
        let (conn, segment_id) = setup_segment();
        record(
            &conn,
            segment_id,
            "cleanup",
            "Bonjour euh le monde",
            "Bonjour le monde",
        )
        .unwrap();
        revert_latest(&conn, segment_id).unwrap();
        record(
            &conn,
            segment_id,
            "manual",
            "Bonjour euh le monde",
            "Coucou le monde",
        )
        .unwrap();
        assert_eq!(
            current_text(&conn, segment_id, "Bonjour euh le monde").unwrap(),
            "Coucou le monde"
        );
        // Undoing again should land back on raw_text, not resurrect "Bonjour le monde".
        revert_latest(&conn, segment_id).unwrap();
        assert_eq!(
            current_text(&conn, segment_id, "Bonjour euh le monde").unwrap(),
            "Bonjour euh le monde"
        );
    }

    #[test]
    fn list_for_segment_orders_most_recent_first() {
        let (conn, segment_id) = setup_segment();
        record(&conn, segment_id, "cleanup", "a", "b").unwrap();
        record(&conn, segment_id, "manual", "b", "c").unwrap();
        let edits = list_for_segment(&conn, segment_id).unwrap();
        assert_eq!(edits.len(), 2);
        assert_eq!(edits[0].after_text, "c");
        assert_eq!(edits[1].after_text, "b");
    }
}
