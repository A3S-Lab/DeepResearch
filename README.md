# A3S DeepResearch

<p align="center">
  <strong>Evidence-First Research Engine for A3S</strong>
</p>

<p align="center">
  <em>Build bounded research workflows with typed evidence, explicit quality gates, and crash-safe publication</em>
</p>

<p align="center">
  <a href="#overview">Overview</a> •
  <a href="#features">Features</a> •
  <a href="#quick-start">Quick Start</a> •
  <a href="#engine-model">Engine Model</a> •
  <a href="#evidence-contract">Evidence Contract</a> •
  <a href="#publication-and-recovery">Publication and Recovery</a> •
  <a href="#architecture">Architecture</a> •
  <a href="#development">Development</a>
</p>

---

## Overview

**A3S DeepResearch** is the standalone research engine used by A3S products.
It owns reusable planning, retrieval orchestration, evidence admission, report
quality gates, and Markdown/HTML publication without depending on the A3S CLI,
TUI, or web application.

DeepResearch is domain-agnostic. It does not route research by topic,
named-entity dictionaries, query keywords, language, publisher, domain, URL
shape, or error prose. Semantic model calls make bounded content judgments;
deterministic code validates closed schemas, exact identities, budgets,
provenance edges, artifact hashes, and durable state.

### Basic usage

The host supplies four asynchronous ports and runs the same engine from any
product surface:

```rust,no_run
use a3s_deep_research::{
    engine::{
        DeepResearchCancellation, DeepResearchEngine, DeepResearchRequest,
        EngineLimits, EvidenceScope, ProgressPort, PublicationPort,
        StructuredGenerationPort, WorkflowExecutionPort, WorkspaceSourceHint,
    },
};

async fn research(
    generation: &dyn StructuredGenerationPort,
    workflow: &dyn WorkflowExecutionPort,
    publication: &dyn PublicationPort,
    progress: &dyn ProgressPort,
    query: &str,
    current_date: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = DeepResearchRequest::new(
        "product-run-id",
        query,
        EvidenceScope::WebAndWorkspace,
    )
    .with_current_date(current_date)
    .with_workspace_source_hints(vec![
        WorkspaceSourceHint::new("docs/product-context.md"),
    ]);
    let cancellation = DeepResearchCancellation::new();

    let result = DeepResearchEngine::new(generation, workflow, publication, progress)
        .with_limits(EngineLimits::default())
        .execute_request(request, cancellation)
        .await?;

    println!("{}", result.artifacts.html.display());
    Ok(())
}
```

`DeepResearchResult` keeps the completed lifecycle separate from the terminal
`PublicationOutcome`, and returns typed quality metrics plus exact artifact
paths. `execute(Value)` and `DeepResearchRun` remain available as a legacy
adapter while products migrate away from constructing or parsing engine JSON.
The typed result's diagnostic `output` uses the same four publication names and
exposes only `artifact_kinds`; exact filesystem paths remain confined to the
typed `artifacts` field.

`DeepResearchRequest::new` infers the reader language from the query. A product
with an explicit locale selection may replace it with
`with_output_language("zh")` or another validated BCP 47 tag.

## Features

- **Exact-Query Bootstrap**: Start retrieval from the unmodified user query
  while semantic planning runs concurrently, and promote at most three explicit
  HTTP(S) references in that query to direct retrieval seeds
- **Bounded Semantic Planning**: Decompose at most 24 atomic user requirements,
  map every requirement to one or more of at most eight material research
  tracks, and add at most 15 supplemental plain-text queries without replacing
  the original query; unmatched, duplicate, or invented mappings fail closed
- **Closed Evidence Admission**: Admit only exact source and chunk identities
  selected through typed track, criterion, and source-role edges
- **Typed Claim Graphs**: Distinguish facts, inferences, and recommendations,
  with explicit citations, basis edges, derivations, contradictions, and gaps
- **Reader-Language Pinning**: Infer or accept one BCP 47 output language and
  bind planning, report schema, admission, and publication to that exact value
- **Per-Dimension Depth Gate**: Require every resolved material dimension in a
  comprehensive report to contain a direct answer, two factual findings from
  distinct sources, a multi-step analytical chain, and an explicit implication
  or applicability boundary. Only complete material coverage can become a
  successful `Synthesized` result; any `Qualified` graph remains an explicitly
  incomplete preview
