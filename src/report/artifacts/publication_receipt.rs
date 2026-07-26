const PUBLICATION_RECEIPT_SCHEMA_VERSION: u32 = 5;
const MINIMUM_PUBLICATION_RECEIPT_SCHEMA_VERSION: u32 = 1;
const PUBLICATION_RECEIPT_FILE_NAME: &str = "publication-receipt.json";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct DeepResearchPublicationReceipt {
    schema_version: u32,
    run_identity_sha256: String,
    query_sha256: String,
    #[serde(default)]
    output_language: Option<String>,
    publication: DeepResearchEvidenceFirstPublication,
    quality: DeepResearchPublicationQualityReceipt,
    markdown_sha256: String,
    html_sha256: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct DeepResearchPublicationQualityReceipt {
    research_scope: DeepResearchReportScope,
    direct_answer_count: usize,
    finding_count: usize,
    accepted_claim_count: usize,
    #[serde(default)]
    accepted_relation_count: usize,
    #[serde(default)]
    accepted_derivation_count: usize,
    #[serde(default)]
    accepted_basis_edge_count: usize,
    #[serde(default)]
    analytical_claim_count: usize,
    #[serde(default)]
    cross_source_synthesis_count: usize,
    #[serde(default)]
    resolved_material_dimension_count: usize,
    #[serde(default)]
    deeply_analyzed_dimension_count: usize,
    #[serde(default)]
    accepted_gap_count: usize,
    cited_source_count: usize,
    substantive_character_count: usize,
    relevant_source_count: usize,
    source_count: usize,
}

impl From<DeepResearchPublicationQuality> for DeepResearchPublicationQualityReceipt {
    fn from(quality: DeepResearchPublicationQuality) -> Self {
        Self {
            research_scope: quality.research_scope,
            direct_answer_count: quality.direct_answer_count,
            finding_count: quality.finding_count,
            accepted_claim_count: quality.accepted_claim_count,
            accepted_relation_count: quality.accepted_relation_count,
            accepted_derivation_count: quality.accepted_derivation_count,
            accepted_basis_edge_count: quality.accepted_basis_edge_count,
            analytical_claim_count: quality.analytical_claim_count,
            cross_source_synthesis_count: quality.cross_source_synthesis_count,
            resolved_material_dimension_count: quality.resolved_material_dimension_count,
            deeply_analyzed_dimension_count: quality.deeply_analyzed_dimension_count,
            accepted_gap_count: quality.accepted_gap_count,
            cited_source_count: quality.cited_source_count,
            substantive_character_count: quality.substantive_character_count,
            relevant_source_count: quality.relevant_source_count,
            source_count: quality.source_count,
        }
    }
}

impl From<DeepResearchPublicationQualityReceipt> for DeepResearchPublicationQuality {
    fn from(quality: DeepResearchPublicationQualityReceipt) -> Self {
        Self {
            research_scope: quality.research_scope,
            direct_answer_count: quality.direct_answer_count,
            finding_count: quality.finding_count,
            accepted_claim_count: quality.accepted_claim_count,
            accepted_relation_count: quality.accepted_relation_count,
            accepted_derivation_count: quality.accepted_derivation_count,
            accepted_basis_edge_count: quality.accepted_basis_edge_count,
            analytical_claim_count: quality.analytical_claim_count,
            cross_source_synthesis_count: quality.cross_source_synthesis_count,
            resolved_material_dimension_count: quality.resolved_material_dimension_count,
            deeply_analyzed_dimension_count: quality.deeply_analyzed_dimension_count,
            accepted_gap_count: quality.accepted_gap_count,
            cited_source_count: quality.cited_source_count,
            substantive_character_count: quality.substantive_character_count,
            relevant_source_count: quality.relevant_source_count,
            source_count: quality.source_count,
        }
    }
}

/// Persist the exact run authority for an already validated report generation.
///
/// The receipt stores only closed publication metadata, the request-owned
/// language, and full artifact digests. Reader prose, source vocabulary,
/// domains, and titles never participate in restart recovery.
pub fn record_deep_research_publication_receipt(
    workspace: &Path,
    query: &str,
    run_identity: &str,
    publication: DeepResearchEvidenceFirstPublication,
    quality: DeepResearchPublicationQuality,
    artifacts: &ResearchReportArtifacts,
) -> Result<(), String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    record_deep_research_publication_receipt_in_language(
        workspace,
        query,
        &output_language,
        run_identity,
        publication,
        quality,
        artifacts,
    )
}

