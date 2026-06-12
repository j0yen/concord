# Changelog

## v0.6.0 — 2026-06-12

concord-cruxes: crux separation engine — separates steelmanned positions into shared values, genuine cruxes (empirical/value-tagged with change-minds fields), and terminological misunderstandings. DisagreementMap schema with JSON roundtrip, MockModel-based offline tests, 70%+ accuracy on independently authored held-out fixture (AC3). CLI subcommands: cruxes analyze / cruxes show. All 22 tests green (15 unit + 7 integration). AC1-AC5 met; AC6 (live LLM) deferred.

## v0.5.0 — 2026-06-12

concord-bridge: balanced brief synthesizer capstone; all 6 automated ACs pass (build+test green, balance gate, citation integrity, coverage clause, end-to-end pipeline, no-network); AC7 deferred (live LLM evaluation)

## v0.4.0 — 2026-06-06

Add concord-deescalate: de-escalation engine that rephrases heated messages into
NVC (observation/feeling/need/request) form while preserving every substantive
ask. Includes contempt lexicon (deterministic, extensible), rule-based +
model-assisted ask extraction, ask-preservation post-check, safety boundary
(refuses threats/harassment), and `concord deescalate` CLI subcommand with
--explain mode. All ACs 1-6 met; AC7 deferred (live model). 27 tests green,
cloud-build-safe (MockModel only).

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
