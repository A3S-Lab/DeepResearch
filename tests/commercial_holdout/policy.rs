use std::collections::HashSet;

use chrono::{DateTime, Utc};

use super::super::crypto::{
    case_descriptor_commitment, sorted_artifact_subtree_root, sorted_attempt_commitment_root,
    sorted_attempt_log_root, sorted_ballot_receipt_root, sorted_commitment_root, validate_sha256,
};
use super::super::model::{
    Answerability, CaseResult, CommercialHoldoutPlan, CommercialHoldoutResult,
};
use super::super::statistics::{
    paired_delta_lower_milli, rate_bps, rate_bps_ceil, wilson_lower_bps,
};
use super::attempt::{validate_and_evaluate, AttemptEvaluation};
use super::coverage::Coverage;
use super::{HoldoutSummary, HOLDOUT_MIN_CASES};

const MAX_CASES: usize = 1_024;
const MAX_MATERIAL_DIMENSIONS_PER_CASE: usize = 128;

#[derive(Default)]
struct Totals {
    attempts: usize,
    answerable_attempts: usize,
    answerable_synthesized: usize,
    commercial_successes: usize,
    unanswerable_attempts: usize,
    safe_abstentions: usize,
    answerable_no_evidence: usize,
    infrastructure_failures: usize,
    successful_cases: usize,
    baseline_losses: usize,
    hard_violations: usize,
    material_claims: usize,
    cited_material_claims: usize,
    audited_pairs: usize,
    entailed_pairs: usize,
    success_ratings: [Vec<u16>; 4],
    paired_deltas: Vec<f64>,
}

#[derive(Default)]
struct CaseTotals {
    commercial_successes: usize,
    candidate_composite_sum: i64,
    baseline_composite_sum: i64,
}

#[derive(Default)]
struct RecordSet<'a> {
    slots: HashSet<&'a str>,
    nonces: HashSet<&'a str>,
    starts: HashSet<&'a str>,
    terminals: HashSet<&'a str>,
    artifacts: HashSet<&'a str>,
    ballots: HashSet<&'a str>,
}

pub(super) fn evaluate(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
) -> Result<HoldoutSummary, String> {
    if result.cases.len() != plan.corpus.case_count
        || result.cases.len() < HOLDOUT_MIN_CASES
        || result.cases.len() > MAX_CASES
    {
        return Err("result case set does not match the bounded sealed plan".to_string());
    }

    let result_started_at = timestamp(&result.started_at, "result started_at")?;
    let result_finished_at = timestamp(&result.finished_at, "result finished_at")?;
    let mut case_commitments = HashSet::new();
    let mut sealed_payloads = HashSet::new();
    let mut records = RecordSet::default();
    let mut coverage = Coverage::default();
    let mut totals = Totals::default();
    let mut unanswerable_cases = 0_usize;

    for case in &result.cases {
        if !case_commitments.insert(case.case_commitment_sha256.as_str()) {
            return Err("result contains a duplicate sealed case commitment".to_string());
        }
        if !sealed_payloads.insert(case.sealed_case_payload_sha256.as_str()) {
            return Err("result reuses a sealed case payload".to_string());
        }
        validate_sha256(&case.case_commitment_sha256, "case commitment")?;
        validate_case(case, plan)?;
        coverage.observe(case)?;

        let mut slot_indices = HashSet::new();
        let mut case_totals = CaseTotals::default();
        for attempt in &case.attempts {
            if !slot_indices.insert(attempt.slot.index) {
                return Err("case contains a duplicate preallocated attempt slot".to_string());
            }
            records.observe(attempt)?;
            let evaluated =
                validate_and_evaluate(attempt, case, plan, result_started_at, result_finished_at)?;
            totals.observe(case.answerability, &evaluated);
            case_totals.observe(&evaluated);
        }
        if slot_indices.len() != plan.execution.attempts_per_case
            || !(0..plan.execution.attempts_per_case)
                .all(|slot_index| slot_indices.contains(&slot_index))
        {
            return Err(
                "case does not retain each preallocated attempt slot exactly once".to_string(),
            );
        }
        totals.finish_case(&case_totals, plan)?;
        unanswerable_cases +=
            usize::from(case.answerability == Answerability::IntentionallyUnanswerable);
    }

    validate_exact_sets(plan, result, &case_commitments, &records)?;
    coverage.validate(result.cases.len())?;
    validate_answerability_mix(plan, result.cases.len(), unanswerable_cases)?;
    finish(plan, result, totals)
}

