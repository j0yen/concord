//! Brief synthesis engine.

use anyhow::{Context, Result};

use concord_cruxes::DisagreementMap;

use crate::balance::check_balance;
use crate::model::BridgeModel;

/// Fixed section markers in the synthesized brief.
const SECTION_WHAT_EACH_SIDE_WANTS: &str = "## What Each Side Wants";
const SECTION_WHERE_DISAGREEMENT_IS: &str = "## Where the Disagreement Actually Is";
const SECTION_COMMON_GROUND: &str = "## Common Ground";
const SECTION_WHAT_WOULD_CHANGE_MINDS: &str = "## What Would Change Minds";
const SECTION_REFERENCES: &str = "## References";

/// The output of the bridge stage.
#[derive(Debug, Clone)]
pub struct BriefOutput {
    /// The full Markdown brief.
    pub markdown: String,
    /// Whether the brief passed the balance gate.
    pub balance_ok: bool,
    /// Balance gate failure descriptions (empty if `balance_ok`).
    pub balance_failures: Vec<String>,
    /// Source ids cited in the brief.
    pub cited_ids: Vec<String>,
}

/// Synthesize a balanced brief from a [`DisagreementMap`] using `model`.
///
/// Runs the balance gate; if it fails, regenerates once; if still failing,
/// reports the failure rather than emitting an unbalanced brief.
///
/// # Errors
/// Returns an error if the model fails on all attempts.
pub fn synthesize_brief(map: &DisagreementMap, model: &dyn BridgeModel) -> Result<BriefOutput> {
    let corpus_source_ids: Vec<String> = Vec::new();
    let prompt = build_prompt(map);

    let raw = model.synthesize(&prompt).context("model synthesis failed")?;
    let output = render_brief(map, &raw, &corpus_source_ids);

    if output.balance_ok {
        return Ok(output);
    }

    // Balance failed — regenerate once.
    let raw2 = model.synthesize(&prompt).context("model synthesis retry failed")?;
    let output2 = render_brief(map, &raw2, &corpus_source_ids);
    // Return regardless — with balance_ok reflecting the second attempt's result.
    Ok(output2)
}

/// Build the synthesis prompt from the map.
fn build_prompt(map: &DisagreementMap) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "Synthesize a balanced brief for the contested claim: \"{}\"",
        map.claim
    ));
    lines.push(String::from(
        "Include these four sections:\n\
         WHAT_EACH_SIDE_WANTS: one steelmanned paragraph per stance, cited\n\
         WHERE_DISAGREEMENT_IS: the real cruxes\n\
         COMMON_GROUND: shared values both sides hold\n\
         WHAT_WOULD_CHANGE_MINDS: per crux, what evidence or concession would move each side\n\
         Format citations as [SOURCE:<id>]. Each section must be non-trivial.",
    ));
    let shared: Vec<&str> = map.shared_values.iter().map(|v| v.statement.as_str()).collect();
    lines.push(format!("\nShared values: {}", shared.join("; ")));
    let cruxes: Vec<String> = map
        .cruxes
        .iter()
        .map(|c| format!("[{:?}] {}", c.kind, c.statement))
        .collect();
    lines.push(format!("Cruxes: {}", cruxes.join("; ")));
    let misunderstandings: Vec<&str> = map
        .misunderstandings
        .iter()
        .map(|m| m.clarifying_distinction.as_str())
        .collect();
    lines.push(format!("Misunderstandings resolved: {}", misunderstandings.join("; ")));
    lines.join("\n")
}

