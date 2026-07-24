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
        DeepResearchEngine, EngineLimits, ProgressPort, PublicationPort,
        StructuredGenerationPort, WorkflowExecutionPort,
    },
    planner::deep_research_loop_contract,
};

async fn research(
    generation: &dyn StructuredGenerationPort,
    workflow: &dyn WorkflowExecutionPort,
    publication: &dyn PublicationPort,
    progress: &dyn ProgressPort,
    query: &str,
    current_date: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let evidence_scope = "web_and_workspace";
    let input = serde_json::json!({
        "run_id": "product-run-id",
        "input": {
            "query": query,
            "current_date": current_date,
            "evidence_scope": evidence_scope,
            "loop_contract": deep_research_loop_contract(
                query,
                current_date,
                evidence_scope,
                4,
            )
        }
    });

    let run = DeepResearchEngine::new(generation, workflow, publication, progress)
        .with_limits(EngineLimits::default())
        .execute(input)
        .await?;

    println!("{}", run.artifacts.html.display());
    Ok(())
}
```

`DeepResearchRun` returns the terminal publication class, artifact paths, and a
structured output that a host can expose through its existing result surface.

## Features

- **Exact-Query Bootstrap**: Start retrieval from the unmodified user query
  while semantic planning runs concurrently
- **Bounded Semantic Planning**: Add validated research tracks and at most three
  supplemental plain-text queries without replacing the original query
- **Closed Evidence Admission**: Admit only exact source and chunk identities
  selected through typed track, criterion, and source-role edges
- **Typed Claim Graphs**: Distinguish facts, inferences, and recommendations,
  with explicit citations, basis edges, derivations, contradictions, and gaps
- **Progressive Publication**: Preserve a source-backed report before attempting
  synthesized or qualified publication
- **Explicit Evidence Boundaries**: Publish a no-evidence report when no source
  can safely support conclusions
- **Crash-Safe Artifacts**: Replace Markdown and HTML as one digest-verified,
  recoverable generation
- **Run-Scoped Recovery**: Bind terminal publication to the exact run, query,
  quality metrics, and full artifact hashes
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
a3s-deep-research = "0.1.2"
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
| `ProgressPort` | Present product-specific stage progress and degradation events |

All four traits are `Send + Sync`. `NoopProgress` is available when the host
does not need progress events.

### Configure execution limits

`EngineLimits::default()` supplies bounded planner, retrieval, report, retry,
and durable-generation grace periods. A host may replace those values with
`with_limits`, but invalid zero-length or unbounded generation settings fail
contract validation before research starts.

Search-engine selection and provider fallback are host runtime policy. A
provider outage or exhausted quota can trigger another provider without
changing the evidence contract: fallback bytes still require the same semantic
admission as bytes from the preferred provider.

## Engine Model

Bootstrap retrieval and semantic planning begin together. Planning may define
evidence tracks, completion criteria, and bounded supplemental queries. It
cannot return URLs, choose transport budgets, replace the exact query, or
publish facts. If planning is invalid or unavailable, the engine continues from
the exact query under a conservative comprehensive, freshness-aware gate.

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
                           synthesized / qualified / source-backed
```

Every selected web candidate is a separate durable workflow effect. Sibling
steps are recorded before external effects begin, successful outputs are
persisted independently, and aggregation uses candidate indexes rather than
completion order. A completed source can therefore be reused after restart,
while an ambiguous running effect is redelivered with the same attempt under
at-least-once semantics.

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
its admitted graph closes every material dimension; otherwise useful findings
remain `Qualified` with an explicit gap.

Small source catalogs use one closed selector. Larger catalogs are partitioned
into complete source-local JSON windows of at most 32 KiB, followed by an
exact-ID reduction that retains at most four excerpts per source. The current
report attempt receives a generated JSON Schema whose dimension, source, and
chunk enums contain only that closed packet.

Reader prose is untrusted data. The Host does not compare query, claim, source,
title, URL, publisher, language, numbers, or punctuation to make admission
decisions. Reader-facing labels and evidence-boundary prose arrive inside the
typed proposal, are shape-validated, and are rendered as inert text.

## Publication and Recovery

Publication is progressive. Once evidence is semantically admitted, the engine
stages a source-backed Markdown/HTML pair before report generation. A failed,
timed-out, or invalid proposal cannot erase it. If synthesized publication
returns an ambiguous error after touching the pair, the engine resolves the
exact run receipt and otherwise re-publishes the closed source snapshot.

Markdown and HTML are installed as one crash-recoverable generation. The
publisher syncs the new files and bounded copies of the previous pair before
installing a transaction journal containing exact local file identities and
full SHA-256 digests. Recovery keeps the new generation only when both target
digests match; otherwise it restores the complete previous pair. An interrupted
first publication is removed instead of exposing mismatched formats.

A successful publication also persists a run-scoped receipt before the port
returns. Receipt schema version 2 binds:

- hashes of the exact run identity and query;
- the closed publication outcome;
- claim, relation, derivation, basis-edge, gap, citation, and source metrics;
- the complete Markdown and HTML SHA-256 digests.

Version 1 receipts remain readable with zero graph counts, but cannot authorize
`Qualified` without an explicit gap. `resolve_deep_research_run_publication`
uses the receipt to recover a committed terminal result after a crash between
artifact publication and the host's terminal journal event. If a workflow
envelope and receipt both exist, their outcome, metrics, and artifact paths must
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
