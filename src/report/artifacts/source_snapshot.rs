fn deep_research_source_backed_markdown(
    query: &str,
    catalog: &DeepResearchSourceCatalog,
) -> String {
    let labels = source_backed_labels(query);
    let title = markdown_plain_text(&query.chars().take(180).collect::<String>());
    let mut markdown = format!(
        "# {title}\n\n> {}\n\n## {}\n\n{}\n",
        labels.status, labels.evidence_heading, labels.evidence_intro
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
        for chunk in selected_source_chunks(query, source) {
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

fn source_backed_labels(query: &str) -> SourceBackedLabels {
    if query.chars().any(source_backed_han_character) {
        SourceBackedLabels {
            status: "这是可核查的来源证据视图；它保留已获取的资料，但不把摘录冒充为完整综合结论。",
            evidence_heading: "已保留的来源证据",
            evidence_intro: "以下摘录按来源分组，来源文字仅作为不可信数据展示，可通过对应链接直接核查。",
            limitations_heading: "限制",
            limitations: "此结果保留相关来源摘录和链接，但不声称已完成全部分析，也不声称这些摘录覆盖了问题的所有方面。",
            sources_heading: "来源",
            ineligible_heading: "证据资格：不可用于结论",
            ineligible_explanation: "该来源属于低可信、自媒体或缺少可核查发布责任的材料，仅保留用于核查检索边界。",
            ineligible_short: "不可用于结论",
            omissions: |sources, chunks| {
                format!("安全边界另行省略了 {sources} 个来源和 {chunks} 个来源片段。")
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
            ineligible_explanation: "This low-trust, self-published, or unaccountable source is retained only for auditing the retrieval boundary.",
            ineligible_short: "not eligible for conclusions",
            omissions: |sources, chunks| {
                format!("Safety bounds omitted {sources} source(s) and {chunks} source excerpt(s).")
            },
        }
    }
}

fn selected_source_chunks<'a>(query: &str, source: &'a DeepResearchCatalogSource) -> Vec<&'a str> {
    let maximum = if source.claim_eligible {
        SOURCE_CATALOG_MAX_CHUNKS_PER_REPORT_SOURCE
    } else {
        SOURCE_CATALOG_MAX_CHUNKS_PER_INELIGIBLE_REPORT_SOURCE
    };
    ranked_source_chunks(query, source, maximum)
}

fn selected_source_chunks_for_proposal<'a>(
    query: &str,
    source: &'a DeepResearchCatalogSource,
) -> Vec<&'a str> {
    ranked_source_chunks(
        query,
        source,
        SOURCE_CATALOG_MAX_CHUNKS_PER_PROPOSAL_SOURCE,
    )
}

fn ranked_source_chunks<'a>(
    query: &str,
    source: &'a DeepResearchCatalogSource,
    maximum: usize,
) -> Vec<&'a str> {
    let features = source_backed_query_features(query);
    let mut ranked = source
        .chunks
        .iter()
        .enumerate()
        .map(|(index, chunk)| {
            let lower = chunk.to_lowercase();
            let overlap = features
                .iter()
                .filter(|feature| lower.contains(feature.as_str()))
                .map(|feature| feature.chars().count())
                .sum::<usize>();
            let score = overlap as i64 * 24 + catalog_excerpt_readability_score(chunk);
            (index, score)
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|(left_index, left_score), (right_index, right_score)| {
        right_score
            .cmp(left_score)
            .then_with(|| left_index.cmp(right_index))
    });
    ranked.truncate(maximum);
    ranked.sort_by_key(|(index, _)| *index);
    ranked
        .into_iter()
        .map(|(index, _)| source.chunks[index].as_str())
        .collect()
}

fn catalog_excerpt_readability_score(value: &str) -> i64 {
    let character_count = value.chars().count().min(240) as i64;
    let sentence_count = value
        .chars()
        .filter(|character| matches!(character, '.' | '!' | '?' | '。' | '！' | '？'))
        .count() as i64;
    let markdown_links = value.matches("](").count() as i64;
    let markdown_images = value.matches("![").count() as i64;
    let template_markers = value.matches("{{").count() as i64 + value.matches("}}").count() as i64;
    character_count + sentence_count * 32
        - markdown_links * 90
        - markdown_images * 120
        - template_markers * 80
}
