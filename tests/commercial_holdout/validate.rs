use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

use super::crypto::{sha256_bytes, validate_sha256, verify_ed25519};
use super::model::{CommercialHoldoutPlan, CommercialHoldoutResult, ReleaseSubject};

#[path = "attempt.rs"]
mod attempt;
#[path = "coverage.rs"]
mod coverage;
#[path = "policy.rs"]
mod policy;

pub(crate) const HOLDOUT_MIN_CASES: usize = 48;
const HOLDOUT_RUNS_PER_CASE: usize = 3;
const MAX_PLAN_VALIDITY_DAYS: i64 = 31;

#[derive(Debug)]
pub(crate) struct ReleaseContext {
    pub(crate) repository: String,
    pub(crate) git_commit: String,
    pub(crate) package_version: String,
    pub(crate) package_sha256: String,
    pub(crate) cargo_lock_sha256: String,
    pub(crate) plan_payload_sha256: String,
    pub(crate) transparency_checkpoint_sha256: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct HoldoutSummary {
    pub(crate) schema: &'static str,
    pub(crate) campaign_id: String,
    pub(crate) case_count: usize,
    pub(crate) attempt_count: usize,
    pub(crate) answerable_synthesized_attempt_rate_bps: u16,
    pub(crate) commercial_success_attempt_rate_bps: u16,
    pub(crate) successful_case_rate_bps: u16,
    pub(crate) successful_case_lcb_bps: u16,
    pub(crate) safe_abstention_rate_bps: u16,
    pub(crate) answerable_no_evidence_rate_bps: u16,
    pub(crate) infrastructure_failure_rate_bps: u16,
    pub(crate) citation_recall_bps: u16,
    pub(crate) citation_precision_bps: u16,
    pub(crate) median_depth_milli: u16,
    pub(crate) median_naturalness_milli: u16,
    pub(crate) median_evidence_use_milli: u16,
    pub(crate) median_decision_value_milli: u16,
    pub(crate) baseline_loss_rate_bps: u16,
    pub(crate) baseline_delta_lcb_milli: i16,
}

pub(crate) struct SignedAttestations<'a> {
    pub(crate) plan_bytes: &'a [u8],
    pub(crate) plan_signature_hex: &'a str,
    pub(crate) plan_public_key_hex: &'a str,
    pub(crate) result_bytes: &'a [u8],
    pub(crate) execution_signature_hex: &'a str,
    pub(crate) execution_public_key_hex: &'a str,
    pub(crate) review_signature_hex: &'a str,
    pub(crate) review_public_key_hex: &'a str,
}

pub(crate) fn validate_signed_attestations(
    signed: SignedAttestations<'_>,
    context: &ReleaseContext,
    minimum_cases: usize,
) -> Result<HoldoutSummary, String> {
    let plan_key_id = verify_ed25519(
        signed.plan_bytes,
        signed.plan_signature_hex,
        signed.plan_public_key_hex,
        "corpus custodian",
    )?;
    let execution_key_id = verify_ed25519(
        signed.result_bytes,
        signed.execution_signature_hex,
        signed.execution_public_key_hex,
        "execution authority",
    )?;
    let review_key_id = verify_ed25519(
        signed.result_bytes,
        signed.review_signature_hex,
        signed.review_public_key_hex,
        "independent review authority",
    )?;
    if [
        plan_key_id.as_str(),
        execution_key_id.as_str(),
        review_key_id.as_str(),
    ]
    .into_iter()
    .collect::<std::collections::HashSet<_>>()
    .len()
        != 3
    {
        return Err("custodian, execution, and review trust roots must be distinct".to_string());
    }

    let plan = serde_json::from_slice::<CommercialHoldoutPlan>(signed.plan_bytes)
        .map_err(|error| format!("decode signed holdout plan: {error}"))?;
    let result = serde_json::from_slice::<CommercialHoldoutResult>(signed.result_bytes)
        .map_err(|error| format!("decode signed holdout result: {error}"))?;
    let plan_digest = sha256_bytes(signed.plan_bytes);
    if plan_digest != context.plan_payload_sha256 {
        return Err("signed plan does not match the externally precommitted digest".to_string());
    }

    validate_plan(&plan, context, &plan_key_id, minimum_cases)?;
    validate_result(
        &plan,
        &result,
        context,
        &plan_digest,
        &execution_key_id,
        &review_key_id,
    )?;
    policy::evaluate(&plan, &result)
}

