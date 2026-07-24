use super::{
    complete_html_document, deep_research_artifact_pair_has_kind, has_research_report_substance,
    read_small_utf8_file, DeepResearchArtifactKind, ResearchReportArtifacts,
};

pub(super) fn recovery_research_report_artifacts(artifacts: &ResearchReportArtifacts) -> bool {
    let markdown = read_small_utf8_file(&artifacts.markdown);
    let html = read_small_utf8_file(&artifacts.html);
    let (Some(markdown), Some(html)) = (markdown, html) else {
        return false;
    };
    deep_research_artifact_pair_has_kind(&markdown, &html, DeepResearchArtifactKind::Recovery)
        && complete_html_document(&html)
        && has_research_report_substance(&markdown, &html)
}
