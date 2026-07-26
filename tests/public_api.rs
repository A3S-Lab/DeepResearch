use a3s_deep_research::{
    engine::{
        DeepResearchCancellation, DeepResearchEngine, DeepResearchRequest, EngineLimits,
        EvidenceScope, ProgressPort, PublicationPort, StructuredGenerationPort,
        WorkflowExecutionPort, WorkspaceSourceHint,
    },
    planner::deep_research_loop_contract,
};

#[allow(dead_code)]
async fn readme_integration_compiles(
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

    let _ = run.artifacts.html.display();
    Ok(())
}

#[allow(dead_code)]
async fn typed_engine_integration_compiles(
    generation: &dyn StructuredGenerationPort,
    workflow: &dyn WorkflowExecutionPort,
    publication: &dyn PublicationPort,
    progress: &dyn ProgressPort,
) -> Result<(), Box<dyn std::error::Error>> {
    let request = DeepResearchRequest::new(
        "product-run-id",
        "Assess the Aurora migration boundary",
        EvidenceScope::WebAndWorkspace,
    )
    .with_current_date("2026-07-23")
    .with_workspace_source_hints(vec![WorkspaceSourceHint::new("docs/aurora.md")]);
    let result = DeepResearchEngine::new(generation, workflow, publication, progress)
        .execute_request(request, DeepResearchCancellation::new())
        .await?;

    let _ = result.publication;
    let _ = result.artifacts.html.display();
    Ok(())
}

#[test]
fn public_planner_contract_preserves_the_host_inputs() {
    let query = "Assess the Aurora migration boundary";
    let contract = deep_research_loop_contract(query, "2026-07-23", "web_and_workspace", 4);

    assert_eq!(contract["goal"], query);
    assert_eq!(contract["controller"], "host_inquiry_reducer");
    assert_eq!(contract["hard_caps"]["max_tracks"], 4);
}
