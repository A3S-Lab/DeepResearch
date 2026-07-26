use super::{html_escape, markdown_backslash_unescape, markdown_plain_text};
use crate::report::html_host::{render_report_menu, report_host_script_element, REPORT_HOST_CSS};
use crate::report::html_style::REPORT_CSS;
use crate::report::report_generation::ReportPresentation;
use comrak::{markdown_to_html, Options};

#[path = "html_composition.rs"]
mod composition;

use composition::compose_report_fragment;

pub(super) const DEEP_RESEARCH_HTML_DOCUMENT_ATTR: &str = r#"data-a3s-deep-research-document="v1""#;

pub(super) fn deep_research_completed_report_html(query: &str, markdown: &str) -> String {
    let output_language = crate::language::infer_deep_research_output_language(query);
    deep_research_report_html_with_state(
        query,
        markdown,
        None,
        None,
        ReportRenderState::Complete,
        &output_language,
    )
}

pub(super) fn deep_research_degraded_report_html(query: &str, markdown: &str) -> String {
    let output_language = crate::language::infer_deep_research_output_language(query);
    deep_research_degraded_report_html_in_language(query, markdown, &output_language)
}

pub(super) fn deep_research_degraded_report_html_in_language(
    query: &str,
    markdown: &str,
    output_language: &str,
) -> String {
    deep_research_report_html_with_state(
        query,
        markdown,
        None,
        None,
        ReportRenderState::Degraded,
        output_language,
    )
}

