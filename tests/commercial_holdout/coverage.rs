use std::collections::HashMap;

use a3s_deep_research::engine::{DeepResearchRequest, EvidenceScope};

use super::super::model::{CaseResult, HoldoutStrata};
use super::super::statistics::rate_bps_ceil;

const MAX_STRATUM_SHARE_BPS: u16 = 6_000;
const MIN_TASK_INTENTS: usize = 4;
const MIN_EVIDENCE_CONDITIONS: usize = 3;
const MIN_FRESHNESS_CLASSES: usize = 2;
const MIN_SOURCE_MIXES: usize = 3;
const MIN_LANGUAGE_GROUPS: usize = 3;
const MIN_READER_LANGUAGES: usize = 3;

#[derive(Default)]
pub(super) struct Coverage<'a> {
    task_intents: HashMap<&'a str, usize>,
    evidence_conditions: HashMap<&'a str, usize>,
    freshness_classes: HashMap<&'a str, usize>,
    source_mixes: HashMap<&'a str, usize>,
    language_groups: HashMap<&'a str, usize>,
    reader_languages: HashMap<String, usize>,
}

impl<'a> Coverage<'a> {
    pub(super) fn observe(&mut self, case: &'a CaseResult) -> Result<(), String> {
        validate_language(&case.reader_language)?;
        for value in strata_values(&case.strata) {
            validate_stratum(value)?;
        }
        increment(&mut self.task_intents, &case.strata.task_intent);
        increment(
            &mut self.evidence_conditions,
            &case.strata.evidence_condition,
        );
        increment(&mut self.freshness_classes, &case.strata.freshness);
        increment(&mut self.source_mixes, &case.strata.source_mix);
        increment(&mut self.language_groups, &case.strata.language_group);
        *self
            .reader_languages
            .entry(
                case.reader_language
                    .split('-')
                    .next()
                    .unwrap_or_default()
                    .to_ascii_lowercase(),
            )
            .or_default() += 1;
        Ok(())
    }

    pub(super) fn validate(&self, case_count: usize) -> Result<(), String> {
        for (values, minimum, field) in [
            (&self.task_intents, MIN_TASK_INTENTS, "task intent"),
            (
                &self.evidence_conditions,
                MIN_EVIDENCE_CONDITIONS,
                "evidence condition",
            ),
            (&self.freshness_classes, MIN_FRESHNESS_CLASSES, "freshness"),
            (&self.source_mixes, MIN_SOURCE_MIXES, "source mix"),
            (&self.language_groups, MIN_LANGUAGE_GROUPS, "language group"),
        ] {
            if values.len() < minimum
                || values
                    .values()
                    .any(|count| rate_bps_ceil(*count, case_count) > MAX_STRATUM_SHARE_BPS)
            {
                return Err(format!("sealed corpus has insufficient {field} diversity"));
            }
        }
        if self.reader_languages.len() < MIN_READER_LANGUAGES
            || self
                .reader_languages
                .values()
                .any(|count| rate_bps_ceil(*count, case_count) > MAX_STRATUM_SHARE_BPS)
        {
            return Err("sealed corpus has insufficient reader language diversity".to_string());
        }
        Ok(())
    }
}

fn validate_language(language: &str) -> Result<(), String> {
    DeepResearchRequest::new(
        "commercial-holdout-language-check",
        "language contract check",
        EvidenceScope::LocalOnly,
    )
    .with_output_language(language)
    .validate()
}

fn strata_values(strata: &HoldoutStrata) -> [&str; 5] {
    [
        &strata.task_intent,
        &strata.evidence_condition,
        &strata.freshness,
        &strata.source_mix,
        &strata.language_group,
    ]
}

fn validate_stratum(value: &str) -> Result<(), String> {
    if !value.is_empty()
        && value.len() <= 64
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
    {
        Ok(())
    } else {
        Err("holdout stratum must be a bounded opaque tag".to_string())
    }
}

fn increment<'a>(counts: &mut HashMap<&'a str, usize>, value: &'a str) {
    *counts.entry(value).or_default() += 1;
}
