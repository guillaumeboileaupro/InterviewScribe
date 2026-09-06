use std::path::Path;
use std::sync::Once;

use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use super::{RawSegment, Transcriber};
use crate::error::AppError;

pub struct WhisperCppTranscriber {
    context: WhisperContext,
}

impl WhisperCppTranscriber {
    pub fn load(model_path: &Path) -> Result<Self, AppError> {
        // Native debug logs may include transcript tokens. No logging adapter
        // features are enabled, so these hooks discard whisper.cpp/GGML logs.
        static LOGGING: Once = Once::new();
        LOGGING.call_once(whisper_rs::install_logging_hooks);
        let path = model_path
            .to_str()
            .ok_or_else(|| AppError::Model("chemin de modele invalide (UTF-8)".into()))?;
        let context = WhisperContext::new_with_params(path, WhisperContextParameters::default())
            .map_err(|err| AppError::Model(format!("echec de chargement du modele: {err}")))?;
        Ok(Self { context })
    }
}

impl Transcriber for WhisperCppTranscriber {
    fn transcribe(&self, pcm: &[f32], language: Option<&str>) -> Result<Vec<RawSegment>, AppError> {
        let mut state = self
            .context
            .create_state()
            .map_err(|err| AppError::Transcription(format!("echec de creation d'etat: {err}")))?;

        let mut params = FullParams::new(SamplingStrategy::BeamSearch {
            beam_size: 5,
            patience: -1.0,
        });
        params.set_language(language);
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);

        state
            .full(params, pcm)
            .map_err(|err| AppError::Transcription(format!("echec de la transcription: {err}")))?;

        let mut segments = Vec::new();
        for segment in state.as_iter() {
            let text = segment
                .to_str_lossy()
                .map_err(|err| AppError::Transcription(format!("texte invalide: {err}")))?
                .trim()
                .to_string();
            if text.is_empty() {
                continue;
            }
            segments.push(RawSegment {
                start_ms: centiseconds_to_ms(segment.start_timestamp()),
                end_ms: centiseconds_to_ms(segment.end_timestamp()),
                text,
                confidence: Some(1.0 - segment.no_speech_probability() as f64),
            });
        }
        Ok(segments)
    }
}

/// whisper.cpp reports timestamps in centiseconds (10ms units); the app stores milliseconds.
fn centiseconds_to_ms(centiseconds: i64) -> i64 {
    centiseconds * 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_centiseconds_to_milliseconds() {
        assert_eq!(centiseconds_to_ms(0), 0);
        assert_eq!(centiseconds_to_ms(150), 1_500);
    }

    /// Manual end-to-end smoke test against a real model and real speech audio.
    /// Never runs in CI (AGENTS.md: never commit a Whisper model or real recording).
    /// Run with:
    ///   INTERVIEWSCRIBE_TEST_MODEL=/path/to/ggml-large-v3-turbo-q5_0.bin \
    ///   INTERVIEWSCRIBE_TEST_WAV=/path/to/jfk.wav \
    ///   cargo test --manifest-path src-tauri/Cargo.toml -- --ignored whisper_smoke
    #[test]
    #[ignore]
    fn whisper_smoke() {
        let model_path = std::env::var("INTERVIEWSCRIBE_TEST_MODEL")
            .expect("set INTERVIEWSCRIBE_TEST_MODEL to a local ggml model path");
        let wav_path = std::env::var("INTERVIEWSCRIBE_TEST_WAV")
            .expect("set INTERVIEWSCRIBE_TEST_WAV to a local wav path");

        let pcm = crate::audio::decode::decode_to_mono_pcm16k(std::path::Path::new(&wav_path))
            .expect("failed to decode test wav");
        let transcriber = WhisperCppTranscriber::load(std::path::Path::new(&model_path))
            .expect("failed to load model");
        let segments = transcriber
            .transcribe(&pcm, Some("en"))
            .expect("failed to transcribe");

        assert!(!segments.is_empty(), "expected at least one segment");
        for segment in &segments {
            println!(
                "[{} - {}] {}",
                segment.start_ms, segment.end_ms, segment.text
            );
            assert!(segment.end_ms > segment.start_ms);
        }
        let full_text: String = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            full_text.to_lowercase().contains("country"),
            "expected the well-known JFK sample to mention 'country', got: {full_text}"
        );
    }
}
