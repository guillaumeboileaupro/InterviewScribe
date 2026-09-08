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

    #[cfg(not(target_os = "android"))]
    #[derive(serde::Deserialize)]
    struct AmiReference {
        words: Vec<AmiWord>,
    }

    #[cfg(not(target_os = "android"))]
    #[derive(serde::Deserialize)]
    struct AmiWord {
        speaker: String,
        start_ms: i64,
        end_ms: i64,
        text: String,
    }

    /// Full opt-in qualification of the desktop Whisper + WeSpeaker pipeline.
    /// Inputs are generated from the pinned AMI source by `pnpm corpus:prepare`.
    /// Only aggregate metrics are printed: neither transcript nor reference text.
    #[cfg(not(target_os = "android"))]
    #[test]
    #[ignore]
    fn ami_four_speaker_quality_metrics() {
        let model_path = std::env::var("INTERVIEWSCRIBE_TEST_MODEL")
            .expect("set INTERVIEWSCRIBE_TEST_MODEL to the local Whisper model");
        let diarization_path = std::env::var("INTERVIEWSCRIBE_TEST_DIARIZATION_MODEL")
            .expect("set INTERVIEWSCRIBE_TEST_DIARIZATION_MODEL to the local WeSpeaker model");
        let audio_path = std::env::var("INTERVIEWSCRIBE_TEST_WAV")
            .expect("set INTERVIEWSCRIBE_TEST_WAV to generated multi-clean-4.wav");
        let reference_path = std::env::var("INTERVIEWSCRIBE_TEST_AMI_REFERENCE")
            .expect("set INTERVIEWSCRIBE_TEST_AMI_REFERENCE to generated ami-reference.json");
        let reference: AmiReference = serde_json::from_slice(
            &std::fs::read(reference_path).expect("failed to read AMI reference"),
        )
        .expect("failed to parse AMI reference");
        let pcm = crate::audio::decode::decode_to_mono_pcm16k(std::path::Path::new(&audio_path))
            .expect("failed to decode AMI audio");
        let transcriber = WhisperCppTranscriber::load(std::path::Path::new(&model_path))
            .expect("failed to load Whisper model");
        let segments = transcriber
            .transcribe(&pcm, Some("en"))
            .expect("failed to transcribe AMI audio");

        let mut extractor =
            crate::diarization::EmbeddingExtractor::load(std::path::Path::new(&diarization_path))
                .expect("failed to load WeSpeaker model");
        let mut clusterer = crate::diarization::Clusterer::new(Some(4));
        let assignments = segments
            .iter()
            .map(|segment| {
                let audio =
                    crate::diarization::slice_pcm_ms(&pcm, segment.start_ms, segment.end_ms);
                let embedding = extractor.extract(audio).expect("embedding failed");
                clusterer.assign(&embedding)
            })
            .collect::<Vec<_>>();

        let labels = best_ami_cluster_labels(&segments, &assignments, &reference.words);
        let word_regions = reference
            .words
            .iter()
            .map(|word| crate::evaluation::SpeakerRegion {
                start_ms: word.start_ms,
                end_ms: word.end_ms,
                speaker: Some(word.speaker.clone()),
                uncertain: false,
            })
            .collect::<Vec<_>>();
        let reference_regions = ami_speaker_turns(&reference.words, 500);
        let hypothesis_regions = segments
            .iter()
            .zip(&assignments)
            .map(|(segment, assignment)| crate::evaluation::SpeakerRegion {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                speaker: labels.get(assignment.speaker_index).cloned(),
                uncertain: assignment.uncertain,
            })
            .collect::<Vec<_>>();
        let word_diarization =
            crate::evaluation::diarization_metrics(&word_regions, &hypothesis_regions);
        let diarization =
            crate::evaluation::diarization_metrics(&reference_regions, &hypothesis_regions);
        let reference_text = reference
            .words
            .iter()
            .map(|word| word.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let hypothesis_text = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let wer = crate::evaluation::word_error_rate(&reference_text, &hypothesis_text).rate;
        let cer = crate::evaluation::character_error_rate(&reference_text, &hypothesis_text).rate;
        let timed = segments
            .iter()
            .map(|segment| crate::evaluation::TimedText {
                start_ms: segment.start_ms,
                end_ms: segment.end_ms,
                text: segment.text.clone(),
            })
            .collect::<Vec<_>>();
        let boundaries = crate::evaluation::boundary_metrics(&[], &timed);
        let duplicate_rate =
            boundaries.duplicate_count as f64 / timed.len().saturating_sub(1).max(1) as f64;

        println!(
            "ami metrics: segments={}, clusters={}, wer={wer:.4}, cer={cer:.4}, der={:.4}, word_region_der={:.4}, missed_ms={}, false_alarm_ms={}, confusion_ms={}, uncertain={:.4}, duplicates={duplicate_rate:.4}",
            segments.len(),
            clusterer.speaker_count(),
            diarization.der,
            word_diarization.der,
            diarization.missed_speech_ms,
            diarization.false_alarm_ms,
            diarization.speaker_confusion_ms,
            diarization.uncertain_coverage,
        );
        assert!(!segments.is_empty(), "expected speech segments");
        assert_eq!(
            clusterer.speaker_count(),
            4,
            "the four-speaker fixture must yield four clusters"
        );
        let violations = crate::evaluation::quality_threshold_violations(
            crate::evaluation::QualitySnapshot {
                wer,
                cer,
                der: diarization.der,
                duplicate_rate,
                // Drift requires word-aligned hypothesis timestamps; it is not
                // fabricated from unrelated segment indexes in this runner.
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
            "AMI quality thresholds exceeded: {violations:?}"
        );
    }

    /// Converts word-level NXT annotations into speech turns. Whisper emits
    /// continuous segment spans, so treating every natural inter-word pause as
    /// non-speech would incorrectly count those pauses as false alarms. A 500 ms
    /// maximum gap is fixed here and reported as part of the qualification
    /// protocol; speaker changes always start a separate turn.
    #[cfg(not(target_os = "android"))]
    fn ami_speaker_turns(
        words: &[AmiWord],
        maximum_gap_ms: i64,
    ) -> Vec<crate::evaluation::SpeakerRegion> {
        let mut turns = Vec::<crate::evaluation::SpeakerRegion>::new();
        for speaker in ["A", "B", "C", "D"] {
            for word in words.iter().filter(|word| word.speaker == speaker) {
                if let Some(turn) = turns.last_mut().filter(|turn| {
                    turn.speaker.as_deref() == Some(speaker)
                        && word.start_ms - turn.end_ms <= maximum_gap_ms
                }) {
                    turn.end_ms = turn.end_ms.max(word.end_ms);
                } else {
                    turns.push(crate::evaluation::SpeakerRegion {
                        start_ms: word.start_ms,
                        end_ms: word.end_ms,
                        speaker: Some(speaker.to_string()),
                        uncertain: false,
                    });
                }
            }
        }
        turns.sort_by_key(|turn| turn.start_ms);
        turns
    }

    #[cfg(not(target_os = "android"))]
    fn best_ami_cluster_labels(
        segments: &[RawSegment],
        assignments: &[crate::diarization::Assignment],
        words: &[AmiWord],
    ) -> Vec<String> {
        let cluster_count = assignments
            .iter()
            .map(|assignment| assignment.speaker_index + 1)
            .max()
            .unwrap_or(0);
        let speakers = ["A", "B", "C", "D"];
        let mut overlap = vec![vec![0i64; speakers.len()]; cluster_count];
        for (segment, assignment) in segments.iter().zip(assignments) {
            for word in words {
                let duration =
                    segment.end_ms.min(word.end_ms) - segment.start_ms.max(word.start_ms);
                if duration > 0 {
                    if let Some(speaker_index) = speakers.iter().position(|s| *s == word.speaker) {
                        overlap[assignment.speaker_index][speaker_index] += duration;
                    }
                }
            }
        }
        let mut best_score = -1;
        let mut best = Vec::new();
        assign_ami_labels(&overlap, 0, &mut Vec::new(), &mut best_score, &mut best);
        best.into_iter()
            .map(|index| speakers[index].to_string())
            .collect()
    }

    #[cfg(not(target_os = "android"))]
    fn assign_ami_labels(
        overlap: &[Vec<i64>],
        cluster: usize,
        current: &mut Vec<usize>,
        best_score: &mut i64,
        best: &mut Vec<usize>,
    ) {
        if cluster == overlap.len() {
            let score = current
                .iter()
                .enumerate()
                .map(|(index, speaker)| overlap[index][*speaker])
                .sum();
            if score > *best_score {
                *best_score = score;
                *best = current.clone();
            }
            return;
        }
        for speaker in 0..4 {
            if !current.contains(&speaker) {
                current.push(speaker);
                assign_ami_labels(overlap, cluster + 1, current, best_score, best);
                current.pop();
            }
        }
    }

    #[cfg(not(target_os = "android"))]
    #[test]
    fn ami_word_annotations_are_grouped_into_speaker_turns() {
        let words = vec![
            AmiWord {
                speaker: "A".into(),
                start_ms: 100,
                end_ms: 300,
                text: String::new(),
            },
            AmiWord {
                speaker: "A".into(),
                start_ms: 600,
                end_ms: 800,
                text: String::new(),
            },
            AmiWord {
                speaker: "A".into(),
                start_ms: 1_400,
                end_ms: 1_600,
                text: String::new(),
            },
            AmiWord {
                speaker: "B".into(),
                start_ms: 400,
                end_ms: 500,
                text: String::new(),
            },
        ];

        let turns = ami_speaker_turns(&words, 500);

        assert_eq!(turns.len(), 3);
        assert_eq!((turns[0].start_ms, turns[0].end_ms), (100, 800));
        assert_eq!(turns[1].speaker.as_deref(), Some("B"));
        assert_eq!((turns[2].start_ms, turns[2].end_ms), (1_400, 1_600));
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
