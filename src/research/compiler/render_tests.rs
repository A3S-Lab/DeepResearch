use super::frozen_fixture::{load_frozen_replays, FrozenFault, FrozenReplay};
use super::*;

fn replay(case_id: &str) -> FrozenReplay {
    load_frozen_replays()
        .into_iter()
        .find(|replay| replay.id == case_id)
        .unwrap_or_else(|| panic!("missing frozen replay `{case_id}`"))
}

fn claim_document(replay: &FrozenReplay) -> ReportDocument {
    let ledger = admit_claim_ledger(&replay.contract, &replay.catalog, replay.proposal.clone())
        .unwrap_or_else(|error| panic!("{}: admit frozen ledger: {error}", replay.id));
    build_report_document(&replay.contract, &replay.catalog, &ledger)
        .unwrap_or_else(|error| panic!("{}: build frozen document: {error}", replay.id))
}

fn document_claims(document: &ReportDocument) -> impl Iterator<Item = &ReportClaim> {
    document.direct_answer_claims.iter().chain(
        document
            .dimensions
            .iter()
            .flat_map(|dimension| dimension.claims.iter()),
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[test]
fn markdown_and_html_project_the_same_frozen_document_content() {
    for replay in load_frozen_replays() {
        let document = if matches!(
            replay.fault.as_ref(),
            Some(FrozenFault::ReportGenerationTimeout)
        ) {
            build_source_backed_document(&replay.contract, &replay.catalog)
                .expect("source-backed frozen document")
        } else {
            claim_document(&replay)
        };
        let rendered = render_report_document(&document);

        assert!(rendered.markdown.contains(&document.title), "{}", replay.id);
        assert!(rendered.html.contains(&document.title), "{}", replay.id);
        for claim in document_claims(&document) {
            assert_eq!(
                rendered.markdown.matches(&claim.text).count(),
                1,
                "{}: Markdown claim `{}`",
                replay.id,
                claim.id
            );
            assert_eq!(
                rendered.html.matches(&claim.text).count(),
                1,
                "{}: HTML claim `{}`",
                replay.id,
                claim.id
            );
        }
        for dimension in &document.dimensions {
            assert!(
                rendered.markdown.contains(&dimension.heading),
                "{}",
                replay.id
            );
            assert!(
                rendered.html.contains(&html_escape(&dimension.heading)),
                "{}",
                replay.id
            );
            for gap in &dimension.gaps {
                assert!(rendered.markdown.contains(&gap.text), "{}", replay.id);
                assert!(
                    rendered.html.contains(&html_escape(&gap.text)),
                    "{}",
                    replay.id
                );
            }
        }
        for source in &document.source_ledger {
            assert!(rendered.markdown.contains(&source.title), "{}", replay.id);
            assert!(rendered.html.contains(&source.title), "{}", replay.id);
            let citation = format!("[{}]", source.number);
            assert!(rendered.markdown.contains(&citation), "{}", replay.id);
            assert!(rendered.html.contains(&citation), "{}", replay.id);
        }
    }
}

#[test]
fn f01_renders_typed_contradiction_without_internal_claim_ids() {
    let replay = replay("F01");
    let document = claim_document(&replay);

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains("Contradiction"));
    assert!(rendered.html.contains("Contradiction"));
    assert!(rendered.markdown.contains("[1](#claim-1)"));
    assert!(rendered.markdown.contains("[2](#claim-2)"));
    assert!(!rendered.markdown.contains("announcement-date"));
    assert!(!rendered.html.contains("deployment-record-date"));
}

#[test]
fn f04_uses_authored_reader_labels_in_both_artifacts() {
    let replay = replay("F04");
    let document = claim_document(&replay);

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains("## 直接结论"));
    assert!(rendered.markdown.contains("## 研究维度"));
    assert!(rendered.markdown.contains("## 来源"));
    assert!(rendered.html.contains("<html lang=\"zh\">"));
    assert!(rendered.html.contains("直接结论"));
    assert!(!rendered.html.contains("Direct Answer"));
}

