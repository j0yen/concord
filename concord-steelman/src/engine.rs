//! Steelman engine: drives prompt → model → parse → guard → output.
//!
//! Entry point: [`steelman_corpus`].

use anyhow::{Context, Result};
use concord_corpus::{Corpus, Stance, StanceSummary};

use crate::{
    guard::{check_anti_caricature, check_citations, filter_valid_citations},
    model::ConcordModel,
    prompt::build_prompt,
    Premise, Steelman,
};

/// Result of steelmanning a corpus.
#[derive(Debug)]
pub struct SteelmanResult {
    /// The original corpus.
    pub corpus: Corpus,
    /// One steelman per stance summary in the corpus.
    pub steelmans: Vec<Steelman>,
}

/// Steelman every stance in `corpus` using `model`.
///
/// For each [`StanceSummary`] in `corpus.stances`:
/// 1. Build a prompt from the corpus sources for that stance.
/// 2. Call `model.complete()` to get a response.
/// 3. Parse the response into a [`Steelman`].
/// 4. Apply citation-integrity and anti-caricature guards.
///    - On caricature failure: regenerate once, then flag if still failing.
///    - On citation failure: flag with reason.
///
/// # Errors
/// Returns an error only on unrecoverable failures (model error on first
/// attempt for a stance). Citation/caricature failures produce flagged
/// steelmans rather than errors.
pub fn steelman_corpus(corpus: &Corpus, model: &dyn ConcordModel) -> Result<SteelmanResult> {
    let mut steelmans = Vec::new();
    for summary in &corpus.stances {
        let st = steelman_one(corpus, summary, model)
            .with_context(|| format!("steelmanning stance {:?}", summary.stance))?;
        steelmans.push(st);
    }
    Ok(SteelmanResult { corpus: corpus.clone(), steelmans })
}

/// Steelman a single stance.
fn steelman_one(
    corpus: &Corpus,
    summary: &StanceSummary,
    model: &dyn ConcordModel,
) -> Result<Steelman> {
    let stance_label = derive_stance_label(corpus, &summary.stance);
    let prompt = build_prompt(corpus, &summary.stance, &stance_label);

    // First attempt.
    let response = model.complete(&prompt)?;
    let steel = parse_response(&response, &summary.stance, corpus.sources.len());

    // Anti-caricature check — regenerate once on failure.
    let builder = if let Err(reason) = check_anti_caricature(&full_text(&steel)) {
        let prompt2 = build_regeneration_prompt(&prompt, &reason);
        model.complete(&prompt2).map_or_else(
            |_| {
                // Model errored on retry — flag original.
                steel.flagged_with(format!("anti-caricature: {reason}"))
            },
            |response2| {
                let steel2 =
                    parse_response(&response2, &summary.stance, corpus.sources.len());
                if let Err(reason2) = check_anti_caricature(&full_text(&steel2)) {
                    // Still failing — flag.
                    steel2.flagged_with(format!("anti-caricature (retry): {reason2}"))
                } else {
                    steel2
                }
            },
        )
    } else {
        steel
    };
    Ok(builder.into_steelman())
}

/// Parse the model's text response into a [`Steelman`].
///
/// Expected format:
/// ```text
/// CLAIM: <text>
/// PREMISE_N: <text> [SOURCE_ID=<n>]
/// CONCLUSION: <text>
/// ```
///
/// Any premise citing a non-existent source id is kept in text but its
/// citation is removed from `cited_source_ids`; if the steelman ends up
/// with all premises having invalid citations, it is flagged.
fn parse_response(response: &str, stance: &Stance, source_count: usize) -> SteelmanBuilder {
    let mut claim = String::new();
    let mut premises = Vec::new();
    let mut conclusion = String::new();
    let mut all_cited: Vec<usize> = Vec::new();
    let mut bad_citations: Vec<usize> = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("CLAIM:") {
            claim = rest.trim().to_string();
        } else if trimmed.starts_with("PREMISE_") {
            if let Some(colon) = trimmed.find(':') {
                let text_raw = trimmed[colon + 1..].trim();
                let (text, cited) = extract_citations(text_raw);
                let valid = filter_valid_citations(&cited, source_count);
                let bad: Vec<usize> =
                    cited.iter().filter(|id| !valid.contains(id)).copied().collect();
                bad_citations.extend_from_slice(&bad);
                all_cited.extend_from_slice(&valid);
                premises.push(Premise { text, cited_source_ids: valid });
            }
        } else if let Some(rest) = trimmed.strip_prefix("CONCLUSION:") {
            conclusion = rest.trim().to_string();
        }
    }

    // Deduplicate cited ids.
    all_cited.sort_unstable();
    all_cited.dedup();

    let flagged = !bad_citations.is_empty();
    let flag_reason = if flagged {
        format!("invalid source ids: {bad_citations:?}")
    } else {
        String::new()
    };

    SteelmanBuilder {
        stance: stance.to_string(),
        claim,
        premises,
        conclusion,
        cited_source_ids: all_cited,
        flagged,
        flag_reason,
    }
}