fn validate_case(case: &CaseResult, plan: &CommercialHoldoutPlan) -> Result<(), String> {
    validate_case_commitment(case)?;
    if case.material_dimension_count == 0
        || case.material_dimension_count > MAX_MATERIAL_DIMENSIONS_PER_CASE
        || case.attempts.len() != plan.execution.attempts_per_case
    {
        return Err(
            "case does not retain exactly three attempts and its material dimensions".to_string(),
        );
    }
    Ok(())
}

fn validate_case_commitment(case: &CaseResult) -> Result<(), String> {
    validate_sha256(&case.sealed_case_payload_sha256, "sealed case payload")?;
    let answerability = match case.answerability {
        Answerability::Answerable => "answerable",
        Answerability::IntentionallyUnanswerable => "intentionally_unanswerable",
    };
    let expected = case_descriptor_commitment(
        &case.sealed_case_payload_sha256,
        &[
            &case.reader_language,
            answerability,
            &case.strata.task_intent,
            &case.strata.evidence_condition,
            &case.strata.freshness,
            &case.strata.source_mix,
            &case.strata.language_group,
        ],
        case.material_dimension_count,
    );
    if expected == case.case_commitment_sha256 {
        Ok(())
    } else {
        Err("case descriptor differs from its precommitted sealed identity".to_string())
    }
}

fn validate_exact_sets(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
    case_commitments: &HashSet<&str>,
    records: &RecordSet<'_>,
) -> Result<(), String> {
    if case_commitments.len() != plan.corpus.case_count
        || sorted_commitment_root(case_commitments.iter().copied())
            != plan.corpus.case_set_root_sha256
    {
        return Err("result cases differ from the precommitted sealed case set".to_string());
    }
    if records.slots.len() != plan.execution.attempt_count {
        return Err("result does not contain the exact preallocated attempt set".to_string());
    }
    let attempt_root = sorted_attempt_commitment_root(records.slots.iter().copied());
    if attempt_root != plan.execution.attempt_set_root_sha256
        || attempt_root != result.bindings.attempt_set_root_sha256
    {
        return Err("result attempt set differs from the precommitted plan root".to_string());
    }
    if sorted_attempt_log_root(records.terminals.iter().copied())
        != result.execution.attempt_log_root_sha256
    {
        return Err("result attempt log root does not match terminal receipts".to_string());
    }
    if sorted_artifact_subtree_root(records.artifacts.iter().copied())
        != result.artifacts.root_sha256
    {
        return Err("result artifact root does not match attempt subtrees".to_string());
    }
    if sorted_ballot_receipt_root(records.ballots.iter().copied())
        != result.review.ballot_root_sha256
    {
        return Err("result review root does not match retained ballot receipts".to_string());
    }
    Ok(())
}

fn validate_answerability_mix(
    plan: &CommercialHoldoutPlan,
    case_count: usize,
    unanswerable_cases: usize,
) -> Result<(), String> {
    if unanswerable_cases < plan.thresholds.minimum_unanswerable_cases
        || unanswerable_cases.saturating_mul(4) > case_count
    {
        return Err(
            "sealed corpus lacks a bounded answerability and abstention calibration stratum"
                .to_string(),
        );
    }
    Ok(())
}

