pub fn materialize_deep_research_source_backed_report(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    materialize_deep_research_source_backed_report_in_language(
        workspace,
        query,
        workflow_output,
        workflow_metadata,
        &output_language,
    )
}
pub fn materialize_deep_research_source_backed_report_in_language(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
    output_language: &str,
) -> Result<Option<ResearchReportArtifacts>, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    let Some(catalog) = deep_research_source_catalog(query, workflow_output, workflow_metadata)?
    else {
        return Ok(None);
    };
    let slug = deep_research_report_slug(query);
    materialize_deep_research_source_catalog_report(
        workspace,
        query,
        &slug,
        &catalog,
        output_language,
    )
    .map(Some)
}

pub fn materialize_deep_research_source_backed_report_for_run(
    workspace: &Path,
    run_id: &str,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    materialize_deep_research_source_backed_report_for_run_in_language(
        workspace,
        run_id,
        query,
        workflow_output,
        workflow_metadata,
        &output_language,
    )
}

pub fn materialize_deep_research_source_backed_report_for_run_in_language(
    workspace: &Path,
    run_id: &str,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
    output_language: &str,
) -> Result<Option<ResearchReportArtifacts>, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    let Some(catalog) = deep_research_source_catalog(query, workflow_output, workflow_metadata)?
    else {
        return Ok(None);
    };
    let (root, report_dir, rel_html) =
        prepare_research_run_report_directory(workspace, run_id)?;
    materialize_deep_research_source_catalog_report_at(
        query,
        &catalog,
        &root,
        &report_dir,
        &rel_html,
        output_language,
    )
    .map(Some)
}

/// Preserve a completed raw-acquisition checkpoint after its Host process was
/// interrupted. The recovery identity is hashed into a separate artifact path,
/// so this cannot overwrite a completed report for the same query or turn an
/// opaque run ID into a path component.
///
/// Raw acquisition is audit-only by contract. If the supplied envelope already
/// claims closed semantic admission, callers must resume the normal publication
/// path instead of relabeling that output as an acquisition recovery.
pub fn materialize_deep_research_acquisition_recovery_report(
    workspace: &Path,
    query: &str,
    recovery_identity: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<ResearchReportArtifacts>, String> {
    if recovery_identity.is_empty() {
        return Err("acquisition recovery requires a non-empty run identity".to_string());
    }
    let Some(catalog) = deep_research_source_catalog(query, workflow_output, workflow_metadata)?
    else {
        return Ok(None);
    };
    if catalog
        .sources
        .iter()
        .any(|source| source.claim_eligible || source.semantically_admitted)
    {
        return Err(
            "acquisition recovery accepts only raw, non-admitted source checkpoints".to_string(),
        );
    }
    let slug = deep_research_acquisition_recovery_slug(query, recovery_identity);
    let output_language = crate::language::infer_deep_research_output_language(query);
    materialize_deep_research_source_catalog_report(
        workspace,
        query,
        &slug,
        &catalog,
        &output_language,
    )
    .map(Some)
}

fn deep_research_acquisition_recovery_slug(query: &str, recovery_identity: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut digest = Sha256::new();
    digest.update(recovery_identity.as_bytes());
    let suffix = format!("{:x}", digest.finalize());
    format!(
        "{}-acquisition-recovery-{}",
        deep_research_report_slug(query),
        &suffix[..16]
    )
}

fn materialize_deep_research_source_catalog_report(
    workspace: &Path,
    query: &str,
    slug: &str,
    catalog: &DeepResearchSourceCatalog,
    output_language: &str,
) -> Result<ResearchReportArtifacts, String> {
    let rel_html = format!(".a3s/research/{slug}/index.html");
    let (root, report_dir) = prepare_research_report_directory(workspace, slug)?;
    materialize_deep_research_source_catalog_report_at(
        query,
        catalog,
        &root,
        &report_dir,
        &rel_html,
        output_language,
    )
}

fn materialize_deep_research_source_catalog_report_at(
    query: &str,
    catalog: &DeepResearchSourceCatalog,
    root: &Path,
    report_dir: &Path,
    rel_html: &str,
    output_language: &str,
) -> Result<ResearchReportArtifacts, String> {
    let markdown =
        deep_research_source_backed_markdown_in_language(query, catalog, output_language);
    let html = format!(
        "<!-- {SOURCE_BACKED_ARTIFACT_MARKER} -->\n{}",
        deep_research_degraded_report_html_in_language(query, &markdown, output_language)
    );
    write_research_report_pair(
        &report_dir.join("report.md"),
        markdown,
        &report_dir.join("index.html"),
        html,
    )?;
    let artifacts = trusted_research_report_artifact_paths(rel_html, root)
        .ok_or_else(|| "source-backed report artifacts failed path validation".to_string())?;
    source_backed_report_artifacts(&artifacts)
        .then_some(artifacts)
        .ok_or_else(|| "source-backed report artifacts failed content validation".to_string())
}

pub fn materialize_deep_research_no_evidence_report(
    workspace: &Path,
    query: &str,
) -> Result<ResearchReportArtifacts, String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    materialize_deep_research_no_evidence_report_in_language(
        workspace,
        query,
        &output_language,
    )
}

