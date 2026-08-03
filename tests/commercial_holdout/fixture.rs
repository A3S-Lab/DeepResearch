use chrono::{Duration, SecondsFormat, Utc};
use ed25519_dalek::{Signer, SigningKey};

use super::model::{
    Answerability, ArtifactResult, AttemptArtifact, AttemptAssessment, AttemptExecution,
    AttemptRecord, AttemptSlot, AttemptTerminal, AuthorityIdentity, BaselinePlan, BlindReview,
    CaseResult, ClaimAudit, CommercialHoldoutPlan, CommercialHoldoutResult, CorpusPlan,
    ExecutionPlan, ExecutionResult, HoldoutStrata, HoldoutThresholds, PublicationOutcome,
    QualityRatings, ReleaseSubject, ResultBindings, ReviewPlan, ReviewResult,
    TransparencyLogBinding, ViolationAudit,
};
use super::validate::{ReleaseContext, HOLDOUT_MIN_CASES};
use crate::crypto::{
    attempt_slot_commitment, attempt_start_receipt, attempt_terminal_receipt,
    case_descriptor_commitment, sha256_bytes, sorted_artifact_subtree_root,
    sorted_attempt_commitment_root, sorted_attempt_log_root, sorted_ballot_receipt_root,
    sorted_commitment_root,
};

const PLAN_KEY_BYTES: [u8; 32] = [1; 32];
const EXECUTION_KEY_BYTES: [u8; 32] = [2; 32];
const REVIEW_KEY_BYTES: [u8; 32] = [3; 32];

pub(crate) struct TestPacket {
    pub(crate) plan: CommercialHoldoutPlan,
    pub(crate) result: CommercialHoldoutResult,
    pub(crate) plan_signature_hex: String,
    pub(crate) plan_public_key_hex: String,
    pub(crate) execution_signature_hex: String,
    pub(crate) execution_public_key_hex: String,
    pub(crate) review_signature_hex: String,
    pub(crate) review_public_key_hex: String,
    pub(crate) context: ReleaseContext,
}

impl TestPacket {
    pub(crate) fn resign(&mut self) {
        let plan_key = SigningKey::from_bytes(&PLAN_KEY_BYTES);
        let execution_key = SigningKey::from_bytes(&EXECUTION_KEY_BYTES);
        let review_key = SigningKey::from_bytes(&REVIEW_KEY_BYTES);
        let plan_bytes = serde_json::to_vec(&self.plan).expect("encode test holdout plan");
        self.context.plan_payload_sha256 = sha256_bytes(&plan_bytes);
        self.result.plan_payload_sha256 = self.context.plan_payload_sha256.clone();
        let result_bytes = serde_json::to_vec(&self.result).expect("encode test holdout result");
        self.plan_signature_hex = hex(&plan_key.sign(&plan_bytes).to_bytes());
        self.execution_signature_hex = hex(&execution_key.sign(&result_bytes).to_bytes());
        self.review_signature_hex = hex(&review_key.sign(&result_bytes).to_bytes());
    }
}

