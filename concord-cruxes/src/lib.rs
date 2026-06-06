//! `concord-cruxes` — separates disagreements into shared values, genuine cruxes,
//! and terminological misunderstandings.
//!
//! # Input
//! A pair of [`Steelman`]s (from `concord-steelman`) representing the two sides
//! of a contested claim.
//!
//! # Output
//! A [`DisagreementMap`] containing:
//! - **`shared_values`** — values both sides already hold in common.
//! - **`cruxes`** — genuine divergences, tagged [`CruxKind::Empirical`] or
//!   [`CruxKind::Value`], each with a "what would change minds" field.
//! - **`misunderstandings`** — apparent conflicts that dissolve on clarification;
//!   each carries the clarifying distinction.
//!
//! # Offline / cloud-build safety
//! Classification is performed by a [`concord_steelman::ConcordModel`]. The
//! [`concord_steelman::MockModel`] is used in all automated tests. No network
//! I/O occurs unless the caller wires a `LadderModel`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod classifier;
pub mod engine;
pub mod map;
pub mod prompt;

pub use engine::{analyze_steelmans, CruxAnalysis};
pub use map::{Crux, CruxKind, DisagreementMap, Misunderstanding, SharedValue};