- **Independent Commercial Review**: After graph admission, require a separate
  editorial generation to review every mapped requirement, dimension, and
  claim for support, depth, temporal status, scope, language, and source-summary
  prose before a synthesized draft can remain successful
- **Evidence-Preserving Editorial Planning**: Let the independent editor improve
  claim phrasing, natural section headings, and paragraph groupings only after
  the claim graph is complete; the Host verifies exact claim identity, numeric
  preservation, dependency order, and one-time placement of every finding
- **Continuous Argument Rendering**: Render claims in authored paragraph order
  without fixed evidence/analysis/recommendation subheadings, while keeping
  basis edges and reproducible derivations available in a collapsed
  traceability disclosure
- **Fixed A3S Report Design**: Render every HTML publication outcome through
  one A3S Code Web-aligned token system; legacy presentation metadata cannot
  change the palette, hero, density, typography, or section composition. The
  shared renderer uses a sticky left action menu, a centered report surface,
  and a sticky right table of contents on desktop; narrow screens stack the
  action menu and table of contents ahead of the report without page overflow
- **Progressive Publication**: Preserve a source-backed report before attempting
  a typed claim graph and independent editorial review
- **Explicit Evidence Boundaries**: Publish a no-evidence report when no source
  can safely support conclusions
- **Crash-Safe Artifacts**: Replace Markdown and HTML as one digest-verified,
  recoverable generation
- **Run-Scoped Recovery**: Bind terminal publication to the exact run, query,
  quality metrics, and full artifact hashes
- **Typed Lifecycle Events**: Emit run, stage, publication, cancellation, and
  failure events without inferring state from report prose
- **Cooperative Cancellation**: Drop in-flight port futures and prevent later
  stages from publishing after cancellation
- **Explicit Workspace Hints**: Acquire validated relative context paths through
  the same exact-path provenance boundary as discovered workspace evidence
- **Replaceable Product Ports**: Keep model execution, workflow runtime,
  artifact storage, provider policy, and progress presentation in the host
- **Replayable Contracts**: Exercise production control flow with frozen,
  domain-neutral fixtures and typed fault injection

### Publication outcomes

Every completed run returns one closed publication outcome.

| Outcome | Admission state | Reader-visible result |
| --- | --- | --- |
| `Synthesized` | Admitted claims close every material dimension | Typed report with validated claims, relations, derivations, and citations |
| `Qualified` | Useful admitted claims remain, with an explicit material gap | Typed report that preserves both supported findings and the unresolved gap |
| `SourceBacked` | Sources are semantically admitted, but final synthesis is unavailable or rejected | Staged source report with no synthesized claim graph |
| `NoEvidence` | No source passes semantic admission | Boundary report that makes no domain conclusion |

Raw acquisition may be retained for audit and recovery, but it never becomes
claim evidence merely because a provider returned it.

## Quick Start

### Installation

```toml
[dependencies]
a3s-deep-research = "0.1.3"
serde_json = "1"
```

The engine uses async ports. The host chooses its executor, model client,
workflow runtime, search providers, persistence, and presentation layer.

### Implement the product ports

| Port | Host responsibility |
| --- | --- |
| `StructuredGenerationPort` | Execute the closed planning and report-object contracts |
| `WorkflowExecutionPort` | Run bootstrap and planned retrieval workflows |
| `PublicationPort` | Materialize the requested report class and return exact artifact paths |
| `ProgressPort` | Persist or present typed run, stage, publication, cancellation, and failure events |

All four traits are `Send + Sync`. `NoopProgress` is available when the host
does not need progress events. Existing implementations of `report_progress`
continue to receive stage events; new adapters should implement `report_event`
to preserve the complete lifecycle.

### Configure execution limits

`EngineLimits::default()` supplies bounded planner, retrieval, report, retry,
and durable-generation grace periods. A host may replace those values with
`with_limits`, but invalid zero-length or unbounded generation settings fail
contract validation before research starts.

Search-engine selection and provider fallback are host runtime policy. A
provider outage or exhausted quota can trigger another provider without
changing the evidence contract: fallback bytes still require the same semantic
admission as bytes from the preferred provider.

