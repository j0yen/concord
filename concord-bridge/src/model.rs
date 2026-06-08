//! [`BridgeModel`] trait and [`MockBridgeModel`] for deterministic testing.

use anyhow::Result;

/// LLM interface for the brief synthesis stage.
pub trait BridgeModel: Send + Sync {
    /// Generate brief sections from a structured prompt.
    ///
    /// # Errors
    /// Returns an error if the backend fails.
    fn synthesize(&self, prompt: &str) -> Result<String>;
}

/// A scripted model for deterministic testing. Never reaches a real backend.
pub struct MockBridgeModel {
    default_response: String,
    responses: Vec<(String, String)>,
}

/// Fingerprint length for prompt matching.
const FINGERPRINT_LEN: usize = 64;

impl MockBridgeModel {
    /// Create a [`MockBridgeModel`] with a given default response.
    #[must_use]
    pub fn new(default_response: impl Into<String>) -> Self {
        Self {
            default_response: default_response.into(),
            responses: Vec::new(),
        }
    }

    /// Create a placeholder model for CLI use when no live model is configured.
    #[must_use]
    pub fn new_placeholder() -> Self {
        Self::new(
            "WHAT_EACH_SIDE_WANTS:\n\
             Supports: [Live model required for real synthesis]\n\
             Opposes: [Live model required for real synthesis]\n\
             WHERE_DISAGREEMENT_IS:\n\
             [No crux data without live model]\n\
             COMMON_GROUND:\n\
             [No shared values data without live model]\n\
             WHAT_WOULD_CHANGE_MINDS:\n\
             [No change-minds data without live model]",
        )
    }

    /// Add a keyed override: when the prompt starts with `key_prefix` (up to
    /// [`FINGERPRINT_LEN`] chars), return `response`.
    #[must_use]
    pub fn with_response(mut self, key_prefix: impl Into<String>, response: impl Into<String>) -> Self {
        self.responses.push((key_prefix.into(), response.into()));
        self
    }
}

impl BridgeModel for MockBridgeModel {
    fn synthesize(&self, prompt: &str) -> Result<String> {
        let fp: String = prompt.chars().take(FINGERPRINT_LEN).collect();
        for (key, resp) in &self.responses {
            let key_fp: String = key.chars().take(FINGERPRINT_LEN).collect();
            if fp.starts_with(key_fp.as_str()) || fp == key_fp {
                return Ok(resp.clone());
            }
        }
        Ok(self.default_response.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_bridge_model_default() {
        let m = MockBridgeModel::new("result");
        assert_eq!(m.synthesize("prompt").expect("ok"), "result");
    }

    #[test]
    fn mock_never_hits_real_backend() {
        let m = MockBridgeModel::new("safe");
        let _ = m.synthesize("anything").expect("ok");
    }
}