#[test]
fn claim_reports_render_an_argument_as_prose_instead_of_a_summary_list() {
    let replay = replay("F08");
    let document = claim_document(&replay);

    let rendered = render_report_document(&document);

    assert!(
        !rendered
            .markdown
            .contains("- Cedar 6 requires Rust 1.82 or newer."),
        "{}",
        rendered.markdown
    );
    assert!(
        !rendered.markdown.contains(
            "- The Acme production toolchain remains pinned to Rust 1.78 through the fourth quarter of 2026."
        ),
        "{}",
        rendered.markdown
    );
    assert!(
        rendered
            .markdown
            .contains("Acme should defer Cedar 6 adoption"),
        "{}",
        rendered.markdown
    );
    assert_eq!(
        rendered
            .html
            .matches("class=\"claim-sentence fact\"")
            .count(),
        2,
        "{}",
        rendered.html
    );
    assert!(rendered
        .html
        .contains("Cedar 6 requires Rust 1.82 or newer."));
    assert!(rendered.html.contains(
        "The Acme production toolchain remains pinned to Rust 1.78 through the fourth quarter of 2026."
    ));
    assert!(
        rendered
            .html
            .contains("class=\"claim-sentence recommendation\""),
        "{}",
        rendered.html
    );
    assert!(rendered.html.contains("<details class=\"traceability\">"));
    assert!(!rendered
        .html
        .contains("<details class=\"traceability\" open>"));
    assert!(
        !rendered.html.contains("<h3>Findings</h3>")
            && !rendered.html.contains("<h3>Analysis</h3>"),
        "{}",
        rendered.html
    );
}

#[test]
fn dimension_arguments_render_as_continuous_prose_before_the_boundary() {
    let replay = replay("F08");
    let mut document = claim_document(&replay);
    let mut implication = document.direct_answer_claims[0].clone();
    implication.id = "dimension-implication".to_string();
    implication.dimension_id = document.dimensions[0].dimension_id.clone();
    implication.placement = ClaimPlacement::Finding;
    implication.text =
        "The combined constraints make a staged adoption decision necessary.".to_string();
    document.dimensions[0].claims.push(implication);
    document.dimensions[0].gaps.push(ReportGap {
        id: "bounded-gap".to_string(),
        text: "The retained evidence does not establish the final rollout date.".to_string(),
        attempted_query_ids: Vec::new(),
        missing_source_target_ids: Vec::new(),
        origin: ReportGapOrigin::ModelProposed,
    });

    let rendered = render_report_document(&document);
    let evidence = rendered
        .html
        .find("Cedar 6 requires Rust 1.82 or newer.")
        .expect("evidence");
    let implication = rendered
        .html
        .find("The combined constraints make a staged adoption decision necessary.")
        .expect("implication");
    let boundary = rendered
        .html
        .find("<aside class=\"limitations\"><h3>Evidence Boundaries</h3>")
        .expect("boundary");

    assert!(evidence < implication);
    assert!(implication < boundary);
    assert!(!rendered.html.contains("<h3>Findings</h3>"));
    assert!(!rendered.html.contains("<h3>Recommendation</h3>"));
    assert!(!rendered
        .html
        .contains("Cedar 6 requires Rust 1.82 or newer. The Acme production"));
}

#[test]
fn direct_answers_from_different_dimensions_render_as_separate_paragraphs() {
    let replay = replay("F04");
    let mut document = claim_document(&replay);
    let mut second_answer = document.direct_answer_claims[0].clone();
    second_answer.id = "second-dimension-answer".to_string();
    second_answer.dimension_id = "another-dimension".to_string();
    second_answer.text = "A separate dimension has its own conclusion.".to_string();
    document.direct_answer_claims.push(second_answer);

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains(
        "Northwind SDK 3.0 支持 Linux 和 macOS。 [1](#source-1)\n\n<a id=\"claim-2\"></a>A separate dimension has its own conclusion."
    ));
    let direct_answer = rendered
        .html
        .split_once("<section id=\"direct-answer\"")
        .and_then(|(_, section)| section.split_once("</section>"))
        .map(|(section, _)| section)
        .expect("direct-answer section");
    assert_eq!(
        direct_answer
            .matches("<p class=\"report-paragraph\">")
            .count(),
        2
    );
}

#[test]
fn reader_labels_are_not_inferred_from_the_language_code() {
    let replay = replay("F01");
    let mut document = claim_document(&replay);
    document.language = "en".to_string();
    document.reader_labels = super::test_support::reader_labels("zh");

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains("## 直接结论"));
    assert!(rendered.html.contains("<html lang=\"en\">"));
    assert!(rendered.html.contains("跳转到报告"));
    assert!(!rendered.html.contains("Skip to report"));
}

