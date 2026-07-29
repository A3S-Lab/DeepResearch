struct ReportTrackCoverageState {
    track_id: String,
    resolved_criterion_indexes: Vec<usize>,
    unsupported_criterion_indexes: Vec<usize>,
    missing_primary_source_criterion_indexes: Vec<usize>,
    missing_independent_corroboration_criterion_indexes: Vec<usize>,
}

impl ReportTrackCoverageState {
    fn is_resolved(&self, criterion_count: usize) -> bool {
        self.resolved_criterion_indexes.len() == criterion_count
    }
}

fn report_track_coverage_state(
    track: &serde_json::Value,
    catalog: &DeepResearchSourceCatalog,
    source_indexes: &HashSet<usize>,
) -> Option<ReportTrackCoverageState> {
    report_track_coverage_state_with_attribution(track, catalog, source_indexes, None)
}

fn report_track_coverage_state_with_attribution(
    track: &serde_json::Value,
    catalog: &DeepResearchSourceCatalog,
    source_indexes: &HashSet<usize>,
    attribution: Option<&DeepResearchSourceAttribution>,
) -> Option<ReportTrackCoverageState> {
    let track_id = track.get("id")?.as_str()?;
    let criteria = track
        .get("completion_criteria")?
        .as_array()
        .filter(|criteria| !criteria.is_empty())?;
    let requirements = track.get("evidence_requirements")?.as_object()?;
    let primary_required = requirements.get("primary_source_required")?.as_bool()?;
    let independent_required = requirements
        .get("independent_corroboration_required")?
        .as_bool()?;
    let mut state = ReportTrackCoverageState {
        track_id: track_id.to_string(),
        resolved_criterion_indexes: Vec::new(),
        unsupported_criterion_indexes: Vec::new(),
        missing_primary_source_criterion_indexes: Vec::new(),
        missing_independent_corroboration_criterion_indexes: Vec::new(),
    };
    for criterion_index in 0..criteria.len() {
        let bindings = source_indexes
            .iter()
            .filter_map(|source_index| {
                catalog
                    .sources
                    .get(*source_index)
                    .map(|source| (*source_index, source))
            })
            .flat_map(|(source_index, source)| {
                source
                    .coverage
                    .iter()
                    .filter(move |binding| {
                        binding.track_id == track_id
                            && binding
                                .completion_criterion_indexes
                                .contains(&criterion_index)
                    })
                    .map(move |binding| (source_index, binding))
            })
            .collect::<Vec<_>>();
        let covered_sources = bindings
            .iter()
            .map(|(source_index, _)| *source_index)
            .collect::<HashSet<_>>();
        let primary_sources = bindings
            .iter()
            .filter(|(_, binding)| binding.primary)
            .map(|(source_index, _)| *source_index)
            .collect::<HashSet<_>>();
        let independent_sources = bindings
            .iter()
            .filter(|(_, binding)| binding.independent)
            .map(|(source_index, _)| *source_index)
            .collect::<HashSet<_>>();
        let supported = !covered_sources.is_empty();
        let primary_satisfied = !primary_required || !primary_sources.is_empty();
        let legacy_independent_satisfied = if !independent_required {
            true
        } else if primary_required {
            primary_sources.iter().any(|primary| {
                independent_sources
                    .iter()
                    .any(|independent| independent != primary)
            })
        } else {
            !independent_sources.is_empty() && covered_sources.len() >= 2
        };
        let attributed_independent_satisfied = attribution.map(|attribution| {
            let source_aliases = |indexes: &HashSet<usize>| {
                indexes
                    .iter()
                    .filter_map(|source_index| catalog.sources.get(*source_index))
                    .map(|source| source.alias.as_str())
                    .collect::<Vec<_>>()
            };
            if !independent_required {
                true
            } else if primary_required {
                attribution.has_verified_independent_pair_between(
                    source_aliases(&primary_sources),
                    source_aliases(&independent_sources),
                )
            } else {
                attribution.has_verified_independent_pair_between(
                    source_aliases(&covered_sources),
                    source_aliases(&independent_sources),
                )
            }
        });
        let independent_satisfied =
            attributed_independent_satisfied.unwrap_or(legacy_independent_satisfied);
        if !supported {
            state.unsupported_criterion_indexes.push(criterion_index);
        }
        if primary_required && !primary_satisfied {
            state
                .missing_primary_source_criterion_indexes
                .push(criterion_index);
        }
        if independent_required && !independent_satisfied {
            state
                .missing_independent_corroboration_criterion_indexes
                .push(criterion_index);
        }
        if supported && primary_satisfied && independent_satisfied {
            state.resolved_criterion_indexes.push(criterion_index);
        }
    }
    Some(state)
}
