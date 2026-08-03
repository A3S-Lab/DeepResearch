use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{FrozenFault, FrozenReplay};
use crate::engine::WorkflowOutput;
use crate::planner::deep_research_loop_contract;
use crate::research::compiler::*;

pub(super) fn workflow_args(replay: &FrozenReplay) -> Value {
    let evidence_scope = match replay.contract.spec.evidence_scope {
        EvidenceScope::Workspace => "local_only",
        EvidenceScope::Web | EvidenceScope::WebAndWorkspace => "web_and_workspace",
    };
    serde_json::json!({
        "source": "frozen-active-replay",
        "run_id": format!("engine-replay-{}", replay.id),
        "input": {
            "query": replay.contract.spec.query,
            "current_date": replay.contract.spec.current_date,
            "evidence_scope": evidence_scope,
            "loop_contract": deep_research_loop_contract(
                &replay.contract.spec.query,
                &replay.contract.spec.current_date,
                evidence_scope,
                replay.contract.spec.dimensions.len(),
            ),
        },
        "limits": {
            "timeoutMs": crate::engine::DEFAULT_PLANNED_RETRIEVAL_STAGE_TIMEOUT_MS,
            "maxToolCalls": 64,
            "maxOutputBytes": 1_048_576,
        }
    })
}

pub(super) fn planner_outline(replay: &FrozenReplay) -> Value {
    let tracks = replay
        .contract
        .spec
        .dimensions
        .iter()
        .enumerate()
        .map(|(index, dimension)| {
            let roles = dimension
                .source_target_ids
                .iter()
                .filter_map(|target_id| replay.contract.target(target_id))
                .map(|target| target.role)
                .collect::<Vec<_>>();
            serde_json::json!({
                "id": dimension.id,
                "title": bounded_text(&dimension.question, 160),
                "focus": bounded_text(&dimension.question, 500),
                "material": dimension.material,
                "requirement_ids": [format!("request.{}", index + 1)],
                "completion_criteria": [bounded_text(&dimension.question, 240)],
                "questions": [{
                    "question": bounded_text(&dimension.question, 240),
                    "role": "establish",
                    "completion_criterion_indexes": [0],
                }],
                "evidence_requirements": {
                    "primary_source_required": roles.iter().any(|role| {
                        matches!(
                            role,
                            SourceRole::Canonical | SourceRole::Official | SourceRole::Primary
                        )
                    }),
                    "independent_corroboration_required": roles.contains(&SourceRole::Independent),
                },
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "report_title": bounded_text(&replay.contract.spec.query, 160),
        // The frozen contract does not contain an active-engine scope
        // classification. Keep the projection fixed instead of inferring one
        // from query or source vocabulary.
        "research_scope": "focused",
        "freshness_required": false,
        "workspace_evidence_required": matches!(
            replay.contract.spec.evidence_scope,
            EvidenceScope::Workspace | EvidenceScope::WebAndWorkspace
        ),
        "request_requirements": replay.contract.spec.dimensions.iter().enumerate().map(|(index, dimension)| {
            serde_json::json!({
                "id": format!("request.{}", index + 1),
                "text": bounded_text(&dimension.question, 300),
            })
        }).collect::<Vec<_>>(),
        "tracks": tracks,
        "supplemental_queries": [],
    })
}

pub(super) fn bootstrap_output(replay: &FrozenReplay) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": replay.contract.spec.query,
            "mode": "bootstrap_acquisition",
            "acquisition": {
                "packet": {
                    "version": 1,
                    "sources": replay
                        .catalog
                        .sources
                        .iter()
                        .map(|source| source_packet(replay, source, false))
                        .collect::<Vec<_>>(),
                },
            },
            "execution": {
                "terminal_authority": "host_inquiry_reducer",
            },
        })
        .to_string(),
        metadata: None,
    }
}

