//! JSON parsing and post-validation for the model's classification response.

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::map::{Crux, CruxKind, DisagreementMap, Misunderstanding, SharedValue};

/// Parse and validate a model response into a [`DisagreementMap`].
///
/// # Errors
/// Returns an error if the JSON is malformed or any entry fails post-validation.
pub fn parse_and_validate(
    claim: &str,
    response: &str,
) -> Result<(DisagreementMap, Vec<String>)> {
    let json_str = extract_json(response).unwrap_or(response);
    let raw: RawMap =
        serde_json::from_str(json_str).context("parsing model classification JSON")?;

    let mut warnings: Vec<String> = Vec::new();
    let mut map = DisagreementMap::new(claim);

    // Shared values.
    for sv in raw.shared_values {
        map.shared_values.push(SharedValue { statement: sv.statement });
    }

    // Cruxes — post-validate: must have non-empty change-minds fields.
    for (i, c) in raw.cruxes.into_iter().enumerate() {
        if c.what_would_change_side_a.trim().is_empty()
            || c.what_would_change_side_b.trim().is_empty()
        {
            warnings.push(format!(
                "crux[{i}] missing change-minds field — skipped"
            ));
            continue;
        }
        map.cruxes.push(Crux {
            statement: c.statement,
            kind: c.kind,
            what_would_change_side_a: c.what_would_change_side_a,
            what_would_change_side_b: c.what_would_change_side_b,
        });
    }

    // Misunderstandings — post-validate: must have a clarifying distinction.
    for (i, m) in raw.misunderstandings.into_iter().enumerate() {
        if m.clarifying_distinction.trim().is_empty() {
            warnings.push(format!(
                "misunderstanding[{i}] missing clarifying_distinction — skipped"
            ));
            continue;
        }
        map.misunderstandings.push(Misunderstanding {
            apparent_conflict: m.apparent_conflict,
            clarifying_distinction: m.clarifying_distinction,
        });
    }

    Ok((map, warnings))
}

/// Pull the first `{...}` block out of `text`.
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0usize;
    let mut end = None;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    end = Some(start + i + 1);
                    break;
                }
            }
            _ => {}
        }
    }
    end.map(|e| &text[start..e])
}

// ── Deserialization helpers ────────────────────────────────────────────────

#[derive(Deserialize)]
struct RawMap {
    #[serde(default)]
    shared_values: Vec<RawSharedValue>,
    #[serde(default)]
    cruxes: Vec<RawCrux>,
    #[serde(default)]
    misunderstandings: Vec<RawMisunderstanding>,
}

#[derive(Deserialize)]
struct RawSharedValue {
    statement: String,
}

#[derive(Deserialize)]
struct RawCrux {
    statement: String,
    kind: CruxKind,
    what_would_change_side_a: String,
    what_would_change_side_b: String,
}

#[derive(Deserialize)]
struct RawMisunderstanding {
    apparent_conflict: String,
    clarifying_distinction: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good_json() -> &'static str {
        r#"{
          "shared_values": [{"statement": "Both value safety."}],
          "cruxes": [{
            "statement": "Does X cause harm?",
            "kind": "empirical",
            "what_would_change_side_a": "A replication study.",
            "what_would_change_side_b": "A long-term safety study."
          }],
          "misunderstandings": [{
            "apparent_conflict": "A vs B on 'natural'.",
            "clarifying_distinction": "They mean different things by 'natural'."
          }]
        }"#
    }

    #[test]
    fn parses_valid_response() {
        let (map, warnings) = parse_and_validate("Is X good?", good_json()).unwrap();
        assert_eq!(map.shared_values.len(), 1);
        assert_eq!(map.cruxes.len(), 1);
        assert_eq!(map.misunderstandings.len(), 1);
        assert!(warnings.is_empty());
    }

    #[test]
    fn crux_without_change_minds_skipped_with_warning() {
        let json = r#"{
          "shared_values": [],
          "cruxes": [{
            "statement": "Incomplete crux",
            "kind": "value",
            "what_would_change_side_a": "",
            "what_would_change_side_b": "something"
          }],
          "misunderstandings": []
        }"#;
        let (map, warnings) = parse_and_validate("claim", json).unwrap();
        assert!(map.cruxes.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("missing change-minds field"));
    }

    #[test]
    fn misunderstanding_without_distinction_skipped() {
        let json = r#"{
          "shared_values": [],
          "cruxes": [],
          "misunderstandings": [{
            "apparent_conflict": "Some conflict",
            "clarifying_distinction": ""
          }]
        }"#;
        let (map, warnings) = parse_and_validate("claim", json).unwrap();
        assert!(map.misunderstandings.is_empty());
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("missing clarifying_distinction"));
    }

    #[test]
    fn crux_kind_value_deserializes() {
        let json = r#"{
          "shared_values": [],
          "cruxes": [{
            "statement": "Value crux",
            "kind": "value",
            "what_would_change_side_a": "If they reconsider priorities.",
            "what_would_change_side_b": "If costs are revisited."
          }],
          "misunderstandings": []
        }"#;
        let (map, _) = parse_and_validate("claim", json).unwrap();
        assert_eq!(map.cruxes[0].kind, CruxKind::Value);
    }

    #[test]
    fn each_entry_in_exactly_one_bucket() {
        let (map, _) = parse_and_validate("claim", good_json()).unwrap();
        // No entry appears in more than one bucket (structural guarantee by schema).
        assert_eq!(map.total_entries(), 3);
    }
}
