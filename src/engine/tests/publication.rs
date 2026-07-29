#[test]
fn cancellation_drops_in_flight_ports_before_publication() {
    let runtime = BlockingRuntime {
        events: Mutex::new(Vec::new()),
        publication_count: Mutex::new(0),
    };
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
    let cancellation = DeepResearchCancellation::new();
    let cancel = cancellation.clone();
    let request = DeepResearchRequest::new(
        "cancelled-engine-run",
        "A query whose ports remain pending",
        EvidenceScope::WebAndWorkspace,
    )
    .with_current_date("2026-07-23");
    let run = engine.execute_request(request, cancellation);
    let cancel_after_first_poll = async move {
        futures::future::ready(()).await;
        cancel.cancel();
    };

    let (result, ()) = futures::executor::block_on(future::join(run, cancel_after_first_poll));

    assert_eq!(result, Err(DeepResearchEngineError::Cancelled));
    assert_eq!(
        *runtime
            .publication_count
            .lock()
            .expect("publication count lock"),
        0
    );
    assert!(matches!(
        runtime.events.lock().expect("events lock").last(),
        Some(DeepResearchEvent::RunCancelled { run_id })
            if run_id == "cancelled-engine-run"
    ));
}
#[test]
fn admitted_closed_evidence_report_replaces_the_staged_source_snapshot() {
    let query = "Which Nimbus release is supported?";
    let source_output = source_output(query);
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output,
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("closed evidence run must publish");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(run.output["publication"]["status"], "synthesized");
    assert_eq!(
        run.output["publication"]["quality"]["direct_answer_count"],
        1
    );
    assert_eq!(run.output["publication"]["quality"]["finding_count"], 1);
    assert_eq!(
        run.output["publication"]["quality"]["accepted_claim_count"],
        2
    );
    assert_eq!(
        run.output["publication"]["html"],
        format!(
            ".a3s/research/{}/index.html",
            deep_research_report_slug(query)
        )
    );
    let publications = runtime.publications.lock().expect("publication lock");
    let [PublicationRequest::SourceBacked {
        run_id: source_run_id,
        quality: source_quality,
        ..
    }, PublicationRequest::Synthesized {
        run_id: report_run_id,
        quality: report_quality,
        ..
    }] = publications.as_slice()
    else {
        panic!("the engine must stage source-backed and synthesized publications");
    };
    assert_eq!(source_run_id, "engine-test");
    assert_eq!(report_run_id, "engine-test");
    assert_eq!(source_quality.accepted_claim_count, 0);
    assert_eq!(source_quality.source_count, 1);
    assert_eq!(report_quality.accepted_claim_count, 2);
    assert_eq!(report_quality.cited_source_count, 1);
    assert!(runtime.progress.lock().expect("progress lock").contains(
        &ResearchProgress::Completed(ResearchStage::FinalPublication)
    ));
    let generation_requests = runtime
        .generation_requests
        .lock()
        .expect("generation requests lock");
    let planning_request = generation_requests
        .iter()
        .find(|request| request.stage == GenerationStage::Planning)
        .expect("planning generation request");
    assert_eq!(
        planning_request.arguments["max_repair_attempts"], 1,
        "one schema repair should preserve a useful semantic outline"
    );
    let report_request = generation_requests
        .iter()
        .find(|request| request.stage == GenerationStage::Report)
        .expect("report generation request");
    assert_eq!(
        report_request.arguments["max_repair_attempts"], 0,
        "the durable attempt policy must not be multiplied by an inner repair loop"
    );
    let claim = &report_request.arguments["schema"]["properties"]["claims"]["items"]["properties"];
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["source_id"]["enum"],
        serde_json::json!(["source-1"])
    );
    assert_eq!(
        claim["evidence_refs"]["items"]["properties"]["chunk_ids"]["items"]["enum"],
        serde_json::json!(["source-1:chunk:1"])
    );
    assert_eq!(
        claim["dimension_id"]["enum"],
        serde_json::json!(["support.boundary"])
    );
    let editorial_request = generation_requests
        .iter()
        .find(|request| request.stage == GenerationStage::Editorial)
        .expect("editorial generation request");
    assert_eq!(
        editorial_request.arguments["schema_name"],
        "deep_research_typed_editorial_plan"
    );
    assert_eq!(
        editorial_request.arguments["max_repair_attempts"], 0,
        "editorial generation must share the durable outer attempt policy"
    );
    let editorial_prompt = editorial_request.arguments["prompt"]
        .as_str()
        .expect("editorial prompt");
    assert!(editorial_prompt.contains(
        "The official Nimbus record identifies version 2 and September 2027 as the support boundary."
    ));
    assert!(editorial_prompt.contains("Nimbus version 2 receives fixes through September 2027."));
    assert!(!editorial_prompt.contains("source-1"));
    assert!(!editorial_prompt.contains("https://research.example"));
    assert_eq!(
        run.output["research"]["metadata"]["required_model_generation_count"],
        2
    );
    assert_eq!(
        run.output["research"]["metadata"]["model_generation_count"],
        2
    );
    assert_eq!(
        run.output["research"]["metadata"]["synthesis_mode"],
        "model_claim_graph_editorial"
    );
    assert_eq!(
        run.output["execution"]["maximum_report_generation_count"],
        4
    );
}

