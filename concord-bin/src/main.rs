//! `concord` — perspective-diverse corpus builder and analysis tool.
//!
//! # Usage
//! ```text
//! concord corpus build "<claim>" [--fixtures <dir>] [--out corpus.json]
//! concord corpus show <corpus.json>
//! concord corpus stats <corpus.json>
//! concord steelman <corpus.json> [--out steelmanned.json] [--model <name>]
//! concord cruxes analyze <steelmanned.json> [--out map.json]
//! concord cruxes show <map.json>
//! concord bridge <map.json> [--out brief.md] [--model <name>]
//! concord run "<claim>" --fixtures <dir> [--out brief.md]
//! concord deescalate [--in msg.txt | "<message>"] [--model <name>] [--explain]
//! ```

// Binary entry point: print_stderr (eprintln!) and print_stdout (println!) are
// intentional for user-facing status messages and output. These are the correct
// idioms for a CLI — not library code.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "concord",
    version,
    about = "Perspective-diverse source corpus builder for contested claims"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Corpus management subcommands.
    Corpus {
        #[command(subcommand)]
        action: CorpusAction,
    },
    /// Steelman every stance in a corpus using a local LLM.
    Steelman {
        /// Path to the input corpus JSON file.
        corpus: PathBuf,
        /// Output path for the steelmanned JSON (default: steelmanned.json).
        #[arg(long, default_value = "steelmanned.json")]
        out: PathBuf,
        /// Ollama model name to use (default: qwen2.5:3b).
        #[arg(long, default_value = "qwen2.5:3b")]
        model: String,
    },
    /// Crux analysis subcommands.
    Cruxes {
        #[command(subcommand)]
        action: CruxesAction,
    },
    /// Synthesize a balanced brief from a disagreement map.
    Bridge {
        /// Path to map.json produced by `concord cruxes analyze`.
        map: PathBuf,
        /// Output path for brief.md (default: brief.md).
        #[arg(long, default_value = "brief.md")]
        out: PathBuf,
        /// Ollama model name to use (default: qwen2.5:3b).
        #[arg(long, default_value = "qwen2.5:3b")]
        model: String,
    },
    /// Run the full pipeline: corpus → steelman → cruxes → bridge.
    Run {
        /// The contested claim to analyse.
        claim: String,
        /// Directory of fixture sources (offline mode, no network).
        #[arg(long)]
        fixtures: PathBuf,
        /// Output path for brief.md (default: brief.md).
        #[arg(long, default_value = "brief.md")]
        out: PathBuf,
    },
    /// Rephrase a heated message into calm, non-inflammatory form.
    Deescalate {
        /// The message to rephrase (use `-` to read from stdin, or omit for --in).
        #[arg(conflicts_with = "file")]
        message: Option<String>,
        /// Path to a file containing the message to rephrase.
        #[arg(long = "in", value_name = "FILE")]
        file: Option<PathBuf>,
        /// Ollama model name to use (default: qwen2.5:3b).
        #[arg(long, default_value = "qwen2.5:3b")]
        model: String,
        /// Print an explanation of each change made.
        #[arg(long)]
        explain: bool,
    },
}

#[derive(Subcommand)]
enum CorpusAction {
    /// Build a perspective-diverse corpus for a contested claim.
    Build {
        /// The contested claim to gather sources for.
        claim: String,
        /// Directory of fixture sources (offline mode, no network).
        #[arg(long)]
        fixtures: Option<PathBuf>,
        /// Output path for the Corpus JSON (default: corpus.json).
        #[arg(long, default_value = "corpus.json")]
        out: PathBuf,
    },
    /// Display a corpus as a human-readable table of sources by stance.
    Show {
        /// Path to the corpus JSON file.
        corpus: PathBuf,
    },
    /// Print stance balance and dedup stats for a corpus.
    Stats {
        /// Path to the corpus JSON file.
        corpus: PathBuf,
    },
}

