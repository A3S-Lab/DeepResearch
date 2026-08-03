#[test]
fn one_sufficient_focused_claim_is_a_completed_report() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Focused record",
        "https://example.test/focused",
        "The focused record establishes the complete bounded answer.",
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [fact(
            "focused-answer",
            "direct_answer",
            "The focused record establishes the complete bounded answer.",
            "source-1",
        )],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "focused query",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("completed focused report");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(report.accepted_claim_count, 1);
    assert_eq!(report.accepted_relation_count, 0);
    assert_eq!(report.accepted_derivation_count, 0);
    assert_eq!(report.accepted_basis_edge_count, 0);
    assert_eq!(report.accepted_gap_count, 0);
    assert_eq!(report.direct_answer_block_count, 1);
    assert_eq!(report.finding_block_count, 0);
    assert!(report.markdown.contains("complete bounded answer"));
    assert!(!report.markdown.contains("focused-answer"));
    assert!(report.markdown.contains("### Requested answer"));
    assert!(!report
        .markdown
        .contains("### Establish the requested answer from the closed evidence."));
    let workspace = tempfile::tempdir().expect("typed report workspace");
    let artifacts =
        materialize_deep_research_admitted_report(workspace.path(), "focused query", &report)
            .expect("materialize typed report");
    assert!(completed_research_report_artifacts(&artifacts));
}

#[test]
fn redundant_recommendation_derivation_is_normalized_to_basis_edges() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Focused record",
        "https://example.test/focused",
        "The focused record establishes the deployment constraint used by the recommendation.",
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "deployment-constraint",
                "finding",
                "The focused record establishes the deployment constraint.",
                "source-1",
            ),
            {
                "id": "bounded-recommendation",
                "dimension_id": "request.answer",
                "placement": "direct_answer",
                "kind": "recommendation",
                "analysis_role": "conclusion",
                "text": "Adopt only when the deployment constraint can be satisfied.",
                "evidence_refs": [],
                "basis_claim_ids": ["deployment-constraint"],
                "derivation": {
                    "method": "Apply the documented constraint as the adoption boundary.",
                    "input_claim_ids": ["deployment-constraint"]
                }
            }
        ],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "decide whether to adopt",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("normalized recommendation report");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(report.accepted_claim_count, 2);
    assert_eq!(report.accepted_derivation_count, 0);
    assert_eq!(report.accepted_basis_edge_count, 1);
    assert!(report.markdown.contains("Adopt only when"));
    assert!(!report
        .markdown
        .contains("Apply the documented constraint as the adoption boundary."));
}

#[test]
fn inference_basis_ignores_a_recommendation_when_factual_support_remains() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Focused record",
        "https://example.test/focused",
        "The focused record establishes the deployment constraint used by the conclusion.",
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "deployment-constraint",
                "finding",
                "The focused record establishes the deployment constraint.",
                "source-1",
            ),
            {
                "id": "adoption-recommendation",
                "dimension_id": "request.answer",
                "placement": "finding",
                "kind": "recommendation",
                "analysis_role": "implication",
                "text": "Adopt only when the deployment constraint can be satisfied.",
                "evidence_refs": [],
                "basis_claim_ids": ["deployment-constraint"],
                "derivation": null
            },
            {
                "id": "bounded-conclusion",
                "dimension_id": "request.answer",
                "placement": "direct_answer",
                "kind": "inference",
                "analysis_role": "conclusion",
                "text": "The deployment constraint defines the bounded adoption scope.",
                "evidence_refs": [],
                "basis_claim_ids": ["deployment-constraint", "adoption-recommendation"],
                "derivation": {
                    "method": "Apply the documented constraint as the adoption boundary.",
                    "input_claim_ids": ["deployment-constraint"]
                }
            }
        ],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "derive the adoption scope",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("normalized inference report");

    assert_eq!(report.accepted_claim_count, 3);
    assert_eq!(report.rejected_block_count, 0);
    assert_eq!(report.direct_answer_block_count, 1);
    assert_eq!(report.accepted_basis_edge_count, 2);
    assert!(report.markdown.contains("bounded adoption scope"));
}

#[test]
fn derivation_prose_cannot_repeat_opaque_graph_ids() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Focused record",
        "https://example.test/focused",
        "The focused record establishes the deployment constraint used by the inference.",
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "deployment-constraint",
                "finding",
                "The focused record establishes the deployment constraint.",
                "source-1",
            ),
            {
                "id": "bounded-inference",
                "dimension_id": "request.answer",
                "placement": "direct_answer",
                "kind": "inference",
                "analysis_role": "conclusion",
                "text": "The deployment constraint defines the bounded adoption scope.",
                "evidence_refs": [],
                "basis_claim_ids": ["deployment-constraint"],
                "derivation": {
                    "method": "Apply deployment-constraint within request.answer.",
                    "input_claim_ids": ["deployment-constraint"]
                }
            }
        ],
        "relations": [],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "derive the adoption scope",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("normalized inference report");

    assert_eq!(report.accepted_claim_count, 2);
    assert_eq!(report.accepted_derivation_count, 0);
    assert_eq!(report.accepted_basis_edge_count, 1);
    assert!(!report
        .markdown
        .contains("Apply deployment-constraint within request.answer."));
}

