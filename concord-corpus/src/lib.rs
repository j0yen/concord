//! `concord-corpus` — perspective-diverse source corpus for the concord workspace.
//!
//! This crate provides the [`Corpus`] data model, the [`SourceGatherer`] trait,
//! a [`FixtureGatherer`] for offline / test use, a deterministic stance tagger,
//! a Jaccard-based dedup pass, and a rule-based credibility scorer.
//!
//! # Guarantee
//! No network I/O occurs unless the caller explicitly provides a network-capable
//! [`SourceGatherer`] implementation. The default [`FixtureGatherer`] reads from
//! a local directory only.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod builder;
pub mod corpus;
pub mod credibility;
pub mod dedup;
pub mod gatherer;
pub mod stance;

pub use builder::{build_corpus, build_corpus_default};
pub use corpus::{Corpus, ProvenanceRecord, Source, StanceSummary};
pub use credibility::score_credibility;
pub use dedup::dedup_sources;
pub use gatherer::{FixtureGatherer, RawDocument, SourceGatherer};
pub use stance::{tag_stance, Stance};
