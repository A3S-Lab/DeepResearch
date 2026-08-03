use chrono::{DateTime, Utc};

use super::super::crypto::{
    attempt_slot_commitment, attempt_start_receipt, attempt_terminal_receipt, validate_sha256,
};
use super::super::model::{
    Answerability, AttemptRecord, AttemptTerminal, CaseResult, CommercialHoldoutPlan,
    PublicationOutcome, QualityRatings,
};
use super::super::statistics::rate_bps;

const MAX_AUDIT_ITEMS_PER_ATTEMPT: usize = 1_000_000;
const MAX_FILES_PER_ATTEMPT: usize = 100_000;
const MIN_FILES_PER_ATTEMPT: usize = 6;

#[derive(Debug)]
pub(super) struct AttemptEvaluation {
    pub(super) synthesized: bool,
    pub(super) no_evidence: bool,
    pub(super) commercial_success: bool,
    pub(super) safe_abstention: bool,
    pub(super) infrastructure_failure: bool,
    pub(super) hard_violations: usize,
    pub(super) material_claims: usize,
    pub(super) cited_material_claims: usize,
    pub(super) audited_pairs: usize,
    pub(super) entailed_pairs: usize,
    pub(super) candidate_ratings: [u16; 4],
    pub(super) candidate_composite_milli: i16,
    pub(super) baseline_composite_milli: i16,
}

pub(super) fn validate_and_evaluate(
    attempt: &AttemptRecord,
    case: &CaseResult,
    plan: &CommercialHoldoutPlan,
    result_started_at: DateTime<Utc>,
    result_finished_at: DateTime<Utc>,
) -> Result<AttemptEvaluation, String> {
    validate_slot(attempt, case, plan.execution.attempts_per_case)?;
    validate_execution(attempt, result_started_at, result_finished_at)?;
    validate_artifact(attempt)?;
    validate_review(attempt)?;
    validate_claims(attempt)?;

    let infrastructure_failure =
        attempt.execution.terminal == AttemptTerminal::InfrastructureFailure;
    validate_terminal_payload(attempt, infrastructure_failure)?;
    let publication = attempt.assessment.publication;
    let synthesized = publication == Some(PublicationOutcome::Synthesized);
    let no_evidence = publication == Some(PublicationOutcome::NoEvidence);
    if attempt.assessment.deeply_closed != synthesized {
        return Err(
            "attempt deeply_closed state must be derived from synthesized publication".to_string(),
        );
    }
    if case.answerability == Answerability::IntentionallyUnanswerable && synthesized {
        return Err("unanswerable attempt claims a synthesized completion".to_string());
    }

    let language_mismatch = usize::from(
        attempt.assessment.output_language.as_deref() != Some(case.reader_language.as_str()),
    ) * usize::from(!infrastructure_failure);
    let hard_violations = violation_count(attempt).saturating_add(language_mismatch);
    let candidate_ratings = &attempt.assessment.review.candidate;
    let rating_floor_met = ratings(candidate_ratings)
        .into_iter()
        .all(|rating| rating >= plan.thresholds.minimum_rating_milli);
    let citation_floor_met = citation_floor_met(attempt, plan);
    let material_depth_met =
        attempt.assessment.claims.material_claim_count >= case.material_dimension_count;
    let safe_abstention = case.answerability == Answerability::IntentionallyUnanswerable
        && !infrastructure_failure
        && no_evidence
        && attempt.assessment.claims.material_claim_count == 0
        && hard_violations == 0;
    let commercial_success = match case.answerability {
        Answerability::Answerable => {
            !infrastructure_failure
                && synthesized
                && material_depth_met
                && citation_floor_met
                && rating_floor_met
                && hard_violations == 0
        }
        Answerability::IntentionallyUnanswerable => safe_abstention && rating_floor_met,
    };

    Ok(AttemptEvaluation {
        synthesized,
        no_evidence,
        commercial_success,
        safe_abstention,
        infrastructure_failure,
        hard_violations,
        material_claims: attempt.assessment.claims.material_claim_count,
        cited_material_claims: attempt.assessment.claims.cited_material_claim_count,
        audited_pairs: attempt.assessment.claims.audited_claim_citation_pair_count,
        entailed_pairs: attempt.assessment.claims.entailed_claim_citation_pair_count,
        candidate_ratings: ratings(candidate_ratings),
        candidate_composite_milli: composite_milli(candidate_ratings),
        baseline_composite_milli: composite_milli(&attempt.assessment.review.baseline),
    })
}

