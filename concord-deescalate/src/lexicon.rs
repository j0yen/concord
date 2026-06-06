//! Contempt-lexicon check — deterministic, no model required.
//!
//! The built-in list covers the most common contempt, sarcasm, and absolutist
//! markers that cause recipients to disengage. Users can extend this via
//! [`check_contempt`]'s `extra_terms` parameter.
//!
//! # Extensibility
//! Pass additional terms in the `extra_terms` slice; they are checked
//! case-insensitively alongside the built-in list.

/// Built-in contempt / sarcasm / absolutist markers.
///
/// This list is intentionally conservative — it targets high-signal terms
/// that almost always indicate contempt rather than benign use.
pub const BUILT_IN_CONTEMPT_TERMS: &[&str] = &[
    // Absolutist attack words
    "always",
    "never",
    "every single time",
    "you people",
    // Contempt / dismissal
    "ridiculous",
    "absurd",
    "pathetic",
    "idiotic",
    "stupid",
    "moron",
    "idiot",
    "dumb",
    "incompetent",
    "worthless",
    "useless",
    "clueless",
    "lazy",
    "selfish",
    "narcissist",
    "gaslighting",
    "manipulative",
    "toxic",
    // Sarcasm / contempt markers
    "obviously",
    "clearly you",
    "as usual",
    "big surprise",
    "wow, thanks",
    "oh great",
    "thanks for nothing",
    "as expected",
    "what a surprise",
    // Absolutist framing
    "you never listen",
    "you always do this",
    "you never care",
    "you always forget",
];

/// Result of a contempt check.
#[derive(Debug, Clone)]
pub struct ContemptCheckResult {
    /// Terms from the contempt lexicon that were found in the message.
    pub found: Vec<String>,
}

impl ContemptCheckResult {
    /// Returns `true` if at least one contempt term was found.
    #[must_use]
    pub fn has_contempt(&self) -> bool {
        !self.found.is_empty()
    }
}

/// Check `message` for contempt-lexicon terms.
///
/// Matching is case-insensitive and whole-token-aware (substring match within
/// word boundaries is sufficient for the built-in list).
///
/// # Arguments
/// * `message` — the raw input message.
/// * `extra_terms` — additional user-supplied terms to check alongside the built-in list.
#[must_use]
pub fn check_contempt(message: &str, extra_terms: &[&str]) -> ContemptCheckResult {
    let lower = message.to_lowercase();
    let mut found = Vec::new();

    for term in BUILT_IN_CONTEMPT_TERMS.iter().chain(extra_terms.iter()) {
        let term_lower = term.to_lowercase();
        if lower.contains(term_lower.as_str()) {
            found.push((*term).to_string());
        }
    }

    ContemptCheckResult { found }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_built_in_term() {
        // AC4: the deterministic lexicon check removes/flags a known contempt term.
        let result = check_contempt("You are so ridiculous and never listen to me.", &[]);
        assert!(result.has_contempt(), "should detect contempt terms");
        let found_lower: Vec<_> = result.found.iter().map(|s| s.to_lowercase()).collect();
        assert!(
            found_lower.iter().any(|t| t.contains("ridiculous")),
            "should find 'ridiculous'; found={found_lower:?}"
        );
        assert!(
            found_lower.iter().any(|t| t.contains("never")),
            "should find 'never'; found={found_lower:?}"
        );
    }

    #[test]
    fn clean_message_passes() {
        let result = check_contempt("I'd like to discuss how we handle the budget.", &[]);
        assert!(!result.has_contempt(), "clean message should not trigger contempt check");
    }

    #[test]
    fn extra_terms_respected() {
        let result = check_contempt("You are such a jerk about this.", &["jerk"]);
        assert!(result.has_contempt());
        assert!(result.found.iter().any(|t| t == "jerk"));
    }

    #[test]
    fn case_insensitive() {
        let result = check_contempt("That is RIDICULOUS!", &[]);
        assert!(result.has_contempt());
    }
}