fn finish(
    plan: &CommercialHoldoutPlan,
    result: &CommercialHoldoutResult,
    totals: Totals,
) -> Result<HoldoutSummary, String> {
    let threshold = &plan.thresholds;
    let synthesized_rate = rate_bps(totals.answerable_synthesized, totals.answerable_attempts);
    let commercial_rate = rate_bps(totals.commercial_successes, totals.attempts);
    let successful_case_rate = rate_bps(totals.successful_cases, result.cases.len());
    let successful_case_lcb = wilson_lower_bps(totals.successful_cases, result.cases.len());
    let abstention_rate = rate_bps(totals.safe_abstentions, totals.unanswerable_attempts);
    let no_evidence_rate = rate_bps_ceil(totals.answerable_no_evidence, totals.answerable_attempts);
    let infrastructure_failure_rate =
        rate_bps_ceil(totals.infrastructure_failures, totals.attempts);
    let baseline_loss_rate = rate_bps_ceil(totals.baseline_losses, result.cases.len());
    let baseline_delta_lcb = paired_delta_lower_milli(&totals.paired_deltas);
    let citation_recall = rate_bps(totals.cited_material_claims, totals.material_claims);
    let citation_precision = rate_bps(totals.entailed_pairs, totals.audited_pairs);
    let median_ratings = totals.success_ratings.map(median);

    if synthesized_rate < threshold.minimum_answerable_synthesized_attempt_rate_bps {
        return Err("answerable synthesized completion rate is below the plan".to_string());
    }
    if commercial_rate < threshold.minimum_commercial_success_attempt_rate_bps
        || successful_case_rate < threshold.minimum_case_success_rate_bps
        || successful_case_lcb < threshold.minimum_case_success_lcb_bps
    {
        return Err("commercial success rate or case-cluster confidence floor failed".to_string());
    }
    if abstention_rate < threshold.minimum_safe_abstention_rate_bps
        || no_evidence_rate > threshold.maximum_answerable_no_evidence_rate_bps
        || infrastructure_failure_rate > threshold.maximum_infrastructure_failure_rate_bps
    {
        return Err(
            "abstention, answerable no-evidence, or infrastructure floor failed".to_string(),
        );
    }
    if totals.hard_violations > threshold.maximum_hard_violations
        || citation_recall < threshold.minimum_citation_recall_bps
        || citation_precision < threshold.minimum_citation_precision_bps
    {
        return Err("claim, citation, or reader-boundary hard gate failed".to_string());
    }
    if median_ratings
        .iter()
        .any(|rating| *rating < threshold.minimum_rating_milli)
    {
        return Err("blind-review rating floor failed".to_string());
    }
    if baseline_loss_rate > threshold.maximum_baseline_case_loss_rate_bps
        || baseline_delta_lcb < threshold.baseline_noninferiority_margin_milli
    {
        return Err("paired baseline non-inferiority floor failed".to_string());
    }

    Ok(HoldoutSummary {
        schema: "a3s/deep-research-commercial-holdout-summary/v2",
        campaign_id: plan.campaign_id.clone(),
        case_count: result.cases.len(),
        attempt_count: totals.attempts,
        answerable_synthesized_attempt_rate_bps: synthesized_rate,
        commercial_success_attempt_rate_bps: commercial_rate,
        successful_case_rate_bps: successful_case_rate,
        successful_case_lcb_bps: successful_case_lcb,
        safe_abstention_rate_bps: abstention_rate,
        answerable_no_evidence_rate_bps: no_evidence_rate,
        infrastructure_failure_rate_bps: infrastructure_failure_rate,
        citation_recall_bps: citation_recall,
        citation_precision_bps: citation_precision,
        median_depth_milli: median_ratings[0],
        median_naturalness_milli: median_ratings[1],
        median_evidence_use_milli: median_ratings[2],
        median_decision_value_milli: median_ratings[3],
        baseline_loss_rate_bps: baseline_loss_rate,
        baseline_delta_lcb_milli: baseline_delta_lcb,
    })
}

