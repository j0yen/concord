//! High-level corpus builder: gather → tag → dedup → score → emit.

use anyhow::Result;
use chrono::Utc;

use crate::{
    corpus::{Corpus, ProvenanceRecord, Source},
    credibility::score_credibility,
    dedup::{dedup_sources, DEFAULT_THRESHOLD},
    gatherer::SourceGatherer,
    stance::tag_stance,
};

/// Build a [`Corpus`] for a claim using the supplied gatherer.
///
/// Pipeline:
/// 1. Gather raw documents via `gatherer.gather(claim)`.
/// 2. Tag stance and score credibility for each document.
/// 3. Deduplicate near-identical sources.
/// 4. Return a [`Corpus`] with provenance records.
///
/// # Errors
/// Propagates errors from the gatherer.
pub fn build_corpus(
    claim: &str,
    gatherer: &dyn SourceGatherer,
    dedup_threshold: f64,
) -> Result<Corpus> {
    let raw_docs = gatherer.gather(claim)?;

    let sources: Vec<Source> = raw_docs
        .iter()
        .enumerate()
        .map(|(rank, doc)| {
            let (stance, stance_label) = tag_stance(&doc.title, &doc.body);
            let credibility = score_credibility(doc);
            let excerpt: String = doc.body.chars().take(500).collect();

            Source {
                title: doc.title.clone(),
                url: doc.url.clone(),
                publisher: doc.publisher.clone(),
                retrieved_at: Utc::now(),
                excerpt,
                full_text: doc.body.clone(),
                stance,
                stance_label,
                credibility,
                provenance: ProvenanceRecord {
                    gatherer: gatherer.name().to_string(),
                    query: claim.to_string(),
                    rank,
                },
            }
        })
        .collect();

    let deduped = dedup_sources(sources, dedup_threshold);
    Ok(Corpus::from_sources(claim, deduped))
}

/// Build with default dedup threshold.
///
/// # Errors
/// Propagates errors from the gatherer.
pub fn build_corpus_default(claim: &str, gatherer: &dyn SourceGatherer) -> Result<Corpus> {
    build_corpus(claim, gatherer, DEFAULT_THRESHOLD)
}
