//! Safety boundary — refuses to rephrase threats or harassment.
//!
//! This check runs **before** any model call.  If a threat-of-harm or
//! harassment pattern is detected the engine returns [`SafetyViolation`]
//! rather than producing a rephrase.
//!
//! The list is documented and tested.  It is intentionally narrow — only
//! explicit threat patterns are blocked, not all aggressive language (that
//! is handled by the contempt lexicon and model rephrase).

/// Threat-of-harm / harassment patterns that block rephrasing.
///
/// Patterns are matched case-insensitively as substrings.
pub const THREAT_PATTERNS: &[&str] = &[
    "i will kill",
    "i'm going to kill",
    "i am going to kill",
    "gonna kill",
    "going to hurt",
    "i will hurt",
    "i'm going to hurt",
    "i am going to hurt",
    "gonna hurt",
    "going to destroy you",
    "i will destroy you",
    "i'll destroy you",
    "going to ruin your life",
    "i will ruin your life",
    "you will pay for this",
    "you're going to regret",
    "you will regret this",
    "i know where you live",
    "watch your back",
    "make you pay",
    "you'll be sorry",
    "you will be sorry",
];

/// Result of the safety check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SafetyCheck {
    /// Input is safe to rephrase.
    Safe,
    /// Input contains a threat or harassment pattern; rephrasing is declined.
    Declined {
        /// The matched pattern that triggered the decline.
        matched_pattern: String,
    },
}

impl SafetyCheck {
    /// Returns `true` if it is safe to proceed with rephrasing.
    #[must_use]
    pub const fn is_safe(&self) -> bool {
        matches!(self, Self::Safe)
    }
}

/// Check `message` for threat-of-harm or harassment content.
///
/// Returns [`SafetyCheck::Declined`] with the first matched pattern if a
/// threat is detected, or [`SafetyCheck::Safe`] otherwise.
#[must_use]
pub fn check_safety(message: &str) -> SafetyCheck {
    let lower = message.to_lowercase();
    for pattern in THREAT_PATTERNS {
        if lower.contains(*pattern) {
            return SafetyCheck::Declined { matched_pattern: (*pattern).to_string() };
        }
    }
    SafetyCheck::Safe
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn threat_is_declined() {
        // AC5: a fixture input containing a threat is declined by the documented rule.
        let result = check_safety("You idiot, I will kill you if you do that again.");
        assert_eq!(
            result,
            SafetyCheck::Declined { matched_pattern: "i will kill".to_string() }
        );
    }

    #[test]
    fn heated_but_not_threat_is_safe() {
        let result = check_safety("You never listen and this is completely ridiculous.");
        assert!(result.is_safe(), "heated message without threat should be safe");
    }

    #[test]
    fn case_insensitive_threat() {
        let result = check_safety("I WILL KILL the project deadline if you keep this up.");
        // "i will kill" is in the phrase — it matches; this is a conservative
        // rule-based check that may over-block in edge cases.
        assert!(!result.is_safe());
    }

    #[test]
    fn watch_your_back_declined() {
        let result = check_safety("Watch your back next time you speak.");
        assert!(!result.is_safe());
    }
}