pub fn materialize_deep_research_no_evidence_report_in_language(
    workspace: &Path,
    query: &str,
    output_language: &str,
) -> Result<ResearchReportArtifacts, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    let slug = deep_research_report_slug(query);
    let rel_html = format!(".a3s/research/{slug}/index.html");
    let (root, report_dir) = prepare_research_report_directory(workspace, &slug)?;
    materialize_deep_research_no_evidence_report_at(
        query,
        &root,
        &report_dir,
        &rel_html,
        output_language,
    )
}

pub fn materialize_deep_research_no_evidence_report_for_run(
    workspace: &Path,
    run_id: &str,
    query: &str,
) -> Result<ResearchReportArtifacts, String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    materialize_deep_research_no_evidence_report_for_run_in_language(
        workspace,
        run_id,
        query,
        &output_language,
    )
}

pub fn materialize_deep_research_no_evidence_report_for_run_in_language(
    workspace: &Path,
    run_id: &str,
    query: &str,
    output_language: &str,
) -> Result<ResearchReportArtifacts, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    let (root, report_dir, rel_html) =
        prepare_research_run_report_directory(workspace, run_id)?;
    materialize_deep_research_no_evidence_report_at(
        query,
        &root,
        &report_dir,
        &rel_html,
        output_language,
    )
}

fn materialize_deep_research_no_evidence_report_at(
    query: &str,
    root: &Path,
    report_dir: &Path,
    rel_html: &str,
    output_language: &str,
) -> Result<ResearchReportArtifacts, String> {
    let title = markdown_plain_text(&query.chars().take(180).collect::<String>());
    let labels = no_evidence_labels(output_language);
    let markdown = format!(
        "# {title}\n\n<!-- {NO_EVIDENCE_ARTIFACT_MARKER} -->\n\n## {status_heading}\n\n{status}\n\n## {limitations_heading}\n\n{limitations}\n\n## {sources_heading}\n\n{sources}\n",
        status_heading = labels.status_heading,
        status = labels.status,
        limitations_heading = labels.limitations_heading,
        limitations = labels.limitations,
        sources_heading = labels.sources_heading,
        sources = labels.sources,
    );
    let html = format!(
        "<!-- {NO_EVIDENCE_ARTIFACT_MARKER} -->\n{}",
        deep_research_degraded_report_html_in_language(query, &markdown, output_language)
    );
    write_research_report_pair(
        &report_dir.join("report.md"),
        markdown,
        &report_dir.join("index.html"),
        html,
    )?;
    let artifacts = trusted_research_report_artifact_paths(rel_html, root)
        .ok_or_else(|| "no-evidence report artifacts failed path validation".to_string())?;
    no_evidence_report_artifacts(&artifacts)
        .then_some(artifacts)
        .ok_or_else(|| "no-evidence report artifacts failed content validation".to_string())
}

