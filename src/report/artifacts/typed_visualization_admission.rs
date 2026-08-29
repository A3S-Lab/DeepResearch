fn admit_typed_visualizations(
    proposed: Vec<TypedWireVisualization>,
    accepted_claims: &[serde_json::Value],
    compiled: &crate::research::compiler::CompiledEvidenceReport,
    sources: &[TypedClosedSource],
    context: &DeepResearchReportContext,
    output_language: &str,
) -> (Vec<TypedWireVisualization>, Vec<AdmittedTypedVisualization>) {
    let claims_by_id = accepted_claims
        .iter()
        .filter_map(|claim| {
            claim
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(|id| (id, claim))
        })
        .collect::<std::collections::HashMap<_, _>>();
    let support_by_id = compiled
        .claim_support
        .iter()
        .enumerate()
        .map(|(index, support)| (support.claim_id.as_str(), (index + 1, support)))
        .collect::<std::collections::HashMap<_, _>>();
    let dimension_ids = context
        .tracks
        .iter()
        .filter_map(|track| track.get("id").and_then(serde_json::Value::as_str))
        .collect::<HashSet<_>>();
    let mut admitted_dimensions = HashSet::new();
    let mut admitted_wire = Vec::new();
    let mut admitted = Vec::new();

    for visualization in proposed {
        if !dimension_ids.contains(visualization.dimension_id.as_str())
            || admitted_dimensions.contains(visualization.dimension_id.as_str())
            || !typed_visualization_matches_output_language(&visualization, output_language)
        {
            continue;
        }
        let Some(candidate) = admit_typed_visualization(
            &visualization,
            &claims_by_id,
            &support_by_id,
            sources,
        ) else {
            continue;
        };
        admitted_dimensions.insert(visualization.dimension_id.clone());
        admitted_wire.push(visualization);
        admitted.push(candidate);
    }
    (admitted_wire, admitted)
}

fn admit_typed_visualization(
    visualization: &TypedWireVisualization,
    claims_by_id: &std::collections::HashMap<&str, &serde_json::Value>,
    support_by_id: &std::collections::HashMap<
        &str,
        (usize, &crate::research::compiler::CompilerClaimSupport),
    >,
    sources: &[TypedClosedSource],
) -> Option<AdmittedTypedVisualization> {
    let caption_claim_id = visualization.caption_claim_id.as_str();
    let caption_claim = claims_by_id.get(caption_claim_id)?;
    let (caption_number, caption_support) = support_by_id.get(caption_claim_id)?;
    let caption_text = caption_claim.get("text")?.as_str()?;
    if caption_support.dimension_id != visualization.dimension_id
        || caption_support.kind == crate::research::compiler::CompilerClaimKind::Recommendation
        || !typed_visual_contains_case_insensitive(caption_text, &visualization.title)
    {
        return None;
    }
    let caption = Some((caption_text.to_string(), *caption_number));
    let mut points = Vec::with_capacity(visualization.points.len());
    let mut x_values = HashSet::new();
    let mut point_pairs = HashSet::new();
    for point in &visualization.points {
        if !typed_visual_point_label_is_grounded(point, claims_by_id, sources) {
            return None;
        }
        let x = admit_typed_visual_coordinate(
            &point.x,
            &visualization.x_axis,
            &visualization.dimension_id,
            claims_by_id,
            support_by_id,
            sources,
        )?;
        let y = admit_typed_visual_coordinate(
            &point.y,
            &visualization.y_axis,
            &visualization.dimension_id,
            claims_by_id,
            support_by_id,
            sources,
        )?;
        let x_key = admitted_visual_value_key(&x.value);
        let y_key = admitted_visual_value_key(&y.value);
        if matches!(
            visualization.kind,
            TypedWireVisualizationKind::Bar
                | TypedWireVisualizationKind::Line
                | TypedWireVisualizationKind::Timeline
                | TypedWireVisualizationKind::Kpi
        ) && !x_values.insert(x_key.clone())
        {
            return None;
        }
        if !point_pairs.insert((x_key, y_key)) {
            return None;
        }
        points.push(AdmittedVisualPoint {
            label: point.label.clone(),
            x,
            y,
        });
    }
    Some(AdmittedTypedVisualization {
        dimension_id: visualization.dimension_id.clone(),
        kind: visualization.kind,
        title: visualization.title.clone(),
        caption,
        x_axis: visualization.x_axis.clone(),
        y_axis: visualization.y_axis.clone(),
        points,
    })
}