/// Persist a validated publication and bind it to the request-owned language.
pub fn record_deep_research_publication_receipt_in_language(
    workspace: &Path,
    query: &str,
    output_language: &str,
    run_identity: &str,
    publication: DeepResearchEvidenceFirstPublication,
    quality: DeepResearchPublicationQuality,
    artifacts: &ResearchReportArtifacts,
) -> Result<(), String> {
    if run_identity.is_empty() {
        return Err("publication receipt requires a non-empty run identity".to_string());
    }
    crate::language::validate_deep_research_output_language(output_language)?;
    validate_deep_research_publication_quality(publication, quality)?;
    let run_scoped = exact_run_publication_artifacts(workspace, run_identity, publication)?;
    let legacy = exact_publication_artifacts(workspace, query, publication)?;
    let trusted = run_scoped
        .into_iter()
        .chain(legacy)
        .find(|candidate| candidate == artifacts)
        .ok_or_else(|| {
            "publication receipt artifacts do not match the run or legacy query path".to_string()
        })?;
    let markdown = std::fs::read(&trusted.markdown)
        .map_err(|error| format!("read publication Markdown for receipt: {error}"))?;
    let html = std::fs::read(&trusted.html)
        .map_err(|error| format!("read publication HTML for receipt: {error}"))?;
    let receipt = DeepResearchPublicationReceipt {
        schema_version: PUBLICATION_RECEIPT_SCHEMA_VERSION,
        run_identity_sha256: sha256_text(run_identity),
        query_sha256: sha256_text(query),
        output_language: Some(output_language.to_string()),
        publication,
        quality: quality.into(),
        markdown_sha256: sha256_bytes(&markdown),
        html_sha256: sha256_bytes(&html),
    };
    let encoded = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("encode publication receipt: {error}"))?;
    let receipt_path = trusted
        .html
        .parent()
        .ok_or_else(|| "publication receipt artifact has no directory".to_string())?
        .join(PUBLICATION_RECEIPT_FILE_NAME);
    write_research_report_file(&receipt_path, encoded)
}

/// Recover a completed publication only when its exact run-scoped receipt and
/// both full artifact digests match the current report pair.
pub fn recover_deep_research_publication_receipt(
    workspace: &Path,
    query: &str,
    run_identity: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    let output_language = crate::language::infer_deep_research_output_language(query);
    recover_deep_research_publication_receipt_with_language(
        workspace,
        query,
        &output_language,
        run_identity,
        true,
    )
}

/// Recover a completed publication only for the request-owned output language.
pub fn recover_deep_research_publication_receipt_in_language(
    workspace: &Path,
    query: &str,
    output_language: &str,
    run_identity: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    recover_deep_research_publication_receipt_with_language(
        workspace,
        query,
        output_language,
        run_identity,
        false,
    )
}

fn recover_deep_research_publication_receipt_with_language(
    workspace: &Path,
    query: &str,
    output_language: &str,
    run_identity: &str,
    accept_legacy_without_language: bool,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    if run_identity.is_empty() {
        return Ok(None);
    }
    crate::language::validate_deep_research_output_language(output_language)?;
    let run_receipt_path = deep_research_run_artifact_relative_directory(run_identity)
        .ok()
        .map(|directory| workspace.join(directory).join(PUBLICATION_RECEIPT_FILE_NAME));
    let legacy_receipt_path = workspace
        .join(".a3s")
        .join("research")
        .join(deep_research_report_slug(query))
        .join(PUBLICATION_RECEIPT_FILE_NAME);
    let (receipt_bytes, run_scoped) = if let Some(receipt_path) = run_receipt_path {
        match read_bounded_plain_file(&receipt_path, 64 * 1024)? {
            Some(bytes) => (bytes, true),
            None => match read_bounded_plain_file(&legacy_receipt_path, 64 * 1024)? {
                Some(bytes) => (bytes, false),
                None => return Ok(None),
            },
        }
    } else {
        match read_bounded_plain_file(&legacy_receipt_path, 64 * 1024)? {
            Some(bytes) => (bytes, false),
            None => return Ok(None),
        }
    };
    let receipt: DeepResearchPublicationReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("decode publication receipt: {error}"))?;
    if !(MINIMUM_PUBLICATION_RECEIPT_SCHEMA_VERSION..=PUBLICATION_RECEIPT_SCHEMA_VERSION)
        .contains(&receipt.schema_version)
    {
        return Err("publication receipt has an unsupported schema version".to_string());
    }
    match (receipt.schema_version, receipt.output_language.as_deref()) {
        (PUBLICATION_RECEIPT_SCHEMA_VERSION, Some(receipt_language)) => {
            crate::language::validate_deep_research_output_language(receipt_language)
                .map_err(|_| "publication receipt has an invalid output language".to_string())?;
            if !crate::language::output_language_matches(output_language, receipt_language) {
                return Ok(None);
            }
        }
        (PUBLICATION_RECEIPT_SCHEMA_VERSION, None) => {
            return Err("publication receipt omitted its output language".to_string());
        }
        (_, None) if accept_legacy_without_language => {}
        (_, None) => return Ok(None),
        (_, Some(_)) => {
            return Err(
                "legacy publication receipt unexpectedly contains an output language".to_string(),
            );
        }
    }
    if receipt.run_identity_sha256 != sha256_text(run_identity)
        || receipt.query_sha256 != sha256_text(query)
    {
        return Ok(None);
    }
    let publication = receipt.publication;
    let quality = DeepResearchPublicationQuality::from(receipt.quality);
    validate_deep_research_publication_quality(publication, quality)?;
    let artifacts = if run_scoped {
        exact_run_publication_artifacts(workspace, run_identity, publication)?
    } else {
        exact_publication_artifacts(workspace, query, publication)?
    };
    let Some(artifacts) = artifacts else {
        return Ok(None);
    };
    let markdown = std::fs::read(&artifacts.markdown)
        .map_err(|error| format!("read publication Markdown during recovery: {error}"))?;
    let html = std::fs::read(&artifacts.html)
        .map_err(|error| format!("read publication HTML during recovery: {error}"))?;
    if receipt.markdown_sha256 != sha256_bytes(&markdown)
        || receipt.html_sha256 != sha256_bytes(&html)
    {
        return Ok(None);
    }
    Ok(Some(DeepResearchPublishedReport {
        artifacts,
        publication,
        quality,
    }))
}

