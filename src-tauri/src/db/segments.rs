use rusqlite::{params, Connection, Row};

use super::models::Segment;
use crate::error::AppError;

pub struct NewSegment {
    pub speaker_id: Option<i64>,
    pub start_ms: i64,
    pub end_ms: i64,
    pub raw_text: String,
    pub confidence: Option<f64>,
}

/// Inserts every segment and returns their new ids, in the same order as
/// `segments` - lets a caller (e.g. diarization) address a specific inserted
/// row afterwards (see `mark_uncertain`) without depending on `start_ms`
/// ordering or ties.
pub fn insert_batch(
    conn: &Connection,
    interview_id: i64,
    segments: &[NewSegment],
) -> Result<Vec<i64>, AppError> {
    let tx = conn.unchecked_transaction()?;
    let mut ids = Vec::with_capacity(segments.len());
    {
        let mut stmt = tx.prepare(
            "INSERT INTO segment (interview_id, speaker_id, start_ms, end_ms, raw_text, confidence, status)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'raw')",
        )?;
        for seg in segments {
            stmt.execute(params![
                interview_id,
                seg.speaker_id,
                seg.start_ms,
                seg.end_ms,
                seg.raw_text,
                seg.confidence,
            ])?;
            ids.push(tx.last_insert_rowid());
        }
    }
    tx.commit()?;
    Ok(ids)
}

/// Flags a segment's speaker attribution as uncertain (diarization's best
/// guess had too little margin over the runner-up) rather than forcing
/// silent confidence - see docs/ARCHITECTURE.md "Diarisation".
pub fn mark_uncertain(conn: &Connection, segment_id: i64) -> Result<(), AppError> {
    conn.execute(
        "UPDATE segment SET status = 'uncertain' WHERE id = ?1",
        params![segment_id],
    )?;
    Ok(())
}

const SELECT_SEGMENT_COLUMNS: &str = "
    s.id, s.interview_id, s.speaker_id, s.start_ms, s.end_ms, s.raw_text,
    COALESCE(
        (SELECT after_text FROM edit WHERE segment_id = s.id AND reverted_at IS NULL ORDER BY id DESC LIMIT 1),
        s.raw_text
    ),
    s.confidence, s.status
";

pub fn list_for_interview(conn: &Connection, interview_id: i64) -> Result<Vec<Segment>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "SELECT {SELECT_SEGMENT_COLUMNS} FROM segment s WHERE s.interview_id = ?1 ORDER BY s.start_ms ASC"
    ))?;
    let rows = stmt.query_map(params![interview_id], row_to_segment)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

/// Returns only the tail needed by the live recording view. A multi-hour
/// interview must not be serialized in full after every stabilized chunk.
pub fn list_recent_for_interview(
    conn: &Connection,
    interview_id: i64,
    limit: usize,
) -> Result<Vec<Segment>, AppError> {
    let bounded_limit = limit.clamp(1, 200) as i64;
    let mut stmt = conn.prepare(&format!(
        "SELECT * FROM (SELECT {SELECT_SEGMENT_COLUMNS} FROM segment s WHERE s.interview_id = ?1 ORDER BY s.start_ms DESC LIMIT ?2) ORDER BY start_ms ASC"
    ))?;
    let rows = stmt.query_map(params![interview_id, bounded_limit], row_to_segment)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, segment_id: i64) -> Result<Segment, AppError> {
    conn.query_row(
        &format!("SELECT {SELECT_SEGMENT_COLUMNS} FROM segment s WHERE s.id = ?1"),
        params![segment_id],
        row_to_segment,
    )
    .map_err(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => AppError::NotFound(format!("segment {segment_id}")),
        other => AppError::Db(other),
    })
}

/// Moves a segment to a different speaker (or clears it with `None`). This is
/// the manual-correction path for diarization mistakes - "separating" a
/// wrongly-merged speaker means moving their segments here, one at a time,
/// rather than re-running automatic clustering.
pub fn reassign_speaker(
    conn: &Connection,
    segment_id: i64,
    speaker_id: Option<i64>,
) -> Result<Segment, AppError> {
    let updated = conn.execute(
        "UPDATE segment SET speaker_id = ?1 WHERE id = ?2",
        params![speaker_id, segment_id],
    )?;
    if updated == 0 {
        return Err(AppError::NotFound(format!("segment {segment_id}")));
    }
    get(conn, segment_id)
}

