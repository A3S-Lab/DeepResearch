use super::*;

pub(super) fn reader_labels(language: &str) -> ReaderLabels {
    let values = if language == "zh" {
        [
            "报告章节",
            "跳转到报告",
            "直接结论",
            "研究维度",
            "来源",
            "覆盖状态",
            "研究发现",
            "证据边界",
            "保留的来源摘录",
            "矛盾",
            "推断",
            "建议",
            "依据",
            "推导",
            "结论",
            "获取时间",
            "原始请求地址",
            "来源保全报告",
            "未获取到可核查来源",
            "该维度尚无可核查的答案；保留的来源摘录仅供核验，不附加解释。",
            "本次运行未能获取可核查的来源，因此无法为该维度给出有证据支持的结论。",
            "已有引用结论",
            "部分回答并保留证据边界",
            "仅保留证据边界",
            "尚未回答",
        ]
    } else {
        [
            "Report sections",
            "Skip to report",
            "Direct Answer",
            "Research Dimensions",
            "Sources",
            "Coverage",
            "Findings",
            "Evidence Boundaries",
            "Retained Source Excerpts",
            "Contradiction",
            "Inference",
            "Recommendation",
            "Basis",
            "Derivation",
            "finding",
            "Captured",
            "Requested as",
            "Source-backed report",
            "No verifiable source was acquired",
            "A verified answer is not available for this dimension. The retained source excerpts are provided without additional interpretation.",
            "This run acquired no verifiable source, so no evidence-backed conclusion is available for this dimension.",
            "Addressed with cited findings",
            "Partially addressed with an explicit boundary",
            "Bounded by the retained evidence",
            "Not addressed",
        ]
    };
    ReaderLabels {
        report_sections: values[0].to_string(),
        skip_to_report: values[1].to_string(),
        direct_answer: values[2].to_string(),
        research_dimensions: values[3].to_string(),
        sources: values[4].to_string(),
        status: values[5].to_string(),
        findings: values[6].to_string(),
        limitations: values[7].to_string(),
        retained_excerpts: values[8].to_string(),
        contradiction: values[9].to_string(),
        inference: values[10].to_string(),
        recommendation: values[11].to_string(),
        basis: values[12].to_string(),
        derivation: values[13].to_string(),
        finding: values[14].to_string(),
        captured: values[15].to_string(),
        requested_as: values[16].to_string(),
        source_backed: values[17].to_string(),
        no_evidence: values[18].to_string(),
        source_backed_gap: values[19].to_string(),
        no_evidence_gap: values[20].to_string(),
        coverage_claims: values[21].to_string(),
        coverage_partial: values[22].to_string(),
        coverage_bounded: values[23].to_string(),
        coverage_missing: values[24].to_string(),
    }
}

pub(super) fn budget(max_queries: usize, max_fetches: usize) -> ResearchBudget {
    ResearchBudget {
        max_queries,
        max_fetches,
    }
}

pub(super) fn dimension(id: &str, target_ids: &[&str]) -> ResearchDimension {
    ResearchDimension {
        id: id.to_string(),
        question: format!("What establishes {id}?"),
        material: true,
        source_target_ids: target_ids.iter().map(|id| (*id).to_string()).collect(),
    }
}

pub(super) fn named_target(
    id: &str,
    family: &str,
    role: SourceRole,
    identity: SourceIdentity,
) -> SourceTarget {
    let transport = identity.transport();
    SourceTarget {
        id: id.to_string(),
        source_family_id: family.to_string(),
        role,
        transport,
        match_policy: TargetMatchPolicy::Named { identity },
    }
}

pub(super) fn exploratory_target(id: &str, family: &str, goal: &str) -> SourceTarget {
    SourceTarget {
        id: id.to_string(),
        source_family_id: family.to_string(),
        role: SourceRole::Independent,
        transport: AcquisitionTransport::Web,
        match_policy: TargetMatchPolicy::Exploratory {
            selection_goal: goal.to_string(),
        },
    }
}

pub(super) fn spec(
    scope: EvidenceScope,
    dimensions: Vec<ResearchDimension>,
    source_targets: Vec<SourceTarget>,
    budget: ResearchBudget,
) -> ResearchSpec {
    ResearchSpec {
        version: 3,
        query: "Evaluate the requested decision from traceable evidence.".to_string(),
        language: "en".to_string(),
        reader_labels: reader_labels("en"),
        current_date: "2026-07-21".to_string(),
        evidence_scope: scope,
        dimensions,
        source_targets,
        budget,
    }
}

pub(super) fn query(
    id: &str,
    transport: AcquisitionTransport,
    mode: QueryMode,
    dimensions: &[&str],
    targets: &[&str],
    fetch_slots: usize,
) -> ResearchQuery {
    ResearchQuery {
        id: id.to_string(),
        text: format!("source-seeking query for {id}"),
        transport,
        mode,
        dimension_ids: dimensions
            .iter()
            .map(|dimension| (*dimension).to_string())
            .collect(),
        source_target_ids: targets.iter().map(|target| (*target).to_string()).collect(),
        fetch_slots,
    }
}

pub(super) fn plan(
    spec: &ResearchSpec,
    queries: Vec<ResearchQuery>,
    gaps: Vec<PlanningGap>,
) -> QueryPlan {
    QueryPlan {
        spec_digest: research_spec_digest(spec),
        queries,
        planning_gaps: gaps,
    }
}
