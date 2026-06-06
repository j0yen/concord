# Changelog

## v0.2.0 — concord-steelman (parallel integrate)

Add concord-steelman: steelman engine for perspective-diverse argument analysis.

Implements ConcordModel trait (MockModel + LadderModel stub), steelman engine
with citation-integrity and anti-caricature guards, and `concord steelman`
subcommand. Restructures workspace: binary + acceptance tests moved to new
concord-bin crate to break dep cycle. All 40 tests green, no network required.

## v0.1.0 — concord-corpus

Initial release: perspective-diverse source corpus builder with FixtureGatherer,
stance tagger, Jaccard dedup, and credibility scorer.
