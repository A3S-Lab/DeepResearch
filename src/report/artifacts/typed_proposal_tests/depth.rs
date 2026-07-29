#[test]
fn comprehensive_report_requires_a_direct_answer_for_each_resolved_material_dimension() {
    const FIRST_ANSWER: &str = "The first material dimension is established by the closed record, which documents the relevant execution boundary, the responsible component, and the observable completion state in one traceable account.";
    const FIRST_FINDING_ONE: &str = "The first record separately describes how the request enters the system, which component accepts it, and which validated state transition occurs before downstream processing begins.";
    const FIRST_FINDING_TWO: &str = "The same record also identifies the settlement boundary and the durable output produced there, allowing that part of the first dimension to be checked without relying on an inferred connection.";
    const SECOND_FINDING_ONE: &str = "The second material dimension is supported by an independent closed record that identifies the presentation handoff, the preferred rendering path, and the observable result returned to the caller.";
    const SECOND_FINDING_TWO: &str = "That second record additionally documents the bounded fallback behavior, including the triggering condition, the alternate path selected, and the final state visible to the user.";
    const SECOND_FINDING_THREE: &str = "The second record distinguishes the preferred path from its fallback and records the result of each, so the report can describe both outcomes without merging separate observations.";
    const CROSS_SOURCE_ANALYSIS: &str = "Taken together, the two independently attributable records show that the execution boundary and presentation handoff must be evaluated as one end-to-end delivery path rather than as isolated source summaries.";
    const FIRST_MECHANISM: &str = "The first dimension depends on both entry validation and durable settlement: if either boundary is checked alone, the system can accept a request without proving that its final state is recoverable.";
    const FIRST_IMPLICATION: &str = "The combined evidence therefore makes end-to-end verification the relevant acceptance unit for the first dimension, with separate checks at request entry and durable settlement.";
    const FIRST_BOUNDARY: &str = "That conclusion does not extend to paths where entry validation and durable settlement cannot be observed independently: in those environments, a successful acceptance signal can coexist with an unrecoverable terminal state, so the evidence supports qualification rather than automatic approval.";
    const SECOND_SYNTHESIS: &str = "Across the two records, the preferred presentation handoff and its fallback are complementary states of one delivery contract rather than unrelated implementation details.";
    const SECOND_MECHANISM: &str = "The fallback preserves a visible result when the preferred renderer cannot complete, which explains why the handoff must be assessed by observable outcome instead of path selection alone.";
    const SECOND_IMPLICATION: &str = "The second dimension should therefore be accepted only when both the preferred path and fallback produce distinguishable, user-visible terminal states under their documented trigger conditions.";
    const SECOND_BOUNDARY: &str = "The result is bounded to the documented trigger conditions and visible terminal states; it does not establish equivalent behavior for an unobserved renderer failure, a fallback that suppresses its status, or a caller that cannot distinguish preferred and alternate outputs. Those cases require separate evidence because the present records cannot show whether apparent delivery reflects the preferred path, a silent fallback, or a stale result retained by the caller.";

    let context = two_track_comprehensive_context();
    let shared_evidence = format!(
        "{FIRST_ANSWER} {FIRST_FINDING_ONE} {FIRST_FINDING_TWO} {SECOND_FINDING_ONE} {SECOND_FINDING_TWO} {SECOND_FINDING_THREE}"
    );
    let mut catalog = catalog(vec![
        source_for_track(
            "source-1",
            "First closed record",
            "https://example.test/first",
            &shared_evidence,
            "request.first",
        ),
        source_for_track(
            "source-2",
            "Second closed record",
            "https://example.test/second",
            &shared_evidence,
            "request.second",
        ),
    ]);
    catalog.sources[0]
        .relevant_track_ids
        .push("request.second".to_string());
    catalog.sources[0]
        .coverage
        .push(DeepResearchSourceCoverage {
            track_id: "request.second".to_string(),
            completion_criterion_indexes: vec![0],
            primary: false,
            independent: true,
        });
    catalog.sources[1]
        .relevant_track_ids
        .push("request.first".to_string());
    catalog.sources[1]
        .coverage
        .push(DeepResearchSourceCoverage {
            track_id: "request.first".to_string(),
            completion_criterion_indexes: vec![0],
            primary: false,
            independent: true,
        });
    let prompt = deep_research_typed_report_proposal_prompt_at(
        "research both material dimensions",
        "2026-07-24",
        &catalog,
        &context,
    )
    .expect("typed prompt");
    assert!(prompt.contains("write exactly one conclusion"));
    assert!(prompt.contains("one cross-source comparison"));
    assert!(prompt.contains("\"substantive_characters\":1200"));
    assert!(!prompt.contains("\"substantive_characters\":800"));

    let mut proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact_for_dimension(
                "first-answer",
                "request.first",
                "direct_answer",
                FIRST_ANSWER,
                "source-1",
            ),
            fact_for_dimension(
                "first-finding-one",
                "request.first",
                "finding",
                FIRST_FINDING_ONE,
                "source-1",
            ),
            fact_for_dimension(
                "first-finding-two",
                "request.first",
                "finding",
                FIRST_FINDING_TWO,
                "source-2",
            ),
            fact_for_dimension(
                "second-finding-one",
                "request.second",
                "finding",
                SECOND_FINDING_ONE,
                "source-2",
            ),
            fact_for_dimension(
                "second-finding-two",
                "request.second",
                "finding",
                SECOND_FINDING_TWO,
                "source-1",
            ),
            fact_for_dimension(
                "second-finding-three",
                "request.second",
                "finding",
                SECOND_FINDING_THREE,
                "source-2",
            ),
            inference_for_dimension(
                "cross-source-analysis",
                "request.first",
                "finding",
                "comparison",
                CROSS_SOURCE_ANALYSIS,
                &["first-finding-one", "second-finding-one"],
            ),
            inference_for_dimension(
                "first-mechanism",
                "request.first",
                "finding",
                "explanation",
                FIRST_MECHANISM,
                &["first-finding-one", "first-finding-two"],
            ),
            inference_for_dimension(
                "first-implication",
                "request.first",
                "finding",
                "implication",
                FIRST_IMPLICATION,
                &["cross-source-analysis", "first-mechanism"],
            ),
            inference_for_dimension(
                "first-boundary",
                "request.first",
                "finding",
                "boundary",
                FIRST_BOUNDARY,
                &["first-mechanism", "first-finding-two"],
            ),
            inference_for_dimension(
                "second-synthesis",
                "request.second",
                "finding",
                "comparison",
                SECOND_SYNTHESIS,
                &["second-finding-one", "second-finding-two"],
            ),
            inference_for_dimension(
                "second-mechanism",
                "request.second",
                "finding",
                "explanation",
                SECOND_MECHANISM,
                &["second-finding-two", "second-finding-three"],
            ),
            inference_for_dimension(
                "second-implication",
                "request.second",
                "finding",
                "implication",
                SECOND_IMPLICATION,
                &["second-synthesis", "second-mechanism"],
            ),
            inference_for_dimension(
                "second-boundary",
                "request.second",
                "finding",
                "boundary",
                SECOND_BOUNDARY,
                &["second-mechanism", "second-finding-three"],
            ),
        ],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "research both material dimensions",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("typed admission")
    .expect("useful claims from the answered dimension must survive");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Qualified,
        "the dimension without a direct answer must be bounded instead of presented as resolved"
    );
    assert_eq!(report.direct_answer_block_count, 1);
    assert_eq!(report.accepted_gap_count, 1);

    proposal["claims"][3]["placement"] = serde_json::json!("direct_answer");
    proposal["claims"][3]["analysis_role"] = serde_json::json!("conclusion");
    let report = admit_deep_research_typed_report_proposal_at(
        "research both material dimensions",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("typed admission")
    .expect("dimension-complete report");
    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(report.direct_answer_block_count, 2);

    proposal["claims"][3]["placement"] = serde_json::json!("finding");
    proposal["claims"][3]["analysis_role"] = serde_json::json!("evidence");
    proposal["gaps"] = serde_json::json!([{
        "id": "second-gap",
        "dimension_id": "request.second",
        "text": "The retained evidence leaves one bounded aspect of the second dimension unresolved."
    }]);
    let redundant_gap = admit_deep_research_typed_report_proposal_at(
        "research both material dimensions",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("typed admission")
    .expect("the Host-owned claim-depth gap must preserve useful claims");
    assert_eq!(
        redundant_gap.publication,
        DeepResearchEvidenceFirstPublication::Qualified,
        "a model-authored redundant gap cannot turn a claim-incomplete dimension into an answer"
    );
    assert_eq!(redundant_gap.direct_answer_block_count, 1);
    assert_eq!(redundant_gap.accepted_gap_count, 1);

    let mut optional_context = context.clone();
    optional_context.tracks[1]["material"] = serde_json::json!(false);
    let report = admit_deep_research_typed_report_proposal_at(
        "research both material dimensions",
        "2026-07-24",
        &catalog,
        &optional_context,
        with_narrative(proposal.clone(), &optional_context),
    )
    .expect("typed admission")
    .expect("report with a resolved optional dimension");
    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Synthesized,
        "a model gap cannot expand a resolved optional dimension"
    );
    assert_eq!(report.accepted_gap_count, 0);
    assert_eq!(report.rejected_block_count, 1);

    let mut bounded_catalog = catalog.clone();
    for source in &mut bounded_catalog.sources {
        source
            .coverage
            .retain(|coverage| coverage.track_id != "request.second");
    }
    proposal["claims"][3]["placement"] = serde_json::json!("direct_answer");
    proposal["claims"][3]["analysis_role"] = serde_json::json!("conclusion");
    let report = admit_deep_research_typed_report_proposal_at(
        "research both material dimensions",
        "2026-07-24",
        &bounded_catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("honestly bounded report");
    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Qualified
    );
    assert_eq!(report.direct_answer_block_count, 1);
    assert_eq!(report.accepted_gap_count, 1);
    let (summary, body) = report
        .markdown
        .split_once("## Findings")
        .expect("summary and report body");
    assert!(!summary.contains(SECOND_FINDING_ONE));
    assert!(body.contains(SECOND_FINDING_ONE));
}
#[test]
fn comprehensive_report_requires_cross_source_analysis_not_fact_padding() {
    const ANSWER: &str = "The retained records establish that the migration is feasible only when the new execution boundary and the existing operational constraint are treated as one coordinated change, with verification at both handoff points.";
    const FACT_ONE: &str = "The first record documents the new execution boundary, identifies the component that owns the transition, and states the observable completion signal that downstream consumers can verify.";
    const FACT_TWO: &str = "The second record independently documents the existing operational constraint, including the compatibility condition that must remain true while the migration is rolled out.";
    const FACT_THREE: &str = "The first record also describes the failure path and the durable state preserved when the preferred transition cannot complete, which bounds the migration's recovery behavior.";
    const FACT_FOUR: &str = "The second record separately describes the consumer-visible handoff and the validation result expected after a successful transition, providing an independent check on the end state.";
    const ANALYSIS: &str = "Because the ownership transition from the first record and the compatibility boundary from the second record constrain the same delivery path, the migration should be evaluated as a coordinated system change rather than as two unrelated implementation updates.";
    const MECHANISM: &str = "The migration fails when ownership changes before the compatibility condition is verified, because downstream consumers can then observe a completed transition while still depending on the old operational boundary.";
    const IMPLICATION: &str = "The combined evidence therefore supports a staged acceptance decision: verify the compatibility condition first, exercise the preserved failure path second, and approve the ownership transition only after both handoff signals agree.";
    const BOUNDARY: &str = "This acceptance sequence does not prove safety when either handoff signal is unavailable, when compatibility can change after verification, or when the preserved failure path has not been exercised under the same operational constraints; any of those conditions leaves the migration conclusion qualified.";

    let mut context = one_track_context();
    context.scope = DeepResearchReportScope::Comprehensive;
    context.report_title = "Migration feasibility and operational boundary".to_string();
    let catalog = catalog(vec![
        source(
            "source-1",
            "Execution boundary record",
            "https://example.test/execution",
            &format!("{ANSWER} {FACT_ONE} {FACT_THREE}"),
        ),
        source(
            "source-2",
            "Operational constraint record",
            "https://example.test/operations",
            &format!("{FACT_TWO} {FACT_FOUR}"),
        ),
    ]);
    let mut proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("fact-one", "finding", FACT_ONE, "source-1"),
            fact("fact-two", "finding", FACT_TWO, "source-2"),
            fact("fact-three", "finding", FACT_THREE, "source-1"),
            fact("fact-four", "finding", FACT_FOUR, "source-2"),
        ],
        "relations": [],
        "gaps": []
    });
    proposal["labels"]["findings"] = serde_json::json!("Evidence");
    proposal["labels"]["inference"] = serde_json::json!("Analysis");

    let shallow = admit_deep_research_typed_report_proposal_in_language_at(
        "Assess the migration",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("evaluate fact-only proposal");
    assert!(
        shallow.is_none(),
        "a comprehensive fact inventory must not pass as deep synthesis"
    );

    proposal["claims"]
        .as_array_mut()
        .expect("claims array")
        .push(inference_for_dimension(
            "cross-source-analysis",
            "request.answer",
            "finding",
            "comparison",
            ANALYSIS,
            &["fact-one", "fact-two"],
        ));
    let shallow_analysis = admit_deep_research_typed_report_proposal_in_language_at(
        "Assess the migration",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("evaluate one-step analytical proposal");
    assert!(
        shallow_analysis.is_none(),
        "one cross-source paragraph must not make an otherwise thin section deep"
    );

    proposal["claims"]
        .as_array_mut()
        .expect("claims array")
        .extend([
            inference_for_dimension(
                "mechanism-analysis",
                "request.answer",
                "finding",
                "explanation",
                MECHANISM,
                &["fact-two", "fact-three"],
            ),
            inference_for_dimension(
                "decision-implication",
                "request.answer",
                "finding",
                "implication",
                IMPLICATION,
                &["cross-source-analysis", "mechanism-analysis"],
            ),
            inference_for_dimension(
                "migration-boundary",
                "request.answer",
                "finding",
                "boundary",
                BOUNDARY,
                &["mechanism-analysis", "fact-four"],
            ),
        ]);
    let report = admit_deep_research_typed_report_proposal_in_language_at(
        "Assess the migration",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("evaluate multi-step analytical proposal")
    .expect("dimension-deep analytical report");

    assert_eq!(report.accepted_basis_edge_count, 8);
    assert_eq!(report.analytical_claim_count, 4);
    assert_eq!(report.cross_source_synthesis_count, 1);
    assert_eq!(report.resolved_material_dimension_count, 1);
    assert_eq!(report.deeply_analyzed_dimension_count, 1);
    assert!(!report.markdown.contains("#### Evidence"));
    assert!(!report.markdown.contains("#### Analysis"));
    assert!(report.markdown.contains(ANALYSIS));
    assert!(report.markdown.contains(MECHANISM));
    assert!(report.markdown.contains(IMPLICATION));
    assert!(report.markdown.contains(BOUNDARY));

    let mut bounded_catalog = catalog;
    for source in &mut bounded_catalog.sources {
        source.coverage.clear();
    }
    let bounded = admit_deep_research_typed_report_proposal_in_language_at(
        "Assess the migration",
        "2026-07-24",
        "en",
        &bounded_catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("evaluate all-bounded analytical proposal");

    assert!(
        bounded.is_none(),
        "an all-bounded comprehensive proposal must remain an incomplete preview"
    );
}

#[test]
fn strict_typed_report_language_is_host_pinned_to_the_users_language() {
    let context = DeepResearchReportContext {
        report_title: "A3S 深度研究实现评估".to_string(),
        scope: DeepResearchReportScope::Focused,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.answer",
            "title": "实现结论",
            "focus": "核查统一实现的边界。",
            "material": true,
            "completion_criteria": ["来源直接说明统一实现的边界。"],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false,
            },
        })],
    };
    let catalog = catalog(vec![source(
        "source-1",
        "实现记录",
        "https://example.test/implementation",
        "实现记录明确说明，终端和网页端通过同一个深度研究引擎执行，并共享相同的证据准入规则。",
    )]);
    let proposal = serde_json::json!({
        "report_language": "zh",
        "labels": chinese_labels(),
        "claims": [fact(
            "answer",
            "direct_answer",
            "终端和网页端通过同一个深度研究引擎执行，并共享相同的证据准入规则。",
            "source-1",
        )],
        "relations": [],
        "gaps": []
    });

    let schema = deep_research_typed_report_proposal_schema_for_language(&catalog, &context, "zh")
        .expect("strict language schema");
    assert_eq!(
        schema["properties"]["report_language"]["enum"],
        serde_json::json!(["zh"])
    );
    let prompt = deep_research_typed_report_proposal_prompt_in_language_at(
        "评估 A3S 深度研究实现",
        "2026-07-24",
        "zh",
        &catalog,
        &context,
    )
    .expect("strict language prompt");
    assert!(prompt.contains("OUTPUT_LANGUAGE=zh"));

    let report = admit_deep_research_typed_report_proposal_in_language_at(
        "评估 A3S 深度研究实现",
        "2026-07-24",
        "zh",
        &catalog,
        &context,
        with_narrative(proposal.clone(), &context),
    )
    .expect("Chinese report admission")
    .expect("Chinese report");
    assert!(report.markdown.contains("## 结论"));
    assert!(report.markdown.contains("## 来源"));
    assert!(!report.markdown.contains("Direct Answer"));
    assert!(report
        .rendered_html
        .as_deref()
        .is_some_and(|html| html.contains("<html lang=\"zh\">")));

    let mut mismatched = proposal;
    mismatched["report_language"] = serde_json::json!("en");
    assert!(
        admit_deep_research_typed_report_proposal_in_language_at(
            "评估 A3S 深度研究实现",
            "2026-07-24",
            "zh",
            &catalog,
            &context,
            with_narrative(mismatched, &context),
        )
        .is_err(),
        "the model must not override the host-owned output language"
    );
}

