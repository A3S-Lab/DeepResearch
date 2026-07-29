#[test]
fn independent_editorial_planning_can_rewrite_and_reorder_admitted_claims() {
    const ANSWER: &str =
        "The closed records establish that durable settlement, not request acceptance alone, is the completion boundary.";
    const INTAKE: &str =
        "The intake record shows that a request crosses validation before downstream execution begins.";
    const SETTLEMENT: &str =
        "A separately maintained settlement record shows that the terminal state remains inspectable after request acceptance.";
    const COMPARISON: &str =
        "Read together, the records distinguish an early acceptance signal from the later durable state that proves completion.";
    const EXPLANATION: &str =
        "The distinction matters because validation can succeed before downstream work reaches a recoverable terminal state.";
    const IMPLICATION: &str =
        "Acceptance testing should therefore verify both entry validation and durable settlement as separate observable checkpoints.";
    const BOUNDARY: &str =
        "The conclusion is limited to paths where the acceptance signal and terminal state can be observed independently.";
    const EDITED_INTAKE: &str =
        "At intake, the record shows a request crossing validation before downstream execution begins.";
    const EDITED_SETTLEMENT: &str =
        "By contrast, a separately maintained settlement record keeps the terminal state inspectable after request acceptance.";

    let context = one_track_context();
    let catalog = catalog(vec![
        source(
            "source-1",
            "Closed intake record",
            "https://example.test/intake",
            &format!("{ANSWER} {INTAKE}"),
        ),
        source(
            "source-2",
            "Closed settlement record",
            "https://example.test/settlement",
            SETTLEMENT,
        ),
    ]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("intake", "finding", INTAKE, "source-1"),
            fact("settlement", "finding", SETTLEMENT, "source-2"),
            inference_for_dimension(
                "comparison",
                "request.answer",
                "finding",
                "comparison",
                COMPARISON,
                &["intake", "settlement"],
            ),
            inference_for_dimension(
                "explanation",
                "request.answer",
                "finding",
                "explanation",
                EXPLANATION,
                &["comparison"],
            ),
            inference_for_dimension(
                "implication",
                "request.answer",
                "finding",
                "implication",
                IMPLICATION,
                &["explanation"],
            ),
            inference_for_dimension(
                "boundary",
                "request.answer",
                "finding",
                "boundary",
                BOUNDARY,
                &["comparison"],
            ),
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "request.answer",
                "heading": "From acceptance to durable completion",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["intake", "settlement"]
                }, {
                    "purpose": "synthesis",
                    "claim_ids": ["comparison", "explanation"]
                }, {
                    "purpose": "implication",
                    "claim_ids": ["implication"]
                }, {
                    "purpose": "boundary",
                    "claim_ids": ["boundary"]
                }]
            }]
        }
    });
    let draft = admit_deep_research_typed_report_draft_in_language_at(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed draft admission")
    .expect("admitted typed draft");
    let baseline = draft.report.clone();

    let prompt = deep_research_typed_editorial_prompt(&draft).expect("editorial prompt");
    assert!(prompt.contains(INTAKE));
    assert!(prompt.contains(SETTLEMENT));
    assert!(prompt.contains(COMPARISON));
    assert!(prompt.contains(ANSWER));
    assert!(prompt.contains("mapped request requirements"));
    assert!(prompt.contains("announced_future"));
    assert!(!prompt.contains("source-1"));
    assert!(!prompt.contains("https://example.test"));
    let schema = deep_research_typed_editorial_schema(&draft);
    assert_eq!(
        schema["properties"]["quality_review"]["properties"]["dimension_reviews"]["minItems"],
        1
    );
    assert_eq!(
        schema["properties"]["quality_review"]["properties"]["claim_reviews"]["minItems"],
        7
    );
    assert_eq!(schema["properties"]["claim_rewrites"]["minItems"], 7);
    assert_eq!(
        schema["properties"]["sections"]["items"]["properties"]["paragraphs"]["items"]
            ["properties"]["claim_ids"]["items"]["enum"],
        serde_json::json!([
            "intake",
            "settlement",
            "comparison",
            "explanation",
            "implication",
            "boundary"
        ])
    );

    let editorial = serde_json::json!({
        "quality_review": {
            "publication_ready": true,
            "dimension_reviews": [{
                "dimension_id": "request.answer",
                "verdict": "pass",
                "issue_codes": []
            }],
            "claim_reviews": [{
                "claim_id": "answer",
                "verdict": "pass",
                "temporal_status": "not_time_sensitive",
                "issue_codes": []
            }, {
                "claim_id": "intake",
                "verdict": "pass",
                "temporal_status": "occurred",
                "issue_codes": []
            }, {
                "claim_id": "settlement",
                "verdict": "pass",
                "temporal_status": "occurred",
                "issue_codes": []
            }, {
                "claim_id": "comparison",
                "verdict": "pass",
                "temporal_status": "not_applicable",
                "issue_codes": []
            }, {
                "claim_id": "explanation",
                "verdict": "pass",
                "temporal_status": "not_applicable",
                "issue_codes": []
            }, {
                "claim_id": "implication",
                "verdict": "pass",
                "temporal_status": "not_applicable",
                "issue_codes": []
            }, {
                "claim_id": "boundary",
                "verdict": "pass",
                "temporal_status": "not_applicable",
                "issue_codes": []
            }]
        },
        "claim_rewrites": [
            { "claim_id": "answer", "text": ANSWER },
            { "claim_id": "intake", "text": EDITED_INTAKE },
            { "claim_id": "settlement", "text": EDITED_SETTLEMENT },
            { "claim_id": "comparison", "text": COMPARISON },
            { "claim_id": "explanation", "text": EXPLANATION },
            { "claim_id": "implication", "text": IMPLICATION },
            { "claim_id": "boundary", "text": BOUNDARY }
        ],
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "Completion begins after acceptance",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["settlement", "intake"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let mut rejected_editorial = editorial.clone();
    rejected_editorial["quality_review"]["publication_ready"] = serde_json::json!(false);
    rejected_editorial["quality_review"]["dimension_reviews"][0]["verdict"] =
        serde_json::json!("fail");
    rejected_editorial["quality_review"]["dimension_reviews"][0]["issue_codes"] =
        serde_json::json!(["shallow_analysis"]);
    let error = apply_deep_research_typed_commercial_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        rejected_editorial,
    )
    .expect_err("a failed independent review must block commercial publication");
    assert!(error.contains("rejected commercial publication"), "{error}");

    let edited = apply_deep_research_typed_commercial_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        editorial,
    )
    .expect("valid editorial plan");

    assert!(edited
        .markdown
        .contains("### Completion begins after acceptance"));
    assert!(edited.markdown.find(EDITED_SETTLEMENT) < edited.markdown.find(EDITED_INTAKE));
    assert!(!edited.markdown.contains(INTAKE));
    assert!(!edited.markdown.contains(SETTLEMENT));
    assert_eq!(edited.markdown.matches(EDITED_INTAKE).count(), 1);
    assert_eq!(edited.markdown.matches(EDITED_SETTLEMENT).count(), 1);
    for claim in [ANSWER, COMPARISON, EXPLANATION, IMPLICATION, BOUNDARY] {
        assert_eq!(edited.markdown.matches(claim).count(), 1, "{claim}");
    }
    assert_eq!(edited.accepted_claim_count, baseline.accepted_claim_count);
    assert!(edited.substantive_character_count > 0);

    let out_of_order = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "An invalid dependency order",
            "paragraphs": [{
                "purpose": "synthesis",
                "claim_ids": ["comparison"]
            }, {
                "purpose": "evidence",
                "claim_ids": ["settlement", "intake"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        out_of_order,
    )
    .expect_err("a dependent claim cannot precede its premises");
    assert!(
        error.contains("supporting premises before dependent claims"),
        "{error}"
    );

    let omitted_claim = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "An incomplete editorial plan",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["intake", "settlement"]
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft.clone(),
        omitted_claim,
    )
    .expect_err("every admitted finding must appear exactly once");
    assert!(
        error.contains("every supporting claim exactly once"),
        "{error}"
    );

    let attempted_rewrite = serde_json::json!({
        "sections": [{
            "dimension_id": "request.answer",
            "heading": "A forbidden rewrite",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["intake", "settlement"],
                "text": "Rewritten prose"
            }, {
                "purpose": "synthesis",
                "claim_ids": ["comparison", "explanation"]
            }, {
                "purpose": "implication",
                "claim_ids": ["implication"]
            }, {
                "purpose": "boundary",
                "claim_ids": ["boundary"]
            }]
        }]
    });
    let error = apply_deep_research_typed_editorial_plan(
        "trace the execution boundary",
        "2026-07-24",
        "en",
        &catalog,
        &context,
        draft,
        attempted_rewrite,
    )
    .expect_err("editorial output cannot carry rewritten prose");
    assert!(error.contains("decode typed editorial plan"), "{error}");
}
#[test]
fn near_duplicate_claim_prose_is_rejected_instead_of_padding_depth() {
    const ANSWER: &str =
        "The closed record establishes a bounded execution result for the reviewed environment.";
    const FIRST: &str = "The retained execution record identifies the validated entry boundary, the owning component, the durable settlement state, and the observable completion signal for the reviewed environment.";
    const SECOND: &str = "The retained execution record identifies the validated entry boundary, the owning component, the durable settlement state, and the observable completion signal for the reviewed environment.";
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed execution record",
        "https://example.test/execution",
        &format!("{ANSWER} {FIRST} {SECOND}"),
    )]);
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact("answer", "direct_answer", ANSWER, "source-1"),
            fact("first", "finding", FIRST, "source-1"),
            fact("second", "finding", SECOND, "source-1"),
        ],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "request.answer",
                "heading": "The reviewed execution boundary",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["first", "second"]
                }]
            }]
        }
    });

    let error = admit_deep_research_typed_report_proposal_at(
        "trace the execution boundary",
        "2026-07-24",
        &catalog,
        &context,
        proposal,
    )
    .expect_err("near-duplicate prose must fail closed");
    assert!(error.contains("near-duplicate claim prose"), "{error}");
}