#[test]
fn resolved_typed_coverage_rejects_model_scope_expansion_gaps() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Complete record",
        "https://example.test/complete",
        "The complete record directly establishes every declared completion criterion.",
    )]);
    let schema =
        deep_research_typed_report_proposal_schema_for(&catalog, &context).expect("typed schema");
    assert_eq!(
        schema["properties"]["gaps"]["maxItems"],
        serde_json::json!(0),
        "a fully resolved typed contract has no model-owned gap slot"
    );
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [fact(
            "complete-answer",
            "direct_answer",
            "The complete record directly establishes every declared completion criterion.",
            "source-1",
        )],
        "relations": [],
        "gaps": [{
            "id": "scope-expansion",
            "dimension_id": "request.answer",
            "text": "The record does not address an additional detail outside the declared completion criterion."
        }]
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "focused query",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("valid claims survive a redundant model gap");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(report.accepted_claim_count, 1);
    assert_eq!(report.accepted_gap_count, 0);
    assert_eq!(report.rejected_block_count, 1);
    assert!(!report.markdown.contains("additional detail"));
}

#[test]
fn typed_report_preserves_contradictions_and_reproducible_derivations() {
    let context = one_track_context();
    let catalog = catalog(vec![
        source(
            "source-1",
            "First primary record",
            "https://example.test/first",
            "The first record reports 100 units on 1 July.",
        ),
        source(
            "source-2",
            "Second primary record",
            "https://example.test/second",
            "The second record reports 80 units on 1 July.",
        ),
    ]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "first-value",
                "finding",
                "The first record reports 100 units on 1 July.",
                "source-1",
            ),
            fact(
                "second-value",
                "finding",
                "The second record reports 80 units on 1 July.",
                "source-2",
            ),
            {
                "id": "derived-difference",
                "dimension_id": "request.answer",
                "placement": "direct_answer",
                "kind": "inference",
                "analysis_role": "conclusion",
                "text": "The two reported values differ by 20 units.",
                "evidence_refs": [],
                "basis_claim_ids": ["first-value", "second-value"],
                "derivation": {
                    "method": "100 - 80 = 20",
                    "input_claim_ids": ["first-value", "second-value"]
                }
            }
        ],
        "relations": [{
            "id": "value-conflict",
            "dimension_id": "request.answer",
            "kind": "contradicts",
            "claim_ids": ["first-value", "second-value"]
        }],
        "gaps": []
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "compare the records",
        "2026-07-24",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("typed graph report");

    assert_eq!(report.accepted_claim_count, 3);
    assert_eq!(report.accepted_relation_count, 1);
    assert_eq!(report.accepted_derivation_count, 1);
    assert_eq!(report.accepted_basis_edge_count, 2);
    assert_eq!(report.analytical_claim_count, 1);
    assert_eq!(report.cross_source_synthesis_count, 0);
    assert_eq!(report.accepted_gap_count, 0);
    assert_eq!(report.cited_source_count, 2);
    assert!(report.markdown.contains("Contradiction"));
    assert!(report.markdown.contains("Basis"));
    assert!(report.markdown.contains("100 - 80 = 20"));
}

#[test]
fn unrelated_reader_vocabulary_does_not_change_typed_admission_shape() {
    fn admit(query: &str, title: &str, source_text: &str, claim_text: &str) -> (usize, usize) {
        let context = one_track_context();
        let catalog = catalog(vec![source(
            "source-1",
            title,
            "https://example.test/opaque",
            source_text,
        )]);
        let proposal = serde_json::json!({
            "report_language": "und",
            "labels": labels(),
            "claims": [fact("answer", "direct_answer", claim_text, "source-1")],
            "relations": [],
            "gaps": []
        });
        let report = admit_deep_research_typed_report_proposal_at(
            query,
            "2026-07-24",
            &catalog,
            &context,
            with_narrative(proposal, &context),
        )
        .expect("typed admission")
        .expect("isomorphic report");
        (report.accepted_claim_count, report.cited_source_count)
    }

    assert_eq!(
        admit(
            "Compare storage boundaries",
            "Storage record",
            "The record establishes the bounded storage behavior.",
            "The bounded storage behavior is established.",
        ),
        admit(
            "核查公共事件",
            "公共记录",
            "该记录明确说明了事件的有限状态。",
            "该事件的有限状态已经由记录明确说明。",
        )
    );
}
