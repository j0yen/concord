//! `concord-deescalate` — rephrases heated messages into non-inflammatory form.
//!
//! Takes a message that wraps a legitimate ask in contemptuous, sarcastic, or
//! absolutist language and rephrases it in observation/feeling/need/request form
//! **while preserving every substantive ask**.
//!
//! # Design
//! - [`engine::deescalate`] — top-level entry point; runs lexicon check, ask
//!   extraction, model rephrase, and post-check in sequence.
//! - [`lexicon`] — deterministic contempt-lexicon check; no model required.
//! - [`ask`] — rule-based + model-assisted substantive-ask extraction.
//! - [`safety`] — threat/harassment detection that blocks rephrasing.
//! - [`prompt`] — prompt builders for the model calls.
//! - [`types`] — shared types ([`DeescalateInput`], [`DeescalateOutput`]).
//!
//! # Offline / cloud-build safety
//! No live model is required. [`concord_steelman::MockModel`] is used in all
//! automated tests.  No network I/O occurs unless the caller wires a
//! `LadderModel`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod ask;
pub mod engine;
pub mod lexicon;
pub mod prompt;
pub mod safety;
pub mod types;

pub use engine::{deescalate, DeescalateError};
pub use types::{DeescalateInput, DeescalateOutput, ExplainEntry};
