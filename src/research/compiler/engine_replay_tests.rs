use std::path::PathBuf;
use std::sync::Mutex;

use async_trait::async_trait;
use serde_json::Value;

use super::frozen_fixture::{load_frozen_replays, FrozenFault, FrozenReplay};
use crate::engine::{
    DeepResearchEngine, GenerationRequest, GenerationStage, ProgressPort, PublicationPort,
    PublicationRequest, ResearchProgress, StructuredGenerationPort, WorkflowExecutionPort,
    WorkflowOutput, WorkflowRequest, WorkflowStage,
};
use crate::report::{
    materialize_deep_research_admitted_report, materialize_deep_research_no_evidence_report,
    materialize_deep_research_source_backed_report, DeepResearchEvidenceFirstPublication,
    ResearchReportArtifacts,
};

mod projection;

use projection::{
    bootstrap_output, planned_output, planner_outline, report_proposal, workflow_args,
};

struct ActiveReplayRuntime {
    outline: Value,
    report: Mutex<Option<Result<Value, String>>>,
    bootstrap: WorkflowOutput,
    planned: WorkflowOutput,
    workspace: PathBuf,
    generation_requests: Mutex<Vec<GenerationRequest>>,
    workflow_requests: Mutex<Vec<WorkflowRequest>>,
    publications: Mutex<Vec<DeepResearchEvidenceFirstPublication>>,
    progress: Mutex<Vec<ResearchProgress>>,
}

impl ActiveReplayRuntime {
    fn new(replay: &FrozenReplay, workspace: PathBuf) -> Self {
        let report = match replay.fault.as_ref() {
            Some(FrozenFault::ReportGenerationTimeout) => {
                Err("scripted typed report-generation timeout".to_string())
            }
            _ => Ok(report_proposal(replay)),
        };
        Self {
            outline: planner_outline(replay),
            report: Mutex::new(Some(report)),
            bootstrap: bootstrap_output(replay),
            planned: planned_output(replay),
            workspace,
            generation_requests: Mutex::new(Vec::new()),
            workflow_requests: Mutex::new(Vec::new()),
            publications: Mutex::new(Vec::new()),
            progress: Mutex::new(Vec::new()),
        }
    }
}

