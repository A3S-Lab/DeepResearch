use super::*;
use crate::report::report_generation::{
    ReportArchetype, ReportDensity, ReportHero, ReportNarrativeMode, ReportPalette,
    ReportSectionComposition, ReportSectionRhythm, ReportSectionTreatment, ReportVisualStance,
};

#[test]
fn markdown_report_html_renders_safe_clickable_sources() {
    let html = deep_research_completed_report_html(
        "Clickable sources",
        "# Clickable sources\n\n- [Official <source>](https://example.com/docs?a=1&b=2)\n- Bare: https://example.org/evidence.\n- [unsafe](javascript:alert(1))",
    );

    assert!(
        html.contains(
            "<a href=\"https://example.com/docs?a=1&amp;b=2\">Official &lt;source&gt;</a>"
        ),
        "{html}"
    );
    assert!(
        html.contains("<a href=\"https://example.org/evidence\">https://example.org/evidence</a>."),
        "{html}"
    );
    assert!(html.contains("<a href=\"\">unsafe</a>"), "{html}");
}

#[test]
fn markdown_table_keeps_escaped_pipes_inside_cells() {
    let fragment = deep_research_markdown_to_html_fragment(
        "| Finding | Source |\n| --- | --- |\n| Alpha \\| Beta | [`a\\|b`](https://example.com/a%7Cb) |",
    );

    assert!(fragment.contains("<table>"), "{fragment}");
    assert_eq!(fragment.matches("<td>").count(), 2, "{fragment}");
    assert!(fragment.contains("<td>Alpha | Beta</td>"), "{fragment}");
    assert!(
        fragment.contains("<td><a href=\"https://example.com/a%7Cb\"><code>a|b</code></a></td>"),
        "{fragment}"
    );
}

#[test]
fn markdown_report_html_escapes_raw_html_and_blocks_dangerous_links() {
    let fragment = deep_research_markdown_to_html_fragment(
        "<script>alert('xss')</script>\n\n<img src=x onerror=alert(1)>\n\n[unsafe](javascript:alert(1)) [encoded](jav&#x61;script:alert(2)) [safe](https://example.com)",
    );

    assert!(!fragment.contains("<script"), "{fragment}");
    assert!(!fragment.contains("<img"), "{fragment}");
    assert!(fragment.contains("&lt;script&gt;"), "{fragment}");
    assert!(
        fragment.contains("&lt;img src=x onerror=alert(1)&gt;"),
        "{fragment}"
    );
    assert_eq!(fragment.matches("href=\"\"").count(), 2, "{fragment}");
    assert!(
        fragment.contains("<a href=\"https://example.com\">safe</a>"),
        "{fragment}"
    );
}

#[test]
fn markdown_report_html_keeps_fetched_relative_links_as_plain_text() {
    let fragment = deep_research_markdown_to_html_fragment(
        "[Joint Typhoon Warning Center](/wiki/JTWC) and [local](../private/report.md)",
    );

    assert!(
        fragment.contains("Joint Typhoon Warning Center"),
        "{fragment}"
    );
    assert!(fragment.contains("and local"), "{fragment}");
    assert!(!fragment.contains("href=\"/wiki/"), "{fragment}");
    assert!(!fragment.contains("href=\"../"), "{fragment}");
}

#[test]
fn markdown_report_html_sanitizes_query_fallback_title() {
    let html = deep_research_completed_report_html(
        "Analyze https://user:password@example.com/private?token=secret#fragment",
        "Findings without a level-one heading.",
    );

    assert!(
        html.contains("<title>Analyze https://example.com/private</title>"),
        "{html}"
    );
    for secret in ["user", "password", "token=secret", "#fragment"] {
        assert!(!html.contains(secret), "{html}");
    }
}

