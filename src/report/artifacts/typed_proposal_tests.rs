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

fn source_attribution(
    source_groups: &[(&str, &str)],
    independent_group_pairs: &[(&str, &str)],
) -> DeepResearchSourceAttribution {
    DeepResearchSourceAttribution {
        source_group_ids: source_groups
            .iter()
            .map(|(source_alias, group_id)| ((*source_alias).to_string(), (*group_id).to_string()))
            .collect(),
        independent_group_pairs: independent_group_pairs
            .iter()
            .map(|(left, right)| {
                if left < right {
                    ((*left).to_string(), (*right).to_string())
                } else {
                    ((*right).to_string(), (*left).to_string())
                }
            })
            .collect(),
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

include!("typed_proposal_tests/depth.rs");
include!("typed_proposal_tests/editorial.rs");
include!("typed_proposal_tests/validation.rs");
