//! Property-based tests for corpus invariants.
//! READ-ONLY after scaffold.

use proptest::prelude::*;

use concord_corpus::dedup::{dedup_sources, DEFAULT_THRESHOLD};
use concord_corpus::stance::tag_stance;

proptest! {
    /// Dedup is idempotent: running it twice gives the same result as once.
    #[test]
    fn dedup_idempotent(seed: u64) {
        // Build a small set of identical sources.
        let sources = (0..3u64).map(|i| {
            let text = format!("seed:{seed} identical body text about some contested claim");
            make_source(&format!("https://example.com/{i}"), &text, 0.5)
        }).collect::<Vec<_>>();

        let once = dedup_sources(sources.clone(), DEFAULT_THRESHOLD);
        let twice = dedup_sources(once.clone(), DEFAULT_THRESHOLD);
        prop_assert_eq!(once.len(), twice.len());
    }

    /// Dedup never increases the source count.
    #[test]
    fn dedup_never_grows(n in 1usize..10) {
        let sources = (0..n).map(|i| {
            make_source(&format!("https://example.com/{i}"), "different body text for this source", 0.5)
        }).collect::<Vec<_>>();
        let original_len = sources.len();
        let result = dedup_sources(sources, DEFAULT_THRESHOLD);
        prop_assert!(result.len() <= original_len);
    }

    /// Stance tagger always returns one of the four valid variants.
    #[test]
    fn stance_tagger_always_valid(title in ".*", body in ".*") {
        use concord_corpus::Stance;
        let (stance, _) = tag_stance(&title, &body);
        let valid = matches!(stance, Stance::Supports | Stance::Opposes | Stance::Mixed | Stance::Background);
        prop_assert!(valid);
    }
}

fn make_source(url: &str, text: &str, credibility: f32) -> concord_corpus::Source {
    use concord_corpus::{corpus::ProvenanceRecord, Stance};
    use chrono::Utc;
    concord_corpus::Source {
        title: url.to_string(),
        url: url.to_string(),
        publisher: "test".to_string(),
        retrieved_at: Utc::now(),
        excerpt: text.chars().take(100).collect(),
        full_text: text.to_string(),
        stance: Stance::Background,
        stance_label: String::new(),
        credibility,
        provenance: ProvenanceRecord {
            gatherer: "test".to_string(),
            query: "test".to_string(),
            rank: 0,
        },
    }
}
