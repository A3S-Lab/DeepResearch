// A bounded, Flint-style semantic chart contract. The model may select a
// relationship and bind fields to admitted facts; it cannot emit renderer
// code, SVG, transforms, or executable expressions. The Host validates and
// renders the resulting state deterministically.

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum TypedWireVisualizationKind {
    Bar,
    Line,
    Scatter,
    Timeline,
    Kpi,
}

impl TypedWireVisualizationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Bar => "bar",
            Self::Line => "line",
            Self::Scatter => "scatter",
            Self::Timeline => "timeline",
            Self::Kpi => "kpi",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum TypedWireVisualScale {
    Nominal,
    Quantitative,
    Temporal,
}

impl TypedWireVisualScale {
    fn as_str(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Quantitative => "quantitative",
            Self::Temporal => "temporal",
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct TypedWireVisualUnit {
    evidence_token: String,
    display_label: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct TypedWireVisualAxis {
    label: String,
    scale: TypedWireVisualScale,
    evidence_measure: Option<String>,
    unit: Option<TypedWireVisualUnit>,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct TypedWireVisualCoordinate {
    value: serde_json::Value,
    verbatim: String,
    claim_id: String,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct TypedWireVisualPoint {
    label: String,
    x: TypedWireVisualCoordinate,
    y: TypedWireVisualCoordinate,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct TypedWireVisualization {
    id: String,
    dimension_id: String,
    kind: TypedWireVisualizationKind,
    title: String,
    caption_claim_id: String,
    x_axis: TypedWireVisualAxis,
    y_axis: TypedWireVisualAxis,
    points: Vec<TypedWireVisualPoint>,
}

#[derive(Clone, Debug)]
enum AdmittedVisualValue {
    Text(String),
    Number(f64),
}

#[derive(Clone, Debug)]
struct AdmittedVisualCoordinate {
    value: AdmittedVisualValue,
    claim_number: usize,
}

#[derive(Clone, Debug)]
struct AdmittedVisualPoint {
    label: String,
    x: AdmittedVisualCoordinate,
    y: AdmittedVisualCoordinate,
}

#[derive(Clone, Debug)]
struct AdmittedTypedVisualization {
    dimension_id: String,
    kind: TypedWireVisualizationKind,
    title: String,
    caption: Option<(String, usize)>,
    x_axis: TypedWireVisualAxis,
    y_axis: TypedWireVisualAxis,
    points: Vec<AdmittedVisualPoint>,
}

fn validate_typed_visualization_shapes(
    visualizations: &[TypedWireVisualization],
) -> Result<(), String> {
    if visualizations.len() > TYPED_REPORT_MAX_VISUALIZATIONS {
        return Err("typed report visualizations exceeded their closed bound".to_string());
    }
    let mut ids = HashSet::new();
    for visualization in visualizations {
        if !typed_narrative_identifier(&visualization.id)
            || !ids.insert(visualization.id.as_str())
            || !typed_visual_text(&visualization.title, 2, TYPED_REPORT_MAX_VISUAL_TEXT_CHARS)
            || !typed_visual_text(&visualization.x_axis.label, 1, TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS)
            || !typed_visual_text(&visualization.y_axis.label, 1, TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS)
            || !typed_narrative_identifier(&visualization.caption_claim_id)
        {
            return Err("typed report visualization returned invalid identity or text".to_string());
        }
        let minimum_points = if visualization.kind == TypedWireVisualizationKind::Scatter {
            3
        } else {
            2
        };
        let maximum_points = if visualization.kind == TypedWireVisualizationKind::Kpi {
            4
        } else {
            TYPED_REPORT_MAX_VISUAL_POINTS
        };
        if !(minimum_points..=maximum_points).contains(&visualization.points.len())
            || !typed_visual_kind_matches_axes(visualization)
        {
            return Err("typed report visualization returned an invalid chart shape".to_string());
        }
        validate_typed_visual_axis(&visualization.x_axis)?;
        validate_typed_visual_axis(&visualization.y_axis)?;
        let mut point_labels = HashSet::new();
        for point in &visualization.points {
            if !typed_visual_text(&point.label, 1, TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS)
                || !point_labels.insert(point.label.as_str())
            {
                return Err("typed report visualization returned an invalid point label".to_string());
            }
            validate_typed_visual_coordinate(&point.x, visualization.x_axis.scale)?;
            validate_typed_visual_coordinate(&point.y, visualization.y_axis.scale)?;
        }
    }
    Ok(())
}

fn validate_typed_visual_axis(axis: &TypedWireVisualAxis) -> Result<(), String> {
    match axis.scale {
        TypedWireVisualScale::Quantitative => {
            let Some(measure) = axis.evidence_measure.as_deref() else {
                return Err("quantitative visualization axis omitted its evidence measure".to_string());
            };
            let Some(unit) = axis.unit.as_ref() else {
                return Err("quantitative visualization axis omitted its evidence unit".to_string());
            };
            if !typed_visual_text(measure, 1, TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS)
                || !typed_visual_text(
                    &unit.evidence_token,
                    1,
                    TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS,
                )
                || !typed_visual_text(
                    &unit.display_label,
                    1,
                    TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS,
                )
            {
                return Err("quantitative visualization axis returned invalid evidence tokens".to_string());
            }
        }
        TypedWireVisualScale::Nominal | TypedWireVisualScale::Temporal => {
            if axis.evidence_measure.is_some() || axis.unit.is_some() {
                return Err("non-quantitative visualization axis cannot declare a measure or unit".to_string());
            }
        }
    }
    Ok(())
}

fn validate_typed_visual_coordinate(
    coordinate: &TypedWireVisualCoordinate,
    scale: TypedWireVisualScale,
) -> Result<(), String> {
    if !typed_narrative_identifier(&coordinate.claim_id)
        || !typed_visual_text(
            &coordinate.verbatim,
            1,
            TYPED_REPORT_MAX_VISUAL_TEXT_CHARS,
        )
    {
        return Err("typed report visualization returned an invalid coordinate reference".to_string());
    }
    let valid_value = match scale {
        TypedWireVisualScale::Quantitative => coordinate.value.as_f64().is_some_and(f64::is_finite),
        TypedWireVisualScale::Nominal | TypedWireVisualScale::Temporal => coordinate
            .value
            .as_str()
            .is_some_and(|value| typed_visual_text(value, 1, TYPED_REPORT_MAX_VISUAL_TEXT_CHARS)),
    };
    if !valid_value {
        return Err("typed report visualization coordinate disagrees with its axis scale".to_string());
    }
    Ok(())
}

fn typed_visual_kind_matches_axes(visualization: &TypedWireVisualization) -> bool {
    matches!(
        (
            visualization.kind,
            visualization.x_axis.scale,
            visualization.y_axis.scale,
        ),
        (
            TypedWireVisualizationKind::Bar | TypedWireVisualizationKind::Kpi,
            TypedWireVisualScale::Nominal,
            TypedWireVisualScale::Quantitative,
        ) | (
            TypedWireVisualizationKind::Line,
            TypedWireVisualScale::Nominal | TypedWireVisualScale::Temporal,
            TypedWireVisualScale::Quantitative,
        ) | (
            TypedWireVisualizationKind::Scatter,
            TypedWireVisualScale::Quantitative,
            TypedWireVisualScale::Quantitative,
        ) | (
            TypedWireVisualizationKind::Timeline,
            TypedWireVisualScale::Temporal,
            TypedWireVisualScale::Nominal,
        )
    )
}

fn typed_visual_text(value: &str, minimum: usize, maximum: usize) -> bool {
    let count = value.chars().count();
    value.trim() == value
        && (minimum..=maximum).contains(&count)
        && !value.chars().any(char::is_control)
}

include!("typed_visualization_schema.rs");
include!("typed_visualization_admission.rs");
include!("typed_visualization_render.rs");
