#[cfg(test)]
mod html_quality_gate_tests {
    use super::complete_html_document;

    #[test]
    fn rejects_html_without_the_host_renderer_contract_or_with_script_content() {
        assert!(!complete_html_document(
            "<!doctype html><html><body><h1>Report</h1><p>Sources and confidence.</p></body></html>"
        ));
        let scripted = format!(
            "<!doctype html><html><head><meta name=\"viewport\"><title>x</title><style></style></head><body><h1>x</h1><main {}><article><script>unsafe()</script></article></main></body></html>",
            super::DEEP_RESEARCH_HTML_DOCUMENT_ATTR
        );
        assert!(!complete_html_document(&scripted));
    }
}

const SYNTHESIZED_ARTIFACT_MARKER: &str =
    "A3S_DEEP_RESEARCH_ARTIFACT:synthesized:v1";
const SOURCE_BACKED_ARTIFACT_MARKER: &str =
    "A3S_DEEP_RESEARCH_ARTIFACT:source_backed:v1";
const NO_EVIDENCE_ARTIFACT_MARKER: &str =
    "A3S_DEEP_RESEARCH_ARTIFACT:no_evidence:v1";
const RECOVERY_ARTIFACT_MARKER: &str = "A3S_DEEP_RESEARCH_ARTIFACT:recovery:v1";
const FALLBACK_ARTIFACT_MARKER: &str = "A3S_DEEP_RESEARCH_ARTIFACT:fallback:v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DeepResearchArtifactKind {
    Synthesized,
    SourceBacked,
    NoEvidence,
    Recovery,
    Fallback,
}

impl DeepResearchArtifactKind {
    fn marker(self) -> &'static str {
        match self {
            Self::Synthesized => SYNTHESIZED_ARTIFACT_MARKER,
            Self::SourceBacked => SOURCE_BACKED_ARTIFACT_MARKER,
            Self::NoEvidence => NO_EVIDENCE_ARTIFACT_MARKER,
            Self::Recovery => RECOVERY_ARTIFACT_MARKER,
            Self::Fallback => FALLBACK_ARTIFACT_MARKER,
        }
    }
}

fn artifact_marker_line(kind: DeepResearchArtifactKind) -> String {
    format!("<!-- {} -->", kind.marker())
}

fn deep_research_artifact_kind(text: &str) -> Option<DeepResearchArtifactKind> {
    let mut observed = None;
    for line in text.lines().map(str::trim) {
        let Some(kind) = [
            DeepResearchArtifactKind::Synthesized,
            DeepResearchArtifactKind::SourceBacked,
            DeepResearchArtifactKind::NoEvidence,
            DeepResearchArtifactKind::Recovery,
            DeepResearchArtifactKind::Fallback,
        ]
        .into_iter()
        .find(|kind| line == artifact_marker_line(*kind))
        else {
            continue;
        };
        match observed {
            Some(previous) if previous != kind => return None,
            Some(_) => {}
            None => observed = Some(kind),
        }
    }
    observed
}

fn deep_research_artifact_pair_has_kind(
    markdown: &str,
    html: &str,
    expected: DeepResearchArtifactKind,
) -> bool {
    deep_research_artifact_kind(markdown) == Some(expected)
        && deep_research_artifact_kind(html) == Some(expected)
}

fn markdown_with_artifact_kind(
    markdown: &str,
    kind: DeepResearchArtifactKind,
) -> Result<String, String> {
    if deep_research_artifact_kind(markdown).is_some() {
        return Err("report content already contains a reserved artifact marker".to_string());
    }
    let body = markdown.trim();
    let marker = artifact_marker_line(kind);
    Ok(match body.split_once('\n') {
        Some((heading, remainder)) => format!("{heading}\n\n{marker}\n\n{}", remainder.trim_start()),
        None => format!("{body}\n\n{marker}\n"),
    })
}

fn html_with_artifact_kind(
    html: &str,
    kind: DeepResearchArtifactKind,
) -> Result<String, String> {
    if deep_research_artifact_kind(html).is_some() {
        return Err("rendered report already contains a reserved artifact marker".to_string());
    }
    Ok(format!("{}\n{}", artifact_marker_line(kind), html.trim_start()))
}

#[cfg(test)]
mod artifact_kind_tests {
    use super::{
        artifact_marker_line, deep_research_artifact_kind, DeepResearchArtifactKind,
    };

    #[test]
    fn reader_facing_words_never_classify_an_artifact() {
        for text in [
            "# DeepResearch Recovery Report\n\nA reader-facing title.",
            "<h1>DeepResearch Fallback Draft</h1><p>Not a final report.</p>",
            "A source discusses A3S_DEEP_RESEARCH_ARTIFACT without a protocol marker.",
        ] {
            assert_eq!(deep_research_artifact_kind(text), None);
        }
    }

