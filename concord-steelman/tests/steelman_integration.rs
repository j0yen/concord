//! Integration tests for concord-steelman (AC1–AC5).
//!
//! All tests are offline and deterministic; no ollama or network is required.

use concord_corpus::{Corpus, ProvenanceRecord, Source, Stance};
use concord_steelman::{steelman_corpus, MockModel};
use chrono::Utc;

// ── Fixtures ──────────────────────────────────────────────────────────────────

fn make_source(title: &str, stance: Stance, stance_label: &str, idx: usize) -> Source {
    Source {
        title: title.to_string(),
        url: format!("http://example.com/source-{idx}"),
        publisher: "Example Publisher".to_string(),
        retrieved_at: Utc::now(),
        excerpt: format!("Excerpt for source {idx}: {title}."),
        full_text: format!("Full text for source {idx}. {title}."),
        stance,
        stance_label: stance_label.to_string(),
        credibility: 0.75,
        provenance: ProvenanceRecord {
            gatherer: "fixture".to_string(),
            query: "test query".to_string(),
            rank: idx,
        },
    }
}

fn make_corpus() -> Corpus {
    Corpus::from_sources(
        "Should governments adopt universal basic income?",
        vec![
            make_source("Studies show UBI reduces poverty", Stance::Supports, "pro-UBI", 0),
            make_source("UBI boosts entrepreneurship and freedom", Stance::Supports, "pro-UBI", 1),
            make_source("UBI is unaffordable and disincentivizes work", Stance::Opposes, "anti-UBI", 2),
        ],
    )
}

/// A well-formed scripted response for AC2.
fn good_response_for_stance(stance_label: &str, source_id: usize) -> String {
    format!(
        "CLAIM: {stance_label} proponents argue that the evidence strongly supports their position.\n\
         PREMISE_1: Multiple peer-reviewed analyses show positive outcomes. [SOURCE_ID={source_id}]\n\
         PREMISE_2: Historical data supports long-term stability. [SOURCE_ID={source_id}]\n\
         CONCLUSION: Therefore, the {stance_label} position deserves serious consideration.\n"
    )
}

// ── AC2: per-stance steelmans with valid citations ────────────────────────────

#[test]
fn ac2_one_steelman_per_stance() {
    let corpus = make_corpus();
    // corpus has 2 stance summaries: Supports and Opposes.
    assert_eq!(corpus.stances.len(), 2, "fixture must have 2 stances");

    // Map each stance to a good response.
    let mut responses = std::collections::HashMap::new();
    for summary in &corpus.stances {
        let label = match summary.stance {
            Stance::Supports => "pro-UBI",
            Stance::Opposes => "anti-UBI",
            _ => "other",
        };
        let source_id = match summary.stance {
            Stance::Supports => 0_usize,
            Stance::Opposes => 2_usize,
            _ => 0_usize,
        };
        let prompt = concord_steelman::build_prompt(&corpus, &summary.stance, label);
        let fp = MockModel::fingerprint(&prompt);
        responses.insert(fp, good_response_for_stance(label, source_id));
    }

    let model = MockModel::new(responses);
    let result = steelman_corpus(&corpus, &model).expect("steelman_corpus should not fail");

    assert_eq!(result.steelmans.len(), 2, "should have one steelman per stance");

    for steel in &result.steelmans {
        assert!(!steel.claim.is_empty(), "claim must not be empty");
        assert!(!steel.premises.is_empty(), "must have at least one premise");
        assert!(!steel.conclusion.is_empty(), "conclusion must not be empty");
        assert!(!steel.flagged, "well-formed response should not be flagged: {:?}", steel.flag_reason);
    }
}

// ── AC3: citation integrity ────────────────────────────────────────────────────

