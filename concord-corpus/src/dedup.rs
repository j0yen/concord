//! Near-duplicate detection and collapsing via shingled-token Jaccard similarity.
//!
//! Two sources are considered duplicates if their normalised body text has a
//! Jaccard similarity score (over character-level shingles) ≥ `threshold`.
//! The retained source is the one with the higher credibility score; ties
//! favour the earlier source in the input order.

use crate::corpus::Source;

/// Default Jaccard threshold above which two sources are considered duplicates.
pub const DEFAULT_THRESHOLD: f64 = 0.70;

/// Shingle size (character n-gram length) for Jaccard computation.
const SHINGLE_K: usize = 5;

/// Collapse near-identical sources in `sources` using Jaccard similarity.
///
/// Operates in O(n²) over the source count (fine for corpus sizes of ~100;
/// the PRD's use case is perspective-diversity, not large-scale IR).
///
/// Returns a new `Vec<Source>` with duplicate clusters collapsed into the
/// highest-credibility representative.
///
/// # Arguments
/// * `sources` — the input sources, consumed.
/// * `threshold` — Jaccard threshold in \[0, 1\]. Use [`DEFAULT_THRESHOLD`] for
///   the standard 0.70 cutoff.
#[must_use]
pub fn dedup_sources(sources: Vec<Source>, threshold: f64) -> Vec<Source> {
    if sources.is_empty() {
        return sources;
    }

    // Build shingle sets lazily.
    let shingle_sets: Vec<std::collections::HashSet<String>> = sources
        .iter()
        .map(|s| shingles(&normalise(&s.full_text), SHINGLE_K))
        .collect();

    let n = sources.len();
    // `kept` is indexed by position in `sources`; length equals `sources.len()`,
    // so `kept[i]` and `kept[j]` are always in-bounds for i,j < n.
    // `shingle_sets` has the same length.
    // We allow indexing_slicing here because the invariant is trivially maintained.
    #[allow(clippy::indexing_slicing)]
    let mut kept = vec![true; n]; // which sources survive

    #[allow(clippy::indexing_slicing)]
    for i in 0..n {
        if !kept[i] {
            continue;
        }
        for j in (i + 1)..n {
            if !kept[j] {
                continue;
            }
            let sim = jaccard(&shingle_sets[i], &shingle_sets[j]);
            if sim >= threshold {
                // Drop the lower-credibility one; ties drop j (keep i = earlier).
                if sources[j].credibility > sources[i].credibility {
                    kept[i] = false;
                    break; // i is gone; no point checking further j's for i
                }
                kept[j] = false;
            }
        }
    }

    #[allow(clippy::indexing_slicing)]
    let result = sources
        .into_iter()
        .enumerate()
        .filter_map(|(i, s)| if kept[i] { Some(s) } else { None })
        .collect();
    result
}

/// Normalise text for shingle computation: lowercase, collapse whitespace.
fn normalise(text: &str) -> String {
    text.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Build a set of character-level k-shingles from `text`.
fn shingles(text: &str, k: usize) -> std::collections::HashSet<String> {
    if text.len() < k {
        // Text too short — treat as one shingle.
        return std::iter::once(text.to_string()).collect();
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .windows(k)
        .map(|w| w.iter().collect::<String>())
        .collect()
}

/// Jaccard similarity between two shingle sets: |A ∩ B| / |A ∪ B|.
// as_conversions / cast_precision_loss: usize→f64 for ratio; sets are corpus-sized
// (small), so precision loss is not a concern. float_arithmetic: intentional ratio.
#[allow(
    clippy::as_conversions,
    clippy::cast_precision_loss,
    clippy::float_arithmetic
)]
fn jaccard(a: &std::collections::HashSet<String>, b: &std::collections::HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_source(url: &str, text: &str, credibility: f32) -> Source {
        use crate::corpus::ProvenanceRecord;
        use crate::stance::Stance;
        use chrono::Utc;
        Source {
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

    #[test]
    fn exact_duplicates_collapse_to_one() {
        let text = "Scientists confirm that coffee reduces the risk of several diseases including type 2 diabetes.";
        let sources = vec![
            make_source("https://a.com/1", text, 0.8),
            make_source("https://b.com/2", text, 0.7),
            make_source("https://c.com/3", text, 0.6),
        ];
        let result = dedup_sources(sources, DEFAULT_THRESHOLD);
        assert_eq!(result.len(), 1, "three identical sources should collapse to one");
        assert_eq!(result[0].url, "https://a.com/1", "highest-credibility source retained");
    }

    #[test]
    fn dissimilar_sources_all_retained() {
        let sources = vec![
            make_source("https://a.com/1", "Coffee is beneficial for health and reduces disease risk significantly in studies.", 0.8),
            make_source("https://b.com/2", "Climate change threatens Arctic ice sheets with accelerating melt rates observed since 1980.", 0.7),
            make_source("https://c.com/3", "Quantum computing advances pose new threats to RSA cryptographic infrastructure globally.", 0.6),
        ];
        let result = dedup_sources(sources, DEFAULT_THRESHOLD);
        assert_eq!(result.len(), 3, "dissimilar sources should all be retained");
    }

    #[test]
    fn empty_input_returns_empty() {
        let result = dedup_sources(vec![], DEFAULT_THRESHOLD);
        assert!(result.is_empty());
    }

    #[test]
    fn higher_credibility_wins_tie() {
        let text = "Identical text about coffee and health research findings published today in a journal.";
        let sources = vec![
            make_source("https://low.com", text, 0.3),
            make_source("https://high.com", text, 0.9),
        ];
        let result = dedup_sources(sources, DEFAULT_THRESHOLD);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].url, "https://high.com");
    }
}