#[async_trait]
impl StructuredGenerationPort for ActiveReplayRuntime {
    async fn generate_object(&self, request: GenerationRequest) -> Result<Value, String> {
        self.generation_requests
            .lock()
            .expect("generation requests lock")
            .push(request.clone());
        match request.stage {
            GenerationStage::Planning => Ok(self.outline.clone()),
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
impl WorkflowExecutionPort for ActiveReplayRuntime {
    async fn execute_workflow(&self, request: WorkflowRequest) -> Result<WorkflowOutput, String> {
        self.workflow_requests
            .lock()
            .expect("workflow requests lock")
            .push(request.clone());
        Ok(match request.stage {
            WorkflowStage::Bootstrap => self.bootstrap.clone(),
            WorkflowStage::PlannedRetrieval => self.planned.clone(),
        })
    }
}

#[async_trait]
impl PublicationPort for ActiveReplayRuntime {
    async fn publish(
        &self,
        request: PublicationRequest,
    ) -> Result<ResearchReportArtifacts, String> {
        let (publication, artifacts) = match request {
            PublicationRequest::SourceBacked {
                query,
                workflow_output,
                workflow_metadata,
                ..
            } => (
                DeepResearchEvidenceFirstPublication::SourceBacked,
                materialize_deep_research_source_backed_report(
                    &self.workspace,
                    &query,
                    &workflow_output,
                    workflow_metadata.as_ref(),
                )?
                .ok_or_else(|| {
                    "frozen active replay produced no source-backed artifact".to_string()
                })?,
            ),
            PublicationRequest::Synthesized {
                query,
                report,
                publication,
                ..
            } => (
                publication,
                materialize_deep_research_admitted_report(&self.workspace, &query, &report)?,
            ),
            PublicationRequest::NoEvidence { query, .. } => (
                DeepResearchEvidenceFirstPublication::NoEvidence,
                materialize_deep_research_no_evidence_report(&self.workspace, &query)?,
            ),
        };
        self.publications
            .lock()
            .expect("publications lock")
            .push(publication);
        Ok(artifacts)
    }
}

#[async_trait]
impl ProgressPort for ActiveReplayRuntime {
    async fn report_progress(&self, progress: ResearchProgress) -> Result<(), String> {
        self.progress.lock().expect("progress lock").push(progress);
        Ok(())
    }
}

#[test]
fn frozen_corpus_reaches_the_active_engine_with_exact_identity_control() {
    let mut outcomes = Vec::new();

    for replay in load_frozen_replays() {
        let workspace = tempfile::tempdir().expect("active replay workspace");
        let runtime = ActiveReplayRuntime::new(&replay, workspace.path().to_path_buf());
        let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
        let run = futures::executor::block_on(engine.execute(workflow_args(&replay)))
            .unwrap_or_else(|error| panic!("{}: active engine replay failed: {error}", replay.id));

        assert_eq!(
            run.output["query"], replay.contract.spec.query,
            "{}",
            replay.id
        );
        assert_ne!(
            run.publication,
            DeepResearchEvidenceFirstPublication::NoEvidence,
            "{}: every frozen case has at least one valid typed source edge",
            replay.id
        );
        assert!(run.artifacts.markdown.is_file(), "{}", replay.id);
        assert!(run.artifacts.html.is_file(), "{}", replay.id);

        let workflow_requests = runtime
            .workflow_requests
            .lock()
            .expect("workflow requests lock");
        assert_eq!(workflow_requests.len(), 2, "{}", replay.id);
        assert_eq!(
            workflow_requests
                .iter()
                .map(|request| request.stage)
                .collect::<Vec<_>>(),
            [WorkflowStage::Bootstrap, WorkflowStage::PlannedRetrieval],
            "{}",
            replay.id
        );
        assert!(workflow_requests.iter().all(|request| {
            request
                .arguments
                .pointer("/input/query")
                .and_then(Value::as_str)
                == Some(replay.contract.spec.query.as_str())
        }));
        drop(workflow_requests);

        let proposal = report_proposal(&replay).to_string();
        for forbidden in &replay.forbidden_statements {
            assert!(
                !proposal.contains(forbidden),
                "{}: forbidden statement reached the active report proposal",
                replay.id
            );
        }

        let publications = runtime.publications.lock().expect("publications lock");
        assert_eq!(
            publications.first(),
            Some(&DeepResearchEvidenceFirstPublication::SourceBacked),
            "{}: source evidence must be durable before optional synthesis",
            replay.id
        );
        assert_eq!(publications.last(), Some(&run.publication), "{}", replay.id);
        drop(publications);

        outcomes.push((replay.id, run.publication));
    }

    assert_eq!(
        outcomes,
        [
            (
                "F01".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
            (
                "F02".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
            (
                "F03".to_string(),
                DeepResearchEvidenceFirstPublication::Qualified,
            ),
            (
                "F04".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
            (
                "F05".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
            (
                "F06".to_string(),
                DeepResearchEvidenceFirstPublication::SourceBacked,
            ),
            (
                "F07".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
            (
                "F08".to_string(),
                DeepResearchEvidenceFirstPublication::Synthesized,
            ),
        ]
    );
}

#[test]
fn active_report_wire_contract_exposes_the_typed_claim_graph() {
    let replays = load_frozen_replays();
    assert!(replays
        .iter()
        .any(|replay| !replay.proposal.relations.is_empty()));
    assert!(replays.iter().any(|replay| replay
        .proposal
        .claims
        .iter()
        .any(|claim| claim.derivation.is_some())));
    assert!(replays.iter().any(|replay| replay
        .proposal
        .claims
        .iter()
        .any(|claim| !claim.basis_claim_ids.is_empty())));

    let workspace = tempfile::tempdir().expect("active replay workspace");
    let replay = replays.first().expect("frozen replay");
    let runtime = ActiveReplayRuntime::new(replay, workspace.path().to_path_buf());
    let engine = DeepResearchEngine::new(&runtime, &runtime, &runtime, &runtime);
    futures::executor::block_on(engine.execute(workflow_args(replay)))
        .expect("active engine replay");

    let requests = runtime
        .generation_requests
        .lock()
        .expect("generation requests lock");
    let schema = &requests
        .iter()
        .find(|request| request.stage == GenerationStage::Report)
        .expect("active report request")
        .arguments["schema"];
    let top_level = schema["properties"]
        .as_object()
        .expect("report schema properties");
    let claim = schema
        .pointer("/properties/claims/items/properties")
        .and_then(Value::as_object)
        .expect("typed claim properties");

    assert!(top_level.contains_key("claims"));
    assert!(top_level.contains_key("relations"));
    assert!(top_level.contains_key("gaps"));
    assert!(claim.contains_key("kind"));
    assert!(claim.contains_key("basis_claim_ids"));
    assert!(claim.contains_key("derivation"));
    assert!(claim.contains_key("evidence_refs"));
}
