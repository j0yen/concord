//! Anti-caricature and citation-integrity guards.
//!
//! The steelman engine applies two deterministic post-generation checks:
//!
//! 1. **Citation integrity** — every premise must cite a source id that exists
//!    in the corpus (0-indexed). Premises citing non-existent ids are
//!    removed; if all premises fail, the steelman is flagged.
//!
//! 2. **Anti-caricature** — the steelman text must not contain contempt-lexicon
//!    terms or cheap dismissal phrases. If it does, the caller can regenerate
//!    once and re-check; if it fails again, the steelman is flagged.

/// Terms that indicate a caricature or contemptuous steelman rather than a
/// charitable one.
const CONTEMPT_LEXICON: &[&str] = &[
    "obviously",
    "any reasonable person",
    "clearly wrong",
    "ridiculous",
    "absurd",
    "stupid",
    "idiotic",
    "ignorant",
    "deluded",
    "lunatic",
    "fringe",
    "conspiracy",
    "denier",
    "flat-earther",
    "anti-science",
];

/// Check whether `text` passes the anti-caricature guard.
///
/// Returns `Ok(())` if clean, or `Err(reason)` identifying the first
/// offending term.
///
/// # Errors
/// Returns a description of the first contempt term found.
pub fn check_anti_caricature(text: &str) -> Result<(), String> {
    let lower = text.to_lowercase();
    for term in CONTEMPT_LEXICON {
        if lower.contains(term) {
            return Err(format!("contempt term found: {term:?}"));
        }
    }
    Ok(())
}

/// Verify that all source ids cited in `premise_ids` are valid given
/// `corpus_source_count` (0-indexed).
///
/// Returns the subset of ids that are valid.
#[must_use]
pub fn filter_valid_citations(premise_ids: &[usize], corpus_source_count: usize) -> Vec<usize> {
    premise_ids
        .iter()
        .filter(|&&id| id < corpus_source_count)
        .copied()
        .collect()
}

/// Check whether all cited ids are valid.
///
/// # Errors
/// Returns the first invalid id found.
pub fn check_citations(cited_ids: &[usize], corpus_source_count: usize) -> Result<(), usize> {
    for &id in cited_ids {
        if id >= corpus_source_count {
            return Err(id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_text_passes() {
        assert!(check_anti_caricature("This is a reasonable argument.").is_ok());
    }

    #[test]
    fn contempt_term_caught() {
        let result = check_anti_caricature("Obviously, anyone can see this.");
        assert!(result.is_err());
        let msg = result.unwrap_err();
        assert!(msg.contains("obviously"));
    }

    #[test]
    fn any_reasonable_person_caught() {
        let result = check_anti_caricature("Any reasonable person would agree.");
        assert!(result.is_err());
    }

    #[test]
    fn valid_citations_pass() {
        assert!(check_citations(&[0, 1, 2], 3).is_ok());
    }

    #[test]
    fn invalid_citation_caught() {
        let result = check_citations(&[0, 5], 3);
        assert_eq!(result, Err(5));
    }

    #[test]
    fn filter_keeps_valid_only() {
        let valid = filter_valid_citations(&[0, 5, 2, 10], 3);
        assert_eq!(valid, vec![0, 2]);
    }
}
