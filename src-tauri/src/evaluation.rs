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
}
