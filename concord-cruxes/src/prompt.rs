//! Prompt construction for the crux classifier.

use concord_steelman::Steelman;

/// Build a pairwise classification prompt for two steelmans.
///
/// The model is asked to return a JSON object with `shared_values`, `cruxes`,
/// and `misunderstandings` arrays.
#[must_use]
pub fn build_crux_prompt(claim: &str, side_a: &Steelman, side_b: &Steelman) -> String {
    format!(
        "Claim: \"{claim}\"\n\n\
         Side A ({stance_a}):\n\
         Claim: {claim_a}\n\
         Premises:\n{premises_a}\n\
         Conclusion: {conclusion_a}\n\n\
         Side B ({stance_b}):\n\
         Claim: {claim_b}\n\
         Premises:\n{premises_b}\n\
         Conclusion: {conclusion_b}\n\n\
         Classify the relationship between these positions into three buckets:\n\
         1. shared_values: values both sides assert or presuppose.\n\
         2. cruxes: genuine divergences. For each, specify:\n\
            - statement: a concise description\n\
            - kind: \"empirical\" (resolvable by evidence) or \"value\" (weight/priority difference)\n\
            - what_would_change_side_a: what would change side A's mind\n\
            - what_would_change_side_b: what would change side B's mind\n\
         3. misunderstandings: apparent conflicts that dissolve on clarification. For each:\n\
            - apparent_conflict: the apparent conflict\n\
            - clarifying_distinction: the distinction that dissolves it\n\n\
         Return ONLY a JSON object:\n\
         {{\n\
           \"shared_values\": [{{\"statement\": \"...\"}}],\n\
           \"cruxes\": [\n\
             {{\n\
               \"statement\": \"...\",\n\
               \"kind\": \"empirical\" | \"value\",\n\
               \"what_would_change_side_a\": \"...\",\n\
               \"what_would_change_side_b\": \"...\"\n\
             }}\n\
           ],\n\
           \"misunderstandings\": [{{\"apparent_conflict\": \"...\", \"clarifying_distinction\": \"...\"}}]\n\
         }}\n",
        claim = claim,
        stance_a = side_a.stance,
        claim_a = side_a.claim,
        premises_a = format_premises(side_a),
        conclusion_a = side_a.conclusion,
        stance_b = side_b.stance,
        claim_b = side_b.claim,
        premises_b = format_premises(side_b),
        conclusion_b = side_b.conclusion,
    )
}

fn format_premises(steelman: &Steelman) -> String {
    steelman
        .premises
        .iter()
        .enumerate()
        .map(|(i, p)| format!("  {i}. {}", p.text))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use concord_steelman::{Premise, Steelman};

    fn make_steelman(stance: &str, claim: &str) -> Steelman {
        Steelman {
            stance: stance.to_string(),
            claim: claim.to_string(),
            premises: vec![Premise {
                text: "A premise.".to_string(),
                cited_source_ids: vec![0],
            }],
            conclusion: "A conclusion.".to_string(),
            cited_source_ids: vec![0],
            flagged: false,
            flag_reason: String::new(),
        }
    }

    #[test]
    fn prompt_contains_both_stances() {
        let a = make_steelman("Supports", "X is good.");
        let b = make_steelman("Opposes", "X is harmful.");
        let prompt = build_crux_prompt("Is X good?", &a, &b);
        assert!(prompt.contains("Supports"));
        assert!(prompt.contains("Opposes"));
        assert!(prompt.contains("Is X good?"));
    }
}
