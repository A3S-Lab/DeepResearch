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
// A commercial-depth material dimension needs distinct acquisition paths for
// its factual baseline, comparison or mechanism, and adversarial boundary.
// Keep one additional path for source failure or a second factual baseline.
// This is a domain-neutral planning capacity, not a topic classifier.
const MIN_QUERY_INTENTS_PER_DEEP_TRACK: usize = 4;
pub(crate) const DEFAULT_DEPTH_FIRST_MAX_TRACKS: u8 = depth_first_track_capacity(
    crate::engine::MAX_DEEP_RESEARCH_TRACKS,
    MAX_PLANNER_SEARCHES as usize,
    MIN_QUERY_INTENTS_PER_DEEP_TRACK,
);
const MAX_GAP_ROUNDS: u64 = 4;
const MAX_GAP_SEARCHES: u64 = MAX_PLANNER_TRACK_EFFECTS * MAX_PLANNER_COMPLETION_CRITERIA as u64;
const MAX_PLANNER_CATALOG_SOURCES: u64 = 32;
const MAX_PLANNER_INITIAL_FETCHES: u64 = MAX_PLANNER_CATALOG_SOURCES / 2;
// Supplemental fetches are an attempt fuse, not a promise that every selected
// URL yields usable text. The workflow still admits at most the closed catalog
// source limit, but may refill slots lost to inaccessible or irrelevant URLs.
const MAX_PLANNER_SUPPLEMENTAL_FETCHES: u64 = MAX_PLANNER_CATALOG_SOURCES;

const fn depth_first_track_capacity(
    hard_track_cap: u8,
    available_searches: usize,
    minimum_query_intents_per_track: usize,
) -> u8 {
    let search_capacity = match available_searches.checked_div(minimum_query_intents_per_track) {
        Some(capacity) => capacity,
        None => hard_track_cap as usize,
    };
    let bounded = if search_capacity == 0 {
        1
    } else if search_capacity > hard_track_cap as usize {
        hard_track_cap as usize
    } else {
        search_capacity
    };
    bounded as u8
}

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

#[cfg(test)]
mod capacity_tests {
    use super::*;

    #[test]
    fn depth_first_track_capacity_scales_with_search_capacity_and_hard_caps() {
        assert_eq!(depth_first_track_capacity(8, 16, 4), 4);
        assert_eq!(depth_first_track_capacity(8, 8, 4), 2);
        assert_eq!(depth_first_track_capacity(3, 64, 4), 3);
        assert_eq!(depth_first_track_capacity(8, 0, 4), 1);
        assert_eq!(depth_first_track_capacity(8, 16, 0), 8);
        assert_eq!(DEFAULT_DEPTH_FIRST_MAX_TRACKS, 4);
    }
}
