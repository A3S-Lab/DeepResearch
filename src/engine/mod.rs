//! Domain-agnostic orchestration over injected research runtime ports.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

use crate::report::{
    AdmittedDeepResearchReport, DeepResearchPublicationQuality, ResearchReportArtifacts,
};

pub const DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS: u64 = 300_000;
pub const DEFAULT_PLANNER_MAX_ATTEMPTS: u8 = 2;
pub const DEFAULT_BOOTSTRAP_STAGE_TIMEOUT_MS: u64 = 150_000;
// Planned retrieval owns the initial closed selection plus as many as four
// typed-gap rounds. Each round can require query generation, discovery,
// source admission, transport, and source-aware evidence shards. Bound the
// complete sequence as one durable stage without making its advertised loop
// cardinality unreachable under the per-generation active timeouts.
pub const DEFAULT_PLANNED_RETRIEVAL_STAGE_TIMEOUT_MS: u64 = 3_600_000;
const REPORT_BASE_ATTEMPT_TIMEOUT_MS: u64 = 240_000;
const REPORT_PAYLOAD_BASE_BYTES: usize = 32 * 1024;
const REPORT_PAYLOAD_STEP_BYTES: usize = 32 * 1024;
const REPORT_PAYLOAD_STEP_TIMEOUT_MS: u64 = 90_000;
/// Maximum active time for one report or editorial generation. The engine
/// selects a smaller timeout from the actual closed prompt and schema size.
pub const DEFAULT_REPORT_ATTEMPT_TIMEOUT_MS: u64 = 420_000;
pub const DEFAULT_REPORT_MAX_ATTEMPTS: u8 = 2;
pub const DEFAULT_DURABLE_GENERATION_GRACE_MS: u64 = 15_000;
pub const DEFAULT_MAX_CONCURRENT_GENERATIONS: u8 = 4;
pub const DEFAULT_REPORT_STAGE_TIMEOUT_MS: u64 = DEFAULT_REPORT_ATTEMPT_TIMEOUT_MS
    * DEFAULT_REPORT_MAX_ATTEMPTS as u64
    + DEFAULT_DURABLE_GENERATION_GRACE_MS;
pub const DEFAULT_PLANNER_STAGE_TIMEOUT_MS: u64 = DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS
    * DEFAULT_PLANNER_MAX_ATTEMPTS as u64
    + DEFAULT_DURABLE_GENERATION_GRACE_MS;
pub const DEFAULT_PLANNING_BOOTSTRAP_STAGE_TIMEOUT_MS: u64 =
    if DEFAULT_PLANNER_STAGE_TIMEOUT_MS > DEFAULT_BOOTSTRAP_STAGE_TIMEOUT_MS {
        DEFAULT_PLANNER_STAGE_TIMEOUT_MS
    } else {
        DEFAULT_BOOTSTRAP_STAGE_TIMEOUT_MS
    };

mod cancellation;
mod contract;
mod event;
mod execution;

pub use cancellation::DeepResearchCancellation;
pub use contract::{
    DeepResearchRequest, DeepResearchRequestLimits, EvidenceScope, WorkspaceSourceHint,
    MAX_DEEP_RESEARCH_TRACKS,
};
pub use event::{DeepResearchEvent, DeepResearchLifecycle, PublicationOutcome};

/// Default execution limits for one progressively publishable research run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EngineLimits {
    pub planner_attempt_timeout_ms: u64,
    pub planner_max_attempts: u8,
    pub bootstrap_stage_timeout_ms: u64,
    pub planned_retrieval_stage_timeout_ms: u64,
    pub report_attempt_timeout_ms: u64,
    pub report_stage_timeout_ms: u64,
    pub report_max_attempts: u8,
    pub durable_generation_grace_ms: u64,
}

