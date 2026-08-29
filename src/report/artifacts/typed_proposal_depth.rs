fn typed_sources_have_independent_attribution<'a>(
    attribution: Option<&DeepResearchSourceAttribution>,
    source_aliases: impl IntoIterator<Item = &'a str>,
) -> bool {
    let source_aliases = source_aliases.into_iter().collect::<Vec<_>>();
    match attribution {
        Some(attribution) => {
            attribution.has_verified_independent_pair(source_aliases.iter().copied())
        }
        None => source_aliases.into_iter().collect::<HashSet<_>>().len() >= 2,
    }
}

fn typed_analytical_quality(
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> (usize, usize) {
    let analytical_claims = compiled.claim_support.iter().filter(|claim| {
        matches!(
            claim.kind,
            crate::research::compiler::CompilerClaimKind::Inference
                | crate::research::compiler::CompilerClaimKind::Recommendation
        )
    });
    let analytical_claim_count = analytical_claims.clone().count();
    let cross_source_synthesis_count = analytical_claims
        .filter(|claim| {
            claim.analysis_role
                == Some(crate::research::compiler::CompilerAnalysisRole::Comparison)
                && claim.basis_claim_ids.len() >= 2
                && typed_sources_have_independent_attribution(
                    attribution,
                    claim.source_ids.iter().map(String::as_str),
                )
        })
        .count();
    (analytical_claim_count, cross_source_synthesis_count)
}

fn typed_dimension_depth_quality(
    context: &DeepResearchReportContext,
    catalog: &DeepResearchSourceCatalog,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> TypedDimensionDepthQuality {
    if context.scope != DeepResearchReportScope::Comprehensive {
        return TypedDimensionDepthQuality::default();
    }

    let mut quality = TypedDimensionDepthQuality::default();
    for track in context.tracks.iter().filter(|track| {
        track
            .get("material")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    }) {
        let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let deeply_analyzed = typed_compiled_dimension_has_required_depth(
            dimension_id,
            attribution,
            compiled,
        );
        if typed_compiled_dimension_is_bounded(dimension_id, compiled) {
            continue;
        }
        if !typed_track_is_resolved_by_claim_support(
            track,
            catalog,
            attribution,
            compiled,
        ) {
            continue;
        }
        quality.resolved_material_dimension_count += 1;
        if deeply_analyzed {
            quality.deeply_analyzed_dimension_count += 1;
            quality.deeply_analyzed_resolved_dimension_count += 1;
        }
    }
    quality.deeply_analyzed_dimension_count = quality.deeply_analyzed_resolved_dimension_count;
    quality
}

fn typed_compiled_dimension_is_bounded(
    dimension_id: &str,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> bool {
    compiled.coverage.iter().any(|coverage| {
        coverage.dimension_id == dimension_id
            && matches!(
                coverage.status,
                crate::research::compiler::CompilerStructuralCoverage::ClaimsAndGap
                    | crate::research::compiler::CompilerStructuralCoverage::GapOnly
            )
    })
}

fn typed_compiled_dimension_has_required_depth(
    dimension_id: &str,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> bool {
    typed_compiled_dimension_depth_metrics(dimension_id, attribution, compiled)
        .satisfies_comprehensive_contract()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct TypedDimensionDepthMetrics {
    has_conclusion: bool,
    evidence_finding_count: usize,
    comparison_count: usize,
    explanation_count: usize,
    implication_count: usize,
    challenge_or_boundary_count: usize,
    independently_attributable_source_count: usize,
    has_independently_attributable_source_pair: bool,
    cross_source_synthesis_count: usize,
    has_integrated_implication: bool,
    substantive_character_count: usize,
}

impl TypedDimensionDepthMetrics {
    fn satisfies_comprehensive_contract(self) -> bool {
        self.has_conclusion
            && self.evidence_finding_count >= COMPREHENSIVE_DIMENSION_MIN_FACT_FINDINGS
            && self.comparison_count >= COMPREHENSIVE_DIMENSION_MIN_COMPARISONS
            && self.explanation_count >= COMPREHENSIVE_DIMENSION_MIN_EXPLANATIONS
            && self.implication_count >= COMPREHENSIVE_DIMENSION_MIN_IMPLICATIONS
            && self.challenge_or_boundary_count
                >= COMPREHENSIVE_DIMENSION_MIN_CHALLENGES_OR_BOUNDARIES
            && self.independently_attributable_source_count
                >= COMPREHENSIVE_DIMENSION_MIN_SOURCES
            && self.cross_source_synthesis_count
                >= COMPREHENSIVE_DIMENSION_MIN_CROSS_SOURCE_SYNTHESES
            && self.has_integrated_implication
            && self.substantive_character_count
                >= COMPREHENSIVE_DIMENSION_MIN_SUBSTANTIVE_CHARACTERS
    }

    fn has_partial_analysis_opportunity(self) -> bool {
        self.evidence_finding_count >= COMPREHENSIVE_PARTIAL_DIMENSION_MIN_FACT_FINDINGS
    }
}

fn typed_compiled_dimension_depth_metrics(
    dimension_id: &str,
    attribution: Option<&DeepResearchSourceAttribution>,
    compiled: &crate::research::compiler::CompiledEvidenceReport,
) -> TypedDimensionDepthMetrics {
    let claims = compiled
        .claim_support
        .iter()
        .filter(|claim| claim.dimension_id == dimension_id)
        .collect::<Vec<_>>();
    let has_conclusion = claims.iter().any(|claim| {
        claim.placement == crate::research::compiler::CompilerClaimPlacement::DirectAnswer
            && claim.analysis_role
                == Some(crate::research::compiler::CompilerAnalysisRole::Conclusion)
    });
    let evidence_finding_count = claims
        .iter()
        .filter(|claim| {
            claim.placement == crate::research::compiler::CompilerClaimPlacement::Finding
                && claim.kind == crate::research::compiler::CompilerClaimKind::Fact
                && claim.analysis_role
                    == Some(crate::research::compiler::CompilerAnalysisRole::Evidence)
        })
        .count();
    let role_count = |roles: &[crate::research::compiler::CompilerAnalysisRole]| {
        claims
            .iter()
            .filter(|claim| {
                claim
                    .analysis_role
                    .is_some_and(|role| roles.contains(&role))
            })
            .count()
    };
    let factual_source_aliases = claims
        .iter()
        .filter(|claim| claim.kind == crate::research::compiler::CompilerClaimKind::Fact)
        .flat_map(|claim| claim.source_ids.iter())
        .map(String::as_str)
        .collect::<Vec<_>>();
    let has_independently_attributable_source_pair =
        typed_sources_have_independent_attribution(
            attribution,
            factual_source_aliases.iter().copied(),
        );
    let independently_attributable_source_count = match attribution {
        Some(attribution) => attribution
            .independently_attributable_group_count(factual_source_aliases.iter().copied()),
        None => factual_source_aliases
            .iter()
            .copied()
            .collect::<HashSet<_>>()
            .len(),
    };
    let cross_source_synthesis_count = claims
        .iter()
        .filter(|claim| {
            claim.analysis_role
                == Some(crate::research::compiler::CompilerAnalysisRole::Comparison)
                && claim.basis_claim_ids.len() >= 2
                && typed_sources_have_independent_attribution(
                    attribution,
                    claim.source_ids.iter().map(String::as_str),
                )
        })
        .count();
    let claims_by_id = claims
        .iter()
        .copied()
        .map(|claim| (claim.claim_id.as_str(), claim))
        .collect::<std::collections::HashMap<_, _>>();
    let has_integrated_implication = claims.iter().any(|claim| {
        claim.analysis_role
            == Some(crate::research::compiler::CompilerAnalysisRole::Implication)
            && typed_claim_has_ancestor_role(
                claim,
                &claims_by_id,
                crate::research::compiler::CompilerAnalysisRole::Comparison,
            )
            && typed_claim_has_ancestor_role(
                claim,
                &claims_by_id,
                crate::research::compiler::CompilerAnalysisRole::Explanation,
            )
    });
    let substantive_character_count = claims
        .iter()
        .map(|claim| claim.substantive_character_count)
        .sum::<usize>();

    TypedDimensionDepthMetrics {
        has_conclusion,
        evidence_finding_count,
        comparison_count: role_count(&[
            crate::research::compiler::CompilerAnalysisRole::Comparison,
        ]),
        explanation_count: role_count(&[
            crate::research::compiler::CompilerAnalysisRole::Explanation,
        ]),
        implication_count: role_count(&[
            crate::research::compiler::CompilerAnalysisRole::Implication,
        ]),
        challenge_or_boundary_count: role_count(&[
            crate::research::compiler::CompilerAnalysisRole::Challenge,
            crate::research::compiler::CompilerAnalysisRole::Boundary,
        ]),
        independently_attributable_source_count,
        has_independently_attributable_source_pair,
        cross_source_synthesis_count,
        has_integrated_implication,
        substantive_character_count,
    }
}

fn typed_claim_has_ancestor_role(
    claim: &crate::research::compiler::CompilerClaimSupport,
    claims_by_id: &std::collections::HashMap<
        &str,
        &crate::research::compiler::CompilerClaimSupport,
    >,
    required_role: crate::research::compiler::CompilerAnalysisRole,
) -> bool {
    let mut pending = claim.basis_claim_ids.clone();
    let mut visited = HashSet::<String>::new();
    while let Some(claim_id) = pending.pop() {
        if !visited.insert(claim_id.clone()) {
            continue;
        }
        let Some(ancestor) = claims_by_id.get(claim_id.as_str()).copied() else {
            continue;
        };
        if ancestor.dimension_id != claim.dimension_id {
            continue;
        }
        if ancestor.analysis_role == Some(required_role) {
            return true;
        }
        pending.extend(ancestor.basis_claim_ids.iter().cloned());
    }
    false
}
