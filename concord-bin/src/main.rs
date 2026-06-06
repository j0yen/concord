//! `concord` — perspective-diverse corpus builder and analysis tool.
//!
//! # Usage
//! ```text
//! concord corpus build "<claim>" [--fixtures <dir>] [--out corpus.json]
//! concord corpus show <corpus.json>
//! concord corpus stats <corpus.json>
//! concord steelman <corpus.json> [--out steelmanned.json] [--model <name>]
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
    use concord_steelman::{steelman_corpus, model::LadderModel};

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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    }
}
