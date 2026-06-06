//! Prompt builders for the de-escalation model calls.

/// Build a prompt asking the model to extract substantive asks from `message`.
///
/// The model is expected to return a JSON object `{"asks": ["ask1", "ask2", ...]}`.
#[must_use]
pub fn build_extract_asks_prompt(message: &str) -> String {
    format!(
        "Extract all substantive asks (requests, demands, questions, things the sender wants \
         the recipient to do or answer) from the following message. Return a JSON object with \
         a single key \"asks\" whose value is an array of strings, one per distinct ask. \
         Be precise — do not paraphrase or combine asks. Do not add commentary.\n\n\
         Message:\n{message}\n\nJSON:"
    )
}

/// Build a prompt asking the model to rephrase `message` in OFNR form.
///
/// `asks` is the list of substantive asks to ensure are preserved.
/// The model should return only the rephrased message — no commentary.
#[must_use]
pub fn build_rephrase_prompt(message: &str, asks: &[&str]) -> String {
    let asks_list = if asks.is_empty() {
        "(extract from the message)".to_string()
    } else {
        asks.iter()
            .enumerate()
            .map(|(i, a)| format!("  {}. {}", i + 1, a))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        "Rephrase the following heated message into calm, non-inflammatory language using the \
         Nonviolent Communication (NVC) format: observation, feeling, need, request. \
         IMPORTANT: every substantive ask listed below MUST appear in the rephrased output. \
         Do NOT add commentary, headers, or explanation — output only the rephrased message.\n\n\
         Substantive asks to preserve:\n{asks_list}\n\n\
         Original message:\n{message}\n\nRephrased message:"
    )
}

/// Build a prompt asking the model to explain what changed in the rephrase.
///
/// The model should return a JSON array of `{{\"change\": ..., \"reason\": ...}}` objects.
#[must_use]
pub fn build_explain_prompt(original: &str, rephrased: &str) -> String {
    format!(
        "Compare the original and rephrased messages below. For each significant change made \
         (contempt term removed, tone softened, framing shifted), output a JSON array where \
         each entry has \"change\" (what changed) and \"reason\" (why it helps de-escalate). \
         Return only the JSON array, no commentary.\n\n\
         Original:\n{original}\n\nRephrased:\n{rephrased}\n\nJSON:"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_prompt_contains_message() {
        let p = build_extract_asks_prompt("Please send the report.");
        assert!(p.contains("Please send the report."));
        assert!(p.contains("asks"));
    }

    #[test]
    fn rephrase_prompt_contains_asks() {
        let p = build_rephrase_prompt("You never listen!", &["listen to me"]);
        assert!(p.contains("listen to me"));
        assert!(p.contains("You never listen!"));
    }
}