fn validate_plan(
    plan: &CommercialHoldoutPlan,
    context: &ReleaseContext,
    plan_key_id: &str,
    minimum_cases: usize,
) -> Result<(), String> {
    if plan.schema != "a3s/deep-research-commercial-holdout-plan/v2" {
        return Err("unsupported signed holdout plan schema".to_string());
    }
    validate_identifier(&plan.campaign_id, "campaign_id")?;
    validate_sha256(&plan.challenge_nonce_sha256, "challenge_nonce_sha256")?;
    validate_subject(&plan.subject, context)?;
    validate_authority(
        &plan.custodian.authority,
        &plan.custodian.key_id,
        plan_key_id,
    )?;
    validate_plan_policy(plan, minimum_cases)?;

    for (value, field) in plan_digests(plan) {
        validate_sha256(value, field)?;
    }
    if plan.execution.attempts_per_case != HOLDOUT_RUNS_PER_CASE
        || plan.execution.attempt_count != plan.corpus.case_count * plan.execution.attempts_per_case
    {
        return Err("plan must preallocate exactly three attempts per case".to_string());
    }
    if plan.corpus.case_set_root_algorithm != "sha256-sorted-commitments-v1" {
        return Err("plan uses an unsupported sealed case-set root algorithm".to_string());
    }
    if plan.execution.attempt_set_root_algorithm != "sha256-sorted-attempt-slots-v1" {
        return Err("plan uses an unsupported preallocated attempt-set root algorithm".to_string());
    }
    if plan.review.initial_reviewers_per_attempt != 2
        || plan.review.reserve_reviewers_per_attempt != 1
        || plan.review.adjudication_rule != "score_delta_ge_2_or_preference_disagrees"
    {
        return Err("plan does not use the fixed blind adjudication protocol".to_string());
    }

    let issued = timestamp(&plan.issued_at, "plan issued_at")?;
    let integrated = timestamp(&plan.transparency.integrated_at, "plan integrated_at")?;
    let valid_until = timestamp(&plan.valid_until, "plan valid_until")?;
    if integrated < issued
        || valid_until <= integrated
        || valid_until - issued > Duration::days(MAX_PLAN_VALIDITY_DAYS)
        || integrated > Utc::now() + Duration::minutes(5)
        || Utc::now() > valid_until
    {
        return Err("plan transparency time or validity window is invalid".to_string());
    }
    validate_identifier(&plan.transparency.log_id, "transparency log_id")?;
    validate_identifier(&plan.transparency.entry_id, "transparency entry_id")?;
    if plan.transparency.checkpoint_sha256 != context.transparency_checkpoint_sha256 {
        return Err("plan is absent from the externally trusted log checkpoint".to_string());
    }
    Ok(())
}

fn validate_result(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
    context: &ReleaseContext,
    plan_digest: &str,
    execution_key_id: &str,
    review_key_id: &str,
) -> Result<(), String> {
    if result.schema != "a3s/deep-research-commercial-holdout-result/v2"
        || result.campaign_id != plan.campaign_id
        || result.plan_payload_sha256 != plan_digest
    {
        return Err("result does not bind the signed holdout plan".to_string());
    }
    validate_subject(&result.subject, context)?;
    validate_authority(
        &result.execution_authority.authority,
        &result.execution_authority.key_id,
        execution_key_id,
    )?;
    validate_authority(
        &result.review_authority.authority,
        &result.review_authority.key_id,
        review_key_id,
    )?;
    if plan.custodian.authority == result.execution_authority.authority
        || plan.custodian.authority == result.review_authority.authority
        || result.execution_authority.authority == result.review_authority.authority
    {
        return Err("holdout authorities must be organizationally distinct".to_string());
    }
    validate_bindings(plan, result)?;
    validate_result_envelope(plan, result)
}

