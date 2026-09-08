use std::path::Path;

use rusqlite::Connection;
use serde::Serialize;

use crate::{audio, db, error::AppError};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RecoveryInspection {
    pub interview_id: i64,
    pub audio_duration_ms: i64,
    pub last_stable_end_ms: i64,
    pub remaining_ms: i64,
    pub segment_count: usize,
}

/// Validates an interrupted recording before recovery. This is deliberately
/// read-only: existing segments (including immutable `raw_text`) remain the
/// evidence boundary for the later resume operation.
pub fn inspect(
    conn: &Connection,
    interview_id: i64,
    audio_dir: &Path,
) -> Result<RecoveryInspection, AppError> {
    let interview = db::interviews::get(conn, interview_id)?;
    if interview.mode != "realtime" || interview.status != "transcribing" {
        return Err(AppError::Audio(
            "recuperation refusee: cette session n'est pas un enregistrement interrompu".into(),
        ));
    }

    let audio_path = Path::new(&interview.audio_path);
    if !db::interviews::is_managed_audio_path(audio_path, audio_dir, interview_id) {
        return Err(AppError::Audio(
            "recuperation refusee: le fichier audio n'appartient pas au stockage prive".into(),
        ));
    }

    let pcm = audio::decode::decode_to_mono_pcm16k(audio_path)?;
    let audio_duration_ms = i64::try_from(pcm.len())
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000)
        / i64::from(audio::decode::WHISPER_SAMPLE_RATE);
    let segments = db::segments::list_for_interview(conn, interview_id)?;
    let last_stable_end_ms = segments
        .iter()
        .map(|segment| segment.end_ms)
        .max()
        .unwrap_or(0)
        .clamp(0, audio_duration_ms);

    Ok(RecoveryInspection {
        interview_id,
        audio_duration_ms,
        last_stable_end_ms,
        remaining_ms: audio_duration_ms.saturating_sub(last_stable_end_ms),
        segment_count: segments.len(),
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;
    use crate::db::{schema, segments::NewSegment};

    fn setup(name: &str) -> (Connection, std::path::PathBuf) {
        let conn = Connection::open_in_memory().unwrap();
        schema::init(&conn).unwrap();
        let audio_dir = std::env::temp_dir().join(format!(
            "interviewscribe-recovery-{}-{name}",
            std::process::id()
        ));
        std::fs::create_dir_all(&audio_dir).unwrap();
        (conn, audio_dir)
    }

    fn write_wav(path: &Path, duration_ms: usize) {
        let sample_count = duration_ms * 16;
        let data_len = (sample_count * 2) as u32;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16u32.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&1u16.to_le_bytes()).unwrap();
        file.write_all(&16_000u32.to_le_bytes()).unwrap();
        file.write_all(&32_000u32.to_le_bytes()).unwrap();
        file.write_all(&2u16.to_le_bytes()).unwrap();
        file.write_all(&16u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        for _ in 0..sample_count {
            file.write_all(&0i16.to_le_bytes()).unwrap();
        }
    }

    fn interrupted(conn: &Connection, audio_dir: &Path) -> db::models::Interview {
        let interview = db::interviews::create_realtime(conn, "Interrompu", "placeholder").unwrap();
        let path = audio_dir.join(format!("{}.wav", interview.id));
        db::interviews::set_audio_path(conn, interview.id, &path.to_string_lossy()).unwrap();
        db::interviews::get(conn, interview.id).unwrap()
    }

    #[test]
    fn validates_wav_and_reports_unstable_remainder_without_changing_segments() {
        let (conn, audio_dir) = setup("valid");
        let interview = interrupted(&conn, &audio_dir);
        write_wav(Path::new(&interview.audio_path), 1_000);
        db::segments::insert_batch(
            &conn,
            interview.id,
            &[NewSegment {
                speaker_id: None,
                start_ms: 100,
                end_ms: 400,
                raw_text: "preuve brute".into(),
                confidence: Some(0.9),
            }],
        )
        .unwrap();

        let result = inspect(&conn, interview.id, &audio_dir).unwrap();

        assert_eq!(result.audio_duration_ms, 1_000);
        assert_eq!(result.last_stable_end_ms, 400);
        assert_eq!(result.remaining_ms, 600);
        assert_eq!(result.segment_count, 1);
        let preserved = db::segments::list_for_interview(&conn, interview.id).unwrap();
        assert_eq!(preserved[0].raw_text, "preuve brute");
        std::fs::remove_dir_all(audio_dir).ok();
    }

    #[test]
    fn rejects_missing_corrupt_and_external_audio() {
        let (conn, audio_dir) = setup("invalid");
        let missing = interrupted(&conn, &audio_dir);
        assert!(inspect(&conn, missing.id, &audio_dir).is_err());

        std::fs::write(&missing.audio_path, b"not audio").unwrap();
        assert!(inspect(&conn, missing.id, &audio_dir).is_err());

        let external = db::interviews::create_realtime(&conn, "Externe", "/tmp/user.wav").unwrap();
        let error = inspect(&conn, external.id, &audio_dir).unwrap_err();
        assert!(matches!(error, AppError::Audio(_)));
        std::fs::remove_dir_all(audio_dir).ok();
    }

    #[test]
    fn rejects_a_session_that_is_not_interrupted() {
        let (conn, audio_dir) = setup("finished");
        let interview = interrupted(&conn, &audio_dir);
        write_wav(Path::new(&interview.audio_path), 100);
        db::interviews::update_status(&conn, interview.id, "transcribed", None).unwrap();

        assert!(inspect(&conn, interview.id, &audio_dir).is_err());
        std::fs::remove_dir_all(audio_dir).ok();
    }
}
