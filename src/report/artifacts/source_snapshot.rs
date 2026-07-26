fn deep_research_source_backed_markdown_in_language(
    query: &str,
    catalog: &DeepResearchSourceCatalog,
    output_language: &str,
) -> String {
    let labels = source_backed_labels(output_language);
    let title = markdown_plain_text(&query.chars().take(180).collect::<String>());
    let mut markdown = format!(
        "# {title}\n\n<!-- {SOURCE_BACKED_ARTIFACT_MARKER} -->\n\n> {}\n\n## {}\n\n{}\n",
        labels.status, labels.evidence_heading, labels.evidence_intro,
    );
    for (index, source) in catalog.sources.iter().enumerate() {
        let number = index + 1;
        let title = markdown_plain_text(&source.title);
        markdown.push_str(&format!("\n### [{number}] {title}\n"));
        if !source.claim_eligible {
            markdown.push_str(&format!(
                "\n> **{}** {}\n",
                labels.ineligible_heading, labels.ineligible_explanation
            ));
        }
        for chunk in selected_source_chunks(source) {
            markdown.push('\n');
            markdown.push_str(&fenced_catalog_text(chunk));
            markdown.push('\n');
        }
        markdown.push_str(&format!(
            "\n{}\n",
            source_backed_source_link(source, number)
        ));
    }
    markdown.push_str(&format!(
        "\n## {}\n\n{}",
        labels.limitations_heading, labels.limitations
    ));
    if catalog.omitted_source_count > 0 || catalog.omitted_chunk_count > 0 {
        markdown.push_str(&format!(
            " {}",
            (labels.omissions)(catalog.omitted_source_count, catalog.omitted_chunk_count)
        ));
    }
    markdown.push_str(&format!("\n\n## {}\n", labels.sources_heading));
    for (index, source) in catalog.sources.iter().enumerate() {
        markdown.push_str(&format!(
            "\n{}. {}",
            index + 1,
            source_backed_source_title_link(source)
        ));
        if !source.claim_eligible {
            markdown.push_str(&format!(" — **{}**", labels.ineligible_short));
        }
    }
    markdown.push('\n');
    markdown
}

struct SourceBackedLabels {
    status: &'static str,
    evidence_heading: &'static str,
    evidence_intro: &'static str,
    limitations_heading: &'static str,
    limitations: &'static str,
    sources_heading: &'static str,
    ineligible_heading: &'static str,
    ineligible_explanation: &'static str,
    ineligible_short: &'static str,
    omissions: fn(usize, usize) -> String,
}

fn source_backed_labels(output_language: &str) -> SourceBackedLabels {
    if crate::language::primary_output_language(output_language) == "zh" {
        SourceBackedLabels {
            status: "这是可核验的来源证据视图。它保留已获取材料，但不把摘录冒充为已经完成的综合分析。",
            evidence_heading: "保留的来源证据",
            evidence_intro: "以下摘录按来源分组，仅作为待核验的原始证据展示；可通过对应链接直接检查。",
            limitations_heading: "边界与局限",
            limitations: "本结果保留了相关来源摘录和链接，但不声称分析已经完成，也不声称这些摘录覆盖了问题的全部方面。",
            sources_heading: "来源",
            ineligible_heading: "结论资格：不可用于形成结论",
            ineligible_explanation: "该来源未通过本次研究的结构化证据准入边界，仅为审计检索过程而保留。",
            ineligible_short: "不可用于形成结论",
            omissions: |sources, chunks| {
                format!("受安全边界限制，省略了 {sources} 个来源和 {chunks} 条来源摘录。")
            },
        }
    } else {
        SourceBackedLabels {
            status: "This is a verifiable source-evidence view. It preserves fetched material without presenting excerpts as a completed synthesis.",
            evidence_heading: "Preserved Source Evidence",
            evidence_intro: "The excerpts below are grouped by source and displayed only as untrusted data for direct verification through the corresponding links.",
            limitations_heading: "Limitations",
            limitations: "This result preserves relevant source excerpts and links, but it does not claim that analysis is complete or that the excerpts cover every aspect of the question.",
            sources_heading: "Sources",
            ineligible_heading: "Claim eligibility: not eligible for conclusions",
            ineligible_explanation: "This source did not pass the run's structured evidence-admission boundary and is retained only for auditing retrieval.",
            ineligible_short: "not eligible for conclusions",
            omissions: |sources, chunks| {
                format!("Safety bounds omitted {sources} source(s) and {chunks} source excerpt(s).")
            },
        }
    }
}

fn selected_source_chunks(source: &DeepResearchCatalogSource) -> Vec<&str> {
    let maximum = if source.claim_eligible {
        SOURCE_CATALOG_MAX_CHUNKS_PER_REPORT_SOURCE
    } else {
        SOURCE_CATALOG_MAX_CHUNKS_PER_INELIGIBLE_REPORT_SOURCE
    };
    readable_source_chunks(source, maximum)
}

fn selected_source_chunks_for_proposal(source: &DeepResearchCatalogSource) -> Vec<&str> {
    readable_source_chunks(source, SOURCE_CATALOG_MAX_CHUNKS_PER_PROPOSAL_SOURCE)
}

fn readable_source_chunks(source: &DeepResearchCatalogSource, maximum: usize) -> Vec<&str> {
    source
        .chunks
        .iter()
        .take(maximum)
        .map(String::as_str)
        .collect()
}