    #[test]
    fn exact_protocol_markers_are_the_only_artifact_authority() {
        let marker = artifact_marker_line(DeepResearchArtifactKind::Recovery);
        assert_eq!(
            deep_research_artifact_kind(&format!("# Report\n\n{marker}\n\nBody")),
            Some(DeepResearchArtifactKind::Recovery)
        );
        assert_eq!(
            deep_research_artifact_kind(&format!(
                "{marker}\n{}",
                artifact_marker_line(DeepResearchArtifactKind::Fallback)
            )),
            None
        );
    }
}

fn has_research_report_substance(markdown: &str, html: &str) -> bool {
    const MIN_MARKDOWN_TEXT_CHARS: usize = 120;
    const MIN_HTML_TEXT_CHARS: usize = 120;

    let markdown_text = markdown.trim();
    let html_text = strip_html_tags(html);
    visible_char_count(markdown_text) >= MIN_MARKDOWN_TEXT_CHARS
        && visible_char_count(&html_text) >= MIN_HTML_TEXT_CHARS
        && markdown_text
            .lines()
            .find(|line| !line.trim().is_empty())
            .is_some_and(|line| line.trim().starts_with("# "))
        && markdown_text
            .lines()
            .any(|line| line.trim_start().starts_with("## "))
        && html.contains("<main")
        && html.contains("<article")
        && html.contains("<h1")
}

pub fn normalize_research_source_anchor(value: &str) -> Option<String> {
    let raw = value.trim();
    if raw.is_empty() || raw.chars().any(char::is_control) {
        return None;
    }
    let normalized = value
        .replace("&amp;", "&")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|ch: char| {
            matches!(
                ch,
                '`' | '"' | '\'' | '<' | '>' | '(' | ')' | '[' | ']' | '{' | '}'
            )
        })
        .trim_end_matches(['.', ',', ';', ':', ')', ']'])
        .trim()
        .trim_end_matches('/')
        .to_string();
    if normalized.len() < 4
        || normalized.starts_with("a3s://")
        || !looks_like_traceable_source(&normalized)
    {
        None
    } else {
        Some(normalized)
    }
}

fn canonical_research_source_anchor(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_control) {
        return None;
    }
    let canonical = if let Ok(mut url) = reqwest::Url::parse(trimmed) {
        if !matches!(url.scheme(), "http" | "https") || url.host_str()?.is_empty() {
            return None;
        }
        url.set_username("").ok()?;
        url.set_password(None).ok()?;
        let mut query_pairs = url
            .query_pairs()
            .map(|(key, value)| (key.into_owned(), value.into_owned()))
            .filter(|(key, value)| {
                !key.is_empty()
                    && key.len() <= 256
                    && value.len() <= 2_048
                    && !key.chars().any(char::is_control)
                    && !value.chars().any(char::is_control)
            })
            .collect::<Vec<_>>();
        query_pairs.sort();
        query_pairs.dedup();
        url.set_query(None);
        if !query_pairs.is_empty() {
            url.query_pairs_mut()
                .extend_pairs(query_pairs.iter().map(|(key, value)| (key, value)));
        }
        url.set_fragment(None);
        url.to_string()
    } else {
        let path = trimmed.replace('\\', "/");
        path.strip_prefix("./").unwrap_or(&path).to_string()
    };
    normalize_research_source_anchor(&canonical)?;
    Some(canonical)
}

fn looks_like_traceable_source(value: &str) -> bool {
    if value.starts_with("http://") || value.starts_with("https://") {
        return reqwest::Url::parse(value)
            .ok()
            .and_then(|url| url.host_str().map(str::to_string))
            .is_some_and(|host| !host.is_empty());
    }
    if value.contains("://")
        || value.starts_with('/')
        || value.starts_with('~')
        || value.split(['/', '\\']).any(|part| part == "..")
    {
        return false;
    }

    let without_fragment = value.split('#').next().unwrap_or(value);
    let without_line = without_fragment
        .rsplit_once(':')
        .filter(|(_, suffix)| suffix.chars().all(|ch| ch.is_ascii_digit()))
        .map(|(path, _)| path)
        .unwrap_or(without_fragment);
    let path = Path::new(without_line);
    let has_file_extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            !extension.is_empty()
                && extension.len() <= 16
                && extension
                    .chars()
                    .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_'))
        });
    let has_relative_path_shape = without_line.contains('/') || without_line.contains('\\');
    has_file_extension || has_relative_path_shape
}

fn visible_char_count(text: &str) -> usize {
    text.chars()
        .filter(|ch| !ch.is_whitespace() && !ch.is_control())
        .count()
}

fn strip_html_tags(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => {
                in_tag = true;
                out.push(' ');
            }
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

pub fn looks_like_deep_research_fallback_draft(text: &str) -> bool {
    deep_research_artifact_kind(text) == Some(DeepResearchArtifactKind::Fallback)
}

fn is_html_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("html" | "htm")
    )
}

fn is_nonempty_file(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}
