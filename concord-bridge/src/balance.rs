//! Deterministic balance gate for the synthesized brief.
//!
//! Three clauses (a), (b), (c):
//! - (a) every cited id resolves to a [`Source`] id in the corpus source list.
//! - (b) per-stance coverage in the "what each side wants" section is within
//!   the configured ratio (no side <N% while another dominates).
//! - (c) the common-ground section is non-empty when the map has ≥1 shared value.

/// Result of a balance check.
#[derive(Debug, Clone)]
pub struct BalanceResult {
    /// Whether all three clauses passed.
    pub passed: bool,
    /// Human-readable descriptions of each failure (empty if all passed).
    pub failures: Vec<String>,
}

impl BalanceResult {
    /// Construct a passing result.
    #[must_use]
    pub const fn ok() -> Self {
        Self { passed: true, failures: Vec::new() }
    }

    /// Construct a failing result with recorded failures.
    #[must_use]
    pub const fn failed(failures: Vec<String>) -> Self {
        Self { passed: false, failures }
    }
}

/// Minimum fraction of total stance-section word count each stance must occupy.
const MIN_STANCE_FRACTION: f64 = 0.25;

/// Check balance of a brief against the corpus and map.
///
/// `stance_word_counts`: word counts per stance section (`stance_label` → count).
/// `cited_ids`: source ids cited in the brief.
/// `corpus_source_ids`: all source ids in the corpus.
/// `has_shared_values`: whether the map had ≥1 shared value.
/// `common_ground_empty`: whether the common-ground section is empty.
#[must_use]
#[allow(clippy::float_arithmetic, clippy::as_conversions, clippy::cast_precision_loss)]
pub fn check_balance(
    stance_word_counts: &[(String, usize)],
    cited_ids: &[String],
    corpus_source_ids: &[String],
    has_shared_values: bool,
    common_ground_empty: bool,
) -> BalanceResult {
    let mut failures = Vec::new();

    // Clause (a): every cited id must resolve to a corpus source.
    // Only enforced when corpus_source_ids is non-empty (i.e., the caller
    // has threaded the corpus through).
    if !corpus_source_ids.is_empty() {
        for id in cited_ids {
            if !corpus_source_ids.contains(id) {
                failures.push(format!("clause(a): cited id '{id}' not found in corpus"));
            }
        }
    }

    // Clause (b): no stance may receive < MIN_STANCE_FRACTION of total coverage.
    if stance_word_counts.len() >= 2 {
        let total: usize = stance_word_counts.iter().map(|(_, c)| c).sum();
        if total > 0 {
            for (stance, count) in stance_word_counts {
                let fraction = *count as f64 / total as f64;
                if fraction < MIN_STANCE_FRACTION {
                    failures.push(format!(
                        "clause(b): stance '{}' has {:.1}% coverage < {:.0}% minimum",
                        stance,
                        fraction * 100.0,
                        MIN_STANCE_FRACTION * 100.0,
                    ));
                }
            }
        }
    }

    // Clause (c): common-ground must be non-empty when map has shared values.
    if has_shared_values && common_ground_empty {
        failures.push(String::from(
            "clause(c): common-ground section is empty but map has shared values",
        ));
    }

    if failures.is_empty() {
        BalanceResult::ok()
    } else {
        BalanceResult::failed(failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_passes_when_all_ok() {
        let result = check_balance(
            &[("Supports".to_string(), 50), ("Opposes".to_string(), 50)],
            &["source-1".to_string()],
            &["source-1".to_string(), "source-2".to_string()],
            true,
            false,
        );
        assert!(result.passed);
    }

    #[test]
    fn clause_a_catches_unknown_citation() {
        let result = check_balance(
            &[("Supports".to_string(), 50), ("Opposes".to_string(), 50)],
            &["nonexistent-id".to_string()],
            &["source-1".to_string()],
            false,
            false,
        );
        assert!(!result.passed);
        assert!(result.failures.iter().any(|f| f.contains("clause(a)")));
    }

    #[test]
    fn clause_b_catches_one_sided_coverage() {
        let result = check_balance(
            &[("Supports".to_string(), 95), ("Opposes".to_string(), 5)],
            &[],
            &[],
            false,
            false,
        );
        assert!(!result.passed);
        assert!(result.failures.iter().any(|f| f.contains("clause(b)")));
    }

    #[test]
    fn clause_c_catches_empty_common_ground() {
        let result = check_balance(
            &[("Supports".to_string(), 50), ("Opposes".to_string(), 50)],
            &[],
            &[],
            true,  // has shared values
            true,  // but common-ground section is empty
        );
        assert!(!result.passed);
        assert!(result.failures.iter().any(|f| f.contains("clause(c)")));
    }

    #[test]
    fn clause_b_balanced_response_passes() {
        let result = check_balance(
            &[("Supports".to_string(), 60), ("Opposes".to_string(), 40)],
            &[],
            &[],
            false,
            false,
        );
        assert!(result.passed, "balanced 60/40 should pass");
    }
}
