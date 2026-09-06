use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;
use serde::Serialize;

/// One span of the analyzed text: either kept (part of `cleaned_text`) or
/// removed (shown struck through in the UI, with a reason).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiffPart {
    pub kept: bool,
    pub text: String,
    pub reason: Option<&'static str>, // "hesitation" | "repetition" | "pause"
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct CleanupOutcome {
    pub cleaned_text: String,
    pub parts: Vec<DiffPart>,
}

/// Lexical hesitations only: non-lexical interjections with no semantic
/// content. Deliberately short and conservative — words like "ben", "bon",
/// "voila", "quoi", "alors", "donc", "enfin", "bah" are NOT included because
/// they carry real meaning depending on context, and removing them could
/// change intent (docs/PRODUCT.md: "ne pas corriger une formulation si cela
/// peut changer l'intention").
const HESITATION_WORDS: &[&str] = &["euh", "heu", "heuh", "hum", "humm", "hmm", "mmh", "mmm"];

static WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[\p{L}\p{N}]+(?:['’\-][\p{L}\p{N}]+)*").expect("hardcoded cleanup regex is valid")
});

#[derive(Debug, Clone)]
enum Token {
    Word(String),
    Sep(String),
}

fn token_text(token: &Token) -> String {
    match token {
        Token::Word(w) => w.clone(),
        Token::Sep(s) => s.clone(),
    }
}

/// Splits text into word tokens (letters/numbers, with a single internal
/// apostrophe/hyphen fused in, so French elisions like "j'ai" or "qu'est-ce"
/// stay one token) and separator tokens for everything else. Concatenating
/// every token's text in order reproduces the input exactly — tested below.
fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut last_end = 0;
    for m in WORD_RE.find_iter(text) {
        if m.start() > last_end {
            tokens.push(Token::Sep(text[last_end..m.start()].to_string()));
        }
        tokens.push(Token::Word(m.as_str().to_string()));
        last_end = m.end();
    }
    if last_end < text.len() {
        tokens.push(Token::Sep(text[last_end..].to_string()));
    }
    tokens
}

fn is_hesitation(word: &str) -> bool {
    let lower = word.to_lowercase();
    HESITATION_WORDS.contains(&lower.as_str())
}

/// A separator counts as a contentless pause only when it sits strictly
/// between two words (checked by the caller), isn't glued to either
/// neighbor, and its trimmed content is "..." (2+ dots) or a single "…". A
/// trailing "..." at the end of a segment usually signals a thought trailing
/// off, not a contentless pause, so it is never touched here (the caller
/// only calls this for separators with a word on both sides).
fn is_contentless_pause(sep_text: &str) -> bool {
    let trimmed = sep_text.trim();
    if trimmed.is_empty() {
        return false;
    }
    if !sep_text.starts_with(char::is_whitespace) || !sep_text.ends_with(char::is_whitespace) {
        return false;
    }
    trimmed == "…" || (trimmed.len() >= 2 && trimmed.chars().all(|c| c == '.'))
}

/// Picks one separator's punctuation (preferring `before`, falling back to
/// `after`) so removing a word between two separators doesn't leave a
/// doubled comma or a double space.
fn merge_separators(before: &str, after: &str) -> String {
    let keep_punct = |s: &str| -> String { s.chars().filter(|c| !c.is_whitespace()).collect() };
    let punct = {
        let b = keep_punct(before);
        if !b.is_empty() {
            b
        } else {
            keep_punct(after)
        }
    };
    if punct.is_empty() {
        " ".to_string()
    } else {
        format!("{punct} ")
    }
}

