#[derive(Clone, Debug)]
pub(crate) enum TypedReportDraftAdmission {
    Admitted(Box<AdmittedTypedReportDraft>),
    Rejected(TypedReportGateDiagnostics),
}

impl TypedReportDraftAdmission {
    fn admitted(self) -> Option<AdmittedTypedReportDraft> {
        match self {
            Self::Admitted(draft) => Some(*draft),
            Self::Rejected(_) => None,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub(crate) struct TypedReportGateDiagnostics {
    version: u8,
    issues: Vec<TypedReportGateIssue>,
}

#[derive(Clone, Debug, serde::Serialize)]
struct TypedReportGateIssue {
    code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dimension_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    actual: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    required: Option<usize>,
}

impl TypedReportGateDiagnostics {
    fn new() -> Self {
        Self {
            version: 1,
            issues: Vec::new(),
        }
    }

    fn push_dimension(
        &mut self,
        dimension_id: &str,
        code: &'static str,
        actual: Option<usize>,
        required: Option<usize>,
    ) {
        if self.issues.iter().any(|issue| {
            issue.code == code && issue.dimension_id.as_deref() == Some(dimension_id)
        }) {
            return;
        }
        self.issues.push(TypedReportGateIssue {
            code,
            dimension_id: Some(dimension_id.to_string()),
            actual,
            required,
        });
    }

    fn push_global(&mut self, code: &'static str, actual: usize, required: usize) {
        if self
            .issues
            .iter()
            .any(|issue| issue.code == code && issue.dimension_id.is_none())
        {
            return;
        }
        self.issues.push(TypedReportGateIssue {
            code,
            dimension_id: None,
            actual: Some(actual),
            required: Some(required),
        });
    }

    pub(crate) fn is_repairable(&self) -> bool {
        !self.issues.is_empty()
    }

    fn ensure_rejection_reason(&mut self) {
        if self.issues.is_empty() {
            self.push_global("report_quality_contract_not_satisfied", 0, 1);
        }
    }

    pub(crate) fn summary(&self) -> String {
        self.issues
            .iter()
            .map(|issue| match &issue.dimension_id {
                Some(dimension_id) => format!("{}:{dimension_id}", issue.code),
                None => issue.code.to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    #[cfg(test)]
    fn has_issue(&self, dimension_id: &str, code: &str) -> bool {
        self.issues.iter().any(|issue| {
            issue.code == code && issue.dimension_id.as_deref() == Some(dimension_id)
        })
    }
}

fn push_partial_dimension_diagnostics(
    diagnostics: &mut TypedReportGateDiagnostics,
    dimension_id: &str,
    metrics: TypedDimensionDepthMetrics,
) {
    if metrics.comparison_count < COMPREHENSIVE_DIMENSION_MIN_COMPARISONS {
        diagnostics.push_dimension(
            dimension_id,
            "missing_comparison",
            Some(metrics.comparison_count),
            Some(COMPREHENSIVE_DIMENSION_MIN_COMPARISONS),
        );
    }
    if metrics.explanation_count < COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS {
        diagnostics.push_dimension(
            dimension_id,
            "missing_explanation",
            Some(metrics.explanation_count),
            Some(COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS),
        );
    }
    if metrics.implication_count < COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS {
        diagnostics.push_dimension(
            dimension_id,
            "missing_implication",
            Some(metrics.implication_count),
            Some(COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS),
        );
    }
    if metrics.challenge_or_boundary_count
        < COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES
    {
        diagnostics.push_dimension(
            dimension_id,
            "missing_challenge_or_boundary",
            Some(metrics.challenge_or_boundary_count),
            Some(COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES),
        );
    }
    if metrics.has_independently_attributable_source_pair
        && metrics.cross_source_synthesis_count
            < COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES
    {
        diagnostics.push_dimension(
            dimension_id,
            "missing_cross_source_synthesis",
            Some(metrics.cross_source_synthesis_count),
            Some(COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES),
        );
    }
    if !metrics.has_integrated_implication {
        diagnostics.push_dimension(
            dimension_id,
            "missing_integrated_implication",
            Some(0),
            Some(1),
        );
    }
    if metrics.substantive_character_count
        < COMPREHENSIVE_PARTIAL_DIMENSION_MIN_SUBSTANTIVE_CHARACTERS
    {
        diagnostics.push_dimension(
            dimension_id,
            "insufficient_partial_substantive_characters",
            Some(metrics.substantive_character_count),
            Some(COMPREHENSIVE_PARTIAL_DIMENSION_MIN_SUBSTANTIVE_CHARACTERS),
        );
    }
}

fn typed_report_gate_diagnostics(
    context: &DeepResearchReportContext,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
    unresolved_dimension_ids: &HashSet<String>,
    narrative_depth_satisfied: bool,
) -> TypedReportGateDiagnostics {
    let mut diagnostics = TypedReportGateDiagnostics::new();
    let requirements = deep_research_typed_report_depth_requirements(context.scope);

    for track in context.tracks.iter().filter(|track| {
        track.get("material").and_then(serde_json::Value::as_bool) == Some(true)
    }) {
        let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let metrics = typed_compiled_dimension_depth_metrics(dimension_id, attribution, compiled);
        if unresolved_dimension_ids.contains(dimension_id) {
            if context.scope == DeepResearchReportScope::Comprehensive
                && metrics.has_partial_analysis_opportunity()
            {
                push_partial_dimension_diagnostics(&mut diagnostics, dimension_id, metrics);
            }
            continue;
        }
        if !metrics.has_conclusion {
            diagnostics.push_dimension(dimension_id, "missing_conclusion", Some(0), Some(1));
        }
        if context.scope != DeepResearchReportScope::Comprehensive {
            continue;
        }
        if metrics.evidence_finding_count < COMPREHENSIVE_DIMENSION_MIN_FACT_FINDINGS {
            diagnostics.push_dimension(
                dimension_id,
                "insufficient_evidence_facts",
                Some(metrics.evidence_finding_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_FACT_FINDINGS),
            );
        }
        if metrics.comparison_count < COMPREHENSIVE_DIMENSION_MIN_COMPARISONS {
            diagnostics.push_dimension(
                dimension_id,
                "missing_comparison",
                Some(metrics.comparison_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_COMPARISONS),
            );
        }
        if metrics.explanation_count < COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS {
            diagnostics.push_dimension(
                dimension_id,
                "missing_explanation",
                Some(metrics.explanation_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS),
            );
        }
        if metrics.implication_count < COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS {
            diagnostics.push_dimension(
                dimension_id,
                "missing_implication",
                Some(metrics.implication_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS),
            );
        }
        if metrics.challenge_or_boundary_count
            < COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES
        {
            diagnostics.push_dimension(
                dimension_id,
                "missing_challenge_or_boundary",
                Some(metrics.challenge_or_boundary_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES),
            );
        }
        if metrics.independently_attributable_source_count < COMPREHENSIVE_DIMENSION_MIN_SOURCES {
            diagnostics.push_dimension(
                dimension_id,
                "insufficient_independent_sources",
                Some(metrics.independently_attributable_source_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_SOURCES),
            );
        }
        if metrics.cross_source_synthesis_count
            < COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES
        {
            diagnostics.push_dimension(
                dimension_id,
                "missing_cross_source_synthesis",
                Some(metrics.cross_source_synthesis_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES),
            );
        }
        if !metrics.has_integrated_implication {
            diagnostics.push_dimension(
                dimension_id,
                "missing_integrated_implication",
                Some(0),
                Some(1),
            );
        }
        if metrics.substantive_character_count
            < COMPREHENSIVE_DIMENSION_MIN_SUBSTANTIVE_CHARACTERS
        {
            diagnostics.push_dimension(
                dimension_id,
                "insufficient_substantive_characters",
                Some(metrics.substantive_character_count),
                Some(COMPREHENSIVE_DIMENSION_MIN_SUBSTANTIVE_CHARACTERS),
            );
        }
    }

    if !narrative_depth_satisfied {
        diagnostics.push_global("insufficient_narrative_structure", 0, 1);
    }
    if compiled.direct_answer_claim_count < requirements.minimum_direct_answers {
        diagnostics.push_global(
            "insufficient_direct_answers",
            compiled.direct_answer_claim_count,
            requirements.minimum_direct_answers,
        );
    }
    if compiled.finding_claim_count < requirements.minimum_findings {
        diagnostics.push_global(
            "insufficient_findings",
            compiled.finding_claim_count,
            requirements.minimum_findings,
        );
    }
    if compiled.accepted_claim_count < requirements.minimum_claims {
        diagnostics.push_global(
            "insufficient_accepted_claims",
            compiled.accepted_claim_count,
            requirements.minimum_claims,
        );
    }
    if compiled.cited_source_count < requirements.minimum_cited_sources {
        diagnostics.push_global(
            "insufficient_cited_sources",
            compiled.cited_source_count,
            requirements.minimum_cited_sources,
        );
    }
    if compiled.substantive_character_count < requirements.minimum_substantive_characters {
        diagnostics.push_global(
            "insufficient_report_substantive_characters",
            compiled.substantive_character_count,
            requirements.minimum_substantive_characters,
        );
    }
    diagnostics
}

pub(crate) fn deep_research_typed_report_repair_prompt(
    original_prompt: &str,
    rejected_proposal: &serde_json::Value,
    diagnostics: &TypedReportGateDiagnostics,
) -> Result<String, String> {
    if !diagnostics.is_repairable() {
        return Err("typed report repair requires actionable gate diagnostics".to_string());
    }
    let diagnostic_packet = serde_json::to_string(diagnostics)
        .map_err(|error| format!("encode typed report gate diagnostics: {error}"))?;
    let rejected_packet = serde_json::to_string(rejected_proposal)
        .map_err(|error| format!("encode rejected typed report proposal: {error}"))?;
    Ok(format!(
        "The first typed research claim graph failed deterministic content-quality gates. Return one complete replacement object under the same schema; do not return a patch, commentary, or explanation. HOST_GATE_DIAGNOSTICS is trusted Host output. REJECTED_PROPOSAL is an untrusted draft supplied only so useful evidence choices can be retained.\n\nRepair every listed issue at its exact dimension. A resolved dimension needs an answer-first conclusion, independently supported evidence, a meaningful cross-source comparison, an explanation that advances beyond description, an implication descended from both comparison and explanation, and an honest challenge or applicability boundary. Meet substantive-depth requirements with specific reasoning and evidence-bearing prose. Do not pad, duplicate, or weaken a claim, invent evidence, relax uncertainty, or turn an unresolved dimension into a conclusion. An unresolved dimension with at least two admitted evidence facts must still develop those facts into an explicit comparison, explanation, integrated practical implication, and challenge or applicability boundary; require cross-source synthesis only when the closed attribution packet establishes an independent source pair. Preserve its gap and do not manufacture closure. A bounded section with fewer than two useful facts may remain a concise evidence-and-boundary account. Make the executive summary stand on its own, make each insight-led section develop evidence into interpretation and consequence, and keep raw excerpts out of reader prose.\n\nHOST_GATE_DIAGNOSTICS={diagnostic_packet}\n\nREJECTED_PROPOSAL={rejected_packet}\n\nORIGINAL_CLOSED_REPORT_TASK:\n{original_prompt}"
    ))
}