`WorkflowExecutionPort` adapters may attach a bounded
`RetrievalRunProvenanceEnvelopeV1` under the reserved top-level
`WorkflowOutput.metadata["retrieval_run_provenance"]` key. The adapter must
first validate each producer receipt against its returned output and retain the
complete receipt plus the verification material required by that producer. The
`receipt_sha256` field records the producer-defined canonical complete-receipt
binding; it is not required to hash the receipt's JSON representation. For a
Search cascade, map `outcome.receipt_binding()?.sha256` to
`receipt_sha256`, the typed `SearchQuery` binding to `request_sha256`, and the
complete ordered `SearchResults` binding to `output_sha256`. The quality floor
and declared tier plan are covered by the complete receipt binding, not by the
SearchQuery binding. DeepResearch does not depend on the Search crate or
inspect tier/provider names. The engine shape-validates and carries this
adapter assertion in terminal
`execution.retrieval_run_provenance` audit metadata. The top-level key is
host-reserved by convention, not a cryptographically separate channel; nested
tool metadata is ignored, and an invalid optional envelope is recorded as
rejected.

Retrieval provenance is deliberately excluded from planning, evidence
admission, report generation, and publication-quality calculations. A
shape-valid envelope states only which retrieval output the adapter says it
validated. It does not make a source relevant, fetched, independent, primary,
claim-eligible, or sufficient for publication, and it does not establish
external authenticity unless an independent authority authenticates the
producer-defined complete-receipt binding, for example by signing or
append-only logging `receipt_sha256`. A failed `WorkflowExecutionPort` call
returns no `WorkflowOutput`, so durable provenance for failed attempts must
remain in the adapter's external append-only attempt log rather than this
optional successful-output channel.

## Engine Model

Bootstrap retrieval and semantic planning begin together. Planning may define
up to 24 atomic request requirements, map them to at most eight evidence tracks,
and return at most 15 supplemental queries. It cannot return URLs, choose
transport budgets, replace the exact query, or publish facts. If planning is
invalid or unavailable, the engine continues from the exact query under a
conservative comprehensive, freshness-aware gate.

For web-capable evidence scopes, the Host also extracts at most three explicit
HTTP(S) references from the exact user query as direct retrieval seeds. It
removes fragments, rejects credential-bearing or malformed references, and
deduplicates normalized URLs. `local_only` scope never emits these seeds and
remains network-free.

```text
exact user query ──> bootstrap retrieval ───────────────┐
                                                       │
semantic outline ──> bounded supplemental queries      │
        │                                              │
        └─ invalid or unavailable ─> exact-query fallback
                                                       │
                                                       v
                                      raw acquisition packet
                                            (audit-only)
                                                       │
                                  closed semantic source selection
                                                       │
                                  Host inquiry projection of exact
                                  source/chunk IDs, criteria, and roles
                                                       │
                                  up to four typed-gap retrieval rounds
                                                       │
                         ┌─────────────┴─────────────┐
                         │                           │
               no admitted evidence       semantically admitted evidence
                         │                           │
               no-evidence artifact      source-backed artifact
                                                     │
                                      optional typed claim graph
                                                     │
                                  deterministic graph admission
                                                     │
                                  independent commercial review
                                                     │
                           synthesized / qualified / source-backed
```

Every selected web candidate is a separate durable workflow effect. Sibling
steps are recorded before external effects begin, successful outputs are
persisted independently, and aggregation uses candidate indexes rather than
completion order. A completed source can therefore be reused after restart,
while an ambiguous running effect is redelivered with the same attempt under
at-least-once semantics.

When typed coverage remains incomplete, up to four gap-directed rounds may use
unused discovery results, explicit HTTPS references from admitted evidence, and
new queries generated only from typed missing criteria or source roles. Each
unresolved track is expanded into one ordered target per missing atomic
criterion; role-only targets enter after their criteria are otherwise covered.
The Host contract derives a maximum of 24 new queries from the eight-track,
three-criteria envelope and permits at most 16 supplemental fetches in total.
The workflow rotates those targets fairly across up to four rounds, reserves
capacity for the later round, subtracts the first round's actual scheduled
effects, and never owns a duplicate numeric budget. Every fetched reference
passes the same closed semantic evidence selection.