#[test]
fn editorial_success_changes_reader_flow_without_changing_admitted_claims() {
    const FINDING: &str =
        "The official Nimbus record identifies version 2 and September 2027 as the support boundary.";
    const EDITED_FINDING: &str =
        "In the official Nimbus record, version 2 remains the supported line through September 2027.";
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    )
    .with_editorial(Ok(serde_json::json!({
        "quality_review": {
            "publication_ready": true,
            "dimension_reviews": [{
                "dimension_id": "support.boundary",
                "verdict": "pass",
                "issue_codes": []
            }],
            "claim_reviews": [{
                "claim_id": "nimbus-answer",
                "verdict": "pass",
                "temporal_status": "current_as_of_evidence",
                "issue_codes": []
            }, {
                "claim_id": "nimbus-boundary",
                "verdict": "pass",
                "temporal_status": "current_as_of_evidence",
                "issue_codes": []
            }]
        },
        "claim_rewrites": [{
            "claim_id": "nimbus-answer",
            "text": "Nimbus version 2 receives fixes through September 2027."
        }, {
            "claim_id": "nimbus-boundary",
            "text": EDITED_FINDING
        }],
        "sections": [{
            "dimension_id": "support.boundary",
            "heading": "Support remains bounded through September 2027",
            "paragraphs": [{
                "purpose": "evidence",
                "claim_ids": ["nimbus-boundary"]
            }]
        }]
    })));
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("valid editorial plan must publish");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(run.quality.accepted_claim_count, 2);
    assert!(run.output["research"]["warnings"]["editorial_error"].is_null());
    let publications = runtime.publications.lock().expect("publication lock");
    let PublicationRequest::Synthesized { report, .. } =
        publications.last().expect("synthesized publication")
    else {
        panic!("the final publication must carry the edited report");
    };
    assert!(report
        .markdown
        .contains("### Support remains bounded through September 2027"));
    assert_eq!(report.markdown.matches(FINDING).count(), 0);
    assert_eq!(report.markdown.matches(EDITED_FINDING).count(), 1);
}

#[test]
fn editorial_failure_cannot_publish_an_unreviewed_report_as_complete() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    )
    .with_editorial(Err("scripted editorial timeout".to_string()));
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("editorial failure must preserve a source-backed preview");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::SourceBacked
    );
    assert_eq!(
        run.output["research"]["metadata"]["synthesis_mode"],
        "model_claim_graph_editorial_fallback"
    );
    assert_eq!(
        run.output["research"]["warnings"]["editorial_error"],
        "scripted editorial timeout"
    );
    assert!(run.output["research"]["warnings"]["report_error"]
        .as_str()
        .is_some_and(|error| error.contains("independent commercial quality review")));
    let publications = runtime.publications.lock().expect("publication lock");
    assert!(matches!(
        publications.last(),
        Some(PublicationRequest::SourceBacked { .. })
    ));
}

