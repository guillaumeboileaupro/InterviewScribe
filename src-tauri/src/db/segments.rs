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

pub fn insert_batch(
    conn: &Connection,
    interview_id: i64,
    segments: &[NewSegment],
) -> Result<(), AppError> {
    let tx = conn.unchecked_transaction()?;
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
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn list_for_interview(conn: &Connection, interview_id: i64) -> Result<Vec<Segment>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, interview_id, speaker_id, start_ms, end_ms, raw_text, confidence, status
         FROM segment WHERE interview_id = ?1 ORDER BY start_ms ASC",
    )?;
    let rows = stmt.query_map(params![interview_id], row_to_segment)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn row_to_segment(row: &Row) -> rusqlite::Result<Segment> {
    Ok(Segment {
        id: row.get(0)?,
        interview_id: row.get(1)?,
        speaker_id: row.get(2)?,
        start_ms: row.get(3)?,
        end_ms: row.get(4)?,
        raw_text: row.get(5)?,
        confidence: row.get(6)?,
        status: row.get(7)?,
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
