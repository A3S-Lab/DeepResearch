fn typed_context_matches_output_language(
    context: &DeepResearchReportContext,
    output_language: &str,
) -> bool {
    let mut reader_text = context.report_title.clone();
    for track in &context.tracks {
        for field in ["title", "focus"] {
            if let Some(value) = track.get(field).and_then(serde_json::Value::as_str) {
                reader_text.push('\n');
                reader_text.push_str(value);
            }
        }
        for criterion in track
            .get("completion_criteria")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
        {
            reader_text.push('\n');
            reader_text.push_str(criterion);
        }
        for question in track
            .get("questions")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
        {
            let text = question
                .as_str()
                .or_else(|| question.get("question").and_then(serde_json::Value::as_str));
            if let Some(text) = text {
                reader_text.push('\n');
                reader_text.push_str(text);
            }
        }
    }
    crate::language::reader_text_matches_output_language(&reader_text, output_language)
}

fn typed_wire_matches_output_language(
    wire: &TypedWireReportProposal,
    output_language: &str,
) -> bool {
    let mut reader_text = [
        wire.labels.answer.as_str(),
        wire.labels.findings.as_str(),
        wire.labels.recommendations.as_str(),
        wire.labels.limitations.as_str(),
        wire.labels.evidence_boundary.as_str(),
        wire.labels.sources.as_str(),
        wire.labels.contradiction.as_str(),
        wire.labels.inference.as_str(),
        wire.labels.basis.as_str(),
        wire.labels.derivation.as_str(),
    ]
    .join("\n");
    for claim in &wire.claims {
        if let Some(text) = claim.get("text").and_then(serde_json::Value::as_str) {
            reader_text.push('\n');
            reader_text.push_str(text);
        }
        if let Some(method) = claim
            .pointer("/derivation/method")
            .and_then(serde_json::Value::as_str)
        {
            reader_text.push('\n');
            reader_text.push_str(method);
        }
    }
    for gap in &wire.gaps {
        reader_text.push('\n');
        reader_text.push_str(&gap.text);
    }
    for section in &wire.narrative.sections {
        reader_text.push('\n');
        reader_text.push_str(&section.heading);
    }
    crate::language::reader_text_matches_output_language(&reader_text, output_language)
}

fn typed_unresolved_dimension_ids(
    catalog: &DeepResearchSourceCatalog,
    attribution: Option<&DeepResearchSourceAttribution>,
    context: &DeepResearchReportContext,
) -> Result<Vec<String>, String> {
    let eligible_source_indexes = typed_closed_sources(catalog, context)
        .iter()
        .map(|source| source.catalog_index)
        .collect::<HashSet<_>>();
    context
        .tracks
        .iter()
        .map(|track| {
            let criterion_count = track
                .get("completion_criteria")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .filter(|count| *count > 0)
                .ok_or_else(|| {
                    "typed report proposal received an invalid track contract".to_string()
                })?;
            let state = report_track_coverage_state_with_attribution(
                track,
                catalog,
                &eligible_source_indexes,
                attribution,
            )
            .ok_or_else(|| {
                    "typed report proposal received an invalid track contract".to_string()
                })?;
            Ok((!state.is_resolved(criterion_count)).then_some(state.track_id))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|ids| ids.into_iter().flatten().collect())
}

fn validate_typed_wire_report(wire: &TypedWireReportProposal) -> Result<(), String> {
    if wire.claims.len() > TYPED_REPORT_MAX_CLAIMS
        || wire.relations.len() > TYPED_REPORT_MAX_RELATIONS
        || wire.gaps.len() > TYPED_REPORT_MAX_GAPS
    {
        return Err("typed report proposal exceeded its closed graph bounds".to_string());
    }
    if wire.report_language.len() > 32
        || wire.report_language.chars().count() < 2
        || !wire
            .report_language
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic())
        || !wire
            .report_language
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '-')
    {
        return Err("typed report proposal returned an invalid report_language".to_string());
    }
    for (field, value, maximum, minimum) in [
        (
            "answer",
            wire.labels.answer.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "findings",
            wire.labels.findings.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "recommendations",
            wire.labels.recommendations.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "limitations",
            wire.labels.limitations.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "evidence_boundary",
            wire.labels.evidence_boundary.as_str(),
            REPORT_PROPOSAL_MAX_EVIDENCE_BOUNDARY_CHARS,
            8,
        ),
        (
            "sources",
            wire.labels.sources.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "contradiction",
            wire.labels.contradiction.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "inference",
            wire.labels.inference.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "basis",
            wire.labels.basis.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
        (
            "derivation",
            wire.labels.derivation.as_str(),
            REPORT_PROPOSAL_MAX_HEADING_CHARS,
            1,
        ),
    ] {
        let count = value.chars().count();
        if value.trim() != value
            || count < minimum
            || count > maximum
            || value.chars().any(char::is_control)
        {
            return Err(format!(
                "typed report proposal returned an invalid `{field}` label"
            ));
        }
    }
    validate_typed_narrative_shape(&wire.narrative)
}

