//! Deterministic, rule-based stance tagger.
//!
//! Assigns a [`Stance`] to a source based on lexical and structural cues in
//! the document text. No LLM is invoked — this is a pure string-analysis pass.
//!
//! # Accuracy target
//! ≥80% on a hand-labelled fixture set of ≥15 sources spanning ≥2 stances
//! (AC4). The rules are designed to be conservative: ambiguous text defaults
//! to [`Stance::Background`] rather than a wrong directional label.

use serde::{Deserialize, Serialize};

/// The stance a source argues with respect to the claim under analysis.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Stance {
    /// Source argues in favour of the claim.
    Supports,
    /// Source argues against the claim.
    Opposes,
    /// Source presents both sides or takes no clear position.
    Mixed,
    /// Background / contextual; does not directly argue the claim.
    Background,
}

impl std::fmt::Display for Stance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Supports => "Supports",
            Self::Opposes => "Opposes",
            Self::Mixed => "Mixed",
            Self::Background => "Background",
        };
        f.write_str(s)
    }
}

/// Signals that lean toward "Supports" (claim is correct / good).
const SUPPORTS_SIGNALS: &[&str] = &[
    "confirms",
    "supports",
    "evidence shows",
    "study finds",
    "research shows",
    "proven",
    "demonstrates",
    "validated",
    "effective",
    "beneficial",
    "advantage",
    "in favor of",
    "in favour of",
    "endorses",
    "recommends",
    "success",
    "positive results",
    "overwhelming evidence",
];

/// Signals that lean toward "Opposes" (claim is wrong / bad).
const OPPOSES_SIGNALS: &[&str] = &[
    "critics say",
    "critics argue",
    "opponents argue",
    "debunked",
    "disproven",
    "myth",
    "false",
    "misleading",
    "no evidence",
    "lack of evidence",
    "disputed",
    "challenged",
    "refutes",
    "contradicts",
    "harmful",
    "dangerous",
    "risk of",
    "risks include",
    "concerns about",
    "questions the",
    "doubt",
    "skeptic",
    "sceptic",
    "against",
    "opposes",
];

/// Signals that indicate a balanced / both-sides treatment.
const MIXED_SIGNALS: &[&str] = &[
    "on the one hand",
    "on the other hand",
    "pros and cons",
    "both sides",
    "mixed evidence",
    "controversial",
    "debate",
    "divided",
    "some argue",
    "others argue",
    "it depends",
    "nuanced",
    "complex issue",
    "competing",
    "balance of evidence",
];

/// Tag the stance of a document given its title and full body text.
///
/// Returns a `(Stance, label)` pair where `label` is a brief rationale string
/// (useful for debugging / display, not stored in the final schema).
#[must_use]
pub fn tag_stance(title: &str, full_text: &str) -> (Stance, String) {
    let haystack = format!("{} {}", title, full_text).to_lowercase();

    let supports_score = count_signals(&haystack, SUPPORTS_SIGNALS);
    let opposes_score = count_signals(&haystack, OPPOSES_SIGNALS);
    let mixed_score = count_signals(&haystack, MIXED_SIGNALS);

    // Mixed beats directional when it fires and the directional advantage is small.
    if mixed_score >= 2 && (supports_score as i32 - opposes_score as i32).unsigned_abs() < 3 {
        return (Stance::Mixed, format!("mixed={mixed_score} sup={supports_score} opp={opposes_score}"));
    }
    if mixed_score >= 3 {
        return (Stance::Mixed, format!("mixed={mixed_score} dominant"));
    }

    let total = supports_score + opposes_score;
    if total == 0 {
        return (Stance::Background, "no directional signals".to_string());
    }

    // Require a clear majority (>60%) to assign a directional label.
    let support_ratio = supports_score as f64 / total as f64;
    if support_ratio > 0.60 {
        (Stance::Supports, format!("sup={supports_score} opp={opposes_score}"))
    } else if support_ratio < 0.40 {
        (Stance::Opposes, format!("opp={opposes_score} sup={supports_score}"))
    } else {
        (Stance::Mixed, format!("balanced sup={supports_score} opp={opposes_score}"))
    }
}

fn count_signals(haystack: &str, signals: &[&str]) -> usize {
    signals.iter().filter(|&&sig| haystack.contains(sig)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supports_signal_wins() {
        let (stance, _) = tag_stance(
            "Study confirms vaccine is effective",
            "The research shows overwhelming evidence of positive results and demonstrates safety.",
        );
        assert_eq!(stance, Stance::Supports);
    }

    #[test]
    fn opposes_signal_wins() {
        let (stance, _) = tag_stance(
            "Critics say the policy is harmful",
            "Opponents argue there is no evidence this approach works. Risks include severe side effects.",
        );
        assert_eq!(stance, Stance::Opposes);
    }

    #[test]
    fn mixed_detected() {
        let (stance, _) = tag_stance(
            "The controversial debate continues",
            "On the one hand supporters endorse it; on the other hand critics argue it is harmful. Both sides present competing evidence.",
        );
        assert_eq!(stance, Stance::Mixed);
    }

    #[test]
    fn no_signals_yields_background() {
        let (stance, _) = tag_stance(
            "History of ancient Rome",
            "In 753 BCE Romulus founded the city.",
        );
        assert_eq!(stance, Stance::Background);
    }
}
