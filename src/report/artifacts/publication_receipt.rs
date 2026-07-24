const PUBLICATION_RECEIPT_SCHEMA_VERSION: u32 = 2;
const MINIMUM_PUBLICATION_RECEIPT_SCHEMA_VERSION: u32 = 1;
const PUBLICATION_RECEIPT_FILE_NAME: &str = "publication-receipt.json";

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct DeepResearchPublicationReceipt {
    schema_version: u32,
    run_identity_sha256: String,
    query_sha256: String,
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
/// The receipt stores only closed publication metadata and full artifact
/// digests. Reader prose, source vocabulary, domains, languages, and titles
/// never participate in restart recovery.
pub fn record_deep_research_publication_receipt(
    workspace: &Path,
    query: &str,
    run_identity: &str,
    publication: DeepResearchEvidenceFirstPublication,
    quality: DeepResearchPublicationQuality,
    artifacts: &ResearchReportArtifacts,
) -> Result<(), String> {
    if run_identity.is_empty() {
        return Err("publication receipt requires a non-empty run identity".to_string());
    }
    validate_deep_research_publication_quality(publication, quality)?;
    let trusted = exact_publication_artifacts(workspace, query, publication)?
        .ok_or_else(|| "publication receipt artifacts failed validation".to_string())?;
    if &trusted != artifacts {
        return Err("publication receipt artifacts do not match the query path".to_string());
    }
    let markdown = std::fs::read(&trusted.markdown)
        .map_err(|error| format!("read publication Markdown for receipt: {error}"))?;
    let html = std::fs::read(&trusted.html)
        .map_err(|error| format!("read publication HTML for receipt: {error}"))?;
    let receipt = DeepResearchPublicationReceipt {
        schema_version: PUBLICATION_RECEIPT_SCHEMA_VERSION,
        run_identity_sha256: sha256_text(run_identity),
        query_sha256: sha256_text(query),
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
    if run_identity.is_empty() {
        return Ok(None);
    }
    let slug = deep_research_report_slug(query);
    let receipt_path = workspace
        .join(".a3s")
        .join("research")
        .join(slug)
        .join(PUBLICATION_RECEIPT_FILE_NAME);
    let receipt_bytes = match read_bounded_plain_file(&receipt_path, 64 * 1024)? {
        Some(bytes) => bytes,
        None => return Ok(None),
    };
    let receipt: DeepResearchPublicationReceipt = serde_json::from_slice(&receipt_bytes)
        .map_err(|error| format!("decode publication receipt: {error}"))?;
    if !(MINIMUM_PUBLICATION_RECEIPT_SCHEMA_VERSION..=PUBLICATION_RECEIPT_SCHEMA_VERSION)
        .contains(&receipt.schema_version)
    {
        return Err("publication receipt has an unsupported schema version".to_string());
    }
    if receipt.run_identity_sha256 != sha256_text(run_identity)
        || receipt.query_sha256 != sha256_text(query)
    {
        return Ok(None);
    }
    let publication = receipt.publication;
    let quality = DeepResearchPublicationQuality::from(receipt.quality);
    validate_deep_research_publication_quality(publication, quality)?;
    let Some(artifacts) = exact_publication_artifacts(workspace, query, publication)? else {
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

    match receipt {
        Some(receipt) => {
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
    let Some(artifacts) = trusted_research_report_artifact_paths(&expected, workspace) else {
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
