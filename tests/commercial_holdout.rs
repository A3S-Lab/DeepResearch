#[path = "commercial_holdout/crypto.rs"]
mod crypto;
#[path = "commercial_holdout/fixture.rs"]
mod fixture;
#[path = "commercial_holdout/model.rs"]
mod model;
#[path = "commercial_holdout/statistics.rs"]
mod statistics;
#[path = "commercial_holdout/validate.rs"]
mod validate;

use std::path::PathBuf;

use fixture::{signed_test_packet, TestPacket};
use validate::{
    validate_signed_attestations, ReleaseContext, SignedAttestations, HOLDOUT_MIN_CASES,
};

const PLAN_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_PLAN";
const PLAN_SIGNATURE_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_PLAN_SIGNATURE_HEX";
const PLAN_PUBLIC_KEY_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_PLAN_PUBLIC_KEY_HEX";
const RESULT_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_RESULT";
const EXECUTION_SIGNATURE_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_EXECUTION_SIGNATURE_HEX";
const EXECUTION_PUBLIC_KEY_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_EXECUTION_PUBLIC_KEY_HEX";
const REVIEW_SIGNATURE_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_REVIEW_SIGNATURE_HEX";
const REVIEW_PUBLIC_KEY_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_REVIEW_PUBLIC_KEY_HEX";
const PLAN_COMMITMENT_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_PLAN_SHA256";
const LOG_CHECKPOINT_ENV: &str = "A3S_DEEP_RESEARCH_HOLDOUT_LOG_CHECKPOINT_SHA256";
const RELEASE_REPOSITORY_ENV: &str = "A3S_DEEP_RESEARCH_RELEASE_REPOSITORY";
const RELEASE_COMMIT_ENV: &str = "A3S_DEEP_RESEARCH_RELEASE_COMMIT";
const RELEASE_VERSION_ENV: &str = "A3S_DEEP_RESEARCH_RELEASE_VERSION";
const RELEASE_PACKAGE_ENV: &str = "A3S_DEEP_RESEARCH_RELEASE_PACKAGE";

#[test]
#[ignore = "requires an explicitly supplied signed commercial holdout"]
fn signed_commercial_holdout_meets_the_predeclared_release_floor() {
    let plan_bytes = std::fs::read(required_path(PLAN_ENV)).expect("read signed holdout plan");
    let result_bytes =
        std::fs::read(required_path(RESULT_ENV)).expect("read signed holdout result");
    let context = ReleaseContext {
        repository: required_string(RELEASE_REPOSITORY_ENV),
        git_commit: required_string(RELEASE_COMMIT_ENV),
        package_version: required_string(RELEASE_VERSION_ENV),
        package_sha256: crypto::sha256_file(required_path(RELEASE_PACKAGE_ENV))
            .expect("hash release package"),
        cargo_lock_sha256: crypto::sha256_file("Cargo.lock").expect("hash Cargo.lock"),
        plan_payload_sha256: required_string(PLAN_COMMITMENT_ENV),
        transparency_checkpoint_sha256: required_string(LOG_CHECKPOINT_ENV),
    };
    let summary = validate_signed_attestations(
        SignedAttestations {
            plan_bytes: &plan_bytes,
            plan_signature_hex: &required_string(PLAN_SIGNATURE_ENV),
            plan_public_key_hex: &required_string(PLAN_PUBLIC_KEY_ENV),
            result_bytes: &result_bytes,
            execution_signature_hex: &required_string(EXECUTION_SIGNATURE_ENV),
            execution_public_key_hex: &required_string(EXECUTION_PUBLIC_KEY_ENV),
            review_signature_hex: &required_string(REVIEW_SIGNATURE_ENV),
            review_public_key_hex: &required_string(REVIEW_PUBLIC_KEY_ENV),
        },
        &context,
        HOLDOUT_MIN_CASES,
    )
    .unwrap_or_else(|error| panic!("commercial holdout failed: {error}"));

    println!(
        "{}",
        serde_json::to_string(&summary).expect("encode commercial holdout summary")
    );
}

#[test]
fn signed_holdout_accepts_a_valid_domain_neutral_campaign() {
    let packet = signed_test_packet();
    validate_test_packet(&packet).expect("accept valid signed holdout");
}

#[test]
fn signed_holdout_rejects_zero_complete_reports() {
    let mut packet = signed_test_packet();
    for case in &mut packet.result.cases {
        if matches!(case.answerability, model::Answerability::Answerable) {
            for attempt in &mut case.attempts {
                attempt.assessment.publication = Some(model::PublicationOutcome::Qualified);
                attempt.assessment.deeply_closed = false;
            }
        }
    }
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject zero complete reports");
    assert!(
        error.contains("zero commercial success") || error.contains("completion"),
        "{error}"
    );
}

