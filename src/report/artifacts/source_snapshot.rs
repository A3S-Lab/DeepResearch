fn deep_research_source_backed_markdown(
    query: &str,
    catalog: &DeepResearchSourceCatalog,
) -> String {
    let labels = source_backed_labels();
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

fn source_backed_labels() -> SourceBackedLabels {
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
