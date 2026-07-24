fn research_report_artifacts_from_output_with_slug(
    output: &str,
    workspace: &Path,
    expected_slug: Option<&str>,
) -> Option<ResearchReportArtifacts> {
    output.lines().rev().find_map(|line| {
        let marker_at = line.find(RESEARCH_VIEW_MARKER)?;
        let raw = &line[marker_at + RESEARCH_VIEW_MARKER.len()..];
        let candidate = clean_research_report_marker_value(raw)?;
        let artifacts = trusted_research_report_artifacts(&candidate, workspace)?;
        match expected_slug {
            Some(slug) if !research_report_artifact_slug_matches(&artifacts, slug) => None,
            _ => Some(artifacts),
        }
    })
}
fn research_report_artifact_slug_matches(
    artifacts: &ResearchReportArtifacts,
    expected_slug: &str,
) -> bool {
    artifacts
        .html
        .parent()
        .and_then(Path::file_name)
        .and_then(|value| value.to_str())
        == Some(expected_slug)
}
fn clean_research_report_marker_value(raw: &str) -> Option<String> {
    let mut value = raw.trim();
    value = value
        .trim_start_matches(['`', '"', '\'', '<'])
        .trim_end_matches(['`', '"', '\'', '>', '.', ',', ';']);
    if value.is_empty() || value.starts_with("file://") {
        return None;
    }
    value
        .split_whitespace()
        .next()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn trusted_research_report_artifacts(
    candidate: &str,
    workspace: &Path,
) -> Option<ResearchReportArtifacts> {
    let artifacts = trusted_research_report_artifact_paths(candidate, workspace)?;
    completed_research_report_artifacts(&artifacts).then_some(artifacts)
}

fn trusted_research_report_artifact_paths(
    candidate: &str,
    workspace: &Path,
) -> Option<ResearchReportArtifacts> {
    let root = workspace.canonicalize().ok()?;
    let candidate = Path::new(candidate);
    let unresolved_path = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        root.join(candidate)
    };
    if unresolved_path.file_name() != Some(std::ffi::OsStr::new("index.html")) {
        return None;
    }
    let unresolved_report_dir = unresolved_path.parent()?;
    let report_dir_metadata = std::fs::symlink_metadata(unresolved_report_dir).ok()?;
    if report_dir_metadata.file_type().is_symlink() || !report_dir_metadata.is_dir() {
        return None;
    }
    let report_dir = unresolved_report_dir.canonicalize().ok()?;
    let rel = report_dir.strip_prefix(&root).ok()?;
    let mut components = rel.components();
    let first = components.next()?.as_os_str();
    let second = components.next()?.as_os_str();
    let slug = components.next()?.as_os_str();
    if components.next().is_some() {
        return None;
    }
    if first != std::ffi::OsStr::new(".a3s") || second != std::ffi::OsStr::new("research") {
        return None;
    }
    if slug.is_empty() {
        return None;
    }
    let html_path = report_dir.join("index.html");
    let markdown_path = report_dir.join("report.md");
    recover_research_report_pair(&markdown_path, &html_path).ok()?;

    let html_metadata = std::fs::symlink_metadata(&html_path).ok()?;
    let markdown_metadata = std::fs::symlink_metadata(&markdown_path).ok()?;
    if html_metadata.file_type().is_symlink()
        || !html_metadata.is_file()
        || markdown_metadata.file_type().is_symlink()
        || !markdown_metadata.is_file()
    {
        return None;
    }
    let html = html_path.canonicalize().ok()?;
    let markdown = markdown_path.canonicalize().ok()?;
    if !is_nonempty_file(&html)
        || !is_nonempty_file(&markdown)
        || html.parent() != Some(report_dir.as_path())
        || markdown.parent() != Some(report_dir.as_path())
        || !is_html_path(&html)
        || markdown.file_name() != Some(std::ffi::OsStr::new("report.md"))
    {
        return None;
    }
    Some(ResearchReportArtifacts {
        markdown,
        html,
    })
}

fn completed_research_report_artifacts(artifacts: &ResearchReportArtifacts) -> bool {
    let markdown = read_small_utf8_file(&artifacts.markdown);
    let html = read_small_utf8_file(&artifacts.html);
    let (Some(markdown), Some(html)) = (markdown, html) else {
        return false;
    };
    deep_research_artifact_pair_has_kind(
        &markdown,
        &html,
        DeepResearchArtifactKind::Synthesized,
    )
        && complete_html_document(&html)
        && has_research_report_substance(&markdown, &html)
}

