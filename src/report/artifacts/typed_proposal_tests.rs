use super::*;

#[test]
fn proposal_narrative_dependency_normalization_is_stable_and_local() {
    let claims = vec![
        serde_json::json!({"id": "evidence", "basis_claim_ids": []}),
        serde_json::json!({"id": "comparison", "basis_claim_ids": ["evidence"]}),
        serde_json::json!({
            "id": "recommendation",
            "basis_claim_ids": ["comparison", "boundary"]
        }),
        serde_json::json!({"id": "boundary", "basis_claim_ids": ["evidence"]}),
    ];
    let mut narrative = serde_json::from_value::<TypedWireNarrativePlan>(serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "A stable argument",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["evidence"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison"]
            }, {
                "purpose": "implication",
                "claim_ids": ["recommendation"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    }))
    .unwrap();

    normalize_typed_narrative_dependency_order(&claims, &mut narrative);

    assert_eq!(
        narrative.sections[0]
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.claim_ids.iter().map(String::as_str))
            .collect::<Vec<_>>(),
        ["evidence", "comparison", "boundary", "recommendation"]
    );
}

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

fn chinese_labels() -> serde_json::Value {
    serde_json::json!({
        "answer": "结论",
        "findings": "证据",
        "recommendations": "建议",
        "limitations": "边界与局限",
        "evidence_boundary": "本报告不发布超出已获取证据范围的结论。",
        "sources": "来源",
        "contradiction": "证据冲突",
        "inference": "分析",
        "basis": "分析依据",
        "derivation": "推导过程"
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

fn two_track_comprehensive_context() -> DeepResearchReportContext {
    DeepResearchReportContext {
        report_title: "Comprehensive typed research report".to_string(),
        scope: DeepResearchReportScope::Comprehensive,
        freshness_required: false,
        tracks: vec![
            serde_json::json!({
                "id": "request.first",
                "title": "First dimension",
                "focus": "Establish the first material dimension.",
                "material": true,
                "completion_criteria": ["The first dimension is directly established."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
            serde_json::json!({
                "id": "request.second",
                "title": "Second dimension",
                "focus": "Establish the second material dimension.",
                "material": true,
                "completion_criteria": ["The second dimension is directly established."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
        ],
    }
}

fn source(alias: &str, title: &str, anchor: &str, text: &str) -> DeepResearchCatalogSource {
    source_for_track(alias, title, anchor, text, "request.answer")
}

fn source_for_track(
    alias: &str,
    title: &str,
    anchor: &str,
    text: &str,
    track_id: &str,
) -> DeepResearchCatalogSource {
    DeepResearchCatalogSource {
        alias: alias.to_string(),
        title: title.to_string(),
        anchor: anchor.to_string(),
        chunks: vec![text.to_string()],
        claim_eligible: true,
        semantically_admitted: true,
        relevant_track_ids: vec![track_id.to_string()],
        coverage: vec![DeepResearchSourceCoverage {
            track_id: track_id.to_string(),
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
    fact_for_dimension(id, "request.answer", placement, text, source_id)
}

fn fact_for_dimension(
    id: &str,
    dimension_id: &str,
    placement: &str,
    text: &str,
    source_id: &str,
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "dimension_id": dimension_id,
        "placement": placement,
        "kind": "fact",
        "analysis_role": if placement == "direct_answer" {
            "conclusion"
        } else {
            "evidence"
        },
        "text": text,
        "evidence_refs": [{
            "source_id": source_id,
            "chunk_ids": [format!("{source_id}:chunk:1")]
        }],
        "basis_claim_ids": [],
        "derivation": null
    })
}

fn inference_for_dimension(
    id: &str,
    dimension_id: &str,
    placement: &str,
    analysis_role: &str,
    text: &str,
    basis_claim_ids: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "id": id,
        "dimension_id": dimension_id,
        "placement": placement,
        "kind": "inference",
        "analysis_role": analysis_role,
        "text": text,
        "evidence_refs": [],
        "basis_claim_ids": basis_claim_ids,
        "derivation": null
    })
}

fn with_narrative(
    mut proposal: serde_json::Value,
    context: &DeepResearchReportContext,
) -> serde_json::Value {
    let claims = proposal
        .get("claims")
        .and_then(serde_json::Value::as_array)
        .cloned()
        .unwrap_or_default();
    let sections = context
        .tracks
        .iter()
        .filter_map(|track| {
            let dimension_id = track.get("id")?.as_str()?;
            let heading = track.get("title")?.as_str()?;
            let findings = claims
                .iter()
                .filter(|claim| {
                    claim
                        .get("dimension_id")
                        .and_then(serde_json::Value::as_str)
                        == Some(dimension_id)
                        && claim.get("placement").and_then(serde_json::Value::as_str)
                            == Some("finding")
                })
                .filter_map(|claim| {
                    Some((
                        claim.get("id")?.as_str()?.to_string(),
                        claim.get("kind")?.as_str()?.to_string(),
                        claim.get("analysis_role")?.as_str()?.to_string(),
                    ))
                })
                .collect::<Vec<_>>();
            let paragraphs = findings
                .into_iter()
                .map(|(claim_id, kind, role)| {
                    let purpose = match role.as_str() {
                        "evidence" => "evidence",
                        "comparison" | "explanation" => "synthesis",
                        "implication" => "implication",
                        "challenge" | "boundary" => "boundary",
                        _ if kind == "fact" => "evidence",
                        _ => "synthesis",
                    };
                    serde_json::json!({
                        "purpose": purpose,
                        "claim_ids": [claim_id],
                    })
                })
                .collect::<Vec<_>>();
            Some(serde_json::json!({
                "dimension_id": dimension_id,
                "heading": heading,
                "paragraphs": paragraphs,
            }))
        })
        .collect::<Vec<_>>();
    proposal["narrative"] = serde_json::json!({ "sections": sections });
    proposal
}

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
    .expect("evaluate all-bounded analytical proposal")
    .expect("deeply analyzed bounded conclusion should remain qualified");

    assert_eq!(
        bounded.publication,
        DeepResearchEvidenceFirstPublication::Qualified
    );
    assert_eq!(bounded.resolved_material_dimension_count, 0);
    assert_eq!(bounded.deeply_analyzed_dimension_count, 1);
    assert_eq!(bounded.accepted_gap_count, 1);
    assert_eq!(bounded.direct_answer_block_count, 1);
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

#[test]
fn independent_editorial_planning_can_reorder_only_admitted_claims() {
    const ANSWER: &str =
        "The closed records establish that durable settlement, not request acceptance alone, is the completion boundary.";
    const INTAKE: &str =
        "The intake record shows that a request crosses validation before downstream execution begins.";
    const SETTLEMENT: &str =
        "A separately maintained settlement record shows that the terminal state remains inspectable after request acceptance.";
    const COMPARISON: &str =
        "Read together, the records distinguish an early acceptance signal from the later durable state that proves completion.";
    const EXPLANATION: &str =
        "The distinction matters because validation can succeed before downstream work reaches a recoverable terminal state.";
    const IMPLICATION: &str =
        "Acceptance testing should therefore verify both entry validation and durable settlement as separate observable checkpoints.";
    const BOUNDARY: &str =
        "The conclusion is limited to paths where the acceptance signal and terminal state can be observed independently.";

    let context = one_track_context();
    let catalog = catalog(vec![
        source(
            "source-1",
            "Closed intake record",
            "https://example.test/intake",
            &format!("{ANSWER} {INTAKE}"),
        ),
        source(
            "source-2",
            "Closed settlement record",
            "https://example.test/settlement",
            SETTLEMENT,
        ),
    ]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("intake", "finding", INTAKE, "source-1"),
            fact("settlement", "finding", SETTLEMENT, "source-2"),
            inference_for_dimension(
                "comparison",
                "request.answer",
                "finding",
                "comparison",
                COMPARISON,
                &["intake", "settlement"],
            ),
            inference_for_dimension(
                "explanation",
                "request.answer",
                "finding",
                "explanation",
                EXPLANATION,
                &["comparison"],
            ),
            inference_for_dimension(
                "implication",
                "request.answer",
                "finding",
                "implication",
                IMPLICATION,
                &["explanation"],
            ),
            inference_for_dimension(
                "boundary",
                "request.answer",
                "finding",
                "boundary",
                BOUNDARY,
                &["comparison"],
            ),
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "request.answer",
                "heading": "From acceptance to durable completion",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["intake", "settlement"]
                }, {
                    "purpose": "synthesis",
                    "claim_ids": ["comparison", "explanation"]
                }, {
                    "purpose": "implication",
                    "claim_ids": ["implication"]
                }, {
                    "purpose": "boundary",
                    "claim_ids": ["boundary"]
                }]
            }]
        }
    });
    let draft = admit_deep_research_typed_report_draft_in_language_at(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed draft admission")
    .expect("admitted typed draft");
    let baseline = draft.report.clone();

    let prompt = deep_research_typed_editorial_prompt(&draft).expect("editorial prompt");
    assert!(prompt.contains(INTAKE));
    assert!(prompt.contains(SETTLEMENT));
    assert!(prompt.contains(COMPARISON));
    assert!(!prompt.contains(ANSWER));
    assert!(!prompt.contains("source-1"));
    assert!(!prompt.contains("https://example.test"));
    let schema = deep_research_typed_editorial_schema(&draft);
    assert_eq!(
        schema["properties"]["sections"]["items"]["properties"]["paragraphs"]["items"]
            ["properties"]["claim_ids"]["items"]["enum"],
        serde_json::json!([
            "intake",
            "settlement",
            "comparison",
            "explanation",
            "implication",
            "boundary"
        ])
    );

    let editorial = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "Completion begins after acceptance",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["settlement", "intake"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let edited = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        editorial,
    )
    .expect("valid editorial plan");

    assert!(edited
        .markdown
        .contains("### Completion begins after acceptance"));
    assert!(edited.markdown.find(SETTLEMENT) < edited.markdown.find(INTAKE));
    for claim in [
        ANSWER,
        INTAKE,
        SETTLEMENT,
        COMPARISON,
        EXPLANATION,
        IMPLICATION,
        BOUNDARY,
    ] {
        assert_eq!(edited.markdown.matches(claim).count(), 1, "{claim}");
    }
    assert_eq!(edited.accepted_claim_count, baseline.accepted_claim_count);
    assert_eq!(
        edited.substantive_character_count,
        baseline.substantive_character_count
    );

    let out_of_order = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "An invalid dependency order",
            "paragraphs": [{
                "purpose": "synthesis",
                "claim_ids": ["comparison"]
            }, {
                "purpose": "evidence",
                "claim_ids": ["settlement", "intake"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        out_of_order,
    )
    .expect_err("a dependent claim cannot precede its premises");
    assert!(
        error.contains("supporting premises before dependent claims"),
        "{error}"
    );

    let omitted_claim = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "An incomplete editorial plan",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["intake", "settlement"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        omitted_claim,
    )
    .expect_err("every admitted finding must appear exactly once");
    assert!(
        error.contains("every supporting claim exactly once"),
        "{error}"
    );

    let attempted_rewrite = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "A forbidden rewrite",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["intake", "settlement"],
                "text": "Rewritten prose"
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft,
        attempted_rewrite,
    )
    .expect_err("editorial output cannot carry rewritten prose");
    assert!(error.contains("decode typed editorial plan"), "{error}");
}

#[test]
fn near_duplicate_claim_prose_is_rejected_instead_of_padding_depth() {
    const ANSWER: &str =
        "The closed record establishes a bounded execution result for the reviewed environment.";
    const FIRST: &str = "The retained execution record identifies the validated entry boundary, the owning component, the durable settlement state, and the observable completion signal for the reviewed environment.";
    const SECOND: &str = "The retained execution record identifies the validated entry boundary, the owning component, the durable settlement state, and the observable completion signal for the reviewed environment.";
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed execution record",
        "https://example.test/execution",
        &format!("{ANSWER} {FIRST} {SECOND}"),
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("first", "finding", FIRST, "source-1"),
            fact("second", "finding", SECOND, "source-1"),
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "request.answer",
                "heading": "The reviewed execution boundary",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["first", "second"]
                }]
            }]
        }
    });

    let error = admit_deep_research_typed_report_proposal_at(
        "trace the execution boundary",
        "2026-07-24",
        &catalog,
        &context,
        proposal,
    )
    .expect_err("near-duplicate prose must fail closed");
    assert!(error.contains("near-duplicate claim prose"), "{error}");
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
    assert!(schema["properties"].get("narrative").is_some());
    assert!(claim.get("basis_claim_ids").is_some());
    assert!(claim.get("derivation").is_some());
}