#[derive(Subcommand)]
enum CruxesAction {
    /// Analyze a steelmanned.json and produce a disagreement map.
    Analyze {
        /// Path to steelmanned.json (from concord steelman).
        steelmanned: PathBuf,
        /// Output path for the DisagreementMap JSON (default: map.json).
        #[arg(long, default_value = "map.json")]
        out: PathBuf,
    },
    /// Display a disagreement map as a human-readable three-column view.
    Show {
        /// Path to map.json (from concord cruxes analyze).
        map: PathBuf,
    },
}

fn main() -> std::process::ExitCode {
    // SIGPIPE reset — must be first. Prevents panic on broken pipe (e.g. | head).
    // See: self_sigpipe_panic_toolkit memory note.
    sigpipe::reset();

    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("concord: error: {e:#}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Corpus { action } => match action {
            CorpusAction::Build { claim, fixtures, out } => {
                cmd_build(&claim, fixtures.as_deref(), &out)
            }
            CorpusAction::Show { corpus } => cmd_show(&corpus),
            CorpusAction::Stats { corpus } => cmd_stats(&corpus),
        },
        Commands::Steelman { corpus, out, model } => cmd_steelman(&corpus, &out, &model),
        Commands::Cruxes { action } => match action {
            CruxesAction::Analyze { steelmanned, out } => {
                cmd_cruxes_analyze(&steelmanned, &out)
            }
            CruxesAction::Show { map } => cmd_cruxes_show(&map),
        },
        Commands::Bridge { map, out, model } => cmd_bridge(&map, &out, &model),
        Commands::Run { claim, fixtures, out } => cmd_run(&claim, &fixtures, &out),
        Commands::Deescalate { message, file, model, explain } => {
            cmd_deescalate(message.as_deref(), file.as_deref(), &model, explain)
        }
    }
}

fn cmd_build(
    claim: &str,
    fixtures: Option<&std::path::Path>,
    out: &std::path::Path,
) -> Result<()> {
    use concord_corpus::{build_corpus_default, FixtureGatherer};

    let gatherer: Box<dyn concord_corpus::SourceGatherer> = match fixtures {
        Some(dir) => Box::new(FixtureGatherer::new(dir)?),
        None => {
            anyhow::bail!(
                "No gatherer configured. Use --fixtures <dir> for offline mode.\n\
                 A network-capable gatherer is out of scope for this release."
            );
        }
    };

    eprintln!("concord: building corpus for claim: {claim}");
    let corpus = build_corpus_default(claim, gatherer.as_ref())?;

    let json = serde_json::to_string_pretty(&corpus)
        .context("serializing corpus to JSON")?;
    std::fs::write(out, &json)
        .with_context(|| format!("writing corpus to {}", out.display()))?;

    eprintln!(
        "concord: wrote {} source(s) to {}",
        corpus.source_count(),
        out.display()
    );
    Ok(())
}

fn cmd_steelman(
    corpus_path: &std::path::Path,
    out: &std::path::Path,
    model_name: &str,
) -> Result<()> {
    use concord_steelman::{model::LadderModel, steelman_corpus};

    let raw = std::fs::read_to_string(corpus_path)
        .with_context(|| format!("reading corpus from {}", corpus_path.display()))?;
    let corpus: concord_corpus::Corpus =
        serde_json::from_str(&raw).context("parsing corpus JSON")?;

    eprintln!("concord: steelmanning {} stance(s) using model '{model_name}'", corpus.stances.len());
    eprintln!("concord: note — LadderModel is a stub; wire wintermute-brain for live inference");

    let model = LadderModel::new(model_name);
    let result = steelman_corpus(&corpus, &model)
        .context("steelman engine failed")?;

    let payload = serde_json::json!({
        "corpus": result.corpus,
        "steelmans": result.steelmans,
    });
    let json = serde_json::to_string_pretty(&payload)
        .context("serializing steelmanned output to JSON")?;
    std::fs::write(out, &json)
        .with_context(|| format!("writing steelmanned output to {}", out.display()))?;

    eprintln!(
        "concord: wrote {} steelman(s) to {}",
        result.steelmans.len(),
        out.display()
    );
    Ok(())
}