#[test]
fn editorial_report_html_has_responsive_print_and_accessibility_contract() {
    let html = deep_research_completed_report_html(
        "一份非常长的中文研究请求，需要生成专业、清晰、可验证且适合移动设备阅读的报告，并且不要把完整用户提示直接当作页面标题反复展示",
        "# 简洁研究标题\n\n## 核心发现\n\n结论正文。\n\n## Sources\n\n- [来源](https://example.com)",
    );

    assert!(html.contains("<html lang=\"zh\">"), "{html}");
    assert!(
        html.contains("<body class=\"a3s-report\" data-a3s-report-state=\"readonly\">"),
        "{html}"
    );
    assert!(html.contains("class=\"report-hero\""), "{html}");
    assert!(html.contains("class=\"report-shell\""), "{html}");
    assert!(html.contains("--a3s-bg: #f7f7f8"), "{html}");
    assert!(html.contains("--a3s-blue: #2864e8"), "{html}");
    assert!(html.contains("--a3s-action: #242424"), "{html}");
    assert!(html.contains("@media (max-width: 820px)"), "{html}");
    assert!(html.contains("@media (max-width: 640px)"), "{html}");
    assert!(html.contains("横向滑动查看全部列"), "{html}");
    assert!(html.contains("@media print"), "{html}");
    assert!(html.contains("prefers-reduced-motion"), "{html}");
    assert!(html.contains(":focus-visible"), "{html}");
    assert!(!html.contains("class=\"evidence-profile\""), "{html}");
    assert!(html.contains("class=\"report-nav\""), "{html}");
    assert!(
        html.contains("class=\"report-menu\"")
            && html.contains("data-a3s-report-host-ui=\"v1\"")
            && html.contains("data-a3s-action=\"edit\"")
            && html.contains("data-a3s-action=\"save\"")
            && html.contains("data-a3s-action=\"print\""),
        "{html}"
    );
    assert!(
        html.contains("grid-template-columns: 176px minmax(0, 1fr) 232px;")
            && html.contains("grid-column: 3;"),
        "{html}"
    );
    let menu = html.find("class=\"report-menu\"").expect("report menu");
    let navigation = html.find("class=\"report-nav\"").expect("report ToC");
    let report_column = html
        .find("class=\"report-column\"")
        .expect("report content column");
    assert!(
        menu < navigation && navigation < report_column,
        "mobile DOM order must place the menu and ToC before the report column: {html}"
    );
    assert_eq!(
        html.matches("class=\"report-section section--narrative")
            .count(),
        2,
        "{html}"
    );
    assert!(
        !html.contains("class=\"report-section section--findings"),
        "{html}"
    );
    assert!(
        !html.contains("class=\"report-section section--sources"),
        "{html}"
    );
    assert!(html.contains("aria-label=\"报告目录\""), "{html}");
    assert!(html.contains("aria-label=\"报告菜单\""), "{html}");
    assert!(html.contains("编辑报告"), "{html}");
    assert!(html.contains("保存 HTML"), "{html}");
    assert!(html.contains("打印报告"), "{html}");
    assert!(html.contains("class=\"skip-link\""), "{html}");
    assert_eq!(html.matches("<h1>").count(), 1, "{html}");
    assert_eq!(html.matches("<script").count(), 1, "{html}");
    assert!(html.contains("data-a3s-report-host=\"v1\""), "{html}");
    assert!(html.contains("data-a3s-editable-region"), "{html}");
    assert!(!html.contains("<script>alert"), "{html}");
    assert!(!html.contains("linear-gradient"), "{html}");
}

#[test]
fn headings_do_not_select_information_shapes_without_a_typed_section_plan() {
    let html = deep_research_completed_report_html(
        "Compare Tokio and async-std",
        "# Tokio and async-std\n\n## Executive Summary\n\n- Tokio is active.\n- async-std is deprecated.\n\n## Key Findings\n\n### Maintenance\n\nLifecycle evidence.\n\n### Adoption\n\nAdoption evidence.\n\n## Evidence Matrix\n\n| Finding | Source |\n| --- | --- |\n| Maintenance | [Docs](https://example.com/docs) |\n\n## Gaps And Caveats\n\n- No workload benchmark.\n\n## Source Quality And Confidence\n\nConfidence is medium-high.\n\n## Sources\n\n- [Docs](https://example.com/docs)",
    );

    assert_eq!(
        html.matches("class=\"report-section section--narrative")
            .count(),
        6,
        "{html}"
    );
    assert!(html.contains("class=\"table-wrap\""), "{html}");
    for forbidden in [
        "class=\"report-section section--summary",
        "class=\"report-section section--findings",
        "class=\"key-point\"",
        "class=\"report-section section--matrix",
        "class=\"report-section section--caveats",
        "class=\"report-section section--confidence",
        "class=\"report-section section--sources",
    ] {
        assert!(!html.contains(forbidden), "unexpected {forbidden}: {html}");
    }
    assert!(html.contains(
        "<span class=\"report-nav__index\">06</span><span class=\"report-nav__text\">Sources</span>"
    ));
    assert!(html.contains("href=\"#section-6\""));
}

