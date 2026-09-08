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

pub fn pcm_after_boundary(pcm: &[f32], boundary_ms: i64) -> &[f32] {
    let boundary_ms = boundary_ms.max(0) as usize;
    let sample = boundary_ms.saturating_mul(audio::decode::WHISPER_SAMPLE_RATE as usize) / 1_000;
    &pcm[sample.min(pcm.len())..]
}

/// Rebases Whisper timestamps from the recovered suffix to the interview
/// timeline and rejects anything wholly covered by stable evidence. Exact
/// duplicate boundary text is also discarded, protecting repeated recovery.
pub fn rebase_and_filter(
    raw: Vec<crate::transcription::RawSegment>,
    boundary_ms: i64,
    existing: &[db::models::Segment],
) -> Vec<crate::transcription::RawSegment> {
    let boundary_ms = boundary_ms.max(0);
    raw.into_iter()
        .map(|segment| crate::transcription::RawSegment {
            start_ms: segment.start_ms.saturating_add(boundary_ms),
            end_ms: segment.end_ms.saturating_add(boundary_ms),
            text: segment.text,
            confidence: segment.confidence,
        })
        .filter(|segment| segment.end_ms > boundary_ms)
        .filter(|segment| {
            !existing.iter().any(|stable| {
                stable.raw_text.trim() == segment.text.trim()
                    && segment.start_ms < stable.end_ms
                    && segment.end_ms > stable.start_ms
            })
        })
        .collect()
}

pub fn transcribe_remainder<T: crate::transcription::Transcriber>(
    transcriber: &T,
    pcm: &[f32],
    language: Option<&str>,
    boundary_ms: i64,
    existing: &[db::models::Segment],
) -> Result<Vec<crate::transcription::RawSegment>, AppError> {
    let suffix = pcm_after_boundary(pcm, boundary_ms);
    if suffix.is_empty() {
        return Ok(Vec::new());
    }
    let raw = transcriber.transcribe(suffix, language)?;
    Ok(rebase_and_filter(raw, boundary_ms, existing))
}

