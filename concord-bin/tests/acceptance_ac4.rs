//! AC4: Stance tagging assigns the correct Stance to ≥80% of a hand-labelled,
//! independently authored fixture set (≥15 sources spanning ≥2 stances).
//! Labels live in checked-in expected.json written before the tagger.

use concord_corpus::{tag_stance, Stance};
use serde::Deserialize;

fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests/fixtures/stance_labelled")
}

#[derive(Debug, Deserialize)]
struct ExpectedLabel {
    url: String,
    expected_stance: String,
    #[allow(dead_code)]
    notes: String,
}

#[derive(Debug, Deserialize)]
struct FixtureSource {
    title: String,
    url: String,
    body: String,
}

fn parse_stance(s: &str) -> Stance {
    match s {
        "Supports" => Stance::Supports,
        "Opposes" => Stance::Opposes,
        "Mixed" => Stance::Mixed,
        _ => Stance::Background,
    }
}

#[test]
fn stance_tagger_achieves_eighty_percent_accuracy() {
    let dir = fixtures_dir();

    let sources_raw = std::fs::read_to_string(dir.join("sources.json"))
        .expect("reading sources.json");
    let sources: Vec<FixtureSource> =
        serde_json::from_str(&sources_raw).expect("parse sources.json");

    let expected_raw = std::fs::read_to_string(dir.join("expected.json"))
        .expect("reading expected.json");
    let expected: Vec<ExpectedLabel> =
        serde_json::from_str(&expected_raw).expect("parse expected.json");

    assert!(
        sources.len() >= 15,
        "fixture set must have ≥15 sources (AC4), got {}",
        sources.len()
    );

    // Build a map from URL to expected stance.
    let expected_map: std::collections::HashMap<&str, Stance> = expected
        .iter()
        .map(|e| (e.url.as_str(), parse_stance(&e.expected_stance)))
        .collect();

    let mut correct = 0usize;
    let mut total = 0usize;
    let mut mismatches = Vec::new();

    for src in &sources {
        let Some(expected_stance) = expected_map.get(src.url.as_str()) else {
            panic!("No expected label for URL: {}", src.url);
        };
        let (actual_stance, label) = tag_stance(&src.title, &src.body);
        total += 1;
        if &actual_stance == expected_stance {
            correct += 1;
        } else {
            mismatches.push(format!(
                "  URL: {}\n  Expected: {expected_stance}\n  Got: {actual_stance}\n  Label: {label}",
                src.url
            ));
        }
    }

    let accuracy = correct as f64 / total as f64;
    assert!(
        accuracy >= 0.80,
        "Stance tagger accuracy {:.1}% ({}/{}) < 80% target.\nMismatches:\n{}",
        accuracy * 100.0,
        correct,
        total,
        mismatches.join("\n")
    );
}

#[test]
fn fixture_set_spans_at_least_two_stances() {
    let dir = fixtures_dir();
    let expected_raw = std::fs::read_to_string(dir.join("expected.json"))
        .expect("reading expected.json");
    let expected: Vec<ExpectedLabel> =
        serde_json::from_str(&expected_raw).expect("parse expected.json");

    let stances: std::collections::HashSet<&str> = expected
        .iter()
        .map(|e| e.expected_stance.as_str())
        .collect();

    assert!(
        stances.len() >= 2,
        "fixture set must span ≥2 stances (AC4), got: {stances:?}"
    );
}
