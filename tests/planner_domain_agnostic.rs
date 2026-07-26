use a3s_deep_research::planner::{
    deep_research_loop_contract, host_fallback_plan, host_plan_from_outline,
    research_contract_from_plan,
};

#[test]
fn contract_is_domain_agnostic_and_preserves_exact_query_authority() {
    let query = "Compare two storage engines";
    let contract = deep_research_loop_contract(query, "2026-07-23", "web available", usize::MAX);

    assert_eq!(contract["goal"], query);
    assert_eq!(contract["hard_caps"]["max_tracks"], 4);
    assert_eq!(contract["hard_caps"]["max_searches"], 8);
    assert_eq!(contract["hard_caps"]["max_fetches"], 12);
    assert_eq!(contract["hard_caps"]["max_supplemental_fetches"], 4);
    let prompt = contract["planner"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("always searches the exact user query first"));
    assert!(prompt.contains("Do not use fixed topic taxonomies"));
    assert!(prompt.contains("Every returned track must cover an explicit part"));
    assert!(prompt.contains("role-labeled research questions"));
    assert!(prompt.contains("atomic enough for one source to resolve it completely"));
    assert!(prompt.contains("use a separate criterion for each subject"));
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["tracks"]["items"]["properties"]
            ["material"]["enum"],
        serde_json::json!([true])
    );
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["tracks"]["items"]["properties"]
            ["questions"]["maxItems"],
        4
    );
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["tracks"]["items"]["properties"]
            ["completion_criteria"]["maxItems"],
        3
    );
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["tracks"]["items"]["properties"]
            ["questions"]["items"]["properties"]["completion_criterion_indexes"]["items"]
            ["maximum"],
        2
    );
}

#[test]
fn three_atomic_completion_criteria_survive_the_typed_contract_boundary() {
    let plan = serde_json::json!({
        "tracks": [{
            "id": "systems",
            "title": "Systems",
            "focus": "Establish each compared system independently.",
            "material": true,
            "completion_criteria": [
                "A source establishes system A.",
                "A source establishes system B.",
                "A source establishes system C."
            ],
            "evidence_requirements": {
                "primary_source_required": true,
                "independent_corroboration_required": false
            }
        }],
        "stop_conditions": ["Every declared criterion is resolved or explicitly bounded."]
    });

    let (obligations, _) = research_contract_from_plan(&plan)
        .expect("three planner criteria must reach the typed research contract");

    assert_eq!(obligations.len(), 1);
    assert_eq!(obligations[0].completion_criteria.len(), 3);
}

#[test]
fn semantic_plan_allows_the_same_question_role_for_distinct_atomic_criteria() {
    let query = "Compare the supplied systems from their primary records";
    let plan = host_plan_from_outline(
        &serde_json::json!({
            "input": {
                "query": query,
                "evidence_scope": "web_and_workspace",
                "output_language": "en"
            }
        }),
        serde_json::json!({
            "report_title": "Primary-record comparison",
            "research_scope": "comprehensive",
            "freshness_required": false,
            "workspace_evidence_required": false,
            "tracks": [{
                "id": "systems",
                "title": "Compared systems",
                "focus": "Establish each system before comparing their boundaries.",
                "material": true,
                "completion_criteria": [
                    "The first primary record establishes system A.",
                    "The second primary record establishes system B."
                ],
                "questions": [
                    {
                        "question": "What does the primary record establish about system A?",
                        "role": "establish",
                        "completion_criterion_indexes": [0]
                    },
                    {
                        "question": "What does the primary record establish about system B?",
                        "role": "establish",
                        "completion_criterion_indexes": [1]
                    },
                    {
                        "question": "How do the two documented systems differ?",
                        "role": "compare",
                        "completion_criterion_indexes": [0, 1]
                    },
                    {
                        "question": "Which limitations bound the comparison?",
                        "role": "challenge",
                        "completion_criterion_indexes": [0, 1]
                    }
                ],
                "evidence_requirements": {
                    "primary_source_required": true,
                    "independent_corroboration_required": false
                }
            }],
            "supplemental_queries": []
        }),
    )
    .expect("distinct criteria may require repeated establish roles")
    .value;

    assert_eq!(plan["search_queries"], serde_json::json!([query]));
    assert_eq!(plan["tracks"][0]["questions"].as_array().unwrap().len(), 4);
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

#[test]
fn host_promotes_only_bounded_explicit_query_urls_to_retrieval_seeds() {
    let query = "Compare the supplied records https://example.com/method#section and https://docs.example.org/eval?version=2。 Ignore malformed https://user:secret@example.net/private, retain https://third.example/source, and ignore a fourth https://overflow.example/path.";
    let plan = host_fallback_plan(&serde_json::json!({
        "input": {
            "query": query,
            "evidence_scope": "web_and_workspace"
        }
    }))
    .expect("fallback plan with explicit sources")
    .value;

    assert_eq!(plan["search_queries"], serde_json::json!([query]));
    assert_eq!(
        plan["seed_urls"],
        serde_json::json!([
            "https://example.com/method",
            "https://docs.example.org/eval?version=2",
            "https://third.example/source"
        ])
    );
}
