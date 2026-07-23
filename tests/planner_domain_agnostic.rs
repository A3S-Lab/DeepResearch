use a3s_deep_research::planner::deep_research_loop_contract;

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
    for forbidden in [
        "world cup",
        "世界杯",
        "fifa",
        "football",
        "soccer",
        "olympic",
    ] {
        assert!(!prompt.to_ascii_lowercase().contains(forbidden));
    }
}
