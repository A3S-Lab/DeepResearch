use super::*;

fn labels() -> serde_json::Value {
    serde_json::json!({
        "answer": "Direct Answer",
        "findings": "Findings",
        "recommendations": "Recommendation",
        "limitations": "Limitations",
        "evidence_boundary": "No conclusion is published beyond the fetched evidence.",
        "sources": "Sources",
        "contradiction": "Contradiction",
        "inference": "Inference",
        "basis": "Basis",
        "derivation": "Derivation"
    })
}

fn one_track_context() -> DeepResearchReportContext {
    DeepResearchReportContext {
        report_title: "Typed research report".to_string(),
        scope: DeepResearchReportScope::Focused,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.answer",
            "title": "Requested answer",
            "focus": "Establish the requested answer from the closed evidence.",
            "material": true,
            "completion_criteria": ["The requested answer is directly established."],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false,
            },
        })],
    }
}

fn source(alias: &str, title: &str, anchor: &str, text: &str) -> DeepResearchCatalogSource {
    DeepResearchCatalogSource {
        alias: alias.to_string(),
        title: title.to_string(),
        anchor: anchor.to_string(),
        chunks: vec![text.to_string()],
        claim_eligible: true,
        semantically_admitted: true,
        relevant_track_ids: vec!["request.answer".to_string()],
        coverage: vec![DeepResearchSourceCoverage {
            track_id: "request.answer".to_string(),
            completion_criterion_indexes: vec![0],
            primary: false,
            independent: false,
        }],
    }
}

fn catalog(sources: Vec<DeepResearchCatalogSource>) -> DeepResearchSourceCatalog {
    DeepResearchSourceCatalog {
        sources,
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    }
}

fn fact(id: &str, placement: &str, text: &str, source_id: &str) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "dimension_id": "request.answer",
        "placement": placement,
        "kind": "fact",
        "text": text,
        "evidence_refs": [{
            "source_id": source_id,
            "chunk_ids": [format!("{source_id}:chunk:1")]
        }],
        "basis_claim_ids": [],
        "derivation": null
    })
}

#[test]
fn active_typed_schema_closes_source_chunk_and_dimension_ids() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The closed record establishes the requested answer.",
    )]);

    let schema =
        deep_research_typed_report_proposal_schema_for(&catalog, &context).expect("typed schema");
    let claim = &schema["properties"]["claims"]["items"]["properties"];

    assert_eq!(
        claim["dimension_id"]["enum"],
        serde_json::json!(["request.answer"])
    );
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["source_id"]["enum"],
        serde_json::json!(["source-1"])
    );
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["chunk_ids"]["items"]["enum"],
        serde_json::json!(["source-1:chunk:1"])
    );
    assert!(schema["properties"].get("relations").is_some());
    assert!(claim.get("basis_claim_ids").is_some());
    assert!(claim.get("derivation").is_some());
}

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
        proposal,
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
    let workspace = tempfile::tempdir().expect("typed report workspace");
    let artifacts =
        materialize_deep_research_admitted_report(workspace.path(), "focused query", &report)
            .expect("materialize typed report");
    assert!(completed_research_report_artifacts(&artifacts));
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
        proposal,
    )
    .expect("typed admission")
    .expect("typed graph report");

    assert_eq!(report.accepted_claim_count, 3);
    assert_eq!(report.accepted_relation_count, 1);
    assert_eq!(report.accepted_derivation_count, 1);
    assert_eq!(report.accepted_basis_edge_count, 2);
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
            proposal,
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
