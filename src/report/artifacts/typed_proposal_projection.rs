fn typed_compiler_projection(
    _query: &str,
    current_date: &str,
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
    claims: &[serde_json::Value],
    report_language: &str,
    labels: &TypedWireReportLabels,
) -> Result<TypedCompilerProjection, String> {
    let mut sources = typed_closed_sources(catalog, context);
    if sources.is_empty() {
        return Err("typed compiler projection contains no admitted sources".to_string());
    }
    let dependency_dimensions = typed_dependency_source_dimensions(claims);
    for source in &mut sources {
        let Some(dimensions) = dependency_dimensions.get(&source.id) else {
            continue;
        };
        source.relevant_track_ids.extend(dimensions.iter().cloned());
        source.relevant_track_ids.sort();
        source.relevant_track_ids.dedup();
    }
    let mut dimensions = std::collections::BTreeMap::new();
    let mut source_targets = Vec::new();
    let mut spec_dimensions = Vec::new();
    let mut queries = Vec::new();
    for (dimension_index, track) in context.tracks.iter().enumerate() {
        let dimension_id = track
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "typed compiler projection track omitted its ID".to_string())?;
        let mut transports = sources
            .iter()
            .filter(|source| {
                source
                    .relevant_track_ids
                    .iter()
                    .any(|track_id| track_id == dimension_id)
            })
            .map(|source| source.transport)
            .collect::<std::collections::BTreeSet<_>>();
        if transports.is_empty() {
            transports.insert(TypedCompilerTransport::Web);
        }
        let selection_goal = track
            .get("focus")
            .and_then(serde_json::Value::as_str)
            .or_else(|| track.get("title").and_then(serde_json::Value::as_str))
            .ok_or_else(|| "typed compiler projection track omitted its focus".to_string())?;
        let mut binding = TypedDimensionBinding {
            query_ids: Vec::new(),
            target_ids: Vec::new(),
            targets_by_transport: std::collections::BTreeMap::new(),
        };
        for transport in transports {
            let suffix = transport.id_suffix();
            let target_id = format!("t{}{}", dimension_index + 1, suffix);
            let query_id = format!("q{}{}", dimension_index + 1, suffix);
            source_targets.push(serde_json::json!({
                "id": target_id,
                "source_family_id": format!("f{}{}", dimension_index + 1, suffix),
                "role": "supporting",
                "transport": transport.as_str(),
                "match_policy": {
                    "kind": "exploratory",
                    "selection_goal": selection_goal,
                },
            }));
            queries.push(serde_json::json!({
                "id": query_id,
                "text": selection_goal,
                "transport": transport.as_str(),
                "mode": "discovery",
                "dimension_ids": [dimension_id],
                "source_target_ids": [target_id],
                "fetch_slots": 1,
            }));
            binding.query_ids.push(query_id.clone());
            binding.target_ids.push(target_id.clone());
            binding
                .targets_by_transport
                .insert(transport, (query_id, target_id));
        }
        spec_dimensions.push(serde_json::json!({
            "id": dimension_id,
            "question": track
                .get("title")
                .and_then(serde_json::Value::as_str)
                .unwrap_or(selection_goal),
            "material": track
                .get("material")
                .and_then(serde_json::Value::as_bool)
                .unwrap_or(true),
            "source_target_ids": binding.target_ids,
        }));
        dimensions.insert(dimension_id.to_string(), binding);
    }
    let reader_labels = typed_compiler_reader_labels(labels);
    let query_count = queries.len();
    let spec = serde_json::json!({
        "version": 3,
        "query": context.report_title,
        "language": report_language,
        "reader_labels": reader_labels,
        "current_date": current_date,
        "evidence_scope": "web_and_workspace",
        "dimensions": spec_dimensions,
        "source_targets": source_targets,
        "budget": {
            "max_queries": query_count,
            "max_fetches": query_count,
        },
    });
    let spec_digest = crate::research::compiler::evidence_spec_digest(&spec)
        .map_err(|error| format!("digest typed report spec: {error}"))?;
    let plan = serde_json::json!({
        "spec_digest": spec_digest,
        "queries": queries,
        "planning_gaps": [],
    });
    let projected_sources = sources
        .iter()
        .map(|source| {
            let provenance = source
                .relevant_track_ids
                .iter()
                .filter_map(|dimension_id| dimensions.get(dimension_id))
                .filter_map(|binding| binding.targets_by_transport.get(&source.transport))
                .map(|(query_id, target_id)| {
                    serde_json::json!({
                        "query_id": query_id,
                        "source_target_id": target_id,
                    })
                })
                .collect::<Vec<_>>();
            let chunks = source
                .chunks
                .iter()
                .map(|chunk| {
                    serde_json::json!({
                        "id": chunk.id,
                        "text": chunk.text,
                    })
                })
                .collect::<Vec<_>>();
            let content_digest =
                crate::research::compiler::evidence_source_content_digest(&serde_json::Value::Array(
                    chunks.clone(),
                ))
                .map_err(|error| format!("digest typed source chunks: {error}"))?;
            Ok(serde_json::json!({
                "id": source.id,
                "title": source.title,
                "requested_anchor": source.anchor,
                "canonical_anchor": source.anchor,
                "captured_at": format!("{current_date}T00:00:00Z"),
                "provenance": provenance,
                "chunks": chunks,
                "content_digest": content_digest,
            }))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let attempts = dimensions
        .values()
        .flat_map(|binding| {
            binding
                .targets_by_transport
                .values()
                .map(|(query_id, target_id)| {
                    let fetched = sources.iter().any(|source| {
                        source.relevant_track_ids.iter().any(|dimension_id| {
                            dimensions.get(dimension_id).is_some_and(|candidate| {
                                candidate
                                    .targets_by_transport
                                    .get(&source.transport)
                                    .is_some_and(|(candidate_query, candidate_target)| {
                                        candidate_query == query_id && candidate_target == target_id
                                    })
                            })
                        })
                    });
                    serde_json::json!({
                        "query_id": query_id,
                        "source_target_ids": [target_id],
                        "outcome": {
                            "status": if fetched { "fetched" } else { "no_candidates" },
                        },
                    })
                })
        })
        .collect::<Vec<_>>();
    let compiler_catalog = serde_json::json!({
        "spec_digest": spec_digest,
        "attempts": attempts,
        "sources": projected_sources,
    });
    Ok(TypedCompilerProjection {
        spec,
        plan,
        catalog: compiler_catalog,
        dimensions,
    })
}

fn typed_dependency_source_dimensions(
    claims: &[serde_json::Value],
) -> std::collections::HashMap<String, HashSet<String>> {
    let id_counts = claims
        .iter()
        .filter_map(|claim| claim.get("id").and_then(serde_json::Value::as_str))
        .fold(
            std::collections::HashMap::<String, usize>::new(),
            |mut counts, id| {
                *counts.entry(id.to_string()).or_default() += 1;
                counts
            },
        );
    let claims_by_id = claims
        .iter()
        .filter_map(|claim| {
            let id = claim.get("id").and_then(serde_json::Value::as_str)?;
            (id_counts.get(id) == Some(&1)).then_some((id.to_string(), claim))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let mut sources_by_claim = claims_by_id
        .iter()
        .map(|(id, claim)| {
            let source_ids = claim
                .get("evidence_refs")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|reference| {
                    reference
                        .get("source_id")
                        .and_then(serde_json::Value::as_str)
                        .map(str::to_string)
                })
                .collect::<HashSet<_>>();
            (id.clone(), source_ids)
        })
        .collect::<std::collections::HashMap<_, _>>();
    for _ in 0..claims_by_id.len() {
        let snapshot = sources_by_claim.clone();
        let mut changed = false;
        for (id, claim) in &claims_by_id {
            let Some(sources) = sources_by_claim.get_mut(id) else {
                continue;
            };
            for basis_id in claim
                .get("basis_claim_ids")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
            {
                let Some(basis_sources) = snapshot.get(basis_id) else {
                    continue;
                };
                let previous_len = sources.len();
                sources.extend(basis_sources.iter().cloned());
                changed |= sources.len() != previous_len;
            }
        }
        if !changed {
            break;
        }
    }
    let mut dimensions_by_source =
        std::collections::HashMap::<String, HashSet<String>>::new();
    for claim in claims {
        if !matches!(
            claim.get("kind").and_then(serde_json::Value::as_str),
            Some("inference" | "recommendation")
        ) {
            continue;
        }
        let Some(dimension_id) = claim
            .get("dimension_id")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        for basis_id in claim
            .get("basis_claim_ids")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            for source_id in sources_by_claim.get(basis_id).into_iter().flatten() {
                dimensions_by_source
                    .entry(source_id.clone())
                    .or_default()
                    .insert(dimension_id.to_string());
            }
        }
    }
    dimensions_by_source
}

fn typed_compiler_reader_labels(labels: &TypedWireReportLabels) -> serde_json::Value {
    serde_json::json!({
        "report_sections": labels.findings,
        "skip_to_report": labels.answer,
        "direct_answer": labels.answer,
        "research_dimensions": labels.findings,
        "sources": labels.sources,
        "status": labels.limitations,
        "findings": labels.findings,
        "limitations": labels.limitations,
        "retained_excerpts": labels.findings,
        "contradiction": labels.contradiction,
        "inference": labels.inference,
        "recommendation": labels.recommendations,
        "basis": labels.basis,
        "derivation": labels.derivation,
        "finding": labels.findings,
        "captured": labels.sources,
        "requested_as": labels.sources,
        "source_backed": labels.limitations,
        "no_evidence": labels.limitations,
        "source_backed_gap": labels.evidence_boundary,
        "no_evidence_gap": labels.evidence_boundary,
        "coverage_claims": labels.answer,
        "coverage_partial": labels.limitations,
        "coverage_bounded": labels.limitations,
        "coverage_missing": labels.limitations,
    })
}

fn typed_material_dimensions_are_answered_or_bounded(
    context: &DeepResearchReportContext,
    catalog: &DeepResearchSourceCatalog,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> bool {
    let directly_answered = compiled
        .claim_support
        .iter()
        .filter(|claim| {
            claim.placement == crate::research::compiler::CompilerClaimPlacement::DirectAnswer
        })
        .map(|claim| claim.dimension_id.as_str())
        .collect::<HashSet<_>>();
    context
        .tracks
        .iter()
        .filter(|track| {
            track
                .get("material")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
        .all(|track| {
            let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
                return false;
            };
            let fully_resolved = typed_track_is_resolved_by_claim_support(
                track,
                catalog,
                attribution,
                compiled,
            );
            let explicitly_bounded = compiled.coverage.iter().any(|coverage| {
                coverage.dimension_id == dimension_id
                    && matches!(
                        coverage.status,
                        crate::research::compiler::CompilerStructuralCoverage::ClaimsAndGap
                            | crate::research::compiler::CompilerStructuralCoverage::GapOnly
                    )
            });
            if explicitly_bounded {
                true
            } else if fully_resolved {
                context.scope != DeepResearchReportScope::Comprehensive
                    || directly_answered.contains(dimension_id)
            } else {
                explicitly_bounded
            }
        })
}

fn typed_material_dimensions_needing_claim_gap(
    context: &DeepResearchReportContext,
    catalog: &DeepResearchSourceCatalog,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> Vec<String> {
    context
        .tracks
        .iter()
        .filter(|track| {
            track
                .get("material")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
        })
        .filter_map(|track| {
            let dimension_id = track.get("id").and_then(serde_json::Value::as_str)?;
            let already_bounded = compiled.coverage.iter().any(|coverage| {
                coverage.dimension_id == dimension_id
                    && matches!(
                        coverage.status,
                        crate::research::compiler::CompilerStructuralCoverage::ClaimsAndGap
                            | crate::research::compiler::CompilerStructuralCoverage::GapOnly
                    )
            });
            let resolved_by_claim_support = typed_track_is_resolved_by_claim_support(
                track,
                catalog,
                attribution,
                compiled,
            );
            let needs_depth_gap = context.scope == DeepResearchReportScope::Comprehensive
                && resolved_by_claim_support
                && !typed_compiled_dimension_has_required_depth(
                    dimension_id,
                    attribution,
                    compiled,
                );
            (!already_bounded && (!resolved_by_claim_support || needs_depth_gap))
                .then(|| dimension_id.to_string())
        })
        .collect()
}

fn typed_track_is_resolved_by_claim_support(
    track: &serde_json::Value,
    catalog: &DeepResearchSourceCatalog,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> bool {
    let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
        return false;
    };
    let source_indexes = compiled
        .claim_support
        .iter()
        .filter(|claim| claim.dimension_id == dimension_id)
        .flat_map(|claim| claim.source_ids.iter())
        .filter_map(|source_id| {
            catalog
                .sources
                .iter()
                .position(|source| source.alias == *source_id)
        })
        .collect::<HashSet<_>>();
    let criterion_count = track
        .get("completion_criteria")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
        .unwrap_or_default();
    criterion_count > 0
        && report_track_coverage_state_with_attribution(
            track,
            catalog,
            &source_indexes,
            attribution,
        )
            .is_some_and(|state| state.is_resolved(criterion_count))
}
