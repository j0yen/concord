//! Core data model for the disagreement map.

use serde::{Deserialize, Serialize};

/// Whether a crux is resolvable by evidence or is a fundamental value difference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CruxKind {
    /// A factual question that could in principle be resolved by evidence.
    Empirical,
    /// A difference in what to weight or prioritize — values, not facts.
    Value,
}

/// A genuine divergence between the two sides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Crux {
    /// A concise statement of the divergence.
    pub statement: String,
    /// Whether the crux is empirical or a value difference.
    pub kind: CruxKind,
    /// What evidence or argument would change the first side's mind.
    pub what_would_change_side_a: String,
    /// What evidence or argument would change the second side's mind.
    pub what_would_change_side_b: String,
}

/// A value that both sides already hold in common.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharedValue {
    /// A concise statement of the shared value.
    pub statement: String,
}

/// An apparent conflict that dissolves on clarification.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Misunderstanding {
    /// A description of the apparent conflict.
    pub apparent_conflict: String,
    /// The clarifying distinction that dissolves it.
    pub clarifying_distinction: String,
}

/// A structured map of where the disagreement actually is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisagreementMap {
    /// The contested claim this map was built for.
    pub claim: String,
    /// Values both sides hold in common.
    pub shared_values: Vec<SharedValue>,
    /// Genuine divergences.
    pub cruxes: Vec<Crux>,
    /// Terminological / framing misunderstandings.
    pub misunderstandings: Vec<Misunderstanding>,
}

impl DisagreementMap {
    /// Create an empty map for the given claim.
    #[must_use]
    pub fn new(claim: impl Into<String>) -> Self {
        Self {
            claim: claim.into(),
            shared_values: Vec::new(),
            cruxes: Vec::new(),
            misunderstandings: Vec::new(),
        }
    }

    /// Total number of entries across all buckets.
    #[must_use]
    pub fn total_entries(&self) -> usize {
        self.shared_values.len() + self.cruxes.len() + self.misunderstandings.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_roundtrip() {
        let map = DisagreementMap {
            claim: "Is X good?".to_string(),
            shared_values: vec![SharedValue {
                statement: "Both sides value safety.".to_string(),
            }],
            cruxes: vec![Crux {
                statement: "Does X cause harm?".to_string(),
                kind: CruxKind::Empirical,
                what_would_change_side_a: "A long-term study.".to_string(),
                what_would_change_side_b: "Replication failures.".to_string(),
            }],
            misunderstandings: vec![Misunderstanding {
                apparent_conflict: "Side A says X is natural; side B says it's artificial."
                    .to_string(),
                clarifying_distinction: "They mean different things by 'natural'.".to_string(),
            }],
        };
        let json = serde_json::to_string(&map).unwrap();
        let decoded: DisagreementMap = serde_json::from_str(&json).unwrap();
        assert_eq!(map, decoded);
    }

    #[test]
    fn crux_requires_change_minds_field() {
        // Deterministic: a Crux struct without both change-minds fields
        // cannot be constructed (both are required by the type).
        let crux = Crux {
            statement: "Divergence".to_string(),
            kind: CruxKind::Value,
            what_would_change_side_a: "If costs are reweighted.".to_string(),
            what_would_change_side_b: "If benefits are reweighted.".to_string(),
        };
        assert!(!crux.what_would_change_side_a.is_empty());
        assert!(!crux.what_would_change_side_b.is_empty());
    }

    #[test]
    fn misunderstanding_requires_clarifying_distinction() {
        let m = Misunderstanding {
            apparent_conflict: "A conflict".to_string(),
            clarifying_distinction: "The distinction".to_string(),
        };
        assert!(!m.clarifying_distinction.is_empty());
    }

    #[test]
    fn total_entries_counts_all_buckets() {
        let mut map = DisagreementMap::new("claim");
        map.shared_values.push(SharedValue { statement: "s".to_string() });
        map.cruxes.push(Crux {
            statement: "c".to_string(),
            kind: CruxKind::Empirical,
            what_would_change_side_a: "a".to_string(),
            what_would_change_side_b: "b".to_string(),
        });
        map.misunderstandings.push(Misunderstanding {
            apparent_conflict: "x".to_string(),
            clarifying_distinction: "y".to_string(),
        });
        assert_eq!(map.total_entries(), 3);
    }
}
