//! Shared types for `concord-deescalate`.

use serde::{Deserialize, Serialize};

/// Input to the de-escalation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeescalateInput {
    /// The raw, potentially heated message to rephrase.
    pub message: String,
    /// Optional override for the model name (e.g. `"qwen2.5:3b"`).
    #[serde(default)]
    pub model: Option<String>,
}

impl DeescalateInput {
    /// Construct a [`DeescalateInput`] from a plain message string.
    #[must_use]
    pub fn from_message(message: impl Into<String>) -> Self {
        Self { message: message.into(), model: None }
    }
}

/// Output from the de-escalation engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeescalateOutput {
    /// The rephrased message in OFNR (observation/feeling/need/request) form.
    pub rephrase: String,
    /// Substantive asks extracted from the original.
    pub extracted_asks: Vec<String>,
    /// Contempt-lexicon terms found and stripped.
    pub contempt_terms_found: Vec<String>,
    /// Whether all asks were verified present in the rephrase.
    pub asks_preserved: bool,
    /// Which asks (if any) were missing from the rephrase.
    pub missing_asks: Vec<String>,
    /// Optional explanation entries (populated with `--explain`).
    #[serde(default)]
    pub explain: Vec<ExplainEntry>,
}

/// A single change-explanation entry (for `--explain` output).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainEntry {
    /// Human-readable description of the change.
    pub change: String,
    /// The rationale for making the change.
    pub reason: String,
}
