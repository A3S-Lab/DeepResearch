use super::*;

fn labels() -> serde_json::Value {
    serde_json::json!({
        "answer": "Direct Answer",
        "findings": "Findings",
        "recommendations": "Recommendation",
        "limitations": "Limitations",
        "evidence_boundary": "The admitted claims leave one declared evidence requirement unresolved.",
        "sources": "Sources",
        "contradiction": "Contradiction",
        "inference": "Inference",
        "basis": "Basis",
        "derivation": "Derivation"
    })
}

fn source(alias: &str, title: &str, track_id: &str, text: String) -> DeepResearchCatalogSource {
    DeepResearchCatalogSource {
        alias: alias.to_string(),
        title: title.to_string(),
        anchor: format!("https://example.test/{alias}"),
        chunks: vec![text],
        claim_eligible: true,
        semantically_admitted: true,
        relevant_track_ids: vec![track_id.to_string()],
        coverage: vec![DeepResearchSourceCoverage {
            track_id: track_id.to_string(),
            completion_criterion_indexes: vec![0],
            primary: false,
            independent: true,
        }],
    }
}

fn fact(
    id: &str,
    dimension_id: &str,
    placement: &str,
    text: String,
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

#[test]
fn cross_dimension_synthesis_survives_and_missing_claim_corroboration_becomes_a_gap() {
    let detail = |subject: &str| {
        match subject {
            "The technical foundation" => "The retained foundation record identifies the execution boundary, its owning component, and the observable state that marks successful completion.".to_string(),
            "The independently observed foundation" => "An independent account reaches the same foundation boundary through a separate deployment observation and records the terminal state visible in the target environment.".to_string(),
            "The first adoption boundary" => "One decision record limits adoption to environments where the execution precondition can be verified before traffic reaches the new path.".to_string(),
            "The second adoption boundary" => "A separate decision record requires operating corroboration after deployment and therefore leaves broad rollout outside the presently supported scope.".to_string(),
            "The first implementation constraint" => "Request handling must cross a validated execution boundary before downstream work begins; otherwise acceptance does not prove that the new path actually owns the operation.".to_string(),
            "The second implementation constraint" => "Deployment must also preserve the target environment state on which the execution path depends, giving the foundation a second independently checkable precondition.".to_string(),
            "The operating boundary" => "At runtime, the retained record treats a durable terminal state—not entry into the new path—as the observable boundary for successful operation.".to_string(),
            "The deployment precondition" => "Before rollout, the environment must expose the required state and keep it available through settlement, or the implementation remains incomplete despite accepting requests.".to_string(),
            "One adoption boundary" => "The available decision evidence supports a bounded rollout after local verification, but it does not independently corroborate operation across the broader target population.".to_string(),
            _ => format!("{subject} remains bounded by the retained evidence."),
        }
    };
    let context = DeepResearchReportContext {
        report_title: "Cross-dimensional adoption assessment".to_string(),
        scope: DeepResearchReportScope::Comprehensive,
        freshness_required: false,
        tracks: vec![
            serde_json::json!({
                "id": "evidence.foundation",
                "title": "Established foundation",
                "focus": "Establish the technical foundation.",
                "material": true,
                "completion_criteria": ["The technical foundation is directly established."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false,
                },
            }),
            serde_json::json!({
                "id": "decision.path",
                "title": "Adoption path",
                "focus": "Derive a bounded adoption path from the established foundation.",
                "material": true,
                "completion_criteria": ["The adoption boundary is independently corroborated."],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": true,
                },
            }),
        ],
    };
    let catalog = DeepResearchSourceCatalog {
        sources: vec![
            source(
                "source-foundation",
                "Foundation record",
                "evidence.foundation",
                detail("The technical foundation"),
            ),
            source(
                "source-foundation-two",
                "Independent foundation record",
                "evidence.foundation",
                detail("The independently observed foundation"),
            ),
            source(
                "source-decision-one",
                "First decision record",
                "decision.path",
                detail("The first adoption boundary"),
            ),
            source(
                "source-decision-two",
                "Second decision record",
                "decision.path",
                detail("The second adoption boundary"),
            ),
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "foundation-answer",
                "evidence.foundation",
                "direct_answer",
                detail("The technical foundation"),
                "source-foundation",
            ),
            fact(
                "foundation-detail-one",
                "evidence.foundation",
                "finding",
                detail("The first implementation constraint"),
                "source-foundation",
            ),
            fact(
                "foundation-detail-two",
                "evidence.foundation",
                "finding",
                detail("The second implementation constraint"),
                "source-foundation-two",
            ),
            fact(
                "foundation-detail-three",
                "evidence.foundation",
                "finding",
                detail("The operating boundary"),
                "source-foundation",
            ),
            fact(
                "foundation-detail-four",
                "evidence.foundation",
                "finding",
                detail("The deployment precondition"),
                "source-foundation-two",
            ),
            {
                "id": "foundation-synthesis",
                "dimension_id": "evidence.foundation",
                "placement": "finding",
                "kind": "inference",
                "analysis_role": "comparison",
                "text": "Together, the independently attributable implementation constraints show that the foundation is established only when both the execution boundary and the deployment precondition hold.",
                "evidence_refs": [],
                "basis_claim_ids": ["foundation-detail-one", "foundation-detail-two"],
                "derivation": null
            },
            {
                "id": "foundation-mechanism",
                "dimension_id": "evidence.foundation",
                "placement": "finding",
                "kind": "inference",
                "analysis_role": "explanation",
                "text": "The operating boundary explains why the deployment precondition is material: without it, the implementation can appear complete while the target environment still lacks the state required for reliable operation.",
                "evidence_refs": [],
                "basis_claim_ids": ["foundation-detail-three", "foundation-detail-four"],
                "derivation": null
            },
            {
                "id": "foundation-implication",
                "dimension_id": "evidence.foundation",
                "placement": "finding",
                "kind": "inference",
                "analysis_role": "implication",
                "text": "The practical implication is to verify the execution boundary and deployment precondition as separate acceptance checks before treating the technical foundation as available to an adoption decision.",
                "evidence_refs": [],
                "basis_claim_ids": ["foundation-synthesis", "foundation-mechanism"],
                "derivation": null
            },
            {
                "id": "foundation-boundary",
                "dimension_id": "evidence.foundation",
                "placement": "finding",
                "kind": "inference",
                "analysis_role": "boundary",
                "text": "The conclusion remains bounded to environments where both checks can be observed before and after deployment; it does not establish dependable operation when target state can change after verification, settlement is not independently visible, or the failure path bypasses the documented ownership boundary. It also cannot be transferred to a broader target population until an independent operating record shows that the same preconditions, ownership transition, recovery behavior, and terminal-state signal remain stable outside the retained deployment observations.",
                "evidence_refs": [],
                "basis_claim_ids": ["foundation-mechanism", "foundation-detail-three"],
                "derivation": null
            },
            fact(
                "decision-detail",
                "decision.path",
                "finding",
                detail("One adoption boundary"),
                "source-decision-one",
            ),
            {
                "id": "adoption-recommendation",
                "dimension_id": "decision.path",
                "placement": "direct_answer",
                "kind": "recommendation",
                "analysis_role": "conclusion",
                "text": "Adopt incrementally only after the established technical foundation is verified in the target environment, and retain the bounded rollout until independent operating evidence is available.",
                "evidence_refs": [],
                "basis_claim_ids": ["foundation-answer", "decision-detail"],
                "derivation": null
            }
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "evidence.foundation",
                "heading": "What makes the technical foundation dependable",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["foundation-detail-one", "foundation-detail-two"]
                }, {
                    "purpose": "evidence",
                    "claim_ids": ["foundation-detail-three", "foundation-detail-four"]
                }, {
                    "purpose": "synthesis",
                    "claim_ids": ["foundation-synthesis", "foundation-mechanism"]
                }, {
                    "purpose": "implication",
                    "claim_ids": ["foundation-implication"]
                }, {
                    "purpose": "boundary",
                    "claim_ids": ["foundation-boundary"]
                }]
            }, {
                "dimension_id": "decision.path",
                "heading": "Where the adoption decision remains bounded",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["decision-detail"]
                }]
            }]
        }
    });

    let report = admit_deep_research_typed_report_proposal_at(
        "derive an adoption path from the evidence",
        "2026-07-25",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed admission")
    .expect("qualified cross-dimensional synthesis");

    assert_eq!(
        report.publication,
        DeepResearchEvidenceFirstPublication::Qualified
    );
    assert_eq!(report.accepted_claim_count, 11);
    assert_eq!(report.accepted_basis_edge_count, 10);
    assert_eq!(report.analytical_claim_count, 5);
    assert_eq!(report.cross_source_synthesis_count, 1);
    assert_eq!(report.resolved_material_dimension_count, 1);
    assert_eq!(report.deeply_analyzed_dimension_count, 1);
    assert_eq!(report.accepted_gap_count, 1);
    assert!(report.markdown.contains("Adopt incrementally only after"));
}
