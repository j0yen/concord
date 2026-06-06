//! AC3: The dedup pass collapses a fixture set containing 3 syndicated copies of
//! one article into a single retained Source.

use concord_corpus::{build_corpus_default, FixtureGatherer};

fn fixtures_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests/fixtures/dedup_test")
}

#[test]
fn three_syndicated_copies_collapse_to_one() {
    let gatherer = FixtureGatherer::new(fixtures_dir()).expect("fixture gatherer");
    let corpus = build_corpus_default(
        "Does coffee reduce diabetes risk?",
        &gatherer,
    )
    .expect("build corpus");

    // The dedup_test fixture has 3 near-identical coffee-diabetes articles + 1 unique
    // climate article = 2 distinct sources after dedup.
    assert_eq!(
        corpus.source_count(),
        2,
        "Expected 2 sources after dedup (3 syndicated copies → 1 + 1 unique), got {}.\nSources: {:#?}",
        corpus.source_count(),
        corpus.sources.iter().map(|s| &s.url).collect::<Vec<_>>()
    );
}
