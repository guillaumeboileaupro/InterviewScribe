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

    /// Manual QA tool for docs/ROADMAP.md's "tester plusieurs accents francais et
    /// niveaux de bruit" checklist item. No content assertion: inputs vary (accent,
    /// noise, language), so the point is printing the transcript for human review,
    /// not automated pass/fail. Never runs in CI; never commits a model or a real
    /// recording (AGENTS.md).
    ///
    /// Optionally mixes in synthetic white noise at a target SNR (dB) to test noise
    /// robustness reproducibly, instead of hunting for real noisy recordings with
    /// unknown ground truth.
    ///
    /// Run with:
    ///   INTERVIEWSCRIBE_TEST_MODEL=/path/to/model.bin \
    ///   INTERVIEWSCRIBE_TEST_WAV=/path/to/sample.mp3 \
    ///   INTERVIEWSCRIBE_TEST_LANG=fr \
    ///   [INTERVIEWSCRIBE_TEST_NOISE_DB=10] \
    ///   cargo test --manifest-path src-tauri/Cargo.toml --release -- --ignored --nocapture whisper_qa_sample
    #[test]
    #[ignore]
    fn whisper_qa_sample() {
        let model_path = std::env::var("INTERVIEWSCRIBE_TEST_MODEL")
            .expect("set INTERVIEWSCRIBE_TEST_MODEL to a local ggml model path");
        let wav_path = std::env::var("INTERVIEWSCRIBE_TEST_WAV")
            .expect("set INTERVIEWSCRIBE_TEST_WAV to a local audio path");
        let language = std::env::var("INTERVIEWSCRIBE_TEST_LANG").ok();
        let noise_db: Option<f32> = std::env::var("INTERVIEWSCRIBE_TEST_NOISE_DB")
            .ok()
            .and_then(|value| value.parse().ok());

        let mut pcm = crate::audio::decode::decode_to_mono_pcm16k(std::path::Path::new(&wav_path))
            .expect("failed to decode test audio");
        if let Some(snr_db) = noise_db {
            pcm = add_white_noise(&pcm, snr_db, 42);
            println!("-- mixed in white noise at {snr_db} dB SNR --");
        }

        let transcriber = WhisperCppTranscriber::load(std::path::Path::new(&model_path))
            .expect("failed to load model");
        let segments = transcriber
            .transcribe(&pcm, language.as_deref())
            .expect("failed to transcribe");

        println!(
            "-- {} segment(s), language hint: {:?} --",
            segments.len(),
            language
        );
        for segment in &segments {
            println!(
                "[{} - {}] (confidence {:?}) {}",
                segment.start_ms, segment.end_ms, segment.confidence, segment.text
            );
        }
    }

    /// Reproducible public-corpus qualification. Unlike `whisper_qa_sample`,
    /// this never prints recognized or reference text: only aggregate WER/CER.
    /// Run manually/nightly with the pinned files from tests/corpus/manifest.json.
    #[test]
    #[ignore]
    fn whisper_public_french_quality_thresholds() {
        let model_path = std::env::var("INTERVIEWSCRIBE_TEST_MODEL")
            .expect("set INTERVIEWSCRIBE_TEST_MODEL to a local ggml model path");
        let audio_path = std::env::var("INTERVIEWSCRIBE_TEST_WAV")
            .expect("set INTERVIEWSCRIBE_TEST_WAV to the pinned public French audio");
        let pcm = crate::audio::decode::decode_to_mono_pcm16k(std::path::Path::new(&audio_path))
            .expect("failed to decode public corpus audio");
        let transcriber = WhisperCppTranscriber::load(std::path::Path::new(&model_path))
            .expect("failed to load model");
        let segments = transcriber
            .transcribe(&pcm, Some("fr"))
            .expect("failed to transcribe public corpus audio");
        let hypothesis = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let reference = "avoir un chat dans la gorge";
        let wer = crate::evaluation::word_error_rate(reference, &hypothesis).rate;
        let cer = crate::evaluation::character_error_rate(reference, &hypothesis).rate;
        println!("public-fr metrics: wer={wer:.4}, cer={cer:.4}");
        let violations = crate::evaluation::quality_threshold_violations(
            crate::evaluation::QualitySnapshot {
                wer,
                cer,
                der: 0.0,
                duplicate_rate: 0.0,
                max_timestamp_drift_ms: 0,
            },
            crate::evaluation::QualityThresholds {
                max_wer: 0.45,
                max_cer: 0.30,
                max_der: 0.50,
                max_duplicate_rate: 0.02,
                max_timestamp_drift_ms: 500,
            },
        );
        assert!(
            violations.is_empty(),
            "quality thresholds exceeded: {violations:?}"
        );
    }

    /// Deterministic PRNG-based white noise, scaled to hit a target SNR against the
    /// given signal. No external `rand` dependency needed for a test-only helper.
    #[cfg(test)]
    fn add_white_noise(pcm: &[f32], snr_db: f32, seed: u64) -> Vec<f32> {
        let signal_power = pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len().max(1) as f32;
        let noise_power = signal_power / 10f32.powf(snr_db / 10.0);
        let noise_amplitude = noise_power.sqrt();

        let mut state = seed.max(1);
        pcm.iter()
            .map(|sample| {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                let uniform = ((state >> 40) as f32) / (1u64 << 24) as f32; // [0, 1)
                let noise = (uniform * 2.0 - 1.0) * noise_amplitude;
                sample + noise
            })
            .collect()
    }
}