fn row_to_segment(row: &Row) -> rusqlite::Result<Segment> {
    Ok(Segment {
        id: row.get(0)?,
        interview_id: row.get(1)?,
        speaker_id: row.get(2)?,
        start_ms: row.get(3)?,
        end_ms: row.get(4)?,
        raw_text: row.get(5)?,
        current_text: row.get(6)?,
        confidence: row.get(7)?,
        status: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{interviews, schema, speakers};

    fn setup() -> (Connection, i64, i64) {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let interview = interviews::create(&conn, "Entretien", None, "/audio/1.wav").unwrap();
        let speaker = speakers::create_default(&conn, interview.id).unwrap();
        (conn, interview.id, speaker.id)
    }

    #[test]
    fn insert_and_list_round_trip_ordered_by_start() {
        let (conn, interview_id, speaker_id) = setup();
        insert_batch(
            &conn,
            interview_id,
            &[
                NewSegment {
                    speaker_id: Some(speaker_id),
                    start_ms: 1000,
                    end_ms: 2000,
                    raw_text: "Second".into(),
                    confidence: Some(0.9),
                },
                NewSegment {
                    speaker_id: Some(speaker_id),
                    start_ms: 0,
                    end_ms: 1000,
                    raw_text: "First".into(),
                    confidence: Some(0.95),
                },
            ],
        )
        .unwrap();
        let segments = list_for_interview(&conn, interview_id).unwrap();
        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].raw_text, "First");
        assert_eq!(segments[1].raw_text, "Second");
    }

    #[test]
    fn recent_list_is_bounded_and_remains_chronological() {
        let (conn, interview_id, speaker_id) = setup();
        let segments: Vec<NewSegment> = (0..250)
            .map(|index| NewSegment {
                speaker_id: Some(speaker_id),
                start_ms: index * 1_000,
                end_ms: index * 1_000 + 900,
                raw_text: format!("Segment {index}"),
                confidence: None,
            })
            .collect();
        insert_batch(&conn, interview_id, &segments).unwrap();

        let recent = list_recent_for_interview(&conn, interview_id, 100).unwrap();
        assert_eq!(recent.len(), 100);
        assert_eq!(recent.first().unwrap().start_ms, 150_000);
        assert_eq!(recent.last().unwrap().start_ms, 249_000);

        let capped = list_recent_for_interview(&conn, interview_id, usize::MAX).unwrap();
        assert_eq!(capped.len(), 200);
    }

    #[test]
    fn deleting_interview_cascades_to_segments() {
        let (conn, interview_id, speaker_id) = setup();
        insert_batch(
            &conn,
            interview_id,
            &[NewSegment {
                speaker_id: Some(speaker_id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour".into(),
                confidence: None,
            }],
        )
        .unwrap();
        conn.execute("DELETE FROM interview WHERE id = ?1", params![interview_id])
            .unwrap();
        let remaining = list_for_interview(&conn, interview_id).unwrap();
        assert!(remaining.is_empty());
    }

    #[test]
    fn current_text_defaults_to_raw_text_with_no_edits() {
        let (conn, interview_id, speaker_id) = setup();
        insert_batch(
            &conn,
            interview_id,
            &[NewSegment {
                speaker_id: Some(speaker_id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour euh le monde".into(),
                confidence: None,
            }],
        )
        .unwrap();
        let segment = list_for_interview(&conn, interview_id).unwrap().remove(0);
        assert_eq!(segment.current_text, segment.raw_text);
        let fetched = get(&conn, segment.id).unwrap();
        assert_eq!(fetched.raw_text, "Bonjour euh le monde");
    }

    #[test]
    fn get_missing_segment_is_not_found() {
        let (conn, _interview_id, _speaker_id) = setup();
        let err = get(&conn, 999).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn reassign_speaker_moves_a_segment_to_another_speaker() {
        let (conn, interview_id, speaker_id) = setup();
        let other = speakers::create_numbered(&conn, interview_id, 2).unwrap();
        insert_batch(
            &conn,
            interview_id,
            &[NewSegment {
                speaker_id: Some(speaker_id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour".into(),
                confidence: None,
            }],
        )
        .unwrap();
        let segment_id = list_for_interview(&conn, interview_id).unwrap()[0].id;

        let updated = reassign_speaker(&conn, segment_id, Some(other.id)).unwrap();
        assert_eq!(updated.speaker_id, Some(other.id));

        let cleared = reassign_speaker(&conn, segment_id, None).unwrap();
        assert_eq!(cleared.speaker_id, None);
    }

    #[test]
    fn reassign_speaker_missing_segment_is_not_found() {
        let (conn, _interview_id, speaker_id) = setup();
        let err = reassign_speaker(&conn, 999, Some(speaker_id)).unwrap_err();
        assert!(matches!(err, AppError::NotFound(_)));
    }

    #[test]
    fn deleting_speaker_keeps_segment_with_null_speaker() {
        let (conn, interview_id, speaker_id) = setup();
        insert_batch(
            &conn,
            interview_id,
            &[NewSegment {
                speaker_id: Some(speaker_id),
                start_ms: 0,
                end_ms: 1000,
                raw_text: "Bonjour".into(),
                confidence: None,
            }],
        )
        .unwrap();
        conn.execute("DELETE FROM speaker WHERE id = ?1", params![speaker_id])
            .unwrap();
        let remaining = list_for_interview(&conn, interview_id).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].speaker_id, None);
    }
}
