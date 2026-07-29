use std::collections::HashMap;

const SOURCE_CATALOG_MAX_SOURCES: usize = 32;
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

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize,
)]
pub struct DeepResearchPublicationQuality {
    pub research_scope: DeepResearchReportScope,
    pub direct_answer_count: usize,
    pub finding_count: usize,
    pub accepted_claim_count: usize,
    pub accepted_relation_count: usize,
    pub accepted_derivation_count: usize,
    pub accepted_basis_edge_count: usize,
    #[serde(default)]
    pub analytical_claim_count: usize,
    #[serde(default)]
    pub cross_source_synthesis_count: usize,
    #[serde(default)]
    pub resolved_material_dimension_count: usize,
    #[serde(default)]
    pub deeply_analyzed_dimension_count: usize,
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
    Ok(
        deep_research_attributed_source_catalog(query, workflow_output, workflow_metadata)?
            .map(|catalog| catalog.catalog),
    )
}

pub(crate) fn deep_research_attributed_source_catalog(
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<DeepResearchAttributedSourceCatalog>, String> {
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
    let mut catalog_index_by_source_id = HashMap::<String, usize>::new();
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
            catalog_index_by_source_id.insert(source_id, index);
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
        let catalog_index = catalog.sources.len();
        source_by_anchor.insert(anchor.clone(), catalog_index);
        catalog_index_by_source_id.insert(source_id, catalog_index);
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
        let attribution = catalog_source_attribution(
            packet.get("source_attribution"),
            raw_sources,
            &catalog_index_by_source_id,
            &catalog,
        );
        Ok(Some(DeepResearchAttributedSourceCatalog {
            catalog,
            attribution,
        }))
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
    let source_attribution = (value
        .pointer("/research/metadata/source_attribution_status")
        .and_then(serde_json::Value::as_str)
        == Some("verified"))
    .then(|| {
        value
            .pointer("/research/metadata/source_attribution")
            .cloned()
    })
    .flatten()
    .unwrap_or(serde_json::Value::Null);
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
            "source_attribution": source_attribution,
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

include!("source_backed/attribution.rs");
include!("source_backed/publication.rs");
include!("source_backed/sanitization.rs");
include!("source_backed/artifact_validation.rs");
