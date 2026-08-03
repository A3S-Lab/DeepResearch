const DEEP_RESEARCH_LOOP_STAGES: [&str; 9] = [
    "bootstrap_acquisition",
    "optional_outline",
    "batched_evidence_extraction",
    "host_coverage_reduction",
    "optional_gap_acquisition",
    "optional_gap_extraction",
    "report_document_generation",
    "report_editorial_planning",
    "deterministic_publication",
];

const DEEP_RESEARCH_LOOP_CARDINALITY: [&str; 7] = [
    "outline_generations",
    "initial_extractions",
    "gap_query_generations",
    "gap_extractions",
    "report_generations",
    "editorial_generations",
    "report_repairs",
];

const GENERATED_SEMANTIC_OUTLINE_FIELDS: [&str; 7] = [
    "report_title",
    "research_scope",
    "freshness_required",
    "workspace_evidence_required",
    "request_requirements",
    "tracks",
    "supplemental_queries",
];
const SEMANTIC_OUTLINE_FIELDS: [&str; 8] = [
    "report_title",
    "research_scope",
    "freshness_required",
    "workspace_evidence_required",
    "request_requirements",
    "tracks",
    "supplemental_queries",
    "stop_conditions",
];
const TRACK_IDENTITY_FIELDS: [&str; 8] = [
    "id",
    "title",
    "focus",
    "material",
    "requirement_ids",
    "completion_criteria",
    "questions",
    "evidence_requirements",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct ValidatedPlanQuestion {
    prompt: String,
    completion_criterion_indexes: Option<Vec<usize>>,
}

pub fn validated_loop_planner(workflow_args: &Value) -> Result<&Map<String, Value>, String> {
    let contract = workflow_args
        .pointer("/input/loop_contract")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "DeepResearch host did not receive its automatic Loop Engineering contract".to_string()
        })?;
    reject_unknown_fields(
        contract,
        &[
            "version",
            "pattern",
            "goal",
            "controller",
            "quota",
            "execution",
            "cardinality",
            "planner",
            "hard_caps",
        ],
        "Loop Engineering contract",
    )?;
    if contract.get("version").and_then(Value::as_u64) != Some(1)
        || contract.get("pattern").and_then(Value::as_str)
            != Some("evidence-first-deep-research")
        || contract.get("controller").and_then(Value::as_str) != Some("host_inquiry_reducer")
    {
        return Err(
            "DeepResearch received an unsupported Loop Engineering identity contract".to_string(),
        );
    }
    let query = workflow_args
        .pointer("/input/query")
        .and_then(Value::as_str)
        .ok_or_else(|| "DeepResearch workflow omitted its query".to_string())?;
    if contract.get("goal").and_then(Value::as_str) != Some(query) {
        return Err(
            "DeepResearch Loop Engineering goal differs from the workflow query".to_string(),
        );
    }

    let quota = contract
        .get("quota")
        .and_then(Value::as_object)
        .ok_or_else(|| "DeepResearch Loop Engineering contract omitted quota".to_string())?;
    reject_unknown_fields(quota, &["mode"], "Loop Engineering quota")?;
    if quota.get("mode").and_then(Value::as_str) != Some("bounded") {
        return Err("DeepResearch Loop Engineering quota must be `bounded`".to_string());
    }

    let execution = contract
        .get("execution")
        .and_then(Value::as_object)
        .ok_or_else(|| "DeepResearch Loop Engineering contract omitted execution".to_string())?;
    reject_unknown_fields(execution, &["mode", "stages"], "Loop Engineering execution")?;
    if execution.get("mode").and_then(Value::as_str) != Some("progressively_publishable") {
        return Err(
            "DeepResearch Loop Engineering execution must be `progressively_publishable`"
                .to_string(),
        );
    }
    let stages = execution
        .get("stages")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            "DeepResearch Loop Engineering execution omitted its stage graph".to_string()
        })?;
    let stages = stages
        .iter()
        .map(Value::as_str)
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| {
            "DeepResearch Loop Engineering stage graph contains a non-string stage".to_string()
        })?;
    if stages.as_slice() != DEEP_RESEARCH_LOOP_STAGES {
        return Err(
            "DeepResearch Loop Engineering stage graph differs from the minimal pipeline"
                .to_string(),
        );
    }

    let cardinality = contract
        .get("cardinality")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "DeepResearch Loop Engineering contract omitted stage cardinality".to_string()
        })?;
    reject_unknown_fields(
        cardinality,
        &DEEP_RESEARCH_LOOP_CARDINALITY,
        "Loop Engineering cardinality",
    )?;
    for (field, expected) in [
        ("outline_generations", 1),
        ("initial_extractions", 1),
        ("gap_query_generations", MAX_GAP_ROUNDS),
        ("gap_extractions", MAX_GAP_ROUNDS),
        ("report_generations", 1),
        ("editorial_generations", 1),
        ("report_repairs", 1),
    ] {
        if cardinality.get(field).and_then(Value::as_u64) != Some(expected) {
            return Err(format!(
                "DeepResearch Loop Engineering cardinality `{field}` must be exactly {expected}"
            ));
        }
    }

    let planner = contract
        .get("planner")
        .and_then(Value::as_object)
        .ok_or_else(|| "DeepResearch Loop Engineering contract omitted its planner".to_string())?;
    reject_unknown_fields(
        planner,
        &[
            "agent",
            "description",
            "max_steps",
            "timeout_ms",
            "prompt",
            "output_schema",
        ],
        "Loop Engineering planner",
    )?;
    if planner.get("agent").and_then(Value::as_str) != Some("research-planner") {
        return Err(
            "DeepResearch Loop Engineering planner has an unsupported agent identity".to_string(),
        );
    }
    for field in ["description", "prompt"] {
        if planner
            .get(field)
            .and_then(Value::as_str)
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err(format!(
                "DeepResearch Loop Engineering planner omitted non-empty `{field}`"
            ));
        }
    }
    required_integer_in_range(
        planner,
        "timeout_ms",
        1_000,
        DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS,
        "Loop Engineering planner",
    )?;
    if !planner.get("output_schema").is_some_and(Value::is_object) {
        return Err(
            "DeepResearch Loop Engineering planner omitted its object output schema".to_string(),
        );
    }
    let schema_max_tracks = planner
        .get("output_schema")
        .and_then(|schema| schema.pointer("/properties/tracks/maxItems"))
        .and_then(Value::as_u64)
        .filter(|maximum| (1..=MAX_PLANNER_TRACK_EFFECTS).contains(maximum))
        .ok_or_else(|| {
            "DeepResearch planner output schema omitted a bounded track maximum".to_string()
        })?;
    let schema_max_supplemental_queries = planner
        .get("output_schema")
        .and_then(|schema| schema.pointer("/properties/supplemental_queries/maxItems"))
        .and_then(Value::as_u64)
        .filter(|maximum| *maximum <= MAX_PLANNER_SUPPLEMENTAL_QUERIES as u64)
        .ok_or_else(|| {
            "DeepResearch planner output schema omitted a bounded supplemental-query maximum"
                .to_string()
        })?;
    if schema_max_supplemental_queries != MAX_PLANNER_SUPPLEMENTAL_QUERIES as u64 {
        return Err(format!(
            "DeepResearch planner schema must allow exactly {MAX_PLANNER_SUPPLEMENTAL_QUERIES} supplemental queries"
        ));
    }
    if planner.get("max_steps").and_then(Value::as_u64) != Some(1) {
        return Err(
            "DeepResearch Loop Engineering planner must contain exactly one optional outline effect"
                .to_string(),
        );
    }

    let hard_caps = contract
        .get("hard_caps")
        .and_then(Value::as_object)
        .ok_or_else(|| {
            "DeepResearch Loop Engineering contract omitted its safety fuses".to_string()
        })?;
    reject_unknown_fields(
        hard_caps,
        &[
            "max_tracks",
            "max_searches",
            "max_gap_searches",
            "max_fetches",
            "max_supplemental_fetches",
            "retrieval_timeout_ms",
        ],
        "Loop Engineering safety fuses",
    )?;
    let max_tracks = required_integer_in_range(
        hard_caps,
        "max_tracks",
        1,
        MAX_PLANNER_TRACK_EFFECTS,
        "Loop Engineering safety fuses",
    )?;
    if schema_max_tracks != max_tracks {
        return Err(
            "DeepResearch planner schema track maximum differs from its safety fuse".to_string(),
        );
    }
    for (field, expected) in [
        ("max_searches", MAX_PLANNER_SEARCHES),
        ("max_gap_searches", MAX_GAP_SEARCHES),
        ("max_fetches", MAX_PLANNER_INITIAL_FETCHES),
        (
            "max_supplemental_fetches",
            MAX_PLANNER_SUPPLEMENTAL_FETCHES,
        ),
        ("retrieval_timeout_ms", 150_000),
    ] {
        if hard_caps.get(field).and_then(Value::as_u64) != Some(expected) {
            return Err(format!(
                "DeepResearch Loop Engineering safety fuse `{field}` must be {expected}"
            ));
        }
    }

    Ok(planner)
}

