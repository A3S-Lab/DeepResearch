# Changelog

All notable changes to A3S DeepResearch are documented in this file.

The project follows Semantic Versioning while the public API remains in the
`0.x` development series.

## [Unreleased]

### Added

- Add a global, byte-bounded source-attribution review over each changed
  selected portfolio when declared independence otherwise appears complete,
  then preserve or refresh that review for the final portfolio. Same-origin
  findings can now drive a bounded supplemental retrieval pass without
  repeating the review for an unchanged portfolio. The workflow returns a
  complete accountable-origin partition plus only the source-group pairs whose
  independent attribution is positively established. The Host revalidates
  every exact source ID and closes groups again after canonical report-source
  coalescing.

### Changed

- Count independent corroboration and comprehensive cross-source synthesis by
  verified attribution groups instead of distinct source aliases. Mirrors,
  syndication, translations, derivative copies, malformed partitions, and
  unknown group relationships now retain their evidence but fail closed for
  independent-depth admission.
- Require each resolved comprehensive material dimension to contain at least
  1,200 Unicode letter-or-number characters in admitted claim prose and an
  implication that integrates both comparison and explanation ancestry.

## [0.1.3] - 2026-07-26

### Added

- Add `DeepResearchRequest`, `EvidenceScope`, `WorkspaceSourceHint`,
  `DeepResearchResult`, `PublicationOutcome`, and `DeepResearchEvent` as the
  typed product-integration contract.
- Add runtime-agnostic cooperative cancellation and terminal cancelled/failed
  lifecycle events.
- Add run-scoped report materializers under
  `.a3s/research/artifacts/<run-id>/` with exact receipt recovery.
- Add request-owned output-language inference and override, strict
  `report_language` schemas, aggregate prose-language admission, and localized
  Chinese source-backed/no-evidence publication.
- Add persisted analytical-claim and cross-source-synthesis quality metrics in
  version-3 publication receipts, bind version-4 receipts to the request-owned
  output language, and persist resolved/deeply-analyzed material-dimension
  counts in version-5 receipts.
- Promote at most three explicit HTTP(S) references from the user query to
  direct retrieval seeds, with fragment removal, credential rejection,
  deduplication, and a network-free `local_only` contract.

### Changed

- Route explicit workspace source hints through exact Host reads and the normal
  provenance boundary, while keeping the raw `execute(Value)` API available for
  legacy adapters.
- Reserve query-slug artifacts for read-only compatibility so concurrent
  same-query runs can publish without overwriting each other.
- Normalize typed result diagnostics to the four publication outcomes and
  artifact kinds without duplicating filesystem paths in the JSON output.
- Replace the plain report link row with a sticky left action menu, a centered
  report surface, and a sticky right table of contents. Narrow screens stack
  both navigation regions ahead of the report without horizontal overflow.
- Require every resolved material dimension in a comprehensive typed report to
  contain a direct answer, two factual findings supported across two sources,
  two inferences, three analytical claims, one cross-source synthesis, and 800
  substantive characters, with a 1,200-character report-wide floor. The prompt
  separates source comparison, mechanism or trade-off, implication, and
  applicability boundary instead of accepting one catch-all synthesis
  paragraph.
- Add an evidence-preserving narrative plan that can choose natural section
  headings and group adjacent claim IDs into paragraphs but cannot add,
  paraphrase, reorder, or omit admitted findings.
- Render the authored argument as continuous prose without fixed
  evidence/analysis/recommendation subheadings, and move basis edges and
  derivations into a collapsed traceability disclosure.
- Reject repeated claim openings and near-duplicate claim prose so length and
  claim-count gates cannot be satisfied with editorial padding.
- Render typed, source-backed, qualified, and degraded HTML through one fixed
  A3S Code Web-aligned visual system. Legacy presentation metadata remains
  readable for compatibility but no longer changes report styling or layout.
- Allow source-native supplemental retrieval queries when they improve recall
  while keeping planner prose and final publication pinned to the user's output
  language.
- Plan a central shared evidence track for comprehensive named comparisons,
  allow distinct atomic criteria to reuse the same question role, and validate
  the required comprehensive role mix once across the complete plan.
- Allow an all-bounded comprehensive report to publish as `Qualified` only when
  exactly one bounded dimension contains the full two-source analytical chain,
  at least 1,200 substantive characters, exactly one partial conclusion, and an
  explicit typed gap. Shallow inventories and one-step comparisons still fail
  closed to the source-backed artifact.

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
[0.1.3]: https://github.com/A3S-Lab/DeepResearch/compare/v0.1.2...v0.1.3
[Unreleased]: https://github.com/A3S-Lab/DeepResearch/compare/v0.1.3...HEAD
