pub fn bound_questions(
    state: &mut InquiryState,
    events: &mut Vec<InquiryEvent>,
    limits: &InquiryLimits,
    reason: &str,
) -> Result<(), String> {
    let queued = state
        .questions
        .iter()
        .filter(|question| question.status == QuestionStatus::Queued)
        .cloned()
        .collect::<Vec<_>>();
    bound_question_batch(state, events, limits, &queued, reason)
}

pub fn bound_question_batch(
    state: &mut InquiryState,
    events: &mut Vec<InquiryEvent>,
    limits: &InquiryLimits,
    questions: &[Question],
    reason: &str,
) -> Result<(), String> {
    let reason = bounded_question_event_reason(reason, limits.max_text_chars);
    for question in questions {
        apply_event(
            state,
            events,
            InquiryEvent::QuestionBounded {
                question_id: question.id.clone(),
                reason: reason.clone(),
            },
            limits,
        )?;
    }
    Ok(())
}

/// Tool/provider diagnostics may include the entire rejected schema or model
/// payload. Durable question events retain a concise single-line prefix and
/// must never fail merely because an upstream error exceeded reducer limits.
fn bounded_question_event_reason(reason: &str, maximum: usize) -> String {
    let normalized = reason.split_whitespace().collect::<Vec<_>>().join(" ");
    let detail = if normalized.is_empty() {
        "question resolution ended without a diagnostic"
    } else {
        normalized.as_str()
    };
    detail.chars().take(maximum).collect()
}

pub fn queue_plan_questions(
    plan: &Value,
    state: &mut InquiryState,
    events: &mut Vec<InquiryEvent>,
    limits: &InquiryLimits,
) -> Result<(), String> {
    let tracks = plan
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or_else(|| "DeepResearch plan has no tracks".to_string())?;
    let mut questions = Vec::new();
    for (track_index, track) in tracks.iter().enumerate() {
        let track = track
            .as_object()
            .ok_or_else(|| "DeepResearch plan contains a non-object track".to_string())?;
        let obligation_id = required_text(track, "id")?;
        let material = track
            .get("material")
            .and_then(Value::as_bool)
            .ok_or_else(|| "DeepResearch plan track omitted boolean `material`".to_string())?;
        let completion_criterion_count = track
            .get("completion_criteria")
            .and_then(Value::as_array)
            .map(Vec::len)
            .filter(|count| *count > 0)
            .ok_or_else(|| {
                format!("DeepResearch plan track `{obligation_id}` has no completion criteria")
            })?;
        let prompts = validated_plan_questions(
            track.get("questions"),
            "track questions",
            completion_criterion_count,
            false,
        )?;
        let question_count = prompts.len();
        for (question_index, planned_question) in prompts.into_iter().enumerate() {
            // Planner-authored IDs are display metadata and may contain
            // provider-dependent punctuation. Inquiry identity is owned by
            // the host so replay and downstream closed schemas remain stable.
            let id = format!("question:plan-{}-{}", track_index + 1, question_index + 1);
            let mut question = Question::queued(id, None, planned_question.prompt);
            question.obligation_ids = vec![obligation_id.to_string()];
            question.completion_criterion_indexes =
                if let Some(indexes) = planned_question.completion_criterion_indexes {
                    indexes
                } else if question_count == completion_criterion_count {
                    vec![question_index]
                } else if question_count == 1 {
                    (0..completion_criterion_count).collect()
                } else if completion_criterion_count == 1 {
                    vec![0]
                } else {
                    return Err(format!(
                        "DeepResearch plan track `{obligation_id}` cannot map {question_count} questions onto {completion_criterion_count} completion criteria"
                    ));
                };
            question.material = material;
            question.round = 0;
            questions.push(question);
        }
    }
    if questions.is_empty() {
        return Err("DeepResearch plan did not queue any research question".to_string());
    }
    apply_event(
        state,
        events,
        InquiryEvent::QuestionsQueued { questions },
        limits,
    )
}

pub fn workflow_args_with_plan(
    mut args: Value,
    plan: Value,
    run_id: Option<&str>,
) -> Result<Value, String> {
    // Flow compares this input byte-for-byte when a stable run is resumed, so
    // wall-clock origins belong to Flow history rather than durable input.
    exact_string_array(
        plan.get("search_queries"),
        "search_queries",
        MAX_PLANNER_SEARCHES as usize,
    )?;
    let plan = normalize_planner_budget(plan)?;
    let input = args
        .get_mut("input")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "DeepResearch workflow args have no input object".to_string())?;
    input.insert("research_plan".to_string(), plan);
    input.insert(
        "execution_mode".to_string(),
        Value::String("collect_only".to_string()),
    );
    input.insert("research_plan_fixture".to_string(), Value::Bool(false));
    input.remove("run_started_at_ms");
    if let Some(run_id) = run_id {
        args.as_object_mut()
            .ok_or_else(|| "DeepResearch workflow args are not an object".to_string())?
            .insert("run_id".to_string(), Value::String(run_id.to_string()));
    }
    Ok(args)
}