#[test]
fn ac3_invalid_citation_is_flagged() {
    let corpus = make_corpus();
    // Response cites SOURCE_ID=99 which does not exist (corpus has 3 sources, ids 0-2).
    let bad_response =
        "CLAIM: This position is well-supported.\n\
         PREMISE_1: Evidence A. [SOURCE_ID=99]\n\
         CONCLUSION: Therefore the position holds.\n";

    let model = MockModel::with_default(bad_response.to_string());
    let result = steelman_corpus(&corpus, &model).expect("should not fail");

    // At least one steelman should be flagged due to invalid citation.
    let flagged_count = result.steelmans.iter().filter(|s| s.flagged).count();
    assert!(flagged_count > 0, "steelman with invalid citation must be flagged");

    // The flagged steelman's premises should have empty cited_source_ids
    // (the invalid id was stripped).
    for steel in result.steelmans.iter().filter(|s| s.flagged) {
        for premise in &steel.premises {
            assert!(
                premise.cited_source_ids.is_empty(),
                "invalid citation ids should be stripped from premise"
            );
        }
    }
}

#[test]
fn ac3_valid_citation_passes() {
    let corpus = make_corpus();
    // SOURCE_ID=0 is valid (first source is Supports).
    let good_response =
        "CLAIM: UBI has proven benefits.\n\
         PREMISE_1: Pilot programs show reduced poverty. [SOURCE_ID=0]\n\
         PREMISE_2: Economic analysis supports viability. [SOURCE_ID=1]\n\
         CONCLUSION: Therefore UBI should be piloted widely.\n";

    let model = MockModel::with_default(good_response.to_string());
    let result = steelman_corpus(&corpus, &model).expect("should not fail");

    // Check that at least one steelman has valid citations and is not flagged for citations.
    let citation_ok: Vec<_> = result
        .steelmans
        .iter()
        .filter(|s| !s.flag_reason.contains("invalid source ids"))
        .collect();
    assert!(!citation_ok.is_empty(), "steelmen with valid citations should exist");
}

// ── AC4: anti-caricature ─────────────────────────────────────────────────────

#[test]
fn ac4_contempt_term_triggers_regen_then_flag() {
    let corpus = make_corpus();
    // Both responses contain "obviously" — should flag after retry.
    let bad_response =
        "CLAIM: Obviously the position is correct.\n\
         PREMISE_1: Evidence is overwhelming. [SOURCE_ID=0]\n\
         CONCLUSION: Obviously this concludes everything.\n";

    let model = MockModel::with_default(bad_response.to_string());
    let result = steelman_corpus(&corpus, &model).expect("should not fail");

    let flagged: Vec<_> = result.steelmans.iter().filter(|s| s.flagged).collect();
    assert!(!flagged.is_empty(), "contempt term should cause flagging after regen");
    for steel in &flagged {
        assert!(
            steel.flag_reason.contains("anti-caricature"),
            "flag reason should mention anti-caricature: {:?}",
            steel.flag_reason
        );
    }
}

#[test]
fn ac4_clean_response_passes_caricature_check() {
    let corpus = make_corpus();
    let clean_response =
        "CLAIM: UBI has documented positive effects on wellbeing.\n\
         PREMISE_1: Research shows reduced financial stress. [SOURCE_ID=0]\n\
         CONCLUSION: Thoughtful implementation could yield lasting benefits.\n";

    let model = MockModel::with_default(clean_response.to_string());
    let result = steelman_corpus(&corpus, &model).expect("should not fail");

    for steel in &result.steelmans {
        assert!(
            !steel.flag_reason.contains("anti-caricature"),
            "clean response should not trigger anti-caricature flag"
        );
    }
}

// ── AC5: no real backend under mock ──────────────────────────────────────────

#[test]
fn ac5_no_real_backend_called() {
    let corpus = make_corpus();
    let model = MockModel::with_default(
        "CLAIM: Test.\nPREMISE_1: Some premise. [SOURCE_ID=0]\nCONCLUSION: End.\n".to_string(),
    );

    let _ = steelman_corpus(&corpus, &model).expect("should not fail");

    // call_count > 0 means the model was called; but since it's MockModel,
    // no real network or ollama was touched. The test passing without network
    // is the guarantee.
    let calls = model.call_count.get();
    assert!(calls > 0, "model should have been called at least once");
    // If this test passes in a sandboxed environment (cloud box, no ollama),
    // it satisfies AC5.
}