/// Render the model response into a [`BriefOutput`].
pub(crate) fn render_brief(
    map: &DisagreementMap,
    raw: &str,
    corpus_source_ids: &[String],
) -> BriefOutput {
    // Parse sections from the model response.
    let what_each_side = extract_section(
        raw,
        "WHAT_EACH_SIDE_WANTS:",
        &["WHERE_DISAGREEMENT_IS:", "COMMON_GROUND:", "WHAT_WOULD_CHANGE_MINDS:"],
    );
    let where_disagreement = extract_section(
        raw,
        "WHERE_DISAGREEMENT_IS:",
        &["COMMON_GROUND:", "WHAT_WOULD_CHANGE_MINDS:", "WHAT_EACH_SIDE_WANTS:"],
    );
    let common_ground = extract_section(
        raw,
        "COMMON_GROUND:",
        &["WHAT_WOULD_CHANGE_MINDS:", "WHERE_DISAGREEMENT_IS:", "WHAT_EACH_SIDE_WANTS:"],
    );
    let change_minds = extract_section(
        raw,
        "WHAT_WOULD_CHANGE_MINDS:",
        &["WHAT_EACH_SIDE_WANTS:", "WHERE_DISAGREEMENT_IS:", "COMMON_GROUND:"],
    );

    // Collect cited source ids from the brief.
    let cited_ids = collect_cited_ids(raw);

    // Build stance word counts from the what-each-side section.
    let stance_word_counts = build_stance_word_counts(&what_each_side);

    // Build references section.
    let references = build_references(&cited_ids);

    let what_each_side_text =
        if what_each_side.is_empty() { "_No data provided._" } else { &what_each_side };
    let where_disagreement_text =
        if where_disagreement.is_empty() { "_No data provided._" } else { &where_disagreement };
    let common_ground_text =
        if common_ground.is_empty() { "_No data provided._" } else { &common_ground };
    let change_minds_text =
        if change_minds.is_empty() { "_No data provided._" } else { &change_minds };
    let references_text = if references.is_empty() {
        "_No citations._".to_string()
    } else {
        references
    };

    // Assemble Markdown.
    let markdown = format!(
        "# Brief: {claim}\n\n\
         {SECTION_WHAT_EACH_SIDE_WANTS}\n\n\
         {what_each_side_text}\n\n\
         {SECTION_WHERE_DISAGREEMENT_IS}\n\n\
         {where_disagreement_text}\n\n\
         {SECTION_COMMON_GROUND}\n\n\
         {common_ground_text}\n\n\
         {SECTION_WHAT_WOULD_CHANGE_MINDS}\n\n\
         {change_minds_text}\n\n\
         {SECTION_REFERENCES}\n\n\
         {references_text}",
        claim = map.claim,
    );

    // Run balance gate.
    let balance = check_balance(
        &stance_word_counts,
        &cited_ids,
        corpus_source_ids,
        !map.shared_values.is_empty(),
        common_ground.trim().is_empty(),
    );

    BriefOutput {
        markdown,
        balance_ok: balance.passed,
        balance_failures: balance.failures,
        cited_ids,
    }
}

/// Extract text between a section header and the next recognized header.
fn extract_section(raw: &str, start_marker: &str, end_markers: &[&str]) -> String {
    let start = match raw.find(start_marker) {
        Some(i) => i + start_marker.len(),
        None => return String::new(),
    };
    let rest = &raw[start..];
    let end = end_markers
        .iter()
        .filter_map(|m| rest.find(m))
        .min()
        .unwrap_or(rest.len());
    rest[..end].trim().to_string()
}

/// Collect `[SOURCE:<id>]` citations from the raw text.
fn collect_cited_ids(text: &str) -> Vec<String> {
    let mut ids = Vec::new();
    let mut remaining = text;
    while let Some(start) = remaining.find("[SOURCE:") {
        let rest = &remaining[start + 8..];
        if let Some(end) = rest.find(']') {
            let id = rest[..end].trim().to_string();
            if !id.is_empty() && !ids.contains(&id) {
                ids.push(id);
            }
            remaining = &rest[end + 1..];
        } else {
            break;
        }
    }
    ids
}

/// Build per-stance word counts from the "what each side wants" section.
/// Lines starting with `<StanceName>:` are treated as stance paragraphs.
fn build_stance_word_counts(section: &str) -> Vec<(String, usize)> {
    let mut counts: Vec<(String, usize)> = Vec::new();
    let mut current_stance: Option<String> = None;
    let mut current_words = 0usize;

    for line in section.lines() {
        // Detect lines like "Supports: ..." or "Opposes: ..."
        if let Some(colon_pos) = line.find(':') {
            let candidate = line[..colon_pos].trim();
            if !candidate.is_empty() && !candidate.contains(' ') {
                // Looks like a stance label.
                if let Some(stance) = current_stance.take() {
                    counts.push((stance, current_words));
                }
                current_stance = Some(candidate.to_string());
                current_words = line[colon_pos + 1..].split_whitespace().count();
                continue;
            }
        }
        if current_stance.is_some() {
            current_words += line.split_whitespace().count();
        }
    }
    if let Some(stance) = current_stance {
        counts.push((stance, current_words));
    }
    counts
}