Replay decisions use only exact run, stage, step, and candidate identities plus
typed state. Retrieval retries are equally closed: only object-shaped error
metadata whose exact `type` is `timeout` or `transport` authorizes the one
bounded fetch retry. Missing, malformed, string-valued, or unknown error kinds
are terminal at this layer regardless of their prose.

## Evidence Contract

The Host-projected inquiry collection is the sole semantic admission authority.
Web and workspace sources both enter through exact selected source/chunk IDs
and typed relationships:

- track relevance admits a source for an atomic claim on that exact track;
- completion-criterion coverage records which declared obligation a source
  satisfies;
- criterion-scoped primary and independent roles record the required source
  shape;
- facts cite admitted source and chunk IDs;
- inferences and recommendations name admitted basis claims;
- derived claims retain their method and exact input claim IDs;
- contradiction relations preserve both supported claims;
- typed gaps bind unresolved obligations to their material dimensions.

Track relevance never closes a completion criterion, and criterion coverage
never manufactures a claim. A focused report may publish one structurally
sufficient cited claim. A comprehensive report reaches `Synthesized` only when
its admitted graph closes every material dimension. A report with both deeply
analyzed resolved dimensions and unresolved dimensions may remain `Qualified`
with explicit typed gaps, but `Qualified` is never a successful completed
research result. When every dimension is unresolved, the Host admits at most
one qualified partial preview, and only if that bounded dimension independently
satisfies the full two-source analytical depth gate and retains its gap.

The typed comprehensive gate retains the report-wide floor of one direct
answer, five supporting findings, six admitted claims, two cited sources, and
1,200 non-whitespace claim characters. It also evaluates every resolved material
dimension independently. Each such dimension needs exactly one leading answer,
at least two factual findings whose support spans two attributable sources, at
least two inferences, at least three analytical claims in total, one
cross-source synthesis, and 800 substantive characters. The generation
contract asks those analytical steps to cover source comparison, mechanism or
trade-off, practical implication, and an applicability boundary or
counterexample. Repeated openings and near-duplicate claim prose are rejected;
multi-cited facts, three paraphrases of one conclusion, and source-by-source
summaries do not satisfy this requirement.

The engine persists both the number of resolved material dimensions and the
number that passed this deeper gate. A synthesized report is valid only when
those counts are equal and every material requirement is resolved. A qualified
artifact may preserve supported partial work, but its nonzero gap count keeps
it incomplete at every CLI, TUI, and Web success boundary. The all-bounded
preview is represented explicitly as zero resolved dimensions, one deeply
analyzed bounded dimension, exactly one direct answer, and at least one typed
gap. Restart recovery therefore cannot promote an older report whose aggregate
metrics hide a thin section.

After the graph is complete, an independent editorial generation receives the
exact query, current date, mapped requirements, admitted claims, and only the
closed evidence excerpts cited by those claims. It must review every dimension
and every claim, classify fact temporal status, and return evidence-preserving
rewrites plus a bounded narrative plan. The Host rejects missing or duplicate
reviews, inconsistent readiness, unsupported temporal classes, numeric drift,
changed claim identities, dependency inversions, or shallow/source-summary
dimensions. A synthesized draft that fails this review falls back to the staged
source-backed artifact instead of becoming successful. Renderers project an
admitted plan as continuous prose while retaining basis edges and derivations
in a collapsed traceability disclosure.

An inference or recommendation may combine admitted premises from other
dimensions through explicit basis edges. Those edges do not manufacture
criterion coverage. After claim admission, the Host recomputes coverage from
the sources actually used by the accepted claims and adds a typed gap when that
smaller evidence set no longer satisfies a declared source requirement.

Small source catalogs use one closed selector. Larger catalogs are partitioned
into complete source-local JSON windows of at most 32 KiB, followed by an
exact-ID reduction that retains at most four excerpts per source. The current
report attempt receives a generated JSON Schema whose dimension, source, and
chunk enums contain only that closed packet.

