//! Deterministic, content-only transcription quality metrics. Callers decide
//! where reports are stored; this module never logs reference or hypothesis.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ErrorRate {
    pub errors: usize,
    pub reference_units: usize,
    pub rate: f64,
}

pub fn normalize_text(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|character| match character {
            '’' | 'ʼ' => '\'',
            character if character.is_alphanumeric() || character == '\'' => character,
            _ => ' ',
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn word_error_rate(reference: &str, hypothesis: &str) -> ErrorRate {
    let reference = normalize_text(reference);
    let hypothesis = normalize_text(hypothesis);
    calculate(
        &reference.split_whitespace().collect::<Vec<_>>(),
        &hypothesis.split_whitespace().collect::<Vec<_>>(),
    )
}

pub fn character_error_rate(reference: &str, hypothesis: &str) -> ErrorRate {
    calculate(
        &normalize_text(reference).chars().collect::<Vec<_>>(),
        &normalize_text(hypothesis).chars().collect::<Vec<_>>(),
    )
}

fn calculate<T: Eq>(reference: &[T], hypothesis: &[T]) -> ErrorRate {
    let errors = edit_distance(reference, hypothesis);
    ErrorRate {
        errors,
        reference_units: reference.len(),
        rate: errors as f64 / reference.len().max(1) as f64,
    }
}

fn edit_distance<T: Eq>(reference: &[T], hypothesis: &[T]) -> usize {
    let mut previous: Vec<usize> = (0..=hypothesis.len()).collect();
    for (reference_index, reference_unit) in reference.iter().enumerate() {
        let mut current = Vec::with_capacity(hypothesis.len() + 1);
        current.push(reference_index + 1);
        for (hypothesis_index, hypothesis_unit) in hypothesis.iter().enumerate() {
            let substitution =
                previous[hypothesis_index] + usize::from(reference_unit != hypothesis_unit);
            let deletion = previous[hypothesis_index + 1] + 1;
            let insertion = current[hypothesis_index] + 1;
            current.push(substitution.min(deletion).min(insertion));
        }
        previous = current;
    }
    previous[hypothesis.len()]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpeakerRegion {
    pub start_ms: i64,
    pub end_ms: i64,
    /// Canonical speaker label after corpus-level speaker mapping. `None`
    /// represents non-speech.
    pub speaker: Option<String>,
    pub uncertain: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiarizationMetrics {
    pub missed_speech_ms: i64,
    pub false_alarm_ms: i64,
    pub speaker_confusion_ms: i64,
    pub reference_speech_ms: i64,
    pub der: f64,
    pub uncertain_ms: i64,
    pub hypothesis_speech_ms: i64,
    pub uncertain_coverage: f64,
}

/// Scores diarization on a deterministic 10 ms grid. Speaker identifiers
/// must already be mapped to the corpus' canonical labels; this keeps label
/// permutation policy explicit in the corpus runner rather than hidden here.
pub fn diarization_metrics(
    reference: &[SpeakerRegion],
    hypothesis: &[SpeakerRegion],
) -> DiarizationMetrics {
    let end_ms = reference
        .iter()
        .chain(hypothesis)
        .map(|region| region.end_ms)
        .max()
        .unwrap_or(0)
        .max(0);
    let mut missed = 0;
    let mut false_alarm = 0;
    let mut confusion = 0;
    let mut reference_speech = 0;
    let mut hypothesis_speech = 0;
    let mut uncertain = 0;

    for time in (0..end_ms).step_by(10) {
        let duration = (end_ms - time).min(10);
        let expected = active_region(reference, time);
        let actual = active_region(hypothesis, time);
        if expected
            .and_then(|region| region.speaker.as_ref())
            .is_some()
        {
            reference_speech += duration;
        }
        if let Some(region) = actual.filter(|region| region.speaker.is_some()) {
            hypothesis_speech += duration;
            if region.uncertain {
                uncertain += duration;
            }
        }
        match (
            expected.and_then(|region| region.speaker.as_deref()),
            actual.and_then(|region| region.speaker.as_deref()),
        ) {
            (Some(_), None) => missed += duration,
            (None, Some(_)) => false_alarm += duration,
            (Some(expected), Some(actual)) if expected != actual => confusion += duration,
            _ => {}
        }
    }

    DiarizationMetrics {
        missed_speech_ms: missed,
        false_alarm_ms: false_alarm,
        speaker_confusion_ms: confusion,
        reference_speech_ms: reference_speech,
        der: (missed + false_alarm + confusion) as f64 / reference_speech.max(1) as f64,
        uncertain_ms: uncertain,
        hypothesis_speech_ms: hypothesis_speech,
        uncertain_coverage: uncertain as f64 / hypothesis_speech.max(1) as f64,
    }
}

fn active_region(regions: &[SpeakerRegion], time_ms: i64) -> Option<&SpeakerRegion> {
    regions
        .iter()
        .find(|region| region.start_ms <= time_ms && time_ms < region.end_ms)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedText {
    pub start_ms: i64,
    pub end_ms: i64,
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundaryMetrics {
    pub duplicate_count: usize,
    pub compared_segments: usize,
    pub mean_start_drift_ms: f64,
    pub mean_end_drift_ms: f64,
    pub max_drift_ms: i64,
}

/// Measures consecutive overlapping duplicates and timestamp drift. Reference
/// and hypothesis segments are paired in chronological order after callers
/// have performed transcript alignment.
pub fn boundary_metrics(reference: &[TimedText], hypothesis: &[TimedText]) -> BoundaryMetrics {
    let duplicate_count = hypothesis
        .windows(2)
        .filter(|pair| {
            normalize_text(&pair[0].text) == normalize_text(&pair[1].text)
                && pair[1].start_ms < pair[0].end_ms
        })
        .count();
    let compared_segments = reference.len().min(hypothesis.len());
    let mut start_total = 0i64;
    let mut end_total = 0i64;
    let mut max_drift = 0i64;
    for (expected, actual) in reference.iter().zip(hypothesis) {
        let start = expected.start_ms.abs_diff(actual.start_ms) as i64;
        let end = expected.end_ms.abs_diff(actual.end_ms) as i64;
        start_total = start_total.saturating_add(start);
        end_total = end_total.saturating_add(end);
        max_drift = max_drift.max(start).max(end);
    }
    BoundaryMetrics {
        duplicate_count,
        compared_segments,
        mean_start_drift_ms: start_total as f64 / compared_segments.max(1) as f64,
        mean_end_drift_ms: end_total as f64 / compared_segments.max(1) as f64,
        max_drift_ms: max_drift,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualitySnapshot {
    pub wer: f64,
    pub cer: f64,
    pub der: f64,
    pub duplicate_rate: f64,
    pub max_timestamp_drift_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QualityThresholds {
    pub max_wer: f64,
    pub max_cer: f64,
    pub max_der: f64,
    pub max_duplicate_rate: f64,
    pub max_timestamp_drift_ms: i64,
}

/// Returns stable metric names for every release-blocking regression. Values
/// equal to a threshold pass; non-finite rates always fail closed.
pub fn quality_threshold_violations(
    snapshot: QualitySnapshot,
    thresholds: QualityThresholds,
) -> Vec<&'static str> {
    let mut violations = Vec::new();
    for (name, value, maximum) in [
        ("wer", snapshot.wer, thresholds.max_wer),
        ("cer", snapshot.cer, thresholds.max_cer),
        ("der", snapshot.der, thresholds.max_der),
        (
            "duplicate_rate",
            snapshot.duplicate_rate,
            thresholds.max_duplicate_rate,
        ),
    ] {
        if !value.is_finite() || value > maximum {
            violations.push(name);
        }
    }
    if snapshot.max_timestamp_drift_ms > thresholds.max_timestamp_drift_ms {
        violations.push("max_timestamp_drift_ms");
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_is_case_and_punctuation_insensitive_but_keeps_accents() {
        assert_eq!(
            normalize_text("  L’ENTRETIEN, était prêt ! "),
            "l'entretien était prêt"
        );
    }

    #[test]
    fn wer_counts_substitution_insertion_and_deletion() {
        let substitution = word_error_rate("un deux trois", "un quatre trois");
        assert_eq!(substitution.errors, 1);
        assert_eq!(substitution.rate, 1.0 / 3.0);
        assert_eq!(word_error_rate("un deux", "un beau deux").errors, 1);
        assert_eq!(word_error_rate("un deux", "un").errors, 1);
    }

    #[test]
    fn cer_uses_unicode_characters_not_utf8_bytes() {
        let metric = character_error_rate("été", "étés");
        assert_eq!(metric.reference_units, 3);
        assert_eq!(metric.errors, 1);
        assert_eq!(metric.rate, 1.0 / 3.0);
    }

    #[test]
    fn empty_inputs_have_finite_reproducible_rates() {
        assert_eq!(word_error_rate("", "").rate, 0.0);
        let insertion = word_error_rate("", "deux mots");
        assert_eq!(insertion.errors, 2);
        assert_eq!(insertion.reference_units, 0);
        assert_eq!(insertion.rate, 2.0);
    }

    #[test]
    fn identical_normalized_text_has_zero_error() {
        assert_eq!(
            word_error_rate("Bonjour, monde !", "bonjour monde").rate,
            0.0
        );
        assert_eq!(character_error_rate("Ça va ?", "ça va").rate, 0.0);
    }

    #[test]
    fn der_separates_misses_false_alarms_and_speaker_confusion() {
        let reference = vec![
            SpeakerRegion {
                start_ms: 0,
                end_ms: 100,
                speaker: Some("A".into()),
                uncertain: false,
            },
            SpeakerRegion {
                start_ms: 100,
                end_ms: 200,
                speaker: Some("B".into()),
                uncertain: false,
            },
        ];
        let hypothesis = vec![
            SpeakerRegion {
                start_ms: 0,
                end_ms: 50,
                speaker: Some("A".into()),
                uncertain: false,
            },
            SpeakerRegion {
                start_ms: 100,
                end_ms: 150,
                speaker: Some("A".into()),
                uncertain: false,
            },
            SpeakerRegion {
                start_ms: 200,
                end_ms: 250,
                speaker: Some("B".into()),
                uncertain: false,
            },
        ];
        let metric = diarization_metrics(&reference, &hypothesis);
        assert_eq!(metric.missed_speech_ms, 100);
        assert_eq!(metric.false_alarm_ms, 50);
        assert_eq!(metric.speaker_confusion_ms, 50);
        assert_eq!(metric.der, 1.0);
    }

    #[test]
    fn uncertain_coverage_is_measured_over_hypothesized_speech() {
        let hypothesis = vec![
            SpeakerRegion {
                start_ms: 0,
                end_ms: 40,
                speaker: Some("A".into()),
                uncertain: true,
            },
            SpeakerRegion {
                start_ms: 40,
                end_ms: 100,
                speaker: Some("A".into()),
                uncertain: false,
            },
        ];
        let metric = diarization_metrics(&hypothesis, &hypothesis);
        assert_eq!(metric.der, 0.0);
        assert_eq!(metric.uncertain_ms, 40);
        assert_eq!(metric.uncertain_coverage, 0.4);
    }

    #[test]
    fn empty_diarization_has_finite_zero_metrics() {
        let metric = diarization_metrics(&[], &[]);
        assert_eq!(metric.der, 0.0);
        assert_eq!(metric.uncertain_coverage, 0.0);
    }

    #[test]
    fn boundary_metrics_detect_overlapping_repeated_text() {
        let hypothesis = vec![
            TimedText {
                start_ms: 0,
                end_ms: 1_000,
                text: "Bonjour !".into(),
            },
            TimedText {
                start_ms: 900,
                end_ms: 1_500,
                text: "bonjour".into(),
            },
            TimedText {
                start_ms: 1_500,
                end_ms: 2_000,
                text: "Suite".into(),
            },
        ];
        assert_eq!(boundary_metrics(&[], &hypothesis).duplicate_count, 1);
    }

    #[test]
    fn boundary_metrics_report_mean_and_max_timestamp_drift() {
        let reference = vec![
            TimedText {
                start_ms: 100,
                end_ms: 500,
                text: "A".into(),
            },
            TimedText {
                start_ms: 600,
                end_ms: 1_000,
                text: "B".into(),
            },
        ];
        let hypothesis = vec![
            TimedText {
                start_ms: 120,
                end_ms: 540,
                text: "A".into(),
            },
            TimedText {
                start_ms: 660,
                end_ms: 990,
                text: "B".into(),
            },
        ];
        let metric = boundary_metrics(&reference, &hypothesis);
        assert_eq!(metric.compared_segments, 2);
        assert_eq!(metric.mean_start_drift_ms, 40.0);
        assert_eq!(metric.mean_end_drift_ms, 25.0);
        assert_eq!(metric.max_drift_ms, 60);
    }

    #[test]
    fn empty_boundaries_have_finite_zero_drift() {
        let metric = boundary_metrics(&[], &[]);
        assert_eq!(metric.mean_start_drift_ms, 0.0);
        assert_eq!(metric.mean_end_drift_ms, 0.0);
        assert_eq!(metric.max_drift_ms, 0);
    }

    fn initial_thresholds() -> QualityThresholds {
        QualityThresholds {
            max_wer: 0.45,
            max_cer: 0.30,
            max_der: 0.50,
            max_duplicate_rate: 0.02,
            max_timestamp_drift_ms: 500,
        }
    }

    #[test]
    fn quality_at_the_threshold_is_accepted() {
        let thresholds = initial_thresholds();
        let snapshot = QualitySnapshot {
            wer: thresholds.max_wer,
            cer: thresholds.max_cer,
            der: thresholds.max_der,
            duplicate_rate: thresholds.max_duplicate_rate,
            max_timestamp_drift_ms: thresholds.max_timestamp_drift_ms,
        };
        assert!(quality_threshold_violations(snapshot, thresholds).is_empty());
    }

    #[test]
    fn every_quality_regression_is_reported() {
        let violations = quality_threshold_violations(
            QualitySnapshot {
                wer: 0.46,
                cer: 0.31,
                der: 0.51,
                duplicate_rate: 0.03,
                max_timestamp_drift_ms: 501,
            },
            initial_thresholds(),
        );
        assert_eq!(
            violations,
            [
                "wer",
                "cer",
                "der",
                "duplicate_rate",
                "max_timestamp_drift_ms"
            ]
        );
    }

    #[test]
    fn non_finite_quality_rates_fail_closed() {
        let violations = quality_threshold_violations(
            QualitySnapshot {
                wer: f64::NAN,
                cer: 0.0,
                der: f64::INFINITY,
                duplicate_rate: 0.0,
                max_timestamp_drift_ms: 0,
            },
            initial_thresholds(),
        );
        assert_eq!(violations, ["wer", "der"]);
    }
}
