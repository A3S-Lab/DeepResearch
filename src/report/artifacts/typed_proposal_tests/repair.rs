#[test]
fn shallow_comprehensive_rejection_exposes_dimension_specific_repair_contract() {
    const ANSWER: &str = "The closed records establish a bounded delivery path, but the operational decision still depends on how the two observed boundaries interact.";
    const FIRST_FACT: &str = "The first record establishes that validation occurs before ownership changes and records the durable state produced after that transition.";
    const SECOND_FACT: &str = "The second record independently establishes that downstream compatibility is checked at the consumer handoff and records the visible fallback state.";

    let mut context = one_track_context();
    context.scope = DeepResearchReportScope::Comprehensive;
    context.report_title = "Delivery decision and operating boundary".to_string();
    let catalog = catalog(vec![
        source(
            "source-1",
            "Ownership transition record",
            "https://example.test/ownership",
            &format!("{ANSWER} {FIRST_FACT}"),
        ),
        source(
            "source-2",
            "Consumer handoff record",
            "https://example.test/handoff",
            SECOND_FACT,
        ),
    ]);
    let proposal = with_narrative(
        serde_json::json!({
            "report_language": "en",
            "labels": labels(),
            "claims": [
                fact("answer", "direct_answer", ANSWER, "source-1"),
                fact("first-fact", "finding", FIRST_FACT, "source-1"),
                fact("second-fact", "finding", SECOND_FACT, "source-2"),
            ],
            "relations": [],
            "gaps": []
        }),
        &context,
    );

    let outcome = evaluate_deep_research_typed_report_draft_in_language_at(
        "Assess the delivery decision",
        "2026-07-30",
        "en",
        &catalog,
        &context,
        proposal.clone(),
    )
    .expect("evaluate shallow comprehensive proposal");
    let TypedReportDraftAdmission::Rejected(diagnostics) = outcome else {
        panic!("a source inventory must be rejected with repair diagnostics");
    };

    assert!(diagnostics.has_issue("request.answer", "missing_comparison"));
    assert!(diagnostics.has_issue("request.answer", "missing_explanation"));
    assert!(diagnostics.has_issue("request.answer", "missing_implication"));
    assert!(diagnostics.has_issue(
        "request.answer",
        "missing_challenge_or_boundary"
    ));
    assert!(diagnostics.has_issue(
        "request.answer",
        "insufficient_substantive_characters"
    ));
    assert!(diagnostics.is_repairable());

    let original_prompt = deep_research_typed_report_proposal_prompt_in_language_at(
        "Assess the delivery decision",
        "2026-07-30",
        "en",
        &catalog,
        &context,
    )
    .expect("original report prompt");
    let repair_prompt = deep_research_typed_report_repair_prompt(
        &original_prompt,
        &proposal,
        &diagnostics,
    )
    .expect("repair prompt");

    assert!(repair_prompt.contains("missing_comparison"));
    assert!(repair_prompt.contains("request.answer"));
    assert!(repair_prompt.contains("insufficient_substantive_characters"));
    assert!(repair_prompt.contains("Return one complete replacement object"));
    assert!(repair_prompt.contains("Do not pad, duplicate, or weaken"));
    assert!(repair_prompt.contains(
        "An unresolved dimension with at least two admitted evidence facts"
    ));
    assert!(repair_prompt.contains("Preserve its gap and do not manufacture closure"));
    assert!(repair_prompt.contains("CLOSED_TYPED_REPORT_PACKET="));
    assert!(!repair_prompt.contains("World Cup"));

    assert!(original_prompt.contains("localized equivalent of Executive Summary"));
    assert!(original_prompt.contains("takeaway first"));
    assert!(original_prompt.contains("comparison, magnitude or material difference"));
    assert!(original_prompt.contains("practical consequence"));
}
