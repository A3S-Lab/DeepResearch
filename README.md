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
                                      merged fetched-source packet
                                                       │
                                  deterministic source admission
                                                       │
                         ┌─────────────┴─────────────┐
                         │                           │
                  no safe evidence             safe evidence
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

Publication is progressive. Once safe fetched text exists, the engine stages a
source-backed Markdown and HTML report before attempting synthesis. A failed,
timed-out, or invalid proposal cannot erase that artifact. If no source passes
admission, the engine publishes an explicit no-evidence boundary report.

Semantic source admission is preserved as typed provenance from retrieval to
publication. The report compiler also preserves each source's exact research
track, completion-criterion, primary-source, and independent-source edges.
Comprehensive reports are admitted only when every material track has findings
that close all declared criteria and satisfy the declared source roles. The
published findings remain grouped by those research tracks.

Discovery fallback is deliberately audit-only for web evidence. Search rank,
query-word overlap, publisher names, TLDs, and maintained site allowlists never
promote a fallback web result into claim evidence. A web source becomes
claim-eligible only through the bounded semantic source-selection path;
workspace evidence remains eligible inside the user-authorized local scope.

## Ownership Boundary

| Layer | Owns |
| --- | --- |
| `DeepResearchEngine` | Stage ordering, bounded fallbacks, evidence merging, progressive publication, and terminal result metadata |
| Planner | Domain-neutral research scope, evidence tracks, completion criteria, and bounded supplemental queries |
| Retrieval workflows | Search/fetch tool orchestration, source selection, sanitization, and bounded evidence materialization |
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

The integration suite includes domain-agnostic contract and provenance tests
that preserve exact-query authority, keep semantic selection closed over exact
candidate IDs, retain typed research coverage, and reject topic-specific
production routing.

## License

MIT