pub(crate) fn signed_test_packet() -> TestPacket {
    let now = Utc::now();
    let result_started_at = now - Duration::hours(2);
    let result_finished_at = now - Duration::minutes(90);
    let plan_key = SigningKey::from_bytes(&PLAN_KEY_BYTES);
    let execution_key = SigningKey::from_bytes(&EXECUTION_KEY_BYTES);
    let review_key = SigningKey::from_bytes(&REVIEW_KEY_BYTES);
    let subject = ReleaseSubject {
        repository: "A3S-Lab/DeepResearch".to_string(),
        git_commit: "a".repeat(40),
        package_version: "0.2.0".to_string(),
        package_sha256: "b".repeat(64),
        cargo_lock_sha256: "c".repeat(64),
    };
    let cases = (0..HOLDOUT_MIN_CASES)
        .map(|index| test_case(index, result_started_at))
        .collect::<Vec<_>>();
    let case_set_root_sha256 = sorted_commitment_root(
        cases
            .iter()
            .map(|case| case.case_commitment_sha256.as_str()),
    );
    let attempt_set_root_sha256 = sorted_attempt_commitment_root(cases.iter().flat_map(|case| {
        case.attempts
            .iter()
            .map(|attempt| attempt.slot.commitment_sha256.as_str())
    }));
    let plan = CommercialHoldoutPlan {
        schema: "a3s/deep-research-commercial-holdout-plan/v2".to_string(),
        campaign_id: "campaign-test-v2".to_string(),
        challenge_nonce_sha256: "d".repeat(64),
        subject: subject.clone(),
        issued_at: time(now - Duration::hours(4)),
        valid_until: time(now + Duration::days(10)),
        transparency: TransparencyLogBinding {
            log_id: "external-log-v1".to_string(),
            entry_id: "entry-test-v2".to_string(),
            checkpoint_sha256: "0".repeat(64),
            inclusion_proof_sha256: "e".repeat(64),
            integrated_at: time(now - Duration::hours(3)),
        },
        corpus: CorpusPlan {
            manifest_sha256: "1".repeat(64),
            case_set_root_algorithm: "sha256-sorted-commitments-v1".to_string(),
            case_set_root_sha256,
            sampling_policy_sha256: "3".repeat(64),
            corpus_epoch: "sealed-test-epoch".to_string(),
            case_count: HOLDOUT_MIN_CASES,
        },
        execution: ExecutionPlan {
            profile_sha256: "4".repeat(64),
            run_plan_sha256: "5".repeat(64),
            attempt_set_root_algorithm: "sha256-sorted-attempt-slots-v1".to_string(),
            attempt_set_root_sha256,
            runner_image_sha256: "7".repeat(64),
            verifier_image_sha256: "8".repeat(64),
            attempts_per_case: 3,
            attempt_count: HOLDOUT_MIN_CASES * 3,
        },
        baseline: BaselinePlan {
            subject_sha256: "9".repeat(64),
            profile_sha256: "a".repeat(64),
        },
        review: ReviewPlan {
            rubric_sha256: "b".repeat(64),
            reviewer_registry_sha256: "c".repeat(64),
            assignment_root_sha256: "d".repeat(64),
            blind_order_root_sha256: "e".repeat(64),
            initial_reviewers_per_attempt: 2,
            reserve_reviewers_per_attempt: 1,
            adjudication_rule: "score_delta_ge_2_or_preference_disagrees".to_string(),
        },
        thresholds: HoldoutThresholds {
            minimum_cases: HOLDOUT_MIN_CASES,
            attempts_per_case: 3,
            minimum_answerable_synthesized_attempt_rate_bps: 9_000,
            minimum_commercial_success_attempt_rate_bps: 9_000,
            case_success_minimum_attempts: 2,
            minimum_case_success_rate_bps: 9_500,
            minimum_case_success_lcb_bps: 9_000,
            confidence_level_bps: 9_500,
            reject_zero_success_case: true,
            maximum_hard_violations: 0,
            minimum_citation_recall_bps: 9_500,
            minimum_citation_precision_bps: 9_500,
            minimum_rating_milli: 4_000,
            minimum_safe_abstention_rate_bps: 9_000,
            minimum_unanswerable_cases: 6,
            maximum_answerable_no_evidence_rate_bps: 500,
            maximum_infrastructure_failure_rate_bps: 500,
            maximum_baseline_case_loss_rate_bps: 2_000,
            baseline_noninferiority_margin_milli: -250,
        },
        custodian: AuthorityIdentity {
            authority: "external-corpus-custodian".to_string(),
            key_id: sha256_bytes(plan_key.verifying_key().as_bytes()),
        },
    };
    let result = CommercialHoldoutResult {
        schema: "a3s/deep-research-commercial-holdout-result/v2".to_string(),
        campaign_id: plan.campaign_id.clone(),
        plan_payload_sha256: String::new(),
        subject: subject.clone(),
        issued_at: time(now - Duration::minutes(30)),
        started_at: time(result_started_at),
        finished_at: time(result_finished_at),
        execution_authority: AuthorityIdentity {
            authority: "external-execution-authority".to_string(),
            key_id: sha256_bytes(execution_key.verifying_key().as_bytes()),
        },
        review_authority: AuthorityIdentity {
            authority: "independent-review-authority".to_string(),
            key_id: sha256_bytes(review_key.verifying_key().as_bytes()),
        },
        bindings: ResultBindings {
            corpus_manifest_sha256: plan.corpus.manifest_sha256.clone(),
            case_set_root_sha256: plan.corpus.case_set_root_sha256.clone(),
            execution_profile_sha256: plan.execution.profile_sha256.clone(),
            run_plan_sha256: plan.execution.run_plan_sha256.clone(),
            attempt_set_root_sha256: plan.execution.attempt_set_root_sha256.clone(),
            runner_image_sha256: plan.execution.runner_image_sha256.clone(),
            verifier_image_sha256: plan.execution.verifier_image_sha256.clone(),
            baseline_subject_sha256: plan.baseline.subject_sha256.clone(),
            baseline_profile_sha256: plan.baseline.profile_sha256.clone(),
            rubric_sha256: plan.review.rubric_sha256.clone(),
            review_assignment_root_sha256: plan.review.assignment_root_sha256.clone(),
            blind_order_root_sha256: plan.review.blind_order_root_sha256.clone(),
        },
        execution: ExecutionResult {
            attempt_log_root_algorithm: "sha256-sorted-terminal-receipts-v1".to_string(),
            attempt_log_root_sha256: sorted_attempt_log_root(cases.iter().flat_map(|case| {
                case.attempts
                    .iter()
                    .map(|attempt| attempt.execution.terminal_receipt_sha256.as_str())
            })),
        },
        artifacts: ArtifactResult {
            root_algorithm: "sha256-sorted-attempt-subtrees-v1".to_string(),
            manifest_sha256: "1".repeat(64),
            root_sha256: sorted_artifact_subtree_root(cases.iter().flat_map(|case| {
                case.attempts
                    .iter()
                    .map(|attempt| attempt.artifact.subtree_sha256.as_str())
            })),
            semantic_overlap_audit_sha256: "5".repeat(64),
            corpus_retirement_receipt_sha256: "6".repeat(64),
        },
        review: ReviewResult {
            assignment_root_sha256: plan.review.assignment_root_sha256.clone(),
            blind_order_reveal_sha256: "3".repeat(64),
            ballot_root_sha256: sorted_ballot_receipt_root(cases.iter().flat_map(|case| {
                case.attempts.iter().flat_map(|attempt| {
                    attempt
                        .assessment
                        .review
                        .initial_ballot_receipt_sha256
                        .iter()
                        .map(String::as_str)
                        .chain(
                            attempt
                                .assessment
                                .review
                                .adjudication_ballot_receipt_sha256
                                .iter()
                                .map(String::as_str),
                        )
                })
            })),
            review_completed_at: time(now - Duration::hours(1)),
            blind_order_revealed_at: time(now - Duration::minutes(45)),
        },
        cases,
    };
    let context = ReleaseContext {
        repository: subject.repository.clone(),
        git_commit: subject.git_commit.clone(),
        package_version: subject.package_version.clone(),
        package_sha256: subject.package_sha256.clone(),
        cargo_lock_sha256: subject.cargo_lock_sha256.clone(),
        plan_payload_sha256: String::new(),
        transparency_checkpoint_sha256: plan.transparency.checkpoint_sha256.clone(),
    };
    let mut packet = TestPacket {
        plan,
        result,
        plan_signature_hex: String::new(),
        plan_public_key_hex: hex(plan_key.verifying_key().as_bytes()),
        execution_signature_hex: String::new(),
        execution_public_key_hex: hex(execution_key.verifying_key().as_bytes()),
        review_signature_hex: String::new(),
        review_public_key_hex: hex(review_key.verifying_key().as_bytes()),
        context,
    };
    packet.resign();
    packet
}

