//! Rule-based credibility scoring for corpus sources.
//!
//! Returns a score in \[0.0, 1.0\] based on transparent, inspectable heuristics.
//! This is **not** a trust oracle — it is documented as a heuristic and surfaced
//! so a human can override it. No ML model is used.
//!
//! # Scoring rubric
//! | Signal | Weight |
//! |--------|--------|
//! | Has a non-empty author | +0.15 |
//! | Has a non-empty publication date | +0.10 |
//! | Primary source (URL heuristic: `.gov`, `.edu`, academic) | +0.25 |
//! | Known-reputable domain class | +0.20 |
//! | HTTPS scheme | +0.10 |
//! | Body text ≥ 300 chars (not a stub) | +0.10 |
//! | Title ≥ 10 chars (not empty/stub) | +0.10 |
//!
//! Maximum total: 1.00.

use crate::gatherer::RawDocument;

/// Score the credibility of a [`RawDocument`] and return a value in \[0.0, 1.0\].
///
/// Uses floating-point accumulation of fixed weights. `float_arithmetic` and
/// `as_conversions` are intentionally allowed here — the scoring algorithm is
/// a fixed-weight sum over boolean signals; the imprecision is acceptable for a
/// documented heuristic.
#[must_use]
#[allow(clippy::float_arithmetic)]
pub fn score_credibility(doc: &RawDocument) -> f32 {
    let mut score: f32 = 0.0;

    // Has a named author.
    if !doc.author.trim().is_empty() {
        score += 0.15;
    }

    // Has a publication date.
    if !doc.published_at.trim().is_empty() {
        score += 0.10;
    }

    // HTTPS scheme.
    if doc.url.starts_with("https://") {
        score += 0.10;
    }

    // Primary source heuristic.
    if is_primary_source(&doc.url) {
        score += 0.25;
    } else if is_reputable_domain(&doc.url) {
        score += 0.20;
    }

    // Non-stub body text.
    if doc.body.len() >= 300 {
        score += 0.10;
    }

    // Non-stub title.
    if doc.title.len() >= 10 {
        score += 0.10;
    }

    score.clamp(0.0, 1.0)
}

/// Heuristic: is this URL likely a primary / authoritative source?
fn is_primary_source(url: &str) -> bool {
    let lower = url.to_lowercase();
    let primary_tlds: &[&str] = &[
        ".gov", ".gov.", // government domains
        ".edu", ".edu.", // educational
        // Academic publishers / preprint servers
        "pubmed", "ncbi.nlm.nih.gov", "arxiv.org", "doi.org",
        "nature.com", "sciencedirect", "springer", "wiley.com",
        "pnas.org", "cell.com", "thelancet.com", "nejm.org",
        "bmj.com", "jamanetwork.com",
    ];
    primary_tlds.iter().any(|&pat| lower.contains(pat))
}

/// Heuristic: is this a domain associated with reputable journalism / fact-checking?
fn is_reputable_domain(url: &str) -> bool {
    let lower = url.to_lowercase();
    let reputable: &[&str] = &[
        "reuters.com",
        "apnews.com",
        "bbc.com",
        "bbc.co.uk",
        "nytimes.com",
        "washingtonpost.com",
        "theguardian.com",
        "economist.com",
        "ft.com",
        "snopes.com",
        "factcheck.org",
        "politifact.com",
        "npr.org",
        "pbs.org",
        "wikipedia.org",
    ];
    reputable.iter().any(|&pat| lower.contains(pat))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(url: &str, author: &str, date: &str, body_len: usize) -> RawDocument {
        RawDocument {
            title: "A sufficiently long title for testing".to_string(),
            url: url.to_string(),
            publisher: String::new(),
            body: "x".repeat(body_len),
            author: author.to_string(),
            published_at: date.to_string(),
        }
    }

    #[test]
    fn gov_url_scores_high() {
        let d = doc("https://www.cdc.gov/vaccines/index.html", "CDC", "2024-01-01", 500);
        let score = score_credibility(&d);
        assert!(score >= 0.6, "CDC gov URL should score >= 0.6, got {score}");
    }

    #[test]
    fn unknown_domain_no_metadata_scores_low() {
        let d = doc("http://random-blog.xyz/post", "", "", 50);
        let score = score_credibility(&d);
        assert!(score <= 0.2, "unknown blog with no metadata should score <= 0.2, got {score}");
    }

    #[test]
    fn score_is_clamped_to_unit_interval() {
        let d = doc(
            "https://ncbi.nlm.nih.gov/article/12345",
            "Dr. Smith",
            "2024-06-01",
            1000,
        );
        let score = score_credibility(&d);
        assert!((0.0..=1.0).contains(&score), "score {score} out of [0,1]");
    }
}
