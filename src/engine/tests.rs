use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::channel::oneshot;
use futures::future;

use super::*;
use crate::planner::deep_research_loop_contract;
use crate::report::{
    deep_research_evidence_first_published_report, deep_research_report_slug,
    materialize_deep_research_admitted_report, materialize_deep_research_no_evidence_report,
    materialize_deep_research_source_backed_report, DeepResearchEvidenceFirstPublication,
};

struct FakeRuntime {
    planning: Mutex<Option<Result<Value, String>>>,
    report: Mutex<Option<Result<Value, String>>>,
    editorial: Mutex<Option<Result<Value, String>>>,
    generation_requests: Mutex<Vec<GenerationRequest>>,
    bootstrap: WorkflowOutput,
    planned: WorkflowOutput,
    workflow_requests: Mutex<Vec<WorkflowRequest>>,
    publications: Mutex<Vec<PublicationRequest>>,
    progress: Mutex<Vec<ResearchProgress>>,
    events: Mutex<Vec<DeepResearchEvent>>,
    bootstrap_signal: Mutex<Option<oneshot::Sender<()>>>,
    planner_gate: Mutex<Option<oneshot::Receiver<()>>>,
    synthesized_publication_failure: Mutex<Option<String>>,
}

impl FakeRuntime {
    fn new(
        planning: Result<Value, String>,
        report: Option<Result<Value, String>>,
        bootstrap: WorkflowOutput,
        planned: WorkflowOutput,
    ) -> Self {
        let (bootstrap_signal, planner_gate) = oneshot::channel();
        let editorial = report.as_ref().and_then(|report| {
            report.as_ref().ok().map(|proposal| {
                Ok(proposal
                    .get("narrative")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({ "sections": [] })))
            })
        });
        Self {
            planning: Mutex::new(Some(planning)),
            report: Mutex::new(report),
            editorial: Mutex::new(editorial),
            generation_requests: Mutex::new(Vec::new()),
            bootstrap,
            planned,
            workflow_requests: Mutex::new(Vec::new()),
            publications: Mutex::new(Vec::new()),
            progress: Mutex::new(Vec::new()),
            events: Mutex::new(Vec::new()),
            bootstrap_signal: Mutex::new(Some(bootstrap_signal)),
            planner_gate: Mutex::new(Some(planner_gate)),
            synthesized_publication_failure: Mutex::new(None),
        }
    }

    fn fail_synthesized_publication(self, message: impl Into<String>) -> Self {
        *self
            .synthesized_publication_failure
            .lock()
            .expect("synthesized publication failure lock") = Some(message.into());
        self
    }

    fn with_editorial(self, editorial: Result<Value, String>) -> Self {
        *self.editorial.lock().expect("editorial result lock") = Some(editorial);
        self
    }
}

#[async_trait]
impl StructuredGenerationPort for FakeRuntime {
    async fn generate_object(&self, request: GenerationRequest) -> Result<Value, String> {
        self.generation_requests
            .lock()
            .expect("generation requests lock")
            .push(request.clone());
        match request.stage {
            GenerationStage::Planning => {
                let gate = self.planner_gate.lock().expect("planner gate lock").take();
                if let Some(gate) = gate {
                    gate.await
                        .map_err(|_| "bootstrap did not start alongside planning".to_string())?;
                }
                self.planning
                    .lock()
                    .expect("planning result lock")
                    .take()
                    .expect("one planning request")
            }
            GenerationStage::Report => self
                .report
                .lock()
                .expect("report result lock")
                .take()
                .expect("one report request"),
            GenerationStage::Editorial => self
                .editorial
                .lock()
                .expect("editorial result lock")
                .take()
                .expect("one editorial request"),
        }
    }
}

#[async_trait]
impl WorkflowExecutionPort for FakeRuntime {
    async fn execute_workflow(&self, request: WorkflowRequest) -> Result<WorkflowOutput, String> {
        self.workflow_requests
            .lock()
            .expect("workflow request lock")
            .push(request.clone());
        match request.stage {
            WorkflowStage::Bootstrap => {
                if let Some(signal) = self
                    .bootstrap_signal
                    .lock()
                    .expect("bootstrap signal lock")
                    .take()
                {
                    signal.send(()).expect("planner retained bootstrap signal");
                }
                Ok(self.bootstrap.clone())
            }
            WorkflowStage::PlannedRetrieval => Ok(self.planned.clone()),
        }
    }
}

