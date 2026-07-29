// Request-requirement validation and track mapping for planner contracts.

fn validated_request_requirements(
    object: &Map<String, Value>,
    required: bool,
) -> Result<Option<BTreeSet<String>>, String> {
    let Some(value) = object.get("request_requirements") else {
        return if required {
            Err("DeepResearch plan omitted `request_requirements`".to_string())
        } else {
            Ok(None)
        };
    };
    let requirements = value.as_array().ok_or_else(|| {
        "DeepResearch plan `request_requirements` is not an array".to_string()
    })?;
    if requirements.is_empty() || requirements.len() > MAX_PLANNER_REQUEST_REQUIREMENTS {
        return Err(format!(
            "DeepResearch plan `request_requirements` must contain between 1 and {MAX_PLANNER_REQUEST_REQUIREMENTS} items"
        ));
    }
    let mut ids = BTreeSet::new();
    for requirement in requirements {
        let requirement = requirement.as_object().ok_or_else(|| {
            "DeepResearch plan contains a non-object request requirement".to_string()
        })?;
        reject_unknown_fields(requirement, &["id", "text"], "request requirement")?;
        let id = required_text(requirement, "id")?;
        let text = required_text(requirement, "text")?;
        if !is_stable_plan_id(id) {
            return Err(format!(
                "DeepResearch request requirement id `{id}` is not a stable ASCII identifier"
            ));
        }
        if text.chars().count() > 300 || text.chars().any(char::is_control) {
            return Err(
                "DeepResearch request requirement contains invalid reader text".to_string(),
            );
        }
        if !ids.insert(id.to_string()) {
            return Err(format!(
                "duplicate DeepResearch request requirement id `{id}`"
            ));
        }
    }
    Ok(Some(ids))
}

fn validated_track_requirement_ids(
    track: &Map<String, Value>,
    declared_requirement_ids: Option<&BTreeSet<String>>,
    required: bool,
) -> Result<Vec<String>, String> {
    let Some(value) = track.get("requirement_ids") else {
        return if required {
            Err("DeepResearch track omitted `requirement_ids`".to_string())
        } else {
            Ok(Vec::new())
        };
    };
    let requirement_ids = exact_string_array(
        Some(value),
        "track requirement_ids",
        MAX_PLANNER_REQUEST_REQUIREMENTS,
    )?;
    if requirement_ids.is_empty() {
        return Err("DeepResearch track has no mapped request requirement".to_string());
    }
    let mut unique_ids = BTreeSet::new();
    for id in &requirement_ids {
        if !is_stable_plan_id(id) || !unique_ids.insert(id.as_str()) {
            return Err(
                "DeepResearch track contains an invalid or duplicate request requirement ID"
                    .to_string(),
            );
        }
        let Some(declared_requirement_ids) = declared_requirement_ids else {
            return Err(
                "DeepResearch track maps request requirements without declaring them".to_string(),
            );
        };
        if !declared_requirement_ids.contains(id) {
            return Err(format!(
                "DeepResearch track maps unknown request requirement `{id}`"
            ));
        }
    }
    Ok(requirement_ids)
}

fn validate_request_requirement_coverage(
    declared: &BTreeSet<String>,
    mapped: &BTreeSet<String>,
    resource: &str,
) -> Result<(), String> {
    if declared == mapped {
        return Ok(());
    }
    let missing = declared.difference(mapped).cloned().collect::<Vec<_>>();
    Err(format!(
        "DeepResearch {resource} left request requirement(s) unmapped: {}",
        missing.join(", ")
    ))
}
