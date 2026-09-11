pub mod model;
pub mod whisper_cpp;

use crate::audio::decode::WHISPER_SAMPLE_RATE;
use crate::capture::chunker::{ChunkEvent, Chunker};
use crate::capture::vad::{rms, Vad, VadConfig};
use crate::db::segments::NewSegment;
use crate::error::AppError;
use std::ops::Range;

/// 20ms at 16kHz - same frame size the live capture pipeline's own tests use
/// (see `capture::chunker`'s test fixtures), just replayed synchronously
/// over an already-decoded buffer instead of a live callback.
const CHUNKING_FRAME_SAMPLES: usize = (WHISPER_SAMPLE_RATE / 50) as usize;

/// Splits an already-decoded PCM buffer into transcription-ready chunks
/// using the same VAD-based cut points as the live capture pipeline
/// (`capture::chunker`), replayed synchronously instead of over a live
/// stream. Lets a posteriori import be transcribed and persisted chunk by
/// chunk - real, observable progress and a safe point to stop/resume between
/// chunks, instead of one uninterruptible pass over the whole file.
#[cfg(test)]
pub fn chunk_pcm(
    pcm: &[f32],
    max_chunk_duration_ms: u32,
    silence_to_close_ms: u32,
) -> Vec<Vec<f32>> {
    chunk_pcm_ranges(pcm, max_chunk_duration_ms, silence_to_close_ms)
        .into_iter()
        .map(|range| pcm[range].to_vec())
        .collect()
}

/// Finds chunk boundaries without retaining a second copy of the complete
/// recording. Production uses these ranges to keep multi-hour imports close
/// to one decoded PCM buffer plus one bounded Chunker window.
pub fn chunk_pcm_ranges(
    pcm: &[f32],
    max_chunk_duration_ms: u32,
    silence_to_close_ms: u32,
) -> Vec<Range<usize>> {
    let mut vad = Vad::new(VadConfig::default());
    let mut chunker = Chunker::new(max_chunk_duration_ms, silence_to_close_ms);
    let mut ranges = Vec::new();
    let mut cursor = 0;
    for frame in pcm.chunks(CHUNKING_FRAME_SAMPLES) {
        let frame_ms = ((frame.len() as f64 / WHISPER_SAMPLE_RATE as f64) * 1000.0)
            .round()
            .max(1.0) as u32;
        let speech_active = vad.process_frame(rms(frame), frame_ms);
        if let ChunkEvent::Ready(chunk) = chunker.push_frame(frame, speech_active, frame_ms) {
            let end = cursor + chunk.len();
            ranges.push(cursor..end);
            cursor = end;
        }
    }
    if let ChunkEvent::Ready(chunk) = chunker.flush() {
        let end = cursor + chunk.len();
        ranges.push(cursor..end);
    }
    ranges
}

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

    #[test]
    fn chunk_pcm_on_empty_audio_produces_no_chunks() {
        assert_eq!(chunk_pcm(&[], 30_000, 800), Vec::<Vec<f32>>::new());
    }

    #[test]
    fn chunk_pcm_preserves_every_sample_across_chunk_boundaries() {
        // Loud tone for longer than the duration cap: no silence anywhere,
        // so only the safety cap can close chunks - exercises the same
        // "no samples lost" guarantee capture::chunker's own tests check,
        // end to end through the VAD this time.
        let pcm = vec![0.5_f32; WHISPER_SAMPLE_RATE as usize * 2]; // 2s
        let chunks = chunk_pcm(&pcm, 500, 800);
        let total: usize = chunks.iter().map(|chunk| chunk.len()).sum();
        assert_eq!(total, pcm.len());
        assert!(
            chunks.len() > 1,
            "expected the duration cap to split this into multiple chunks"
        );
    }

    #[test]
    fn chunk_pcm_closes_a_trailing_partial_chunk_via_flush() {
        // Speech followed by a silence too short to close the chunk on its
        // own, and total duration under the cap - only `flush()` at the end
        // should close it; if that path were missing, this audio would be
        // silently dropped.
        let mut pcm = vec![0.5_f32; WHISPER_SAMPLE_RATE as usize / 2]; // 0.5s speech
        pcm.extend(vec![0.0_f32; WHISPER_SAMPLE_RATE as usize / 10]); // 0.1s silence
        let chunks = chunk_pcm(&pcm, 30_000, 800);
        let total: usize = chunks.iter().map(|chunk| chunk.len()).sum();
        assert_eq!(total, pcm.len());
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn chunk_ranges_cover_audio_without_retaining_chunk_copies() {
        let pcm = vec![0.5_f32; WHISPER_SAMPLE_RATE as usize * 3];
        let ranges = chunk_pcm_ranges(&pcm, 500, 800);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, pcm.len());
        assert!(ranges.windows(2).all(|pair| pair[0].end == pair[1].start));
    }
}
