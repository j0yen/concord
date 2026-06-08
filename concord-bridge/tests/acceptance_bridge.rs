//! AC1-6 integration-level acceptance tests for concord-bridge.

use concord_bridge::{synthesize_brief, MockBridgeModel};
use concord_cruxes::{Crux, CruxKind, DisagreementMap, Misunderstanding, SharedValue};

fn make_map() -> DisagreementMap {
    DisagreementMap {
        claim: "Does coffee reduce diabetes risk?".to_string(),
        shared_values: vec![SharedValue {
            statement: "Human health matters".to_string(),
        }],
        cruxes: vec![Crux {
            statement: "Does moderate coffee consumption lower HbA1c?".to_string(),
            kind: CruxKind::Empirical,
            what_would_change_side_a: "A meta-analysis with HbA1c as primary endpoint"
                .to_string(),
            what_would_change_side_b: "Replication failures in large cohort studies".to_string(),
        }],
        misunderstandings: vec![Misunderstanding {
            apparent_conflict: "risk reduction vs elimination".to_string(),
            clarifying_distinction: "The claim is about relative risk, not absolute elimination"
                .to_string(),
        }],
    }
}

fn balanced_model() -> MockBridgeModel {
    MockBridgeModel::new(
        "WHAT_EACH_SIDE_WANTS:\n\
         Supports: Coffee consumption has been associated with reduced diabetes risk in several large cohort studies [SOURCE:source-1]\n\
         Opposes: The evidence is largely observational and potentially confounded by lifestyle factors [SOURCE:source-2]\n\
         WHERE_DISAGREEMENT_IS:\nThe central empirical question is whether the association is causal.\n\
         COMMON_GROUND:\nBoth sides value rigorous peer-reviewed evidence and patient welfare.\n\
         WHAT_WOULD_CHANGE_MINDS:\nA large RCT with HbA1c as the primary endpoint would resolve the empirical crux.",
    )
}

// AC1: help command tested via CLI; build tested by compilation.
// AC2: brief contains all four named sections and a references list.
#[test]
fn brief_has_all_four_sections_and_references() {
    let map = make_map();
    let model = balanced_model();
    let output = synthesize_brief(&map, &model).expect("synthesis ok");
    let md = &output.markdown;
    assert!(md.contains("## What Each Side Wants"), "WESAS section missing");
    assert!(md.contains("## Where the Disagreement Actually Is"), "disagreement section missing");
    assert!(md.contains("## Common Ground"), "common ground missing");
    assert!(md.contains("## What Would Change Minds"), "change minds missing");
    assert!(md.contains("## References"), "references missing");
    assert!(md.contains("source-"), "references should list cited ids");
}

// AC3: citation integrity — non-existent id is caught by balance gate clause (a).
#[test]
fn citation_integrity_bad_id_caught() {
    use concord_bridge::check_balance;
    let result = check_balance(
        &[("Supports".to_string(), 50), ("Opposes".to_string(), 50)],
        &["NONEXISTENT".to_string()],
        &["source-1".to_string()],
        false,
        false,
    );
    assert!(!result.passed, "unknown citation should fail clause (a)");
    assert!(result.failures.iter().any(|f| f.contains("clause(a)")));
}

// AC4a: one-sided model trips coverage clause (b) → regenerate-then-report.
#[test]
fn one_sided_model_trips_coverage_clause() {
    let map = make_map();
    let model = MockBridgeModel::new(
        "WHAT_EACH_SIDE_WANTS:\n\
         Supports: An extremely detailed multi-sentence paragraph comprehensively covering every aspect of the pro-coffee argument with many many words and citations [SOURCE:s1]\n\
         Opposes: Brief.\n\
         WHERE_DISAGREEMENT_IS:\nSomething.\nCOMMON_GROUND:\nNone.\nWHAT_WOULD_CHANGE_MINDS:\nSomething.",
    );
    let output = synthesize_brief(&map, &model).expect("ok");
    assert!(!output.balance_ok, "one-sided brief should fail balance gate");
    assert!(
        output.balance_failures.iter().any(|f| f.contains("clause(b)")),
        "expected clause(b) failure, got: {:?}",
        output.balance_failures
    );
}

// AC4b: balanced response passes and emits.
#[test]
fn balanced_response_passes_balance_gate() {
    let map = make_map();
    let model = balanced_model();
    let output = synthesize_brief(&map, &model).expect("ok");
    assert!(
        output.balance_ok,
        "balanced brief should pass; failures: {:?}",
        output.balance_failures
    );
}

// AC6: full suite passes with no ollama / no network.
#[test]
fn no_network_required() {
    let map = make_map();
    let model = MockBridgeModel::new(
        "WHAT_EACH_SIDE_WANTS:\n\
         Supports: Coffee may protect via chlorogenic acid pathways [SOURCE:s1]\n\
         Opposes: The effect size is clinically marginal [SOURCE:s2]\n\
         WHERE_DISAGREEMENT_IS:\nCausation.\nCOMMON_GROUND:\nBoth value evidence.\nWHAT_WOULD_CHANGE_MINDS:\nRCT data.",
    );
    let out = synthesize_brief(&map, &model).expect("no-network synthesis ok");
    assert!(!out.markdown.is_empty());
}
