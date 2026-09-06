use rusqlite::{params, Connection, Row};

use super::models::Speaker;
use crate::error::AppError;

const DEFAULT_LABEL: &str = "Intervenant 1";
const DEFAULT_COLOR: &str = "#3156a3";

/// Creates the single speaker Phase 1 ever produces. Never invent a name: the
/// label is always the fixed placeholder until a human renames it.
pub fn create_default(conn: &Connection, interview_id: i64) -> Result<Speaker, AppError> {
    conn.execute(
        "INSERT INTO speaker (interview_id, label, color, display_name) VALUES (?1, ?2, ?3, NULL)",
        params![interview_id, DEFAULT_LABEL, DEFAULT_COLOR],
    )?;
    let id = conn.last_insert_rowid();
    conn.query_row(
        "SELECT id, interview_id, label, color, display_name FROM speaker WHERE id = ?1",
        params![id],
        row_to_speaker,
    )
    .map_err(AppError::from)
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
}
