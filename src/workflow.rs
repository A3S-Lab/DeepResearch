//! Durable workflow programs owned by the DeepResearch engine.

use std::sync::OnceLock;

/// The replay-safe workflow used for exact-query bootstrap, planned retrieval,
/// source selection, and bounded typed-coverage supplementation.
pub fn retrieval_workflow_source() -> &'static str {
    static SOURCE: OnceLock<String> = OnceLock::new();
    SOURCE.get_or_init(|| {
        compact_workflow_source(concat!(
            include_str!("workflow/retrieval_foundation.js"),
            include_str!("workflow/retrieval_web.js"),
            include_str!("workflow/retrieval_source_selection.js"),
            include_str!("workflow/retrieval_web_projection.js"),
            include_str!("workflow/retrieval_selection.js"),
            include_str!("workflow/retrieval_reduction.js"),
            include_str!("workflow/retrieval_materialization.js"),
            include_str!("workflow/retrieval_gap.js"),
            include_str!("workflow/retrieval_loop.js"),
            include_str!("workflow/retrieval_local.js"),
            include_str!("workflow/retrieval_local_collection.js"),
            include_str!("workflow/retrieval_execution.js"),
        ))
    })
}

/// The replay-safe wrapper for one bounded structured generation.
pub const GENERATION_WORKFLOW_SOURCE: &str = include_str!("workflow/generation.js");

fn compact_workflow_source(source: &str) -> String {
    let mut compact = String::with_capacity(source.len());
    for line in source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        compact.push_str(line);
        if !matches!(line.as_bytes().last(), Some(b';' | b',' | b'{')) {
            compact.push('\n');
        }
    }
    compact
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retrieval_workflow_contains_the_exact_query_merge_contract() {
        let source = retrieval_workflow_source();
        assert!(source.contains("bootstrap_acquisition"));
        assert!(source.contains("plannedQueryCount"));
        assert!(source.contains("skippedBootstrapQueryCount"));
        assert!(source.contains("combineMaterializedSelections"));
        assert!(!source.contains("fallbackCandidatePriority"));
        assert!(!source.contains("accountableAlternatives"));
        assert!(!source.contains("tool output truncated:"));
        assert!(!source.contains("more fetched content available"));
        assert!(!source.contains("more lines available"));
    }
}
