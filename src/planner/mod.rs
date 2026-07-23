//! Bounded, domain-agnostic research planning contracts.

use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::research::{
    EvidenceQualityRequirements, InquiryEvent, InquiryLimits, InquiryState, Question,
    QuestionStatus, ResearchObligation,
};

const MAX_PLANNER_TRACK_EFFECTS: u64 = 4;
const PLANNER_OUTLINE_ATTEMPT_TIMEOUT_MS: u64 = 90_000;

#[derive(Clone, Debug)]
pub struct PlannedInquiry {
    pub value: Value,
}

mod contract;
pub use contract::deep_research_loop_contract;

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
