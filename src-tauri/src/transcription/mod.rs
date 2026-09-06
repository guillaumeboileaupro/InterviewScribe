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

/// Attributes every produced segment to the single given speaker. Phase 1 never
/// diarizes, so this is the one place segments and a speaker are joined -
/// callers must not create more than one speaker per interview.
pub fn to_new_segments(raw: Vec<RawSegment>, speaker_id: i64) -> Vec<NewSegment> {
    raw.into_iter()
        .map(|segment| NewSegment {
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

    #[test]
    fn every_segment_is_attributed_to_the_one_given_speaker() {
        let transcriber = FakeTranscriber {
            segments: vec![
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
            ],
        };
        let raw = transcriber.transcribe(&[], None).unwrap();
        let new_segments = to_new_segments(raw, 42);

        assert_eq!(new_segments.len(), 2);
        assert!(new_segments
            .iter()
            .all(|segment| segment.speaker_id == Some(42)));
        assert_eq!(new_segments[0].raw_text, "Bonjour");
        assert_eq!(new_segments[1].raw_text, "Au revoir");
    }
}