fn deep_research_report_sources_trace_workflow(
    artifacts: &ResearchReportArtifacts,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> bool {
    let markdown = read_small_utf8_file(&artifacts.markdown);
    let html = read_small_utf8_file(&artifacts.html);
    let (Some(markdown), Some(html)) = (markdown, html) else {
        return false;
    };

    deep_research_report_content_sources_trace_workflow(
        &markdown,
        &html,
        query,
        workflow_output,
        workflow_metadata,
    )
}

fn deep_research_report_content_sources_trace_workflow(
    markdown: &str,
    html: &str,
    query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> bool {
    deep_research_report_source_trace_diagnostic(
        markdown,
        html,
        query,
        workflow_output,
        workflow_metadata,
    )
    .is_ok()
}

fn deep_research_report_source_trace_diagnostic(
    markdown: &str,
    html: &str,
    _query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> Result<(), String> {
    let anchors = deep_research_workflow_source_anchors(workflow_output, workflow_metadata);
    if anchors.is_empty() {
        // A DeepResearch report is only "completed" when it can be traced to
        // evidence gathered by this run. Failing open here lets a polished
        // model answer—or an old deterministic-slug report—mask a collection
        // failure that captured no source at all. Callers will materialize an
        // explicit recovery report instead.
        return Err(
            "source trace rejected: the workflow captured no traceable research sources"
                .to_string(),
        );
    }

    let sources = anchors
        .iter()
        .enumerate()
        .map(
            |(index, anchor)| super::deep_research_report_audit::ReportSourceReference {
                source_id: format!("source:{index}"),
                anchor: anchor.clone(),
            },
        )
        .collect::<Vec<_>>();
    let audit = super::deep_research_report_audit::audit_report(
        markdown,
        html,
        &sources,
        super::deep_research_report_audit::CitationRequirement::AtLeastOne,
    );
    if !audit.passed {
        return Err(format!("source trace rejected: {}", audit.reason));
    }

    let observed = anchors
        .iter()
        .filter_map(|anchor| super::deep_research_report_audit::canonical_citation_target(anchor))
        .collect::<HashSet<_>>();
    let mut unmatched = super::deep_research_report_audit::report_citation_targets(markdown, html)
        .into_iter()
        .filter(|citation| {
            !citation.starts_with('#')
                && !citation.starts_with("mailto:")
                && !observed.contains(citation)
        })
        .collect::<Vec<_>>();
    unmatched.sort();
    unmatched.dedup();
    if unmatched.is_empty() {
        return Ok(());
    }

    let displayed = unmatched.iter().take(8).cloned().collect::<Vec<_>>();
    let omitted = unmatched.len().saturating_sub(displayed.len());
    let mut message = format!(
        "source trace rejected: {} citation{} were not observed in this run: {}",
        unmatched.len(),
        if unmatched.len() == 1 { "" } else { "s" },
        displayed.join(", ")
    );
    if omitted > 0 {
        message.push_str(&format!(", plus {omitted} more"));
    }
    Err(message)
}

fn sanitize_unobserved_markdown_http_citations(
    markdown: &str,
    _query: &str,
    workflow_output: &str,
    workflow_metadata: Option<&serde_json::Value>,
) -> String {
    let observed = deep_research_workflow_source_anchors(workflow_output, workflow_metadata)
        .into_iter()
        .filter_map(|anchor| super::deep_research_report_audit::canonical_citation_target(&anchor))
        .collect::<HashSet<_>>();
    if observed.is_empty() {
        return markdown.to_string();
    }

    let unobserved = super::deep_research_report_audit::report_citation_targets(markdown, "")
        .into_iter()
        .filter(|citation| citation.starts_with("http://") || citation.starts_with("https://"))
        .filter(|citation| !observed.contains(citation))
        .collect::<HashSet<_>>();
    if unobserved.is_empty() {
        return markdown.to_string();
    }

    let invalid_targets = http_source_targets(markdown)
        .into_iter()
        .filter(|target| {
            super::deep_research_report_audit::canonical_citation_target(target)
                .is_some_and(|target| unobserved.contains(&target))
        })
        .collect::<HashSet<_>>();
    if invalid_targets.is_empty() {
        return markdown.to_string();
    }

    let mut cleaned = Vec::new();
    for line in markdown.lines() {
        if line.trim_start().starts_with("# ") {
            cleaned.push(line.to_string());
        } else {
            cleaned.push(strip_unobserved_http_targets(line, &invalid_targets));
        }
    }

    let mut output = cleaned.join("\n");
    if markdown.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn strip_unobserved_http_targets(line: &str, invalid_targets: &HashSet<String>) -> String {
    let mut output = line.to_string();
    let mut targets = invalid_targets.iter().collect::<Vec<_>>();
    targets.sort_by_key(|target| std::cmp::Reverse(target.len()));
    for target in targets {
        let mut cursor = 0;
        while let Some(offset) = output[cursor..].find(target.as_str()) {
            let start = cursor + offset;
            let end = start + target.len();
            if let Some((link_start, link_end, label)) =
                markdown_link_replacement(&output, start, end)
            {
                output.replace_range(link_start..link_end, &label);
                cursor = link_start + label.len();
            } else {
                output.replace_range(start..end, "");
                cursor = start;
            }
        }
    }
    output
}

fn markdown_link_replacement(
    text: &str,
    target_start: usize,
    target_end: usize,
) -> Option<(usize, usize, String)> {
    if target_start < 2 || text.get(target_start - 2..target_start)? != "](" {
        return None;
    }
    let label_start = text[..target_start - 2].rfind('[')?;
    if text[label_start + 1..target_start - 2]
        .chars()
        .any(|ch| matches!(ch, '\n' | '\r'))
    {
        return None;
    }
    let suffix = &text[target_end..];
    let close_offset = suffix.find(')')?;
    if suffix[..close_offset]
        .chars()
        .any(|ch| matches!(ch, '\n' | '\r'))
    {
        return None;
    }
    let link_end = target_end + close_offset + 1;
    let label = text[label_start + 1..target_start - 2].to_string();
    Some((label_start, link_end, label))
}

fn http_source_targets(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut cursor = 0;
    let mut targets = Vec::new();
    while cursor < text.len() {
        let next = ["http://", "https://"]
            .into_iter()
            .filter_map(|prefix| lower[cursor..].find(prefix).map(|index| cursor + index))
            .min();
        let Some(start) = next else {
            break;
        };
        let mut nested_parentheses = 0usize;
        let end = text[start..]
            .char_indices()
            .find_map(|(offset, ch)| {
                if ch.is_whitespace() || matches!(ch, '<' | '>' | '"' | '\'' | '`' | ']' | '}') {
                    return Some(start + offset);
                }
                match ch {
                    '(' => nested_parentheses += 1,
                    ')' if nested_parentheses == 0 => return Some(start + offset),
                    ')' => nested_parentheses -= 1,
                    _ => {}
                }
                None
            })
            .unwrap_or(text.len());
        let candidate =
            text[start..end].trim_end_matches(['.', ',', ';', ':', '!', '?', '*', '_', '~']);
        targets.push(candidate.to_string());
        cursor = end;
    }
    targets
}

fn read_small_utf8_file(path: &Path) -> Option<String> {
    const MAX_REPORT_VALIDATION_BYTES: u64 = 2 * 1024 * 1024;
    let metadata = std::fs::symlink_metadata(path).ok()?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_REPORT_VALIDATION_BYTES
    {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

fn complete_html_document(html: &str) -> bool {
    let Some(document) = html_document_after_optional_artifact_marker(html) else {
        return false;
    };
    let lower = document.to_ascii_lowercase();
    document.contains(DEEP_RESEARCH_HTML_DOCUMENT_ATTR)
        && lower.contains("<html")
        && lower.contains("</html>")
        && lower.contains("<body")
        && lower.contains("</body>")
        && lower.contains("name=\"viewport\"")
        && lower.contains("<title>")
        && lower.contains("</title>")
        && lower.contains("<style")
        && lower.contains("<main")
        && lower.contains("</main>")
        && lower.contains("<article")
        && lower.contains("</article>")
        && lower.matches("<h1").count() == 1
        && !lower.contains("<script")
}

fn html_document_after_optional_artifact_marker(html: &str) -> Option<&str> {
    let trimmed = html.trim_start();
    if trimmed.starts_with("<!doctype html>") {
        return Some(trimmed);
    }
    let (first_line, remainder) = trimmed.split_once('\n')?;
    deep_research_artifact_kind(first_line.trim())?;
    let document = remainder.trim_start();
    document
        .starts_with("<!doctype html>")
        .then_some(document)
}