fn coalesce_typed_claim_evidence_refs(claims: &mut [serde_json::Value]) {
    for claim in claims {
        let Some(evidence_refs) = claim
            .get_mut("evidence_refs")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        let mut normalized = Vec::<serde_json::Value>::with_capacity(evidence_refs.len());
        let mut source_positions = std::collections::HashMap::<String, usize>::new();
        for evidence_ref in std::mem::take(evidence_refs) {
            let normalized_identity = evidence_ref.as_object().and_then(|object| {
                if object.len() != 2 {
                    return None;
                }
                let source_id = object.get("source_id")?.as_str()?.to_string();
                let chunk_ids = object
                    .get("chunk_ids")?
                    .as_array()?
                    .iter()
                    .map(|chunk_id| chunk_id.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()?;
                (!chunk_ids.is_empty()).then_some((source_id, chunk_ids))
            });
            let Some((source_id, chunk_ids)) = normalized_identity else {
                normalized.push(evidence_ref);
                continue;
            };
            let Some(position) = source_positions.get(&source_id).copied() else {
                source_positions.insert(source_id, normalized.len());
                normalized.push(evidence_ref);
                continue;
            };
            let Some(retained_chunk_ids) = normalized[position]
                .get_mut("chunk_ids")
                .and_then(serde_json::Value::as_array_mut)
            else {
                normalized.push(evidence_ref);
                continue;
            };
            for chunk_id in chunk_ids {
                if retained_chunk_ids
                    .iter()
                    .any(|retained| retained.as_str() == Some(chunk_id.as_str()))
                {
                    continue;
                }
                retained_chunk_ids.push(serde_json::Value::String(chunk_id));
            }
        }
        *evidence_refs = normalized;
    }
}

fn normalize_typed_claim_placements(
    claims: &mut [serde_json::Value],
    exclude_unresolved_dimensions: bool,
    unresolved_dimension_ids: &HashSet<String>,
) -> HashSet<String> {
    let mut answered_dimensions = HashSet::<String>::new();
    let mut demoted_claim_ids = HashSet::<String>::new();
    for claim in claims {
        if claim.get("placement").and_then(serde_json::Value::as_str) != Some("direct_answer") {
            continue;
        }
        let Some(dimension_id) = claim
            .get("dimension_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        if exclude_unresolved_dimensions && unresolved_dimension_ids.contains(&dimension_id) {
            demote_typed_direct_answer(claim);
            if let Some(claim_id) = claim.get("id").and_then(serde_json::Value::as_str) {
                demoted_claim_ids.insert(claim_id.to_string());
            }
            continue;
        }
        if answered_dimensions.insert(dimension_id) {
            continue;
        }
        demote_typed_direct_answer(claim);
        if let Some(claim_id) = claim.get("id").and_then(serde_json::Value::as_str) {
            demoted_claim_ids.insert(claim_id.to_string());
        }
    }
    demoted_claim_ids
}

fn demote_typed_direct_answer(claim: &mut serde_json::Value) {
    claim["placement"] = serde_json::Value::String("finding".to_string());
    if claim
        .get("analysis_role")
        .and_then(serde_json::Value::as_str)
        != Some("conclusion")
    {
        return;
    }
    let role = match claim.get("kind").and_then(serde_json::Value::as_str) {
        Some("fact") => "evidence",
        Some("recommendation") => "implication",
        _ => "boundary",
    };
    claim["analysis_role"] = serde_json::Value::String(role.to_string());
}

fn normalize_typed_recommendation_derivations(claims: &mut [serde_json::Value]) {
    for claim in claims {
        if claim.get("kind").and_then(serde_json::Value::as_str) == Some("recommendation") {
            claim["derivation"] = serde_json::Value::Null;
        }
    }
}

fn normalize_typed_inference_basis_kinds(claims: &mut [serde_json::Value]) {
    let recommendation_ids = claims
        .iter()
        .filter(|claim| {
            claim.get("kind").and_then(serde_json::Value::as_str) == Some("recommendation")
        })
        .filter_map(|claim| claim.get("id").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect::<HashSet<_>>();

    for claim in claims.iter_mut().filter(|claim| {
        claim.get("kind").and_then(serde_json::Value::as_str) == Some("inference")
    }) {
        let Some(basis_claim_ids) = claim
            .get_mut("basis_claim_ids")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        basis_claim_ids.retain(|basis_id| {
            basis_id
                .as_str()
                .is_none_or(|basis_id| !recommendation_ids.contains(basis_id))
        });
        let retained_basis_ids = basis_claim_ids
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect::<HashSet<_>>();
        let Some(derivation) = claim
            .get_mut("derivation")
            .and_then(serde_json::Value::as_object_mut)
        else {
            continue;
        };
        let Some(input_claim_ids) = derivation
            .get_mut("input_claim_ids")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        input_claim_ids.retain(|input_id| {
            input_id
                .as_str()
                .is_none_or(|input_id| retained_basis_ids.contains(input_id))
        });
        if input_claim_ids.is_empty() {
            claim["derivation"] = serde_json::Value::Null;
        }
    }
}

fn normalize_typed_derivation_prose(
    claims: &mut [serde_json::Value],
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
) {
    let mut opaque_ids = claims
        .iter()
        .filter_map(|claim| claim.get("id").and_then(serde_json::Value::as_str))
        .map(str::to_string)
        .collect::<HashSet<_>>();
    opaque_ids.extend(
        context
            .tracks
            .iter()
            .filter_map(|track| track.get("id").and_then(serde_json::Value::as_str))
            .map(str::to_string),
    );
    for source in typed_closed_sources(catalog, context) {
        opaque_ids.insert(source.id);
        opaque_ids.extend(source.chunks.into_iter().map(|chunk| chunk.id));
    }
    for claim in claims {
        let leaks_opaque_id = claim
            .pointer("/derivation/method")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|method| {
                method
                    .split(|character: char| {
                        !(character.is_ascii_alphanumeric()
                            || matches!(character, '.' | '_' | ':' | '-'))
                    })
                    .any(|token| !token.is_empty() && opaque_ids.contains(token))
            });
        if leaks_opaque_id {
            claim["derivation"] = serde_json::Value::Null;
        }
    }
}

fn typed_closed_sources(
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
) -> Vec<TypedClosedSource> {
    let valid_track_ids = context
        .tracks
        .iter()
        .filter_map(|track| track.get("id").and_then(serde_json::Value::as_str))
        .collect::<HashSet<_>>();
    catalog
        .sources
        .iter()
        .enumerate()
        .filter(|(_, source)| source.claim_eligible && source.semantically_admitted)
        .filter_map(|(catalog_index, source)| {
            let mut relevant_track_ids = source
                .relevant_track_ids
                .iter()
                .filter(|track_id| valid_track_ids.contains(track_id.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            relevant_track_ids.sort();
            relevant_track_ids.dedup();
            if relevant_track_ids.is_empty() {
                return None;
            }
            let chunks = selected_source_chunks_for_proposal(source)
                .into_iter()
                .enumerate()
                .map(|(index, text)| TypedClosedChunk {
                    id: format!("{}:chunk:{}", source.alias, index + 1),
                    text: text.to_string(),
                })
                .collect::<Vec<_>>();
            (!chunks.is_empty()).then(|| TypedClosedSource {
                catalog_index,
                id: source.alias.clone(),
                title: source.title.clone(),
                anchor: source.anchor.clone(),
                transport: typed_compiler_transport(&source.anchor),
                relevant_track_ids,
                chunks,
            })
        })
        .collect()
}

fn typed_compiler_transport(anchor: &str) -> TypedCompilerTransport {
    if reqwest::Url::parse(anchor).is_ok_and(|url| matches!(url.scheme(), "http" | "https")) {
        TypedCompilerTransport::Web
    } else {
        TypedCompilerTransport::Workspace
    }
}

fn typed_dimension_ids(context: &DeepResearchReportContext) -> Result<Vec<String>, String> {
    let ids = context
        .tracks
        .iter()
        .map(|track| {
            track
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| "typed report context contains a track without an ID".to_string())
        })
        .collect::<Result<Vec<_>, String>>()?;
    if ids.is_empty() {
        return Err("typed report context contains no dimensions".to_string());
    }
    Ok(ids)
}
