# Changelog

All notable changes to A3S DeepResearch are documented in this file.

The project follows Semantic Versioning while the public API remains in the
`0.x` development series.

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
- Add an executable discovery-workflow smoke test for both structured search
  output and plain-URL fallback output.
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
