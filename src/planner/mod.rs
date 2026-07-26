//! Bounded, domain-agnostic research planning contracts.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::research::{
    EvidenceQualityRequirements, InquiryEvent, InquiryLimits, InquiryState, Question,
    QuestionStatus, ResearchObligation,
};

const MAX_PLANNER_TRACK_EFFECTS: u64 = 4;
const MAX_PLANNER_QUESTIONS_PER_TRACK: usize = 4;
const MAX_PLANNER_COMPLETION_CRITERIA: usize = 3;
const MAX_PLANNER_SUPPLEMENTAL_QUERIES: usize = 7;
const MAX_PLANNER_SEARCHES: u64 = 8;
const MAX_PLANNER_INITIAL_FETCHES: u64 = 12;
const MAX_PLANNER_SUPPLEMENTAL_FETCHES: u64 = 4;
const PLANNER_OUTLINE_ATTEMPT_TIMEOUT_MS: u64 = 90_000;

#[derive(Clone, Debug)]
pub struct PlannedInquiry {
    pub value: Value,
}

mod contract;
pub use contract::{deep_research_loop_contract, deep_research_loop_contract_for_language};

include!("planning.rs");
include!("bounding.rs");

fn apply_event(
    state: &mut InquiryState,
    events: &mut Vec<InquiryEvent>,
    event: InquiryEvent,
    limits: &InquiryLimits,
) -> Result<(), String> {
    state
        .apply(&event, limits)
        .map_err(|error| format!("apply inquiry event `{}`: {error}", event.name()))?;
    events.push(event);
    Ok(())
}
