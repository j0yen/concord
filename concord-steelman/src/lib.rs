//! `concord-steelman` — produces the strongest good-faith version of each stance.
//!
//! Given a [`Corpus`] this crate generates one [`Steelman`] per
//! [`StanceSummary`] by prompting a [`ConcordModel`] implementation and then
//! validating:
//!
//! 1. **Citation integrity** — every premise must cite a [`Source`] id present
//!    in the corpus; uncited assertions are rejected.
//! 2. **Anti-caricature** — the steelman may not contain contempt-lexicon terms
//!    or cheap dismissal phrases; violations trigger one regeneration attempt
//!    before the stance is flagged.
//!
//! # Offline / cloud-build safety
//! No live model is ever required. The [`MockModel`] returns scripted responses
//! keyed by a short prompt fingerprint and is used in all automated tests.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod engine;
pub mod guard;
pub mod model;
pub mod prompt;

pub use engine::{steelman_corpus, SteelmanResult};
pub use model::{ConcordModel, MockModel};
pub use prompt::build_prompt;

use serde::{Deserialize, Serialize};

/// A steelmanned version of a stance: the argument a thoughtful proponent
/// would actually endorse.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Steelman {
    /// The stance this steelman represents.
    pub stance: String,
    /// The central claim the proponent makes.
    pub claim: String,
    /// Supporting premises, each citing at least one source by its index in
    /// [`Corpus::sources`].
    pub premises: Vec<Premise>,
    /// Concise conclusion.
    pub conclusion: String,
    /// Ids (0-indexed positions in [`Corpus::sources`]) cited by any premise.
    pub cited_source_ids: Vec<usize>,
    /// Whether this steelman was flagged (citation failure or caricature).
    pub flagged: bool,
    /// Human-readable reason if flagged, empty otherwise.
    pub flag_reason: String,
}

/// A single supporting premise with source citations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Premise {
    /// The premise text.
    pub text: String,
    /// Source indices (0-indexed) this premise cites.
    pub cited_source_ids: Vec<usize>,
}