#[test]
fn host_closes_an_omitted_unresolved_dimension_with_the_authored_evidence_boundary() {
    let context = one_track_context();
    let mut incomplete_source = source(
        "source-1",
        "Incomplete record",
        "https://example.test/incomplete",
        "The retained record supports a useful partial answer but does not resolve the declared completion criterion.",
    );
    incomplete_source.coverage.clear();
    let catalog = catalog(vec![incomplete_source]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [fact(
            "partial-answer",
            "direct_answer",
            "The retained record supports a useful partial answer.",
            "source-1",
        )],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "establish the answer",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("qualified report");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Qualified
    );
    assert_eq!(report.accepted_claim_count, 1);
    assert_eq!(report.accepted_gap_count, 1);
    assert!(report
        .markdown
        .contains("No conclusion is published beyond the fetched evidence."));
}

#[test]
fn repeated_chunk_references_from_one_source_are_coalesced_before_compilation() {
    let context = one_track_context();
    let mut record = source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The first retained passage establishes the requested answer.",
    );
    record
        .chunks
        .push("The second retained passage provides the supporting detail.".to_string());
    let catalog = catalog(vec![record]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [{
            "id": "answer",
            "dimension_id": "request.answer",
            "placement": "direct_answer",
            "kind": "fact",
            "analysis_role": "conclusion",
            "text": "The closed record establishes the requested answer and provides the supporting detail.",
            "evidence_refs": [
                {
                    "source_id": "source-1",
                    "chunk_ids": ["source-1:chunk:1"]
                },
                {
                    "source_id": "source-1",
                    "chunk_ids": ["source-1:chunk:2"]
                }
            ],
            "basis_claim_ids": [],
            "derivation": null
        }],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "establish the answer",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("coalesced report");

    assert_eq!(report.accepted_claim_count, 1);
    assert_eq!(report.rejected_block_count, 0);
    assert_eq!(report.cited_source_count, 1);
}

