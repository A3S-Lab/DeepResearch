const COMPREHENSIVE_REPORT_MIN_FINDINGS: usize = 4;
const COMPREHENSIVE_REPORT_MIN_CLAIMS: usize = 5;
const COMPREHENSIVE_REPORT_MIN_CITED_SOURCES: usize = 2;
const COMPREHENSIVE_REPORT_MIN_SUBSTANTIVE_CHARACTERS: usize = 480;

#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, serde::Deserialize, serde::Serialize,
)]
#[serde(rename_all = "snake_case")]
pub enum DeepResearchReportScope {
    #[default]
    Focused,
    Comprehensive,
}

impl DeepResearchReportScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Focused => "focused",
            Self::Comprehensive => "comprehensive",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeepResearchReportContext {
    pub report_title: String,
    pub scope: DeepResearchReportScope,
    pub freshness_required: bool,
    pub tracks: Vec<serde_json::Value>,
}

pub fn deep_research_report_context_from_plan(
    plan: &serde_json::Value,
) -> Result<DeepResearchReportContext, String> {
    let scope = match plan
        .get("research_scope")
        .and_then(serde_json::Value::as_str)
    {
        Some("focused") => DeepResearchReportScope::Focused,
        Some("comprehensive") => DeepResearchReportScope::Comprehensive,
        Some(scope) => {
            return Err(format!(
                "DeepResearch report plan has unsupported research scope `{scope}`"
            ))
        }
        None => return Err("DeepResearch report plan omitted `research_scope`".to_string()),
    };
    let freshness_required = plan
        .get("freshness_required")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            "DeepResearch report plan omitted boolean `freshness_required`".to_string()
        })?;
    let report_title = plan
        .get("report_title")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| "DeepResearch report plan omitted `report_title`".to_string())?
        .to_string();
    let raw_tracks = plan
        .get("tracks")
        .and_then(serde_json::Value::as_array)
        .filter(|tracks| !tracks.is_empty())
        .ok_or_else(|| "DeepResearch report plan omitted its semantic tracks".to_string())?;
    let mut tracks = Vec::with_capacity(raw_tracks.len());
    for track in raw_tracks.iter().take(4) {
        let object = track
            .as_object()
            .ok_or_else(|| "DeepResearch report plan contains a non-object track".to_string())?;
        let text = |field: &str| {
            object
                .get(field)
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
                .ok_or_else(|| {
                    format!("DeepResearch report plan track omitted non-empty `{field}`")
                })
        };
        let criteria = object
            .get("completion_criteria")
            .and_then(serde_json::Value::as_array)
            .filter(|criteria| !criteria.is_empty())
            .ok_or_else(|| {
                "DeepResearch report plan track omitted completion criteria".to_string()
            })?
            .iter()
            .map(|criterion| {
                criterion
                    .as_str()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        "DeepResearch report plan contains an invalid completion criterion"
                            .to_string()
                    })
            })
            .collect::<Result<Vec<_>, String>>()?;
        let evidence_requirements = object
            .get("evidence_requirements")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| {
                "DeepResearch report plan track omitted evidence requirements".to_string()
            })?;
        let primary_source_required = evidence_requirements
            .get("primary_source_required")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| {
                "DeepResearch report plan track omitted boolean `primary_source_required`"
                    .to_string()
            })?;
        let independent_corroboration_required = evidence_requirements
            .get("independent_corroboration_required")
            .and_then(serde_json::Value::as_bool)
            .ok_or_else(|| {
                "DeepResearch report plan track omitted boolean `independent_corroboration_required`"
                    .to_string()
            })?;
        tracks.push(serde_json::json!({
            "id": text("id")?,
            "title": text("title")?,
            "focus": text("focus")?,
            "material": object
                .get("material")
                .and_then(serde_json::Value::as_bool)
                .ok_or_else(|| {
                    "DeepResearch report plan track omitted boolean `material`".to_string()
                })?,
            "completion_criteria": criteria,
            "evidence_requirements": {
                "primary_source_required": primary_source_required,
                "independent_corroboration_required": independent_corroboration_required,
            },
        }));
    }
    Ok(DeepResearchReportContext {
        report_title,
        scope,
        freshness_required,
        tracks,
    })
}

#[doc(hidden)]
fn focused_report_context() -> DeepResearchReportContext {
    DeepResearchReportContext {
        report_title: "Research report".to_string(),
        scope: DeepResearchReportScope::Focused,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.primary",
            "title": "Requested answer",
            "focus": "Answer the user's request from the closed evidence.",
            "material": true,
            "completion_criteria": ["The requested answer is directly supported."],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false,
            },
        })],
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DeepResearchReportDepthRequirements {
    minimum_direct_answers: usize,
    minimum_findings: usize,
    minimum_claims: usize,
    minimum_cited_sources: usize,
    minimum_substantive_characters: usize,
}

fn deep_research_report_depth_requirements(
    scope: DeepResearchReportScope,
) -> DeepResearchReportDepthRequirements {
    match scope {
        DeepResearchReportScope::Comprehensive => DeepResearchReportDepthRequirements {
            minimum_direct_answers: 1,
            minimum_findings: COMPREHENSIVE_REPORT_MIN_FINDINGS,
            minimum_claims: COMPREHENSIVE_REPORT_MIN_CLAIMS,
            minimum_cited_sources: COMPREHENSIVE_REPORT_MIN_CITED_SOURCES,
            minimum_substantive_characters: COMPREHENSIVE_REPORT_MIN_SUBSTANTIVE_CHARACTERS,
        },
        DeepResearchReportScope::Focused => DeepResearchReportDepthRequirements {
            minimum_direct_answers: 1,
            minimum_findings: 0,
            minimum_claims: 1,
            minimum_cited_sources: 1,
            minimum_substantive_characters: 0,
        },
    }
}

fn report_substantive_character_count(text: &str) -> usize {
    text.chars()
        .filter(|character| !character.is_whitespace() && !character.is_control())
        .count()
}