pub fn close_semantic_outline(mut outline: Value) -> Result<Value, String> {
    let object = outline
        .as_object_mut()
        .ok_or_else(|| "DeepResearch outline planner returned a non-object fragment".to_string())?;
    reject_unknown_fields(
        object,
        &GENERATED_SEMANTIC_OUTLINE_FIELDS,
        "generated semantic outline",
    )?;
    object.insert(
        "stop_conditions".to_string(),
        serde_json::json!([
            "Every material evidence target is resolved from traceable evidence or explicitly bounded.",
            "Any remaining limitation is disclosed and cannot make the qualified answer misleading."
        ]),
    );
    Ok(outline)
}

fn semantic_outline_track_targets(outline: &Value) -> Result<Vec<Value>, String> {
    let object = outline
        .as_object()
        .ok_or_else(|| "DeepResearch outline planner returned a non-object fragment".to_string())?;
    reject_unknown_fields(object, &SEMANTIC_OUTLINE_FIELDS, "semantic outline")?;
    required_text(object, "report_title")?;
    let research_scope = required_research_scope(object, "semantic outline")?;
    required_bool(object, "freshness_required", "semantic outline")?;
    required_bool(object, "workspace_evidence_required", "semantic outline")?;
    let request_requirement_ids = validated_request_requirements(object, true)?
        .ok_or_else(|| "DeepResearch semantic outline omitted request requirements".to_string())?;
    exact_string_array(
        object.get("supplemental_queries"),
        "supplemental_queries",
        MAX_PLANNER_SUPPLEMENTAL_QUERIES,
    )?;
    let stop_conditions = string_array(
        object.get("stop_conditions"),
        "semantic outline stop_conditions",
        3,
    )?;
    if stop_conditions.is_empty() {
        return Err("DeepResearch semantic outline has no stop condition".to_string());
    }
    let tracks = object
        .get("tracks")
        .and_then(Value::as_array)
        .filter(|tracks| !tracks.is_empty())
        .ok_or_else(|| "DeepResearch semantic outline has no track identity".to_string())?;
    let maximum_tracks = MAX_PLANNER_TRACK_EFFECTS as usize;
    if tracks.len() > maximum_tracks {
        return Err(format!(
            "DeepResearch semantic outline has {} tracks; maximum is {}",
            tracks.len(),
            maximum_tracks
        ));
    }
    let mut ids = BTreeSet::new();
    let mut mapped_requirement_ids = BTreeSet::new();
    let mut material = false;
    for track in tracks {
        let track = track.as_object().ok_or_else(|| {
            "DeepResearch semantic outline contains a non-object track".to_string()
        })?;
        reject_unknown_fields(track, &TRACK_IDENTITY_FIELDS, "outline track identity")?;
        let id = required_text(track, "id")?;
        if !is_stable_plan_id(id) {
            return Err(format!(
                "DeepResearch outline track id `{id}` is not a stable ASCII identifier"
            ));
        }
        if !ids.insert(id) {
            return Err(format!("duplicate DeepResearch outline track id `{id}`"));
        }
        required_text(track, "title")?;
        required_text(track, "focus")?;
        material |= required_bool(track, "material", "outline track identity")?;
        mapped_requirement_ids.extend(validated_track_requirement_ids(
            track,
            Some(&request_requirement_ids),
            true,
        )?);
        let completion_criteria = string_array(
            track.get("completion_criteria"),
            "outline track completion_criteria",
            MAX_PLANNER_COMPLETION_CRITERIA,
        )?;
        if completion_criteria.is_empty() {
            return Err("DeepResearch outline track has no completion criterion".to_string());
        }
        validated_plan_questions(
            track.get("questions"),
            "outline track questions",
            completion_criteria.len(),
            true,
        )?;
        let evidence_requirements = track
            .get("evidence_requirements")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                "DeepResearch outline track omitted object `evidence_requirements`".to_string()
            })?;
        reject_unknown_fields(
            evidence_requirements,
            &[
                "primary_source_required",
                "independent_corroboration_required",
            ],
            "outline track evidence requirements",
        )?;
        required_bool(
            evidence_requirements,
            "primary_source_required",
            "outline track evidence requirements",
        )?;
        required_bool(
            evidence_requirements,
            "independent_corroboration_required",
            "outline track evidence requirements",
        )?;
    }
    if !material {
        return Err("DeepResearch semantic outline has no material track".to_string());
    }
    validate_request_requirement_coverage(
        &request_requirement_ids,
        &mapped_requirement_ids,
        "semantic outline",
    )?;
    validate_structured_plan_question_roles(tracks, research_scope)?;
    Ok(tracks.clone())
}