fn validate_plan_policy(plan: &CommercialHoldoutPlan, minimum_cases: usize) -> Result<(), String> {
    let policy = &plan.thresholds;
    if plan.corpus.case_count < minimum_cases
        || plan.corpus.case_count > 1_024
        || policy.minimum_cases < minimum_cases
        || plan.corpus.case_count < policy.minimum_cases
        || policy.attempts_per_case != HOLDOUT_RUNS_PER_CASE
        || policy.minimum_answerable_synthesized_attempt_rate_bps < 9_000
        || policy.minimum_commercial_success_attempt_rate_bps < 9_000
        || policy.case_success_minimum_attempts < 2
        || policy.minimum_case_success_rate_bps < 9_500
        || policy.minimum_case_success_lcb_bps < 9_000
        || policy.confidence_level_bps != 9_500
        || !policy.reject_zero_success_case
        || policy.maximum_hard_violations != 0
        || policy.minimum_citation_recall_bps < 9_500
        || policy.minimum_citation_precision_bps < 9_500
        || policy.minimum_rating_milli < 4_000
        || policy.minimum_safe_abstention_rate_bps < 9_000
        || policy.minimum_unanswerable_cases < 6
        || policy.maximum_answerable_no_evidence_rate_bps > 500
        || policy.maximum_infrastructure_failure_rate_bps > 500
        || policy.maximum_baseline_case_loss_rate_bps > 2_000
        || policy.baseline_noninferiority_margin_milli < -250
    {
        return Err("signed plan weakens the commercial release policy floor".to_string());
    }
    Ok(())
}

fn validate_result_envelope(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
) -> Result<(), String> {
    if result.execution.attempt_log_root_algorithm != "sha256-sorted-terminal-receipts-v1"
        || result.artifacts.root_algorithm != "sha256-sorted-attempt-subtrees-v1"
    {
        return Err("result uses an unsupported attempt or artifact root algorithm".to_string());
    }
    if result.review.assignment_root_sha256 != plan.review.assignment_root_sha256 {
        return Err("blind review does not bind the signed reviewer assignment".to_string());
    }
    for (value, field) in result_digests(result) {
        validate_sha256(value, field)?;
    }
    let integrated = timestamp(&plan.transparency.integrated_at, "plan integrated_at")?;
    let started = timestamp(&result.started_at, "result started_at")?;
    let finished = timestamp(&result.finished_at, "result finished_at")?;
    let reviewed = timestamp(&result.review.review_completed_at, "review completed_at")?;
    let revealed = timestamp(
        &result.review.blind_order_revealed_at,
        "blind order revealed_at",
    )?;
    let issued = timestamp(&result.issued_at, "result issued_at")?;
    let valid_until = timestamp(&plan.valid_until, "plan valid_until")?;
    if !(integrated < started
        && started < finished
        && finished <= reviewed
        && reviewed <= revealed
        && revealed <= issued
        && issued <= valid_until
        && issued <= Utc::now() + Duration::minutes(5))
    {
        return Err(
            "plan, execution, review, reveal, and result times are out of order".to_string(),
        );
    }
    Ok(())
}

fn validate_bindings(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
) -> Result<(), String> {
    let binding = &result.bindings;
    let matches = binding.corpus_manifest_sha256 == plan.corpus.manifest_sha256
        && binding.case_set_root_sha256 == plan.corpus.case_set_root_sha256
        && binding.execution_profile_sha256 == plan.execution.profile_sha256
        && binding.run_plan_sha256 == plan.execution.run_plan_sha256
        && binding.attempt_set_root_sha256 == plan.execution.attempt_set_root_sha256
        && binding.runner_image_sha256 == plan.execution.runner_image_sha256
        && binding.verifier_image_sha256 == plan.execution.verifier_image_sha256
        && binding.baseline_subject_sha256 == plan.baseline.subject_sha256
        && binding.baseline_profile_sha256 == plan.baseline.profile_sha256
        && binding.rubric_sha256 == plan.review.rubric_sha256
        && binding.review_assignment_root_sha256 == plan.review.assignment_root_sha256
        && binding.blind_order_root_sha256 == plan.review.blind_order_root_sha256;
    if matches {
        Ok(())
    } else {
        Err("result bindings differ from the precommitted plan".to_string())
    }
}