#[async_trait]
impl PublicationPort for FakeRuntime {
    async fn publish(
        &self,
        request: PublicationRequest,
    ) -> Result<ResearchReportArtifacts, String> {
        let synthesized = matches!(request, PublicationRequest::Synthesized { .. });
        self.publications
            .lock()
            .expect("publication lock")
            .push(request);
        if synthesized {
            if let Some(error) = self
                .synthesized_publication_failure
                .lock()
                .expect("synthesized publication failure lock")
                .take()
            {
                return Err(error);
            }
        }
        Ok(ResearchReportArtifacts {
            markdown: PathBuf::from("/virtual/report.md"),
            html: PathBuf::from("/virtual/index.html"),
        })
    }
}

#[async_trait]
impl ProgressPort for FakeRuntime {
    async fn report_progress(&self, progress: ResearchProgress) -> Result<(), String> {
        self.progress.lock().expect("progress lock").push(progress);
        Ok(())
    }

    async fn report_event(&self, event: DeepResearchEvent) -> Result<(), String> {
        if let Some(progress) = event.legacy_progress() {
            self.progress.lock().expect("progress lock").push(progress);
        }
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }
}

struct MutatingPublicationPort {
    workspace: PathBuf,
    fail_synthesized_once: Mutex<bool>,
}

impl MutatingPublicationPort {
    fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            fail_synthesized_once: Mutex::new(true),
        }
    }
}

#[async_trait]
impl PublicationPort for MutatingPublicationPort {
    async fn publish(
        &self,
        request: PublicationRequest,
    ) -> Result<ResearchReportArtifacts, String> {
        match request {
            PublicationRequest::SourceBacked {
                query,
                workflow_output,
                workflow_metadata,
                ..
            } => materialize_deep_research_source_backed_report(
                &self.workspace,
                &query,
                &workflow_output,
                workflow_metadata.as_ref(),
            )?
            .ok_or_else(|| "source-backed fixture publication produced no artifacts".to_string()),
            PublicationRequest::Synthesized { query, report, .. } => {
                let artifacts =
                    materialize_deep_research_admitted_report(&self.workspace, &query, &report)?;
                let mut fail = self
                    .fail_synthesized_once
                    .lock()
                    .expect("synthesized failure lock");
                if *fail {
                    *fail = false;
                    Err("injected error after replacing the report pair".to_string())
                } else {
                    Ok(artifacts)
                }
            }
            PublicationRequest::NoEvidence { query, .. } => {
                materialize_deep_research_no_evidence_report(&self.workspace, &query)
            }
        }
    }
}

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
    assert_eq!(planner_request.max_attempts, 2);
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

struct BlockingRuntime {
    events: Mutex<Vec<DeepResearchEvent>>,
    publication_count: Mutex<usize>,
}

#[async_trait]
impl StructuredGenerationPort for BlockingRuntime {
    async fn generate_object(&self, _request: GenerationRequest) -> Result<Value, String> {
        future::pending().await
    }
}

#[async_trait]
impl WorkflowExecutionPort for BlockingRuntime {
    async fn execute_workflow(&self, _request: WorkflowRequest) -> Result<WorkflowOutput, String> {
        future::pending().await
    }
}

#[async_trait]
impl PublicationPort for BlockingRuntime {
    async fn publish(
        &self,
        _request: PublicationRequest,
    ) -> Result<ResearchReportArtifacts, String> {
        *self
            .publication_count
            .lock()
            .expect("publication count lock") += 1;
        future::pending().await
    }
}

#[async_trait]
impl ProgressPort for BlockingRuntime {
    async fn report_progress(&self, _progress: ResearchProgress) -> Result<(), String> {
        Ok(())
    }