// ── concord cruxes analyze ────────────────────────────────────────────────────

/// Steelmanned JSON file as expected by `concord cruxes analyze`.
#[derive(serde::Deserialize)]
struct SteelmanedFile {
    claim: String,
    steelmans: Vec<concord_steelman::Steelman>,
}

fn cmd_cruxes_analyze(steelmanned_path: &std::path::Path, out: &std::path::Path) -> Result<()> {
    use concord_cruxes::analyze_steelmans;
    use concord_steelman::MockModel;

    let raw = std::fs::read_to_string(steelmanned_path)
        .with_context(|| format!("reading {}", steelmanned_path.display()))?;
    let sm: SteelmanedFile =
        serde_json::from_str(&raw).context("parsing steelmanned JSON")?;

    if sm.steelmans.len() < 2 {
        anyhow::bail!(
            "Need at least 2 steelmans to analyze cruxes; got {}",
            sm.steelmans.len()
        );
    }

    // Use MockModel with a canned response for offline/CI mode.
    // For live use with a real model, wire LadderModel here (deferred AC6).
    let model = MockModel::with_default(
        serde_json::json!({
            "shared_values": [],
            "cruxes": [{
                "statement": "Core empirical divergence between positions",
                "kind": "empirical",
                "what_would_change_side_a": "Replicated counter-evidence.",
                "what_would_change_side_b": "Replicated supporting evidence."
            }],
            "misunderstandings": []
        })
        .to_string(),
    );

    eprintln!("concord: analyzing cruxes for claim: {}", sm.claim);
    let analysis =
        analyze_steelmans(&sm.claim, &sm.steelmans[0], &sm.steelmans[1], &model)?;

    if !analysis.warnings.is_empty() {
        for w in &analysis.warnings {
            eprintln!("concord: warning: {w}");
        }
    }

    let json = serde_json::to_string_pretty(&analysis.map).context("serializing map")?;
    std::fs::write(out, &json)
        .with_context(|| format!("writing map to {}", out.display()))?;

    eprintln!(
        "concord: wrote disagreement map to {} ({} shared, {} cruxes, {} misunderstandings)",
        out.display(),
        analysis.map.shared_values.len(),
        analysis.map.cruxes.len(),
        analysis.map.misunderstandings.len(),
    );
    Ok(())
}

// ── concord cruxes show ───────────────────────────────────────────────────────

#[allow(clippy::print_stdout)]
fn cmd_cruxes_show(map_path: &std::path::Path) -> Result<()> {
    use concord_cruxes::{CruxKind, DisagreementMap};

    let raw = std::fs::read_to_string(map_path)
        .with_context(|| format!("reading {}", map_path.display()))?;
    let map: DisagreementMap = serde_json::from_str(&raw).context("parsing map JSON")?;

    println!("Claim: {}", map.claim);
    println!();

    println!("=== SHARED VALUES ({}) ===", map.shared_values.len());
    for sv in &map.shared_values {
        println!("  \u{2022} {}", sv.statement);
    }
    println!();

    println!("=== CRUXES ({}) ===", map.cruxes.len());
    for crux in &map.cruxes {
        let kind = match crux.kind {
            CruxKind::Empirical => "empirical",
            CruxKind::Value => "value",
        };
        println!("  [{kind}] {}", crux.statement);
        println!("    What would change side A: {}", crux.what_would_change_side_a);
        println!("    What would change side B: {}", crux.what_would_change_side_b);
    }
    println!();

    println!("=== MISUNDERSTANDINGS ({}) ===", map.misunderstandings.len());
    for m in &map.misunderstandings {
        println!("  Conflict:     {}", m.apparent_conflict);
        println!("  Distinction:  {}", m.clarifying_distinction);
    }
    Ok(())
}