struct NoEvidenceLabels {
    status_heading: &'static str,
    status: &'static str,
    limitations_heading: &'static str,
    limitations: &'static str,
    sources_heading: &'static str,
    sources: &'static str,
}

fn no_evidence_labels(output_language: &str) -> NoEvidenceLabels {
    if crate::language::primary_output_language(output_language) == "zh" {
        NoEvidenceLabels {
            status_heading: "证据状态",
            status: "本次检索未获得可安全发布的来源文本，因此不生成领域结论。",
            limitations_heading: "边界与局限",
            limitations: "此页面只说明证据边界。检索失败不等于相关事实不存在，不能单独作为决策依据。",
            sources_heading: "来源",
            sources: "未获得可安全发布的来源。",
        }
    } else {
        NoEvidenceLabels {
            status_heading: "Evidence Status",
            status: "This retrieval obtained no source text that can be published safely, so no domain conclusion is generated.",
            limitations_heading: "Limitations",
            limitations: "This page states only the evidence boundary. It does not treat retrieval failure as proof that relevant facts do not exist and should not be used alone for a decision.",
            sources_heading: "Sources",
            sources: "No safely publishable source was obtained.",
        }
    }
}

pub fn deep_research_evidence_first_published_report(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    deep_research_evidence_first_published_report_with_language(
        workspace,
        query,
        workflow_output,
        None,
    )
}

pub fn deep_research_evidence_first_published_report_in_language(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
    output_language: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    deep_research_evidence_first_published_report_with_language(
        workspace,
        query,
        workflow_output,
        Some(output_language),
    )
}

fn deep_research_evidence_first_published_report_with_language(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
    output_language: Option<&str>,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    let value = match serde_json::from_str::<serde_json::Value>(workflow_output.trim()) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if value.get("mode").and_then(serde_json::Value::as_str) != Some("evidence_first_report") {
        return Ok(None);
    }
    if value.get("query").and_then(serde_json::Value::as_str) != Some(query) {
        return Err("evidence-first publication belongs to a different query".to_string());
    }
    if let Some(output_language) = output_language {
        let observed_language = value
            .get("output_language")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| {
                "evidence-first publication omitted its output language".to_string()
            })?;
        if !crate::language::output_language_matches(output_language, observed_language) {
            return Err(
                "evidence-first publication belongs to a different output language".to_string(),
            );
        }
    }
    let publication = value
        .pointer("/publication/status")
        .cloned()
        .ok_or_else(|| "evidence-first publication omitted its status".to_string())
        .and_then(|status| {
            serde_json::from_value::<DeepResearchEvidenceFirstPublication>(status)
                .map_err(|_| "evidence-first publication has an unknown status".to_string())
        })?;
    let quality = deep_research_publication_quality(&value)?;
    validate_deep_research_publication_quality(publication, quality)?;
    let slug = deep_research_report_slug(query);
    let expected = format!(".a3s/research/{slug}/index.html");
    let expected_markdown = format!(".a3s/research/{slug}/report.md");
    if value
        .pointer("/publication/markdown")
        .and_then(serde_json::Value::as_str)
        != Some(expected_markdown.as_str())
    {
        return Err("evidence-first publication points to an unexpected artifact".to_string());
    }
    if value
        .pointer("/publication/html")
        .and_then(serde_json::Value::as_str)
        != Some(expected.as_str())
    {
        return Err("evidence-first publication points to an unexpected artifact".to_string());
    }
    let artifacts = trusted_research_report_artifact_paths(&expected, workspace)
        .ok_or_else(|| "evidence-first publication artifacts failed path validation".to_string())?;
    let valid = match publication {
        DeepResearchEvidenceFirstPublication::Synthesized
        | DeepResearchEvidenceFirstPublication::Qualified => {
            completed_research_report_artifacts(&artifacts)
        }
        DeepResearchEvidenceFirstPublication::SourceBacked => {
            source_backed_report_artifacts(&artifacts)
        }
        DeepResearchEvidenceFirstPublication::NoEvidence => {
            no_evidence_report_artifacts(&artifacts)
        }
    };
    if !valid {
        return Err("evidence-first publication artifacts failed content validation".to_string());
    }
    Ok(Some(DeepResearchPublishedReport {
        artifacts,
        publication,
        quality,
    }))
}

