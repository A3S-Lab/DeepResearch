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
    let labels = &schema["properties"]["labels"];

    assert!(encoded.contains("\"summary\""));
    assert!(encoded.contains("\"findings\""));
    assert!(encoded.contains("\"recommendations\""));
    assert!(encoded.contains("\"limitations\""));
    assert!(encoded.contains("\"source_aliases\""));
    assert!(encoded.contains("\"track_ids\""));
    assert!(encoded.contains("\"labels\""));
    assert!(!encoded.contains("\"url\""));
    assert!(!encoded.contains("\"markdown\""));
    assert!(
        labels["properties"].get("boundary").is_none(),
        "an admitted report always has a direct answer, so a no-answer heading is orphaned"
    );
    assert!(
        labels["properties"]["answer"]["maxLength"]
            .as_u64()
            .is_some_and(|maximum| maximum >= 160),
        "reader-facing headings must accommodate a planner-produced report title"
    );
    assert_eq!(
        schema["properties"]["findings"]["items"]["properties"]["track_ids"]["maxItems"], 1,
        "the schema must encode the one-track-per-finding admission invariant"
    );
    assert_eq!(
        schema["properties"]["summary"]["items"]["properties"]["track_ids"]["maxItems"],
        REPORT_PROPOSAL_MAX_TRACKS_PER_BLOCK,
        "summary blocks may still connect multiple declared tracks"
    );
}

