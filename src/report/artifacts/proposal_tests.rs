use super::*;

fn report_context(scope: DeepResearchReportScope) -> DeepResearchReportContext {
    DeepResearchReportContext {
        report_title: "Research report".to_string(),
        scope,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.primary",
            "title": "Primary evidence",
            "focus": "Establish the requested answer from traceable evidence.",
            "material": true,
            "completion_criteria": ["The answer and its support are both explicit."],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false,
            },
        })],
    }
}

fn report_proposal(mut value: serde_json::Value) -> serde_json::Value {
    value
        .as_object_mut()
        .expect("report proposal fixture")
        .insert(
            "labels".to_string(),
            serde_json::json!({
                "answer": "Direct Answer",
                "findings": "Findings",
                "recommendations": "Evidence-Based Recommendations",
                "boundary": "Evidence Boundary",
                "limitations": "Limitations",
                "evidence_boundary": "This report publishes no conclusion beyond the fetched evidence.",
                "sources": "Sources"
            }),
        );
    value
}

#[test]
fn proposal_schema_keeps_model_output_block_only() {
    let schema = deep_research_report_proposal_schema();
    let encoded = schema.to_string();

    assert!(encoded.contains("\"summary\""));
    assert!(encoded.contains("\"findings\""));
    assert!(encoded.contains("\"recommendations\""));
    assert!(encoded.contains("\"limitations\""));
    assert!(encoded.contains("\"source_aliases\""));
    assert!(encoded.contains("\"track_ids\""));
    assert!(encoded.contains("\"labels\""));
    assert!(!encoded.contains("\"url\""));
    assert!(!encoded.contains("\"markdown\""));
}

#[test]
fn proposal_prompt_contains_semantic_scope_tracks_and_no_catalog_anchor() {
    let catalog = focused_catalog();
    let context = report_context(DeepResearchReportScope::Comprehensive);

    let prompt = deep_research_report_proposal_prompt_at(
        "Assess Nimbus support and migration risk",
        "2026-07-23",
        &catalog,
        &context,
    )
    .expect("closed prompt");

    assert!(prompt.contains("\"research_scope\":\"comprehensive\""));
    assert!(prompt.contains("\"research_tracks\""));
    assert!(prompt.contains("\"findings\":4"));
    assert!(prompt.contains("\"supported_claims\":5"));
    assert!(prompt.contains("\"cited_sources\":2"));
    assert!(prompt.contains("\"coverage\""));
    assert!(!prompt.contains("https://docs.rs/nimbus"));
}

#[test]
fn host_builds_fixed_sections_citations_and_ledger_from_valid_blocks() {
    let catalog = focused_catalog();
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("admit proposal")
    .expect("qualified focused report");

    assert!(admitted.markdown.contains("## Direct Answer"));
    assert!(admitted.markdown.contains("## Findings"));
    assert!(admitted.markdown.contains("### Requested answer"));
    assert!(admitted.markdown.contains("## Sources"));
    assert!(admitted.markdown.contains("[[1]]("));
    assert_eq!(admitted.direct_answer_block_count, 1);
    assert_eq!(admitted.finding_block_count, 1);
    assert_eq!(admitted.accepted_claim_count, 2);
    assert_eq!(admitted.cited_source_count, 1);
}

#[test]
fn invalid_blocks_are_removed_without_losing_valid_siblings() {
    let catalog = focused_catalog();
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }, {
            "text": "Nimbus is supported through 2099.",
            "source_aliases": ["source-99"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("admit proposal")
    .expect("valid siblings survive");

    assert!(!admitted.markdown.contains("2099"));
    assert_eq!(admitted.accepted_claim_count, 2);
    assert_eq!(admitted.rejected_block_count, 1);
}

#[test]
fn comprehensive_scope_rejects_a_shallow_single_fact_report() {
    let catalog = comprehensive_catalog();
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "The Aurora program entered public operation in July 2026.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The official release records the July 2026 public operation milestone.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal_at(
        "Provide a complete assessment of the Aurora program",
        "2026-07-23",
        &catalog,
        &report_context(DeepResearchReportScope::Comprehensive),
        proposal,
    )
    .expect("evaluate comprehensive proposal");

    assert!(admitted.is_none());
}

#[test]
fn recommendation_padding_cannot_satisfy_comprehensive_depth() {
    let catalog = comprehensive_catalog();
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "The Aurora program entered public operation in July 2026.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The official release records the July 2026 public operation milestone.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [{
            "text": "Organizations should review the July 2026 release before adoption and document every operational dependency in detail.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }, {
            "text": "Teams should conduct extensive planning, validation, monitoring, training, governance, and contingency exercises before migration.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal_at(
        "Provide a complete assessment of the Aurora program",
        "2026-07-23",
        &catalog,
        &report_context(DeepResearchReportScope::Comprehensive),
        proposal,
    )
    .expect("evaluate padded proposal");

    assert!(admitted.is_none());
}

