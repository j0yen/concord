//! Core data model for a perspective-diverse source corpus.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::stance::Stance;

/// A single normalised source document with stance tagging and credibility score.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    /// Document title.
    pub title: String,
    /// Canonical URL.
    pub url: String,
    /// Publisher / domain name.
    pub publisher: String,
    /// When this source was retrieved by the gatherer.
    pub retrieved_at: DateTime<Utc>,
    /// Short excerpt (first ~500 chars of body text after normalization).
    pub excerpt: String,
    /// Full normalized body text (HTML tags stripped).
    pub full_text: String,
    /// Stance this source argues with respect to the claim.
    pub stance: Stance,
    /// Human-readable label for the specific position taken (e.g. "pro-GMO",
    /// "climate-skeptic"). May be empty if the stance is `Background`.
    pub stance_label: String,
    /// Heuristic credibility score in \[0.0, 1.0\]. Not a trust oracle.
    pub credibility: f32,
    /// Provenance: how this source entered the corpus.
    pub provenance: ProvenanceRecord,
}

/// How a source was located and ingested.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProvenanceRecord {
    /// Gatherer implementation name (e.g. "fixture", "web").
    pub gatherer: String,
    /// Original query that surfaced this source.
    pub query: String,
    /// Rank in the gatherer's result list (0-indexed).
    pub rank: usize,
}

/// Summary of a stance's representation in the corpus.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StanceSummary {
    /// The stance category.
    pub stance: Stance,
    /// Number of sources with this stance.
    pub count: usize,
    /// Mean credibility of sources with this stance.
    pub mean_credibility: f32,
}

/// A perspective-diverse evidence base for a single contested claim.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Corpus {
    /// The contested claim this corpus was built for.
    pub claim: String,
    /// Normalised, deduplicated, tagged sources.
    pub sources: Vec<Source>,
    /// Per-stance aggregate statistics.
    pub stances: Vec<StanceSummary>,
}

impl Corpus {
    /// Build a [`Corpus`] from a claim and a raw list of tagged, scored sources.
    ///
    /// Computes [`StanceSummary`] entries automatically.
    #[must_use]
    pub fn from_sources(claim: impl Into<String>, sources: Vec<Source>) -> Self {
        let claim = claim.into();
        let stances = summarize_stances(&sources);
        Self { claim, sources, stances }
    }

    /// Total number of sources.
    #[must_use]
    pub fn source_count(&self) -> usize {
        self.sources.len()
    }
}

fn summarize_stances(sources: &[Source]) -> Vec<StanceSummary> {
    use std::collections::HashMap;

    let mut groups: HashMap<String, (usize, f64)> = HashMap::new();
    for s in sources {
        let key = format!("{:?}", s.stance);
        let entry = groups.entry(key).or_default();
        entry.0 += 1;
        entry.1 += f64::from(s.credibility);
    }

    // Build in a deterministic order.
    let mut summaries: Vec<StanceSummary> = groups
        .into_iter()
        .map(|(key, (count, cred_sum))| {
            let stance: Stance = match key.as_str() {
                "Supports" => Stance::Supports,
                "Opposes" => Stance::Opposes,
                "Mixed" => Stance::Mixed,
                _ => Stance::Background,
            };
            #[allow(clippy::as_conversions, clippy::cast_possible_truncation)]
            let mean_credibility = (cred_sum / count as f64) as f32;
            StanceSummary {
                stance,
                count,
                mean_credibility,
            }
        })
        .collect();

    summaries.sort_by_key(|s| format!("{:?}", s.stance));
    summaries
}
