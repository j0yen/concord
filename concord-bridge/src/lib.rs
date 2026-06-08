//! `concord-bridge` — balanced brief synthesizer for the concord workspace.
//!
//! Takes a [`concord_cruxes::DisagreementMap`] and synthesizes a single Markdown
//! brief covering: what each side wants, where the real disagreement is, common
//! ground, and what would change minds. Applies a deterministic balance gate
//! before emitting.
//!
//! # Guarantee
//! No network I/O unless the caller provides a network-capable [`BridgeModel`].
//! [`MockBridgeModel`] is the canonical test implementation.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod balance;
pub mod engine;
pub mod model;

pub use balance::check_balance;
pub use engine::{synthesize_brief, BriefOutput};
pub use model::{BridgeModel, MockBridgeModel};