#[test]
fn presentation_inputs_do_not_change_the_fixed_a3s_report_style() {
    let markdown = "# Shared evidence\n\n## Executive Summary\n\nA source-backed answer.\n\n## Key Findings\n\n### Finding\n\nInterpretation and implication.\n\n## Sources\n\n- [Source](https://example.com/source)";
    let analytical = ReportPresentation {
        narrative_mode: ReportNarrativeMode::Pyramid,
        archetype: ReportArchetype::Analytical,
        palette: ReportPalette::Graphite,
        density: ReportDensity::Compact,
        hero: ReportHero::Metrics,
        visual_stance: ReportVisualStance::Safe,
        rationale: "Dense comparison for a decision reader.".to_string(),
        section_plan: vec![
            ReportSectionTreatment {
                heading: "Executive Summary".to_string(),
                rhythm: ReportSectionRhythm::Anchor,
                composition: ReportSectionComposition::Prose,
            },
            ReportSectionTreatment {
                heading: "Key Findings".to_string(),
                rhythm: ReportSectionRhythm::Dense,
                composition: ReportSectionComposition::KeyPoints,
            },
            ReportSectionTreatment {
                heading: "Sources".to_string(),
                rhythm: ReportSectionRhythm::Breathing,
                composition: ReportSectionComposition::SourceLedger,
            },
        ],
    };
    let chronicle = ReportPresentation {
        narrative_mode: ReportNarrativeMode::Narrative,
        archetype: ReportArchetype::Chronicle,
        palette: ReportPalette::Amber,
        density: ReportDensity::Spacious,
        hero: ReportHero::Statement,
        visual_stance: ReportVisualStance::Bold,
        rationale: "Ordered change needs a chronological reading rhythm.".to_string(),
        section_plan: vec![
            ReportSectionTreatment {
                heading: "Executive Summary".to_string(),
                rhythm: ReportSectionRhythm::Breathing,
                composition: ReportSectionComposition::Prose,
            },
            ReportSectionTreatment {
                heading: "Key Findings".to_string(),
                rhythm: ReportSectionRhythm::Anchor,
                composition: ReportSectionComposition::Timeline,
            },
            ReportSectionTreatment {
                heading: "Sources".to_string(),
                rhythm: ReportSectionRhythm::Dense,
                composition: ReportSectionComposition::SourceLedger,
            },
        ],
    };

    let analytical_html = deep_research_completed_report_html_with_presentation(
        "Shared evidence",
        markdown,
        Some(&analytical),
        Some("The same evidence supports the same bounded decision."),
    );
    let chronicle_html = deep_research_completed_report_html_with_presentation(
        "Shared evidence",
        markdown,
        Some(&chronicle),
        Some("The same evidence supports the same bounded decision."),
    );

    assert_eq!(analytical_html, chronicle_html);
    assert!(
        analytical_html.contains("<body class=\"a3s-report\" data-a3s-report-state=\"readonly\">")
    );
    assert!(analytical_html.contains("The same evidence supports the same bounded decision."));
    for forbidden in [
        "mode-pyramid",
        "mode-narrative",
        "archetype-",
        "palette-",
        "density-",
        "hero-metrics",
        "hero-statement",
        "stance-",
        "composition-key-points",
        "composition-timeline",
        "evidence-profile",
        "hero-map",
    ] {
        assert!(!analytical_html.contains(forbidden), "{forbidden}");
    }
}

#[test]
fn fixed_report_header_omits_decorative_metrics_and_reading_maps() {
    let presentation = ReportPresentation::default();
    let html = deep_research_completed_report_html_with_presentation(
        "A bounded decision",
        "# Decision\n\n## Answer\n\nAct now.\n\n## Trade-offs\n\nThe boundary is explicit.\n\n## Sources\n\n- [Source](https://example.com)",
        Some(&presentation),
        Some("The evidence changes the decision boundary."),
    );

    assert!(html.contains("class=\"report-hero\""), "{html}");
    assert!(html.contains("The evidence changes the decision boundary."));
    assert!(!html.contains("class=\"evidence-profile\""), "{html}");
    assert!(!html.contains("class=\"hero-map\""), "{html}");
}

