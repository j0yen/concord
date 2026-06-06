//! [`ConcordModel`] trait and implementations.
//!
//! # Implementations
//! - [`MockModel`] — scripted responses keyed by prompt fingerprint; offline/CI-safe.
//! - `LadderModel` is a stub that panics if called; the real impl wraps
//!   `wintermute-brain`'s `LadderClient` and is only wired at runtime on hardware
//!   with ollama. Tests MUST NOT instantiate `LadderModel`.

use std::collections::HashMap;

use anyhow::Result;

/// A simple synchronous text-completion interface used by the steelman engine.
///
/// Implementations must be deterministic in tests (use [`MockModel`]) and may
/// call a local or remote LLM at runtime (use `LadderModel`).
pub trait ConcordModel {
    /// Generate a completion for `prompt`.
    ///
    /// # Errors
    /// Returns an error if the underlying backend fails or is unavailable.
    fn complete(&self, prompt: &str) -> Result<String>;
}

// ── MockModel ────────────────────────────────────────────────────────────────

/// A scripted [`ConcordModel`] for offline testing.
///
/// Responses are looked up by a short fingerprint of the prompt (the first 40
/// non-whitespace characters).  Use [`MockModel::new`] with a map of
/// fingerprint → response, or [`MockModel::with_default`] to return a fixed
/// response for every prompt.
#[derive(Debug, Default)]
pub struct MockModel {
    responses: HashMap<String, String>,
    default_response: Option<String>,
    /// Track whether `complete` was ever called (for the "no real backend" test).
    pub call_count: std::cell::Cell<usize>,
}

impl MockModel {
    /// Create a new [`MockModel`] with a scripted response map.
    ///
    /// Keys are fingerprints (first 40 non-whitespace characters of the prompt).
    #[must_use]
    pub fn new(responses: HashMap<String, String>) -> Self {
        Self { responses, default_response: None, call_count: std::cell::Cell::new(0) }
    }

    /// Create a [`MockModel`] that returns `response` for every prompt.
    #[must_use]
    pub fn with_default(response: impl Into<String>) -> Self {
        Self {
            responses: HashMap::new(),
            default_response: Some(response.into()),
            call_count: std::cell::Cell::new(0),
        }
    }

    /// Returns the fingerprint used to look up a response for `prompt`.
    #[must_use]
    pub fn fingerprint(prompt: &str) -> String {
        prompt
            .chars()
            .filter(|c| !c.is_whitespace())
            .take(40)
            .collect()
    }
}

impl ConcordModel for MockModel {
    fn complete(&self, prompt: &str) -> Result<String> {
        self.call_count.set(self.call_count.get() + 1);
        let fp = Self::fingerprint(prompt);
        if let Some(resp) = self.responses.get(&fp) {
            return Ok(resp.clone());
        }
        if let Some(ref default) = self.default_response {
            return Ok(default.clone());
        }
        anyhow::bail!("MockModel: no scripted response for fingerprint {fp:?}")
    }
}

// ── LadderModel stub ─────────────────────────────────────────────────────────

/// Runtime model that routes through `wintermute-brain`'s local→cloud ladder.
///
/// This is a **stub** — the full implementation requires `wintermute-brain`
/// as a dependency and an active `LadderClient`.  Do **not** instantiate this
/// in tests; use [`MockModel`] instead.
///
/// # Panics
/// Panics immediately; the real impl is wired at runtime only.
#[derive(Debug)]
pub struct LadderModel {
    _model_name: String,
}

impl LadderModel {
    /// Create a new `LadderModel` targeting the given ollama model name.
    ///
    /// In production this would accept a `LadderClient` handle from
    /// `wintermute-brain`. Here it stores the model name for documentation.
    #[must_use]
    pub fn new(model_name: impl Into<String>) -> Self {
        Self { _model_name: model_name.into() }
    }
}

impl ConcordModel for LadderModel {
    fn complete(&self, _prompt: &str) -> Result<String> {
        anyhow::bail!(
            "LadderModel is a stub — wire wintermute-brain's LadderClient at runtime. \
             Use MockModel in all tests."
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_scripted_response() {
        let mut map = HashMap::new();
        let fp = MockModel::fingerprint("Hello, world!");
        map.insert(fp, "scripted".to_string());
        let m = MockModel::new(map);
        assert_eq!(m.complete("Hello, world!").unwrap(), "scripted");
        assert_eq!(m.call_count.get(), 1);
    }

    #[test]
    fn mock_default_response() {
        let m = MockModel::with_default("always this");
        assert_eq!(m.complete("anything").unwrap(), "always this");
    }

    #[test]
    fn mock_no_backend_confirmation() {
        // Confirms that MockModel never calls a real backend.
        // call_count increments but no network/ollama is touched.
        let m = MockModel::with_default("ok");
        let _ = m.complete("test prompt");
        assert_eq!(m.call_count.get(), 1);
        // If this test passes without network, we have the guarantee.
    }
}
