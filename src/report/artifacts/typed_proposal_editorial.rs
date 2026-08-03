const COMPREHENSIVE_MIN_NARRATIVE_PARAGRAPHS_PER_DIMENSION: usize = 4;
const MAX_REPEATED_CLAIM_OPENING: usize = 2;
const CLAIM_OPENING_CHARACTER_LIMIT: usize = 28;
const NEAR_DUPLICATE_SHINGLE_SIZE: usize = 3;
const NEAR_DUPLICATE_SIMILARITY_PERCENT: usize = 90;

fn normalize_typed_narrative_dependency_order(
    claims: &[serde_json::Value],
    plan: &mut TypedWireNarrativePlan,
) {
    let basis_by_claim = claims
        .iter()
        .filter_map(|claim| {
            let id = claim.get("id").and_then(serde_json::Value::as_str)?;
            let basis_ids = claim
                .get("basis_claim_ids")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<HashSet<_>>();
            Some((id.to_string(), basis_ids))
        })
        .collect::<std::collections::HashMap<_, _>>();

    for section in &mut plan.sections {
        let claim_paragraphs = section
            .paragraphs
            .iter()
            .enumerate()
            .flat_map(|(paragraph_index, paragraph)| {
                paragraph
                    .claim_ids
                    .iter()
                    .map(move |claim_id| (claim_id.as_str(), paragraph_index))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let mut paragraph_dependencies = vec![HashSet::<usize>::new(); section.paragraphs.len()];
        for (paragraph_index, paragraph) in section.paragraphs.iter().enumerate() {
            for claim_id in &paragraph.claim_ids {
                for basis_id in basis_by_claim.get(claim_id).into_iter().flatten() {
                    let Some(basis_paragraph) = claim_paragraphs.get(basis_id.as_str()) else {
                        continue;
                    };
                    if *basis_paragraph != paragraph_index {
                        paragraph_dependencies[paragraph_index].insert(*basis_paragraph);
                    }
                }
            }
        }
        if let Some(order) = stable_topological_indexes(&paragraph_dependencies) {
            let original = section.paragraphs.clone();
            section.paragraphs = order
                .into_iter()
                .map(|index| original[index].clone())
                .collect();
        }
        for paragraph in &mut section.paragraphs {
            let claim_indexes = paragraph
                .claim_ids
                .iter()
                .enumerate()
                .map(|(index, claim_id)| (claim_id.as_str(), index))
                .collect::<std::collections::HashMap<_, _>>();
            let mut claim_dependencies = vec![HashSet::<usize>::new(); paragraph.claim_ids.len()];
            for (claim_index, claim_id) in paragraph.claim_ids.iter().enumerate() {
                for basis_id in basis_by_claim.get(claim_id).into_iter().flatten() {
                    if let Some(basis_index) = claim_indexes.get(basis_id.as_str()) {
                        if *basis_index != claim_index {
                            claim_dependencies[claim_index].insert(*basis_index);
                        }
                    }
                }
            }
            if let Some(order) = stable_topological_indexes(&claim_dependencies) {
                let original = paragraph.claim_ids.clone();
                paragraph.claim_ids = order
                    .into_iter()
                    .map(|index| original[index].clone())
                    .collect();
            }
        }
    }
}

fn stable_topological_indexes(dependencies: &[HashSet<usize>]) -> Option<Vec<usize>> {
    let mut emitted = HashSet::<usize>::new();
    let mut order = Vec::with_capacity(dependencies.len());
    while order.len() < dependencies.len() {
        let next = (0..dependencies.len()).find(|index| {
            !emitted.contains(index)
                && dependencies[*index]
                    .iter()
                    .all(|dependency| emitted.contains(dependency))
        })?;
        emitted.insert(next);
        order.push(next);
    }
    Some(order)
}

fn validate_typed_narrative_shape(plan: &TypedWireNarrativePlan) -> Result<(), String> {
    if plan.sections.is_empty() || plan.sections.len() > 8 {
        return Err("typed report narrative returned an invalid section count".to_string());
    }
    for section in &plan.sections {
        let heading_chars = section.heading.chars().count();
        if section.heading.trim() != section.heading
            || !(2..=TYPED_REPORT_MAX_SECTION_HEADING_CHARS).contains(&heading_chars)
            || section.heading.chars().any(char::is_control)
            || section.paragraphs.len() > TYPED_REPORT_MAX_PARAGRAPHS_PER_SECTION
        {
            return Err("typed report narrative returned an invalid section".to_string());
        }
        if section.paragraphs.iter().any(|paragraph| {
            paragraph.claim_ids.is_empty()
                || paragraph.claim_ids.len() > TYPED_REPORT_MAX_CLAIMS_PER_PARAGRAPH
                || paragraph
                    .claim_ids
                    .iter()
                    .any(|claim_id| !typed_narrative_identifier(claim_id))
        }) {
            return Err("typed report narrative returned an invalid paragraph".to_string());
        }
    }
    Ok(())
}

fn validate_typed_narrative_plan(
    wire: &TypedWireReportProposal,
    context: &DeepResearchReportContext,
) -> Result<(), String> {
    validate_typed_narrative_shape(&wire.narrative)?;
    if wire.narrative.sections.len() != context.tracks.len() {
        return Err(
            "typed report narrative must contain exactly one section per research dimension"
                .to_string(),
        );
    }

    let mut section_ids = HashSet::<&str>::new();
    let mut normalized_headings = HashSet::<String>::new();
    let claims_by_id = wire
        .claims
        .iter()
        .filter_map(|claim| {
            let id = claim.get("id").and_then(serde_json::Value::as_str)?;
            Some((id, claim))
        })
        .collect::<std::collections::HashMap<_, _>>();

    for track in &context.tracks {
        let dimension_id = track
            .get("id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "typed report narrative received an invalid track".to_string())?;
        let sections = wire
            .narrative
            .sections
            .iter()
            .filter(|section| section.dimension_id == dimension_id)
            .collect::<Vec<_>>();
        if sections.len() != 1 || !section_ids.insert(dimension_id) {
            return Err(
                "typed report narrative must identify every research dimension exactly once"
                    .to_string(),
            );
        }
        let section = sections[0];
        let normalized_heading = normalize_editorial_text(&section.heading);
        if normalized_heading.is_empty() || !normalized_headings.insert(normalized_heading) {
            return Err("typed report narrative section headings must be distinct".to_string());
        }

        let expected_claim_ids = wire
            .claims
            .iter()
            .filter(|claim| {
                claim.get("dimension_id").and_then(serde_json::Value::as_str)
                    == Some(dimension_id)
                    && claim.get("placement").and_then(serde_json::Value::as_str)
                        == Some("finding")
            })
            .filter_map(|claim| {
                claim
                    .get("id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>();
        let planned_claim_ids = section
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.claim_ids.iter().cloned())
            .collect::<Vec<_>>();
        let expected_claim_id_set = expected_claim_ids.iter().collect::<HashSet<_>>();
        let planned_claim_id_set = planned_claim_ids.iter().collect::<HashSet<_>>();
        if expected_claim_ids.len() != planned_claim_ids.len()
            || expected_claim_id_set != planned_claim_id_set
        {
            return Err(
                "typed report narrative must place every supporting claim exactly once"
                    .to_string(),
            );
        }
        let positions = planned_claim_ids
            .iter()
            .enumerate()
            .map(|(index, claim_id)| (claim_id.as_str(), index))
            .collect::<std::collections::HashMap<_, _>>();
        for claim_id in &planned_claim_ids {
            let Some(claim) = claims_by_id.get(claim_id.as_str()) else {
                return Err("typed report narrative references an unknown claim".to_string());
            };
            let claim_position = positions[claim_id.as_str()];
            let basis_is_ordered = claim
                .get("basis_claim_ids")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .all(|basis_id| {
                    positions
                        .get(basis_id)
                        .is_none_or(|basis_position| *basis_position < claim_position)
                });
            if !basis_is_ordered {
                return Err(
                    "typed report narrative must place supporting premises before dependent claims"
                        .to_string(),
                );
            }
        }

        for paragraph in &section.paragraphs {
            let roles = paragraph
                .claim_ids
                .iter()
                .filter_map(|claim_id| claims_by_id.get(claim_id.as_str()))
                .filter_map(|claim| {
                    claim
                        .get("analysis_role")
                        .and_then(serde_json::Value::as_str)
                })
                .collect::<Vec<_>>();
            let purpose_matches = match paragraph.purpose {
                TypedWireNarrativePurpose::Evidence => roles.contains(&"evidence"),
                TypedWireNarrativePurpose::Synthesis => roles
                    .iter()
                    .any(|role| matches!(*role, "comparison" | "explanation")),
                TypedWireNarrativePurpose::Implication => roles.contains(&"implication"),
                TypedWireNarrativePurpose::Boundary => roles
                    .iter()
                    .any(|role| matches!(*role, "challenge" | "boundary")),
            };
            if !purpose_matches {
                return Err(
                    "typed report narrative paragraph purpose disagrees with its claims"
                        .to_string(),
                );
            }
        }

    }

    if wire
        .narrative
        .sections
        .iter()
        .any(|section| !section_ids.contains(section.dimension_id.as_str()))
    {
        return Err("typed report narrative introduced an unknown dimension".to_string());
    }
    validate_typed_claim_variety(&wire.claims)
}

fn typed_narrative_has_required_depth(
    wire: &TypedWireReportProposal,
    context: &DeepResearchReportContext,
    unresolved_dimension_ids: &HashSet<String>,
) -> bool {
    if context.scope != DeepResearchReportScope::Comprehensive {
        return true;
    }
    context.tracks.iter().all(|track| {
        let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
            return false;
        };
        let material = track
            .get("material")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if !material || unresolved_dimension_ids.contains(dimension_id) {
            return true;
        }
        let Some(section) = wire
            .narrative
            .sections
            .iter()
            .find(|section| section.dimension_id == dimension_id)
        else {
            return false;
        };
        let claims = wire
            .claims
            .iter()
            .filter(|claim| {
                claim.get("dimension_id").and_then(serde_json::Value::as_str)
                    == Some(dimension_id)
            })
            .collect::<Vec<_>>();
        let has_conclusion = claims.iter().any(|claim| {
            claim.get("placement").and_then(serde_json::Value::as_str)
                == Some("direct_answer")
                && claim
                    .get("analysis_role")
                    .and_then(serde_json::Value::as_str)
                    == Some("conclusion")
        });
        let evidence_findings = claims
            .iter()
            .filter(|claim| {
                claim.get("placement").and_then(serde_json::Value::as_str) == Some("finding")
                    && claim.get("kind").and_then(serde_json::Value::as_str) == Some("fact")
                    && claim
                        .get("analysis_role")
                        .and_then(serde_json::Value::as_str)
                        == Some("evidence")
            })
            .count();
        let role_count = |roles: &[&str]| {
            claims
                .iter()
                .filter(|claim| {
                    claim
                        .get("analysis_role")
                        .and_then(serde_json::Value::as_str)
                        .is_some_and(|role| roles.contains(&role))
                })
                .count()
        };
        if !has_conclusion
            || evidence_findings < COMPREHENSIVE_DIMENSION_MIN_FACT_FINDINGS
            || role_count(&["comparison"]) < COMPREHENSIVE_DIMENSION_MIN_COMPARISONS
            || role_count(&["explanation"]) < COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS
            || role_count(&["implication"]) < COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS
            || role_count(&["challenge", "boundary"])
                < COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES
        {
            // The compiler may preserve useful claims from this dimension and
            // attach a Host-owned claim-depth gap. Narrative-purpose checks
            // apply only after the semantic role chain itself is complete.
            return true;
        }
        let purposes = section
            .paragraphs
            .iter()
            .map(|paragraph| paragraph.purpose)
            .collect::<HashSet<_>>();
        section.paragraphs.len() >= COMPREHENSIVE_MIN_NARRATIVE_PARAGRAPHS_PER_DIMENSION
            && purposes.contains(&TypedWireNarrativePurpose::Evidence)
            && purposes.contains(&TypedWireNarrativePurpose::Synthesis)
            && purposes.contains(&TypedWireNarrativePurpose::Implication)
            && purposes.contains(&TypedWireNarrativePurpose::Boundary)
    })
}

fn reconcile_typed_narrative_after_demotions(
    wire: &mut TypedWireReportProposal,
    demoted_claim_ids: &HashSet<String>,
) {
    if demoted_claim_ids.is_empty() {
        return;
    }
    for section in &mut wire.narrative.sections {
        let expected = wire
            .claims
            .iter()
            .filter(|claim| {
                claim.get("dimension_id").and_then(serde_json::Value::as_str)
                    == Some(section.dimension_id.as_str())
                    && claim.get("placement").and_then(serde_json::Value::as_str)
                        == Some("finding")
            })
            .filter_map(|claim| {
                Some((
                    claim.get("id")?.as_str()?.to_string(),
                    claim.get("kind")?.as_str()?.to_string(),
                    claim
                        .get("analysis_role")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                ))
            })
            .collect::<Vec<_>>();
        let planned = section
            .paragraphs
            .iter()
            .flat_map(|paragraph| paragraph.claim_ids.iter().cloned())
            .collect::<Vec<_>>();
        let retained_expected = expected
            .iter()
            .filter(|(claim_id, _, _)| !demoted_claim_ids.contains(claim_id))
            .map(|(claim_id, _, _)| claim_id.clone())
            .collect::<Vec<_>>();
        let missing = expected
            .iter()
            .filter(|(claim_id, _, _)| !planned.contains(claim_id))
            .map(|(claim_id, _, _)| claim_id)
            .collect::<HashSet<_>>();
        if missing.is_empty()
            || planned != retained_expected
            || !missing
                .iter()
                .all(|claim_id| demoted_claim_ids.contains(claim_id.as_str()))
        {
            continue;
        }
        let planned_purposes = section
            .paragraphs
            .iter()
            .flat_map(|paragraph| {
                paragraph
                    .claim_ids
                    .iter()
                    .map(|claim_id| (claim_id.clone(), paragraph.purpose))
            })
            .collect::<std::collections::HashMap<_, _>>();
        section.paragraphs = expected
            .into_iter()
            .map(|(claim_id, kind, role)| TypedWireNarrativeParagraph {
                purpose: planned_purposes
                    .get(&claim_id)
                    .copied()
                    .unwrap_or_else(|| match role.as_str() {
                        "evidence" => TypedWireNarrativePurpose::Evidence,
                        "comparison" | "explanation" => TypedWireNarrativePurpose::Synthesis,
                        "implication" => TypedWireNarrativePurpose::Implication,
                        "challenge" | "boundary" => TypedWireNarrativePurpose::Boundary,
                        _ if kind == "fact" => TypedWireNarrativePurpose::Evidence,
                        _ if kind == "recommendation" => {
                            TypedWireNarrativePurpose::Implication
                        }
                        _ => TypedWireNarrativePurpose::Synthesis,
                    }),
                claim_ids: vec![claim_id],
            })
            .collect();
    }
}

fn typed_narrative_compiler_value(plan: &TypedWireNarrativePlan) -> serde_json::Value {
    serde_json::json!({
        "sections": plan.sections.iter().map(|section| {
            serde_json::json!({
                "dimension_id": section.dimension_id,
                "heading": section.heading,
                "paragraphs": section.paragraphs.iter().map(|paragraph| {
                    serde_json::json!({
                        "claim_ids": paragraph.claim_ids,
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
    })
}

fn validate_typed_claim_variety(claims: &[serde_json::Value]) -> Result<(), String> {
    let texts = claims
        .iter()
        .filter_map(|claim| claim.get("text").and_then(serde_json::Value::as_str))
        .collect::<Vec<_>>();
    let mut opening_counts = std::collections::HashMap::<String, usize>::new();
    for text in &texts {
        let opening = claim_opening_key(text);
        if opening.is_empty() {
            continue;
        }
        let count = opening_counts.entry(opening).or_default();
        *count += 1;
        if *count > MAX_REPEATED_CLAIM_OPENING {
            return Err(
                "typed report narrative repeats the same claim opening too often".to_string(),
            );
        }
    }
    for (index, left) in texts.iter().enumerate() {
        for right in texts.iter().skip(index + 1) {
            if near_duplicate_claims(left, right) {
                return Err(
                    "typed report narrative contains near-duplicate claim prose".to_string(),
                );
            }
        }
    }
    Ok(())
}

fn claim_opening_key(text: &str) -> String {
    normalize_editorial_text(text)
        .chars()
        .take(CLAIM_OPENING_CHARACTER_LIMIT)
        .collect()
}

fn near_duplicate_claims(left: &str, right: &str) -> bool {
    let left = normalize_editorial_text(left);
    let right = normalize_editorial_text(right);
    if left.chars().count() < 48 || right.chars().count() < 48 {
        return left == right;
    }
    let left_shingles = editorial_shingles(&left);
    let right_shingles = editorial_shingles(&right);
    if left_shingles.is_empty() || right_shingles.is_empty() {
        return left == right;
    }
    let intersection = left_shingles.intersection(&right_shingles).count();
    let union = left_shingles.union(&right_shingles).count();
    intersection.saturating_mul(100)
        >= union.saturating_mul(NEAR_DUPLICATE_SIMILARITY_PERCENT)
}

fn editorial_shingles(value: &str) -> HashSet<String> {
    let characters = value.chars().collect::<Vec<_>>();
    characters
        .windows(NEAR_DUPLICATE_SHINGLE_SIZE)
        .map(|window| window.iter().collect())
        .collect()
}

fn normalize_editorial_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn typed_narrative_identifier(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && value.chars().count() <= 64
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-')
        })
}
