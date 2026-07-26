//! Evidence-first, domain-agnostic deep research primitives for A3S.
//!
//! The engine owns research planning contracts, retrieval orchestration,
//! evidence admission, report quality gates, and publication artifacts.
//! Product adapters provide model, search, fetch, persistence, progress, and
//! presentation capabilities without adding topic-specific routing.

pub mod engine;
mod language;
pub mod planner;
pub mod report;
pub mod research;
pub mod workflow;

extern crate self as a3s;

#[path = "report_audit.rs"]
mod deep_research_report_audit;

mod asset_naming {
    pub(crate) fn asset_slug(name: &str) -> String {
        let mut out = String::new();
        for character in name.chars() {
            if character.is_ascii_alphanumeric() {
                out.push(character.to_ascii_lowercase());
            } else if !out.ends_with('-') {
                out.push('-');
            }
        }
        let slug = out.trim_matches('-').to_string();
        if slug.is_empty() {
            "asset".to_string()
        } else {
            slug
        }
    }
}