#[test]
fn typed_graph_quality_survives_the_engine_publication_boundary() {
    let query = "Compare the two bounded records";
    let proposal = serde_json::json!({
        "report_language": "en",
        "labels": {
            "answer": "Direct Answer",
            "findings": "Findings",
            "recommendations": "Evidence-Based Recommendations",
            "limitations": "Limitations",
            "evidence_boundary": "This report publishes no conclusion beyond the fetched evidence.",
            "sources": "Sources",
            "contradiction": "Contradiction",
            "inference": "Inference",
            "basis": "Basis",
            "derivation": "Derivation"
        },
        "claims": [{
            "id": "first-record",
            "dimension_id": "support.boundary",
            "placement": "finding",
            "kind": "fact",
            "analysis_role": "evidence",
            "text": "The first bounded record reports 100 units.",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }, {
            "id": "second-record",
            "dimension_id": "support.boundary",
            "placement": "finding",
            "kind": "fact",
            "analysis_role": "evidence",
            "text": "The second bounded record reports 80 units.",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }, {
            "id": "difference",
            "dimension_id": "support.boundary",
            "placement": "direct_answer",
            "kind": "inference",
            "analysis_role": "conclusion",
            "text": "The two bounded records differ by 20 units.",
            "evidence_refs": [],
            "basis_claim_ids": ["first-record", "second-record"],
            "derivation": {
                "method": "100 - 80 = 20",
                "input_claim_ids": ["first-record", "second-record"]
            }
        }],
        "relations": [{
            "id": "record-conflict",
            "dimension_id": "support.boundary",
            "kind": "contradicts",
            "claim_ids": ["first-record", "second-record"]
        }],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "support.boundary",
                "heading": "How the two bounded records compare",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["first-record", "second-record"]
                }]
            }]
        }
    });
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(proposal)),
        source_output(query),
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("typed graph report must publish");

    assert_eq!(
        run.output["publication"]["quality"]["accepted_relation_count"],
        1
    );
    assert_eq!(
        run.output["publication"]["quality"]["accepted_derivation_count"],
        1
    );
    assert_eq!(
        run.output["publication"]["quality"]["accepted_basis_edge_count"],
        2
    );
    assert_eq!(
        run.output["publication"]["quality"]["analytical_claim_count"],
        1
    );
    assert_eq!(
        run.output["publication"]["quality"]["cross_source_synthesis_count"],
        0
    );
    assert_eq!(
        run.output["publication"]["quality"]["accepted_gap_count"],
        0
    );
    let publications = runtime.publications.lock().expect("publication lock");
    let PublicationRequest::Synthesized { quality, .. } = &publications[1] else {
        panic!("the final publication must carry typed graph quality");
    };
    assert_eq!(quality.accepted_relation_count, 1);
    assert_eq!(quality.accepted_derivation_count, 1);
    assert_eq!(quality.accepted_basis_edge_count, 2);
    assert_eq!(quality.analytical_claim_count, 1);
    assert_eq!(quality.cross_source_synthesis_count, 0);
    assert_eq!(quality.accepted_gap_count, 0);
}

#[test]
fn planned_semantic_evidence_reaches_synthesis_without_a_host_allowlist() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        empty_bootstrap_output(query),
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("semantic inquiry evidence must publish");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(
        run.output["research"]["metadata"]["relevant_source_count"],
        1
    );
    assert_eq!(run.output["publication"]["quality"]["finding_count"], 1);
}

#[test]
fn malformed_planned_envelope_does_not_promote_raw_bootstrap_bytes() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        None,
        source_output(query),
        malformed_packet_output(query, "inquiry_collection"),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("a malformed planned envelope must fail closed with an artifact");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(run.output["publication"]["quality"]["source_count"], 0);
    assert_eq!(
        run.output["publication"]["quality"]["accepted_claim_count"],
        0
    );
    assert!(
        run.output["research"]["warnings"]["report_error"].is_string(),
        "the structural decode failure must remain visible as bounded diagnostics"
    );
    assert!(runtime
        .progress
        .lock()
        .expect("progress lock")
        .iter()
        .any(|progress| matches!(
            progress,
            ResearchProgress::Degraded {
                stage: ResearchStage::PlannedRetrieval,
                ..
            }
        )));
}

