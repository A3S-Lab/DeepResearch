struct ReportEditorialOutcome {
    report: Option<AdmittedDeepResearchReport>,
    error: Option<String>,
    generation_attempted: bool,
}

#[allow(clippy::too_many_arguments)]
async fn editorially_review_report(
    engine: &DeepResearchEngine<'_>,
    cancellation: &DeepResearchCancellation,
    limits: &super::EngineLimits,
    query: &str,
    current_date: &str,
    output_language: &str,
    catalog: &DeepResearchSourceCatalog,
    report_context: &DeepResearchReportContext,
    draft: AdmittedTypedReportDraft,
) -> Result<ReportEditorialOutcome, DeepResearchEngineError> {
    let fallback_report = draft.report.clone();
    let editorial_prompt = match deep_research_typed_editorial_prompt(&draft) {
        Ok(prompt) => prompt,
        Err(error) => {
            return Ok(ReportEditorialOutcome {
                report: incomplete_editorial_fallback(fallback_report),
                error: Some(error),
                generation_attempted: false,
            });
        }
    };
    let editorial_schema = deep_research_typed_editorial_schema(&draft);
    let editorial_payload_bytes = editorial_prompt
        .len()
        .saturating_add(editorial_schema.to_string().len());
    let editorial_attempt_timeout_ms =
        limits.report_attempt_timeout_for_payload(editorial_payload_bytes);
    let editorial_stage_timeout_ms =
        limits.report_stage_timeout_for_attempt(editorial_attempt_timeout_ms);
    let editorial_args = serde_json::json!({
        "schema": editorial_schema,
        "schema_name": "deep_research_typed_editorial_plan",
        "schema_description": "Independent requirement, evidence, temporal, depth, and prose review followed by evidence-preserving claim rewrites and narrative planning over already admitted claims",
        "prompt": editorial_prompt,
        "system": "You are the independent commercial-quality reviewer and final editor of an admitted research argument. Audit every mapped requirement and claim against the closed evidence, classify temporal status, and fail readiness on any omission, unsupported proposition, shallow analysis, misleading modality, or source-summary prose. Then rewrite for natural long-form reading while preserving the admitted graph and evidence boundary. Return only the requested object.",
        "mode": "auto",
        "max_repair_attempts": 0,
        "include_raw_text": false,
        "timeout_ms": editorial_attempt_timeout_ms,
    });
    let editorial = await_or_cancel(
        cancellation,
        engine.generation.generate_object(GenerationRequest {
            stage: GenerationStage::Editorial,
            arguments: editorial_args,
            execution_timeout_ms: editorial_stage_timeout_ms,
            max_attempts: limits.report_max_attempts,
        }),
    )
    .await?;
    let report = match editorial {
        Ok(editorial) => apply_deep_research_typed_commercial_editorial_plan(
            query,
            current_date,
            output_language,
            catalog,
            report_context,
            draft,
            editorial,
        ),
        Err(error) => Err(error),
    };
    Ok(match report {
        Ok(report) => ReportEditorialOutcome {
            report: Some(report),
            error: None,
            generation_attempted: true,
        },
        Err(error) => ReportEditorialOutcome {
            report: incomplete_editorial_fallback(fallback_report),
            error: Some(error),
            generation_attempted: true,
        },
    })
}