fn validate_slot(
    attempt: &AttemptRecord,
    case: &CaseResult,
    attempts_per_case: usize,
) -> Result<(), String> {
    validate_sha256(&attempt.slot.nonce_sha256, "attempt slot nonce")?;
    validate_sha256(&attempt.slot.commitment_sha256, "attempt slot commitment")?;
    if attempt.slot.index >= attempts_per_case {
        return Err("attempt uses a slot outside the preallocated case range".to_string());
    }
    let expected = attempt_slot_commitment(
        &case.case_commitment_sha256,
        attempt.slot.index,
        &attempt.slot.nonce_sha256,
    );
    if expected != attempt.slot.commitment_sha256 {
        return Err("attempt slot differs from its preallocated commitment".to_string());
    }
    Ok(())
}

fn validate_execution(
    attempt: &AttemptRecord,
    result_started_at: DateTime<Utc>,
    result_finished_at: DateTime<Utc>,
) -> Result<(), String> {
    validate_sha256(
        &attempt.execution.start_receipt_sha256,
        "attempt start receipt",
    )?;
    validate_sha256(
        &attempt.execution.terminal_receipt_sha256,
        "attempt terminal receipt",
    )?;
    let started = timestamp(&attempt.execution.started_at, "attempt started_at")?;
    let finished = timestamp(&attempt.execution.finished_at, "attempt finished_at")?;
    if started < result_started_at || finished > result_finished_at || started >= finished {
        return Err("attempt execution is outside the signed campaign window".to_string());
    }
    let expected_start = attempt_start_receipt(
        &attempt.slot.commitment_sha256,
        &attempt.execution.started_at,
    );
    if expected_start != attempt.execution.start_receipt_sha256 {
        return Err("attempt start receipt does not bind its preallocated slot".to_string());
    }
    let expected_terminal = attempt_terminal_receipt(
        &attempt.execution.start_receipt_sha256,
        terminal_tag(attempt.execution.terminal),
        &attempt.execution.finished_at,
        &attempt.artifact.subtree_sha256,
    );
    if expected_terminal != attempt.execution.terminal_receipt_sha256 {
        return Err("attempt terminal receipt does not bind its artifact subtree".to_string());
    }
    Ok(())
}

fn validate_artifact(attempt: &AttemptRecord) -> Result<(), String> {
    validate_sha256(&attempt.artifact.subtree_sha256, "attempt artifact subtree")?;
    if !(MIN_FILES_PER_ATTEMPT..=MAX_FILES_PER_ATTEMPT).contains(&attempt.artifact.file_count) {
        return Err("attempt artifact subtree has an invalid bound file count".to_string());
    }
    Ok(())
}

fn validate_review(attempt: &AttemptRecord) -> Result<(), String> {
    let review = &attempt.assessment.review;
    for (index, receipt) in review.initial_ballot_receipt_sha256.iter().enumerate() {
        validate_sha256(receipt, &format!("initial review ballot receipt {index}"))?;
    }
    if review.initial_ballot_receipt_sha256[0] == review.initial_ballot_receipt_sha256[1] {
        return Err("attempt initial review ballots must be independent".to_string());
    }
    if let Some(receipt) = &review.adjudication_ballot_receipt_sha256 {
        validate_sha256(receipt, "adjudication ballot receipt")?;
    }
    for rating in ratings(&review.candidate)
        .into_iter()
        .chain(ratings(&review.baseline))
    {
        if !(1_000..=5_000).contains(&rating) {
            return Err("blind-review rating must be within 1,000..=5,000".to_string());
        }
    }
    Ok(())
}

