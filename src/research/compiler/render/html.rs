use super::shared::RenderContext;
use crate::report::html_host::{render_report_menu, report_host_script_element, REPORT_HOST_CSS};
use crate::report::html_style::REPORT_CSS;
use crate::research::compiler::{
    ClaimKind, ReportClaim, ReportDimension, ReportDocumentKind, ReportRelation,
};

pub(super) fn render(context: &RenderContext<'_>) -> String {
    let document = context.document;
    let labels = context.labels;
    let report_status = if document.kind == ReportDocumentKind::NoEvidence {
        labels.no_evidence.as_str()
    } else {
        labels.source_backed.as_str()
    };
    let report_menu = render_report_menu(&document.language, report_status, None);
    let mut output = String::new();
    output.push_str("<!doctype html>\n");
    output.push_str(&format!(
        "<html lang=\"{}\">\n<head>\n",
        escape_attribute(&document.language)
    ));
    output.push_str("<meta charset=\"utf-8\">\n");
    output.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    output.push_str(&format!(
        "<title>{}</title>\n<style>{}{}</style>\n</head>\n<body class=\"a3s-report\" data-a3s-report-state=\"readonly\">\n",
        escape_html(&document.title),
        REPORT_CSS,
        REPORT_HOST_CSS,
    ));
    output.push_str(&format!(
        "<a class=\"skip-link\" href=\"#report-main\">{}</a>\n",
        escape_html(&labels.skip_to_report)
    ));
    output.push_str("<div class=\"report-shell\">\n");
    output.push_str(&report_menu);
    output.push('\n');
    output.push_str(&format!(
        "<nav aria-label=\"{}\" class=\"report-nav\">\n<div class=\"report-nav__context\" aria-hidden=\"true\"><span>{}</span></div>\n<div class=\"report-nav__track\">\n",
        escape_attribute(&labels.report_sections),
        escape_html(&labels.report_sections),
    ));
    let mut nav_ordinal = 1usize;
    if !document.direct_answer_claims.is_empty() {
        output.push_str(&format!(
            "<a href=\"#direct-answer\"><span class=\"report-nav__index\">{nav_ordinal:02}</span><span class=\"report-nav__text\">{}</span></a>\n",
            escape_html(&labels.direct_answer),
        ));
        nav_ordinal += 1;
    }
    for (index, dimension) in document.dimensions.iter().enumerate() {
        output.push_str(&format!(
            "<a href=\"#dimension-{}\"><span class=\"report-nav__index\">{nav_ordinal:02}</span><span class=\"report-nav__text\">{}</span></a>\n",
            index + 1,
            escape_html(&dimension.heading),
        ));
        nav_ordinal += 1;
    }
    output.push_str(&format!(
        "<a href=\"#sources\"><span class=\"report-nav__index\">{nav_ordinal:02}</span><span class=\"report-nav__text\">{}</span></a>\n</div>\n</nav>\n",
        escape_html(&labels.sources),
    ));
    output.push_str("<div class=\"report-column\">\n<header class=\"report-hero\" data-a3s-editable-region contenteditable=\"false\" spellcheck=\"false\">\n");
    output.push_str(&format!("<h1>{}</h1>\n", escape_html(&document.title)));
    output.push_str("</header>\n");
    output.push_str(
        "<main id=\"report-main\" data-a3s-deep-research-document=\"v1\">\n<article id=\"report\" data-a3s-editable-region contenteditable=\"false\" spellcheck=\"false\">\n",
    );
    if !document.direct_answer_claims.is_empty() {
        output.push_str("<section id=\"direct-answer\" class=\"report-section direct-answer\">\n");
        output.push_str("<div class=\"section-index\">01</div>\n");
        output.push_str(&format!(
            "<h2>{}</h2>\n",
            escape_html(&labels.direct_answer)
        ));
        let paragraph_claim_ids = document
            .direct_answer_claims
            .iter()
            .map(|claim| vec![claim.id.clone()])
            .collect::<Vec<_>>();
        render_narrative(
            &mut output,
            context,
            &document.direct_answer_claims,
            &paragraph_claim_ids,
        );
        render_traceability(&mut output, context, &document.direct_answer_claims);
        output.push_str("</section>\n");
    }
    for (index, dimension) in document.dimensions.iter().enumerate() {
        render_dimension(&mut output, context, dimension, index + 1);
    }
    let sources_ordinal =
        usize::from(!document.direct_answer_claims.is_empty()) + document.dimensions.len() + 1;
    output.push_str("<section id=\"sources\" class=\"report-section sources\">\n");
    output.push_str(&format!(
        "<div class=\"section-index\">{sources_ordinal:02}</div>\n"
    ));
    output.push_str(&format!(
        "<h2>{}</h2>\n<ol>\n",
        escape_html(&labels.sources)
    ));
    for source in &document.source_ledger {
        output.push_str(&format!(
            "<li id=\"source-{}\"><strong>[{}]</strong> ",
            source.number, source.number
        ));
        if safe_https_anchor(&source.canonical_anchor) {
            output.push_str(&format!(
                "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                escape_attribute(&source.canonical_anchor),
                escape_html(&source.title)
            ));
        } else {
            output.push_str(&format!(
                "<strong>{}</strong> <code>{}</code>",
                escape_html(&source.title),
                escape_html(&source.canonical_anchor)
            ));
        }
        output.push_str(&format!(
            "<span class=\"source-meta\">{}: {}</span>",
            escape_html(&labels.captured),
            escape_html(&source.captured_at)
        ));
        if source.requested_anchor != source.canonical_anchor {
            output.push_str(&format!(
                "<span class=\"source-meta\">{}: ",
                escape_html(&labels.requested_as)
            ));
            if safe_https_anchor(&source.requested_anchor) {
                output.push_str(&format!(
                    "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">{}</a>",
                    escape_attribute(&source.requested_anchor),
                    escape_html(&source.requested_anchor)
                ));
            } else {
                output.push_str(&format!(
                    "<code>{}</code>",
                    escape_html(&source.requested_anchor)
                ));
            }
            output.push_str("</span>");
        }
        output.push_str("</li>\n");
    }
    output.push_str("</ol>\n</section>\n</article>\n</main>\n</div>\n</div>\n");
    output.push_str(&report_host_script_element());
    output.push_str("\n</body>\n</html>\n");
    output
}

