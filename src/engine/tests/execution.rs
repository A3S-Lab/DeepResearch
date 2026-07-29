#[test]
fn planning_failure_keeps_exact_query_bootstrap_and_publishes_no_evidence() {
    let query = "Investigate the current support boundary";
    let runtime = FakeRuntime::new(
        Err("planner unavailable".to_string()),
        None,
        empty_bootstrap_output(query),
        empty_planned_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("fallback run must publish");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(
        run.output["research"]["metadata"]["planning_mode"],
        "exact_query_fallback"
    );
    assert_eq!(run.output["publication"]["status"], "no_evidence");
    let requests = runtime
        .workflow_requests
        .lock()
        .expect("workflow request lock");
    assert_eq!(requests[0].stage, WorkflowStage::Bootstrap);
    assert_eq!(requests[1].stage, WorkflowStage::PlannedRetrieval);
    assert_eq!(
        requests[1]
            .arguments
            .pointer("/input/research_plan/search_queries")
            .and_then(Value::as_array)
            .expect("fallback search query"),
        &[Value::String(query.to_string())]
    );
    assert_eq!(
        requests[1]
            .arguments
            .pointer("/input/research_plan/freshness_required"),
        Some(&serde_json::json!(true)),
        "planner failure must retain the stronger unknown-freshness contract"
    );
    drop(requests);
    assert!(matches!(
        runtime
            .publications
            .lock()
            .expect("publication lock")
            .as_slice(),
        [PublicationRequest::NoEvidence {
            run_id,
            query: published,
            quality,
            ..
        }] if run_id == "engine-test"
            && published == query
            && quality.source_count == 0
            && quality.accepted_claim_count == 0
    ));
}
#[test]
fn planner_prose_in_another_language_falls_back_to_the_users_query() {
    let query = "核查 Nimbus 当前的支持边界";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        None,
        empty_bootstrap_output(query),
        empty_planned_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
    let request = DeepResearchRequest::new(
        "planner-language-fallback",
        query,
        EvidenceScope::WebAndWorkspace,
    )
    .with_current_date("2026-07-25");

    let result = futures::executor::block_on(
        engine.execute_request(request, DeepResearchCancellation::new()),
    )
    .expect("wrong-language planning prose must use the deterministic fallback");

    assert_eq!(result.output["output_language"], "zh");
    assert_eq!(
        result.output["research"]["metadata"]["planning_mode"],
        "exact_query_fallback"
    );
    assert!(result.output["research"]["warnings"]["report_error"]
        .as_str()
        .is_some_and(|error| error.contains("different language")));
    let workflow_requests = runtime
        .workflow_requests
        .lock()
        .expect("workflow request lock");
    assert_eq!(
        workflow_requests[1].arguments["input"]["research_plan"]["report_title"],
        query
    );
}

#[test]
fn typed_execution_keeps_lifecycle_and_publication_outcome_distinct() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
    let request =
        DeepResearchRequest::new("typed-engine-run", query, EvidenceScope::WebAndWorkspace)
            .with_current_date("2026-07-23");

    let result = futures::executor::block_on(
        engine.execute_request(request, DeepResearchCancellation::new()),
    )
    .expect("typed run must publish");

    assert_eq!(result.lifecycle, DeepResearchLifecycle::Completed);
    assert_eq!(result.publication, PublicationOutcome::Synthesized);
    assert_eq!(result.quality.accepted_claim_count, 2);
    assert_eq!(
        result.output["publication"]["artifact_kinds"],
        serde_json::json!(["markdown", "html"])
    );
    assert_eq!(result.output["research"]["status"], "synthesized");
    assert!(result.output["publication"].get("markdown").is_none());
    assert!(result.output["publication"].get("html").is_none());
    assert!(!result.output.to_string().contains(".a3s/research/"));
    let generation_requests = runtime
        .generation_requests
        .lock()
        .expect("generation requests lock");
    let planner_request = generation_requests
        .iter()
        .find(|request| request.stage == GenerationStage::Planning)
        .expect("planner generation request");
    assert_eq!(planner_request.max_attempts, DEFAULT_PLANNER_MAX_ATTEMPTS);
    assert_eq!(
        planner_request.arguments["timeout_ms"],
        DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS
    );
    assert_eq!(
        planner_request.execution_timeout_ms,
        DEFAULT_PLANNER_STAGE_TIMEOUT_MS
    );
    let report_requests = generation_requests
        .iter()
        .filter(|request| {
            matches!(
                request.stage,
                GenerationStage::Report | GenerationStage::Editorial
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(report_requests.len(), 2);
    assert!(report_requests.iter().all(|request| {
        request.max_attempts == 2
            && request.arguments["timeout_ms"] == 240_000
            && request.execution_timeout_ms == 495_000
    }));
    drop(generation_requests);
    let events = runtime.events.lock().expect("events lock");
    assert!(matches!(
        events.first(),
        Some(DeepResearchEvent::RunStarted { run_id, query: observed })
            if run_id == "typed-engine-run" && observed == query
    ));
    assert!(events.iter().any(|event| matches!(
        event,
        DeepResearchEvent::PublicationCompleted {
            run_id,
            outcome: PublicationOutcome::Synthesized,
            ..
        } if run_id == "typed-engine-run"
    )));
    assert!(matches!(
        events.last(),
        Some(DeepResearchEvent::RunCompleted {
            run_id,
            outcome: PublicationOutcome::Synthesized,
        }) if run_id == "typed-engine-run"
    ));
}

#[test]
fn adapter_asserted_retrieval_receipts_are_carried_only_as_audit_provenance() {
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        with_retrieval_provenance(source_output(query), 'a'),
        with_retrieval_provenance(inquiry_collection_source_output(query), 'd'),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("receipt provenance must not alter successful synthesis");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(run.quality.accepted_claim_count, 2);
    let audits = run.output["execution"]["retrieval_run_provenance"]
        .as_array()
        .expect("stage provenance audits");
    assert_eq!(audits.len(), 2);
    assert_eq!(audits[0]["stage"], "bootstrap");
    assert_eq!(audits[1]["stage"], "planned_retrieval");
    assert!(audits
        .iter()
        .all(|audit| audit["status"] == "shape_validated"));
    assert_eq!(
        audits[0]["envelope"]["bindings"][0]["receipt_sha256"],
        "a".repeat(64)
    );
    assert_eq!(
        audits[1]["envelope"]["bindings"][0]["receipt_sha256"],
        "d".repeat(64)
    );

    let generation_requests = runtime
        .generation_requests
        .lock()
        .expect("generation requests lock");
    assert!(generation_requests.iter().all(|request| {
        let arguments = request.arguments.to_string();
        !arguments.contains(&"a".repeat(64)) && !arguments.contains(&"d".repeat(64))
    }));
}

#[test]
fn untrusted_nested_or_invalid_provenance_cannot_affect_publication_quality() {
    let query = "Investigate the current support boundary";
    let mut bootstrap = empty_bootstrap_output(query);
    bootstrap.metadata = Some(serde_json::json!({
        "dynamic_workflow": {
            RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
                "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
                "bindings": [],
            }
        }
    }));
    let mut planned = empty_planned_output(query);
    planned.metadata = Some(serde_json::json!({
        RETRIEVAL_RUN_PROVENANCE_METADATA_KEY: {
            "schema": RETRIEVAL_RUN_PROVENANCE_V1_SCHEMA,
            "bindings": [{"receipt_sha256": "not-a-binding"}],
        }
    }));
    let runtime = FakeRuntime::new(Err("planner unavailable".to_string()), None, bootstrap, planned);
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("invalid optional provenance must not replace evidence gates");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(run.quality.accepted_claim_count, 0);
    assert_eq!(run.quality.source_count, 0);
    let audits = run.output["execution"]["retrieval_run_provenance"]
        .as_array()
        .expect("invalid top-level provenance audit");
    assert_eq!(audits.len(), 1, "nested workflow data is not host provenance");
    assert_eq!(audits[0]["stage"], "planned_retrieval");
    assert_eq!(audits[0]["status"], "rejected");
    assert!(audits[0].get("envelope").is_none());
}

#[test]
fn failed_workflow_calls_cannot_contribute_success_output_provenance() {
    let query = "Investigate the current support boundary";
    let runtime = FakeRuntime::new(
        Err("planner unavailable".to_string()),
        None,
        with_retrieval_provenance(empty_bootstrap_output(query), 'a'),
        with_retrieval_provenance(empty_planned_output(query), 'd'),
    )
    .fail_planned_retrieval("planned retrieval transport failed");
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);

    let run = futures::executor::block_on(engine.execute(workflow_args(query)))
        .expect("a failed planned retrieval must degrade without inventing metadata");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    let audits = run.output["execution"]["retrieval_run_provenance"]
        .as_array()
        .expect("successful bootstrap provenance audit");
    assert_eq!(audits.len(), 1);
    assert_eq!(audits[0]["stage"], "bootstrap");
    assert_eq!(audits[0]["status"], "shape_validated");
    assert_eq!(
        audits[0]["envelope"]["bindings"][0]["receipt_sha256"],
        "a".repeat(64)
    );
    assert!(run.output.to_string().find(&"d".repeat(64)).is_none());
}

#[test]
fn chinese_output_language_survives_generation_and_every_publication_path() {
    let query = "Nimbus 目前支持哪个版本，维护边界是什么？";
    let runtime = FakeRuntime::new(
        Ok(valid_chinese_outline()),
        Some(Ok(valid_chinese_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    );
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
    let request =
        DeepResearchRequest::new("typed-engine-zh", query, EvidenceScope::WebAndWorkspace)
            .with_current_date("2026-07-25");

    let result = futures::executor::block_on(
        engine.execute_request(request, DeepResearchCancellation::new()),
    )
    .expect("Chinese typed run must publish");

    assert_eq!(result.output["output_language"], "zh");
    let generation_requests = runtime
        .generation_requests
        .lock()
        .expect("generation requests lock");
    let report_request = generation_requests
        .iter()
        .find(|request| request.stage == GenerationStage::Report)
        .expect("report generation request");
    assert_eq!(
        report_request.arguments["schema"]["properties"]["report_language"]["enum"],
        serde_json::json!(["zh"])
    );
    drop(generation_requests);
    assert!(matches!(
        runtime
            .publications
            .lock()
            .expect("publication lock")
            .as_slice(),
        [
            PublicationRequest::SourceBacked {
                output_language: source_language,
                ..
            },
            PublicationRequest::Synthesized {
                output_language: report_language,
                ..
            }
        ] if source_language == "zh" && report_language == "zh"
    ));

    let no_evidence_runtime = FakeRuntime::new(
        Err("planner unavailable".to_string()),
        None,
        empty_bootstrap_output(query),
        empty_planned_output(query),
    );
    let no_evidence_engine = DeepResearchEngine::new(
        &no_evidence_runtime,
        &no_evidence_runtime,
        &no_evidence_runtime,
        &no_evidence_runtime,
    );
    let no_evidence_request = DeepResearchRequest::new(
        "typed-engine-zh-empty",
        query,
        EvidenceScope::WebAndWorkspace,
    )
    .with_current_date("2026-07-25");

    let no_evidence_result = futures::executor::block_on(
        no_evidence_engine.execute_request(no_evidence_request, DeepResearchCancellation::new()),
    )
    .expect("Chinese no-evidence run must publish");

    assert_eq!(no_evidence_result.output["output_language"], "zh");
    assert!(matches!(
        no_evidence_runtime
            .publications
            .lock()
            .expect("publication lock")
            .as_slice(),
        [PublicationRequest::NoEvidence {
            output_language,
            ..
        }] if output_language == "zh"
    ));
}