Reader prose is untrusted data. The Host never derives identity, relevance, or
evidence support from query, claim, source, title, URL, publisher, numbers, or
punctuation. It does pin `report_language` to the request-owned output language
and rejects obvious aggregate prose-language mismatches. Source-defined names
and quotations may remain in their original language. Reader-facing labels and
evidence-boundary prose arrive inside the typed proposal, are shape-validated,
and are rendered as inert text.

The planner writes titles, tracks, criteria, and the final report in the pinned
reader language. Supplemental retrieval queries may use a source's native
language when that materially improves recall; this never changes the report
language.

## Publication and Recovery

Publication is progressive. Once evidence is semantically admitted, the engine
stages a source-backed Markdown/HTML pair before report generation. A failed,
timed-out, invalid, or commercially rejected synthesized proposal cannot erase
it. If synthesized publication
returns an ambiguous error after touching the pair, the engine resolves the
exact run receipt and otherwise re-publishes the closed source snapshot.

Markdown and HTML are installed as one crash-recoverable generation. The
publisher syncs the new files and bounded copies of the previous pair before
installing a transaction journal containing exact local file identities and
full SHA-256 digests. Recovery keeps the new generation only when both target
digests match; otherwise it restores the complete previous pair. An interrupted
first publication is removed instead of exposing mismatched formats.

A successful publication also persists a run-scoped receipt before the port
returns. New product adapters write the report pair under
`.a3s/research/artifacts/<run-id>/`; legacy query-slug reports remain readable
but are no longer the target for new runs. Receipt schema version 5 binds:

- hashes of the exact run identity and query;
- the request-owned output language;
- the closed publication outcome;
- claim, relation, derivation, basis-edge, analytical-claim, cross-source
  synthesis, resolved-dimension, deeply-analyzed-dimension, gap, citation, and
  source metrics;
- the complete Markdown and HTML SHA-256 digests.

Version 1 and 2 receipts remain readable with missing metrics defaulted to
zero, and version 3 remains readable without a language binding. Version 4 has
the language binding but predates per-dimension depth metrics. These receipts
cannot authorize `Qualified` without an explicit gap, and an older
comprehensive receipt cannot pass the current per-dimension depth gate. The
compatibility recovery API may read legacy receipts, while language-bound
recovery treats versions without a language as absent. New recovery paths keep
the version-4 language binding and add the version-5 depth metrics;
`resolve_deep_research_run_publication_in_language` additionally requires a
workflow envelope to carry the same language. If a workflow envelope and
receipt both exist, their outcome, metrics, language, and artifact paths must
agree exactly.

Interrupted acquisition can be retained separately with
`materialize_deep_research_acquisition_recovery_report`. That artifact accepts
only a raw, non-admitted source catalog, uses the exact run identity in its
location, and marks every source ineligible for conclusions. It cannot
overwrite or upgrade a completed report.

## Architecture

The engine owns research state transitions while products own external effects:

| Layer | Owns |
| --- | --- |
| `DeepResearchEngine` | Stage ordering, execution limits, bounded degradation, progressive publication, and terminal metadata |
| Planner | Domain-neutral scope, dimensions, evidence tracks, completion criteria, and supplemental queries |
| Retrieval workflows | Search/fetch orchestration, closed source selection, durable acquisition, and bounded materialization |
| Report pipeline | Source catalog, typed claim graph admission, quality gates, citations, Markdown, and HTML |
| Product adapter | Model calls, workflow tools, provider fallback, artifact storage, progress, and user interaction |

```text
product adapter
  ├── StructuredGenerationPort ──> model runtime
  ├── WorkflowExecutionPort ─────> search / fetch / workspace tools
  ├── PublicationPort ───────────> artifact store
  └── ProgressPort ──────────────> CLI / TUI / web presentation
                │
                v
       DeepResearchEngine
                │
      planner + retrieval contracts
                │
       typed evidence compiler
                │
    quality gate + durable publication
```

The repository follows the same ownership boundaries:

```text
src/
├── engine/       # Port-based asynchronous orchestration
├── planner/      # Domain-neutral planning contracts and validation
├── report/       # Evidence admission, quality gates, and publication
├── research/     # Replayable research state machine
└── workflow/     # Embedded retrieval and generation workflow assets
```

## Development

