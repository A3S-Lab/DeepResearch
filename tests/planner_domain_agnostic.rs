use a3s_deep_research::planner::{
    deep_research_loop_contract, deep_research_loop_contract_for_language, host_fallback_plan,
    host_plan_from_outline, research_contract_from_plan,
};
use a3s_deep_research::report::deep_research_report_context_from_plan;

#[test]
fn contract_is_domain_agnostic_and_preserves_exact_query_authority() {
    let query = "Compare two storage engines";
    let contract = deep_research_loop_contract(query, "2026-07-23", "web available", usize::MAX);

    assert_eq!(contract["goal"], query);
    assert_eq!(contract["hard_caps"]["max_tracks"], 8);
    assert_eq!(contract["hard_caps"]["max_searches"], 16);
    assert_eq!(contract["hard_caps"]["max_gap_searches"], 24);
    assert_eq!(contract["hard_caps"]["max_fetches"], 16);
    assert_eq!(contract["hard_caps"]["max_supplemental_fetches"], 32);
    assert_eq!(contract["cardinality"]["gap_query_generations"], 4);
    assert_eq!(contract["cardinality"]["gap_extractions"], 4);
    assert_eq!(
        contract["planner"]["timeout_ms"],
        a3s_deep_research::engine::DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS
    );
    let prompt = contract["planner"]["prompt"].as_str().unwrap();
    assert!(prompt.contains("always searches the exact user query first"));
    assert!(prompt.contains("Do not use fixed topic taxonomies"));
    assert!(prompt.contains("Every returned track must cover an explicit part"));
    assert!(prompt.contains("atomic request_requirements"));
    assert!(prompt.contains("map every requirement ID to at least one track"));
    assert!(prompt.contains("role-labeled research questions"));
    assert!(prompt.contains("atomic enough for one source to resolve it completely"));
    assert!(prompt.contains("outcome-neutral and observable by the stated date"));
    assert!(prompt.contains("failed search alone never resolves a criterion"));
    assert!(prompt.contains("single-source yes-or-no evidence decision"));
    assert!(prompt.contains("not a demand for every possible metric or record"));
    assert!(prompt.contains("use a separate criterion for each subject"));
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["request_requirements"]["maxItems"],
        24
    );
    assert_eq!(
        contract["planner"]["output_schema"]["properties"]["tracks"]["items"]["properties"]
            ["requirement_ids"]["minItems"],
        1
    );
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
fn planner_contract_is_structurally_invariant_across_unrelated_subjects() {
    let queries = [
        "Assess heat-pump adoption evidence for a regional utility",
        "Compare two postoperative rehabilitation protocols",
        "Explain a distributed database migration and its failure modes",
        "评估一项跨境体育赛事的公共价值与风险",
    ];
    let contract_for = |query: &str| {
        deep_research_loop_contract_for_language(
            query,
            "2026-07-26",
            "web available",
            usize::MAX,
            "en",
        )
    };
    let baseline = contract_for(queries[0]);
    let baseline_prompt = baseline["planner"]["prompt"]
        .as_str()
        .expect("planner prompt")
        .replace(queries[0], "<untrusted-query>");

    for query in queries.into_iter().skip(1) {
        let candidate = contract_for(query);
        for pointer in [
            "/version",
            "/pattern",
            "/controller",
            "/quota",
            "/execution",
            "/cardinality",
            "/planner/agent",
            "/planner/max_steps",
            "/planner/timeout_ms",
            "/planner/output_schema",
            "/hard_caps",
        ] {
            assert_eq!(
                baseline.pointer(pointer),
                candidate.pointer(pointer),
                "contract structure changed for {pointer}"
            );
        }
        let candidate_prompt = candidate["planner"]["prompt"]
            .as_str()
            .expect("planner prompt")
            .replace(query, "<untrusted-query>");
        assert_eq!(baseline_prompt, candidate_prompt);
    }
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
            "request_requirements": [{
                "id": "request.compare-systems",
                "text": "Compare the supplied systems from their primary records."
            }],
            "tracks": [{
                "id": "systems",
                "title": "Compared systems",
                "focus": "Establish each system before comparing their boundaries.",
                "material": true,
                "requirement_ids": ["request.compare-systems"],
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
    let context = deep_research_report_context_from_plan(&plan)
        .expect("mapped request requirements must reach synthesis");
    assert_eq!(
        context.tracks[0]["request_requirements"][0]["id"],
        "request.compare-systems"
    );
}

#[test]
fn semantic_plan_rejects_any_unmapped_explicit_requirement() {
    let error = host_plan_from_outline(
        &serde_json::json!({
            "input": {
                "query": "Establish two independent requested outcomes",
                "evidence_scope": "web_and_workspace",
                "output_language": "en"
            }
        }),
        serde_json::json!({
            "report_title": "Requested outcomes",
            "research_scope": "focused",
            "freshness_required": false,
            "workspace_evidence_required": false,
            "request_requirements": [{
                "id": "request.first",
                "text": "Establish the first requested outcome."
            }, {
                "id": "request.second",
                "text": "Establish the second requested outcome."
            }],
            "tracks": [{
                "id": "outcomes",
                "title": "Requested outcomes",
                "focus": "Establish the requested outcomes.",
                "material": true,
                "requirement_ids": ["request.first"],
                "completion_criteria": ["The first outcome is established."],
                "questions": [{
                    "question": "What establishes the first outcome?",
                    "role": "establish",
                    "completion_criterion_indexes": [0]
                }],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false
                }
            }],
            "supplemental_queries": []
        }),
    )
    .expect_err("an unmapped user requirement must fail closed");

    assert!(error.contains("left request requirement(s) unmapped"));
    assert!(error.contains("request.second"));
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