/// Build the minimum Host-owned contract that keeps acquisition and qualified
/// reporting available when semantic planning is slow, invalid, or absent.
/// The original query is the only provider query. The fallback deliberately
/// contains no topic inference or query expansion.
pub fn host_fallback_plan(workflow_args: &Value) -> Result<PlannedInquiry, String> {
    let query = workflow_args
        .pointer("/input/query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| "DeepResearch fallback contract requires a non-empty query".to_string())?;
    let local_only = workflow_args
        .pointer("/input/evidence_scope")
        .and_then(Value::as_str)
        == Some("local_only");
    let workspace_evidence_required = local_only || has_workspace_source_hints(workflow_args);
    let report_title = bounded_fallback_text(query, 160);
    let focus = bounded_fallback_text(query, 500);
    let criterion = bounded_fallback_text(query, 240);
    let search_queries = if local_only {
        Vec::new()
    } else {
        vec![query.to_string()]
    };
    let seed_urls = if local_only {
        Vec::new()
    } else {
        user_query_seed_urls(query)
    };
    let direct_searches = search_queries.len();
    let plan = serde_json::json!({
        "report_title": report_title,
        // Unknown semantic scope fails toward the stronger publication gate.
        // This is a safety default, not a topic classification.
        "research_scope": "comprehensive",
        // Unknown temporal requirements likewise fail toward the stronger
        // evidence contract instead of authorizing an undated final answer.
        "freshness_required": true,
        "workspace_evidence_required": workspace_evidence_required,
        "request_requirements": [{
            "id": "request.primary",
            "text": criterion.clone(),
        }],
        "tracks": [{
            "id": "request.primary",
            "title": bounded_fallback_text(query, 160),
            "focus": focus,
            "material": true,
            "requirement_ids": ["request.primary"],
            "questions": [criterion.clone()],
            "completion_criteria": [criterion],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false
            }
        }],
        "search_queries": search_queries,
        "seed_urls": seed_urls,
        "budget": {
            "retrieval_timeout_ms": 150_000,
            "direct_searches": direct_searches,
            "direct_fetches": if local_only { 0 } else { MAX_PLANNER_INITIAL_FETCHES }
        },
        "stop_conditions": [
            "Material evidence is retained or the request is explicitly bounded."
        ]
    });
    validate_plan(plan)
}

/// Complete one small model-authored outline into the legacy retrieval and
/// Inquiry contract without another semantic generation. The Host owns
/// question identity, transport budgets, the exact-query prefix, and universal
/// stop conditions. It does not rewrite semantic tracks or search queries.
pub fn host_plan_from_outline(
    workflow_args: &Value,
    outline: Value,
) -> Result<PlannedInquiry, String> {
    let query = workflow_args
        .pointer("/input/query")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|query| !query.is_empty())
        .ok_or_else(|| "DeepResearch outline contract requires a non-empty query".to_string())?;
    let local_only = workflow_args
        .pointer("/input/evidence_scope")
        .and_then(Value::as_str)
        == Some("local_only");
    let workspace_evidence_required = local_only || has_workspace_source_hints(workflow_args);
    let mut outline = close_semantic_outline(outline)?;
    let output_language = workflow_args
        .pointer("/input/output_language")
        .and_then(Value::as_str)
        .map(str::to_string)
        // `DeepResearchRequest` always supplies this field. Legacy adapters
        // retain language fidelity by deriving it from the exact user query
        // at the same Host boundary.
        .unwrap_or_else(|| crate::language::infer_deep_research_output_language(query));
    if !semantic_outline_matches_output_language(&outline, &output_language) {
        return Err(
            "DeepResearch semantic planner returned reader-facing fields in a different language"
                .to_string(),
        );
    }
    let tracks = semantic_outline_track_targets(&outline)?;
    let object = outline
        .as_object_mut()
        .ok_or_else(|| "DeepResearch outline planner returned a non-object fragment".to_string())?;
    object.insert("tracks".to_string(), Value::Array(tracks));
    if workspace_evidence_required {
        object.insert("workspace_evidence_required".to_string(), Value::Bool(true));
    }
    let supplemental_queries = exact_string_array(
        object.get("supplemental_queries"),
        "supplemental_queries",
        MAX_PLANNER_SUPPLEMENTAL_QUERIES,
    )?;
    object.remove("supplemental_queries");
    let search_queries = if local_only {
        Vec::new()
    } else {
        validated_semantic_search_queries(query, supplemental_queries)?
    };
    let direct_searches = search_queries.len();
    object.insert(
        "search_queries".to_string(),
        serde_json::to_value(search_queries)
            .map_err(|error| format!("encode Host search queries: {error}"))?,
    );
    object.insert(
        "seed_urls".to_string(),
        serde_json::to_value(if local_only {
            Vec::new()
        } else {
            user_query_seed_urls(query)
        })
        .map_err(|error| format!("encode Host seed URLs: {error}"))?,
    );
    object.insert(
        "budget".to_string(),
        serde_json::json!({
            "retrieval_timeout_ms": 150_000,
            "direct_searches": direct_searches,
            "direct_fetches": if local_only { 0 } else { MAX_PLANNER_INITIAL_FETCHES }
        }),
    );
    validate_plan(outline)
}