Run checks from the `a3s-deep-research` crate directory:

```bash
cargo fmt --all -- --check
cargo test --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo package --locked
```

### Commercial holdout release gate

The public F01-F08 replay corpus and C01-C09 live manifest are development
evidence, never stable-release evidence. Stable release evidence is a separately
operated campaign over at least 48 sealed cases, with exactly three preallocated
attempts per case and no replacement runs.

The campaign has two signed stages. Before execution, a corpus custodian signs
a plan that binds the exact commit, package bytes, lockfile, sealed corpus,
generic sampling policy, run slots, execution profile, runner and verifier
images, baseline, rubric, blind assignment, and quality thresholds. Its payload
digest and transparency-log receipt must be committed outside this repository
before the first attempt starts. After execution, an execution authority and an
independent review authority sign the same result statement with distinct
Ed25519 trust roots. The result binds every start and terminal receipt, the
append-only attempt log, all report and audit artifacts, the blind ballots, a
semantic development-overlap audit, and the corpus-retirement receipt.
Schema v2 carries each preallocated attempt instead of self-reported case
aggregates. The verifier recomputes the exact slot set, receipt and artifact
roots, completion, citations, hard violations, language parity, blind ratings,
abstention, infrastructure failure, and paired-baseline statistics from those
records.

The repository-local verifier is defense in depth, not the certification
authority. The stable release workflow currently fails closed before publishing
and does not expose holdout material to candidate-controlled code. It can be
unblocked only after the candidate-independent verifier tracked in
`A3S-Lab/DeepResearch#2` verifies the frozen `.crate` and returns an attestation
bound to the exact subject and raw evidence bundle. Prerelease tags are marked
as prereleases and do not claim commercial certification.

Repository code is only defense in depth: a tag executes the workflow stored in
the tagged commit, and an administrator can otherwise create a GitHub Release
manually. Commercial certification therefore also requires organization-level
`v*` tag rules, immutable releases, restricted publish identity, protected
environment reviewers, and a required verifier operated from A3S Bench or
another candidate-independent system. Until those controls exist, a GitHub
Release is not commercial certification even when its local workflow passes.

The fixed policy floor requires all of the following:

- at least 90% `Synthesized` attempts among answerable cases; `Qualified`
  remains incomplete;
- at least 90% commercial-success attempts, at least 95% successful cases, a
  one-sided 95% Wilson lower bound of at least 90%, and no zero-success case;
- zero unsupported material claims, citation-integrity violations,
  reader-boundary violations, reader-language mismatches, or artifact-parity
  violations, with 100% material-claim audit coverage;
- at least 95% citation recall and precision on each commercial success and in
  the aggregate, plus blind depth, naturalness, evidence-use, and decision-value
  scores of at least 4 out of 5;
- at least six intentionally unanswerable cases and at least 90% safe
  abstention, so the gate does not reward hallucination merely to reduce
  `NoEvidence` outcomes;
- paired baseline non-inferiority and bounded infrastructure failure, with
  statistics aggregated by case rather than treating three repeated attempts
  as independent tasks.

Sampling coverage is expressed only through domain-neutral strata such as task
intent, evidence condition, freshness, source mix, answerability, and language
group. Product code contains no evaluation topic, entity, publisher, site, or
language special case. Once failure details are exposed, that sealed corpus is
retired rather than tuned against.

The repository-side verifier checks signatures, bindings, policy floors, and
case-level statistics. The independent authorities remain responsible for
recomputing the signed result from the raw artifact bundle and ballots; signing
unverified summary fields is not valid release evidence. Passing implementation
tests or a hand-picked demonstration is never a substitute for this campaign.

The test suite covers cross-topic structural isomorphism, exact-query
authority, domain-neutral planning, closed source selection, typed provenance,
report admission, artifact transaction faults, receipt recovery, and embedded
JavaScript workflow execution.

Frozen F01-F08 fixtures pass through `DeepResearchEngine::execute` using the
same production contracts. Their control flow varies only closed protocol
enums, exact identities, explicit graph edges, and injected typed failures.
The publication fault suite interrupts both report-file replacements and
verifies rollback, commit recognition, first-generation cleanup, and recovery
through the normal report resolver.

## License

MIT
