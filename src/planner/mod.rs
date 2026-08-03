//! Bounded, domain-agnostic research planning contracts.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::engine::DEFAULT_PLANNER_ATTEMPT_TIMEOUT_MS;
use crate::research::{
    EvidenceQualityRequirements, InquiryEvent, InquiryLimits, InquiryState, Question,
    QuestionStatus, ResearchObligation,
};

pub(crate) const MAX_PLANNER_TRACK_EFFECTS: u64 = crate::engine::MAX_DEEP_RESEARCH_TRACKS as u64;
const MAX_PLANNER_REQUEST_REQUIREMENTS: usize = 24;
const MAX_PLANNER_QUESTIONS_PER_TRACK: usize = 4;
const MAX_PLANNER_COMPLETION_CRITERIA: usize = 3;
const MAX_PLANNER_SUPPLEMENTAL_QUERIES: usize = 15;
const MAX_PLANNER_SEARCHES: u64 = 16;
const MAX_GAP_ROUNDS: u64 = 4;
const MAX_GAP_SEARCHES: u64 = MAX_PLANNER_TRACK_EFFECTS * MAX_PLANNER_COMPLETION_CRITERIA as u64;
const MAX_PLANNER_CATALOG_SOURCES: u64 = 32;
const MAX_PLANNER_INITIAL_FETCHES: u64 = MAX_PLANNER_CATALOG_SOURCES / 2;
// Supplemental fetches are an attempt fuse, not a promise that every selected
// URL yields usable text. The workflow still admits at most the closed catalog
// source limit, but may refill slots lost to inaccessible or irrelevant URLs.
const MAX_PLANNER_SUPPLEMENTAL_FETCHES: u64 = MAX_PLANNER_CATALOG_SOURCES;

#[derive(Clone, Debug)]
pub struct PlannedInquiry {
    pub value: Value,
}

mod contract;
pub use contract::{deep_research_loop_contract, deep_research_loop_contract_for_language};

include!("planning.rs");
include!("requirements.rs");
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