/// Extract `[SOURCE_ID=N]` annotations from a premise text, returning
/// `(text_without_annotations, ids)`.
fn extract_citations(raw: &str) -> (String, Vec<usize>) {
    let mut ids = Vec::new();
    let mut text = raw.to_string();

    // Find all [SOURCE_ID=N] occurrences.
    let tag = "[SOURCE_ID=";
    let mut search = raw;
    while let Some(start) = search.find(tag) {
        let after_tag = &search[start + tag.len()..];
        if let Some(end) = after_tag.find(']') {
            let id_str = &after_tag[..end];
            if let Ok(id) = id_str.trim().parse::<usize>() {
                ids.push(id);
            }
            search = &after_tag[end + 1..];
        } else {
            break;
        }
    }

    // Strip annotations from the text.
    text = strip_source_id_tags(&text);

    (text.trim().to_string(), ids)
}

/// Remove all `[SOURCE_ID=N]` annotations from `text`.
fn strip_source_id_tags(text: &str) -> String {
    let mut result = String::new();
    let mut remaining = text;
    let tag = "[SOURCE_ID=";
    loop {
        if let Some(start) = remaining.find(tag) {
            result.push_str(&remaining[..start]);
            let after_tag = &remaining[start + tag.len()..];
            if let Some(end) = after_tag.find(']') {
                remaining = &after_tag[end + 1..];
            } else {
                // Malformed tag — keep the rest verbatim.
                result.push_str(tag);
                result.push_str(after_tag);
                break;
            }
        } else {
            result.push_str(remaining);
            break;
        }
    }
    result
}

/// Derive a stance label from the first source matching the stance, or fall back
/// to the stance name.
fn derive_stance_label(corpus: &Corpus, stance: &Stance) -> String {
    corpus
        .sources
        .iter()
        .find(|s| &s.stance == stance)
        .map_or_else(
            || stance.to_string(),
            |s| {
                if s.stance_label.is_empty() {
                    stance.to_string()
                } else {
                    s.stance_label.clone()
                }
            },
        )
}

/// Build a prompt asking the model to retry without the offending term.
fn build_regeneration_prompt(original_prompt: &str, reason: &str) -> String {
    format!(
        "{original_prompt}\n\n\
         IMPORTANT: Your previous response was rejected because: {reason}.\n\
         Rewrite the steelman WITHOUT using any contemptuous or dismissive language.\n\
         A good steelman shows genuine understanding, not condescension.\n"
    )
}

/// Compose the full text of a steelman for guard checks.
fn full_text(b: &SteelmanBuilder) -> String {
    let premise_texts: Vec<&str> = b.premises.iter().map(|p| p.text.as_str()).collect();
    format!("{} {} {}", b.claim, premise_texts.join(" "), b.conclusion)
}

// ── Builder ──────────────────────────────────────────────────────────────────

/// Intermediate representation during parse + guard.
struct SteelmanBuilder {
    stance: String,
    claim: String,
    premises: Vec<Premise>,
    conclusion: String,
    cited_source_ids: Vec<usize>,
    flagged: bool,
    flag_reason: String,
}

impl SteelmanBuilder {
    /// Mark as flagged with a reason, returning `Self`.
    #[must_use]
    fn flagged_with(mut self, reason: impl Into<String>) -> Self {
        self.flagged = true;
        self.flag_reason = reason.into();
        self
    }

    /// Convert to [`Steelman`].
    fn into_steelman(self) -> Steelman {
        Steelman {
            stance: self.stance,
            claim: self.claim,
            premises: self.premises,
            conclusion: self.conclusion,
            cited_source_ids: self.cited_source_ids,
            flagged: self.flagged,
            flag_reason: self.flag_reason,
        }
    }
}

/// Convenience — check citations across all premises; exposed for tests.
///
/// # Errors
/// Returns `Err(bad_ids)` if any premise references a source index ≥ `source_count`.
pub fn check_all_citations(steel: &Steelman, source_count: usize) -> Result<(), Vec<usize>> {
    let mut bad = Vec::new();
    for premise in &steel.premises {
        if let Err(id) = check_citations(&premise.cited_source_ids, source_count) {
            bad.push(id);
        }
    }
    if bad.is_empty() {
        Ok(())
    } else {
        Err(bad)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_citations_basic() {
        let (text, ids) = extract_citations(
            "The data is strong. [SOURCE_ID=0] [SOURCE_ID=2]",
        );
        assert_eq!(ids, vec![0, 2]);
        assert!(!text.contains("SOURCE_ID"));
        assert!(text.contains("The data is strong."));
    }

    #[test]
    fn extract_citations_no_tags() {
        let (text, ids) = extract_citations("Plain text with no citations.");
        assert!(ids.is_empty());
        assert_eq!(text, "Plain text with no citations.");
    }
}