pub(super) fn planned_output(replay: &FrozenReplay) -> WorkflowOutput {
    let failed_dimension = match replay.fault.as_ref() {
        Some(FrozenFault::MalformedEvidenceExtraction { dimension_id }) => {
            Some(dimension_id.as_str())
        }
        _ => None,
    };
    let sources = replay
        .catalog
        .sources
        .iter()
        .map(|source| source_packet(replay, source, true))
        .collect::<Vec<_>>();
    let source_relevance = replay
        .catalog
        .sources
        .iter()
        .flat_map(|source| {
            source_dimensions(replay, source)
                .into_iter()
                .filter(|dimension| Some(dimension.id.as_str()) != failed_dimension)
                .map(|dimension| {
                    serde_json::json!({
                        "source_id": source.id,
                        "obligation_id": dimension.id,
                    })
                })
        })
        .collect::<Vec<_>>();
    let source_coverage = replay
        .catalog
        .sources
        .iter()
        .flat_map(|source| {
            source_dimensions(replay, source)
                .into_iter()
                .filter(|dimension| Some(dimension.id.as_str()) != failed_dimension)
                .map(|dimension| {
                    serde_json::json!({
                        "source_id": source.id,
                        "obligation_id": dimension.id,
                        "completion_criterion_indexes": [0],
                        "roles": source_roles(replay, source, dimension),
                    })
                })
        })
        .collect::<Vec<_>>();
    let relevant_obligation_ids = source_relevance
        .iter()
        .filter_map(|binding| binding.get("obligation_id").and_then(Value::as_str))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();

    WorkflowOutput {
        output: serde_json::json!({
            "query": replay.contract.spec.query,
            "mode": "inquiry_collection",
            "research": {
                "status": if failed_dimension.is_some() { "partial" } else { "success" },
                "metadata": {
                    "evidence_selection_mode": "semantic_chunk_ids_with_typed_coverage",
                },
                "results": [{
                    "task_id": "frozen-evidence-projection",
                    "agent": "workflow",
                    "success": true,
                    "structured": {
                        "summary": "Frozen evidence projection.",
                        "sources": sources,
                        "source_relevance": source_relevance,
                        "source_coverage": source_coverage,
                        "relevant_obligation_ids": relevant_obligation_ids,
                        "key_evidence": [],
                        "contradictions": [],
                        "confidence": "Closed fixture projection.",
                        "gaps": [],
                    },
                }],
                "warnings": {
                    "collection_errors": [],
                },
            },
        })
        .to_string(),
        metadata: None,
    }
}

