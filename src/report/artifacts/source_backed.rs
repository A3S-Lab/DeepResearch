use std::collections::HashMap;

const SOURCE_CATALOG_MAX_SOURCES: usize = 16;
const SOURCE_CATALOG_MAX_CHUNKS: usize = 384;
const SOURCE_CATALOG_MAX_CHUNKS_PER_REPORT_SOURCE: usize = 2;
const SOURCE_CATALOG_MAX_CHUNKS_PER_PROPOSAL_SOURCE: usize = 4;
const SOURCE_CATALOG_MAX_CHUNKS_PER_INELIGIBLE_REPORT_SOURCE: usize = 1;
const SOURCE_CATALOG_MAX_CHUNK_CHARS: usize = 700;
const SOURCE_CATALOG_MAX_TITLE_CHARS: usize = 240;
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepResearchSourceCatalog {
    pub sources: Vec<DeepResearchCatalogSource>,
    pub omitted_source_count: usize,
    pub omitted_chunk_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepResearchCatalogSource {
    pub alias: String,
    pub title: String,
    pub anchor: String,
    pub chunks: Vec<String>,
    pub claim_eligible: bool,
    pub semantically_admitted: bool,
    pub relevant_track_ids: Vec<String>,
    pub coverage: Vec<DeepResearchSourceCoverage>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct DeepResearchSourceCoverage {
    pub track_id: String,
    pub completion_criterion_indexes: Vec<usize>,
    pub primary: bool,
    pub independent: bool,
}

#[derive(
    Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchEvidenceFirstPublication {
    Synthesized,
    Qualified,
    SourceBacked,
    NoEvidence,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeepResearchPublicationQuality {
    pub research_scope: DeepResearchReportScope,
    pub direct_answer_count: usize,
    pub finding_count: usize,
    pub accepted_claim_count: usize,
    pub accepted_relation_count: usize,
    pub accepted_derivation_count: usize,
    pub accepted_basis_edge_count: usize,
    pub accepted_gap_count: usize,
    pub cited_source_count: usize,
    pub substantive_character_count: usize,
    pub relevant_source_count: usize,
    pub source_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepResearchPublishedReport {
    pub artifacts: ResearchReportArtifacts,
    pub publication: DeepResearchEvidenceFirstPublication,
    pub quality: DeepResearchPublicationQuality,
}

pub fn deep_research_source_catalog(
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<DeepResearchSourceCatalog>, String> {
    let canonical = deep_research_canonical_workflow_output(workflow_output, workflow_metadata);
    if canonical.trim().is_empty() {
        return Ok(None);
    }
    let value = serde_json::from_str::<serde_json::Value>(&canonical)
        .map_err(|error| format!("decode DeepResearch source catalog: {error}"))?;
    let observed_query = value
        .get("query")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "DeepResearch source catalog omitted its query".to_string())?;
    if observed_query != query.trim() {
        return Err("DeepResearch source catalog belongs to a different query".to_string());
    }

    let selected_acquisition = selected_research_acquisition(&value);
    // An inquiry collection is the Host's closed semantic projection. Prefer
    // it over any raw bootstrap acquisition that may also be retained in the
    // workflow envelope for audit or replay.
    let Some(acquisition) = selected_acquisition
        .as_ref()
        .or_else(|| value.get("acquisition"))
    else {
        return Ok(None);
    };
    let Some(packet) = acquisition
        .get("packet")
        .filter(|packet| packet.is_object())
    else {
        return Ok(None);
    };
    if packet.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err("DeepResearch source catalog has an unsupported packet version".to_string());
    }
    let Some(raw_sources) = packet.get("sources").and_then(serde_json::Value::as_array) else {
        return Ok(None);
    };
    if raw_sources.is_empty() {
        return Ok(None);
    }
    // Only the Host-projected inquiry collection is a closed semantic
    // admission. Raw acquisition metadata records transport history and cannot
    // promote fetched bytes merely by naming a selector mode.
    let semantic_source_admission = selected_acquisition.is_some();

    let mut catalog = DeepResearchSourceCatalog {
        sources: Vec::new(),
        omitted_source_count: raw_sources.len().saturating_sub(SOURCE_CATALOG_MAX_SOURCES),
        omitted_chunk_count: 0,
    };
    let mut source_ids = HashSet::new();
    let mut chunk_ids = HashSet::new();
    let mut source_by_anchor = HashMap::<String, usize>::new();
    let mut retained_chunk_count = 0usize;

    for raw_source in raw_sources.iter().take(SOURCE_CATALOG_MAX_SOURCES) {
        let Some(source_id) =
            bounded_catalog_text(raw_source.get("source_id"), 160, stable_catalog_identity)
        else {
            catalog.omitted_source_count += 1;
            continue;
        };
        if !source_ids.insert(source_id.clone()) {
            catalog.omitted_source_count += 1;
            continue;
        }
        let Some(anchor) = raw_source
            .get("url_or_path")
            .and_then(serde_json::Value::as_str)
            .and_then(canonical_research_source_anchor)
        else {
            catalog.omitted_source_count += 1;
            continue;
        };
        let title = bounded_catalog_text(
            raw_source.get("title"),
            SOURCE_CATALOG_MAX_TITLE_CHARS,
            |_| true,
        )
        .unwrap_or_else(|| anchor.clone());
        let Some(raw_chunks) = raw_source
            .get("chunks")
            .and_then(serde_json::Value::as_array)
        else {
            catalog.omitted_source_count += 1;
            continue;
        };
        let coverage = catalog_source_coverage(raw_source, &source_id);
        let relevant_track_ids = catalog_source_relevance(raw_source, &source_id);
        let claim_eligible = semantic_source_admission && !relevant_track_ids.is_empty();

        let mut chunks = Vec::new();
        for raw_chunk in raw_chunks {
            if retained_chunk_count >= SOURCE_CATALOG_MAX_CHUNKS {
                catalog.omitted_chunk_count += 1;
                continue;
            }
            let Some(chunk_id) =
                bounded_catalog_text(raw_chunk.get("chunk_id"), 200, stable_catalog_identity)
            else {
                catalog.omitted_chunk_count += 1;
                continue;
            };
            let Some(text) = raw_chunk
                .get("text")
                .and_then(serde_json::Value::as_str)
                .and_then(sanitize_catalog_chunk)
            else {
                catalog.omitted_chunk_count += 1;
                continue;
            };
            if !chunk_ids.insert(chunk_id) {
                catalog.omitted_chunk_count += 1;
                continue;
            }
            if !chunks.iter().any(|existing| existing == &text) {
                chunks.push(text);
                retained_chunk_count += 1;
            }
        }
        if chunks.is_empty() {
            catalog.omitted_source_count += 1;
            continue;
        }
        if let Some(index) = source_by_anchor.get(&anchor).copied() {
            let retained_source = &mut catalog.sources[index];
            retained_source.claim_eligible &= claim_eligible;
            retained_source.semantically_admitted |= semantic_source_admission;
            for track_id in relevant_track_ids {
                if !retained_source.relevant_track_ids.contains(&track_id) {
                    retained_source.relevant_track_ids.push(track_id);
                }
            }
            retained_source.relevant_track_ids.sort();
            for binding in coverage {
                if !retained_source.coverage.contains(&binding) {
                    retained_source.coverage.push(binding);
                }
            }
            let retained = &mut retained_source.chunks;
            for chunk in chunks {
                if !retained.contains(&chunk) {
                    retained.push(chunk);
                }
            }
            continue;
        }
        let alias = format!("source-{}", catalog.sources.len() + 1);
        source_by_anchor.insert(anchor.clone(), catalog.sources.len());
        catalog.sources.push(DeepResearchCatalogSource {
            alias,
            title,
            anchor,
            chunks,
            claim_eligible,
            semantically_admitted: semantic_source_admission,
            relevant_track_ids,
            coverage,
        });
    }

    if catalog.sources.is_empty() {
        Ok(None)
    } else {
        Ok(Some(catalog))
    }
}

fn selected_research_acquisition(value: &serde_json::Value) -> Option<serde_json::Value> {
    if value.get("mode").and_then(serde_json::Value::as_str) != Some("inquiry_collection") {
        return None;
    }
    if value
        .pointer("/research/metadata/evidence_selection_mode")
        .and_then(serde_json::Value::as_str)
        != Some("semantic_chunk_ids_with_typed_coverage")
    {
        return None;
    }
    let results = value
        .pointer("/research/results")
        .and_then(serde_json::Value::as_array)?;
    let mut sources = Vec::new();
    for result in results {
        let Some(structured) = result.get("structured") else {
            continue;
        };
        let Some(result_sources) = structured
            .get("sources")
            .and_then(serde_json::Value::as_array)
        else {
            continue;
        };
        for source in result_sources {
            let Some(source_id) = source
                .get("source_id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
            else {
                continue;
            };
            let excerpts = source
                .get("evidence_excerpts")
                .and_then(serde_json::Value::as_array)
                .cloned()
                .unwrap_or_else(|| {
                    source
                        .get("quote_or_fact")
                        .and_then(serde_json::Value::as_str)
                        .map(|text| {
                            vec![serde_json::json!({
                                "quote_or_fact": text,
                            })]
                        })
                        .unwrap_or_default()
                });
            let chunks = excerpts
                .into_iter()
                .enumerate()
                .filter_map(|(index, excerpt)| {
                    let text = excerpt
                        .get("quote_or_fact")
                        .and_then(serde_json::Value::as_str)
                        .map(str::trim)
                        .filter(|text| !text.is_empty())?;
                    Some(serde_json::json!({
                        "chunk_id": format!("{source_id}:selected:{}", index + 1),
                        "text": text,
                    }))
                })
                .collect::<Vec<_>>();
            if chunks.is_empty() {
                continue;
            }
            let source_coverage = structured
                .get("source_coverage")
                .and_then(serde_json::Value::as_array)
                .map(|bindings| {
                    bindings
                        .iter()
                        .filter(|binding| {
                            binding
                                .get("source_id")
                                .and_then(serde_json::Value::as_str)
                                == Some(source_id.as_str())
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let source_relevance = structured
                .get("source_relevance")
                .and_then(serde_json::Value::as_array)
                .map(|bindings| {
                    bindings
                        .iter()
                        .filter(|binding| {
                            binding
                                .get("source_id")
                                .and_then(serde_json::Value::as_str)
                                == Some(source_id.as_str())
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            sources.push(serde_json::json!({
                "source_id": source_id,
                "title": source.get("title").cloned().unwrap_or(serde_json::Value::Null),
                "url_or_path": source.get("url_or_path").cloned().unwrap_or(serde_json::Value::Null),
                "reliability": source.get("reliability").cloned().unwrap_or(serde_json::Value::Null),
                "chunks": chunks,
                "source_relevance": source_relevance,
                "source_coverage": source_coverage,
            }));
        }
    }
    if sources.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "status": value
            .pointer("/research/status")
            .cloned()
            .unwrap_or_else(|| serde_json::json!("partial")),
        "packet": {
            "version": 1,
            "sources": sources,
        },
        "errors": value
            .pointer("/research/warnings/collection_errors")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
        "metadata": {
            "source_selection_mode": "semantic_chunk_ids_with_typed_coverage",
        },
    }))
}

fn catalog_source_relevance(
    source: &serde_json::Value,
    source_id: &str,
) -> Vec<String> {
    let mut retained = source
        .get("source_relevance")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_object)
        .filter(|binding| {
            binding.len() == 2
                && binding
                    .get("source_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(source_id)
        })
        .filter_map(|binding| binding.get("obligation_id"))
        .filter_map(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|track_id| stable_catalog_identity(track_id))
        .take(8)
        .map(str::to_string)
        .collect::<Vec<_>>();
    retained.sort();
    retained.dedup();
    retained
}

fn catalog_source_coverage(
    source: &serde_json::Value,
    source_id: &str,
) -> Vec<DeepResearchSourceCoverage> {
    let Some(bindings) = source
        .get("source_coverage")
        .and_then(serde_json::Value::as_array)
    else {
        return Vec::new();
    };
    let mut retained = Vec::new();
    for binding in bindings {
        let Some(object) = binding.as_object() else {
            continue;
        };
        if object.len() != 4
            || object
                .get("source_id")
                .and_then(serde_json::Value::as_str)
                != Some(source_id)
        {
            continue;
        }
        let Some(track_id) = object
            .get("obligation_id")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|value| stable_catalog_identity(value))
        else {
            continue;
        };
        let Some(raw_indexes) = object
            .get("completion_criterion_indexes")
            .and_then(serde_json::Value::as_array)
            .filter(|indexes| !indexes.is_empty() && indexes.len() <= 8)
        else {
            continue;
        };
        let Some(mut completion_criterion_indexes) = raw_indexes
            .iter()
            .map(|index| {
                index
                    .as_u64()
                    .and_then(|value| usize::try_from(value).ok())
                    .filter(|value| *value < 8)
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        completion_criterion_indexes.sort_unstable();
        completion_criterion_indexes.dedup();
        if completion_criterion_indexes.len() != raw_indexes.len() {
            continue;
        }
        let Some((primary, independent)) = catalog_source_roles(object.get("roles")) else {
            continue;
        };
        let coverage = DeepResearchSourceCoverage {
            track_id: track_id.to_string(),
            completion_criterion_indexes,
            primary,
            independent,
        };
        if !retained.contains(&coverage) {
            retained.push(coverage);
        }
    }
    retained.sort_by(|left, right| left.track_id.cmp(&right.track_id));
    retained
}

fn catalog_source_roles(value: Option<&serde_json::Value>) -> Option<(bool, bool)> {
    let roles = value?.as_array()?;
    if roles.is_empty() || roles.len() > 3 {
        return None;
    }
    let roles = roles
        .iter()
        .map(serde_json::Value::as_str)
        .collect::<Option<Vec<_>>>()?;
    let unique = roles.iter().copied().collect::<HashSet<_>>();
    if unique.len() != roles.len()
        || !unique.contains("supporting")
        || unique
            .iter()
            .any(|role| !matches!(*role, "supporting" | "primary" | "independent"))
    {
        return None;
    }
    Some((
        unique.contains("primary"),
        unique.contains("independent"),
    ))
}

pub fn materialize_deep_research_source_backed_report(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<ResearchReportArtifacts>, String> {
    let Some(catalog) = deep_research_source_catalog(query, workflow_output, workflow_metadata)?
    else {
        return Ok(None);
    };
    let slug = deep_research_report_slug(query);
    materialize_deep_research_source_catalog_report(workspace, query, &slug, &catalog).map(Some)
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
    materialize_deep_research_source_catalog_report(workspace, query, &slug, &catalog).map(Some)
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
) -> Result<ResearchReportArtifacts, String> {
    let markdown = deep_research_source_backed_markdown(query, catalog);
    let html = format!(
        "<!-- {SOURCE_BACKED_ARTIFACT_MARKER} -->\n{}",
        deep_research_degraded_report_html(query, &markdown)
    );
    let rel_html = format!(".a3s/research/{slug}/index.html");
    let (root, report_dir) = prepare_research_report_directory(workspace, slug)?;
    write_research_report_pair(
        &report_dir.join("report.md"),
        markdown,
        &report_dir.join("index.html"),
        html,
    )?;
    let artifacts = trusted_research_report_artifact_paths(&rel_html, &root)
        .ok_or_else(|| "source-backed report artifacts failed path validation".to_string())?;
    source_backed_report_artifacts(&artifacts)
        .then_some(artifacts)
        .ok_or_else(|| "source-backed report artifacts failed content validation".to_string())
}

pub fn materialize_deep_research_no_evidence_report(
    workspace: &Path,
    query: &str,
) -> Result<ResearchReportArtifacts, String> {
    let title = markdown_plain_text(&query.chars().take(180).collect::<String>());
    let markdown = format!(
        "# {title}\n\n<!-- {NO_EVIDENCE_ARTIFACT_MARKER} -->\n\n## Evidence Status\n\nThis retrieval obtained no source text that can be published safely, so no domain conclusion is generated.\n\n## Limitations\n\nThis page states only the evidence boundary. It does not treat retrieval failure as proof that relevant facts do not exist and should not be used alone for a decision.\n\n## Sources\n\nNo safely publishable source was obtained.\n"
    );
    let html = format!(
        "<!-- {NO_EVIDENCE_ARTIFACT_MARKER} -->\n{}",
        deep_research_degraded_report_html(query, &markdown)
    );
    let slug = deep_research_report_slug(query);
    let rel_html = format!(".a3s/research/{slug}/index.html");
    let (root, report_dir) = prepare_research_report_directory(workspace, &slug)?;
    write_research_report_pair(
        &report_dir.join("report.md"),
        markdown,
        &report_dir.join("index.html"),
        html,
    )?;
    let artifacts = trusted_research_report_artifact_paths(&rel_html, &root)
        .ok_or_else(|| "no-evidence report artifacts failed path validation".to_string())?;
    no_evidence_report_artifacts(&artifacts)
        .then_some(artifacts)
        .ok_or_else(|| "no-evidence report artifacts failed content validation".to_string())
}

pub fn deep_research_evidence_first_published_report(
    workspace: &Path,
    query: &str,
    workflow_output: &str,
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
        && quality.accepted_gap_count == 0;
    if quality.accepted_derivation_count > quality.accepted_claim_count
        || (quality.accepted_claim_count == 0 && !empty_claim_graph)
    {
        return Err("publication reported inconsistent typed claim-graph metrics".to_string());
    }
    match publication {
        DeepResearchEvidenceFirstPublication::Synthesized => {
            let requirements = deep_research_report_depth_requirements(quality.research_scope);
            if quality.direct_answer_count < requirements.minimum_direct_answers
                || quality.finding_count < requirements.minimum_findings
                || quality.accepted_claim_count < requirements.minimum_claims
                || quality.cited_source_count < requirements.minimum_cited_sources
                || quality.substantive_character_count
                    < requirements.minimum_substantive_characters
                || quality.cited_source_count > quality.relevant_source_count
                || quality.relevant_source_count == 0
                || quality.relevant_source_count > quality.source_count
            {
                return Err(
                    "synthesized publication failed the closed answer-depth, independent-source, or source-relevance quality gate"
                        .to_string(),
                );
            }
        }
        DeepResearchEvidenceFirstPublication::Qualified => {
            let requirements = deep_research_report_depth_requirements(quality.research_scope);
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
            {
                return Err(
                    "qualified publication failed the closed answer-depth, evidence-gap, independent-source, or source-relevance quality gate"
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

fn sanitize_catalog_chunk(value: &str) -> Option<String> {
    if value
        .chars()
        .any(|character| character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
    {
        return None;
    }
    let mut text = value.replace("\r\n", "\n").replace('\r', "\n");
    for tag in ["script", "style", "noscript"] {
        text = strip_html_element_blocks(&text, tag);
    }
    text = strip_markdown_link_targets(&text);
    text = strip_catalog_html_tags(&text);
    let lines = text
        .lines()
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    let text = lines.join(" ");
    let text = text.trim();
    (!text.is_empty() && text.chars().count() <= SOURCE_CATALOG_MAX_CHUNK_CHARS)
        .then(|| text.to_string())
}

/// Keep visible Markdown labels while removing transport URLs and image
/// syntax. The source anchor remains available in the Host-owned source
/// ledger, so inline targets add prompt weight without adding evidence.
fn strip_markdown_link_targets(value: &str) -> String {
    let without_images = strip_markdown_targets(value, true);
    let without_links = strip_markdown_targets(&without_images, false);
    strip_orphan_markdown_targets(&without_links)
}

fn strip_markdown_targets(value: &str, images_only: bool) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < characters.len() {
        let image = characters[cursor] == '!'
            && characters
                .get(cursor + 1)
                .is_some_and(|character| *character == '[');
        if images_only != image {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let label_start = if image {
            cursor + 2
        } else if !images_only && characters[cursor] == '[' {
            cursor + 1
        } else {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        };
        let Some(label_end) = characters[label_start..]
            .iter()
            .position(|character| *character == ']')
            .map(|offset| label_start + offset)
        else {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        };
        if characters.get(label_end + 1) != Some(&'(') {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let mut target_end = label_end + 2;
        let mut depth = 1usize;
        while target_end < characters.len() && depth > 0 {
            match characters[target_end] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            target_end += 1;
        }
        if depth != 0 {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let label = characters[label_start..label_end]
            .iter()
            .collect::<String>();
        if !label.trim().is_empty() {
            output.push_str(label.trim());
        }
        output.push(' ');
        cursor = target_end;
    }
    output
}

fn strip_orphan_markdown_targets(value: &str) -> String {
    let characters = value.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(value.len());
    let mut cursor = 0usize;
    while cursor < characters.len() {
        if characters[cursor] != ']'
            || characters.get(cursor + 1).is_none_or(|character| *character != '(')
        {
            output.push(characters[cursor]);
            cursor += 1;
            continue;
        }
        let mut target_end = cursor + 2;
        let mut depth = 1usize;
        while target_end < characters.len() && depth > 0 {
            match characters[target_end] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            target_end += 1;
        }
        if depth == 0 {
            output.push(' ');
            cursor = target_end;
        } else {
            output.push(characters[cursor]);
            cursor += 1;
        }
    }
    output
}

fn strip_catalog_html_tags(value: &str) -> String {
    static HTML_TAG: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let pattern = HTML_TAG.get_or_init(|| {
        regex::Regex::new(r"(?is)\\?</?[a-z][^>]{0,1200}>")
            .expect("static HTML tag regex")
    });
    pattern.replace_all(value, " ").into_owned()
}

fn strip_html_element_blocks(value: &str, tag: &str) -> String {
    let mut output = value.to_string();
    let opening = format!("<{tag}");
    let closing = format!("</{tag}>");
    loop {
        let lower = output.to_ascii_lowercase();
        let Some(start) = lower.find(&opening) else {
            break;
        };
        let end = lower[start..]
            .find(&closing)
            .map(|offset| start + offset + closing.len())
            .or_else(|| lower[start..].find('>').map(|offset| start + offset + 1))
            .unwrap_or(output.len());
        output.replace_range(start..end, " ");
    }
    output
}

include!("source_backed/artifact_validation.rs");