#[test]
fn typed_report_contract_defines_labels_as_short_structural_headings() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The closed record establishes the requested answer.",
    )]);

    let schema =
        deep_research_typed_report_proposal_schema_for(&catalog, &context).expect("typed schema");
    assert_eq!(
        schema["properties"]["labels"]["properties"]["answer"]["description"],
        "A short section heading, never an answer, claim, or sentence."
    );
    assert_eq!(
        schema["properties"]["claims"]["description"],
        "A bounded claim graph with at most 32 claims total."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["evidence_refs"]["description"],
        "Use at most one entry per source_id and put every cited chunk from that source in the same chunk_ids array."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["evidence_refs"]["maxItems"],
        serde_json::json!(4)
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["placement"]["description"],
        "Use direct_answer for the leading conclusion claim in a resolved dimension. When every material dimension is bounded, one deeply supported dimension may instead carry one explicitly qualified partial conclusion. Use finding for supporting detail."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["text"]["description"],
        "Reader-facing claim prose. Put opaque workflow, dimension, source, chunk, query, target, and criterion IDs only in their typed fields."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["derivation"]["description"],
        "Use only for a reproducible inference. A recommendation must set derivation to null and express its rationale through basis_claim_ids."
    );
    assert_eq!(
        schema["properties"]["relations"]["items"]["properties"]["kind"]["description"],
        "Use only when two facts give mutually incompatible answers to the same proposition under the same scope and time."
    );
    assert_eq!(
        schema["properties"]["gaps"]["items"]["properties"]["text"]["description"],
        "A reader-facing evidence limitation stated in natural language without opaque internal IDs or workflow diagnostics."
    );
    assert_eq!(
        schema["properties"]["labels"]["properties"]["evidence_boundary"]["description"],
        "One concise evidence-boundary sentence with at most 360 characters."
    );

    let prompt = deep_research_typed_report_proposal_prompt_at(
        "focused query",
        "2026-07-24",
        &catalog,
        &context,
    )
    .expect("typed prompt");
    assert!(prompt.contains(
        "Every label except evidence_boundary is a short interface label, never an answer, claim, or sentence."
    ));
    assert!(prompt.contains(
        "Return at most 32 claims total and keep evidence_boundary to one concise sentence of at most 360 characters."
    ));
    assert!(prompt.contains(
        "Order each dimension's claims as a coherent argument, not an inventory of source summaries."
    ));
    assert!(prompt.contains(
        "write neighboring claims so they read as one developing argument rather than isolated cards placed side by side"
    ));
    assert!(prompt.contains(
        "Write topical synthesis for the reader; do not narrate the retrieval process or introduce claims as source-by-source summaries."
    ));
    assert!(prompt.contains(
        "In a comprehensive report, keep useful partial claims from an unresolved dimension as findings and pair them with its gap"
    ));
    assert!(prompt.contains(
        "single explicitly qualified partial conclusion allowed when every material dimension is unresolved"
    ));
    assert!(prompt.contains(
        "When at least one material dimension is resolved, never place a bounded conclusion in the report summary."
    ));
    assert!(prompt.contains(
        "Use at most one evidence_ref per source_id; when one source contributes multiple chunks"
    ));
    assert!(prompt.contains(
        "Attribute a single-source anecdote, estimate, forecast, benchmark, or reported case to that source"
    ));
    assert!(prompt.contains(
        "A recommendation must remain normative, name every factual or inferential premise in basis_claim_ids, set derivation to null"
    ));
    assert!(prompt.contains(
        "Different tools, capabilities, scopes, maturity levels, or compatible parts of one system are not a contradiction."
    ));
    assert!(prompt.contains(
        "Opaque workflow, dimension, source, chunk, query, target, and criterion IDs belong only in their typed fields"
    ));
    assert!(prompt.contains(
        "A workspace source establishes its contents, not that it belongs to the active build"
    ));
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
