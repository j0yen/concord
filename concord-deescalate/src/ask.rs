//! Substantive-ask extraction — rule-based pass followed by model-assisted refinement.
//!
//! The rule-based pass uses imperative sentence detection, question detection,
//! and demand/request keyword detection to produce an initial list of asks.
//! The model-assisted pass then refines this list via a structured prompt.
//!
//! The post-check in [`verify_asks_preserved`] is deterministic: it checks
//! each extracted ask (lowercased key-noun/verb) appears semantically in the
//! rephrased text.

use anyhow::Result;
use concord_steelman::ConcordModel;

use crate::prompt;

/// A single substantive ask extracted from the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    /// The ask in the form extracted (verbatim or cleaned).
    pub text: String,
}

// ── Rule-based extraction ────────────────────────────────────────────────────

/// Keyword / pattern indicators that a sentence is a demand or request.
const REQUEST_KEYWORDS: &[&str] = &[
    "please",
    "need you to",
    "want you to",
    "require",
    "would like",
    "should",
    "must",
    "have to",
    "need to",
    "ask you to",
    "requesting",
    "demand",
    "expect you to",
];

/// Split `message` into sentences (naive, period/exclamation/question split).
fn split_sentences(message: &str) -> Vec<&str> {
    // Split on sentence-ending punctuation while keeping the delimiter in the
    // preceding segment (for question detection).
    let mut out = Vec::new();
    let mut start = 0;
    for (i, ch) in message.char_indices() {
        if matches!(ch, '.' | '!' | '?') {
            let seg = message[start..=i].trim();
            if !seg.is_empty() {
                out.push(seg);
            }
            start = i + 1;
        }
    }
    let tail = message[start..].trim();
    if !tail.is_empty() {
        out.push(tail);
    }
    out
}

/// Rule-based ask extraction (no model).
///
/// Extracts sentences that look like questions, imperatives, or requests.
#[must_use]
pub fn extract_asks_rule_based(message: &str) -> Vec<Ask> {
    let sentences = split_sentences(message);
    let mut asks = Vec::new();

    for sent in sentences {
        let lower = sent.to_lowercase();
        let is_question = sent.ends_with('?');
        let has_request_keyword = REQUEST_KEYWORDS.iter().any(|k| lower.contains(k));
        // Imperative: starts with a verb-like word (simple heuristic)
        let first_word = sent.split_whitespace().next().unwrap_or("").to_lowercase();
        let imperative_starts: &[&str] = &[
            "stop", "start", "fix", "send", "give", "tell", "show",
            "respond", "reply", "return", "finish", "complete", "do",
            "make", "explain", "help", "apologize", "acknowledge",
            "provide", "update", "change", "get", "bring", "write",
            "call", "email", "let", "schedule", "cancel", "move",
            "confirm", "approve", "review", "submit", "forward",
            "share", "check", "create", "delete", "add", "remove",
        ];
        let is_imperative = imperative_starts.contains(&first_word.as_str());

        if is_question || has_request_keyword || is_imperative {
            asks.push(Ask { text: sent.to_string() });
        }
    }

    asks
}

// ── Model-assisted extraction ────────────────────────────────────────────────

/// Model-parsed ask from the structured JSON response.
#[derive(serde::Deserialize)]
struct ModelAsks {
    asks: Vec<String>,
}

/// Use the model to extract substantive asks from `message`.
///
/// Falls back to the rule-based result if the model response cannot be parsed.
///
/// # Errors
/// Returns an error only if the model backend itself fails (network, timeout).
/// JSON parse failures fall back to rule-based output, not an error.
pub fn extract_asks_with_model(
    message: &str,
    model: &dyn ConcordModel,
) -> Result<Vec<Ask>> {
    let p = prompt::build_extract_asks_prompt(message);
    let response = model.complete(&p)?;

    // Attempt to parse the JSON response.
    if let Ok(parsed) = serde_json::from_str::<ModelAsks>(&response) {
        if !parsed.asks.is_empty() {
            return Ok(parsed.asks.into_iter().map(|t| Ask { text: t }).collect());
        }
    }

    // Fallback: rule-based.
    Ok(extract_asks_rule_based(message))
}