fn typed_visual_point_label_is_grounded(
    point: &TypedWireVisualPoint,
    claims_by_id: &std::collections::HashMap<&str, &serde_json::Value>,
    sources: &[TypedClosedSource],
) -> bool {
    [&point.x, &point.y].into_iter().any(|coordinate| {
        let Some(claim) = claims_by_id.get(coordinate.claim_id.as_str()) else {
            return false;
        };
        claim
            .get("text")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|text| typed_visual_contains_case_insensitive(text, &point.label))
            || typed_visual_claim_chunks(claim, sources)
                .into_iter()
                .any(|chunk| typed_visual_contains_case_insensitive(chunk, &point.label))
    })
}

fn admit_typed_visual_coordinate(
    coordinate: &TypedWireVisualCoordinate,
    axis: &TypedWireVisualAxis,
    dimension_id: &str,
    claims_by_id: &std::collections::HashMap<&str, &serde_json::Value>,
    support_by_id: &std::collections::HashMap<
        &str,
        (usize, &crate::research::compiler::CompilerClaimSupport),
    >,
    sources: &[TypedClosedSource],
) -> Option<AdmittedVisualCoordinate> {
    let claim = *claims_by_id.get(coordinate.claim_id.as_str())?;
    let (claim_number, support) = *support_by_id.get(coordinate.claim_id.as_str())?;
    if support.dimension_id != dimension_id
        || support.kind != crate::research::compiler::CompilerClaimKind::Fact
        || claim.get("kind").and_then(serde_json::Value::as_str) != Some("fact")
        || claim
            .get("dimension_id")
            .and_then(serde_json::Value::as_str)
            != Some(dimension_id)
    {
        return None;
    }
    let cited_chunks = typed_visual_claim_chunks(claim, sources);
    let relevant_chunks = cited_chunks
        .iter()
        .copied()
        .filter(|chunk| chunk.contains(&coordinate.verbatim))
        .filter(|chunk| typed_visual_chunk_matches_axis(chunk, axis))
        .collect::<Vec<_>>();
    if relevant_chunks.is_empty() {
        return None;
    }
    let claim_text = claim.get("text")?.as_str()?;
    let value = match axis.scale {
        TypedWireVisualScale::Quantitative => {
            let value = coordinate.value.as_f64()?;
            if !typed_visual_number_appears_in_text(&coordinate.verbatim, value)
                || !typed_visual_number_appears_in_text(claim_text, value)
            {
                return None;
            }
            AdmittedVisualValue::Number(value)
        }
        TypedWireVisualScale::Nominal | TypedWireVisualScale::Temporal => {
            let value = coordinate.value.as_str()?;
            if !relevant_chunks
                .iter()
                .any(|chunk| typed_visual_contains_case_insensitive(chunk, value))
                || (!typed_visual_contains_case_insensitive(claim_text, value)
                    && !typed_visual_contains_case_insensitive(
                        claim_text,
                        &coordinate.verbatim,
                    ))
            {
                return None;
            }
            AdmittedVisualValue::Text(value.to_string())
        }
    };
    Some(AdmittedVisualCoordinate {
        value,
        claim_number,
    })
}