#[test]
fn signed_holdout_rejects_a_modified_result() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].attempts[0]
        .assessment
        .review
        .candidate
        .depth_milli = 1_000;
    let error = validate_test_packet(&packet).expect_err("reject modified signed bytes");
    assert!(error.contains("signature"), "{error}");
}

#[test]
fn signed_holdout_rejects_an_uncommitted_plan() {
    let mut packet = signed_test_packet();
    packet.context.plan_payload_sha256 = "f".repeat(64);
    let error = validate_test_packet(&packet).expect_err("reject post-hoc holdout plan");
    assert!(error.contains("precommitted"), "{error}");
}

#[test]
fn signed_holdout_rejects_attempt_replacement() {
    let mut packet = signed_test_packet();
    {
        let case_commitment = packet.result.cases[0].case_commitment_sha256.clone();
        let attempt = &mut packet.result.cases[0].attempts[0];
        attempt.slot.nonce_sha256 = crypto::sha256_bytes(b"replacement-slot-nonce");
        attempt.slot.commitment_sha256 = crypto::attempt_slot_commitment(
            &case_commitment,
            attempt.slot.index,
            &attempt.slot.nonce_sha256,
        );
        attempt.execution.start_receipt_sha256 = crypto::attempt_start_receipt(
            &attempt.slot.commitment_sha256,
            &attempt.execution.started_at,
        );
        attempt.execution.terminal_receipt_sha256 = crypto::attempt_terminal_receipt(
            &attempt.execution.start_receipt_sha256,
            "completed",
            &attempt.execution.finished_at,
            &attempt.artifact.subtree_sha256,
        );
    }
    packet.result.execution.attempt_log_root_sha256 =
        crypto::sorted_attempt_log_root(packet.result.cases.iter().flat_map(|case| {
            case.attempts
                .iter()
                .map(|attempt| attempt.execution.terminal_receipt_sha256.as_str())
        }));
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject cherry-picked attempts");
    assert!(error.contains("precommitted plan root"), "{error}");
}

#[test]
fn signed_holdout_rejects_a_missing_attempt_record() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].attempts.pop();
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject missing attempt record");
    assert!(error.contains("exactly three attempts"), "{error}");
}

#[test]
fn signed_holdout_recomputes_the_attempt_set_root() {
    let mut packet = signed_test_packet();
    packet.plan.execution.attempt_set_root_sha256 = "f".repeat(64);
    packet.result.bindings.attempt_set_root_sha256 = "f".repeat(64);
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject a forged attempt set root");
    assert!(error.contains("precommitted plan root"), "{error}");
}

#[test]
fn signed_holdout_rejects_an_unbound_start_receipt() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].attempts[0]
        .execution
        .start_receipt_sha256 = "f".repeat(64);
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject unbound start receipt");
    assert!(error.contains("start receipt"), "{error}");
}

#[test]
fn signed_holdout_rejects_an_unbound_artifact_subtree() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].attempts[0].artifact.subtree_sha256 = "f".repeat(64);
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject unbound artifact subtree");
    assert!(error.contains("terminal receipt"), "{error}");
}

#[test]
fn signed_holdout_recomputes_the_ballot_root() {
    let mut packet = signed_test_packet();
    packet.result.review.ballot_root_sha256 = "f".repeat(64);
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject a forged review ballot root");
    assert!(error.contains("ballot receipts"), "{error}");
}

#[test]
fn signed_holdout_rejects_a_post_plan_case_swap() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].case_commitment_sha256 = "f".repeat(64);
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject case-set replacement");
    assert!(
        error.contains("sealed identity") || error.contains("sealed case set"),
        "{error}"
    );
}

#[test]
fn signed_holdout_rejects_post_plan_answerability_reclassification() {
    let mut packet = signed_test_packet();
    let case = &mut packet.result.cases[6];
    case.answerability = model::Answerability::IntentionallyUnanswerable;
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject answerability laundering");
    assert!(error.contains("sealed identity"), "{error}");
}

#[test]
fn signed_holdout_rejects_claim_free_complete_reports() {
    let mut packet = signed_test_packet();
    for case in &mut packet.result.cases {
        if matches!(case.answerability, model::Answerability::Answerable) {
            for attempt in &mut case.attempts {
                attempt.assessment.claims.material_claim_count = 0;
                attempt.assessment.claims.audited_material_claim_count = 0;
                attempt.assessment.claims.cited_material_claim_count = 0;
                attempt.assessment.claims.audited_claim_citation_pair_count = 0;
                attempt.assessment.claims.entailed_claim_citation_pair_count = 0;
            }
        }
    }
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject claim-free completion");
    assert!(error.contains("zero commercial success"), "{error}");
}

