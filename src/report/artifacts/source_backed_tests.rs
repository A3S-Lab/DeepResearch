use super::*;

include!("source_backed_tests/catalog.rs");
fn inquiry_relevance_fixture(
    query: &str,
    relevant_obligation_ids: serde_json::Value,
) -> serde_json::Value {
    let source_relevance = relevant_obligation_ids
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(|obligation_id| {
            serde_json::json!({
                "source_id": "source:aurora",
                "obligation_id": obligation_id,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "query": query,
        "mode": "inquiry_collection",
        "research": {
            "status": "partial",
            "metadata": {
                "evidence_selection_mode": "semantic_chunk_ids_with_typed_coverage"
            },
            "results": [{
                "task_id": "evidence_retrieval:source:aurora",
                "agent": "workflow",
                "success": true,
                "structured": {
                    "summary": "Semantic selection retained partial evidence.",
                    "sources": [{
                        "source_id": "source:aurora",
                        "title": "Aurora migration note",
                        "url_or_path": "https://research.example/aurora/partial",
                        "reliability": "fetched",
                        "evidence_excerpts": [{
                            "focus": "Assess the migration boundary.",
                            "quote_or_fact": "The note establishes one bounded migration constraint."
                        }]
                    }],
                    "source_coverage": [],
                    "source_relevance": source_relevance,
                    "relevant_obligation_ids": relevant_obligation_ids,
                    "key_evidence": ["The note establishes one bounded migration constraint."],
                    "contradictions": [],
                    "confidence": "Closed-evidence review required.",
                    "gaps": []
                }
            }],
            "warnings": {
                "collection_errors": []
            }
        }
    })
}

include!("source_backed_tests/publication.rs");
fn source_backed_fixture(query: &str, sources: serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "query": query,
        "mode": "evidence_first_inquiry",
        "acquisition": {
            "status": "success",
            "metadata": {
                "source_selection_mode": "semantic_candidate_ids"
            },
            "packet": {
                "version": 1,
                "focuses": [],
                "sources": sources,
            }
        },
        "research": {
            "status": "failed",
            "warnings": {
                "collection_errors": ["model extraction failed"]
            }
        }
    })
}

fn fallback_source_backed_fixture(query: &str, sources: serde_json::Value) -> serde_json::Value {
    let mut fixture = source_backed_fixture(query, sources);
    fixture["acquisition"]["metadata"]["source_selection_mode"] =
        serde_json::json!("bounded_discovery_fallback");
    fixture
}

fn source_fixture(source_id: &str, title: &str, anchor: &str, text: &str) -> serde_json::Value {
    serde_json::json!({
        "source_id": source_id,
        "title": title,
        "url_or_path": anchor,
        "reliability": "fetched",
        "chunks": [{
            "chunk_id": format!("{source_id}:chunk:1"),
            "text": text,
        }]
    })
}

fn evidence_first_publication_fixture(query: &str, slug: &str, status: &str) -> serde_json::Value {
    let source_count = usize::from(status == "source_backed");
    serde_json::json!({
        "query": query,
        "mode": "evidence_first_report",
        "publication": {
            "status": status,
            "markdown": format!(".a3s/research/{slug}/report.md"),
            "html": format!(".a3s/research/{slug}/index.html"),
            "quality": {
                "direct_answer_count": 0,
                "finding_count": 0,
                "accepted_claim_count": 0,
                "cited_source_count": 0,
                "substantive_character_count": 0,
                "relevant_source_count": source_count,
                "source_count": source_count
            }
        }
    })
}
