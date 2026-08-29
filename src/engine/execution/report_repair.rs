#[allow(clippy::too_many_arguments)]
async fn repair_rejected_report_draft(
    engine: &DeepResearchEngine<'_>,
    cancellation: &DeepResearchCancellation,
    limits: &super::EngineLimits,
    query: &str,
    current_date: &str,
    output_language: &str,
    catalog: &DeepResearchSourceCatalog,
    source_attribution: &DeepResearchSourceAttribution,
    report_context: &DeepResearchReportContext,
    report_schema: &Value,
    report_prompt: &str,
    rejected_proposal: &Value,
    diagnostics: &TypedReportGateDiagnostics,
) -> Result<Result<AdmittedTypedReportDraft, String>, DeepResearchEngineError> {
    let repair_prompt = match deep_research_typed_report_repair_prompt(
        report_prompt,
        rejected_proposal,
        diagnostics,
    ) {
        Ok(prompt) => prompt,
        Err(error) => return Ok(Err(error)),
    };
    let repair_payload_bytes = repair_prompt
        .len()
        .saturating_add(report_schema.to_string().len());
    let repair_attempt_timeout_ms =
        limits.report_attempt_timeout_for_payload(repair_payload_bytes);
    let repair_stage_timeout_ms =
        limits.report_stage_timeout_for_attempt(repair_attempt_timeout_ms);
    let repair_args = serde_json::json!({
        "schema": report_schema.clone(),
        "schema_name": "deep_research_typed_claim_graph_repair",
        "schema_description": "A complete replacement claim graph that repairs deterministic answer, evidence, comparison, explanation, implication, boundary, and depth failures while retaining only fact-verifiable semantic charts",
        "prompt": repair_prompt,
        "system": "You repair a rejected source-grounded research argument against trusted deterministic gate diagnostics. Return one complete replacement object. Resolve every listed content issue without padding, weakening evidence requirements, inventing facts, turning an unresolved dimension into a conclusion, or manufacturing chart data. Omit a visualization unless every coordinate remains directly traceable to comparable facts in the closed evidence.",
        "mode": "auto",
        "max_repair_attempts": 0,
        "include_raw_text": false,
        "timeout_ms": repair_attempt_timeout_ms,
    });
    let repaired = await_or_cancel(
        cancellation,
        engine.generation.generate_object(GenerationRequest {
            stage: GenerationStage::Report,
            arguments: repair_args,
            execution_timeout_ms: repair_stage_timeout_ms,
            max_attempts: limits.report_max_attempts,
        }),
    )
    .await?;
    let repaired_proposal = match repaired {
        Ok(proposal) => proposal,
        Err(error) => return Ok(Err(error)),
    };
    Ok(match evaluate_attributed_report_draft(
        query,
        current_date,
        output_language,
        catalog,
        source_attribution,
        report_context,
        repaired_proposal,
    ) {
        Ok(TypedReportDraftAdmission::Admitted(draft)) => Ok(*draft),
        Ok(TypedReportDraftAdmission::Rejected(diagnostics)) => Err(format!(
            "the repaired report proposal still failed content gates: {}",
            diagnostics.summary()
        )),
        Err(error) => Err(error),
    })
}