pub fn validate_plan(value: Value) -> Result<PlannedInquiry, String> {
    let value = normalize_planner_budget(value)?;
    let object = value
        .as_object()
        .ok_or_else(|| "DeepResearch planner returned a non-object plan".to_string())?;
    reject_unknown_fields(
        object,
        &[
            "report_title",
            "research_scope",
            "freshness_required",
            "workspace_evidence_required",
            "request_requirements",
            "tracks",
            "search_queries",
            "seed_urls",
            "budget",
            "stop_conditions",
        ],
        "plan",
    )?;
    required_text(object, "report_title")?;
    let research_scope = required_research_scope(object, "plan")?;
    required_bool(object, "freshness_required", "plan")?;
    required_bool(object, "workspace_evidence_required", "plan")?;
    let request_requirement_ids = validated_request_requirements(object, false)?;
    let _search_queries = exact_string_array(
        object.get("search_queries"),
        "search_queries",
        MAX_PLANNER_SEARCHES as usize,
    )?;
    let _seed_urls = string_array(object.get("seed_urls"), "seed_urls", 3)?;
    let budget = object
        .get("budget")
        .and_then(Value::as_object)
        .ok_or_else(|| "DeepResearch plan omitted its retrieval budget".to_string())?;
    reject_unknown_fields(
        budget,
        &["retrieval_timeout_ms", "direct_searches", "direct_fetches"],
        "retrieval budget",
    )?;
    required_integer_in_range(
        budget,
        "retrieval_timeout_ms",
        30_000,
        150_000,
        "retrieval budget",
    )?;
    required_integer_in_range(
        budget,
        "direct_searches",
        0,
        MAX_PLANNER_SEARCHES,
        "retrieval budget",
    )?;
    required_integer_in_range(
        budget,
        "direct_fetches",
        0,
        MAX_PLANNER_INITIAL_FETCHES,
        "retrieval budget",
    )?;
    let (obligations, _) = research_contract_from_plan(&value)?;
    let tracks = object
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or_else(|| "DeepResearch plan did not contain stable research tracks".to_string())?;
    if tracks.len() > MAX_PLANNER_TRACK_EFFECTS as usize {
        return Err(format!(
            "DeepResearch plan has {} tracks; maximum is {}",
            tracks.len(),
            MAX_PLANNER_TRACK_EFFECTS
        ));
    }
    let mut mapped_requirement_ids = BTreeSet::new();
    for track in tracks {
        let track = track
            .as_object()
            .ok_or_else(|| "DeepResearch planner returned a non-object track".to_string())?;
        reject_unknown_fields(
            track,
            &[
                "id",
                "title",
                "focus",
                "material",
                "requirement_ids",
                "questions",
                "completion_criteria",
                "evidence_requirements",
            ],
            "track",
        )?;
        required_bool(track, "material", "track")?;
        mapped_requirement_ids.extend(validated_track_requirement_ids(
            track,
            request_requirement_ids.as_ref(),
            request_requirement_ids.is_some(),
        )?);
        let completion_criteria = string_array(
            track.get("completion_criteria"),
            "track completion_criteria",
            MAX_PLANNER_COMPLETION_CRITERIA,
        )?;
        let questions = validated_plan_questions(
            track.get("questions"),
            "track questions",
            completion_criteria.len(),
            false,
        )?;
        if questions.is_empty() {
            return Err("DeepResearch track has no research question".to_string());
        }
        if completion_criteria.is_empty() {
            return Err("DeepResearch track has no completion criterion".to_string());
        }
        let evidence_requirements = track
            .get("evidence_requirements")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                "DeepResearch track omitted object `evidence_requirements`".to_string()
            })?;
        reject_unknown_fields(
            evidence_requirements,
            &[
                "primary_source_required",
                "independent_corroboration_required",
            ],
            "track evidence requirements",
        )?;
    }
    if let Some(request_requirement_ids) = request_requirement_ids.as_ref() {
        validate_request_requirement_coverage(
            request_requirement_ids,
            &mapped_requirement_ids,
            "plan",
        )?;
    }
    validate_structured_plan_question_roles(tracks, research_scope)?;
    debug_assert!(obligations.iter().any(|obligation| obligation.material));
    Ok(PlannedInquiry { value })
}

