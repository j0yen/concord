//! AC2: `concord corpus build "<claim>" --fixtures <dir>` produces a Corpus JSON
//! validating against the documented schema, with every Source carrying a Stance
//! and a provenance record.

use std::process::Command;

use tempfile::TempDir;

fn fixtures_dir() -> std::path::PathBuf {
    // tests/fixtures/basic_claim is committed to the repo.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests/fixtures/basic_claim")
}

#[test]
fn corpus_build_produces_valid_json_with_stances_and_provenance() {
    let tmp = TempDir::new().expect("tempdir");
    let out_path = tmp.path().join("corpus.json");

    let status = Command::new(env!("CARGO_BIN_EXE_concord"))
        .args([
            "corpus",
            "build",
            "Does coffee reduce diabetes risk?",
            "--fixtures",
            fixtures_dir().to_str().unwrap(),
            "--out",
            out_path.to_str().unwrap(),
        ])
        .status()
        .expect("failed to run concord corpus build");

    assert!(status.success(), "concord corpus build should exit 0, got: {status}");
    assert!(out_path.exists(), "corpus.json should be written");

    let raw = std::fs::read_to_string(&out_path).expect("reading corpus.json");
    let corpus: serde_json::Value = serde_json::from_str(&raw).expect("parse corpus JSON");

    // Schema: must have claim, sources array, stances array.
    assert!(corpus["claim"].is_string(), "corpus.claim must be a string");
    let sources = corpus["sources"].as_array().expect("corpus.sources must be array");
    assert!(!sources.is_empty(), "corpus must have at least one source");

    for (i, source) in sources.iter().enumerate() {
        assert!(source["title"].is_string(), "source[{i}].title missing");
        assert!(source["url"].is_string(), "source[{i}].url missing");
        assert!(source["stance"].is_string(), "source[{i}].stance missing");
        assert!(source["provenance"].is_object(), "source[{i}].provenance missing");
        let prov = &source["provenance"];
        assert!(prov["gatherer"].is_string(), "source[{i}].provenance.gatherer missing");
        assert!(prov["query"].is_string(), "source[{i}].provenance.query missing");
        assert!(prov["rank"].is_number(), "source[{i}].provenance.rank missing");
    }

    let stances = corpus["stances"].as_array().expect("corpus.stances must be array");
    assert!(!stances.is_empty(), "corpus.stances must not be empty");
}