/// Resolve the terminal publication for one exact run.
///
/// A matching receipt is the durable authority because it binds the run
/// identity, query, publication enum, quality metrics, and both full artifact
/// digests. The workflow envelope remains a compatibility authority when no
/// matching receipt exists. If both authorities are present, they must agree
/// exactly. An invalid or incomplete workflow envelope cannot discard an
/// already committed receipt-backed publication.
pub fn resolve_deep_research_run_publication(
    workspace: &Path,
    query: &str,
    run_identity: &str,
    workflow_output: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    let receipt = recover_deep_research_publication_receipt(workspace, query, run_identity)?;
    let envelope = deep_research_evidence_first_published_report(workspace, query, workflow_output);
    reconcile_deep_research_publication_authorities(receipt, envelope)
}

/// Resolve a run publication only when every durable authority agrees on the
/// request-owned output language.
pub fn resolve_deep_research_run_publication_in_language(
    workspace: &Path,
    query: &str,
    output_language: &str,
    run_identity: &str,
    workflow_output: &str,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    let receipt = recover_deep_research_publication_receipt_in_language(
        workspace,
        query,
        output_language,
        run_identity,
    )?;
    let envelope = deep_research_evidence_first_published_report_in_language(
        workspace,
        query,
        workflow_output,
        output_language,
    );
    reconcile_deep_research_publication_authorities(receipt, envelope)
}

fn reconcile_deep_research_publication_authorities(
    receipt: Option<DeepResearchPublishedReport>,
    envelope: Result<Option<DeepResearchPublishedReport>, String>,
) -> Result<Option<DeepResearchPublishedReport>, String> {
    match receipt {
        Some(receipt) => {
            let run_scoped = receipt
                .artifacts
                .html
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                == Some(std::ffi::OsStr::new("artifacts"));
            if run_scoped {
                return Ok(Some(receipt));
            }
            if let Ok(Some(envelope)) = envelope {
                if envelope != receipt {
                    return Err(
                        "the workflow publication disagrees with the exact run receipt".to_string(),
                    );
                }
            }
            Ok(Some(receipt))
        }
        None => envelope,
    }
}

fn exact_publication_artifacts(
    workspace: &Path,
    query: &str,
    publication: DeepResearchEvidenceFirstPublication,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let slug = deep_research_report_slug(query);
    let expected = format!(".a3s/research/{slug}/index.html");
    exact_publication_artifacts_at(workspace, &expected, publication)
}

fn exact_run_publication_artifacts(
    workspace: &Path,
    run_identity: &str,
    publication: DeepResearchEvidenceFirstPublication,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let relative = match deep_research_run_artifact_relative_directory(run_identity) {
        Ok(relative) => relative,
        Err(_) => return Ok(None),
    };
    let expected = relative.join("index.html").to_string_lossy().replace('\\', "/");
    exact_publication_artifacts_at(workspace, &expected, publication)
}

fn exact_publication_artifacts_at(
    workspace: &Path,
    expected: &str,
    publication: DeepResearchEvidenceFirstPublication,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let Some(artifacts) = trusted_research_report_artifact_paths(expected, workspace) else {
        return Ok(None);
    };
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
    Ok(valid.then_some(artifacts))
}

fn sha256_text(value: &str) -> String {
    sha256_bytes(value.as_bytes())
}

fn sha256_bytes(value: &[u8]) -> String {
    use sha2::{Digest, Sha256};

    format!("{:x}", Sha256::digest(value))
}

fn read_bounded_plain_file(path: &Path, maximum_bytes: u64) -> Result<Option<Vec<u8>>, String> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("inspect publication receipt: {error}")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err("publication receipt is not a plain file".to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() > 1 {
            return Err("publication receipt is hard linked".to_string());
        }
    }
    if metadata.len() == 0 || metadata.len() > maximum_bytes {
        return Err("publication receipt has an invalid size".to_string());
    }
    std::fs::read(path)
        .map(Some)
        .map_err(|error| format!("read publication receipt: {error}"))
}
