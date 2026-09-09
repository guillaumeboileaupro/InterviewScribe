pub mod edits;
pub mod interviews;
pub mod models;
pub mod schema;
pub mod segments;
pub mod speakers;

#[cfg(test)]
mod upgrade_fixture_tests;

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::Connection;

use crate::error::AppError;
use models::InterviewDetail;

pub fn open(path: &Path) -> Result<Connection, AppError> {
    let conn = Connection::open(path)?;
    schema::init(&conn)?;
    Ok(conn)
}

pub fn get_detail(conn: &Connection, interview_id: i64) -> Result<InterviewDetail, AppError> {
    let interview = interviews::get(conn, interview_id)?;
    let speakers = speakers::list_for_interview(conn, interview_id)?;
    let segments = segments::list_for_interview(conn, interview_id)?;
    Ok(InterviewDetail {
        interview,
        speakers,
        segments,
    })
}

pub(crate) fn now_epoch_secs() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::{cleanup, export};

    fn unique_test_db(name: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "interviewscribe-{name}-{}-{nonce}.sqlite3",
            std::process::id()
        ))
    }

    #[test]
    fn raw_text_survives_cleanup_speaker_merge_exports_and_undo() {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let interview =
            interviews::create(&conn, "Entretien", Some("fr"), "/audio/test.wav").unwrap();
        let keep = speakers::create_numbered(&conn, interview.id, 1).unwrap();
        let remove = speakers::create_numbered(&conn, interview.id, 2).unwrap();
        let original = "Je euh je confirme le choix.";
        let segment_id = segments::insert_batch(
            &conn,
            interview.id,
            &[segments::NewSegment {
                speaker_id: Some(remove.id),
                start_ms: 1_000,
                end_ms: 2_500,
                raw_text: original.into(),
                confidence: Some(0.91),
            }],
        )
        .unwrap()[0];

        let cleanup = cleanup::analyze(original);
        assert_ne!(cleanup.cleaned_text, original);
        edits::record(
            &conn,
            segment_id,
            "cleanup",
            original,
            &cleanup.cleaned_text,
        )
        .unwrap();
        let after_cleanup = segments::get(&conn, segment_id).unwrap();
        assert_eq!(after_cleanup.raw_text, original);
        assert_eq!(after_cleanup.current_text, cleanup.cleaned_text);

        let manually_edited = "Je confirme ce choix.";
        edits::record(
            &conn,
            segment_id,
            "manual",
            &after_cleanup.current_text,
            manually_edited,
        )
        .unwrap();
        speakers::merge(&conn, interview.id, keep.id, remove.id).unwrap();
        let after_merge = segments::get(&conn, segment_id).unwrap();
        assert_eq!(after_merge.raw_text, original);
        assert_eq!(after_merge.current_text, manually_edited);
        assert_eq!(after_merge.speaker_id, Some(keep.id));

        let detail = get_detail(&conn, interview.id).unwrap();
        let raw_export = export::render(
            "txt",
            &detail,
            &export::ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        )
        .unwrap();
        let cleaned_export = export::render(
            "txt",
            &detail,
            &export::ExportOptions {
                show_timestamps: false,
                use_cleaned_text: true,
            },
        )
        .unwrap();
        assert!(String::from_utf8(raw_export).unwrap().contains(original));
        assert!(String::from_utf8(cleaned_export)
            .unwrap()
            .contains(manually_edited));
        assert_eq!(segments::get(&conn, segment_id).unwrap().raw_text, original);

        edits::revert_latest(&conn, segment_id).unwrap();
        edits::revert_latest(&conn, segment_id).unwrap();
        let after_undo = segments::get(&conn, segment_id).unwrap();
        assert_eq!(after_undo.raw_text, original);
        assert_eq!(after_undo.current_text, original);
    }

    #[test]
    fn interrupted_realtime_metadata_and_partial_segments_survive_database_reopen() {
        let db_path = unique_test_db("recovery");
        let audio_path = db_path.with_extension("wav");
        let interview_id;
        {
            let conn = open(&db_path).unwrap();
            let interview = interviews::create_realtime(
                &conn,
                "Session interrompue",
                &audio_path.to_string_lossy(),
            )
            .unwrap();
            interview_id = interview.id;
            let speaker = speakers::create_numbered(&conn, interview.id, 1).unwrap();
            segments::insert_batch(
                &conn,
                interview.id,
                &[segments::NewSegment {
                    speaker_id: Some(speaker.id),
                    start_ms: 0,
                    end_ms: 1_000,
                    raw_text: "Segment deja stabilise".into(),
                    confidence: Some(0.9),
                }],
            )
            .unwrap();
        }

        let reopened = open(&db_path).unwrap();
        let detail = get_detail(&reopened, interview_id).unwrap();
        assert_eq!(detail.interview.status, "transcribing");
        assert_eq!(detail.interview.audio_path, audio_path.to_string_lossy());
        assert_eq!(detail.segments.len(), 1);
        assert_eq!(detail.segments[0].raw_text, "Segment deja stabilise");
        drop(reopened);
        std::fs::remove_file(db_path).ok();
    }

    #[test]
    fn deleting_an_interview_cascades_to_all_database_children() {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let interview = interviews::create(&conn, "A supprimer", None, "/audio/1.wav").unwrap();
        let speaker = speakers::create_numbered(&conn, interview.id, 1).unwrap();
        let segment_id = segments::insert_batch(
            &conn,
            interview.id,
            &[segments::NewSegment {
                speaker_id: Some(speaker.id),
                start_ms: 0,
                end_ms: 1_000,
                raw_text: "Donnee sentinelle".into(),
                confidence: None,
            }],
        )
        .unwrap()[0];
        edits::record(
            &conn,
            segment_id,
            "manual",
            "Donnee sentinelle",
            "Texte modifie",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO setting (interview_id, model_path, device) VALUES (?1, ?2, ?3)",
            rusqlite::params![interview.id, "/models/test.bin", "cpu"],
        )
        .unwrap();

        conn.execute(
            "DELETE FROM interview WHERE id = ?1",
            rusqlite::params![interview.id],
        )
        .unwrap();

        for table in ["interview", "speaker", "segment", "edit", "setting"] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .unwrap();
            assert_eq!(count, 0, "table {table} still contains private data");
        }
    }
}
