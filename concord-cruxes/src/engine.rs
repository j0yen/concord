//! Crux analysis engine: orchestrates prompt, model, and validation.

use anyhow::{Context, Result};
use concord_steelman::{ConcordModel, Steelman};

use crate::{
    classifier::parse_and_validate,
    map::DisagreementMap,
    prompt::build_crux_prompt,
};

/// The result of analyzing a pair of steelmans.
#[derive(Debug, Clone)]
pub struct CruxAnalysis {
    /// The structured disagreement map.
    pub map: DisagreementMap,
    /// Non-fatal warnings emitted during post-validation (e.g. skipped entries).
    pub warnings: Vec<String>,
}

/// Analyze two steelmans against a contested `claim` using `model`.
///
/// Produces a [`DisagreementMap`] separating shared values, cruxes, and
/// misunderstandings. The model is called once; parsing failures are returned
/// as errors.
///
/// # Errors
/// Returns an error if the model fails or the response cannot be parsed.
pub fn analyze_steelmans(
    claim: &str,
    side_a: &Steelman,
    side_b: &Steelman,
    model: &dyn ConcordModel,
) -> Result<CruxAnalysis> {
    let prompt = build_crux_prompt(claim, side_a, side_b);
    let response = model.complete(&prompt).context("model completion for crux analysis")?;
    let (map, warnings) =
        parse_and_validate(claim, &response).context("parsing crux classification")?;
    Ok(CruxAnalysis { map, warnings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use concord_steelman::{MockModel, Premise};

    fn make_steelman(stance: &str, claim_text: &str) -> Steelman {
        Steelman {
            stance: stance.to_string(),
            claim: claim_text.to_string(),
            premises: vec![Premise {
                text: "A supporting premise.".to_string(),
                cited_source_ids: vec![0],
            }],
            conclusion: "A conclusion.".to_string(),
            cited_source_ids: vec![0],
            flagged: false,
            flag_reason: String::new(),
        }
    }

    fn good_response() -> String {
        serde_json::json!({
            "shared_values": [{"statement": "Both sides value safety."}],
            "cruxes": [{
                "statement": "Does X cause harm?",
                "kind": "empirical",
                "what_would_change_side_a": "A long-term study.",
                "what_would_change_side_b": "Replication failures."
            }],
            "misunderstandings": [{
                "apparent_conflict": "Different uses of 'natural'.",
                "clarifying_distinction": "Side A means organic; side B means unmodified."
            }]
        })
        .to_string()
    }

    #[test]
    fn emits_valid_disagreement_map() {
        let a = make_steelman("Supports", "X is beneficial.");
        let b = make_steelman("Opposes", "X is harmful.");
        let model = MockModel::with_default(good_response());
        let analysis = analyze_steelmans("Is X good?", &a, &b, &model).unwrap();

        assert_eq!(analysis.map.shared_values.len(), 1);
        assert_eq!(analysis.map.cruxes.len(), 1);
        assert_eq!(analysis.map.misunderstandings.len(), 1);
        assert!(analysis.warnings.is_empty());
    }

    #[test]
    fn every_entry_in_exactly_one_bucket() {
        let a = make_steelman("Supports", "X is beneficial.");
        let b = make_steelman("Opposes", "X is harmful.");
        let model = MockModel::with_default(good_response());
        let analysis = analyze_steelmans("Is X good?", &a, &b, &model).unwrap();
        // 1 shared + 1 crux + 1 misunderstanding = 3 total.
        assert_eq!(analysis.map.total_entries(), 3);
    }

    #[test]
    fn crux_has_non_empty_change_minds() {
        let a = make_steelman("Supports", "X is good.");
        let b = make_steelman("Opposes", "X is bad.");
        let model = MockModel::with_default(good_response());
        let analysis = analyze_steelmans("X?", &a, &b, &model).unwrap();
        for crux in &analysis.map.cruxes {
            assert!(
                !crux.what_would_change_side_a.is_empty(),
                "what_would_change_side_a is empty"
            );
            assert!(
                !crux.what_would_change_side_b.is_empty(),
                "what_would_change_side_b is empty"
            );
        }
    }

    #[test]
    fn misunderstanding_has_clarifying_distinction() {
        let a = make_steelman("Supports", "X is good.");
        let b = make_steelman("Opposes", "X is bad.");
        let model = MockModel::with_default(good_response());
        let analysis = analyze_steelmans("X?", &a, &b, &model).unwrap();
        for m in &analysis.map.misunderstandings {
            assert!(!m.clarifying_distinction.is_empty());
        }
    }

    #[test]
    fn no_model_backend_required_with_mock() {
        // MockModel increments call_count; verifies no real network is hit.
        let a = make_steelman("Supports", "X is good.");
        let b = make_steelman("Opposes", "X is bad.");
        let model = MockModel::with_default(good_response());
        let analysis = analyze_steelmans("X?", &a, &b, &model).unwrap();
        // call_count==1 means exactly one call, no retry or network.
        assert_eq!(model.call_count.get(), 1);
        assert_eq!(analysis.map.total_entries(), 3);
    }
}