fn test_case(index: usize, campaign_started_at: chrono::DateTime<Utc>) -> CaseResult {
    let unanswerable = index < 6;
    let reader_language = ["en", "zh-CN", "fr"][index % 3].to_string();
    let answerability = if unanswerable {
        Answerability::IntentionallyUnanswerable
    } else {
        Answerability::Answerable
    };
    let strata = HoldoutStrata {
        task_intent: ["decision", "comparison", "explanation", "forecast"][index % 4].to_string(),
        evidence_condition: ["consistent", "conflicting", "sparse"][index % 3].to_string(),
        freshness: ["stable", "current"][index % 2].to_string(),
        source_mix: ["primary", "multi_source", "mixed"][index % 3].to_string(),
        language_group: ["group_1", "group_2", "group_3"][index % 3].to_string(),
    };
    let sealed_case_payload_sha256 = sha256_bytes(format!("sealed-case-{index}").as_bytes());
    let answerability_tag = if unanswerable {
        "intentionally_unanswerable"
    } else {
        "answerable"
    };
    let case_commitment_sha256 = case_descriptor_commitment(
        &sealed_case_payload_sha256,
        &[
            &reader_language,
            answerability_tag,
            &strata.task_intent,
            &strata.evidence_condition,
            &strata.freshness,
            &strata.source_mix,
            &strata.language_group,
        ],
        3,
    );
    let attempts = (0..3)
        .map(|slot| {
            test_attempt(
                index,
                slot,
                &case_commitment_sha256,
                &reader_language,
                answerability,
                campaign_started_at,
            )
        })
        .collect();
    CaseResult {
        case_commitment_sha256,
        sealed_case_payload_sha256,
        reader_language,
        answerability,
        strata,
        material_dimension_count: 3,
        attempts,
    }
}

