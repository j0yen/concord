//! `concord` — perspective-diverse corpus builder and analysis tool.
//!
//! # Usage
//! ```text
//! concord corpus build "<claim>" [--fixtures <dir>] [--out corpus.json]
//! concord corpus show <corpus.json>
//! concord corpus stats <corpus.json>
//! ```

// SIGPIPE reset MUST be the very first thing in main() so that
// `concord corpus show <file> | head` does not panic with a broken-pipe
// error. See: self_sigpipe_panic_toolkit memory note.
use sigpipe;

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

fn cmd_show(corpus_path: &std::path::Path) -> Result<()> {
    use tabled::{Table, Tabled};

    let raw = std::fs::read_to_string(corpus_path)
        .with_context(|| format!("reading {}", corpus_path.display()))?;
    let corpus: concord_corpus::Corpus =
        serde_json::from_str(&raw).context("parsing corpus JSON")?;

    println!("Claim: {}", corpus.claim);
    println!("Sources: {}", corpus.source_count());
    println!();

    #[derive(Tabled)]
    struct Row {
        #[tabled(rename = "Stance")]
        stance: String,
        #[tabled(rename = "Credibility")]
        credibility: String,
        #[tabled(rename = "Publisher")]
        publisher: String,
        #[tabled(rename = "Title")]
        title: String,
    }

    let rows: Vec<Row> = corpus
        .sources
        .iter()
        .map(|s| Row {
            stance: s.stance.to_string(),
            credibility: format!("{:.2}", s.credibility),
            publisher: truncate(&s.publisher, 20),
            title: truncate(&s.title, 50),
        })
        .collect();

    let table = Table::new(rows).to_string();
    println!("{table}");
    Ok(())
}

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