fn validate_claims(attempt: &AttemptRecord) -> Result<(), String> {
    let claims = &attempt.assessment.claims;
    for count in [
        claims.material_claim_count,
        claims.audited_material_claim_count,
        claims.cited_material_claim_count,
        claims.audited_claim_citation_pair_count,
        claims.entailed_claim_citation_pair_count,
    ] {
        if count > MAX_AUDIT_ITEMS_PER_ATTEMPT {
            return Err("attempt audit count exceeds the bounded protocol".to_string());
        }
    }
    if claims.audited_material_claim_count != claims.material_claim_count
        || claims.cited_material_claim_count > claims.material_claim_count
        || claims.audited_claim_citation_pair_count < claims.cited_material_claim_count
        || claims.entailed_claim_citation_pair_count > claims.audited_claim_citation_pair_count
        || (claims.material_claim_count == 0
            && (claims.cited_material_claim_count != 0
                || claims.audited_claim_citation_pair_count != 0
                || claims.entailed_claim_citation_pair_count != 0))
    {
        return Err("attempt claim and citation audit records are inconsistent".to_string());
    }
    for count in [
        attempt
            .assessment
            .violations
            .unsupported_material_claim_count,
        attempt
            .assessment
            .violations
            .citation_integrity_violation_count,
        attempt
            .assessment
            .violations
            .reader_boundary_violation_count,
        attempt
            .assessment
            .violations
            .artifact_parity_violation_count,
    ] {
        if count > MAX_AUDIT_ITEMS_PER_ATTEMPT {
            return Err("attempt violation count exceeds the bounded protocol".to_string());
        }
    }
    Ok(())
}

fn validate_terminal_payload(
    attempt: &AttemptRecord,
    infrastructure_failure: bool,
) -> Result<(), String> {
    let assessment = &attempt.assessment;
    if infrastructure_failure {
        if assessment.publication.is_some()
            || assessment.output_language.is_some()
            || assessment.deeply_closed
            || assessment.claims.material_claim_count != 0
            || violation_count(attempt) != 0
        {
            return Err(
                "infrastructure failure attempt contains a fabricated publication".to_string(),
            );
        }
    } else if assessment.publication.is_none() || assessment.output_language.is_none() {
        return Err("completed attempt is missing its typed publication or language".to_string());
    }
    Ok(())
}

fn citation_floor_met(attempt: &AttemptRecord, plan: &CommercialHoldoutPlan) -> bool {
    let claims = &attempt.assessment.claims;
    claims.material_claim_count > 0
        && claims.audited_claim_citation_pair_count > 0
        && rate_bps(
            claims.cited_material_claim_count,
            claims.material_claim_count,
        ) >= plan.thresholds.minimum_citation_recall_bps
        && rate_bps(
            claims.entailed_claim_citation_pair_count,
            claims.audited_claim_citation_pair_count,
        ) >= plan.thresholds.minimum_citation_precision_bps
}

fn violation_count(attempt: &AttemptRecord) -> usize {
    let violations = &attempt.assessment.violations;
    violations
        .unsupported_material_claim_count
        .saturating_add(violations.citation_integrity_violation_count)
        .saturating_add(violations.reader_boundary_violation_count)
        .saturating_add(violations.artifact_parity_violation_count)
}

fn ratings(ratings: &QualityRatings) -> [u16; 4] {
    [
        ratings.depth_milli,
        ratings.naturalness_milli,
        ratings.evidence_use_milli,
        ratings.decision_value_milli,
    ]
}

fn composite_milli(values: &QualityRatings) -> i16 {
    let sum = ratings(values).into_iter().map(u32::from).sum::<u32>();
    (sum / 4) as i16
}

pub(super) const fn terminal_tag(terminal: AttemptTerminal) -> &'static str {
    match terminal {
        AttemptTerminal::Completed => "completed",
        AttemptTerminal::InfrastructureFailure => "infrastructure_failure",
    }
}

fn timestamp(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| format!("invalid {field}: {error}"))
}
