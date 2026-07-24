use a3s_deep_research::planner::{deep_research_loop_contract, host_fallback_plan};

#[test]
fn contract_is_domain_agnostic_and_preserves_exact_query_authority() {
    let query = "Compare two storage engines";
    let contract = deep_research_loop_contract(query, "2026-07-23", "web available", usize::MAX);

    assert_eq!(contract["goal"], query);
    assert_eq!(contract["hard_caps"]["max_tracks"], 4);
    assert_eq!(contract["hard_caps"]["max_searches"], 4);
    let prompt = contract["planner"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("always searches the exact user query first"));
    assert!(prompt.contains("Do not use fixed topic taxonomies"));
}

#[test]
fn host_fallback_is_structurally_isomorphic_across_unrelated_queries() {
    let plan_for = |query: &str| {
        host_fallback_plan(&serde_json::json!({
            "input": {
                "query": query,
                "evidence_scope": "web_and_workspace"
            }
        }))
        .expect("fallback plan")
        .value
    };
    let first = plan_for("Compare two storage engines");
    let second = plan_for("核查一个公共事件的最新状态");

    for pointer in [
        "/research_scope",
        "/freshness_required",
        "/workspace_evidence_required",
        "/budget",
        "/stop_conditions",
        "/tracks/0/id",
        "/tracks/0/material",
        "/tracks/0/evidence_requirements",
    ] {
        assert_eq!(first.pointer(pointer), second.pointer(pointer), "{pointer}");
    }
    assert_eq!(
        first.pointer("/research_scope"),
        Some(&serde_json::json!("comprehensive")),
        "unknown semantic scope must use the stronger publication gate"
    );
    assert_eq!(
        first.pointer("/freshness_required"),
        Some(&serde_json::json!(true)),
        "unknown freshness must not authorize an undated synthesized answer"
    );
    assert_eq!(
        first.pointer("/search_queries/0"),
        Some(&serde_json::json!("Compare two storage engines"))
    );
    assert_eq!(
        second.pointer("/search_queries/0"),
        Some(&serde_json::json!("核查一个公共事件的最新状态"))
    );
}