fn render_dimension(
    output: &mut String,
    context: &RenderContext<'_>,
    dimension: &ReportDimension,
    ordinal: usize,
) {
    let labels = context.labels;
    output.push_str(&format!(
        "<section id=\"dimension-{ordinal}\" class=\"report-section dimension\">\n<div class=\"section-index\">{:02}</div>\n<h2>{}</h2>\n",
        ordinal + usize::from(!context.document.direct_answer_claims.is_empty()),
        escape_html(&dimension.heading)
    ));
    if dimension.claims.is_empty() && dimension.relations.is_empty() && dimension.gaps.is_empty() {
        output.push_str(&format!(
            "<p class=\"coverage\">{}</p>\n",
            escape_html(context.coverage_label(dimension.coverage))
        ));
    }
    if !dimension.claims.is_empty() {
        render_narrative(
            output,
            context,
            &dimension.claims,
            &dimension.paragraph_claim_ids,
        );
        render_traceability(output, context, &dimension.claims);
    }
    if !dimension.relations.is_empty() {
        output.push_str(&format!(
            "<div class=\"relations\"><h3>{}</h3>\n",
            escape_html(&labels.contradiction)
        ));
        for relation in &dimension.relations {
            render_relation(output, context, relation);
        }
        output.push_str("</div>\n");
    }
    if !dimension.gaps.is_empty() {
        output.push_str(&format!(
            "<aside class=\"limitations\"><h3>{}</h3><ul>\n",
            escape_html(&labels.limitations)
        ));
        for gap in &dimension.gaps {
            output.push_str(&format!("<li>{}</li>\n", escape_html(&gap.text)));
        }
        output.push_str("</ul></aside>\n");
    }
    if context.document.kind == ReportDocumentKind::SourceBacked && !dimension.source_ids.is_empty()
    {
        output.push_str(&format!(
            "<div class=\"retained-excerpts\"><h3>{}</h3>\n",
            escape_html(&labels.retained_excerpts)
        ));
        for source_id in &dimension.source_ids {
            let Some(source) = context.source(source_id) else {
                continue;
            };
            output.push_str(&format!(
                "<article class=\"source-excerpt\"><h4><a href=\"#source-{}\">[{}]</a> {}</h4>\n",
                source.number,
                source.number,
                escape_html(&source.title)
            ));
            for chunk in &source.chunks {
                output.push_str(&format!(
                    "<pre><code>{}</code></pre>\n",
                    escape_html(&chunk.text)
                ));
            }
            output.push_str("</article>\n");
        }
        output.push_str("</div>\n");
    }
    output.push_str("</section>\n");
}