fn deep_research_publication_quality(
    value: &serde_json::Value,
) -> Result<DeepResearchPublicationQuality, String> {
    let quality = value
        .pointer("/publication/quality")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "evidence-first publication omitted its quality metrics".to_string())?;
    let metric = |name: &str| {
        quality
            .get(name)
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| format!("evidence-first publication has an invalid `{name}` metric"))
    };
    let optional_metric = |name: &str| -> Result<usize, String> {
        let Some(value) = quality.get(name) else {
            return Ok(0);
        };
        value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| format!("evidence-first publication has an invalid `{name}` metric"))
    };
    let research_scope = match quality.get("research_scope") {
        Some(scope) => serde_json::from_value::<DeepResearchReportScope>(scope.clone())
            .map_err(|_| "evidence-first publication has unsupported research scope".to_string())?,
        // Version-1 publications predate semantic scope in the quality
        // envelope. Preserve rediscovery without reclassifying their query.
        None => DeepResearchReportScope::Focused,
    };
    Ok(DeepResearchPublicationQuality {
        research_scope,
        direct_answer_count: metric("direct_answer_count")?,
        finding_count: metric("finding_count")?,
        accepted_claim_count: metric("accepted_claim_count")?,
        accepted_relation_count: optional_metric("accepted_relation_count")?,
        accepted_derivation_count: optional_metric("accepted_derivation_count")?,
        accepted_basis_edge_count: optional_metric("accepted_basis_edge_count")?,
        analytical_claim_count: optional_metric("analytical_claim_count")?,
        cross_source_synthesis_count: optional_metric("cross_source_synthesis_count")?,
        resolved_material_dimension_count: optional_metric(
            "resolved_material_dimension_count",
        )?,
        deeply_analyzed_dimension_count: optional_metric(
            "deeply_analyzed_dimension_count",
        )?,
        accepted_gap_count: optional_metric("accepted_gap_count")?,
        cited_source_count: metric("cited_source_count")?,
        // Version-1 evidence-first publications did not expose this metric.
        // Treating it as zero preserves focused reports while ensuring old
        // shallow broad reports cannot pass the new depth gate.
        substantive_character_count: quality
            .get("substantive_character_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0),
        relevant_source_count: metric("relevant_source_count")?,
        source_count: metric("source_count")?,
    })
}