impl Default for EngineLimits {
    fn default() -> Self {
        Self {
            planner_attempt_timeout_ms: DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS,
            planner_max_attempts: DEFAULT_PLANNER_MAX_ATTEMPTS,
            bootstrap_stage_timeout_ms: DEFAULT_BOOTSTRAP_STAGE_TIMEOUT_MS,
            planned_retrieval_stage_timeout_ms: DEFAULT_PLANNED_RETRIEVAL_STAGE_TIMEOUT_MS,
            report_attempt_timeout_ms: DEFAULT_REPORT_ATTEMPT_TIMEOUT_MS,
            report_stage_timeout_ms: DEFAULT_REPORT_STAGE_TIMEOUT_MS,
            report_max_attempts: DEFAULT_REPORT_MAX_ATTEMPTS,
            durable_generation_grace_ms: DEFAULT_DURABLE_GENERATION_GRACE_MS,
        }
    }
}

impl EngineLimits {
    fn validate(self) -> Result<Self, DeepResearchEngineError> {
        if self.planner_attempt_timeout_ms < 1_000
            || self.bootstrap_stage_timeout_ms < 1_000
            || self.planned_retrieval_stage_timeout_ms < 1_000
            || self.report_attempt_timeout_ms < 1_000
            || self.report_stage_timeout_ms < 1_000
            || !(1..=2).contains(&self.planner_max_attempts)
            || !(1..=2).contains(&self.report_max_attempts)
        {
            return Err(DeepResearchEngineError::Contract(
                "engine limits require stage timeouts of at least 1000 ms and one or two generation attempts"
                    .to_string(),
            ));
        }
        Ok(self)
    }

    fn planner_stage_timeout_ms(self, planner_timeout_ms: u64) -> u64 {
        planner_timeout_ms
            .saturating_mul(u64::from(self.planner_max_attempts))
            .saturating_add(self.durable_generation_grace_ms)
    }

    fn report_attempt_timeout_for_payload(self, payload_bytes: usize) -> u64 {
        let excess_bytes = payload_bytes.saturating_sub(REPORT_PAYLOAD_BASE_BYTES);
        let payload_steps = excess_bytes.div_ceil(REPORT_PAYLOAD_STEP_BYTES);
        let payload_steps = u64::try_from(payload_steps).unwrap_or(u64::MAX);
        REPORT_BASE_ATTEMPT_TIMEOUT_MS
            .saturating_add(payload_steps.saturating_mul(REPORT_PAYLOAD_STEP_TIMEOUT_MS))
            .min(self.report_attempt_timeout_ms)
    }

