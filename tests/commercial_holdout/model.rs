use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommercialHoldoutPlan {
    pub(crate) schema: String,
    pub(crate) campaign_id: String,
    pub(crate) challenge_nonce_sha256: String,
    pub(crate) subject: ReleaseSubject,
    pub(crate) issued_at: String,
    pub(crate) valid_until: String,
    pub(crate) transparency: TransparencyLogBinding,
    pub(crate) corpus: CorpusPlan,
    pub(crate) execution: ExecutionPlan,
    pub(crate) baseline: BaselinePlan,
    pub(crate) review: ReviewPlan,
    pub(crate) thresholds: HoldoutThresholds,
    pub(crate) custodian: AuthorityIdentity,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CommercialHoldoutResult {
    pub(crate) schema: String,
    pub(crate) campaign_id: String,
    pub(crate) plan_payload_sha256: String,
    pub(crate) subject: ReleaseSubject,
    pub(crate) issued_at: String,
    pub(crate) started_at: String,
    pub(crate) finished_at: String,
    pub(crate) execution_authority: AuthorityIdentity,
    pub(crate) review_authority: AuthorityIdentity,
    pub(crate) bindings: ResultBindings,
    pub(crate) execution: ExecutionResult,
    pub(crate) artifacts: ArtifactResult,
    pub(crate) review: ReviewResult,
    pub(crate) cases: Vec<CaseResult>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReleaseSubject {
    pub(crate) repository: String,
    pub(crate) git_commit: String,
    pub(crate) package_version: String,
    pub(crate) package_sha256: String,
    pub(crate) cargo_lock_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuthorityIdentity {
    pub(crate) authority: String,
    pub(crate) key_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransparencyLogBinding {
    pub(crate) log_id: String,
    pub(crate) entry_id: String,
    pub(crate) checkpoint_sha256: String,
    pub(crate) inclusion_proof_sha256: String,
    pub(crate) integrated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CorpusPlan {
    pub(crate) manifest_sha256: String,
    pub(crate) case_set_root_algorithm: String,
    pub(crate) case_set_root_sha256: String,
    pub(crate) sampling_policy_sha256: String,
    pub(crate) corpus_epoch: String,
    pub(crate) case_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutionPlan {
    pub(crate) profile_sha256: String,
    pub(crate) run_plan_sha256: String,
    pub(crate) attempt_set_root_algorithm: String,
    pub(crate) attempt_set_root_sha256: String,
    pub(crate) runner_image_sha256: String,
    pub(crate) verifier_image_sha256: String,
    pub(crate) attempts_per_case: usize,
    pub(crate) attempt_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BaselinePlan {
    pub(crate) subject_sha256: String,
    pub(crate) profile_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewPlan {
    pub(crate) rubric_sha256: String,
    pub(crate) reviewer_registry_sha256: String,
    pub(crate) assignment_root_sha256: String,
    pub(crate) blind_order_root_sha256: String,
    pub(crate) initial_reviewers_per_attempt: usize,
    pub(crate) reserve_reviewers_per_attempt: usize,
    pub(crate) adjudication_rule: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoldoutThresholds {
    pub(crate) minimum_cases: usize,
    pub(crate) attempts_per_case: usize,
    pub(crate) minimum_answerable_synthesized_attempt_rate_bps: u16,
    pub(crate) minimum_commercial_success_attempt_rate_bps: u16,
    pub(crate) case_success_minimum_attempts: usize,
    pub(crate) minimum_case_success_rate_bps: u16,
    pub(crate) minimum_case_success_lcb_bps: u16,
    pub(crate) confidence_level_bps: u16,
    pub(crate) reject_zero_success_case: bool,
    pub(crate) maximum_hard_violations: usize,
    pub(crate) minimum_citation_recall_bps: u16,
    pub(crate) minimum_citation_precision_bps: u16,
    pub(crate) minimum_rating_milli: u16,
    pub(crate) minimum_safe_abstention_rate_bps: u16,
    pub(crate) minimum_unanswerable_cases: usize,
    pub(crate) maximum_answerable_no_evidence_rate_bps: u16,
    pub(crate) maximum_infrastructure_failure_rate_bps: u16,
    pub(crate) maximum_baseline_case_loss_rate_bps: u16,
    pub(crate) baseline_noninferiority_margin_milli: i16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ResultBindings {
    pub(crate) corpus_manifest_sha256: String,
    pub(crate) case_set_root_sha256: String,
    pub(crate) execution_profile_sha256: String,
    pub(crate) run_plan_sha256: String,
    pub(crate) attempt_set_root_sha256: String,
    pub(crate) runner_image_sha256: String,
    pub(crate) verifier_image_sha256: String,
    pub(crate) baseline_subject_sha256: String,
    pub(crate) baseline_profile_sha256: String,
    pub(crate) rubric_sha256: String,
    pub(crate) review_assignment_root_sha256: String,
    pub(crate) blind_order_root_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ExecutionResult {
    pub(crate) attempt_log_root_algorithm: String,
    pub(crate) attempt_log_root_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ArtifactResult {
    pub(crate) root_algorithm: String,
    pub(crate) manifest_sha256: String,
    pub(crate) root_sha256: String,
    pub(crate) semantic_overlap_audit_sha256: String,
    pub(crate) corpus_retirement_receipt_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReviewResult {
    pub(crate) assignment_root_sha256: String,
    pub(crate) blind_order_reveal_sha256: String,
    pub(crate) ballot_root_sha256: String,
    pub(crate) review_completed_at: String,
    pub(crate) blind_order_revealed_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CaseResult {
    pub(crate) case_commitment_sha256: String,
    pub(crate) sealed_case_payload_sha256: String,
    pub(crate) reader_language: String,
    pub(crate) answerability: Answerability,
    pub(crate) strata: HoldoutStrata,
    pub(crate) material_dimension_count: usize,
    pub(crate) attempts: Vec<AttemptRecord>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptRecord {
    pub(crate) slot: AttemptSlot,
    pub(crate) execution: AttemptExecution,
    pub(crate) artifact: AttemptArtifact,
    pub(crate) assessment: AttemptAssessment,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptSlot {
    pub(crate) index: usize,
    pub(crate) nonce_sha256: String,
    pub(crate) commitment_sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptExecution {
    pub(crate) started_at: String,
    pub(crate) start_receipt_sha256: String,
    pub(crate) finished_at: String,
    pub(crate) terminal_receipt_sha256: String,
    pub(crate) terminal: AttemptTerminal,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptArtifact {
    pub(crate) subtree_sha256: String,
    pub(crate) file_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AttemptAssessment {
    pub(crate) publication: Option<PublicationOutcome>,
    pub(crate) output_language: Option<String>,
    pub(crate) deeply_closed: bool,
    pub(crate) claims: ClaimAudit,
    pub(crate) violations: ViolationAudit,
    pub(crate) review: BlindReview,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ClaimAudit {
    pub(crate) material_claim_count: usize,
    pub(crate) audited_material_claim_count: usize,
    pub(crate) cited_material_claim_count: usize,
    pub(crate) audited_claim_citation_pair_count: usize,
    pub(crate) entailed_claim_citation_pair_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ViolationAudit {
    pub(crate) unsupported_material_claim_count: usize,
    pub(crate) citation_integrity_violation_count: usize,
    pub(crate) reader_boundary_violation_count: usize,
    pub(crate) artifact_parity_violation_count: usize,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BlindReview {
    pub(crate) initial_ballot_receipt_sha256: [String; 2],
    pub(crate) adjudication_ballot_receipt_sha256: Option<String>,
    pub(crate) candidate: QualityRatings,
    pub(crate) baseline: QualityRatings,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct QualityRatings {
    pub(crate) depth_milli: u16,
    pub(crate) naturalness_milli: u16,
    pub(crate) evidence_use_milli: u16,
    pub(crate) decision_value_milli: u16,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HoldoutStrata {
    pub(crate) task_intent: String,
    pub(crate) evidence_condition: String,
    pub(crate) freshness: String,
    pub(crate) source_mix: String,
    pub(crate) language_group: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Answerability {
    Answerable,
    IntentionallyUnanswerable,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AttemptTerminal {
    Completed,
    InfrastructureFailure,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PublicationOutcome {
    Synthesized,
    Qualified,
    SourceBacked,
    NoEvidence,
}