// print_stdout: these functions are the user-facing display; println! is correct here.
#[allow(clippy::print_stdout)]
fn cmd_show(corpus_path: &std::path::Path) -> Result<()> {
    let raw = std::fs::read_to_string(corpus_path)
        .with_context(|| format!("reading {}", corpus_path.display()))?;
    let corpus: concord_corpus::Corpus =
        serde_json::from_str(&raw).context("parsing corpus JSON")?;

    println!("Claim: {}", corpus.claim);
    println!("Sources: {}", corpus.source_count());
    println!();

    // Hand-rolled ASCII table (avoids tabled crate's MSRV-1.86 transitive deps).
    let col_widths = (10usize, 6usize, 22usize, 52usize);
    let sep = format!(
        "+-{}-+-{}-+-{}-+-{}-+",
        "-".repeat(col_widths.0),
        "-".repeat(col_widths.1),
        "-".repeat(col_widths.2),
        "-".repeat(col_widths.3),
    );
    println!("{sep}");
    println!(
        "| {:<10} | {:<6} | {:<22} | {:<52} |",
        "Stance", "Cred", "Publisher", "Title"
    );
    println!("{sep}");
    for s in &corpus.sources {
        println!(
            "| {:<10} | {:<6} | {:<22} | {:<52} |",
            truncate(&s.stance.to_string(), col_widths.0),
            format!("{:.2}", s.credibility),
            truncate(&s.publisher, col_widths.2),
            truncate(&s.title, col_widths.3),
        );
    }
    println!("{sep}");
    Ok(())
}

#[allow(clippy::print_stdout)]
fn cmd_stats(corpus_path: &std::path::Path) -> Result<()> {
    let raw = std::fs::read_to_string(corpus_path)
        .with_context(|| format!("reading {}", corpus_path.display()))?;
    let corpus: concord_corpus::Corpus =
        serde_json::from_str(&raw).context("parsing corpus JSON")?;

    println!("Claim: {}", corpus.claim);
    println!("Total sources: {}", corpus.source_count());
    println!();
    println!("{:<12} {:>8} {:>12}", "Stance", "Count", "Avg Cred");
    println!("{}", "-".repeat(35));
    for s in &corpus.stances {
        println!(
            "{:<12} {:>8} {:>12.3}",
            s.stance.to_string(),
            s.count,
            s.mean_credibility
        );
    }
    Ok(())
}

// ── concord deescalate ────────────────────────────────────────────────────────

#[allow(clippy::print_stdout)]
fn cmd_deescalate(
    message: Option<&str>,
    file: Option<&std::path::Path>,
    model_name: &str,
    explain: bool,
) -> Result<()> {
    use concord_deescalate::{DeescalateError, DeescalateInput};
    use concord_steelman::model::LadderModel;

    // Resolve the input message.
    let raw_message = match (message, file) {
        (Some(msg), None) => msg.to_string(),
        (None, Some(path)) => std::fs::read_to_string(path)
            .with_context(|| format!("reading message from {}", path.display()))?,
        (None, None) => {
            // Read from stdin.
            use std::io::Read;
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .context("reading message from stdin")?;
            buf
        }
        (Some(_), Some(_)) => {
            anyhow::bail!("provide either a message argument or --in FILE, not both");
        }
    };

    let input = DeescalateInput::from_message(raw_message.trim());

    eprintln!("concord: de-escalating with model '{model_name}'");
    eprintln!("concord: note — LadderModel is a stub; wire wintermute-brain for live inference");

    let model = LadderModel::new(model_name);
    let out = concord_deescalate::deescalate(&input, &model, explain).map_err(|e| {
        // Surface SafetyDeclined as a clear user-facing error.
        if let Some(DeescalateError::SafetyDeclined { message: msg }) =
            e.downcast_ref::<DeescalateError>()
        {
            anyhow::anyhow!("{msg}")
        } else {
            e
        }
    })?;

    if !out.contempt_terms_found.is_empty() {
        eprintln!(
            "concord: contempt terms found: {}",
            out.contempt_terms_found.join(", ")
        );
    }
    if !out.asks_preserved {
        eprintln!(
            "concord: warning — post-check flagged missing asks: {}",
            out.missing_asks.join("; ")
        );
    }

    println!("{}", out.rephrase);

    if explain && !out.explain.is_empty() {
        println!();
        println!("--- Changes ---");
        for entry in &out.explain {
            println!("• {} ({})", entry.change, entry.reason);
        }
    }

    Ok(())
}

