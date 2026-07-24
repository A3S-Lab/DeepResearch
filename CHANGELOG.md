# Changelog

All notable changes to A3S DeepResearch are documented in this file.

The project follows Semantic Versioning while the public API remains in the
`0.x` development series.

## [Unreleased]

## [0.1.2] - 2026-07-24

### Added

- Replay the complete frozen F01-F08 corpus through
  `DeepResearchEngine::execute`, including typed single-target extraction
  failure and report-generation timeout injection.
- Make `deep_research_typed_claim_graph` the active report-generation protocol.
  It carries fact, inference, and recommendation claims; exact source/chunk
  support; basis edges; reproducible derivations; contradiction relations; and
  typed dimension gaps through the production engine.
- Add the `Qualified` publication outcome for useful admitted claims with a
  remaining material gap, including publication receipts and product-adapter
  recovery.

### Changed

- Allow a focused report to publish one structurally sufficient cited claim
  instead of requiring an unrelated extra finding.
- Persist accepted relation, derivation, basis-edge, and gap counts through the
  engine quality envelope, version-2 publication receipt, and product journal.
  Version-1 receipts remain readable, but cannot manufacture a qualified
  outcome without an explicit typed gap.
- The active F01-F08 replay now produces synthesized reports for F01, F02, F04,
  F05, F07, and F08; a qualified report for F03; and the intended source-backed
  timeout result for F06.

### Fixed

- Resolve an ambiguous publication-port failure from the exact run-scoped
  receipt when the report pair and receipt were already committed. A valid
  workflow publication and receipt must agree exactly; invalid result prose
  cannot discard a committed publication.
- Preserve the staged source-backed report when final synthesized publication
  fails. The engine re-publishes the closed source snapshot so the report pair,
  publication status, quality metrics, and durable receipt cannot describe
  different artifact generations.
- Align the executable discovery smoke fixture with the exact current batch
  envelope and verify that unstructured search output fails closed.
- Verify every frozen-corpus source path and SHA-256 digest so evaluation
  snapshots cannot drift while the replay suite remains green.
- Remove the TUI adapter's duplicate nonzero-finding requirement so a valid
  focused direct-answer claim reaches terminal settlement unchanged.
- Isolate malformed bootstrap and planned retrieval envelopes by their closed
  stage and packet-version contracts. A valid planned sibling still reaches
  synthesis, while raw bootstrap bytes remain audit-only and cannot be
  promoted into evidence after planned-envelope failure.

## [0.1.1] - 2026-07-23

### Changed

- Treat raw web and workspace acquisition as audit-only; only the Host-projected
  inquiry collection can grant semantic source admission.
- Preserve structurally valid source query parameters without parameter-name
  allowlists or provider-specific URL rewriting.
- Require reader-facing compiler labels and boundary prose in the closed
  research contract instead of selecting templates from a language code.
- Identify degraded artifact classes with explicit versioned markers rather
  than matching English or Chinese report sentences.

### Fixed

- Remove stale calls to the deleted `lowValueUrl` classifier that caused web
  discovery to fail at runtime while still passing JavaScript syntax checks.
- Add an executable discovery-workflow smoke test for structured search output
  and fail-closed unstructured output.
- Remove publisher/domain authority inference, fetched-text noise vocabulary,
  language/script branching, token-overlap deduplication, and topic-specific
  source transformations from production admission paths.

## [0.1.0] - 2026-07-23

### Added

- Standalone, port-based DeepResearch orchestration for A3S products.
- Exact-query bootstrap retrieval with bounded semantic planning and one
  typed-coverage supplemental pass.
- Progressive source-backed, synthesized, and no-evidence publication modes.
- Closed report proposals with host-owned citations and research-track IDs.
- Typed source coverage for completion criteria, primary sources, and
  independent corroboration.
- Domain-neutral frozen replay fixtures and publication-quality gates.

### Changed

- Web discovery fallback remains audit-only unless a closed semantic evidence
  pass admits the source.
- Comprehensive reports must cover every material research track and all
  declared completion criteria.
- Report findings are grouped by the planner's semantic research tracks.

### Fixed

- Preserve final inquiry-selection provenance instead of falling back to stale
  bootstrap acquisition metadata.
- Decode the durable string-array source-role format emitted by the retrieval
  workflow so real coverage edges reach report admission.
- Prevent one source from satisfying both primary-source and independent-source
  requirements without a distinct corroborating source.
- Remove publisher allowlists, domain-name authority inference, query-token
  overlap admission, and topic-specific source routing.

[0.1.0]: https://github.com/A3S-Lab/DeepResearch/releases/tag/v0.1.0
[0.1.1]: https://github.com/A3S-Lab/DeepResearch/compare/v0.1.0...v0.1.1
[0.1.2]: https://github.com/A3S-Lab/DeepResearch/compare/v0.1.1...v0.1.2
[Unreleased]: https://github.com/A3S-Lab/DeepResearch/compare/v0.1.2...HEAD