fn validate_subject(subject: &ReleaseSubject, context: &ReleaseContext) -> Result<(), String> {
    for (value, field) in [
        (&subject.package_sha256, "subject package_sha256"),
        (&subject.cargo_lock_sha256, "subject cargo_lock_sha256"),
    ] {
        validate_sha256(value, field)?;
    }
    if subject.repository != context.repository
        || subject.git_commit != context.git_commit
        || subject.package_version != context.package_version
        || subject.package_sha256 != context.package_sha256
        || subject.cargo_lock_sha256 != context.cargo_lock_sha256
    {
        return Err("attestation subject differs from the exact release artifact".to_string());
    }
    if subject.git_commit.len() != 40
        || !subject
            .git_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("release git commit must be a complete object ID".to_string());
    }
    Ok(())
}

fn validate_authority(authority: &str, declared: &str, verified: &str) -> Result<(), String> {
    validate_identifier(authority, "authority")?;
    validate_sha256(declared, "authority key_id")?;
    if declared == verified {
        Ok(())
    } else {
        Err(format!(
            "{authority} key ID does not match its verified public key"
        ))
    }
}

fn plan_digests(plan: &CommercialHoldoutPlan) -> [(&str, &str); 16] {
    [
        (
            &plan.transparency.checkpoint_sha256,
            "transparency checkpoint",
        ),
        (
            &plan.transparency.inclusion_proof_sha256,
            "plan inclusion proof",
        ),
        (&plan.corpus.manifest_sha256, "corpus manifest"),
        (&plan.corpus.case_set_root_sha256, "case set root"),
        (&plan.corpus.sampling_policy_sha256, "sampling policy"),
        (&plan.execution.profile_sha256, "execution profile"),
        (&plan.execution.run_plan_sha256, "run plan"),
        (&plan.execution.attempt_set_root_sha256, "attempt set root"),
        (&plan.execution.runner_image_sha256, "runner image"),
        (&plan.execution.verifier_image_sha256, "verifier image"),
        (&plan.baseline.subject_sha256, "baseline subject"),
        (&plan.baseline.profile_sha256, "baseline profile"),
        (&plan.review.rubric_sha256, "review rubric"),
        (&plan.review.reviewer_registry_sha256, "reviewer registry"),
        (&plan.review.assignment_root_sha256, "review assignment"),
        (&plan.review.blind_order_root_sha256, "blind order root"),
    ]
}

fn result_digests(result: &CommercialHoldoutResult) -> [(&str, &str); 8] {
    [
        (
            &result.execution.attempt_log_root_sha256,
            "attempt log root",
        ),
        (&result.artifacts.manifest_sha256, "artifact manifest"),
        (&result.artifacts.root_sha256, "artifact root"),
        (
            &result.artifacts.semantic_overlap_audit_sha256,
            "semantic overlap audit",
        ),
        (
            &result.artifacts.corpus_retirement_receipt_sha256,
            "corpus retirement receipt",
        ),
        (
            &result.review.blind_order_reveal_sha256,
            "blind order reveal",
        ),
        (&result.review.ballot_root_sha256, "review ballot root"),
        (&result.bindings.blind_order_root_sha256, "blind order root"),
    ]
}

fn timestamp(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| format!("invalid {field}: {error}"))
}

fn validate_identifier(value: &str, field: &str) -> Result<(), String> {
    if !value.is_empty()
        && value.len() <= 128
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'/' | b'-')
        })
    {
        Ok(())
    } else {
        Err(format!("{field} is blank, oversized, or unsafe"))
    }
}
