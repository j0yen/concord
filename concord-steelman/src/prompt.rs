//! Prompt assembly for the steelman engine.
//!
//! Builds a structured prompt from a [`StanceSummary`] and its associated
//! [`Source`]s in a [`Corpus`].

use concord_corpus::{Corpus, Stance};

/// Build the steelman prompt for a given stance within `corpus`.
///
/// The prompt asks the model for the strongest good-faith version of the
/// argument that `stance_label` proponents would endorse, grounded in the
/// sources provided.
///
/// # Format
/// The returned prompt is plain text, under ~800 words for fast CPU inference.
#[must_use]
pub fn build_prompt(corpus: &Corpus, stance: &Stance, stance_label: &str) -> String {
    let matching: Vec<(usize, &concord_corpus::Source)> = corpus
        .sources
        .iter()
        .enumerate()
        .filter(|(_, s)| &s.stance == stance)
        .collect();

    let mut parts = Vec::new();
    parts.push(format!(
        "You are helping build a steelmanned argument.\n\
         Claim under analysis: \"{}\"\n\
         Stance: {} ({})\n\n\
         Your task: Produce the STRONGEST, most good-faith version of the argument \
         that a thoughtful {} proponent would actually endorse.\n\
         Do NOT use straw men, contempt, or dismissals.\n\
         Do NOT use phrases like \"obviously\", \"any reasonable person\", \
         \"clearly\", or contemptuous language.\n\n\
         Ground each premise in the sources below (cite by SOURCE_ID).\n",
        corpus.claim,
        stance,
        stance_label,
        stance_label,
    ));

    if matching.is_empty() {
        parts.push(
            "No direct sources available for this stance; reason from first principles.\n"
                .to_string(),
        );
    } else {
        parts.push("Sources:\n".to_string());
        for (idx, src) in &matching {
            let excerpt = if src.excerpt.len() > 300 {
                &src.excerpt[..300]
            } else {
                &src.excerpt
            };
            parts.push(format!(
                "[SOURCE_ID={idx}] {}: {}\nExcerpt: {}\n",
                src.publisher, src.title, excerpt
            ));
        }
    }

    parts.push(
        "\nRespond in this EXACT format (no extra text before or after):\n\
         CLAIM: <one sentence central claim>\n\
         PREMISE_1: <premise text> [SOURCE_ID=<id>]\n\
         PREMISE_2: <premise text> [SOURCE_ID=<id>]\n\
         PREMISE_3: <premise text> [SOURCE_ID=<id>]\n\
         CONCLUSION: <concise conclusion>\n"
            .to_string(),
    );

    parts.join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use concord_corpus::{Corpus, ProvenanceRecord, Source, Stance};
    use chrono::Utc;

    fn make_corpus() -> Corpus {
        let src = Source {
            title: "Test Title".to_string(),
            url: "http://example.com".to_string(),
            publisher: "Example".to_string(),
            retrieved_at: Utc::now(),
            excerpt: "Short excerpt.".to_string(),
            full_text: "Full text body.".to_string(),
            stance: Stance::Supports,
            stance_label: "pro-test".to_string(),
            credibility: 0.8,
            provenance: ProvenanceRecord {
                gatherer: "fixture".to_string(),
                query: "test".to_string(),
                rank: 0,
            },
        };
        Corpus::from_sources("Should X be done?", vec![src])
    }

    #[test]
    fn prompt_contains_claim() {
        let corpus = make_corpus();
        let prompt = build_prompt(&corpus, &Stance::Supports, "pro-test");
        assert!(prompt.contains("Should X be done?"));
        assert!(prompt.contains("SOURCE_ID=0"));
        assert!(prompt.contains("CLAIM:"));
        assert!(prompt.contains("CONCLUSION:"));
    }

    #[test]
    fn prompt_empty_stance_sources() {
        let corpus = make_corpus();
        let prompt = build_prompt(&corpus, &Stance::Opposes, "anti-test");
        assert!(prompt.contains("No direct sources available"));
    }
}