/// Analyzes `text` and proposes a cleaned version that removes only lexical
/// hesitations, immediate word repetitions, and contentless pauses — never
/// any actual wording. Concatenating every `kept: true` part's text
/// reproduces `cleaned_text` exactly; this is the single source of truth
/// used both to build the cleaned text and to render the diff in the UI (no
/// separate diff algorithm needed on the frontend).
pub fn analyze(text: &str) -> CleanupOutcome {
    let tokens = tokenize(text);
    let word_positions: Vec<usize> = tokens
        .iter()
        .enumerate()
        .filter_map(|(i, t)| matches!(t, Token::Word(_)).then_some(i))
        .collect();

    let mut removed_reason: Vec<Option<&'static str>> = vec![None; tokens.len()];

    // Hesitations and immediate repetitions, decided over the word-only
    // subsequence (so a separator in between never breaks the comparison).
    let mut prev_lower: Option<String> = None;
    for &pos in &word_positions {
        let Token::Word(word) = &tokens[pos] else {
            unreachable!("word_positions only contains Word token indices")
        };
        let lower = word.to_lowercase();
        if is_hesitation(word) {
            removed_reason[pos] = Some("hesitation");
        } else if prev_lower.as_deref() == Some(lower.as_str()) {
            removed_reason[pos] = Some("repetition");
        }
        prev_lower = Some(lower);
    }

    // Contentless pauses: a separator with a word strictly before and after
    // it in the original sequence (not after removal).
    for i in 0..tokens.len() {
        if let Token::Sep(sep) = &tokens[i] {
            let word_before = i > 0 && matches!(tokens[i - 1], Token::Word(_));
            let word_after = i + 1 < tokens.len() && matches!(tokens[i + 1], Token::Word(_));
            if word_before && word_after && is_contentless_pause(sep) {
                removed_reason[i] = Some("pause");
            }
        }
    }

    // Group consecutive removed words (a run like "je je je" spans several
    // word-subsequence positions). The separators strictly *between* the
    // words of one run are just stranded once the words are gone, so they
    // are dropped too; only the two *outer* boundary separators (if they
    // exist and are not already claimed by a neighboring run) might need
    // merging into one.
    let mut merged_insertion: HashMap<usize, String> = HashMap::new();
    let mut drop_boundary: HashSet<usize> = HashSet::new();

    let mut k = 0;
    while k < word_positions.len() {
        if removed_reason[word_positions[k]].is_none() {
            k += 1;
            continue;
        }
        let group_start_k = k;
        let mut group_end_k = k;
        while group_end_k + 1 < word_positions.len()
            && removed_reason[word_positions[group_end_k + 1]].is_some()
        {
            group_end_k += 1;
        }
        let first_pos = word_positions[group_start_k];
        let last_pos = word_positions[group_end_k];

        for slot in removed_reason.iter_mut().take(last_pos).skip(first_pos + 1) {
            if slot.is_none() {
                *slot = Some("repetition");
            }
        }

        let before_idx = first_pos
            .checked_sub(1)
            .filter(|&bi| matches!(tokens[bi], Token::Sep(_)) && removed_reason[bi].is_none());
        let after_idx = (last_pos + 1 < tokens.len())
            .then_some(last_pos + 1)
            .filter(|&ai| matches!(tokens[ai], Token::Sep(_)) && removed_reason[ai].is_none());
        if let (Some(bi), Some(ai)) = (before_idx, after_idx) {
            let before_text = token_text(&tokens[bi]);
            let after_text = token_text(&tokens[ai]);
            merged_insertion.insert(bi, merge_separators(&before_text, &after_text));
            drop_boundary.insert(ai);
        }

        k = group_end_k + 1;
    }

    let mut parts = Vec::with_capacity(tokens.len());
    let mut cleaned = String::with_capacity(text.len());
    let mut i = 0;
    while i < tokens.len() {
        if drop_boundary.contains(&i) {
            i += 1;
            continue;
        }
        if let Some(merged) = merged_insertion.get(&i) {
            parts.push(DiffPart {
                kept: true,
                text: merged.clone(),
                reason: None,
            });
            cleaned.push_str(merged);
            i += 1;
            continue;
        }
        match removed_reason[i] {
            Some("pause") => {
                // Removing a pause still needs a single space left behind so
                // the surrounding words don't glue together.
                parts.push(DiffPart {
                    kept: false,
                    text: token_text(&tokens[i]),
                    reason: Some("pause"),
                });
                parts.push(DiffPart {
                    kept: true,
                    text: " ".to_string(),
                    reason: None,
                });
                cleaned.push(' ');
            }
            Some(reason) => {
                parts.push(DiffPart {
                    kept: false,
                    text: token_text(&tokens[i]),
                    reason: Some(reason),
                });
            }
            None => {
                let text = token_text(&tokens[i]);
                cleaned.push_str(&text);
                parts.push(DiffPart {
                    kept: true,
                    text,
                    reason: None,
                });
            }
        }
        i += 1;
    }

    CleanupOutcome {
        cleaned_text: cleaned,
        parts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reconstruct(tokens: &[Token]) -> String {
        tokens.iter().map(token_text).collect()
    }

    #[test]
    fn tokenizer_round_trips_arbitrary_text() {
        for input in [
            "",
            "Bonjour",
            "Bonjour, le monde !",
            "  espaces   multiples  ",
            "j'ai qu'est-ce peut-être",
            "Ça coûte 12,50€ ; vraiment ?",
            "euh euh euh",
        ] {
            assert_eq!(reconstruct(&tokenize(input)), input, "input: {input:?}");
        }
    }

    #[test]
    fn removes_each_hesitation_word_in_isolation() {
        for word in HESITATION_WORDS {
            let input = format!("Alors, {word}, je pense que oui.");
            let outcome = analyze(&input);
            assert!(
                !outcome.cleaned_text.to_lowercase().contains(word),
                "expected {word:?} to be removed from {:?}",
                outcome.cleaned_text
            );
        }
    }

    #[test]
    fn never_touches_discourse_markers_that_carry_meaning() {
        for word in [
            "ben", "bon", "voila", "quoi", "alors", "donc", "enfin", "bah",
        ] {
            let input = format!("{word} je pense que oui");
            let outcome = analyze(&input);
            assert_eq!(
                outcome.cleaned_text, input,
                "word {word:?} must be untouched"
            );
            assert!(outcome.parts.iter().all(|p| p.kept));
        }
    }

    #[test]
    fn collapses_a_simple_hesitation_with_surrounding_commas() {
        let outcome = analyze("Alors, euh, je pense");
        assert_eq!(outcome.cleaned_text, "Alors, je pense");
    }

    #[test]
    fn collapses_a_bare_hesitation_between_spaces() {
        let outcome = analyze("Alors euh je pense");
        assert_eq!(outcome.cleaned_text, "Alors je pense");
    }

    #[test]
    fn collapses_a_run_of_three_immediate_repetitions() {
        let outcome = analyze("je je je pense");
        assert_eq!(outcome.cleaned_text, "je pense");
    }

    #[test]
    fn repetition_detection_is_case_insensitive_and_handles_elision() {
        let outcome = analyze("j'ai j'ai oublié");
        assert_eq!(outcome.cleaned_text, "j'ai oublié");

        let outcome = analyze("Le Le chat");
        assert_eq!(outcome.cleaned_text, "Le chat");
    }

    #[test]
    fn removes_an_internal_contentless_pause() {
        let outcome = analyze("il a dit ... qu'il reviendrait");
        assert_eq!(outcome.cleaned_text, "il a dit qu'il reviendrait");
    }

    #[test]
    fn preserves_a_trailing_ellipsis() {
        let input = "je ne sais pas...";
        let outcome = analyze(input);
        assert_eq!(outcome.cleaned_text, input);
    }

    #[test]
    fn preserves_an_ellipsis_glued_to_a_word() {
        let input = "attends...voila";
        let outcome = analyze(input);
        assert_eq!(outcome.cleaned_text, input);
    }

    #[test]
    fn combined_realistic_sentence() {
        let outcome = analyze("Alors euh, je je pense que ... c'est bien");
        assert_eq!(outcome.cleaned_text, "Alors, je pense que c'est bien");
    }

    #[test]
    fn nothing_to_clean_is_a_no_op() {
        let input = "Ceci est une phrase tout a fait normale.";
        let outcome = analyze(input);
        assert_eq!(outcome.cleaned_text, input);
        assert!(outcome.parts.iter().all(|p| p.kept));
    }

    #[test]
    fn kept_parts_concatenated_always_equal_cleaned_text() {
        for input in [
            "Alors, euh, je je pense que ... c'est bien",
            "Rien a nettoyer ici.",
            "",
        ] {
            let outcome = analyze(input);
            let rebuilt: String = outcome
                .parts
                .iter()
                .filter(|p| p.kept)
                .map(|p| p.text.as_str())
                .collect();
            assert_eq!(rebuilt, outcome.cleaned_text, "input: {input:?}");
        }
    }
}
