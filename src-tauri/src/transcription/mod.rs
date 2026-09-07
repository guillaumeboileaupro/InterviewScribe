pub mod model;
pub mod whisper_cpp;

use crate::db::segments::NewSegment;
use crate::error::AppError;

/// A single transcribed span of speech, before it is attributed to a speaker
/// and persisted. Timestamps are milliseconds from the start of the audio.
pub struct RawSegment {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
    pub confidence: Option<f64>,
}

/// Abstraction over the transcription engine, so the model implementation can
/// change without touching the rest of the pipeline (see docs/ARCHITECTURE.md
/// "Strategie Whisper").
pub trait Transcriber: Send {
    fn transcribe(&self, pcm: &[f32], language: Option<&str>) -> Result<Vec<RawSegment>, AppError>;
}

/// Joins each transcribed segment with the speaker diarization assigned it.
/// `speaker_ids[i]` is the database id for `raw[i]` - the caller (`lib.rs`'s
/// `transcribe_local`) resolves diarization::Assignment.speaker_index into a
/// real speaker row before calling this, so this function stays free of any
/// diarization-specific type.
///
/// # Panics
/// If `raw` and `speaker_ids` have different lengths - a caller bug, not a
/// possible user input.
pub fn to_new_segments(raw: Vec<RawSegment>, speaker_ids: &[i64]) -> Vec<NewSegment> {
    assert_eq!(
        raw.len(),
        speaker_ids.len(),
        "one speaker id is required per segment"
    );
    raw.into_iter()
        .zip(speaker_ids)
        .map(|(segment, &speaker_id)| NewSegment {
            speaker_id: Some(speaker_id),
            start_ms: segment.start_ms,
            end_ms: segment.end_ms,
            raw_text: segment.text,
            confidence: segment.confidence,
        })
        .collect()
}

#[cfg(test)]
pub mod tests_support {
    use super::*;

    /// A fake transcriber for tests that must not depend on a real model
    /// (AGENTS.md: never commit a Whisper model).
    pub struct FakeTranscriber {
        pub segments: Vec<RawSegment>,
    }

    impl Transcriber for FakeTranscriber {
        fn transcribe(
            &self,
            _pcm: &[f32],
            _language: Option<&str>,
        ) -> Result<Vec<RawSegment>, AppError> {
            Ok(self
                .segments
                .iter()
                .map(|segment| RawSegment {
                    start_ms: segment.start_ms,
                    end_ms: segment.end_ms,
                    text: segment.text.clone(),
                    confidence: segment.confidence,
                })
                .collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tests_support::FakeTranscriber;
    use super::*;

    fn two_segments() -> Vec<RawSegment> {
        vec![
            RawSegment {
                start_ms: 0,
                end_ms: 1_000,
                text: "Bonjour".into(),
                confidence: Some(0.9),
            },
            RawSegment {
                start_ms: 1_000,
                end_ms: 2_000,
                text: "Au revoir".into(),
                confidence: Some(0.8),
            },
        ]
    }

    #[test]
    fn every_segment_is_attributed_to_the_one_given_speaker() {
        let transcriber = FakeTranscriber {
            segments: two_segments(),
        };
        let raw = transcriber.transcribe(&[], None).unwrap();
        let new_segments = to_new_segments(raw, &[42, 42]);

        assert_eq!(new_segments.len(), 2);
        assert!(new_segments
            .iter()
            .all(|segment| segment.speaker_id == Some(42)));
        assert_eq!(new_segments[0].raw_text, "Bonjour");
        assert_eq!(new_segments[1].raw_text, "Au revoir");
    }

    #[test]
    fn each_segment_can_be_attributed_to_a_different_speaker() {
        let raw = two_segments();
        let new_segments = to_new_segments(raw, &[1, 2]);
        assert_eq!(new_segments[0].speaker_id, Some(1));
        assert_eq!(new_segments[1].speaker_id, Some(2));
    }

    #[test]
    #[should_panic(expected = "one speaker id is required per segment")]
    fn mismatched_lengths_panics() {
        to_new_segments(two_segments(), &[1]);
    }
}