fn typed_visual_claim_chunks<'a>(
    claim: &serde_json::Value,
    sources: &'a [TypedClosedSource],
) -> Vec<&'a str> {
    claim
        .get("evidence_refs")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .flat_map(|reference| {
            let Some(source_id) = reference
                .get("source_id")
                .and_then(serde_json::Value::as_str)
            else {
                return Vec::new();
            };
            let Some(source) = sources.iter().find(|source| source.id == source_id) else {
                return Vec::new();
            };
            let chunk_ids = reference
                .get("chunk_ids")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .collect::<HashSet<_>>();
            source
                .chunks
                .iter()
                .filter(|chunk| chunk_ids.contains(chunk.id.as_str()))
                .map(|chunk| chunk.text.as_str())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn typed_visual_chunk_matches_axis(chunk: &str, axis: &TypedWireVisualAxis) -> bool {
    if axis.scale != TypedWireVisualScale::Quantitative {
        return true;
    }
    let Some(measure) = axis.evidence_measure.as_deref() else {
        return false;
    };
    let Some(unit) = axis.unit.as_ref() else {
        return false;
    };
    typed_visual_contains_case_insensitive(chunk, measure)
        && typed_visual_contains_case_insensitive(chunk, &unit.evidence_token)
}

fn typed_visualization_matches_output_language(
    visualization: &TypedWireVisualization,
    output_language: &str,
) -> bool {
    let mut text = format!(
        "{}\n{}\n{}",
        visualization.title, visualization.x_axis.label, visualization.y_axis.label
    );
    for axis in [&visualization.x_axis, &visualization.y_axis] {
        if let Some(unit) = &axis.unit {
            text.push('\n');
            text.push_str(&unit.display_label);
        }
    }
    for point in &visualization.points {
        text.push('\n');
        text.push_str(&point.label);
    }
    crate::language::reader_text_matches_output_language(&text, output_language)
}

fn typed_visual_contains_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack.to_lowercase().contains(&needle.to_lowercase())
}

fn admitted_visual_value_key(value: &AdmittedVisualValue) -> String {
    match value {
        AdmittedVisualValue::Text(value) => format!("s:{value}"),
        AdmittedVisualValue::Number(value) => format!("n:{value:.12}"),
    }
}

fn typed_visual_number_appears_in_text(text: &str, expected: f64) -> bool {
    typed_visual_numeric_tokens(text)
        .into_iter()
        .flat_map(|token| typed_visual_number_candidates(&token))
        .any(|candidate| typed_visual_numbers_equal(candidate, expected))
}

fn typed_visual_numeric_tokens(text: &str) -> Vec<String> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut tokens = Vec::new();
    let mut index = 0usize;
    while index < characters.len() {
        let signed = matches!(characters[index], '+' | '-' | '−')
            && characters.get(index + 1).is_some_and(char::is_ascii_digit);
        if !characters[index].is_ascii_digit() && !signed {
            index += 1;
            continue;
        }
        let start = index;
        index += 1;
        while index < characters.len()
            && (characters[index].is_ascii_digit()
                || matches!(characters[index], '.' | ',' | '_' | ' ' | '\u{00a0}'))
        {
            index += 1;
        }
        let token = characters[start..index]
            .iter()
            .collect::<String>()
            .trim_end()
            .to_string();
        if token.chars().any(|character| character.is_ascii_digit()) {
            tokens.push(token);
        }
    }
    tokens
}

fn typed_visual_number_candidates(token: &str) -> Vec<f64> {
    let compact = token
        .replace([' ', '_', '\u{00a0}'], "")
        .replace('−', "-");
    let mut representations = vec![compact.clone()];
    representations.push(compact.replace(',', ""));
    representations.push(compact.replace('.', ""));
    if compact.contains(',') {
        if let Some(index) = compact.rfind(',') {
            let mut decimal = compact.replace(',', "");
            let digits_after = compact[index + 1..]
                .chars()
                .filter(char::is_ascii_digit)
                .count();
            if digits_after > 0 && decimal.len() > digits_after {
                decimal.insert(decimal.len() - digits_after, '.');
                representations.push(decimal);
            }
        }
    }
    representations
        .into_iter()
        .filter_map(|candidate| candidate.parse::<f64>().ok())
        .filter(|candidate| candidate.is_finite())
        .collect()
}

fn typed_visual_numbers_equal(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1e-9_f64.max(right.abs() * 1e-9)
}