#[test]
fn signed_holdout_recomputes_baseline_losses_from_attempt_ratings() {
    let mut packet = signed_test_packet();
    for case in &mut packet.result.cases {
        for attempt in &mut case.attempts {
            let baseline = &mut attempt.assessment.review.baseline;
            baseline.depth_milli = 5_000;
            baseline.naturalness_milli = 5_000;
            baseline.evidence_use_milli = 5_000;
            baseline.decision_value_milli = 5_000;
        }
    }
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject derived baseline losses");
    assert!(error.contains("baseline non-inferiority"), "{error}");
}

#[test]
fn signed_holdout_derives_language_violations_from_attempt_output() {
    let mut packet = signed_test_packet();
    packet.result.cases[0].attempts[0]
        .assessment
        .output_language = Some("de".to_string());
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject language boundary violation");
    assert!(error.contains("hard gate"), "{error}");
}

#[test]
fn signed_holdout_derives_safe_abstention_from_attempt_evidence() {
    let mut packet = signed_test_packet();
    for attempt in &mut packet.result.cases[0].attempts {
        attempt.assessment.claims.material_claim_count = 1;
        attempt.assessment.claims.audited_material_claim_count = 1;
        attempt.assessment.claims.cited_material_claim_count = 1;
        attempt.assessment.claims.audited_claim_citation_pair_count = 1;
        attempt.assessment.claims.entailed_claim_citation_pair_count = 1;
    }
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject abstention laundering");
    assert!(error.contains("zero commercial success"), "{error}");
}

#[test]
fn signed_holdout_derives_infrastructure_failure_from_terminal_records() {
    let mut packet = signed_test_packet();
    let attempt = &mut packet.result.cases[0].attempts[0];
    attempt.execution.terminal = model::AttemptTerminal::InfrastructureFailure;
    attempt.assessment.publication = None;
    attempt.assessment.output_language = None;
    attempt.assessment.deeply_closed = false;
    attempt.execution.terminal_receipt_sha256 = crypto::attempt_terminal_receipt(
        &attempt.execution.start_receipt_sha256,
        "infrastructure_failure",
        &attempt.execution.finished_at,
        &attempt.artifact.subtree_sha256,
    );
    packet.result.execution.attempt_log_root_sha256 =
        crypto::sorted_attempt_log_root(packet.result.cases.iter().flat_map(|case| {
            case.attempts
                .iter()
                .map(|attempt| attempt.execution.terminal_receipt_sha256.as_str())
        }));
    packet.resign();
    let summary = validate_test_packet(&packet).expect("accept retained infrastructure failure");
    assert_eq!(summary.infrastructure_failure_rate_bps, 70);
    assert_eq!(summary.attempt_count, HOLDOUT_MIN_CASES * 3);
}

#[test]
fn signed_holdout_rejects_a_weakened_predeclared_policy() {
    let mut packet = signed_test_packet();
    packet
        .plan
        .thresholds
        .minimum_commercial_success_attempt_rate_bps = 5_000;
    packet.resign();
    let error = validate_test_packet(&packet).expect_err("reject weakened quality policy");
    assert!(error.contains("weakens"), "{error}");
}

fn validate_test_packet(packet: &TestPacket) -> Result<validate::HoldoutSummary, String> {
    let plan_bytes = serde_json::to_vec(&packet.plan).expect("encode test plan");
    let result_bytes = serde_json::to_vec(&packet.result).expect("encode test result");
    validate_signed_attestations(
        SignedAttestations {
            plan_bytes: &plan_bytes,
            plan_signature_hex: &packet.plan_signature_hex,
            plan_public_key_hex: &packet.plan_public_key_hex,
            result_bytes: &result_bytes,
            execution_signature_hex: &packet.execution_signature_hex,
            execution_public_key_hex: &packet.execution_public_key_hex,
            review_signature_hex: &packet.review_signature_hex,
            review_public_key_hex: &packet.review_public_key_hex,
        },
        &packet.context,
        HOLDOUT_MIN_CASES,
    )
}

fn required_string(variable: &str) -> String {
    std::env::var(variable)
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| panic!("set {variable} for the signed holdout validator"))
}

fn required_path(variable: &str) -> PathBuf {
    PathBuf::from(required_string(variable))
}
