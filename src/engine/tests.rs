use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::channel::oneshot;

use super::*;
use crate::planner::deep_research_loop_contract;
use crate::report::{deep_research_report_slug, DeepResearchEvidenceFirstPublication};

struct FakeRuntime {
    planning: Mutex<Option<Result<Value, String>>>,
    report: Mutex<Option<Result<Value, String>>>,
    bootstrap: WorkflowOutput,
    planned: WorkflowOutput,
    workflow_requests: Mutex<Vec<WorkflowRequest>>,
    publications: Mutex<Vec<PublicationRequest>>,
    progress: Mutex<Vec<ResearchProgress>>,
    bootstrap_signal: Mutex<Option<oneshot::Sender<()>>>,
    planner_gate: Mutex<Option<oneshot::Receiver<()>>>,
}

impl FakeRuntime {
    fn new(
        planning: Result<Value, String>,
        report: Option<Result<Value, String>>,
        bootstrap: WorkflowOutput,
        planned: WorkflowOutput,
    ) -> Self {
        let (bootstrap_signal, planner_gate) = oneshot::channel();
        Self {
            planning: Mutex::new(Some(planning)),
            report: Mutex::new(report),
            bootstrap,
            planned,
            workflow_requests: Mutex::new(Vec::new()),
            publications: Mutex::new(Vec::new()),
            progress: Mutex::new(Vec::new()),
            bootstrap_signal: Mutex::new(Some(bootstrap_signal)),
            planner_gate: Mutex::new(Some(planner_gate)),
        }
    }
}

#[async_trait]
impl StructuredGenerationPort for FakeRuntime {
    async fn generate_object(&self, request: GenerationRequest) -> Result<Value, String> {
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
        self.publications
            .lock()
            .expect("publication lock")
            .push(request);
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
    drop(requests);
    assert!(matches!(
        runtime
            .publications
            .lock()
            .expect("publication lock")
            .as_slice(),
        [PublicationRequest::NoEvidence { query: published }] if published == query
    ));
}

#[test]
fn admitted_closed_evidence_report_replaces_the_staged_source_snapshot() {
    let query = "Which Nimbus release is supported?";
    let source_output = source_output(query);
    let runtime = FakeRuntime::new(
        Ok(valid_outline()),
        Some(Ok(valid_report_proposal())),
        source_output.clone(),
        source_output,
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
    assert!(matches!(
        publications.as_slice(),
        [
            PublicationRequest::SourceBacked { .. },
            PublicationRequest::Synthesized { .. }
        ]
    ));
    assert!(runtime.progress.lock().expect("progress lock").contains(
        &ResearchProgress::Completed(ResearchStage::FinalPublication)
    ));
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
        "summary": [{
            "text": "Nimbus version 2 receives fixes through September 2027.",
            "source_aliases": ["source-1"],
            "track_ids": ["support.boundary"]
        }],
        "findings": [{
            "text": "The official Nimbus record identifies version 2 and September 2027 as the support boundary.",
            "source_aliases": ["source-1"],
            "track_ids": ["support.boundary"]
        }],
        "recommendations": [],
        "limitations": []
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
                            "roles": {
                                "supporting": true,
                                "primary": true,
                                "independent": false
                            }
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
