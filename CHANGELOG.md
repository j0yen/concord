# Changelog

## v0.3.0 — concord-cruxes

Add concord-cruxes: disagreement map engine separating positions into shared
values, genuine cruxes (empirical/value-tagged with change-minds fields), and
terminological misunderstandings. Output is DisagreementMap JSON, input for
concord-bridge. Wires `concord cruxes analyze/show` into concord-bin. Includes
independently authored held-out fixture (13 labeled points) for accuracy gate
at ≥70%. All ACs 1-5 met; AC6 deferred (live model). 22 new tests green.

## v0.2.0 — concord-steelman (parallel integrate)

Add concord-steelman: steelman engine for perspective-diverse argument analysis.

Implements ConcordModel trait (MockModel + LadderModel stub), steelman engine
with citation-integrity and anti-caricature guards, and `concord steelman`
subcommand. Restructures workspace: binary + acceptance tests moved to new
concord-bin crate to break dep cycle. All 40 tests green, no network required.

## v0.1.0 — concord-corpus

Initial release: perspective-diverse source corpus builder with FixtureGatherer,
stance tagger, Jaccard dedup, and credibility scorer.
