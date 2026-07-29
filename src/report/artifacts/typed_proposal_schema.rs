pub fn deep_research_typed_report_proposal_schema_for(
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
) -> Result<serde_json::Value, String> {
    deep_research_typed_report_proposal_schema(catalog, context, None)
}
pub fn deep_research_typed_report_proposal_schema_for_language(
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
    output_language: &str,
) -> Result<serde_json::Value, String> {
    crate::language::validate_deep_research_output_language(output_language)?;
    deep_research_typed_report_proposal_schema(catalog, context, Some(output_language))
}

fn deep_research_typed_report_proposal_schema(
    catalog: &DeepResearchSourceCatalog,
    context: &DeepResearchReportContext,
    output_language: Option<&str>,
) -> Result<serde_json::Value, String> {
    let sources = typed_closed_sources(catalog, context);
    if sources.is_empty() {
        return Err(
            "typed report proposal schema requires at least one admitted source".to_string(),
        );
    }
    let source_ids = sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    let chunk_ids = sources
        .iter()
        .flat_map(|source| source.chunks.iter().map(|chunk| chunk.id.clone()))
        .collect::<Vec<_>>();
    let dimension_ids = typed_dimension_ids(context)?;
    let unresolved_dimension_ids = typed_unresolved_dimension_ids(catalog, context)?;
    let gap_dimension_ids = if unresolved_dimension_ids.is_empty() {
        typed_dimension_ids(context)?
    } else {
        unresolved_dimension_ids.clone()
    };
    let identifier = serde_json::json!({
        "type": "string",
        "pattern": "^[A-Za-z0-9][A-Za-z0-9._:-]{0,63}$"
    });
    let evidence_ref = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "source_id": {
                "type": "string",
                "enum": source_ids,
                "description": "An exact opaque source ID from CLOSED_TYPED_REPORT_PACKET."
            },
            "chunk_ids": {
                "type": "array",
                "minItems": 1,
                "maxItems": SOURCE_CATALOG_MAX_CHUNKS_PER_PROPOSAL_SOURCE,
                "uniqueItems": true,
                "items": {
                    "type": "string",
                    "enum": chunk_ids,
                    "description": "An exact opaque chunk ID from the same source_id."
                }
            }
        },
        "required": ["source_id", "chunk_ids"]
    });
    let derivation = serde_json::json!({
        "description": "Use only for a reproducible inference. A recommendation must set derivation to null and express its rationale through basis_claim_ids.",
        "oneOf": [
            {"type": "null"},
            {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "method": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": TYPED_REPORT_MAX_DERIVATION_CHARS
                    },
                    "input_claim_ids": {
                        "type": "array",
                        "minItems": 1,
                        "maxItems": TYPED_REPORT_MAX_BASIS_CLAIMS,
                        "uniqueItems": true,
                        "items": identifier.clone()
                    }
                },
                "required": ["method", "input_claim_ids"]
            }
        ]
    });
    let claim = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "id": identifier.clone(),
            "dimension_id": {
                "type": "string",
                "enum": dimension_ids.clone()
            },
            "placement": {
                "type": "string",
                "enum": ["direct_answer", "finding"],
                "description": "Use direct_answer only for the leading conclusion claim in a resolved dimension. Use finding for supporting detail; unresolved dimensions must remain findings plus explicit gaps."
            },
            "kind": {
                "type": "string",
                "enum": ["fact", "inference", "recommendation"]
            },
            "analysis_role": {
                "type": "string",
                "enum": [
                    "conclusion",
                    "evidence",
                    "comparison",
                    "explanation",
                    "challenge",
                    "implication",
                    "boundary"
                ],
                "description": "The claim's exact intellectual function in the dimension argument. Roles are validated independently from claim kind."
            },
            "text": {
                "type": "string",
                "minLength": 1,
                "maxLength": TYPED_REPORT_MAX_CLAIM_CHARS,
                "description": "Reader-facing claim prose. Put opaque workflow, dimension, source, chunk, query, target, and criterion IDs only in their typed fields."
            },
            "evidence_refs": {
                "type": "array",
                "maxItems": TYPED_REPORT_MAX_EVIDENCE_REFS,
                "uniqueItems": true,
                "description": "Use at most one entry per source_id and put every cited chunk from that source in the same chunk_ids array.",
                "items": evidence_ref
            },
            "basis_claim_ids": {
                "type": "array",
                "maxItems": TYPED_REPORT_MAX_BASIS_CLAIMS,
                "uniqueItems": true,
                "items": identifier.clone()
            },
            "derivation": derivation
        },
        "required": [
            "id",
            "dimension_id",
            "placement",
            "kind",
            "analysis_role",
            "text",
            "evidence_refs",
            "basis_claim_ids",
            "derivation"
        ]
    });
    let relation = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "id": identifier.clone(),
            "dimension_id": {
                "type": "string",
                "enum": typed_dimension_ids(context)?
            },
            "kind": {
                "type": "string",
                "enum": ["contradicts"],
                "description": "Use only when two facts give mutually incompatible answers to the same proposition under the same scope and time."
            },
            "claim_ids": {
                "type": "array",
                "minItems": 2,
                "maxItems": 2,
                "uniqueItems": true,
                "items": identifier.clone()
            }
        },
        "required": ["id", "dimension_id", "kind", "claim_ids"]
    });
    let gap = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "id": identifier.clone(),
            "dimension_id": {
                "type": "string",
                "enum": gap_dimension_ids
            },
            "text": {
                "type": "string",
                "minLength": 1,
                "maxLength": TYPED_REPORT_MAX_GAP_CHARS,
                "description": "A reader-facing evidence limitation stated in natural language without opaque internal IDs or workflow diagnostics."
            }
        },
        "required": ["id", "dimension_id", "text"]
    });
    let narrative_paragraph = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "purpose": {
                "type": "string",
                "enum": ["evidence", "synthesis", "implication", "boundary"],
                "description": "The paragraph's reader-facing rhetorical purpose. This controls composition only and never changes claim semantics."
            },
            "claim_ids": {
                "type": "array",
                "minItems": 1,
                "maxItems": TYPED_REPORT_MAX_CLAIMS_PER_PARAGRAPH,
                "uniqueItems": true,
                "items": identifier.clone(),
                "description": "Exact authored claim IDs in reading order. Claims supply all paragraph prose; this plan cannot add facts."
            }
        },
        "required": ["purpose", "claim_ids"]
    });
    let narrative_section = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "dimension_id": {
                "type": "string",
                "enum": dimension_ids.clone()
            },
            "heading": {
                "type": "string",
                "minLength": 2,
                "maxLength": TYPED_REPORT_MAX_SECTION_HEADING_CHARS,
                "description": "A concise, natural reader heading for this exact dimension. Do not expose internal graph or workflow terminology."
            },
            "paragraphs": {
                "type": "array",
                "maxItems": TYPED_REPORT_MAX_PARAGRAPHS_PER_SECTION,
                "items": narrative_paragraph
            }
        },
        "required": ["dimension_id", "heading", "paragraphs"]
    });
    let report_language = output_language.map_or_else(
        || {
            serde_json::json!({
                "type": "string",
                "minLength": 2,
                "maxLength": 32,
                "pattern": "^[A-Za-z][A-Za-z0-9-]{1,31}$"
            })
        },
        |language| {
            serde_json::json!({
                "type": "string",
                "enum": [language],
                "description": "The exact Host-owned output language for all reader-facing prose."
            })
        },
    );
    Ok(serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "report_language": report_language,
            "labels": typed_report_labels_schema(),
            "claims": {
                "type": "array",
                "maxItems": TYPED_REPORT_MAX_CLAIMS,
                "description": format!(
                    "A bounded claim graph with at most {TYPED_REPORT_MAX_CLAIMS} claims total."
                ),
                "items": claim
            },
            "relations": {
                "type": "array",
                "maxItems": TYPED_REPORT_MAX_RELATIONS,
                "items": relation
            },
            "gaps": {
                "type": "array",
                "maxItems": if unresolved_dimension_ids.is_empty() {
                    0
                } else {
                    TYPED_REPORT_MAX_GAPS
                },
                "items": gap
            },
            "narrative": {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "sections": {
                        "type": "array",
                        "minItems": dimension_ids.len(),
                        "maxItems": dimension_ids.len(),
                        "items": narrative_section
                    }
                },
                "required": ["sections"]
            }
        },
        "required": [
            "report_language",
            "labels",
            "claims",
            "relations",
            "gaps",
            "narrative"
        ]
    }))
}

fn typed_report_labels_schema() -> serde_json::Value {
    let heading = || {
        serde_json::json!({
            "type": "string",
            "minLength": 1,
            "maxLength": REPORT_PROPOSAL_MAX_HEADING_CHARS,
            "description": "A short section heading, never an answer, claim, or sentence."
        })
    };
    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "answer": heading(),
            "findings": heading(),
            "recommendations": heading(),
            "limitations": heading(),
            "evidence_boundary": {
                "type": "string",
                "minLength": 8,
                "maxLength": REPORT_PROPOSAL_MAX_EVIDENCE_BOUNDARY_CHARS,
                "description": format!(
                    "One concise evidence-boundary sentence with at most {REPORT_PROPOSAL_MAX_EVIDENCE_BOUNDARY_CHARS} characters."
                )
            },
            "sources": heading(),
            "contradiction": heading(),
            "inference": heading(),
            "basis": heading(),
            "derivation": heading()
        },
        "required": [
            "answer",
            "findings",
            "recommendations",
            "limitations",
            "evidence_boundary",
            "sources",
            "contradiction",
            "inference",
            "basis",
            "derivation"
        ]
    })
}
