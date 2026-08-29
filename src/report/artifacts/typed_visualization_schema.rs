fn typed_visualizations_schema(
    identifier: &serde_json::Value,
    dimension_ids: &[String],
) -> serde_json::Value {
    let mut coordinate_claim_id = identifier.clone();
    coordinate_claim_id["description"] = serde_json::json!(
        "An exact fact claim ID in the same dimension. Inferences and recommendations cannot supply raw chart coordinates."
    );
    let mut caption_claim_id = identifier.clone();
    caption_claim_id["description"] = serde_json::json!(
        "An admitted fact or inference claim ID in the same dimension. The title must be an exact phrase from that claim."
    );
    let unit = serde_json::json!({
        "oneOf": [
            {"type": "null"},
            {
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "evidence_token": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS,
                        "description": "One exact unit token repeated in every cited evidence excerpt on this quantitative axis. Do not convert units."
                    },
                    "display_label": {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS,
                        "description": "The reader-facing unit label in OUTPUT_LANGUAGE."
                    }
                },
                "required": ["evidence_token", "display_label"]
            }
        ]
    });
    let axis = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "label": {
                "type": "string",
                "minLength": 1,
                "maxLength": TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS
            },
            "scale": {
                "type": "string",
                "enum": ["nominal", "quantitative", "temporal"]
            },
            "evidence_measure": {
                "description": "For a quantitative axis, one exact measure token that appears in every supporting excerpt; otherwise null.",
                "oneOf": [
                    {"type": "null"},
                    {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS
                    }
                ]
            },
            "unit": unit
        },
        "required": ["label", "scale", "evidence_measure", "unit"]
    });
    let coordinate = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "value": {
                "description": "A number for a quantitative axis; otherwise an exact evidence-backed string. Never normalize, convert, estimate, or calculate a coordinate.",
                "oneOf": [
                    {"type": "number"},
                    {
                        "type": "string",
                        "minLength": 1,
                        "maxLength": TYPED_REPORT_MAX_VISUAL_TEXT_CHARS
                    }
                ]
            },
            "verbatim": {
                "type": "string",
                "minLength": 1,
                "maxLength": TYPED_REPORT_MAX_VISUAL_TEXT_CHARS,
                "description": "The shortest exact substring in the cited source chunk that proves this coordinate, including its original unit when quantitative."
            },
            "claim_id": coordinate_claim_id
        },
        "required": ["value", "verbatim", "claim_id"]
    });
    let point = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "label": {
                "type": "string",
                "minLength": 1,
                "maxLength": TYPED_REPORT_MAX_VISUAL_TOKEN_CHARS,
                "description": "A concise reader-facing point label in OUTPUT_LANGUAGE, preserving source-defined names."
            },
            "x": coordinate.clone(),
            "y": coordinate
        },
        "required": ["label", "x", "y"]
    });
    let chart = serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "id": identifier.clone(),
            "dimension_id": {
                "type": "string",
                "enum": dimension_ids
            },
            "kind": {
                "type": "string",
                "enum": ["bar", "line", "scatter", "timeline", "kpi"]
            },
            "title": {
                "type": "string",
                "minLength": 2,
                "maxLength": TYPED_REPORT_MAX_VISUAL_TEXT_CHARS,
                "description": "An insight-led chart title in OUTPUT_LANGUAGE."
            },
            "caption_claim_id": caption_claim_id,
            "x_axis": axis.clone(),
            "y_axis": axis,
            "points": {
                "type": "array",
                "minItems": 2,
                "maxItems": TYPED_REPORT_MAX_VISUAL_POINTS,
                "items": point
            }
        },
        "required": [
            "id", "dimension_id", "kind", "title", "caption_claim_id",
            "x_axis", "y_axis", "points"
        ]
    });
    serde_json::json!({
        "type": "array",
        "maxItems": TYPED_REPORT_MAX_VISUALIZATIONS.min(dimension_ids.len()),
        "description": "Optional bounded Flint-style semantic charts. Return [] unless the closed fact graph contains at least two directly comparable values or events. The Host, never the model, renders admitted state.",
        "items": chart
    })
}
