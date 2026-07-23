# A3S DeepResearch

`a3s-deep-research` is the standalone, evidence-first research engine used by
A3S products. It owns the reusable research control flow and report admission
rules without depending on the A3S CLI, TUI, or web application.

The implementation is domain-agnostic. Topic dictionaries, named-entity
branches, keyword-triggered routing, and domain-specific report fast paths are
not part of the engine.

## Architecture

```text
exact user query ──> bootstrap retrieval ───────────────┐
                                                       │
semantic outline ──> 0..3 validated supplemental queries
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
                                      optional closed report proposal
                                                     │
                                  deterministic claim/citation gates
                                                     │
                                admitted report or staged-source fallback
```

Bootstrap retrieval starts concurrently with semantic planning. Planning can
add bounded evidence tracks and supplemental plain-text queries, but it cannot
replace the exact query, return URLs, select transport budgets, or publish
facts. A planning failure therefore degrades to the original query instead of
ending the run.

Publication is progressive. Once the Host has a semantically admitted inquiry
projection, the engine stages a source-backed Markdown and HTML report before
attempting synthesis. A failed, timed-out, or invalid proposal cannot erase
that artifact. Raw acquisition may still be retained in an audit-only source
view, but it cannot support a conclusion. If no source passes semantic
admission, the engine publishes an explicit no-evidence boundary report.

The Host-projected inquiry collection is the sole semantic admission
authority. Raw acquisition metadata cannot promote source bytes by naming a
selector mode, and workspace paths receive no special trust. Web and workspace
sources must both arrive through exact selected source/chunk identities and
typed track, completion-criterion, primary-source, and independent-source
edges. Comprehensive reports are admitted only when every material track has
findings that close all declared criteria and satisfy the declared source
roles. The published findings remain grouped by those research tracks.

Discovery fallback is deliberately audit-only. Search rank, query-word
overlap, publisher names, domains, TLDs, URL path vocabulary, maintained site
lists, and workspace path shape never promote a result into claim evidence.
The Host validates only closed schemas, exact IDs, budgets, source bytes,
typed relationships, and provenance.

Reader-facing titles, section labels, and evidence-boundary prose are authored
inside closed model contracts. The Host validates their shape and renders them;
it does not detect a query language or choose a language-specific template.

## Ownership Boundary

| Layer | Owns |
| --- | --- |
| `DeepResearchEngine` | Stage ordering, bounded fallbacks, evidence merging, progressive publication, and terminal result metadata |
| Planner | Domain-neutral research scope, evidence tracks, completion criteria, and bounded supplemental queries |
| Retrieval workflows | Search/fetch orchestration, closed semantic selection, raw acquisition checkpoints, and bounded evidence materialization |
| Report pipeline | Closed-evidence prompts, source catalog construction, claim admission, citation gates, Markdown, and HTML |
| Product adapter | Model execution, workflow runtime/tool access, artifact storage, and progress presentation |

The product adapter implements four asynchronous ports:

- `StructuredGenerationPort` for the planning and report object generations;
- `WorkflowExecutionPort` for bootstrap and planned retrieval workflows;
- `PublicationPort` for atomic report artifact materialization; and
- `ProgressPort` for product-specific progress events.

Search-provider selection and provider fallback remain runtime policy. They are
not encoded as topic logic in the engine, and a provider fallback is never
confused with evidence admission.

## Integration

Add the crate and implement the four ports for the host product:

```rust
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
) -> Result<(), Box<dyn std::error::Error>> {
    let current_date = "2026-07-23";
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

`DeepResearchRun` returns the terminal publication class, the artifact paths,
and a structured output suitable for the host's existing workflow result
surface.

## Repository Layout

```text
src/
├── engine/       # Port-based asynchronous orchestration
├── planner/      # Domain-neutral planning contracts and validation
├── report/       # Evidence admission, quality gates, and publication
├── research/     # Replayable research state machine
└── workflow/     # Embedded retrieval and generation workflow assets
```

## Development

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo package --locked
```

The integration suite includes cross-topic structural-isomorphism tests,
domain-agnostic contract and provenance tests, and a runtime smoke test for the
embedded JavaScript discovery workflow. Together they preserve exact-query
authority, keep semantic selection closed over exact candidate IDs, and retain
typed research coverage without testing a blacklist of forbidden topics.

## License

MIT
