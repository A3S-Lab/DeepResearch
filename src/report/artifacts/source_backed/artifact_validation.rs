fn source_backed_source_link(source: &DeepResearchCatalogSource, number: usize) -> String {
    format!("[{number}] {}", source_backed_source_title_link(source))
}

fn source_backed_source_title_link(source: &DeepResearchCatalogSource) -> String {
    let title = markdown_plain_text(&source.title);
    if source.anchor.starts_with("http://") || source.anchor.starts_with("https://") {
        format!("[{title}]({})", source.anchor)
    } else {
        format!("{title} — {}", markdown_plain_text(&source.anchor))
    }
}

fn source_backed_report_artifacts(artifacts: &ResearchReportArtifacts) -> bool {
    let markdown = read_small_utf8_file(&artifacts.markdown);
    let html = read_small_utf8_file(&artifacts.html);
    let (Some(markdown), Some(html)) = (markdown, html) else {
        return false;
    };
    deep_research_artifact_pair_has_kind(
        &markdown,
        &html,
        DeepResearchArtifactKind::SourceBacked,
    )
        && complete_html_document(&html)
        && has_research_report_substance(&markdown, &html)
}

fn no_evidence_report_artifacts(artifacts: &ResearchReportArtifacts) -> bool {
    let markdown = read_small_utf8_file(&artifacts.markdown);
    let html = read_small_utf8_file(&artifacts.html);
    let (Some(markdown), Some(html)) = (markdown, html) else {
        return false;
    };
    deep_research_artifact_pair_has_kind(
        &markdown,
        &html,
        DeepResearchArtifactKind::NoEvidence,
    )
        && complete_html_document(&html)
}

fn fenced_catalog_text(content: &str) -> String {
    let longest_run = content
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or_default();
    let fence = "`".repeat(longest_run.saturating_add(1).max(3));
    format!("{fence}\n{}\n{fence}", content.trim())
}

fn bounded_catalog_text(
    value: Option<&serde_json::Value>,
    maximum_chars: usize,
    predicate: impl Fn(&str) -> bool,
) -> Option<String> {
    let value = value?
        .as_str()?
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    (!value.is_empty() && value.chars().count() <= maximum_chars && predicate(&value))
        .then_some(value)
}

fn stable_catalog_identity(value: &str) -> bool {
    let mut characters = value.chars();
    characters
        .next()
        .is_some_and(|character| character.is_ascii_alphanumeric())
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | ':' | '-')
        })
}
