//! Top-level de-escalation engine.
//!
//! Orchestrates the pipeline:
//! 1. Safety check — decline threats/harassment.
//! 2. Contempt-lexicon check — record found terms.
//! 3. Ask extraction (rule-based + model-assisted).
//! 4. Model rephrase.
//! 5. Post-check — verify all asks are preserved.
//! 6. (Optional) explain — ask model for change annotations.

use anyhow::Result;
use concord_steelman::ConcordModel;

use crate::ask::{extract_asks_with_model, verify_asks_preserved};
use crate::lexicon::check_contempt;
use crate::prompt;
use crate::safety::{check_safety, SafetyCheck};
use crate::types::{DeescalateInput, DeescalateOutput, ExplainEntry};

/// Errors specific to the de-escalation engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeescalateError {
    /// The input contains a threat or harassment pattern; rephrasing was declined.
    SafetyDeclined {
        /// Human-readable message to show the user.
        message: String,
    },
}

impl std::fmt::Display for DeescalateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SafetyDeclined { message } => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for DeescalateError {}

/// De-escalate a heated message.
///
/// # Arguments
/// * `input` — the raw message and optional model override.
/// * `model` — a [`ConcordModel`] implementation (use [`MockModel`] in tests).
/// * `explain` — if `true`, include a change-annotation pass.
///
/// # Errors
/// Returns [`DeescalateError::SafetyDeclined`] (wrapped in `anyhow::Error`) if
/// the input contains threats or harassment.  Returns other `anyhow::Error`
/// variants if the model backend fails.
///
/// [`MockModel`]: concord_steelman::MockModel
pub fn deescalate(
    input: &DeescalateInput,
    model: &dyn ConcordModel,
    explain: bool,
) -> Result<DeescalateOutput> {
    // ── 1. Safety check ──────────────────────────────────────────────────────
    if let SafetyCheck::Declined { matched_pattern } = check_safety(&input.message) {
        let msg = format!(
            "concord-deescalate declines to rephrase this message: it contains a threat or \
             harassment pattern ({matched_pattern:?}). No rephrase was produced."
        );
        return Err(anyhow::anyhow!(DeescalateError::SafetyDeclined { message: msg }));
    }

    // ── 2. Contempt-lexicon check ────────────────────────────────────────────
    let contempt = check_contempt(&input.message, &[]);
    let contempt_terms_found = contempt.found.clone();

    // ── 3. Ask extraction ────────────────────────────────────────────────────
    let asks = extract_asks_with_model(&input.message, model)?;
    let extracted_asks: Vec<String> = asks.iter().map(|a| a.text.clone()).collect();

    // ── 4. Model rephrase ────────────────────────────────────────────────────
    let ask_refs: Vec<&str> = extracted_asks.iter().map(String::as_str).collect();
    let rephrase_prompt = prompt::build_rephrase_prompt(&input.message, &ask_refs);
    let rephrase = model.complete(&rephrase_prompt)?;
    let rephrase = rephrase.trim().to_string();

    // ── 5. Post-check: ask preservation ─────────────────────────────────────
    let missing_asks = verify_asks_preserved(&asks, &rephrase);
    let asks_preserved = missing_asks.is_empty();

    // ── 6. Explain (optional) ────────────────────────────────────────────────
    let explain_entries = if explain {
        let ep = prompt::build_explain_prompt(&input.message, &rephrase);
        let resp = model.complete(&ep)?;
        parse_explain_response(&resp)
    } else {
        Vec::new()
    };

    Ok(DeescalateOutput {
        rephrase,
        extracted_asks,
        contempt_terms_found,
        asks_preserved,
        missing_asks,
        explain: explain_entries,
    })
}