fn render_narrative(
    output: &mut String,
    context: &RenderContext<'_>,
    claims: &[ReportClaim],
    paragraph_claim_ids: &[Vec<String>],
) {
    output.push_str("<div class=\"narrative\">\n");
    for paragraph in context.narrative_paragraphs(claims, paragraph_claim_ids) {
        let paragraph_class = if paragraph
            .iter()
            .any(|claim| claim.kind == ClaimKind::Recommendation)
        {
            " report-paragraph--implication"
        } else {
            ""
        };
        output.push_str(&format!("<p class=\"report-paragraph{paragraph_class}\">"));
        for (index, claim) in paragraph.iter().enumerate() {
            if index > 0 {
                output.push(' ');
            }
            render_claim_sentence(output, context, claim);
        }
        output.push_str("</p>\n");
    }
    output.push_str("</div>\n");
}

fn render_claim_sentence(output: &mut String, context: &RenderContext<'_>, claim: &ReportClaim) {
    let number = context
        .claim_number(&claim.id)
        .expect("every report claim has a presentation number");
    let kind_class = match claim.kind {
        ClaimKind::Fact => "fact",
        ClaimKind::Inference => "inference",
        ClaimKind::Recommendation => "recommendation",
    };
    output.push_str(&format!(
        "<span id=\"claim-{number}\" class=\"claim-sentence {kind_class}\">"
    ));
    output.push_str(&escape_html(&claim.text));
    render_citations(output, claim);
    output.push_str("</span>");
}

fn render_citations(output: &mut String, claim: &ReportClaim) {
    for citation in &claim.citation_numbers {
        output.push_str(&format!(
            " <a class=\"citation\" href=\"#source-{citation}\">[{citation}]</a>"
        ));
    }
}

fn render_traceability(output: &mut String, context: &RenderContext<'_>, claims: &[ReportClaim]) {
    let traceable_claims = claims
        .iter()
        .filter(|claim| !claim.basis_claim_ids.is_empty() || claim.derivation.is_some())
        .collect::<Vec<_>>();
    if traceable_claims.is_empty() {
        return;
    }
    output.push_str(&format!(
        "<details class=\"traceability\"><summary>{}</summary><ol>\n",
        escape_html(&context.labels.basis)
    ));
    for claim in traceable_claims {
        let number = context
            .claim_number(&claim.id)
            .expect("every report claim has a presentation number");
        let label = match claim.kind {
            ClaimKind::Fact => context.labels.finding.as_str(),
            ClaimKind::Inference => context.labels.inference.as_str(),
            ClaimKind::Recommendation => context.labels.recommendation.as_str(),
        };
        output.push_str(&format!(
            "<li><a href=\"#claim-{number}\">{} {number}</a>",
            escape_html(label)
        ));
        let basis = claim
            .basis_claim_ids
            .iter()
            .filter_map(|claim_id| context.claim_number(claim_id))
            .collect::<Vec<_>>();
        if !basis.is_empty() {
            output.push_str(&format!(
                "<span class=\"traceability__basis\"><strong>{}:</strong> ",
                escape_html(&context.labels.basis)
            ));
            for (index, basis_number) in basis.iter().enumerate() {
                if index > 0 {
                    output.push_str(", ");
                }
                output.push_str(&format!(
                    "<a href=\"#claim-{basis_number}\">{} {basis_number}</a>",
                    escape_html(&context.labels.finding)
                ));
            }
            output.push_str("</span>");
        }
        if let Some(derivation) = &claim.derivation {
            output.push_str(&format!(
                "<span class=\"traceability__derivation\"><strong>{}:</strong> {}</span>",
                escape_html(&context.labels.derivation),
                escape_html(&derivation.method)
            ));
        }
        output.push_str("</li>\n");
    }
    output.push_str("</ol></details>\n");
}

fn render_relation(output: &mut String, context: &RenderContext<'_>, relation: &ReportRelation) {
    let references = relation
        .claim_ids
        .iter()
        .filter_map(|claim_id| context.claim_number(claim_id))
        .collect::<Vec<_>>();
    if references.len() == 2 {
        output.push_str(&format!(
            "<p class=\"contradiction\"><strong>{}:</strong> <a href=\"#claim-{}\">{} {}</a> / <a href=\"#claim-{}\">{} {}</a>.</p>\n",
            escape_html(&context.labels.contradiction),
            references[0],
            escape_html(&context.labels.finding),
            references[0],
            references[1],
            escape_html(&context.labels.finding),
            references[1]
        ));
    }
}

fn escape_html(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(character),
        }
    }
    escaped
}

fn escape_attribute(value: &str) -> String {
    escape_html(value)
}

fn safe_https_anchor(value: &str) -> bool {
    reqwest::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}