    fn report_stage_timeout_for_attempt(self, attempt_timeout_ms: u64) -> u64 {
        attempt_timeout_ms
            .saturating_mul(u64::from(self.report_max_attempts))
            .saturating_add(self.durable_generation_grace_ms)
            .min(self.report_stage_timeout_ms)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenerationStage {
    Planning,
    Report,
    Editorial,
}

impl GenerationStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Planning => "planner-outline",
            Self::Report => "report-proposal",
            Self::Editorial => "report-editorial",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkflowStage {
    Bootstrap,
    PlannedRetrieval,
}

impl WorkflowStage {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bootstrap => "bootstrap acquisition",
            Self::PlannedRetrieval => "semantic-plan retrieval",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchStage {
    Planning,
    BootstrapRetrieval,
    PlannedRetrieval,
    SourcePublication,
    ReportGeneration,
    FinalPublication,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResearchProgress {
    Started(ResearchStage),
    Completed(ResearchStage),
    Degraded {
        stage: ResearchStage,
        reason: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct GenerationRequest {
    pub stage: GenerationStage,
    pub arguments: Value,
    pub execution_timeout_ms: u64,
    pub max_attempts: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowRequest {
    pub stage: WorkflowStage,
    pub arguments: Value,
    pub timeout_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowOutput {
    pub output: String,
    pub metadata: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PublicationRequest {
    SourceBacked {
        run_id: String,
        query: String,
        output_language: String,
        workflow_output: String,
        workflow_metadata: Option<Value>,
        quality: DeepResearchPublicationQuality,
    },
    Synthesized {
        run_id: String,
        query: String,
        output_language: String,
        report: AdmittedDeepResearchReport,
        publication: crate::report::DeepResearchEvidenceFirstPublication,
        quality: DeepResearchPublicationQuality,
    },
    NoEvidence {
        run_id: String,
        query: String,
        output_language: String,
        quality: DeepResearchPublicationQuality,
    },
}

#[async_trait]
pub trait StructuredGenerationPort: Send + Sync {
    async fn generate_object(&self, request: GenerationRequest) -> Result<Value, String>;
}

#[async_trait]
pub trait WorkflowExecutionPort: Send + Sync {
    async fn execute_workflow(&self, request: WorkflowRequest) -> Result<WorkflowOutput, String>;
}

#[async_trait]
pub trait PublicationPort: Send + Sync {
    async fn publish(&self, request: PublicationRequest)
        -> Result<ResearchReportArtifacts, String>;
}

#[async_trait]
pub trait ProgressPort: Send + Sync {
    async fn report_progress(&self, progress: ResearchProgress) -> Result<(), String>;

    async fn report_event(&self, event: DeepResearchEvent) -> Result<(), String> {
        match event.legacy_progress() {
            Some(progress) => self.report_progress(progress).await,
            None => Ok(()),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NoopProgress;

#[async_trait]
impl ProgressPort for NoopProgress {
    async fn report_progress(&self, _progress: ResearchProgress) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Debug, Error, Eq, PartialEq)]
pub enum DeepResearchEngineError {
    #[error("invalid DeepResearch contract: {0}")]
    Contract(String),
    #[error("DeepResearch {stage} failed: {message}")]
    Stage {
        stage: &'static str,
        message: String,
    },
    #[error("DeepResearch publication failed: {0}")]
    Publication(String),
    #[error("DeepResearch progress reporting failed: {0}")]
    Progress(String),
    #[error("DeepResearch run was cancelled")]
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepResearchRun {
    pub output: Value,
    pub artifacts: ResearchReportArtifacts,
    pub publication: crate::report::DeepResearchEvidenceFirstPublication,
    pub quality: DeepResearchPublicationQuality,
}

impl DeepResearchRun {
    pub fn output_json(&self) -> String {
        self.output.to_string()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeepResearchResult {
    pub run_id: String,
    pub query: String,
    pub lifecycle: DeepResearchLifecycle,
    pub publication: PublicationOutcome,
    pub quality: DeepResearchPublicationQuality,
    pub artifacts: ResearchReportArtifacts,
    pub output: Value,
}

impl DeepResearchResult {
    pub fn output_json(&self) -> String {
        self.output.to_string()
    }
}

/// Owns the research state transitions while delegating side effects to ports.
pub struct DeepResearchEngine<'a> {
    generation: &'a dyn StructuredGenerationPort,
    workflow: &'a dyn WorkflowExecutionPort,
    publication: &'a dyn PublicationPort,
    progress: &'a dyn ProgressPort,
    limits: EngineLimits,
}

impl<'a> DeepResearchEngine<'a> {
    pub fn new(
        generation: &'a dyn StructuredGenerationPort,
        workflow: &'a dyn WorkflowExecutionPort,
        publication: &'a dyn PublicationPort,
        progress: &'a dyn ProgressPort,
    ) -> Self {
        Self {
            generation,
            workflow,
            publication,
            progress,
            limits: EngineLimits::default(),
        }
    }

    pub fn with_limits(mut self, limits: EngineLimits) -> Self {
        self.limits = limits;
        self
    }

    async fn event(&self, event: DeepResearchEvent) -> Result<(), DeepResearchEngineError> {
        self.progress
            .report_event(event)
            .await
            .map_err(DeepResearchEngineError::Progress)
    }

    async fn progress(
        &self,
        run_id: &str,
        progress: ResearchProgress,
    ) -> Result<(), DeepResearchEngineError> {
        self.event(DeepResearchEvent::from_progress(run_id, progress))
            .await
    }
}

#[cfg(test)]
mod tests;
