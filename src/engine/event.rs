use serde::{Deserialize, Serialize};

use super::{ResearchProgress, ResearchStage};
use crate::report::{
    DeepResearchEvidenceFirstPublication, DeepResearchPublicationQuality, ResearchReportArtifacts,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchLifecycle {
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationOutcome {
    Synthesized,
    Qualified,
    SourceBacked,
    NoEvidence,
}

impl From<DeepResearchEvidenceFirstPublication> for PublicationOutcome {
    fn from(publication: DeepResearchEvidenceFirstPublication) -> Self {
        match publication {
            DeepResearchEvidenceFirstPublication::Synthesized => Self::Synthesized,
            DeepResearchEvidenceFirstPublication::Qualified => Self::Qualified,
            DeepResearchEvidenceFirstPublication::SourceBacked => Self::SourceBacked,
            DeepResearchEvidenceFirstPublication::NoEvidence => Self::NoEvidence,
        }
    }
}

impl From<PublicationOutcome> for DeepResearchEvidenceFirstPublication {
    fn from(publication: PublicationOutcome) -> Self {
        match publication {
            PublicationOutcome::Synthesized => Self::Synthesized,
            PublicationOutcome::Qualified => Self::Qualified,
            PublicationOutcome::SourceBacked => Self::SourceBacked,
            PublicationOutcome::NoEvidence => Self::NoEvidence,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeepResearchEvent {
    RunStarted {
        run_id: String,
        query: String,
    },
    StageStarted {
        run_id: String,
        stage: ResearchStage,
    },
    StageCompleted {
        run_id: String,
        stage: ResearchStage,
    },
    StageDegraded {
        run_id: String,
        stage: ResearchStage,
        reason: String,
    },
    PublicationCompleted {
        run_id: String,
        outcome: PublicationOutcome,
        quality: DeepResearchPublicationQuality,
        artifacts: ResearchReportArtifacts,
    },
    RunCompleted {
        run_id: String,
        outcome: PublicationOutcome,
    },
    RunCancelled {
        run_id: String,
    },
    RunFailed {
        run_id: String,
        message: String,
    },
}

impl DeepResearchEvent {
    pub(crate) fn from_progress(run_id: &str, progress: ResearchProgress) -> Self {
        match progress {
            ResearchProgress::Started(stage) => Self::StageStarted {
                run_id: run_id.to_string(),
                stage,
            },
            ResearchProgress::Completed(stage) => Self::StageCompleted {
                run_id: run_id.to_string(),
                stage,
            },
            ResearchProgress::Degraded { stage, reason } => Self::StageDegraded {
                run_id: run_id.to_string(),
                stage,
                reason,
            },
        }
    }

    pub(crate) fn legacy_progress(&self) -> Option<ResearchProgress> {
        match self {
            Self::StageStarted { stage, .. } => Some(ResearchProgress::Started(*stage)),
            Self::StageCompleted { stage, .. } => Some(ResearchProgress::Completed(*stage)),
            Self::StageDegraded { stage, reason, .. } => Some(ResearchProgress::Degraded {
                stage: *stage,
                reason: reason.clone(),
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::report::DeepResearchReportScope;

    #[test]
    fn terminal_event_wire_keeps_publication_separate_from_lifecycle() {
        let event = DeepResearchEvent::PublicationCompleted {
            run_id: "run-1".to_string(),
            outcome: PublicationOutcome::SourceBacked,
            quality: DeepResearchPublicationQuality {
                research_scope: DeepResearchReportScope::Focused,
                source_count: 1,
                relevant_source_count: 1,
                ..DeepResearchPublicationQuality::default()
            },
            artifacts: ResearchReportArtifacts {
                markdown: PathBuf::from(".a3s/research/artifacts/run-1/report.md"),
                html: PathBuf::from(".a3s/research/artifacts/run-1/index.html"),
            },
        };

        let wire = serde_json::to_value(&event).expect("serialize typed event");

        assert_eq!(wire["type"], "publication_completed");
        assert_eq!(wire["outcome"], "source_backed");
        assert!(wire.get("lifecycle").is_none());
        assert_eq!(
            serde_json::from_value::<DeepResearchEvent>(wire).expect("decode typed event"),
            event
        );
    }
}