#[test]
fn malformed_bootstrap_envelope_does_not_poison_valid_planned_evidence() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        malformed_packet_output(query, "bootstrap_acquisition"),
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("valid planned evidence must survive a malformed bootstrap sibling");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(
        run.output["publication"]["quality"]["accepted_claim_count"],
        2
    );
    let requests = runtime
        .workflow_requests
        .lock()
        .expect("workflow request lock");
    assert!(
        requests[1]
            .arguments
            .pointer("/input/bootstrap_acquisition")
            .is_none(),
        "an invalid bootstrap packet cannot become planned-retrieval input"
    );
    drop(requests);
    assert!(runtime
        .progress
        .lock()
        .expect("progress lock")
        .iter()
        .any(|progress| matches!(
            progress,
            ResearchProgress::Degraded {
                stage: ResearchStage::BootstrapRetrieval,
                ..
            }
        )));
}

#[test]
fn malformed_retrieval_envelopes_publish_a_no_evidence_boundary() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        None,
        malformed_packet_output(query, "bootstrap_acquisition"),
        malformed_packet_output(query, "inquiry_collection"),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("malformed retrieval envelopes must fail closed with an artifact");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(run.output["publication"]["quality"]["source_count"], 0);
    assert_eq!(
        run.output["publication"]["quality"]["accepted_claim_count"],
        0
    );
    assert!(matches!(
        runtime
            .publications
            .lock()
            .expect("publication lock")
            .as_slice(),
        [PublicationRequest::NoEvidence { .. }]
    ));
}

#[test]
fn synthesized_publication_failure_restores_the_staged_source_report() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    )
    .fail_synthesized_publication("injected final publication failure");
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("the staged source-backed publication must survive");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::SourceBacked
    );
    assert_eq!(run.output["publication"]["status"], "source_backed");
    assert_eq!(
        run.output["publication"]["quality"]["accepted_claim_count"],
        0
    );
    assert_eq!(
        run.output["publication"]["quality"]["cited_source_count"],
        0
    );
    assert_eq!(
        run.output["research"]["metadata"]["accepted_report_block_count"],
        0
    );
    assert_eq!(
        run.output["research"]["metadata"]["accepted_claim_count"],
        0
    );
    assert_eq!(
        runtime
            .publications
            .lock()
            .expect("publication lock")
            .iter()
            .map(|request| match request {
                PublicationRequest::SourceBacked { .. } => "source_backed",
                PublicationRequest::Synthesized { .. } => "synthesized",
                PublicationRequest::NoEvidence { .. } => "no_evidence",
            })
            .collect::<Vec<_>>(),
        ["source_backed", "synthesized", "source_backed"],
        "the engine must re-confirm the deterministic source publication after a failed final write"
    );
    let progress = runtime.progress.lock().expect("progress lock");
    assert!(progress.contains(&ResearchProgress::Completed(
        ResearchStage::ReportGeneration
    )));
    assert!(matches!(
        progress.as_slice(),
        [
            ..,
            ResearchProgress::Degraded {
                stage: ResearchStage::FinalPublication,
                ..
            },
            ResearchProgress::Completed(ResearchStage::SourcePublication)
        ]
    ));
}

#[test]
fn final_write_then_error_restores_a_valid_source_backed_artifact_pair() {
    let query = "Which Nimbus release is supported?";
    let workspace = tempfile::tempdir().expect("publication workspace");
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    );
    let publication = MutatingPublicationPort::new(workspace.path().to_path_buf());
    let engine = DeepResearchEngine::new(&runtime, &runtime, &publication, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("source-backed recovery must replace the failed final generation");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::SourceBacked
    );
    let published =
        deep_research_evidence_first_published_report(workspace.path(), query, &run.output_json())
            .expect("validate recovered publication")
            .expect("recover source-backed publication");
    assert_eq!(
        published.publication,
        DeepResearchEvidenceFirstPublication::SourceBacked
    );
    assert_eq!(published.artifacts, run.artifacts);
    let markdown = std::fs::read_to_string(&run.artifacts.markdown).expect("read recovered report");
    assert!(
        markdown.contains("The official Nimbus record states that version 2 receives fixes"),
        "{markdown}"
    );
    assert!(
        markdown.contains("A3S_DEEP_RESEARCH_ARTIFACT:source_backed:v1"),
        "{markdown}"
    );
    assert!(
        !markdown.contains("A3S_DEEP_RESEARCH_ARTIFACT:synthesized:v1"),
        "{markdown}"
    );
}