#[test]
fn editorial_rewrite_cannot_change_or_invent_numeric_facts() {
    let mut proposal = serde_json::json!({
        "claims": [{
            "id": "support-boundary",
            "text": "Support remains available through September 2027."
        }]
    });
    let rewrites = vec![TypedWireEditorialClaimRewrite {
        claim_id: "support-boundary".to_string(),
        text: "Support remains available through September 2028.".to_string(),
    }];

    let error = apply_typed_editorial_claim_rewrites(&mut proposal, &rewrites)
        .expect_err("editorial prose must preserve every numeric fact");

    assert!(error.contains("numeric fact"), "{error}");
}

#[test]
fn active_typed_schema_closes_source_chunk_and_dimension_ids() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The closed record establishes the requested answer.",
    )]);

    let schema =
        deep_research_typed_report_proposal_schema_for(&catalog, &context).expect("typed schema");
    let claim = &schema["properties"]["claims"]["items"]["properties"];

    assert_eq!(
        claim["dimension_id"]["enum"],
        serde_json::json!(["request.answer"])
    );
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["source_id"]["enum"],
        serde_json::json!(["source-1"])
    );
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["chunk_ids"]["items"]["enum"],
        serde_json::json!(["source-1:chunk:1"])
    );
    assert!(schema["properties"].get("relations").is_some());
    assert!(schema["properties"].get("narrative").is_some());
    assert!(claim.get("basis_claim_ids").is_some());
    assert!(claim.get("derivation").is_some());
}