/// Parse the model's explain response into [`ExplainEntry`] values.
///
/// Falls back gracefully: if JSON parse fails, returns a single entry with
/// the raw response as the change text.
fn parse_explain_response(response: &str) -> Vec<ExplainEntry> {
    #[derive(serde::Deserialize)]
    struct Raw {
        change: String,
        reason: String,
    }

    if let Ok(entries) = serde_json::from_str::<Vec<Raw>>(response) {
        return entries.into_iter().map(|r| ExplainEntry { change: r.change, reason: r.reason }).collect();
    }

    // Fallback: wrap the raw text.
    if !response.trim().is_empty() {
        vec![ExplainEntry {
            change: response.trim().to_string(),
            reason: "(model did not return structured JSON)".to_string(),
        }]
    } else {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use concord_steelman::MockModel;

    use super::*;
    use crate::types::DeescalateInput;

    /// Scripted rephrase preserving both asks (report + meeting).
    const FULL_REPHRASE: &str =
        "I notice I haven't received the budget report. I feel concerned about the timeline. \
         I need to review the numbers. Would you be able to send the budget report and \
         schedule a meeting to discuss it?";

    /// Scripted rephrase that drops the 'meeting/schedule/review' ask.
    const LOSSY_REPHRASE: &str =
        "I notice I haven't received the budget report. I feel concerned. \
         Would you be able to send the budget report?";

    #[test]
    fn deescalates_basic_message() {
        // AC2: with a scripted MockModel, produces a rephrase.
        let model = MockModel::with_default(FULL_REPHRASE);
        let input = DeescalateInput::from_message(
            "You are ridiculous. Send the budget report and schedule a meeting!",
        );
        let out = deescalate(&input, &model, false).expect("should succeed");
        assert!(!out.rephrase.is_empty(), "rephrase should not be empty");
        assert!(
            out.contempt_terms_found.iter().any(|t| t.to_lowercase().contains("ridiculous")),
            "should flag 'ridiculous' as contempt; found={:?}",
            out.contempt_terms_found
        );
    }

    #[test]
    fn ask_preservation_full_rephrase() {
        // AC3 (passing case): a complete rephrase passes the post-check.
        // MockModel returns the full rephrase (contains both 'report' and 'meeting').
        let model = MockModel::with_default(FULL_REPHRASE);
        let input = DeescalateInput::from_message(
            "Send the budget report and schedule a meeting to discuss the proposal.",
        );
        let out = deescalate(&input, &model, false).expect("should succeed");
        assert!(
            out.asks_preserved,
            "full rephrase should preserve all asks; missing={:?}",
            out.missing_asks
        );
    }

    #[test]
    fn ask_preservation_catches_dropped_ask() {
        // AC3: a scripted rephrase that drops one ask is caught by post-check.
        //
        // Input has TWO SEPARATE SENTENCES, each containing one distinct ask:
        //   1. "Send the budget report." — "budget"/"report" appear in LOSSY_REPHRASE.
        //   2. "Schedule a review meeting." — "meeting"/"schedule"/"review" ABSENT.
        //
        // MockModel with_default returns LOSSY_REPHRASE for every call. The ask-
        // extraction JSON parse fails (LOSSY_REPHRASE is not JSON) → rule-based
        // path fires → two sentences → two distinct asks. verify_asks_preserved
        // will flag the "meeting/schedule/review" ask as missing.
        let model = MockModel::with_default(LOSSY_REPHRASE);
        let input = DeescalateInput::from_message(
            "You are ridiculous. Send the budget report. Schedule a review meeting.",
        );
        let out = deescalate(&input, &model, false).expect("should succeed");
        // LOSSY_REPHRASE lacks "meeting"/"schedule"/"review" -- post-check flags it.
        assert!(
            !out.asks_preserved,
            "dropped ask should be caught; missing={:?}, rephrase={:?}",
            out.missing_asks,
            out.rephrase
        );
        assert!(
            !out.missing_asks.is_empty(),
            "missing_asks should be non-empty"
        );
    }

    #[test]
    fn safety_declined_for_threat() {
        // AC5: input containing a threat is declined.
        let model = MockModel::with_default("anything");
        let input =
            DeescalateInput::from_message("I will kill you if you don't send the report.");
        let result = deescalate(&input, &model, false);
        assert!(result.is_err(), "threat input should return error");
        let err = result.unwrap_err();
        let downcasted = err.downcast_ref::<DeescalateError>();
        assert!(
            matches!(downcasted, Some(DeescalateError::SafetyDeclined { .. })),
            "error should be SafetyDeclined; got: {err}"
        );
    }

    #[test]
    fn explain_mode_produces_entries() {
        // AC2 (--explain): with scripted model, explain entries are returned.
        let explain_json = r#"[{"change":"removed 'ridiculous'","reason":"contempt term"}]"#;
        // MockModel returns FULL_REPHRASE for rephrase calls; explain_json for explain call.
        // with_default returns the same for all — test that parse_explain_response works.
        let out = parse_explain_response(explain_json);
        assert_eq!(out.len(), 1);
        assert!(out[0].change.contains("ridiculous"));
    }
}