fn test_attempt(
    case_index: usize,
    slot_index: usize,
    case_commitment_sha256: &str,
    reader_language: &str,
    answerability: Answerability,
    campaign_started_at: chrono::DateTime<Utc>,
) -> AttemptRecord {
    let nonce_sha256 = sha256_bytes(format!("slot-nonce-{case_index}-{slot_index}").as_bytes());
    let commitment_sha256 =
        attempt_slot_commitment(case_commitment_sha256, slot_index, &nonce_sha256);
    let started_at = time(campaign_started_at + Duration::minutes(1 + slot_index as i64));
    let finished_at = time(campaign_started_at + Duration::minutes(10 + slot_index as i64));
    let start_receipt_sha256 = attempt_start_receipt(&commitment_sha256, &started_at);
    let subtree_sha256 =
        sha256_bytes(format!("artifact-subtree-{case_index}-{slot_index}").as_bytes());
    let terminal = AttemptTerminal::Completed;
    let terminal_receipt_sha256 = attempt_terminal_receipt(
        &start_receipt_sha256,
        "completed",
        &finished_at,
        &subtree_sha256,
    );
    let unanswerable = answerability == Answerability::IntentionallyUnanswerable;
    let claim_count = usize::from(!unanswerable) * 10;
    AttemptRecord {
        slot: AttemptSlot {
            index: slot_index,
            nonce_sha256,
            commitment_sha256,
        },
        execution: AttemptExecution {
            started_at,
            start_receipt_sha256,
            finished_at,
            terminal_receipt_sha256,
            terminal,
        },
        artifact: AttemptArtifact {
            subtree_sha256,
            file_count: 6,
        },
        assessment: AttemptAssessment {
            publication: Some(if unanswerable {
                PublicationOutcome::NoEvidence
            } else {
                PublicationOutcome::Synthesized
            }),
            output_language: Some(reader_language.to_string()),
            deeply_closed: !unanswerable,
            claims: ClaimAudit {
                material_claim_count: claim_count,
                audited_material_claim_count: claim_count,
                cited_material_claim_count: claim_count,
                audited_claim_citation_pair_count: claim_count,
                entailed_claim_citation_pair_count: claim_count,
            },
            violations: ViolationAudit {
                unsupported_material_claim_count: 0,
                citation_integrity_violation_count: 0,
                reader_boundary_violation_count: 0,
                artifact_parity_violation_count: 0,
            },
            review: BlindReview {
                initial_ballot_receipt_sha256: [
                    sha256_bytes(format!("ballot-a-{case_index}-{slot_index}").as_bytes()),
                    sha256_bytes(format!("ballot-b-{case_index}-{slot_index}").as_bytes()),
                ],
                adjudication_ballot_receipt_sha256: None,
                candidate: ratings(4_500),
                baseline: ratings(4_000),
            },
        },
    }
}

fn ratings(value: u16) -> QualityRatings {
    QualityRatings {
        depth_milli: value,
        naturalness_milli: value,
        evidence_use_milli: value,
        decision_value_milli: value,
    }
}

fn time(value: chrono::DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
