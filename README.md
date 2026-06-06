# concord

> You cannot bridge a divide you can only see one side of.

`concord` is a Rust workspace for perspective-diverse argument analysis. Given a contested claim, it gathers sources from multiple stances, tags each by the position it argues, deduplicates near-identical framings, scores credibility, and emits a structured `Corpus` that later reasoning stages consume.

This repo is the foundation crate (`concord-corpus`) — pure data plumbing, no LLM reasoning — so it ships cloud-build-safe and deterministic.

## Crates

| Crate | Role |
|---|---|
| `concord-corpus` | Corpus schema, fixture-based source gathering, stance tagging, dedup, credibility scoring |
| `concord-steelman` | LLM-assisted steelmanning of each stance's best argument |
| `concord-cruxes` | Disagreement-map engine — finds cruxes vs misunderstandings |
| `concord-deescalate` | Message de-escalation pipeline |
| `concord-bin` | Thin CLI binary wiring all subcommands |

## Install

```sh
cargo install --git https://github.com/j0yen/concord concord-bin
```

Or clone and build locally:

```sh
git clone https://github.com/j0yen/concord
cd concord
cargo build --release
# binary at target/release/concord
```

Requires Rust 1.85+.

## Usage

```sh
# Build a perspective-diverse corpus from fixture sources
concord corpus build "climate change is primarily human-caused" --fixtures tests/fixtures/climate

# Show sources grouped by stance
concord corpus show corpus.json

# Stance balance and dedup stats
concord corpus stats corpus.json
```

## Acceptance criteria (v1)

1. `cargo build` + `cargo test` green; `concord --help` lists `corpus` subcommand. MSRV 1.85.
2. `concord corpus build "<claim>" --fixtures <dir>` produces valid `Corpus` JSON with every `Source` carrying a `Stance` and provenance record.
3. Dedup pass collapses 3 syndicated copies of one article into a single retained `Source` (asserted in unit tests).
4. Stance tagger assigns correct `Stance` to ≥80% of a hand-labelled fixture set (≥15 sources, ≥2 stances), with labels authored before the tagger.
5. No code path in default `build` reaches the network unless a real gatherer is explicitly configured; test suite passes with networking unavailable.
6. `main()` calls `sigpipe::reset()` first; `concord corpus show <file> | head` does not panic.

## License

Licensed under either of MIT or Apache-2.0 at your option.