#[test]
fn only_the_leading_answer_claim_per_dimension_stays_in_the_report_summary() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The closed record states the bounded conclusion and separately documents the supporting implementation detail.",
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "answer",
                "direct_answer",
                "The closed record states the bounded conclusion.",
                "source-1",
            ),
            fact(
                "detail",
                "direct_answer",
                "The same record separately documents the supporting implementation detail.",
                "source-1",
            ),
        ],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "establish the answer",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("structured report");

    assert_eq!(report.direct_answer_block_count, 1);
    assert_eq!(report.finding_block_count, 1);
    let (summary, body) = report
        .markdown
        .split_once("## Findings")
        .expect("summary and report body");
    assert!(summary.contains("states the bounded conclusion"));
    assert!(!summary.contains("supporting implementation detail"));
    assert!(body.contains("supporting implementation detail"));
}

#[test]
fn authored_narrative_heading_and_paragraphs_control_the_reader_flow() {
    const ANSWER: &str =
        "The closed record establishes a validated path from request entry to durable settlement.";
    const INTAKE: &str =
        "At intake, the request crosses an explicit validation boundary before any downstream work begins.";
    const SETTLEMENT: &str =
        "After validation, the owning component records a durable terminal state that can be checked independently of request acceptance.";
    const BOUNDARY: &str =
        "That separation makes settlement, rather than entry alone, the relevant completion boundary for the reviewed path.";
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed execution record",
        "https://example.test/execution",
        &format!("{ANSWER} {INTAKE} {SETTLEMENT} {BOUNDARY}"),
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("intake", "finding", INTAKE, "source-1"),
            fact("settlement", "finding", SETTLEMENT, "source-1"),
            fact("boundary", "finding", BOUNDARY, "source-1"),
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "request.answer",
                "heading": "From validated entry to durable settlement",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["intake", "settlement"]
                }, {
                    "purpose": "evidence",
                    "claim_ids": ["boundary"]
                }]
            }]
        }
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "trace the execution boundary",
        "2026-07-24",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed admission")
    .expect("narrated report");

    assert!(report
        .markdown
        .contains("### From validated entry to durable settlement"));
    assert!(!report.markdown.contains("#### Evidence"));
    assert_eq!(report.markdown.matches(INTAKE).count(), 1);
    assert_eq!(report.markdown.matches(SETTLEMENT).count(), 1);
    let html = report.rendered_html.expect("typed HTML");
    assert!(html.contains("From validated entry to durable settlement"));
    assert_eq!(html.matches("class=\"report-paragraph\"").count(), 3);
}