#[test]
fn recovery_report_is_visually_and_semantically_degraded() {
    let html = deep_research_degraded_report_html(
        "Current market state",
        "# DeepResearch Recovery Report\n\n## Findings\n\nEvidence collection did not complete.\n\n## Sources And Evidence\n\n- https://example.com/partial\n\n## Confidence And Limits\n\nConfidence is low.",
    );

    assert!(html.contains("report-degraded"), "{html}");
    assert!(html.contains("Insufficient evidence · Degraded"), "{html}");
    assert!(html.contains("Not a final domain conclusion"), "{html}");
    assert!(html.contains("<title>DeepResearch Recovery Report</title>"));
}

#[test]
fn editorial_report_wraps_unparsed_relative_markdown_without_page_overflow() {
    let html = deep_research_completed_report_html(
        "台风巴威",
        "# 台风巴威研究\n\n## 证据\n\n- [菲律宾大气地球物理和天文管理局](/wiki/%E8%8F%B2%E5%BE%8B%E8%B3%93%E5%A4%A7%E6%B0%A3%E5%9C%B0%E7%90%83%E7%89%A9%E7%90%86%E5%92%8C%E5%A4%A9%E6%96%87%E7%AE%A1%E7%90%86%E5%B1%80",
    );

    assert!(html.contains("overflow-wrap: anywhere"), "{html}");
    assert!(html.contains("article {"), "{html}");
    assert!(html.contains("min-width: 0;"), "{html}");
    assert!(html.contains(".table-wrap {"), "{html}");
    assert!(html.contains("overflow-x: auto;"), "{html}");
    assert!(html.contains("min-width: 720px;"), "{html}");
}

#[test]
fn editorial_report_truncates_long_titles_without_language_rules() {
    let title = concise_report_title(&"Evidence boundary ".repeat(12));

    assert!(title.ends_with('…'), "{title}");
    assert_eq!(title.matches('…').count(), 1, "{title}");
    assert!(title.chars().count() <= 97, "{title}");
}

#[test]
fn editorial_report_preserves_an_ordinary_descriptive_cjk_title() {
    let title = "Tokio 与 async-std：维护状态、生态采用与迁移建议（截至 2026 年 7 月）";
    let html = deep_research_completed_report_html(
        "Compare two Rust runtimes",
        &format!("# {title}\n\n## 结论\n\n证据支持该结论。"),
    );

    assert!(html.contains(&format!("<title>{title}</title>")), "{html}");
    assert!(html.contains(&format!("<h1>{title}</h1>")), "{html}");
}

#[test]
fn print_layout_does_not_expand_every_inline_link_or_pin_large_tables() {
    let html = deep_research_completed_report_html(
        "Printable report",
        "# Printable report\n\nA [source](https://example.com/very/long/path).",
    );

    assert!(!html.contains("attr(href)"), "{html}");
    assert!(
        html.contains(".key-point,")
            && html.contains(".timeline-entry,")
            && html.contains(".process-step,")
            && html.contains("break-inside: avoid;"),
        "{html}"
    );
    assert!(html.contains(".table-wrap {"), "{html}");
    assert!(html.contains("overflow: visible;"), "{html}");
}

#[test]
fn print_layout_uses_the_fixed_page_box_when_legacy_presentation_requests_shifting() {
    let html = deep_research_completed_report_html_with_presentation(
        "Printable shifted report",
        "# Printable shifted report\n\n## Findings\n\nA long paragraph must remain inside the printable page box.",
        Some(&ReportPresentation {
            visual_stance: ReportVisualStance::Shifted,
            ..ReportPresentation::default()
        }),
        Some("The screen composition may be shifted without moving print content off-page."),
    );

    assert!(html.contains("article,"), "{html}");
    assert!(html.contains(".report-section,"), "{html}");
    assert!(html.contains(".section-body {"), "{html}");
    assert!(html.contains("width: 100%;"), "{html}");
    assert!(html.contains("max-width: 100%;"), "{html}");
    assert!(!html.contains("stance-shifted"), "{html}");
}