pub(super) fn report_proposal(replay: &FrozenReplay) -> Value {
    let source_indexes = replay
        .catalog
        .sources
        .iter()
        .enumerate()
        .map(|(index, source)| (source.id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let claims = replay
        .proposal
        .claims
        .iter()
        .map(|claim| {
            let analysis_role = match (claim.placement, claim.kind) {
                (ClaimPlacement::DirectAnswer, _) => "conclusion",
                (ClaimPlacement::Finding, ClaimKind::Fact) => "evidence",
                (ClaimPlacement::Finding, ClaimKind::Inference)
                    if claim.basis_claim_ids.len() >= 2 =>
                {
                    "comparison"
                }
                (ClaimPlacement::Finding, ClaimKind::Inference) => "explanation",
                (ClaimPlacement::Finding, ClaimKind::Recommendation) => "implication",
            };
            serde_json::json!({
                "id": claim.id,
                "dimension_id": claim.dimension_id,
                "placement": claim.placement,
                "kind": claim.kind,
                "analysis_role": analysis_role,
                "text": claim.text,
                "evidence_refs": claim
                    .evidence_refs
                    .iter()
                    .filter_map(|evidence| {
                        let source_index = *source_indexes.get(evidence.source_id.as_str())?;
                        let source = replay.catalog.sources.get(source_index)?;
                        let source_id = format!("source-{}", source_index + 1);
                        let chunk_ids = evidence
                            .chunk_ids
                            .iter()
                            .filter_map(|chunk_id| {
                                source
                                    .chunks
                                    .iter()
                                    .position(|chunk| chunk.id == *chunk_id)
                                    .map(|chunk_index| {
                                        format!("{source_id}:chunk:{}", chunk_index + 1)
                                    })
                            })
                            .collect::<Vec<_>>();
                        Some(serde_json::json!({
                            "source_id": source_id,
                            "chunk_ids": chunk_ids,
                        }))
                    })
                    .collect::<Vec<_>>(),
                "basis_claim_ids": claim.basis_claim_ids,
                "derivation": claim.derivation,
            })
        })
        .collect::<Vec<_>>();
    let labels = &replay.contract.spec.reader_labels;
    let narrative_sections = replay
        .contract
        .spec
        .dimensions
        .iter()
        .map(|dimension| {
            let paragraphs = replay
                .proposal
                .claims
                .iter()
                .filter(|claim| {
                    claim.dimension_id == dimension.id && claim.placement == ClaimPlacement::Finding
                })
                .map(|claim| {
                    let purpose = match claim.kind {
                        ClaimKind::Fact => "evidence",
                        ClaimKind::Inference => "synthesis",
                        ClaimKind::Recommendation => "implication",
                    };
                    serde_json::json!({
                        "purpose": purpose,
                        "claim_ids": [claim.id],
                    })
                })
                .collect::<Vec<_>>();
            serde_json::json!({
                "dimension_id": dimension.id,
                "heading": bounded_text(&dimension.question, 96),
                "paragraphs": paragraphs,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "report_language": replay.contract.spec.language,
        "labels": {
            "answer": labels.direct_answer,
            "findings": labels.findings,
            "recommendations": labels.recommendation,
            "limitations": labels.limitations,
            "evidence_boundary": labels.source_backed_gap,
            "sources": labels.sources,
            "contradiction": labels.contradiction,
            "inference": labels.inference,
            "basis": labels.basis,
            "derivation": labels.derivation,
        },
        "claims": claims,
        "relations": replay.proposal.relations,
        "gaps": replay.proposal.gaps.iter().map(|gap| {
            serde_json::json!({
                "id": gap.id,
                "dimension_id": gap.dimension_id,
                "text": gap.text,
            })
        }).collect::<Vec<_>>(),
        "narrative": {
            "sections": narrative_sections,
        },
    })
}

fn source_packet(replay: &FrozenReplay, source: &SourceRecord, selected: bool) -> Value {
    let chunks = source
        .chunks
        .iter()
        .map(|chunk| {
            if selected {
                serde_json::json!({
                    "focus": "",
                    "quote_or_fact": chunk.text,
                })
            } else {
                serde_json::json!({
                    "chunk_id": chunk.id,
                    "text": chunk.text,
                })
            }
        })
        .collect::<Vec<_>>();
    if selected {
        serde_json::json!({
            "source_id": source.id,
            "title": source.title,
            "url_or_path": active_source_anchor(replay, source),
            "reliability": "fetched",
            "evidence_excerpts": chunks,
        })
    } else {
        serde_json::json!({
            "source_id": source.id,
            "title": source.title,
            "url_or_path": active_source_anchor(replay, source),
            "reliability": "fetched",
            "chunks": chunks,
        })
    }
}

fn active_source_anchor(replay: &FrozenReplay, source: &SourceRecord) -> String {
    source
        .provenance
        .iter()
        .filter_map(|edge| replay.contract.target(&edge.source_target_id))
        .find_map(|target| match &target.match_policy {
            TargetMatchPolicy::Named {
                identity: SourceIdentity::WorkspacePath(path),
            } => Some(path.clone()),
            _ => None,
        })
        .unwrap_or_else(|| source.canonical_anchor.clone())
}

fn source_dimensions<'a>(
    replay: &'a FrozenReplay,
    source: &SourceRecord,
) -> Vec<&'a ResearchDimension> {
    let target_ids = source
        .provenance
        .iter()
        .map(|edge| edge.source_target_id.as_str())
        .collect::<BTreeSet<_>>();
    replay
        .contract
        .spec
        .dimensions
        .iter()
        .filter(|dimension| {
            dimension
                .source_target_ids
                .iter()
                .any(|target_id| target_ids.contains(target_id.as_str()))
        })
        .collect()
}

fn source_roles(
    replay: &FrozenReplay,
    source: &SourceRecord,
    dimension: &ResearchDimension,
) -> Vec<&'static str> {
    let roles = source
        .provenance
        .iter()
        .filter(|edge| dimension.source_target_ids.contains(&edge.source_target_id))
        .filter_map(|edge| replay.contract.target(&edge.source_target_id))
        .map(|target| target.role)
        .collect::<Vec<_>>();
    let mut projected = vec!["supporting"];
    if roles.iter().any(|role| {
        matches!(
            role,
            SourceRole::Canonical | SourceRole::Official | SourceRole::Primary
        )
    }) {
        projected.push("primary");
    }
    if roles.contains(&SourceRole::Independent) {
        projected.push("independent");
    }
    projected
}

fn bounded_text(value: &str, maximum: usize) -> String {
    value.chars().take(maximum).collect()
}
