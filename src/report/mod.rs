mod convergence;
#[cfg(test)]
mod evidence_ledger;
mod host_digest;
mod host_evidence;
mod host_metadata;
pub mod report_generation;

use convergence::{
    inquiry_terminal_outcome, validated_inquiry_projection, validated_inquiry_publication_outcome,
    InquiryTerminalOutcome, ValidatedInquiryProjection,
};
#[cfg(test)]
use evidence_ledger::accepted_evidence_ledger;
use host_digest::*;
use host_evidence::*;
use host_metadata::deep_research_prompt_workflow_output;
use host_metadata::*;
use report_generation::{validate_report_obligation_coverage, GeneratedDeepResearchReport};
#[cfg(test)]
use report_generation::{
    ReportEditorialPlan, ReportPresentation, ReportTrackCoverage, ReportTrackStatus,
};

const RESEARCH_VIEW_MARKER: &str = "A3S_RESEARCH_VIEW:";
const DEEP_RESEARCH_PROMPT_SUCCESS_OUTPUT_LIMIT: usize = 1_200;
const DEEP_RESEARCH_PROMPT_TEXT_LIMIT: usize = 12_000;
const DEEP_RESEARCH_MAX_DIGEST_EVIDENCE: usize = 18;
const DEEP_RESEARCH_MAX_DIGEST_SOURCES: usize = 12;
const DEEP_RESEARCH_MAX_DIGEST_STRINGS: usize = 12;

/// Resolve the durable completed workflow snapshot used by report admission.
pub fn canonical_workflow_output(
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> String {
    deep_research_canonical_workflow_output(workflow_output, workflow_metadata)
}

fn deep_research_inquiry_publication_outcome(
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<Option<InquiryTerminalOutcome>, String> {
    let canonical = deep_research_canonical_workflow_output(workflow_output, workflow_metadata);
    let value = serde_json::from_str::<serde_json::Value>(&canonical)
        .map_err(|error| format!("decode DeepResearch workflow for publication: {error}"))?;
    validated_inquiry_publication_outcome(&value)
}

use std::collections::HashSet;
use std::path::{Path, PathBuf};

#[path = "artifacts/evidence_parser.rs"]
mod evidence_parser;
#[path = "artifacts/html.rs"]
mod html;
#[cfg(test)]
#[path = "artifacts/proposal_tests.rs"]
mod proposal_tests;
#[path = "artifacts/recovery.rs"]
mod recovery;
#[cfg(test)]
#[path = "artifacts/source_backed_tests.rs"]
mod source_backed_tests;
#[path = "artifacts/sources.rs"]
mod sources;
#[cfg(test)]
#[path = "artifacts/validation_tests.rs"]
mod validation_tests;

pub use evidence_parser::parse_embedded_structured_evidence_json;
use html::{
    deep_research_completed_report_html, deep_research_completed_report_html_with_presentation,
};
#[doc(hidden)]
pub fn deep_research_completed_report_html_for_test(query: &str, markdown: &str) -> String {
    deep_research_completed_report_html(query, markdown)
}
#[doc(hidden)]
pub fn deep_research_write_report_pair_for_test(
    markdown_path: &Path,
    markdown: impl AsRef<[u8]>,
    html_path: &Path,
    html: impl AsRef<[u8]>,
) -> Result<(), String> {
    write_research_report_pair(markdown_path, markdown, html_path, html)
}
use recovery::{looks_like_deep_research_recovery_report, recovery_research_report_artifacts};
use sources::{
    deep_research_workflow_evidence_omitted_count, deep_research_workflow_source_anchors,
    deep_research_workflow_source_omitted_count,
};
use std::io::Write;

// Keep the artifact pipeline in one module while splitting each concern into a
// reviewable source file without widening internal visibility.
include!("artifacts/publication.rs");
include!("artifacts/generated.rs");
include!("artifacts/resolution.rs");
include!("artifacts/quality.rs");
include!("artifacts/fallback.rs");
include!("artifacts/source_quality.rs");
include!("artifacts/source_backed.rs");
include!("artifacts/source_snapshot.rs");
include!("artifacts/proposal.rs");
include!("artifacts/report_scope.rs");
include!("artifacts/artifact_tests.rs");