fn validated_plan_questions(
    value: Option<&Value>,
    resource: &str,
    completion_criterion_count: usize,
    structured_required: bool,
) -> Result<Vec<ValidatedPlanQuestion>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("DeepResearch plan {resource} is not an array"))?;
    if values.is_empty() || values.len() > MAX_PLANNER_QUESTIONS_PER_TRACK {
        return Err(format!(
            "DeepResearch plan {resource} must contain between 1 and {MAX_PLANNER_QUESTIONS_PER_TRACK} items"
        ));
    }
    if completion_criterion_count == 0 {
        return Err(format!(
            "DeepResearch plan {resource} cannot map questions without completion criteria"
        ));
    }

    let contains_strings = values.iter().any(Value::is_string);
    let contains_objects = values.iter().any(Value::is_object);
    if values
        .iter()
        .any(|value| !value.is_string() && !value.is_object())
        || (contains_strings && contains_objects)
    {
        return Err(format!(
            "DeepResearch plan {resource} must contain either legacy strings or structured question objects"
        ));
    }
    if structured_required && contains_strings {
        return Err(format!(
            "DeepResearch semantic planner {resource} must use structured question objects"
        ));
    }

    if contains_strings {
        return values
            .iter()
            .map(|value| {
                let prompt = value
                    .as_str()
                    .map(str::trim)
                    .filter(|prompt| !prompt.is_empty())
                    .filter(|prompt| prompt.chars().count() <= 240)
                    .ok_or_else(|| {
                        format!(
                            "DeepResearch plan {resource} contains an invalid legacy question"
                        )
                    })?;
                Ok(ValidatedPlanQuestion {
                    prompt: prompt.to_string(),
                    completion_criterion_indexes: None,
                })
            })
            .collect();
    }

    let mut covered_criteria = BTreeSet::new();
    let questions = values
        .iter()
        .map(|value| {
            let question = value.as_object().ok_or_else(|| {
                format!("DeepResearch plan {resource} contains a non-object question")
            })?;
            reject_unknown_fields(
                question,
                &["question", "role", "completion_criterion_indexes"],
                resource,
            )?;
            let prompt = required_text(question, "question")?;
            if prompt.chars().count() > 240 || prompt.chars().any(char::is_control) {
                return Err(format!(
                    "DeepResearch plan {resource} contains an invalid question prompt"
                ));
            }
            question
                .get("role")
                .and_then(Value::as_str)
                .filter(|role| {
                    matches!(
                        *role,
                        "establish" | "compare" | "explain" | "challenge" | "decide"
                    )
                })
                .ok_or_else(|| {
                    format!("DeepResearch plan {resource} contains an unsupported question role")
                })?;
            let raw_indexes = question
                .get("completion_criterion_indexes")
                .and_then(Value::as_array)
                .filter(|indexes| !indexes.is_empty())
                .ok_or_else(|| {
                    format!(
                        "DeepResearch plan {resource} question omitted completion-criterion indexes"
                    )
                })?;
            if raw_indexes.len() > completion_criterion_count {
                return Err(format!(
                    "DeepResearch plan {resource} question maps too many completion criteria"
                ));
            }
            let mut indexes = Vec::with_capacity(raw_indexes.len());
            for raw_index in raw_indexes {
                let index = raw_index.as_u64().and_then(|index| usize::try_from(index).ok())
                    .filter(|index| *index < completion_criterion_count)
                    .ok_or_else(|| {
                        format!(
                            "DeepResearch plan {resource} question contains an invalid completion-criterion index"
                        )
                    })?;
                if indexes.contains(&index) {
                    return Err(format!(
                        "DeepResearch plan {resource} question repeats a completion-criterion index"
                    ));
                }
                indexes.push(index);
                covered_criteria.insert(index);
            }
            Ok(ValidatedPlanQuestion {
                prompt: prompt.to_string(),
                completion_criterion_indexes: Some(indexes),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    if covered_criteria.len() != completion_criterion_count {
        return Err(format!(
            "DeepResearch plan {resource} does not cover every completion criterion"
        ));
    }
    Ok(questions)
}

fn validate_structured_plan_question_roles(
    tracks: &[Value],
    research_scope: &str,
) -> Result<(), String> {
    let mut roles = BTreeSet::new();
    for track in tracks {
        let Some(questions) = track.get("questions").and_then(Value::as_array) else {
            continue;
        };
        if questions.iter().any(Value::is_string) {
            // Host fallback plans retain their legacy string questions. Their
            // generic shape is validated separately and must not pretend to
            // carry model-authored analytical roles.
            return Ok(());
        }
        roles.extend(
            questions
                .iter()
                .filter_map(|question| question.get("role").and_then(Value::as_str))
                .map(str::to_string),
        );
    }
    match research_scope {
        "focused" if !roles.contains("establish") => Err(
            "DeepResearch focused plans require at least one `establish` question".to_string(),
        ),
        "comprehensive"
            if !roles.contains("establish")
                || !roles.contains("challenge")
                || !(roles.contains("compare") || roles.contains("explain")) =>
        {
            Err(
                "DeepResearch comprehensive plans must cover `establish`, `challenge`, and `compare` or `explain` across their tracks"
                    .to_string(),
            )
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod plan_question_role_tests {
    use super::*;

    #[test]
    fn comprehensive_role_mix_is_validated_across_tracks() {
        let tracks = serde_json::json!([{
            "questions": [
                { "role": "establish" },
                { "role": "compare" },
                { "role": "challenge" }
            ]
        }, {
            "questions": [
                { "role": "establish" },
                { "role": "explain" },
                { "role": "decide" }
            ]
        }]);

        validate_structured_plan_question_roles(tracks.as_array().unwrap(), "comprehensive")
            .expect("the plan-wide role mix should admit a decision track without challenge");
    }

    #[test]
    fn comprehensive_plan_still_requires_a_global_challenge() {
        let tracks = serde_json::json!([{
            "questions": [
                { "role": "establish" },
                { "role": "compare" },
                { "role": "decide" }
            ]
        }]);

        let error = validate_structured_plan_question_roles(
            tracks.as_array().unwrap(),
            "comprehensive",
        )
        .expect_err("a comprehensive plan without any challenge must fail");

        assert!(error.contains("across their tracks"));
    }
}

fn reject_unknown_fields(
    object: &Map<String, Value>,
    allowed: &[&str],
    resource: &str,
) -> Result<(), String> {
    let unexpected = object
        .keys()
        .filter(|key| !allowed.contains(&key.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if unexpected.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "DeepResearch {resource} contains unsupported field(s): {}",
            unexpected.join(", ")
        ))
    }
}

fn required_bool(object: &Map<String, Value>, key: &str, resource: &str) -> Result<bool, String> {
    object
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("DeepResearch {resource} omitted boolean `{key}`"))
}

fn required_research_scope<'a>(
    object: &'a Map<String, Value>,
    resource: &str,
) -> Result<&'a str, String> {
    let scope = object
        .get("research_scope")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("DeepResearch {resource} omitted `research_scope`"))?;
    if matches!(scope, "focused" | "comprehensive") {
        Ok(scope)
    } else {
        Err(format!(
            "DeepResearch {resource} has unsupported research scope `{scope}`"
        ))
    }
}

fn required_integer_in_range(
    object: &Map<String, Value>,
    key: &str,
    minimum: u64,
    maximum: u64,
    resource: &str,
) -> Result<u64, String> {
    let value = object
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("DeepResearch {resource} omitted integer `{key}`"))?;
    if (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "DeepResearch {resource} `{key}` must be between {minimum} and {maximum}"
        ))
    }
}