fn validate_deep_research_publication_quality(
    publication: DeepResearchEvidenceFirstPublication,
    quality: DeepResearchPublicationQuality,
) -> Result<(), String> {
    let empty_claims = quality.direct_answer_count == 0
        && quality.finding_count == 0
        && quality.accepted_claim_count == 0
        && quality.cited_source_count == 0
        && quality.substantive_character_count == 0;
    let empty_claim_graph = quality.accepted_relation_count == 0
        && quality.accepted_derivation_count == 0
        && quality.accepted_basis_edge_count == 0
        && quality.analytical_claim_count == 0
        && quality.cross_source_synthesis_count == 0
        && quality.resolved_material_dimension_count == 0
        && quality.deeply_analyzed_dimension_count == 0
        && quality.accepted_gap_count == 0;
    if quality.accepted_derivation_count > quality.accepted_claim_count
        || quality.accepted_derivation_count > quality.analytical_claim_count
        || quality.analytical_claim_count > quality.accepted_claim_count
        || quality.cross_source_synthesis_count > quality.analytical_claim_count
        || (quality.resolved_material_dimension_count > 0
            && quality.deeply_analyzed_dimension_count
                > quality.resolved_material_dimension_count)
        || quality.analytical_claim_count
            < quality
                .deeply_analyzed_dimension_count
                .saturating_mul(COMPREHENSIVE_DIMENSION_MIN_ANALYTICAL_CLAIMS)
        || quality.cross_source_synthesis_count
            < quality
                .deeply_analyzed_dimension_count
                .saturating_mul(COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES)
        || quality.accepted_basis_edge_count
            < quality.cross_source_synthesis_count.saturating_mul(2)
        || (quality.accepted_claim_count == 0 && !empty_claim_graph)
    {
        return Err("publication reported inconsistent typed claim-graph metrics".to_string());
    }
    match publication {
        DeepResearchEvidenceFirstPublication::Synthesized => {
            let requirements =
                deep_research_typed_report_depth_requirements(quality.research_scope);
            if quality.direct_answer_count < requirements.minimum_direct_answers
                || quality.finding_count < requirements.minimum_findings
                || quality.accepted_claim_count < requirements.minimum_claims
                || quality.cited_source_count < requirements.minimum_cited_sources
                || quality.substantive_character_count
                    < requirements.minimum_substantive_characters
                || quality.cited_source_count > quality.relevant_source_count
                || quality.relevant_source_count == 0
                || quality.relevant_source_count > quality.source_count
                || quality.accepted_gap_count > 0
                || (quality.research_scope == DeepResearchReportScope::Comprehensive
                    && (quality.analytical_claim_count == 0
                        || quality.cross_source_synthesis_count == 0
                        || quality.resolved_material_dimension_count == 0
                        || quality.deeply_analyzed_dimension_count
                            != quality.resolved_material_dimension_count))
            {
                return Err(
                    "synthesized publication failed the closed answer-depth, analytical-synthesis, evidence-completion, independent-source, or source-relevance quality gate"
                        .to_string(),
                );
            }
        }
        DeepResearchEvidenceFirstPublication::Qualified => {
            let requirements =
                deep_research_typed_report_depth_requirements(quality.research_scope);
            if quality.direct_answer_count < requirements.minimum_direct_answers
                || quality.finding_count < requirements.minimum_findings
                || quality.accepted_claim_count < requirements.minimum_claims
                || quality.cited_source_count < requirements.minimum_cited_sources
                || quality.substantive_character_count
                    < requirements.minimum_substantive_characters
                || quality.cited_source_count > quality.relevant_source_count
                || quality.relevant_source_count == 0
                || quality.relevant_source_count > quality.source_count
                || quality.accepted_gap_count == 0
                || (quality.research_scope == DeepResearchReportScope::Comprehensive
                    && (quality.analytical_claim_count == 0
                        || quality.cross_source_synthesis_count == 0
                        || quality.resolved_material_dimension_count == 0
                        || quality.deeply_analyzed_dimension_count
                            != quality.resolved_material_dimension_count))
            {
                return Err(
                    "qualified publication failed the closed answer-depth, analytical-synthesis, evidence-gap, independent-source, or source-relevance quality gate"
                        .to_string(),
                );
            }
        }
        DeepResearchEvidenceFirstPublication::SourceBacked => {
            if !empty_claims
                || !empty_claim_graph
                || quality.source_count == 0
                || quality.relevant_source_count == 0
                || quality.relevant_source_count > quality.source_count
            {
                return Err(
                    "source-backed publication reported synthesized claims or invalid source metrics"
                        .to_string(),
                );
            }
        }
        DeepResearchEvidenceFirstPublication::NoEvidence => {
            if !empty_claims
                || !empty_claim_graph
                || quality.source_count != 0
                || quality.relevant_source_count != 0
            {
                return Err("no-evidence publication reported evidence or claims".to_string());
            }
        }
    }
    Ok(())
}