pub(super) fn deep_research_completed_report_html_with_presentation(
    query: &str,
    markdown: &str,
    presentation: Option<&ReportPresentation>,
    authored_thesis: Option<&str>,
) -> String {
    let output_language = crate::language::infer_deep_research_output_language(query);
    deep_research_report_html_with_state(
        query,
        markdown,
        presentation,
        authored_thesis,
        ReportRenderState::Complete,
        &output_language,
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReportRenderState {
    Complete,
    Degraded,
}

fn deep_research_report_html_with_state(
    query: &str,
    markdown: &str,
    _presentation: Option<&ReportPresentation>,
    authored_thesis: Option<&str>,
    render_state: ReportRenderState,
    output_language: &str,
) -> String {
    let degraded = render_state == ReportRenderState::Degraded;
    let title = concise_report_title(&deep_research_markdown_report_title(markdown, query));
    let labels = report_labels(degraded, output_language);
    let language = output_language;
    let raw_body = strip_first_h1(&deep_research_markdown_to_html_fragment(markdown));
    let composition = compose_report_fragment(&raw_body, &[], labels.contents, labels.contents);
    let report_menu = render_report_menu(language, labels.evidence, Some(labels.confidence));
    let host_script = report_host_script_element();
    let body_class = if degraded {
        "a3s-report report-degraded"
    } else {
        "a3s-report"
    };
    let thesis = authored_thesis
        .map(str::trim)
        .filter(|thesis| (12..=1_200).contains(&thesis.chars().count()))
        .unwrap_or(labels.fallback_thesis);
    format!(
        r##"<!doctype html>
<html lang="{language}">
<head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{title}</title><style>:root{{--table-scroll-hint:'{table_scroll_hint}';}}{css}{host_css}</style></head>
<body class="{theme}" data-a3s-report-state="readonly">
<a class="skip-link" href="#report-main">{skip_to_report}</a>
<div class="report-shell">
{report_menu}
{toc}
<div class="report-column">
<header class="report-hero" data-a3s-editable-region contenteditable="false" spellcheck="false"><h1>{title}</h1><p class="report-thesis">{thesis}</p></header>
<main id="report-main" {document_attr}><article id="report" data-a3s-editable-region contenteditable="false" spellcheck="false">{body}</article></main>
<footer class="footer-note">{brief_label} · {evidence_label}</footer>
</div>
</div>
{host_script}
</body></html>
"##,
        language = language,
        title = html_escape(&title),
        css = REPORT_CSS,
        host_css = REPORT_HOST_CSS,
        table_scroll_hint = labels.table_scroll_hint,
        theme = body_class,
        brief_label = labels.brief,
        thesis = html_escape(thesis),
        evidence_label = labels.evidence,
        skip_to_report = labels.skip_to_report,
        document_attr = DEEP_RESEARCH_HTML_DOCUMENT_ATTR,
        report_menu = report_menu,
        toc = composition.toc,
        body = composition.body,
        host_script = host_script,
    )
}

#[derive(Clone, Copy)]
struct ReportLabels {
    brief: &'static str,
    evidence: &'static str,
    confidence: &'static str,
    contents: &'static str,
    fallback_thesis: &'static str,
    table_scroll_hint: &'static str,
    skip_to_report: &'static str,
}

fn report_labels(degraded: bool, output_language: &str) -> ReportLabels {
    if crate::language::primary_output_language(output_language) == "zh" {
        ReportLabels {
            brief: if degraded {
                "A3S 深度研究 · 降级"
            } else {
                "A3S 深度研究"
            },
            evidence: if degraded {
                "证据不足 · 降级"
            } else {
                "证据可追溯"
            },
            confidence: if degraded {
                "并非最终领域结论"
            } else {
                "已说明置信度与边界"
            },
            contents: "报告目录",
            fallback_thesis: if degraded {
                "本次研究未通过证据准入门槛；此页面仅保留可追溯来源、证据边界和后续核验线索。"
            } else {
                "一份区分结论、证据强度与未决边界的来源支撑型研究报告。"
            },
            table_scroll_hint: "← 横向滑动查看全部列 →",
            skip_to_report: "跳到报告正文",
        }
    } else {
        ReportLabels {
            brief: if degraded {
                "A3S Deep Research · Degraded"
            } else {
                "A3S Deep Research"
            },
            evidence: if degraded {
                "Insufficient evidence · Degraded"
            } else {
                "Traceable evidence"
            },
            confidence: if degraded {
                "Not a final domain conclusion"
            } else {
                "Confidence & limits stated"
            },
            contents: "Report contents",
            fallback_thesis: if degraded {
                "This run did not meet the evidence gate; the page preserves only traceable sources, failure limits, and next actions."
            } else {
                "A source-backed reading experience separating conclusions, evidence strength, and unresolved limits."
            },
            table_scroll_hint: "← swipe to inspect all columns →",
            skip_to_report: "Skip to report",
        }
    }
}

fn concise_report_title(value: &str) -> String {
    let clean = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let limit = 96;
    if clean.chars().count() <= limit {
        return clean;
    }
    let prefix = clean.chars().take(limit).collect::<String>();
    let shortened = prefix
        .rsplit_once(' ')
        .map(|(head, _)| head)
        .unwrap_or(&prefix);
    format!(
        "{}…",
        shortened.trim_end_matches([':', ';', ',', '，', '；', '：'])
    )
}

fn strip_first_h1(fragment: &str) -> String {
    let Some(start) = fragment.find("<h1>") else {
        return fragment.to_string();
    };
    let Some(relative_end) = fragment[start..].find("</h1>") else {
        return fragment.to_string();
    };
    let end = start + relative_end + "</h1>".len();
    format!("{}{}", &fragment[..start], &fragment[end..])
}

fn deep_research_markdown_report_title(markdown: &str, query: &str) -> String {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(markdown_backslash_unescape)
        .unwrap_or_else(|| markdown_backslash_unescape(&markdown_plain_text(query)))
}

fn deep_research_markdown_to_html_fragment(markdown: &str) -> String {
    let mut options = Options::default();
    options.extension.table = true;
    options.extension.autolink = true;
    options.render.unsafe_ = false;
    options.render.escape = true;
    strip_relative_report_links(&markdown_to_html(markdown, &options))
}

fn strip_relative_report_links(fragment: &str) -> String {
    let mut output = String::with_capacity(fragment.len());
    let mut remaining = fragment;

    while let Some(anchor_start) = remaining.find("<a href=\"") {
        output.push_str(&remaining[..anchor_start]);
        let anchor = &remaining[anchor_start..];
        let href = &anchor["<a href=\"".len()..];
        let Some(href_end) = href.find('"') else {
            output.push_str(anchor);
            return output;
        };
        let href_value = &href[..href_end];
        let Some(open_end) = anchor.find('>') else {
            output.push_str(anchor);
            return output;
        };
        let allowed = href_value.is_empty()
            || href_value.starts_with('#')
            || href_value.starts_with("https://")
            || href_value.starts_with("http://")
            || href_value.starts_with("mailto:");
        if allowed {
            output.push_str(&anchor[..=open_end]);
            remaining = &anchor[open_end + 1..];
            continue;
        }

        let inner = &anchor[open_end + 1..];
        let Some(close_start) = inner.find("</a>") else {
            output.push_str(inner);
            return output;
        };
        output.push_str(&inner[..close_start]);
        remaining = &inner[close_start + "</a>".len()..];
    }

    output.push_str(remaining);
    output
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