fn has_workspace_source_hints(workflow_args: &Value) -> bool {
    workflow_args
        .pointer("/input/workspace_source_hints")
        .and_then(Value::as_array)
        .is_some_and(|hints| !hints.is_empty())
}

fn semantic_outline_matches_output_language(outline: &Value, output_language: &str) -> bool {
    let mut reader_text = outline
        .get("report_title")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    for requirement in outline
        .get("request_requirements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(value) = requirement.get("text").and_then(Value::as_str) {
            reader_text.push('\n');
            reader_text.push_str(value);
        }
    }
    for track in outline
        .get("tracks")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for field in ["title", "focus"] {
            if let Some(value) = track.get(field).and_then(Value::as_str) {
                reader_text.push('\n');
                reader_text.push_str(value);
            }
        }
        for criterion in track
            .get("completion_criteria")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            reader_text.push('\n');
            reader_text.push_str(criterion);
        }
    }
    crate::language::reader_text_matches_output_language(&reader_text, output_language)
}

pub fn bootstrap_workflow_args(args: Value, run_id: &str) -> Result<Value, String> {
    let plan = host_fallback_plan(&args)?;
    let mut args = workflow_args_with_plan(args, plan.value, Some(run_id))?;
    args.pointer_mut("/input/execution_mode")
        .ok_or_else(|| "DeepResearch bootstrap args omitted execution mode".to_string())?
        .clone_from(&Value::String("bootstrap_acquisition".to_string()));
    Ok(args)
}

pub fn attach_bootstrap_acquisition(
    workflow_args: &mut Value,
    acquisition: Value,
) -> Result<(), String> {
    let sources = acquisition
        .pointer("/packet/sources")
        .and_then(Value::as_array)
        .filter(|sources| !sources.is_empty())
        .ok_or_else(|| {
            "DeepResearch bootstrap acquisition contains no reusable raw source packet".to_string()
        })?;
    if sources.len() > 16 {
        return Err(
            "DeepResearch bootstrap acquisition exceeds the source catalog limit".to_string(),
        );
    }
    workflow_args
        .get_mut("input")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "DeepResearch workflow args have no input object".to_string())?
        .insert("bootstrap_acquisition".to_string(), acquisition);
    Ok(())
}

fn bounded_fallback_text(value: &str, maximum_chars: usize) -> String {
    value.chars().take(maximum_chars).collect()
}

fn validated_semantic_search_queries(
    exact_query: &str,
    supplemental_queries: Vec<String>,
) -> Result<Vec<String>, String> {
    let mut queries = vec![exact_query.to_string()];
    for query in supplemental_queries {
        let query = portable_search_query(&query);
        if query.is_empty() {
            return Err(
                "DeepResearch semantic planner returned an empty portable search query"
                    .to_string(),
            );
        }
        if query == exact_query {
            return Err(
                "DeepResearch semantic planner repeated the exact query as a supplement"
                    .to_string(),
            );
        }
        if query_is_standalone_url(&query) {
            return Err(
                "DeepResearch semantic planner returned a URL instead of a search query"
                    .to_string(),
            );
        }
        if queries.contains(&query) {
            return Err("DeepResearch semantic planner returned a duplicate query".to_string());
        }
        queries.push(query);
    }
    Ok(queries)
}

fn portable_search_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(|token| {
            token
                .split_once(':')
                .filter(|(operator, value)| {
                    operator.eq_ignore_ascii_case("site") && !value.is_empty()
                })
                .map_or(token, |(_, value)| value)
        })
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn query_is_standalone_url(query: &str) -> bool {
    reqwest::Url::parse(query).is_ok_and(|url| {
        matches!(url.scheme(), "http" | "https") && url.host_str().is_some()
    })
}