// ── concord bridge ────────────────────────────────────────────────────────────

fn cmd_bridge(map_path: &std::path::Path, out: &std::path::Path, _model_name: &str) -> Result<()> {
    use concord_bridge::{synthesize_brief, MockBridgeModel};
    use concord_cruxes::DisagreementMap;

    let raw = std::fs::read_to_string(map_path)
        .with_context(|| format!("reading {}", map_path.display()))?;
    let map: DisagreementMap = serde_json::from_str(&raw).context("parsing map JSON")?;

    // Use placeholder model for offline/stub mode.
    // For live use with a real model, wire LadderModel here (deferred AC7).
    let model = MockBridgeModel::new_placeholder();
    eprintln!("concord: synthesizing brief from {} crux(es)…", map.cruxes.len());
    let brief = synthesize_brief(&map, &model)?;

    std::fs::write(out, &brief.markdown)
        .with_context(|| format!("writing brief to {}", out.display()))?;

    eprintln!("concord: wrote brief to {}", out.display());
    if !brief.balance_ok {
        eprintln!(
            "concord: WARNING — brief failed balance check: {}",
            brief.balance_failures.join("; ")
        );
    }
    Ok(())
}

// ── concord run (full pipeline) ───────────────────────────────────────────────

fn cmd_run(claim: &str, fixtures: &std::path::Path, out: &std::path::Path) -> Result<()> {
    use concord_bridge::{synthesize_brief, MockBridgeModel};
    use concord_corpus::{build_corpus_default, FixtureGatherer};
    use concord_cruxes::analyze_steelmans;
    use concord_steelman::{steelman_corpus, MockModel};

    eprintln!("concord run: step 1/4 — building corpus…");
    let gatherer = FixtureGatherer::new(fixtures)?;
    let corpus = build_corpus_default(claim, &gatherer)?;
    eprintln!("  {} source(s) gathered", corpus.source_count());

    eprintln!("concord run: step 2/4 — steelmanning…");
    let steelman_model = MockModel::with_default(
        "CLAIM: [Live model required]\nPREMISE: [no premises]\nCONCLUSION: [Live model required]",
    );
    let result = steelman_corpus(&corpus, &steelman_model)?;
    eprintln!("  {} steelman(s) generated", result.steelmans.len());

    eprintln!("concord run: step 3/4 — mapping cruxes…");
    let map = if result.steelmans.len() >= 2 {
        let crux_model = MockModel::with_default(
            serde_json::json!({
                "shared_values": [],
                "cruxes": [{
                    "statement": "Core empirical divergence between positions",
                    "kind": "empirical",
                    "what_would_change_side_a": "Replicated counter-evidence.",
                    "what_would_change_side_b": "Replicated supporting evidence."
                }],
                "misunderstandings": []
            })
            .to_string(),
        );
        let analysis = analyze_steelmans(
            claim,
            &result.steelmans[0],
            &result.steelmans[1],
            &crux_model,
        )?;
        eprintln!(
            "  {} shared, {} cruxes, {} misunderstandings",
            analysis.map.shared_values.len(),
            analysis.map.cruxes.len(),
            analysis.map.misunderstandings.len()
        );
        analysis.map
    } else {
        concord_cruxes::DisagreementMap::new(claim)
    };

    eprintln!("concord run: step 4/4 — synthesizing brief…");
    let bridge_model = MockBridgeModel::new_placeholder();
    let brief = synthesize_brief(&map, &bridge_model)?;

    std::fs::write(out, &brief.markdown)
        .with_context(|| format!("writing brief to {}", out.display()))?;
    eprintln!("concord run: done — wrote brief to {}", out.display());
    if !brief.balance_ok {
        eprintln!("concord run: WARNING — brief failed balance check");
    }
    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}