    async fn report_event(&self, event: DeepResearchEvent) -> Result<(), String> {
        self.events.lock().expect("events lock").push(event);
        Ok(())
    }
}

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
    assert!(!editorial_prompt.contains("Nimbus version 2 receives fixes through September 2027."));
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
    let query = "Which Nimbus release is supported?";
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output(query),
        inquiry_collection_source_output(query),
    )
    .with_editorial(Ok(serde_json::json!({
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
    assert_eq!(report.markdown.matches(FINDING).count(), 1);
}

#[test]
fn editorial_failure_safely_publishes_the_admitted_claim_graph() {
    const FINDING: &str =
        "The official Nimbus record identifies version 2 and September 2027 as the support boundary.";
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
        .expect("editorial failure must preserve the admitted report");

    assert_eq!(
        run.publication,
        DeepResearchEvidenceFirstPublication::Synthesized
    );
    assert_eq!(
        run.output["research"]["metadata"]["synthesis_mode"],
        "model_claim_graph_editorial_fallback"
    );
    assert_eq!(
        run.output["research"]["warnings"]["editorial_error"],
        "scripted editorial timeout"
    );
    assert!(run.output["research"]["warnings"]["report_error"].is_null());
    let publications = runtime.publications.lock().expect("publication lock");
    let PublicationRequest::Synthesized { report, .. } =
        publications.last().expect("synthesized publication")
    else {
        panic!("the fallback must still publish a synthesized report");
    };
    assert!(report
        .markdown
        .contains("### Nimbus version 2 support boundary"));
    assert_eq!(report.markdown.matches(FINDING).count(), 1);
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

fn workflow_args(query: &str) -> Value {
    let current_date = "2026-07-23";
    serde_json::json!({
        "source": "standalone-test",
        "run_id": "engine-test",
        "input": {
            "query": query,
            "current_date": current_date,
            "evidence_scope": "web_and_workspace",
            "loop_contract": deep_research_loop_contract(
                query,
                current_date,
                "web_and_workspace",
                4,
            ),
        },
        "limits": {
            "timeoutMs": 600000,
            "maxToolCalls": 64,
            "maxOutputBytes": 1048576,
            "maxConcurrentGenerations": 2,
        }
    })
}

fn valid_outline() -> Value {
    serde_json::json!({
        "report_title": "Nimbus support research",
        "research_scope": "focused",
        "freshness_required": false,
        "workspace_evidence_required": false,
        "tracks": [{
            "id": "support.boundary",
            "title": "Support boundary",
            "focus": "Establish the supported Nimbus release and maintenance boundary.",
            "material": true,
            "completion_criteria": [
                "A traceable source identifies the release and support boundary."
            ],
            "questions": [{
                "question": "Which Nimbus release is supported, and what is its maintenance boundary?",
                "role": "establish",
                "completion_criterion_indexes": [0]
            }],
            "evidence_requirements": {
                "primary_source_required": true,
                "independent_corroboration_required": false
            }
        }],
        "supplemental_queries": []
    })
}

fn valid_report_proposal() -> Value {
    serde_json::json!({
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
            "id": "nimbus-answer",
            "dimension_id": "support.boundary",
            "placement": "direct_answer",
            "kind": "fact",
            "analysis_role": "conclusion",
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }, {
            "id": "nimbus-boundary",
            "dimension_id": "support.boundary",
            "placement": "finding",
            "kind": "fact",
            "analysis_role": "evidence",
            "text": "The official Nimbus record identifies version 2 and September 2027 as the support boundary.",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "support.boundary",
                "heading": "Nimbus version 2 support boundary",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["nimbus-boundary"]
                }]
            }]
        }
    })
}

fn valid_chinese_outline() -> Value {
    serde_json::json!({
        "report_title": "Nimbus 支持范围研究",
        "research_scope": "focused",
        "freshness_required": false,
        "workspace_evidence_required": false,
        "tracks": [{
            "id": "support.boundary",
            "title": "支持边界",
            "focus": "确认 Nimbus 当前受支持的版本及其维护边界。",
            "material": true,
            "completion_criteria": [
                "可追溯来源明确说明版本和支持期限。"
            ],
            "questions": [{
                "question": "Nimbus 当前受支持的版本及其维护边界是什么？",
                "role": "establish",
                "completion_criterion_indexes": [0]
            }],
            "evidence_requirements": {
                "primary_source_required": true,
                "independent_corroboration_required": false
            }
        }],
        "supplemental_queries": []
    })
}

fn valid_chinese_report_proposal() -> Value {
    serde_json::json!({
        "report_language": "zh",
        "labels": {
            "answer": "直接结论",
            "findings": "证据发现",
            "recommendations": "研究建议",
            "limitations": "证据边界",
            "evidence_boundary": "本报告不作超出已获取证据的结论。",
            "sources": "来源",
            "contradiction": "证据矛盾",
            "inference": "分析",
            "basis": "分析依据",
            "derivation": "推导方法"
        },
        "claims": [{
            "id": "nimbus-answer",
            "dimension_id": "support.boundary",
            "placement": "direct_answer",
            "kind": "fact",
            "analysis_role": "conclusion",
            "text": "Nimbus 版本 2 将持续获得修复，直至 2027 年 9 月。",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }, {
            "id": "nimbus-boundary",
            "dimension_id": "support.boundary",
            "placement": "finding",
            "kind": "fact",
            "analysis_role": "evidence",
            "text": "官方记录将版本 2 和 2027 年 9 月明确为当前支持边界。",
            "evidence_refs": [{
                "source_id": "source-1",
                "chunk_ids": ["source-1:chunk:1"]
            }],
            "basis_claim_ids": [],
            "derivation": null
        }],
        "relations": [],
        "gaps": [],
        "narrative": {
            "sections": [{
                "dimension_id": "support.boundary",
                "heading": "Nimbus 版本 2 的支持边界",
                "paragraphs": [{
                    "purpose": "evidence",
                    "claim_ids": ["nimbus-boundary"]
                }]
            }]
        }
    })
}