fn user_query_seed_urls(query: &str) -> Vec<String> {
    const MAX_USER_SEED_URLS: usize = 3;
    const MAX_USER_SEED_URL_CHARS: usize = 2_048;

    let lower = query.to_ascii_lowercase();
    let mut cursor = 0usize;
    let mut urls = Vec::new();
    while cursor < query.len() && urls.len() < MAX_USER_SEED_URLS {
        let Some(start) = ["https://", "http://"]
            .into_iter()
            .filter_map(|prefix| lower[cursor..].find(prefix).map(|offset| cursor + offset))
            .min()
        else {
            break;
        };
        let end = query[start..]
            .char_indices()
            .find_map(|(offset, character)| {
                (character.is_whitespace()
                    || matches!(
                        character,
                        '<' | '>' | '"' | '\'' | '`' | '。' | '，' | '；' | '！' | '？'
                    ))
                .then_some(start + offset)
            })
            .unwrap_or(query.len());
        let candidate = query[start..end].trim_end_matches([
            '.', ',', ';', ':', '!', '?', ')', ']', '}', '。', '，', '；', '！', '？',
        ]);
        cursor = end.max(start + "http://".len());
        if candidate.chars().count() > MAX_USER_SEED_URL_CHARS {
            continue;
        }
        let Ok(mut url) = reqwest::Url::parse(candidate) else {
            continue;
        };
        if !matches!(url.scheme(), "http" | "https")
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
        {
            continue;
        }
        url.set_fragment(None);
        let normalized = url.to_string();
        if !urls.contains(&normalized) {
            urls.push(normalized);
        }
    }
    urls
}

fn normalize_planner_budget(mut plan: Value) -> Result<Value, String> {
    // The provider-facing planner schema uses seconds, while the workflow
    // runtime contract uses milliseconds. Injected host plans bypass the
    // JavaScript planner-result normalizer, so close that boundary here.
    let Some(budget) = plan.get_mut("budget").and_then(Value::as_object_mut) else {
        return Ok(plan);
    };
    let Some(seconds) = budget.remove("retrieval_timeout_secs") else {
        return Ok(plan);
    };
    let seconds = seconds
        .as_u64()
        .filter(|seconds| *seconds > 0)
        .ok_or_else(|| {
            "DeepResearch plan budget `retrieval_timeout_secs` must be a positive integer"
                .to_string()
        })?;
    let milliseconds = seconds.checked_mul(1_000).ok_or_else(|| {
        "DeepResearch plan budget `retrieval_timeout_secs` exceeds millisecond range".to_string()
    })?;
    budget.insert(
        "retrieval_timeout_ms".to_string(),
        Value::from(milliseconds),
    );
    Ok(plan)
}

fn string_array(
    value: Option<&Value>,
    resource: &str,
    maximum: usize,
) -> Result<Vec<String>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("DeepResearch plan {resource} is not an array"))?;
    if values.len() > maximum {
        return Err(format!(
            "DeepResearch plan {resource} has {} items; maximum is {maximum}",
            values.len()
        ));
    }
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| format!("DeepResearch plan {resource} contains a blank item"))
        })
        .collect()
}

pub fn exact_string_array(
    value: Option<&Value>,
    resource: &str,
    maximum: usize,
) -> Result<Vec<String>, String> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("DeepResearch plan {resource} is not an array"))?;
    if values.len() > maximum {
        return Err(format!(
            "DeepResearch plan {resource} has {} items; maximum is {maximum}",
            values.len()
        ));
    }
    values
        .iter()
        .map(|value| {
            let value = value.as_str().ok_or_else(|| {
                format!("DeepResearch plan {resource} contains a non-string item")
            })?;
            if value.is_empty() || value.trim().is_empty() {
                return Err(format!(
                    "DeepResearch plan {resource} contains a blank item"
                ));
            }
            if value.trim() != value {
                return Err(format!(
                    "DeepResearch plan {resource} contains an item with surrounding whitespace"
                ));
            }
            Ok(value.to_string())
        })
        .collect()
}

fn required_text<'a>(object: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("DeepResearch plan omitted non-empty `{key}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_query_normalization_is_transport_only_and_subject_invariant() {
        for (query, expected) in [
            (
                "site:standards.example protocol conformance record",
                "standards.example protocol conformance record",
            ),
            (
                "SITE:public-health.example clinical guidance revision",
                "public-health.example clinical guidance revision",
            ),
            (
                "grid operator capacity outlook",
                "grid operator capacity outlook",
            ),
        ] {
            assert_eq!(portable_search_query(query), expected);
        }
    }

    #[test]
    fn normalized_search_queries_remain_unique_and_keep_exact_query_authority() {
        let queries = validated_semantic_search_queries(
            "Compare the current records",
            vec!["site:records.example current policy".to_string()],
        )
        .expect("portable search query");
        assert_eq!(queries[0], "Compare the current records");
        assert_eq!(queries[1], "records.example current policy");

        assert!(validated_semantic_search_queries(
            "Compare the current records",
            vec![
                "site:records.example current policy".to_string(),
                "records.example current policy".to_string(),
            ],
        )
        .is_err());
    }
}