/// Convert the LLM-authored stable tracks into the typed coverage contract
/// consumed by the replayable Inquiry reducer. This is the only planner-to-
/// state boundary for research obligations and stopping conditions.
pub fn research_contract_from_plan(
    plan: &Value,
) -> Result<(Vec<ResearchObligation>, Vec<String>), String> {
    let object = plan
        .as_object()
        .ok_or_else(|| "DeepResearch planner returned a non-object plan".to_string())?;
    let tracks = object
        .get("tracks")
        .and_then(Value::as_array)
        .filter(|tracks| !tracks.is_empty())
        .ok_or_else(|| "DeepResearch plan did not contain stable research tracks".to_string())?;
    let limits = InquiryLimits::default();
    if tracks.len() > limits.max_obligations {
        return Err(format!(
            "DeepResearch plan has {} stable research tracks; maximum is {}",
            tracks.len(),
            limits.max_obligations
        ));
    }

    let mut track_ids = BTreeSet::new();
    let mut obligations = Vec::with_capacity(tracks.len());
    for track in tracks {
        let track = track
            .as_object()
            .ok_or_else(|| "DeepResearch planner returned a non-object track".to_string())?;
        let id = required_text(track, "id")?;
        if !is_stable_plan_id(id) {
            return Err(format!(
                "DeepResearch track id `{id}` is not a stable ASCII identifier"
            ));
        }
        if !track_ids.insert(id) {
            return Err(format!("duplicate DeepResearch track id `{id}`"));
        }
        let title = required_text(track, "title")?;
        let focus = required_text(track, "focus")?;
        let material = track
            .get("material")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("DeepResearch track `{id}` omitted boolean `material`"))?;
        let completion_criteria = string_array(
            track.get("completion_criteria"),
            "track completion_criteria",
            MAX_PLANNER_COMPLETION_CRITERIA,
        )?;
        if completion_criteria.is_empty() {
            return Err(format!(
                "DeepResearch track `{id}` has no completion criterion"
            ));
        }
        let evidence_requirements = track
            .get("evidence_requirements")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                format!("DeepResearch track `{id}` omitted object `evidence_requirements`")
            })?;
        let primary_source_required = evidence_requirements
            .get("primary_source_required")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                format!("DeepResearch track `{id}` omitted boolean `primary_source_required`")
            })?;
        let independent_corroboration_required = evidence_requirements
            .get("independent_corroboration_required")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                format!(
                    "DeepResearch track `{id}` omitted boolean `independent_corroboration_required`"
                )
            })?;
        obligations.push(
            ResearchObligation::new(id, title, focus, material, completion_criteria)
                .with_evidence_requirements(EvidenceQualityRequirements {
                    primary_source_required,
                    independent_corroboration_required,
                }),
        );
    }
    if !obligations.iter().any(|obligation| obligation.material) {
        return Err("DeepResearch plan must contain at least one material track".to_string());
    }

    let stop_conditions = string_array(
        object.get("stop_conditions"),
        "stop_conditions",
        limits.max_stop_conditions,
    )?;
    if stop_conditions.is_empty() {
        return Err("DeepResearch plan has no stopping condition".to_string());
    }
    Ok((obligations, stop_conditions))
}

pub fn commit_plan_research_contract(
    plan: &Value,
    state: &mut InquiryState,
    events: &mut Vec<InquiryEvent>,
    limits: &InquiryLimits,
) -> Result<(), String> {
    let (obligations, stop_conditions) = research_contract_from_plan(plan)?;
    apply_event(
        state,
        events,
        InquiryEvent::ResearchObligationsCommitted {
            obligations,
            stop_conditions,
        },
        limits,
    )
}

fn is_stable_plan_id(value: &str) -> bool {
    value.len() <= 64
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        && value.chars().skip(1).all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-')
        })
}