#[test]
fn web_source_without_semantic_provenance_cannot_pass_the_strong_support_gate() {
    let catalog = DeepResearchSourceCatalog {
        sources: vec![DeepResearchCatalogSource {
            alias: "source-1".to_string(),
            title: "Nimbus support note".to_string(),
            anchor: "https://unknown.example/nimbus".to_string(),
            chunks: vec![
                "Nimbus version 2 receives fixes through September 2027. The support record identifies version 2 and September 2027 as the maintenance boundary."
                    .to_string(),
            ],
            claim_eligible: true,
            semantically_admitted: false,
            coverage: Vec::new(),
        }],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("evaluate source without semantic provenance");

    assert!(admitted.is_none());
}

#[test]
fn semantic_admission_is_not_replaced_by_a_publisher_allowlist() {
    let mut catalog = focused_catalog();
    catalog.sources[0].anchor = "https://research.example/nimbus".to_string();
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    }));

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("evaluate semantically admitted publisher");

    assert!(
        admitted.is_some(),
        "closed semantic admission must remain authoritative"
    );
}

#[test]
fn comprehensive_track_gate_requires_criteria_and_declared_source_roles() {
    let context = DeepResearchReportContext {
        report_title: "Research report".to_string(),
        scope: DeepResearchReportScope::Comprehensive,
        freshness_required: false,
        tracks: vec![
            serde_json::json!({
                "id": "deployment.boundary",
                "title": "Deployment boundary",
                "focus": "Establish the deployment boundary.",
                "material": true,
                "completion_criteria": ["Boundary is explicit.", "Timing is explicit."],
                "evidence_requirements": {
                    "primary_source_required": true,
                    "independent_corroboration_required": true,
                },
            }),
            serde_json::json!({
                "id": "operational.risk",
                "title": "Operational risk",
                "focus": "Establish material operational risks.",
                "material": true,
                "completion_criteria": ["A material risk is explicit."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
        ],
    };
    let catalog = DeepResearchSourceCatalog {
        sources: vec![
            coverage_source("source-1", "deployment.boundary", &[0, 1], true, false),
            coverage_source("source-2", "deployment.boundary", &[0, 1], false, true),
            coverage_source("source-3", "operational.risk", &[0], false, false),
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    let deployment = AdmittedReportBlock {
        text: "Deployment evidence".to_string(),
        source_indexes: vec![0, 1],
        track_ids: vec!["deployment.boundary".to_string()],
    };
    let risk = AdmittedReportBlock {
        text: "Risk evidence".to_string(),
        source_indexes: vec![2],
        track_ids: vec!["operational.risk".to_string()],
    };

    assert!(report_material_tracks_have_closed_coverage(
        &context,
        &catalog,
        &[deployment.clone(), risk]
    ));
    assert!(!report_material_tracks_have_closed_coverage(
        &context,
        &catalog,
        std::slice::from_ref(&deployment)
    ));
    let without_independent = AdmittedReportBlock {
        source_indexes: vec![0],
        ..deployment
    };
    assert!(!report_material_tracks_have_closed_coverage(
        &context,
        &catalog,
        &[
            without_independent,
            AdmittedReportBlock {
                text: "Risk evidence".to_string(),
                source_indexes: vec![2],
                track_ids: vec!["operational.risk".to_string()],
            },
        ]
    ));
    let conflated_roles = DeepResearchSourceCatalog {
        sources: vec![
            coverage_source("source-1", "deployment.boundary", &[0, 1], true, true),
            coverage_source("source-2", "deployment.boundary", &[0, 1], false, false),
            coverage_source("source-3", "operational.risk", &[0], false, false),
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    assert!(!report_material_tracks_have_closed_coverage(
        &context,
        &conflated_roles,
        &[
            AdmittedReportBlock {
                text: "Deployment evidence".to_string(),
                source_indexes: vec![0, 1],
                track_ids: vec!["deployment.boundary".to_string()],
            },
            AdmittedReportBlock {
                text: "Risk evidence".to_string(),
                source_indexes: vec![2],
                track_ids: vec!["operational.risk".to_string()],
            },
        ],
    ));
}

#[test]
fn report_admission_is_isomorphic_across_unrelated_content() {
    fn admit(
        title: &str,
        source_text: &str,
        summary: &str,
        finding: &str,
        labels: serde_json::Value,
    ) -> AdmittedDeepResearchReport {
        let catalog = DeepResearchSourceCatalog {
            sources: vec![DeepResearchCatalogSource {
                alias: "source-1".to_string(),
                title: title.to_string(),
                anchor: "https://example.test/record".to_string(),
                chunks: vec![source_text.to_string()],
                claim_eligible: true,
                semantically_admitted: true,
                coverage: Vec::new(),
            }],
            omitted_source_count: 0,
            omitted_chunk_count: 0,
        };
        let mut proposal = serde_json::json!({
            "summary": [{
                "text": summary,
                "source_aliases": ["source-1"],
                "track_ids": ["request.primary"]
            }],
            "findings": [{
                "text": finding,
                "source_aliases": ["source-1"],
                "track_ids": ["request.primary"]
            }],
            "recommendations": [],
            "limitations": []
        });
        proposal["labels"] = labels;
        admit_deep_research_report_proposal("untrusted query", &catalog, proposal)
            .expect("structural admission")
            .expect("isomorphic focused report")
    }

    let first = admit(
        "Material record",
        "The retained record establishes the observed material state. The same record describes its bounded implication.",
        "The retained record establishes the observed material state.",
        "The same record describes its bounded implication.",
        serde_json::json!({
            "answer": "Answer",
            "findings": "Findings",
            "recommendations": "Recommendations",
            "boundary": "Boundary",
            "limitations": "Limitations",
            "evidence_boundary": "No conclusion is published beyond the fetched evidence.",
            "sources": "Sources"
        }),
    );
    let second = admit(
        "观察记录",
        "保留的记录明确说明了已观察状态。同一记录也说明了它的有限影响。",
        "保留的记录明确说明了已观察状态。",
        "同一记录也说明了它的有限影响。",
        serde_json::json!({
            "answer": "回答",
            "findings": "发现",
            "recommendations": "建议",
            "boundary": "边界",
            "limitations": "限制",
            "evidence_boundary": "报告不会发布超出已获取证据的结论。",
            "sources": "来源"
        }),
    );

    assert_eq!(
        (
            first.direct_answer_block_count,
            first.finding_block_count,
            first.accepted_claim_count,
            first.cited_source_count,
        ),
        (
            second.direct_answer_block_count,
            second.finding_block_count,
            second.accepted_claim_count,
            second.cited_source_count,
        )
    );
}

fn focused_catalog() -> DeepResearchSourceCatalog {
    DeepResearchSourceCatalog {
        sources: vec![DeepResearchCatalogSource {
            alias: "source-1".to_string(),
            title: "Official Nimbus support record".to_string(),
            anchor: "https://docs.rs/nimbus/latest/nimbus/support".to_string(),
            chunks: vec![
                "Nimbus version 2 receives fixes through September 2027. The support record identifies version 2 and September 2027 as the maintenance boundary."
                    .to_string(),
            ],
            claim_eligible: true,
            semantically_admitted: true,
            coverage: Vec::new(),
        }],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    }
}

fn comprehensive_catalog() -> DeepResearchSourceCatalog {
    DeepResearchSourceCatalog {
        sources: vec![
            DeepResearchCatalogSource {
                alias: "source-1".to_string(),
                title: "Aurora official release".to_string(),
                anchor: "https://docs.rs/aurora/latest/aurora/release".to_string(),
                chunks: vec![
                    "The Aurora program entered public operation in July 2026. The official release records the July 2026 public operation milestone."
                        .to_string(),
                ],
                claim_eligible: true,
                semantically_admitted: true,
                coverage: vec![DeepResearchSourceCoverage {
                    track_id: "request.primary".to_string(),
                    completion_criterion_indexes: vec![0],
                    primary: false,
                    independent: false,
                }],
            },
            DeepResearchCatalogSource {
                alias: "source-2".to_string(),
                title: "Independent Aurora assessment".to_string(),
                anchor: "https://www.reuters.com/technology/aurora-assessment".to_string(),
                chunks: vec![
                    "The independent assessment documents Aurora deployment constraints, operating costs, adoption patterns, and unresolved implementation risks."
                        .to_string(),
                ],
                claim_eligible: true,
                semantically_admitted: true,
                coverage: vec![DeepResearchSourceCoverage {
                    track_id: "request.primary".to_string(),
                    completion_criterion_indexes: vec![0],
                    primary: false,
                    independent: false,
                }],
            },
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    }
}

fn coverage_source(
    alias: &str,
    track_id: &str,
    completion_criterion_indexes: &[usize],
    primary: bool,
    independent: bool,
) -> DeepResearchCatalogSource {
    DeepResearchCatalogSource {
        alias: alias.to_string(),
        title: format!("{alias} record"),
        anchor: format!("https://{alias}.example/research"),
        chunks: vec!["Closed evidence text.".to_string()],
        claim_eligible: true,
        semantically_admitted: true,
        coverage: vec![DeepResearchSourceCoverage {
            track_id: track_id.to_string(),
            completion_criterion_indexes: completion_criterion_indexes.to_vec(),
            primary,
            independent,
        }],
    }
}