/// Persists recovered speech without guessing continuity with the diarization
/// state lost during interruption. A human may assign speakers afterwards.
pub fn insert_as_uncertain(
    conn: &Connection,
    interview_id: i64,
    recovered: Vec<crate::transcription::RawSegment>,
) -> Result<Vec<i64>, AppError> {
    let segments: Vec<db::segments::NewSegment> = recovered
        .into_iter()
        .map(|segment| db::segments::NewSegment {
            speaker_id: None,
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            raw_text: segment.text,
            confidence: segment.confidence,
        })
        .collect();
    let ids = db::segments::insert_batch(conn, interview_id, &segments)?;
    for id in &ids {
        db::segments::mark_uncertain(conn, *id)?;
    }
    Ok(ids)
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

    let remaining_ms = i64::try_from(pcm_after_boundary(&pcm, last_stable_end_ms).len())
        .unwrap_or(i64::MAX)
        .saturating_mul(1_000)
        / i64::from(audio::decode::WHISPER_SAMPLE_RATE);
    Ok(RecoveryInspection {
        interview_id,
        audio_duration_ms,
        last_stable_end_ms,
        remaining_ms,
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
    fn a_session_without_segments_recovers_from_the_start() {
        let (conn, audio_dir) = setup("empty");
        let interview = interrupted(&conn, &audio_dir);
        write_wav(Path::new(&interview.audio_path), 500);

        let result = inspect(&conn, interview.id, &audio_dir).unwrap();

        assert_eq!(result.segment_count, 0);
        assert_eq!(result.last_stable_end_ms, 0);
        assert_eq!(result.remaining_ms, 500);
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

    #[test]
    fn slices_pcm_at_the_stable_boundary_without_losing_the_suffix() {
        let pcm = vec![0.0; 16_000];
        assert_eq!(pcm_after_boundary(&pcm, 250).len(), 12_000);
        assert!(pcm_after_boundary(&pcm, 2_000).is_empty());
    }

    #[test]
    fn rebases_timestamps_and_filters_a_duplicate_boundary_segment() {
        let existing = vec![db::models::Segment {
            id: 1,
            interview_id: 1,
            speaker_id: None,
            start_ms: 0,
            end_ms: 1_000,
            raw_text: "Bonjour".into(),
            current_text: "Bonjour".into(),
            confidence: None,
            status: "raw".into(),
        }];
        let raw = vec![
            crate::transcription::RawSegment {
                start_ms: -100,
                end_ms: 100,
                text: " Bonjour ".into(),
                confidence: None,
            },
            crate::transcription::RawSegment {
                start_ms: 100,
                end_ms: 500,
                text: "Suite".into(),
                confidence: Some(0.8),
            },
        ];

        let recovered = rebase_and_filter(raw, 1_000, &existing);

        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].start_ms, 1_100);
        assert_eq!(recovered[0].end_ms, 1_500);
        assert_eq!(recovered[0].text, "Suite");
    }

    #[test]
    fn transcribes_only_the_unstable_suffix() {
        let transcriber = crate::transcription::tests_support::FakeTranscriber {
            segments: vec![crate::transcription::RawSegment {
                start_ms: 0,
                end_ms: 200,
                text: "Suite".into(),
                confidence: None,
            }],
        };
        let pcm = vec![0.0; 16_000];
        let recovered = transcribe_remainder(&transcriber, &pcm, None, 750, &[]).unwrap();
        assert_eq!(recovered[0].start_ms, 750);
        assert_eq!(recovered[0].end_ms, 950);
    }

    #[test]
    fn final_silence_produces_no_segment_and_preserves_existing_evidence() {
        let transcriber = crate::transcription::tests_support::FakeTranscriber { segments: vec![] };
        let existing = vec![db::models::Segment {
            id: 1,
            interview_id: 1,
            speaker_id: None,
            start_ms: 0,
            end_ms: 750,
            raw_text: "Déjà stable".into(),
            current_text: "Déjà stable".into(),
            confidence: None,
            status: "raw".into(),
        }];

        let recovered =
            transcribe_remainder(&transcriber, &vec![0.0; 16_000], Some("fr"), 750, &existing)
                .unwrap();
        assert!(recovered.is_empty());
        assert_eq!(existing[0].raw_text, "Déjà stable");
    }

    #[test]
    fn repeated_recovery_at_audio_end_is_a_no_op() {
        let transcriber = crate::transcription::tests_support::FakeTranscriber {
            segments: vec![crate::transcription::RawSegment {
                start_ms: 0,
                end_ms: 100,
                text: "Ne doit pas être produit".into(),
                confidence: None,
            }],
        };
        let recovered =
            transcribe_remainder(&transcriber, &vec![0.0; 16_000], None, 1_000, &[]).unwrap();
        assert!(recovered.is_empty());
    }

    #[test]
    fn recovered_segments_have_no_guessed_speaker_and_are_uncertain() {
        let (conn, audio_dir) = setup("speaker");
        let interview = interrupted(&conn, &audio_dir);
        let ids = insert_as_uncertain(
            &conn,
            interview.id,
            vec![crate::transcription::RawSegment {
                start_ms: 500,
                end_ms: 900,
                text: "Voix non reliee".into(),
                confidence: Some(0.7),
            }],
        )
        .unwrap();

        let segment = db::segments::get(&conn, ids[0]).unwrap();
        assert_eq!(segment.speaker_id, None);
        assert_eq!(segment.status, "uncertain");
        assert_eq!(segment.raw_text, "Voix non reliee");
        std::fs::remove_dir_all(audio_dir).ok();
    }

    #[test]
    fn recovered_raw_text_can_be_exported_with_its_rebased_timestamp() {
        let (conn, audio_dir) = setup("export");
        let interview = interrupted(&conn, &audio_dir);
        insert_as_uncertain(
            &conn,
            interview.id,
            vec![crate::transcription::RawSegment {
                start_ms: 65_000,
                end_ms: 66_000,
                text: "Texte récupéré".into(),
                confidence: None,
            }],
        )
        .unwrap();
        db::interviews::update_status(&conn, interview.id, "transcribed", None).unwrap();

        let detail = db::get_detail(&conn, interview.id).unwrap();
        let bytes = crate::export::render(
            "txt",
            &detail,
            &crate::export::ExportOptions {
                show_timestamps: true,
                use_cleaned_text: false,
            },
        )
        .unwrap();
        let output = String::from_utf8(bytes).unwrap();
        assert!(output.contains("[01:05] Intervenant: Texte récupéré"));
        std::fs::remove_dir_all(audio_dir).ok();
    }
}