#[test]
fn report_header_does_not_turn_source_counts_into_decorative_metrics() {
    let html = deep_research_completed_report_html(
        "Source count",
        "# Source count\n\n## Summary\n\nLead with [supporting context](https://context.example.net).\n\n## Sources\n\n- [Primary](https://example.com/evidence) — evidence: see [nested detail](https://nested.example.net)\n- [Primary again](https://example.com/evidence)\n- [Secondary](http://example.org/report)\n- [Internal](/local/report)",
    );

    assert!(html.contains("https://example.com/evidence"), "{html}");
    assert!(html.contains("https://nested.example.net"), "{html}");
    assert!(!html.contains("class=\"evidence-profile\""), "{html}");
    assert!(!html.contains("<strong>04</strong>"), "{html}");
}

#[test]
fn report_header_uses_the_neutral_a3s_surface_without_a_decorative_gradient() {
    let html = deep_research_completed_report_html(
        "Responsive hero",
        "# Responsive hero\n\n## Summary\n\nLead.",
    );

    assert!(html.contains("background: var(--a3s-bg);"), "{html}");
    assert!(html.contains(".report-hero {"), "{html}");
    assert!(html.contains("background: var(--a3s-panel);"), "{html}");
    assert!(!html.contains("linear-gradient"), "{html}");
}

#[test]
fn ordinary_headings_keep_the_neutral_section_treatment() {
    let html = deep_research_completed_report_html(
        "Lead paragraph",
        "# Lead paragraph\n\n## Executive summary\n\nThis paragraph is the report lead.\n\nMore detail.",
    );

    assert!(
        html.contains(
            "class=\"report-section section--narrative rhythm-breathing composition-prose\""
        ) && html.contains("<h2>Executive summary</h2>")
            && html.contains("<p>This paragraph is the report lead.</p>"),
        "{html}"
    );
}

#[test]
fn right_toc_moves_before_the_report_on_mobile_without_metric_cards() {
    let html = deep_research_completed_report_html(
        "Mobile metadata",
        "# Mobile metadata\n\n## Summary\n\nLead.\n\n## Sources\n\n- [Source](https://example.com)",
    );

    assert!(html.contains("class=\"report-nav\""), "{html}");
    assert!(html.contains("href=\"#section-2\""), "{html}");
    assert!(
        html.contains(".report-nav__track {")
            && html.contains("order: -1;")
            && html.contains("flex-direction: column;"),
        "{html}"
    );
    assert!(!html.contains("rail-stat"), "{html}");
    assert!(!html.contains("evidence-profile"), "{html}");
}

#[test]
fn legacy_archetype_input_does_not_add_mobile_theme_overrides() {
    let html = deep_research_completed_report_html_with_presentation(
        "Responsive analysis",
        "# Responsive analysis\n\n## Findings\n\nThe bounded conclusion.\n\n## Sources\n\n- [Source](https://example.com)",
        Some(&ReportPresentation {
            archetype: ReportArchetype::Analytical,
            ..ReportPresentation::default()
        }),
        Some("The layout must preserve a readable article width on narrow screens."),
    );

    assert!(html.contains("@media (max-width: 640px)"), "{html}");
    assert!(!html.contains("archetype-analytical"), "{html}");
}

#[test]
fn print_source_ledger_is_single_column_and_does_not_create_a_footer_only_page() {
    let html = deep_research_completed_report_html(
        "Printable sources",
        "# Printable sources\n\n## Sources\n\n- [One](https://example.com/one)\n- [Two](https://example.com/two)",
    );

    assert!(html.contains(".composition-source-ledger {"), "{html}");
    assert!(
        html.contains(".composition-source-ledger .section-body > ul")
            && html.contains("grid-template-columns: 1fr;"),
        "{html}"
    );
    assert!(
        html.contains(".composition-source-ledger .section-body > ul > li,"),
        "{html}"
    );
    assert!(!html.contains("body.density-compact"), "{html}");
    assert!(html.contains(".footer-note {"), "{html}");
    assert!(html.contains("display: none;"), "{html}");
}