fn empty_bootstrap_output(query: &str) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": query,
            "mode": "bootstrap_acquisition",
            "acquisition": {
                "packet": {
                    "version": 1,
                    "sources": [],
                },
            },
            "execution": {
                "terminal_authority": "host_inquiry_reducer",
            },
        })
        .to_string(),
        metadata: None,
    }
}

fn empty_planned_output(query: &str) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": query,
            "mode": "inquiry_collection",
            "acquisition": {
                "packet": {
                    "version": 1,
                    "sources": [],
                },
                "metadata": {
                    "source_selection_mode": "semantic_candidate_ids",
                },
            },
        })
        .to_string(),
        metadata: None,
    }
}

fn malformed_packet_output(query: &str, mode: &str) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": query,
            "mode": mode,
            "acquisition": {
                "packet": {
                    "version": 2,
                    "sources": [{
                        "source_id": "source:nimbus",
                        "title": "Closed fixture record",
                        "url_or_path": "https://research.example/closed/record",
                        "chunks": [{
                            "chunk_id": "source:nimbus:chunk:1",
                            "text": "Closed fixture evidence.",
                        }],
                    }],
                },
            },
            "execution": {
                "terminal_authority": "host_inquiry_reducer",
            },
        })
        .to_string(),
        metadata: None,
    }
}

fn source_output(query: &str) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": query,
            "mode": "bootstrap_acquisition",
            "acquisition": {
                "packet": {
                    "version": 1,
                    "sources": [{
                        "source_id": "source:nimbus",
                        "title": "Official Nimbus support record",
                        "url_or_path": "https://docs.rs/nimbus/latest/nimbus/support",
                        "reliability": "fetched",
                        "chunks": [{
                            "chunk_id": "source:nimbus:chunk:1",
                            "text": "The official Nimbus record states that version 2 receives fixes through September 2027 and identifies that date as the support boundary.",
                        }],
                    }],
                },
                "metadata": {
                    "source_selection_mode": "semantic_candidate_ids",
                },
            },
            "execution": {
                "terminal_authority": "host_inquiry_reducer",
            },
        })
        .to_string(),
        metadata: None,
    }
}

fn inquiry_collection_source_output(query: &str) -> WorkflowOutput {
    WorkflowOutput {
        output: serde_json::json!({
            "query": query,
            "mode": "inquiry_collection",
            "research": {
                "status": "success",
                "metadata": {
                    "evidence_selection_mode": "semantic_chunk_ids_with_typed_coverage"
                },
                "results": [{
                    "task_id": "evidence_retrieval:source:nimbus",
                    "agent": "workflow",
                    "success": true,
                    "structured": {
                        "summary": "Semantic selection retained one fetched evidence chunk.",
                        "sources": [{
                            "source_id": "source:nimbus",
                            "title": "Nimbus support record",
                            "url_or_path": "https://research.example/nimbus/support",
                            "reliability": "fetched",
                            "evidence_excerpts": [{
                                "focus": "Establish the supported Nimbus release and maintenance boundary.",
                                "quote_or_fact": "The official Nimbus record states that version 2 receives fixes through September 2027 and identifies that date as the support boundary."
                            }]
                        }],
                        "source_coverage": [{
                            "source_id": "source:nimbus",
                            "obligation_id": "support.boundary",
                            "completion_criterion_indexes": [0],
                            "roles": ["supporting", "primary"]
                        }],
                        "source_relevance": [{
                            "source_id": "source:nimbus",
                            "obligation_id": "support.boundary"
                        }],
                        "relevant_obligation_ids": ["support.boundary"],
                        "key_evidence": [
                            "The official Nimbus record states that version 2 receives fixes through September 2027 and identifies that date as the support boundary."
                        ],
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
        .to_string(),
        metadata: None,
    }
}