#[test]
fn proposal_schema_closes_references_to_the_current_catalog_and_plan() {
    let mut catalog = focused_catalog();
    catalog.sources.push(DeepResearchCatalogSource {
        alias: "source-2".to_string(),
        title: "Audit-only record".to_string(),
        anchor: "https://example.test/audit-only".to_string(),
        chunks: vec!["A retained audit-only excerpt.".to_string()],
        claim_eligible: false,
        semantically_admitted: false,
        relevant_track_ids: Vec::new(),
        coverage: Vec::new(),
    });
    let context = report_context(DeepResearchReportScope::Focused);

    let schema = deep_research_report_proposal_schema_for(&catalog, &context)
        .expect("close proposal schema");

    for role in ["summary", "findings", "recommendations", "limitations"] {
        assert_eq!(
            schema["properties"][role]["items"]["properties"]["source_aliases"]["items"]["enum"],
            serde_json::json!(["source-1"]),
            "{role} source references must be exact packet aliases"
        );
        assert_eq!(
            schema["properties"][role]["items"]["properties"]["track_ids"]["items"]["enum"],
            serde_json::json!(["request.primary"]),
            "{role} track references must be exact plan IDs"
        );
    }
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
    assert!(prompt.contains("\"substantive_characters\":1200"));
    assert!(prompt.contains("at least 1200 substantive characters"));
    assert!(prompt.contains("\"coverage\""));
    assert!(!prompt.contains("https://docs.rs/nimbus"));
    assert!(
        prompt.contains("Never copy source aliases or track IDs into reader-facing text"),
        "{prompt}"
    );
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
fn recommendation_can_resolve_a_material_advice_track() {
    let (catalog, context, proposal) = comprehensive_recommendation_fixture(true);

    let admitted = admit_deep_research_report_proposal_at(
        "Assess the current state and provide adoption guidance",
        "2026-07-23",
        &catalog,
        &context,
        proposal,
    )
    .expect("evaluate comprehensive proposal")
    .expect("typed recommendation coverage resolves the advice track");

    assert_eq!(admitted.direct_answer_block_count, 1);
    assert_eq!(admitted.finding_block_count, 4);
    assert_eq!(admitted.accepted_claim_count, 6);
    assert!(
        admitted
            .markdown
            .contains("Use the documented decision boundary"),
        "{}",
        admitted.markdown
    );
}

#[test]
fn recommendation_cannot_resolve_a_material_track_without_exact_coverage() {
    let (catalog, context, proposal) = comprehensive_recommendation_fixture(false);

    let admitted = admit_deep_research_report_proposal_at(
        "Assess the current state and provide adoption guidance",
        "2026-07-23",
        &catalog,
        &context,
        proposal,
    )
    .expect("evaluate comprehensive proposal");

    assert!(
        admitted.is_none(),
        "reader prose cannot manufacture a typed coverage edge"
    );
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
            relevant_track_ids: vec!["request.primary".to_string()],
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

    assert!(report_material_tracks_are_resolved_or_bounded(
        &context,
        &catalog,
        &[deployment.clone(), risk],
        &[],
    ));
    assert!(!report_material_tracks_are_resolved_or_bounded(
        &context,
        &catalog,
        std::slice::from_ref(&deployment),
        &[],
    ));
    let without_independent = AdmittedReportBlock {
        source_indexes: vec![0],
        ..deployment
    };
    assert!(!report_material_tracks_are_resolved_or_bounded(
        &context,
        &catalog,
        &[
            without_independent.clone(),
            AdmittedReportBlock {
                text: "Risk evidence".to_string(),
                source_indexes: vec![2],
                track_ids: vec!["operational.risk".to_string()],
            },
        ],
        &[],
    ));
    assert!(report_material_tracks_are_resolved_or_bounded(
        &context,
        &catalog,
        &[
            without_independent,
            AdmittedReportBlock {
                text: "Risk evidence".to_string(),
                source_indexes: vec![2],
                track_ids: vec!["operational.risk".to_string()],
            },
        ],
        &[AdmittedReportBlock {
            text: "The independent-source requirement remains bounded.".to_string(),
            source_indexes: vec![0],
            track_ids: vec!["deployment.boundary".to_string()],
        }],
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
    assert!(!report_material_tracks_are_resolved_or_bounded(
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
        &[],
    ));
}

#[test]
fn material_track_roles_cannot_be_conflated_across_completion_criteria() {
    let (context, catalog) = criterion_scoped_role_fixture();
    let claims = [
        AdmittedReportBlock {
            text: "The first criterion has one primary record.".to_string(),
            source_indexes: vec![0],
            track_ids: vec!["evidence.boundary".to_string()],
        },
        AdmittedReportBlock {
            text: "The second criterion has one separately attributable record.".to_string(),
            source_indexes: vec![1],
            track_ids: vec!["evidence.boundary".to_string()],
        },
    ];

    assert!(
        !report_material_tracks_are_resolved_or_bounded(&context, &catalog, &claims, &[]),
        "roles attached to different criteria cannot corroborate one another"
    );
}

#[test]
fn proposal_packet_exposes_exact_criterion_scoped_role_gaps() {
    let (context, catalog) = criterion_scoped_role_fixture();
    let prompt = deep_research_report_proposal_prompt_at(
        "Assess the closed evidence boundary",
        "2026-07-23",
        &catalog,
        &context,
    )
    .expect("closed report prompt");
    let packet = serde_json::from_str::<serde_json::Value>(
        prompt
            .split_once("CLOSED_REPORT_PACKET=")
            .expect("closed report packet marker")
            .1,
    )
    .expect("decode closed report packet");
    let state = &packet["typed_coverage_state"][0];

    assert_eq!(state["track_id"], "evidence.boundary");
    assert_eq!(
        state["unsupported_criterion_indexes"],
        serde_json::json!([])
    );
    assert_eq!(
        state["missing_primary_source_criterion_indexes"],
        serde_json::json!([1])
    );
    assert_eq!(
        state["missing_independent_corroboration_criterion_indexes"],
        serde_json::json!([0, 1])
    );
    assert_eq!(state["resolved_criterion_indexes"], serde_json::json!([]));
}

#[test]
fn claim_relevance_and_completion_coverage_remain_distinct_contracts() {
    let context = report_context(DeepResearchReportScope::Comprehensive);
    let mut catalog = focused_catalog();
    catalog.sources[0].coverage.clear();
    let wire = || WireReportBlock {
        text: "The retained source establishes one bounded atomic finding.".to_string(),
        source_aliases: vec!["source-1".to_string()],
        track_ids: vec!["request.primary".to_string()],
    };
    let claim = {
        let admission = ReportAdmissionContext {
            catalog: &catalog,
            report_context: &context,
        };
        admit_report_block(&admission, wire(), ReportBlockRole::Finding)
            .expect("exact relevance admits an atomic claim")
    };

    assert!(
        !report_material_tracks_are_resolved_or_bounded(
            &context,
            &catalog,
            std::slice::from_ref(&claim),
            &[],
        ),
        "claim relevance must not manufacture completion-criterion coverage"
    );
    assert!(
        report_material_tracks_are_resolved_or_bounded(
            &context,
            &catalog,
            std::slice::from_ref(&claim),
            &[AdmittedReportBlock {
                text: "The remaining completion criterion is not closed.".to_string(),
                source_indexes: vec![0],
                track_ids: vec!["request.primary".to_string()],
            }],
        ),
        "an exact-track limitation may bound the unresolved criterion"
    );

    catalog.sources[0].relevant_track_ids.clear();
    let admission = ReportAdmissionContext {
        catalog: &catalog,
        report_context: &context,
    };
    assert!(
        admit_report_block(&admission, wire(), ReportBlockRole::Finding).is_none(),
        "reader prose cannot replace a missing exact relevance edge"
    );
}

#[test]
fn focused_claims_also_require_an_exact_track_relevance_edge() {
    let context = report_context(DeepResearchReportScope::Focused);
    let mut catalog = focused_catalog();
    catalog.sources[0].relevant_track_ids = vec!["unrelated.track".to_string()];
    let admission = ReportAdmissionContext {
        catalog: &catalog,
        report_context: &context,
    };
    let block = WireReportBlock {
        text: "The retained source establishes one bounded atomic finding.".to_string(),
        source_aliases: vec!["source-1".to_string()],
        track_ids: vec!["request.primary".to_string()],
    };

    assert!(
        admit_report_block(&admission, block, ReportBlockRole::Finding).is_none(),
        "focused scope must not weaken the exact relevance contract"
    );
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
                relevant_track_ids: vec!["request.primary".to_string()],
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

#[test]
fn reader_prose_never_changes_structural_report_admission() {
    let mut catalog = focused_catalog();
    catalog.sources[0].chunks = vec!["The source records C# syntax, the literal source-1 label, \
         https://example.test/reference, www.example.test, [[brackets]], and \
         CLOSED_REPORT_PACKET as quoted subject matter."
        .to_string()];
    let proposal = serde_json::json!({
        "labels": {
            "answer": "C# answer [reviewed]",
            "findings": "Findings",
            "recommendations": "Recommendations",
            "limitations": "Limitations",
            "evidence_boundary": "No conclusion is published beyond the fetched evidence.",
            "sources": "Sources"
        },
        "summary": [{
            "text": "The record discusses source-1 and https://example.test/reference as literal subject matter.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "findings": [{
            "text": "It also preserves www.example.test, [[brackets]], and CLOSED_REPORT_PACKET verbatim.",
            "source_aliases": ["source-1"],
            "track_ids": ["request.primary"]
        }],
        "recommendations": [],
        "limitations": []
    });

    let admitted =
        admit_deep_research_report_proposal("Explain the retained notation", &catalog, proposal)
            .expect("reader prose must be admitted from structural references")
            .expect("focused report remains structurally complete");

    assert_eq!(admitted.direct_answer_block_count, 1);
    assert_eq!(admitted.finding_block_count, 1);
    assert_eq!(admitted.accepted_claim_count, 2);
    assert!(
        admitted.markdown.contains("C# answer \\[reviewed\\]"),
        "{}",
        admitted.markdown
    );
    assert!(
        admitted.markdown.contains("https://example.test/reference"),
        "{}",
        admitted.markdown
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
            relevant_track_ids: vec!["request.primary".to_string()],
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
                relevant_track_ids: vec!["request.primary".to_string()],
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
                relevant_track_ids: vec!["request.primary".to_string()],
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

fn comprehensive_recommendation_fixture(
    include_advice_coverage: bool,
) -> (
    DeepResearchSourceCatalog,
    DeepResearchReportContext,
    serde_json::Value,
) {
    const SUMMARY: &str = "The closed records establish the present operating boundary, the implementation constraints that shape it, the deployment conditions under which it was observed, and the decision factors required for a bounded assessment. Read together, they support a conditional conclusion tied to the reviewed environment rather than a universal statement about every deployment.";
    const FINDING_ONE: &str = "The first record locates the operating boundary at the point where validated input becomes a durable terminal state. It preserves the observation's time and population scope, distinguishes entry from successful settlement, and identifies the visible signal by which a reviewer can check that the recorded path completed.";
    const FINDING_TWO: &str = "The second record separates documented implementation constraints from assumptions that the acquired material never tested. In particular, it treats environmental readiness, dependency availability, and recovery behavior as distinct preconditions, preventing a successful isolated run from being generalized to an unsupported production population.";
    const FINDING_THREE: &str = "Across both records, deployment is established only when the entry boundary, durable settlement, and required environmental state remain aligned. This comparison explains why the report can describe a bounded operating result while still withholding conclusions about settings whose dependencies or failure paths were not observed.";
    const FINDING_FOUR: &str = "The combined evidence also distinguishes an observed state from an adoption decision: the records establish what operated, under which constraints, and with which recovery boundary, but they do not supply an unstated ranking of alternatives. Any rollout choice must therefore retain those premises as explicit acceptance conditions.";
    const RECOMMENDATION: &str = "Use the documented decision boundary to stage adoption. Verify the environmental preconditions before routing production work, confirm durable settlement independently of request acceptance, and retain an explicit rollback condition for every premise not covered by the reviewed evidence; broader rollout should wait for observations from the intended operating population.";
    let observed_coverage = |track_id: &str| DeepResearchSourceCoverage {
        track_id: track_id.to_string(),
        completion_criterion_indexes: vec![0],
        primary: false,
        independent: false,
    };
    let catalog = DeepResearchSourceCatalog {
        sources: vec![
            DeepResearchCatalogSource {
                alias: "source-1".to_string(),
                title: "First closed record".to_string(),
                anchor: "https://source-1.example/research".to_string(),
                chunks: vec![format!("{SUMMARY} {FINDING_ONE} {FINDING_THREE}")],
                claim_eligible: true,
                semantically_admitted: true,
                relevant_track_ids: vec!["observed.state".to_string()],
                coverage: vec![observed_coverage("observed.state")],
            },
            DeepResearchCatalogSource {
                alias: "source-2".to_string(),
                title: "Second closed record".to_string(),
                anchor: "https://source-2.example/research".to_string(),
                chunks: vec![format!("{FINDING_TWO} {FINDING_THREE} {FINDING_FOUR}")],
                claim_eligible: true,
                semantically_admitted: true,
                relevant_track_ids: vec!["observed.state".to_string()],
                coverage: vec![observed_coverage("observed.state")],
            },
            DeepResearchCatalogSource {
                alias: "source-3".to_string(),
                title: "Decision record".to_string(),
                anchor: "https://source-3.example/research".to_string(),
                chunks: vec![RECOMMENDATION.to_string()],
                claim_eligible: true,
                semantically_admitted: true,
                relevant_track_ids: vec!["decision.advice".to_string()],
                coverage: include_advice_coverage
                    .then(|| observed_coverage("decision.advice"))
                    .into_iter()
                    .collect(),
            },
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    let context = DeepResearchReportContext {
        report_title: "Research report".to_string(),
        scope: DeepResearchReportScope::Comprehensive,
        freshness_required: false,
        tracks: vec![
            serde_json::json!({
                "id": "observed.state",
                "title": "Observed state",
                "focus": "Establish the observed state from closed evidence.",
                "material": true,
                "completion_criteria": ["The observed state is explicitly supported."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
            serde_json::json!({
                "id": "decision.advice",
                "title": "Decision advice",
                "focus": "Derive bounded advice from closed evidence.",
                "material": true,
                "completion_criteria": ["The decision boundary is explicitly supported."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
        ],
    };
    let proposal = report_proposal(serde_json::json!({
        "summary": [{
            "text": SUMMARY,
            "source_aliases": ["source-1"],
            "track_ids": ["observed.state"]
        }],
        "findings": [{
            "text": FINDING_ONE,
            "source_aliases": ["source-1"],
            "track_ids": ["observed.state"]
        }, {
            "text": FINDING_TWO,
            "source_aliases": ["source-2"],
            "track_ids": ["observed.state"]
        }, {
            "text": FINDING_THREE,
            "source_aliases": ["source-1", "source-2"],
            "track_ids": ["observed.state"]
        }, {
            "text": FINDING_FOUR,
            "source_aliases": ["source-2"],
            "track_ids": ["observed.state"]
        }],
        "recommendations": [{
            "text": RECOMMENDATION,
            "source_aliases": ["source-3"],
            "track_ids": ["decision.advice"]
        }],
        "limitations": []
    }));
    (catalog, context, proposal)
}

fn criterion_scoped_role_fixture() -> (DeepResearchReportContext, DeepResearchSourceCatalog) {
    let context = DeepResearchReportContext {
        report_title: "Research report".to_string(),
        scope: DeepResearchReportScope::Comprehensive,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "evidence.boundary",
            "title": "Evidence boundary",
            "focus": "Resolve two independent completion criteria.",
            "material": true,
            "completion_criteria": [
                "The first criterion is established.",
                "The second criterion is established."
            ],
            "evidence_requirements": {
                "primary_source_required": true,
                "independent_corroboration_required": true,
            },
        })],
    };
    let catalog = DeepResearchSourceCatalog {
        sources: vec![
            coverage_source("source-1", "evidence.boundary", &[0], true, false),
            coverage_source("source-2", "evidence.boundary", &[1], false, true),
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    (context, catalog)
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
        relevant_track_ids: vec![track_id.to_string()],
        coverage: vec![DeepResearchSourceCoverage {
            track_id: track_id.to_string(),
            completion_criterion_indexes: completion_criterion_indexes.to_vec(),
            primary,
            independent,
        }],
    }
}