impl<'a> RecordSet<'a> {
    fn observe(&mut self, attempt: &'a super::super::model::AttemptRecord) -> Result<(), String> {
        insert_unique(
            &mut self.slots,
            &attempt.slot.commitment_sha256,
            "attempt slot commitment",
        )?;
        insert_unique(
            &mut self.nonces,
            &attempt.slot.nonce_sha256,
            "attempt slot nonce",
        )?;
        insert_unique(
            &mut self.starts,
            &attempt.execution.start_receipt_sha256,
            "attempt start receipt",
        )?;
        insert_unique(
            &mut self.terminals,
            &attempt.execution.terminal_receipt_sha256,
            "attempt terminal receipt",
        )?;
        insert_unique(
            &mut self.artifacts,
            &attempt.artifact.subtree_sha256,
            "attempt artifact subtree",
        )?;
        for ballot in &attempt.assessment.review.initial_ballot_receipt_sha256 {
            insert_unique(&mut self.ballots, ballot, "initial review ballot")?;
        }
        if let Some(ballot) = &attempt.assessment.review.adjudication_ballot_receipt_sha256 {
            insert_unique(&mut self.ballots, ballot, "adjudication review ballot")?;
        }
        Ok(())
    }
}

impl Totals {
    fn observe(&mut self, answerability: Answerability, attempt: &AttemptEvaluation) {
        self.attempts += 1;
        self.commercial_successes += usize::from(attempt.commercial_success);
        self.infrastructure_failures += usize::from(attempt.infrastructure_failure);
        self.hard_violations += attempt.hard_violations;
        self.material_claims += attempt.material_claims;
        self.cited_material_claims += attempt.cited_material_claims;
        self.audited_pairs += attempt.audited_pairs;
        self.entailed_pairs += attempt.entailed_pairs;
        if attempt.commercial_success {
            for (ratings, rating) in self
                .success_ratings
                .iter_mut()
                .zip(attempt.candidate_ratings)
            {
                ratings.push(rating);
            }
        }
        match answerability {
            Answerability::Answerable => {
                self.answerable_attempts += 1;
                self.answerable_synthesized += usize::from(attempt.synthesized);
                self.answerable_no_evidence += usize::from(attempt.no_evidence);
            }
            Answerability::IntentionallyUnanswerable => {
                self.unanswerable_attempts += 1;
                self.safe_abstentions += usize::from(attempt.safe_abstention);
            }
        }
    }

    fn finish_case(
        &mut self,
        case: &CaseTotals,
        plan: &CommercialHoldoutPlan,
    ) -> Result<(), String> {
        if plan.thresholds.reject_zero_success_case && case.commercial_successes == 0 {
            return Err("case has zero commercial success attempts".to_string());
        }
        self.successful_cases +=
            usize::from(case.commercial_successes >= plan.thresholds.case_success_minimum_attempts);
        let attempts = plan.execution.attempts_per_case as f64;
        let delta = (case.candidate_composite_sum - case.baseline_composite_sum) as f64 / attempts;
        self.baseline_losses += usize::from(delta < 0.0);
        self.paired_deltas.push(delta);
        Ok(())
    }
}

impl CaseTotals {
    fn observe(&mut self, attempt: &AttemptEvaluation) {
        self.commercial_successes += usize::from(attempt.commercial_success);
        self.candidate_composite_sum += i64::from(attempt.candidate_composite_milli);
        self.baseline_composite_sum += i64::from(attempt.baseline_composite_milli);
    }
}

fn insert_unique<'a>(
    values: &mut HashSet<&'a str>,
    value: &'a str,
    field: &str,
) -> Result<(), String> {
    if values.insert(value) {
        Ok(())
    } else {
        Err(format!("result contains a duplicate {field}"))
    }
}

fn median(mut values: Vec<u16>) -> u16 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        ((u32::from(values[middle - 1]) + u32::from(values[middle])) / 2) as u16
    } else {
        values[middle]
    }
}

fn timestamp(value: &str, field: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| format!("invalid {field}: {error}"))
}