// ── Post-check ───────────────────────────────────────────────────────────────

/// Verify that each extracted ask is still (semantically) present in `rephrase`.
///
/// "Semantically present" is approximated deterministically: for each ask we
/// extract its key nouns/verbs (words >4 chars that aren't stop words) and
/// check that at least one appears in the rephrase (case-insensitive).
///
/// Returns the list of asks that appear to be **missing** from the rephrase.
/// An empty return means all asks are preserved.
#[must_use]
pub fn verify_asks_preserved(asks: &[Ask], rephrase: &str) -> Vec<String> {
    const STOP_WORDS: &[&str] = &[
        "about", "after", "again", "also", "another", "because", "been",
        "before", "could", "doing", "during", "every", "from", "have",
        "here", "into", "just", "like", "more", "most", "need", "next",
        "only", "other", "over", "please", "really", "should", "since",
        "some", "such", "than", "that", "their", "there", "these", "they",
        "this", "those", "through", "under", "want", "were", "what", "when",
        "where", "which", "while", "will", "with", "would", "your",
    ];

    let rephrase_lower = rephrase.to_lowercase();
    let mut missing = Vec::new();

    'outer: for ask in asks {
        let ask_lower = ask.text.to_lowercase();
        // Extract key words: >4 chars, not a stop word.
        let key_words: Vec<&str> = ask_lower
            .split(|c: char| !c.is_alphabetic())
            .filter(|w| w.len() > 4 && !STOP_WORDS.contains(w))
            .collect();

        if key_words.is_empty() {
            // Can't verify; assume present.
            continue;
        }

        // If at least one key word appears in the rephrase, the ask is preserved.
        for kw in &key_words {
            if rephrase_lower.contains(*kw) {
                continue 'outer;
            }
        }

        missing.push(ask.text.clone());
    }

    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_based_finds_question() {
        let asks = extract_asks_rule_based("Why did you cancel the meeting?");
        assert!(!asks.is_empty(), "should detect question as ask");
    }

    #[test]
    fn rule_based_finds_request_keyword() {
        let asks =
            extract_asks_rule_based("I need you to send me the report by Friday.");
        assert!(!asks.is_empty(), "should detect 'need you to' as ask");
    }

    #[test]
    fn rule_based_finds_imperative() {
        let asks = extract_asks_rule_based("Stop interrupting me when I'm speaking.");
        assert!(!asks.is_empty(), "should detect imperative as ask");
    }

    #[test]
    fn verify_preserves_present_ask() {
        let asks = vec![Ask { text: "Please send the report by Friday.".to_string() }];
        let rephrase = "I would appreciate receiving the report before the end of Friday.";
        let missing = verify_asks_preserved(&asks, rephrase);
        // "report" and "friday" both appear — preserved.
        assert!(missing.is_empty(), "ask should be preserved; missing={missing:?}");
    }

    #[test]
    fn verify_catches_dropped_ask() {
        // AC3: a MockModel rephrase that drops one ask is caught by post-check.
        let asks = vec![
            Ask { text: "Please send the quarterly budget report.".to_string() },
            Ask { text: "Schedule a meeting to discuss the proposal.".to_string() },
        ];
        // Rephrase only covers the budget ask, drops the meeting ask entirely.
        let rephrase = "I would appreciate receiving the quarterly budget report.";
        let missing = verify_asks_preserved(&asks, rephrase);
        assert!(
            !missing.is_empty(),
            "dropped ask should be flagged; missing={missing:?}"
        );
        // The budget ask should be preserved.
        assert!(
            missing.iter().all(|m| !m.to_lowercase().contains("budget")),
            "budget ask should be preserved; missing={missing:?}"
        );
    }
}
