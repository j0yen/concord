# concord

Takes a contested claim and works it into a balanced brief: gather sources across stances, steelman each side, map where the real disagreement is, then synthesize. A Rust pipeline for arguing honestly.

> You cannot bridge a divide you can only see one side of.

## Why it exists

Most argument tooling optimizes for winning. The harder and more useful problem is the opposite: lay out a contested claim so that every side is represented at its strongest, and the disagreement is located precisely — which parts are empirical, which are value clashes, which are just two people using the same word differently. Do that well and a bridge becomes possible; skip it and you're arguing past each other.

`concord` is that work, staged. Gather a perspective-diverse corpus, steelman each stance, separate genuine cruxes from misunderstandings, and synthesize a brief that holds the balance. Each stage is its own crate with a checked output schema, so a later stage consumes the earlier one's JSON rather than redoing it.

## The pipeline

```
corpus → steelman → cruxes → bridge
```

| Crate | Stage | What it does |
|---|---|---|
| `concord-corpus` | gather | Build a `Corpus`: tag each source by stance, dedup near-identical framings, score credibility. Pure data — no network in default `build`, no LLM. |
| `concord-steelman` | strengthen | State each stance's best argument, with citation-integrity and anti-caricature guards. |
| `concord-cruxes` | locate | Separate steelmanned positions into shared values, genuine cruxes (empirical vs value), and terminological misunderstandings → a `DisagreementMap`. |
| `concord-bridge` | synthesize | Compose the map into a balanced brief, gated on balance and citation integrity. |
| `concord-deescalate` | (standalone) | Rephrase a heated message into NVC form (observation / feeling / need / request), preserving every substantive ask. |
| `concord-bin` | CLI | Wires every subcommand into the `concord` binary. |

The LLM stages (`steelman`, `cruxes`, `bridge`, `deescalate`) call a local Ollama model — default `qwen2.5:3b`. The test suite swaps in a deterministic `MockModel`, so the whole workspace builds and tests offline with no model and no network.

## Install

Requires Rust 1.85+.

```sh
cargo install --git https://github.com/j0yen/concord concord-bin
```

Or build from a checkout:

```sh
git clone https://github.com/j0yen/concord
cd concord
cargo build --release          # binary at target/release/concord
```

## Usage

Run the whole pipeline offline against fixture sources:

```sh
concord run "climate change is primarily human-caused" \
  --fixtures concord-bin/tests/fixtures/basic_claim \
  --out brief.md
```

Or drive the stages one at a time:

```sh
concord corpus build "<claim>" --fixtures <dir> --out corpus.json
concord corpus show corpus.json          # sources grouped by stance
concord corpus stats corpus.json         # stance balance + dedup stats

concord steelman corpus.json --out steelmanned.json
concord cruxes analyze steelmanned.json --out map.json
concord cruxes show map.json             # shared values | cruxes | misunderstandings
concord bridge map.json --out brief.md
```

And the standalone de-escalator:

```sh
concord deescalate "you never listen and you're wrong" --explain
```

`main()` resets `SIGPIPE` before any I/O, so `concord corpus show <file> | head` doesn't panic.

## Status

`v0.6.0`. All four pipeline stages plus the de-escalator are built and wired; every stage has a checked output schema and an accuracy or balance gate proven against independently authored fixtures. The live-LLM acceptance criteria (real Ollama, not `MockModel`) are deferred per stage — see [CHANGELOG.md](CHANGELOG.md) for exactly what's proven where.

## License

MIT OR Apache-2.0, at your option.
