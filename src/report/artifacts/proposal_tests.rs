use super::*;

fn report_context(scope: DeepResearchReportScope) -> DeepResearchReportContext {
    DeepResearchReportContext {
        scope,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.primary",
            "title": "Primary evidence",
            "focus": "Establish the requested answer from traceable evidence.",
            "material": true,
            "completion_criteria": ["The answer and its support are both explicit."],
        })],
    }
}

#[test]
fn proposal_schema_keeps_model_output_block_only() {
    let schema = deep_research_report_proposal_schema();
    let encoded = schema.to_string();

    assert!(encoded.contains("\"summary\""));
    assert!(encoded.contains("\"findings\""));
    assert!(encoded.contains("\"recommendations\""));
    assert!(encoded.contains("\"limitations\""));
    assert!(encoded.contains("\"source_aliases\""));
    assert!(!encoded.contains("\"url\""));
    assert!(!encoded.contains("\"markdown\""));
    assert!(!encoded.contains("\"sources\""));
}

#[test]
fn proposal_prompt_contains_semantic_scope_tracks_and_no_catalog_anchor() {
    let catalog = focused_catalog();
    let context = report_context(DeepResearchReportScope::Comprehensive);

    let prompt = deep_research_report_proposal_prompt_at(
        "Assess Nimbus support and migration risk",
        "2026-07-23",
        &catalog,
        &context,
    )
    .expect("closed prompt");

    assert!(prompt.contains("\"research_scope\":\"comprehensive\""));
    assert!(prompt.contains("\"research_tracks\""));
    assert!(prompt.contains("\"findings\":4"));
    assert!(prompt.contains("\"supported_claims\":5"));
    assert!(prompt.contains("\"cited_sources\":2"));
    assert!(!prompt.contains("https://docs.rs/nimbus"));
}

#[test]
fn host_builds_fixed_sections_citations_and_ledger_from_valid_blocks() {
    let catalog = focused_catalog();
    let proposal = serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"]
        }],
        "recommendations": [],
        "limitations": []
    });

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("admit proposal")
    .expect("qualified focused report");

    assert!(admitted.markdown.contains("## Direct Answer"));
    assert!(admitted.markdown.contains("## Findings"));
    assert!(admitted.markdown.contains("## Sources"));
    assert!(admitted.markdown.contains("[[1]]("));
    assert_eq!(admitted.direct_answer_block_count, 1);
    assert_eq!(admitted.finding_block_count, 1);
    assert_eq!(admitted.accepted_claim_count, 2);
    assert_eq!(admitted.cited_source_count, 1);
}

#[test]
fn invalid_blocks_are_removed_without_losing_valid_siblings() {
    let catalog = focused_catalog();
    let proposal = serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"]
        }, {
            "text": "Nimbus is supported through 2099.",
            "source_aliases": ["source-99"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"]
        }],
        "recommendations": [],
        "limitations": []
    });

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("admit proposal")
    .expect("valid siblings survive");

    assert!(!admitted.markdown.contains("2099"));
    assert_eq!(admitted.accepted_claim_count, 2);
    assert_eq!(admitted.rejected_block_count, 1);
}

#[test]
fn comprehensive_scope_rejects_a_shallow_single_fact_report() {
    let catalog = comprehensive_catalog();
    let proposal = serde_json::json!({
        "summary": [{
            "text": "The Aurora program entered public operation in July 2026.",
            "source_aliases": ["source-1"]
        }],
        "findings": [{
            "text": "The official release records the July 2026 public operation milestone.",
            "source_aliases": ["source-1"]
        }],
        "recommendations": [],
        "limitations": []
    });

    let admitted = admit_deep_research_report_proposal_at(
        "Provide a complete assessment of the Aurora program",
        "2026-07-23",
        &catalog,
        &report_context(DeepResearchReportScope::Comprehensive),
        proposal,
    )
    .expect("evaluate comprehensive proposal");

    assert!(admitted.is_none());
}

#[test]
fn recommendation_padding_cannot_satisfy_comprehensive_depth() {
    let catalog = comprehensive_catalog();
    let proposal = serde_json::json!({
        "summary": [{
            "text": "The Aurora program entered public operation in July 2026.",
            "source_aliases": ["source-1"]
        }],
        "findings": [{
            "text": "The official release records the July 2026 public operation milestone.",
            "source_aliases": ["source-1"]
        }],
        "recommendations": [{
            "text": "Organizations should review the July 2026 release before adoption and document every operational dependency in detail.",
            "source_aliases": ["source-1"]
        }, {
            "text": "Teams should conduct extensive planning, validation, monitoring, training, governance, and contingency exercises before migration.",
            "source_aliases": ["source-1"]
        }],
        "limitations": []
    });

    let admitted = admit_deep_research_report_proposal_at(
        "Provide a complete assessment of the Aurora program",
        "2026-07-23",
        &catalog,
        &report_context(DeepResearchReportScope::Comprehensive),
        proposal,
    )
    .expect("evaluate padded proposal");

    assert!(admitted.is_none());
}

#[test]
fn unknown_publisher_cannot_pass_the_strong_support_gate() {
    let catalog = DeepResearchSourceCatalog {
        sources: vec![DeepResearchCatalogSource {
            alias: "source-1".to_string(),
            title: "Nimbus support note".to_string(),
            anchor: "https://unknown.example/nimbus".to_string(),
            chunks: vec![
                "Nimbus version 2 receives fixes through September 2027. The support record identifies version 2 and September 2027 as the maintenance boundary."
                    .to_string(),
            ],
            claim_eligible: true,
        }],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    };
    let proposal = serde_json::json!({
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"]
        }],
        "findings": [{
            "text": "The support record identifies version 2 and September 2027 as the maintenance boundary.",
            "source_aliases": ["source-1"]
        }],
        "recommendations": [],
        "limitations": []
    });

    let admitted = admit_deep_research_report_proposal(
        "Which Nimbus release is supported?",
        &catalog,
        proposal,
    )
    .expect("evaluate unknown publisher");

    assert!(admitted.is_none());
}

fn focused_catalog() -> DeepResearchSourceCatalog {
    DeepResearchSourceCatalog {
        sources: vec![DeepResearchCatalogSource {
            alias: "source-1".to_string(),
            title: "Official Nimbus support record".to_string(),
            anchor: "https://docs.rs/nimbus/latest/nimbus/support".to_string(),
            chunks: vec![
                "Nimbus version 2 receives fixes through September 2027. The support record identifies version 2 and September 2027 as the maintenance boundary."
                    .to_string(),
            ],
            claim_eligible: true,
        }],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    }
}

fn comprehensive_catalog() -> DeepResearchSourceCatalog {
    DeepResearchSourceCatalog {
        sources: vec![
            DeepResearchCatalogSource {
                alias: "source-1".to_string(),
                title: "Aurora official release".to_string(),
                anchor: "https://docs.rs/aurora/latest/aurora/release".to_string(),
                chunks: vec![
                    "The Aurora program entered public operation in July 2026. The official release records the July 2026 public operation milestone."
                        .to_string(),
                ],
                claim_eligible: true,
            },
            DeepResearchCatalogSource {
                alias: "source-2".to_string(),
                title: "Independent Aurora assessment".to_string(),
                anchor: "https://www.reuters.com/technology/aurora-assessment".to_string(),
                chunks: vec![
                    "The independent assessment documents Aurora deployment constraints, operating costs, adoption patterns, and unresolved implementation risks."
                        .to_string(),
                ],
                claim_eligible: true,
            },
        ],
        omitted_source_count: 0,
        omitted_chunk_count: 0,
    }
}