#[test]
fn typed_report_contract_defines_labels_as_short_structural_headings() {
    let context = one_track_context();
    let focused_catalog = catalog(vec![source(
        "source-1",
        "Closed record",
        "https://example.test/record",
        "The closed record establishes the requested answer.",
    )]);

    let schema = deep_research_typed_report_proposal_schema_for(&focused_catalog, &context)
        .expect("typed schema");
    assert_eq!(
        schema["properties"]["labels"]["properties"]["answer"]["description"],
        "A short section heading, never an answer, claim, or sentence."
    );
    assert_eq!(
        schema["properties"]["claims"]["description"],
        "A bounded claim graph with at most 72 claims total."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["evidence_refs"]["description"],
        "Use at most one entry per source_id and put every cited chunk from that source in the same chunk_ids array."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["evidence_refs"]["maxItems"],
        serde_json::json!(4)
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["placement"]["description"],
        "Use direct_answer only for the leading conclusion claim in a resolved dimension. Use finding for supporting detail; unresolved dimensions must remain findings plus explicit gaps."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["text"]["description"],
        "Reader-facing claim prose. Put opaque workflow, dimension, source, chunk, query, target, and criterion IDs only in their typed fields."
    );
    assert_eq!(
        schema["properties"]["claims"]["items"]["properties"]["derivation"]["description"],
        "Use only for a reproducible inference. A recommendation must set derivation to null and express its rationale through basis_claim_ids."
    );
    assert_eq!(
        schema["properties"]["relations"]["items"]["properties"]["kind"]["description"],
        "Use only when two facts give mutually incompatible answers to the same proposition under the same scope and time."
    );
    assert_eq!(
        schema["properties"]["gaps"]["items"]["properties"]["text"]["description"],
        "A reader-facing evidence limitation stated in natural language without opaque internal IDs or workflow diagnostics."
    );
    assert_eq!(
        schema["properties"]["labels"]["properties"]["evidence_boundary"]["description"],
        "One concise evidence-boundary sentence with at most 360 characters."
    );

    let prompt = deep_research_typed_report_proposal_prompt_at(
        "focused query",
        "2026-07-24",
        &focused_catalog,
        &context,
    )
    .expect("typed prompt");
    assert!(prompt.contains(
        "Every label except evidence_boundary is a short interface label, never an answer, claim, or sentence."
    ));
    assert!(prompt.contains(
        "Return at most 72 claims total and keep evidence_boundary to one concise sentence of at most 360 characters."
    ));
    assert!(prompt.contains(
        "Order each dimension's claims as a coherent argument, not an inventory of source summaries."
    ));
    assert!(prompt.contains(
        "write neighboring claims so they read as one developing argument rather than isolated cards placed side by side"
    ));
    assert!(prompt.contains(
        "Write topical synthesis for the reader; do not narrate the retrieval process or introduce claims as source-by-source summaries."
    ));
    assert!(prompt.contains(
        "In a comprehensive report, keep useful partial claims from an unresolved dimension as findings and pair them with its gap"
    ));
    let comprehensive_context = two_track_comprehensive_context();
    let comprehensive_catalog = catalog(vec![
        source_for_track(
            "source-1",
            "First record",
            "https://example.test/first",
            "The first record establishes its material dimension.",
            "request.first",
        ),
        source_for_track(
            "source-2",
            "Second record",
            "https://example.test/second",
            "The second record establishes its material dimension.",
            "request.second",
        ),
    ]);
    let comprehensive_prompt = deep_research_typed_report_proposal_prompt_at(
        "comprehensive query",
        "2026-07-24",
        &comprehensive_catalog,
        &comprehensive_context,
    )
    .expect("comprehensive typed prompt");
    assert!(comprehensive_prompt.contains(
        "If every material dimension is unresolved, return only useful findings and gaps so the Host retains an explicitly incomplete preview."
    ));
    assert!(prompt.contains("never place a bounded conclusion in the report summary."));
    assert!(prompt.contains(
        "Use at most one evidence_ref per source_id; when one source contributes multiple chunks"
    ));
    assert!(prompt.contains(
        "Attribute a single-source anecdote, estimate, forecast, benchmark, or reported case to that source"
    ));
    assert!(prompt.contains(
        "A recommendation must remain normative, name every factual or inferential premise in basis_claim_ids, set derivation to null"
    ));
    assert!(prompt.contains(
        "Different tools, capabilities, scopes, maturity levels, or compatible parts of one system are not a contradiction."
    ));
    assert!(prompt.contains(
        "Opaque workflow, dimension, source, chunk, query, target, and criterion IDs belong only in their typed fields"
    ));
    assert!(prompt.contains(
        "A workspace source establishes its contents, not that it belongs to the active build"
    ));
}