#[test]
fn dimension_coverage_avoids_redundant_status_pairs() {
    let replay = replay("F08");
    let mut document = claim_document(&replay);
    document.reader_labels.status = "Boundary".to_string();
    document.reader_labels.coverage_claims = "Answered above".to_string();
    let dimension = &mut document.dimensions[0];
    dimension.coverage = StructuralCoverage::ClaimsOnly;
    dimension.claims.clear();
    dimension.relations.clear();
    dimension.gaps.clear();

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains("*Answered above*"));
    assert!(!rendered.markdown.contains("**Boundary:**"));
    assert!(rendered
        .html
        .contains("<p class=\"coverage\">Answered above</p>"));
    assert!(!rendered.html.contains("<span>Boundary</span>"));
}

#[test]
fn f06_source_backed_fallback_renders_retained_excerpts_and_reader_safe_limits() {
    let replay = replay("F06");
    let document =
        build_source_backed_document(&replay.contract, &replay.catalog).expect("F06 fallback");

    let rendered = render_report_document(&document);

    assert!(rendered.markdown.contains("30 September 2027"));
    assert!(rendered.html.contains("30 September 2027"));
    for forbidden_internal_term in ["workflow", "model", "claim synthesis", "packet"] {
        assert!(
            !rendered
                .markdown
                .to_ascii_lowercase()
                .contains(forbidden_internal_term),
            "{forbidden_internal_term}"
        );
        assert!(
            !rendered
                .html
                .to_ascii_lowercase()
                .contains(forbidden_internal_term),
            "{forbidden_internal_term}"
        );
    }
}

#[test]
fn html_is_responsive_printable_and_does_not_depend_on_markdown_parsing() {
    let replay = replay("F08");
    let document = claim_document(&replay);

    let rendered = render_report_document(&document);

    assert!(rendered.html.starts_with("<!doctype html>"));
    assert!(rendered
        .html
        .contains("name=\"viewport\" content=\"width=device-width, initial-scale=1\""));
    assert!(rendered.html.contains("@media (max-width: 640px)"));
    assert!(rendered.html.contains("@media print"));
    assert!(rendered.html.contains("overflow-wrap: anywhere"));
    assert!(rendered.html.contains("flex-wrap: wrap"));
    assert!(rendered
        .html
        .contains("grid-template-columns: 176px minmax(0, 1fr) 232px;"));
    assert!(rendered.html.contains("grid-column: 3;"));
    assert!(rendered
        .html
        .contains("<body class=\"a3s-report\" data-a3s-report-state=\"readonly\">"));
    assert!(rendered.html.contains("--a3s-bg: #f7f7f8"));
    assert!(rendered.html.contains("--a3s-panel: #ffffff"));
    assert!(rendered.html.contains("--a3s-ink: #17181a"));
    assert!(rendered.html.contains("--a3s-blue: #2864e8"));
    assert!(rendered.html.contains("--a3s-action: #242424"));
    assert!(!rendered.html.contains("linear-gradient"));
    assert!(!rendered.html.contains("archetype-"));
    assert!(!rendered.html.contains("palette-"));
    assert!(rendered.html.contains("<main"));
    assert!(rendered
        .html
        .contains("<nav aria-label=\"Report sections\""));
    assert!(rendered.html.contains("class=\"report-menu\""));
    assert!(rendered.html.contains("data-a3s-action=\"edit\""));
    assert!(rendered.html.contains("data-a3s-action=\"save\""));
    assert!(rendered.html.contains("data-a3s-action=\"print\""));
    assert_eq!(rendered.html.matches("<script").count(), 1);
    let menu = rendered
        .html
        .find("class=\"report-menu\"")
        .expect("report menu");
    let navigation = rendered
        .html
        .find("<nav aria-label=\"Report sections\"")
        .expect("report ToC");
    let report_column = rendered
        .html
        .find("<div class=\"report-column\">")
        .expect("report content column");
    assert!(menu < navigation && navigation < report_column);
    assert!(!rendered.html.contains("<article><h1>"));
}

#[test]
fn renderers_escape_untrusted_prose_and_only_link_https_sources() {
    let replay = replay("F08");
    let mut document = claim_document(&replay);
    document.direct_answer_claims[0].text =
        "<script>alert('x')</script> [unsafe](javascript:alert(1))".to_string();

    let rendered = render_report_document(&document);

    assert!(!rendered.markdown.contains("<script>"));
    assert!(rendered.markdown.contains("&lt;script&gt;"));
    assert_eq!(rendered.html.matches("<script").count(), 1);
    assert!(!rendered.html.contains("<script>alert"));
    assert!(rendered.html.contains("&lt;script&gt;"));
    assert!(!rendered.html.contains("href=\"local://"));
    assert!(!rendered.markdown.contains("](local://"));
    assert!(rendered
        .html
        .contains("href=\"https://docs.example.test/cedar/6/requirements\""));
}