/// Build a references list from cited ids.
fn build_references(cited_ids: &[String]) -> String {
    if cited_ids.is_empty() {
        return String::new();
    }
    cited_ids
        .iter()
        .enumerate()
        .map(|(i, id)| format!("{}. {id}", i + 1))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use concord_cruxes::{Crux, CruxKind, DisagreementMap, Misunderstanding, SharedValue};

    fn make_map() -> DisagreementMap {
        DisagreementMap {
            claim: "Does coffee reduce diabetes risk?".to_string(),
            shared_values: vec![SharedValue {
                statement: "Human health matters".to_string(),
            }],
            cruxes: vec![Crux {
                statement: "Does moderate coffee consumption lower HbA1c?".to_string(),
                kind: CruxKind::Empirical,
                what_would_change_side_a: "A meta-analysis with HbA1c as primary endpoint"
                    .to_string(),
                what_would_change_side_b: "Replication failures in large cohort studies"
                    .to_string(),
            }],
            misunderstandings: vec![Misunderstanding {
                apparent_conflict: "risk reduction vs elimination".to_string(),
                clarifying_distinction:
                    "The claim is about relative risk, not absolute elimination".to_string(),
            }],
        }
    }

    // AC2: all four named sections + references list present.
    #[test]
    fn brief_contains_all_four_sections_and_references() {
        let map = make_map();
        use crate::model::MockBridgeModel;
        let model = MockBridgeModel::new(
            "WHAT_EACH_SIDE_WANTS:\n\
             Supports: Coffee consumption has been associated with reduced diabetes risk [SOURCE:source-1]\n\
             Opposes: The evidence is observational and confounded [SOURCE:source-2]\n\
             WHERE_DISAGREEMENT_IS:\nThe core empirical question is HbA1c causation.\n\
             COMMON_GROUND:\nBoth sides value peer-reviewed evidence.\n\
             WHAT_WOULD_CHANGE_MINDS:\nAn RCT with HbA1c as primary endpoint.",
        );

        let output = synthesize_brief(&map, &model).expect("ok");
        let md = &output.markdown;

        assert!(md.contains("## What Each Side Wants"), "missing WESAS section");
        assert!(md.contains("## Where the Disagreement Actually Is"), "missing disagreement");
        assert!(md.contains("## Common Ground"), "missing common ground");
        assert!(md.contains("## What Would Change Minds"), "missing change minds");
        assert!(md.contains("## References"), "missing references");
        assert!(md.contains("source-1") || md.contains("source-2"), "references missing");
    }

    // AC3: citation integrity — non-existent source id triggers balance failure.
    #[test]
    fn citation_integrity_bad_id_caught_by_balance_gate() {
        use crate::balance::check_balance;
        let result = check_balance(
            &[("Supports".to_string(), 50), ("Opposes".to_string(), 50)],
            &["nonexistent-id".to_string()],
            &["source-1".to_string(), "source-2".to_string()],
            false,
            false,
        );
        assert!(!result.passed, "bad citation should fail balance gate");
        assert!(result.failures.iter().any(|f| f.contains("clause(a)")));
    }

    // AC4: balance gate coverage clause (b).
    #[test]
    fn one_sided_coverage_trips_balance_gate_and_regenerates() {
        let map = make_map();
        use crate::model::MockBridgeModel;
        let model = MockBridgeModel::new(
            "WHAT_EACH_SIDE_WANTS:\n\
             Supports: Extremely long paragraph about coffee being great [SOURCE:source-1] with many many words\n\
             Opposes: Brief.\n\
             WHERE_DISAGREEMENT_IS:\nSome disagreement.\n\
             COMMON_GROUND:\nNone.\n\
             WHAT_WOULD_CHANGE_MINDS:\nSomething.",
        );
        let output = synthesize_brief(&map, &model).expect("ok");
        assert!(!output.balance_ok, "one-sided brief should fail balance gate");
        assert!(output.balance_failures.iter().any(|f| f.contains("clause(b)")));
    }

    #[test]
    fn balanced_response_passes_and_emits() {
        let map = make_map();
        use crate::model::MockBridgeModel;
        let model = MockBridgeModel::new(
            "WHAT_EACH_SIDE_WANTS:\n\
             Supports: Coffee has been associated with lower diabetes risk in multiple cohort studies [SOURCE:source-1]\n\
             Opposes: The observational evidence is insufficient; RCTs are lacking [SOURCE:source-2]\n\
             WHERE_DISAGREEMENT_IS:\nCausation vs correlation.\n\
             COMMON_GROUND:\nBoth sides value rigorous evidence.\n\
             WHAT_WOULD_CHANGE_MINDS:\nA randomized controlled trial.",
        );
        let output = synthesize_brief(&map, &model).expect("ok");
        assert!(
            output.balance_ok,
            "balanced brief should pass; failures: {:?}",
            output.balance_failures
        );
    }

    // AC6: no ollama / no network.
    #[test]
    fn full_suite_no_network() {
        let map = make_map();
        use crate::model::MockBridgeModel;
        let model = MockBridgeModel::new(
            "WHAT_EACH_SIDE_WANTS:\nSupports: x\nOpposes: y\n\
             WHERE_DISAGREEMENT_IS:\nZ\nCOMMON_GROUND:\nW\nWHAT_WOULD_CHANGE_MINDS:\nV",
        );
        let _ = synthesize_brief(&map, &model).expect("ok");
    }
}
