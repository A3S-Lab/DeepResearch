fn numeric_visualization_catalog(second_unit: &str) -> DeepResearchSourceCatalog {
    catalog(vec![
        source(
            "source-1",
            "Baseline record",
            "https://example.test/baseline",
            "Baseline field recorded 32 teams in the confirmed field.",
        ),
        source(
            "source-2",
            "Expanded record",
            "https://example.test/expanded",
            &format!("Expanded field recorded 48 {second_unit} in the confirmed field."),
        ),
    ])
}

fn numeric_visualization_proposal(
    second_value: f64,
    second_unit: &str,
    second_claim_id: &str,
) -> serde_json::Value {
    let mut proposal = serde_json::json!({
        "report_language": "en",
        "labels": labels(),
        "claims": [
            fact(
                "baseline",
                "finding",
                "Baseline field recorded 32 teams in the confirmed field.",
                "source-1",
            ),
            fact(
                "expanded",
                "finding",
                &format!("Expanded field recorded 48 {second_unit} in the confirmed field."),
                "source-2",
            ),
            inference_for_dimension(
                "answer",
                "request.answer",
                "direct_answer",
                "conclusion",
                "The confirmed field expands from 32 teams to 48 teams.",
                &["baseline", "expanded"],
            ),
        ],
        "relations": [],
        "gaps": [],
        "visualizations": [{
            "id": "field-comparison",
            "dimension_id": "request.answer",
            "kind": "bar",
            "title": "The confirmed field expands from 32 teams to 48 teams",
            "caption_claim_id": "answer",
            "x_axis": {
                "label": "Format",
                "scale": "nominal",
                "evidence_measure": null,
                "unit": null
            },
            "y_axis": {
                "label": "Field size",
                "scale": "quantitative",
                "evidence_measure": "field",
                "unit": {
                    "evidence_token": "teams",
                    "display_label": "teams"
                }
            },
            "points": [{
                "label": "Baseline",
                "x": {
                    "value": "Baseline",
                    "verbatim": "Baseline",
                    "claim_id": "baseline"
                },
                "y": {
                    "value": 32,
                    "verbatim": "32 teams",
                    "claim_id": "baseline"
                }
            }, {
                "label": "Expanded",
                "x": {
                    "value": "Expanded",
                    "verbatim": "Expanded",
                    "claim_id": "expanded"
                },
                "y": {
                    "value": second_value,
                    "verbatim": format!("48 {second_unit}"),
                    "claim_id": second_claim_id
                }
            }]
        }]
    });
    proposal = with_narrative(proposal, &one_track_context());
    proposal
}

fn multi_shape_visualization_catalog() -> DeepResearchSourceCatalog {
    catalog(vec![
        source(
            "source-alpha",
            "Alpha observation",
            "https://example.test/alpha",
            "Alpha observation dated 2024 recorded latency 10 ms and throughput 100 requests.",
        ),
        source(
            "source-beta",
            "Beta observation",
            "https://example.test/beta",
            "Beta observation dated 2025 recorded latency 20 ms and throughput 140 requests.",
        ),
        source(
            "source-gamma",
            "Gamma observation",
            "https://example.test/gamma",
            "Gamma observation dated 2026 recorded latency 30 ms and throughput 180 requests.",
        ),
    ])
}

fn multi_shape_visualization_proposal(kind: &str) -> serde_json::Value {
    let points = [
        ("alpha", "Alpha", "Alpha observation", "2024", 10, 100),
        ("beta", "Beta", "Beta observation", "2025", 20, 140),
        ("gamma", "Gamma", "Gamma observation", "2026", 30, 180),
    ];
    let (x_axis, y_axis, visual_points) = match kind {
        "line" => (
            serde_json::json!({
                "label": "Observation date",
                "scale": "temporal",
                "evidence_measure": null,
                "unit": null
            }),
            serde_json::json!({
                "label": "Throughput",
                "scale": "quantitative",
                "evidence_measure": "throughput",
                "unit": {"evidence_token": "requests", "display_label": "requests"}
            }),
            points
                .iter()
                .map(|(id, label, _, year, _, throughput)| {
                    serde_json::json!({
                        "label": label,
                        "x": {
                            "value": year,
                            "verbatim": format!("dated {year}"),
                            "claim_id": id
                        },
                        "y": {
                            "value": throughput,
                            "verbatim": format!("{throughput} requests"),
                            "claim_id": id
                        }
                    })
                })
                .collect::<Vec<_>>(),
        ),
        "scatter" => (
            serde_json::json!({
                "label": "Latency",
                "scale": "quantitative",
                "evidence_measure": "latency",
                "unit": {"evidence_token": "ms", "display_label": "ms"}
            }),
            serde_json::json!({
                "label": "Throughput",
                "scale": "quantitative",
                "evidence_measure": "throughput",
                "unit": {"evidence_token": "requests", "display_label": "requests"}
            }),
            points
                .iter()
                .map(|(id, label, _, _, latency, throughput)| {
                    serde_json::json!({
                        "label": label,
                        "x": {
                            "value": latency,
                            "verbatim": format!("{latency} ms"),
                            "claim_id": id
                        },
                        "y": {
                            "value": throughput,
                            "verbatim": format!("{throughput} requests"),
                            "claim_id": id
                        }
                    })
                })
                .collect::<Vec<_>>(),
        ),
        "timeline" => (
            serde_json::json!({
                "label": "Observation date",
                "scale": "temporal",
                "evidence_measure": null,
                "unit": null
            }),
            serde_json::json!({
                "label": "Observation",
                "scale": "nominal",
                "evidence_measure": null,
                "unit": null
            }),
            points
                .iter()
                .map(|(id, label, observation, year, _, _)| {
                    serde_json::json!({
                        "label": label,
                        "x": {
                            "value": year,
                            "verbatim": format!("dated {year}"),
                            "claim_id": id
                        },
                        "y": {
                            "value": observation,
                            "verbatim": observation,
                            "claim_id": id
                        }
                    })
                })
                .collect::<Vec<_>>(),
        ),
        "kpi" => (
            serde_json::json!({
                "label": "Observation",
                "scale": "nominal",
                "evidence_measure": null,
                "unit": null
            }),
            serde_json::json!({
                "label": "Throughput",
                "scale": "quantitative",
                "evidence_measure": "throughput",
                "unit": {"evidence_token": "requests", "display_label": "requests"}
            }),
            points
                .iter()
                .map(|(id, label, observation, _, _, throughput)| {
                    serde_json::json!({
                        "label": label,
                        "x": {
                            "value": observation,
                            "verbatim": observation,
                            "claim_id": id
                        },
                        "y": {
                            "value": throughput,
                            "verbatim": format!("{throughput} requests"),
                            "claim_id": id
                        }
                    })
                })
                .collect::<Vec<_>>(),
        ),
        other => panic!("unsupported test chart kind: {other}"),
    };
    with_narrative(
        serde_json::json!({
            "report_language": "en",
            "labels": labels(),
            "claims": [
                fact(
                    "alpha",
                    "finding",
                    "Alpha observation dated 2024 recorded latency 10 ms and throughput 100 requests.",
                    "source-alpha",
                ),
                fact(
                    "beta",
                    "finding",
                    "Beta observation dated 2025 recorded latency 20 ms and throughput 140 requests.",
                    "source-beta",
                ),
                fact(
                    "gamma",
                    "finding",
                    "Gamma observation dated 2026 recorded latency 30 ms and throughput 180 requests.",
                    "source-gamma",
                ),
                inference_for_dimension(
                    "answer",
                    "request.answer",
                    "direct_answer",
                    "conclusion",
                    "The admitted observations form a directly comparable series.",
                    &["alpha", "beta", "gamma"],
                ),
            ],
            "relations": [],
            "gaps": [],
            "visualizations": [{
                "id": format!("{kind}-comparison"),
                "dimension_id": "request.answer",
                "kind": kind,
                "title": "The admitted observations form a directly comparable series",
                "caption_claim_id": "answer",
                "x_axis": x_axis,
                "y_axis": y_axis,
                "points": visual_points
            }]
        }),
        &one_track_context(),
    )
}

fn assert_fact_backed_visualization_renders(kind: &str, renderer_marker: &str) {
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the admitted observations",
        "2026-07-30",
        &multi_shape_visualization_catalog(),
        &one_track_context(),
        multi_shape_visualization_proposal(kind),
    )
    .expect("typed admission")
    .unwrap_or_else(|| panic!("{kind} report"));
    let html = report.rendered_html.expect("typed HTML");

    assert!(
        html.contains(&format!("data-a3s-chart-kind=\"{kind}\"")),
        "missing {kind} chart"
    );
    assert!(html.contains(renderer_marker), "missing {kind} marks");
    assert!(html.contains("<table"), "missing {kind} data table");
    assert!(html.contains("href=\"#claim-"), "missing {kind} evidence links");
}

#[test]
fn typed_schema_exposes_only_bounded_semantic_visualizations() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("teams");
    let schema =
        deep_research_typed_report_proposal_schema_for(&catalog, &context).expect("typed schema");
    let visuals = &schema["properties"]["visualizations"];

    assert_eq!(visuals["type"], "array");
    assert!(visuals["maxItems"].as_u64().is_some_and(|value| value > 0));
    assert!(schema["required"]
        .as_array()
        .is_some_and(|required| required.iter().any(|field| field == "visualizations")));
    let encoded = visuals.to_string();
    for kind in ["bar", "line", "scatter", "timeline", "kpi"] {
        assert!(encoded.contains(kind));
    }
    assert!(!encoded.contains("javascript"));
    assert!(!encoded.contains("svg_markup"));
    assert!(!encoded.contains("renderer_code"));
}

#[test]
fn fact_backed_values_render_as_an_accessible_inline_chart_and_table() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("teams");
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the confirmed field sizes",
        "2026-07-30",
        &catalog,
        &context,
        numeric_visualization_proposal(48.0, "teams", "expanded"),
    )
    .expect("typed admission")
    .expect("visual report");
    let html = report.rendered_html.expect("typed HTML");

    assert!(html.contains("data-a3s-visualization=\"v1\""));
    assert!(html.contains("data-a3s-chart-kind=\"bar\""));
    assert!(html.contains("<svg"));
    assert!(html.contains("role=\"img\""));
    assert!(html.contains("The confirmed field expands from 32 teams to 48 teams"));
    assert!(html.contains("<table"));
    assert!(html.contains("href=\"#claim-"));
    assert_eq!(html.matches("<script").count(), 1);
    assert!(!html.contains("https://cdn"));
}

#[test]
fn fact_backed_series_renders_as_a_line_chart() {
    assert_fact_backed_visualization_renders("line", "<polyline class=\"chart-line\"");
}

#[test]
fn fact_backed_pairs_render_as_a_scatter_chart() {
    assert_fact_backed_visualization_renders("scatter", "class=\"chart-point-label\"");
}

#[test]
fn fact_backed_events_render_as_a_timeline() {
    assert_fact_backed_visualization_renders("timeline", "chart-mark--timeline");
}

#[test]
fn fact_backed_metrics_render_as_kpi_cards() {
    assert_fact_backed_visualization_renders("kpi", "class=\"chart-kpi\"");
}

#[test]
fn fabricated_numeric_coordinate_is_omitted_without_sacrificing_the_report() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("teams");
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the confirmed field sizes",
        "2026-07-30",
        &catalog,
        &context,
        numeric_visualization_proposal(64.0, "teams", "expanded"),
    )
    .expect("typed admission")
    .expect("report remains publishable");

    assert!(report.markdown.contains("expands from 32 teams to 48 teams"));
    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}

#[test]
fn chart_title_cannot_introduce_an_unadmitted_calculation() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("teams");
    let mut proposal = numeric_visualization_proposal(48.0, "teams", "expanded");
    proposal["visualizations"][0]["title"] =
        serde_json::json!("The field more than doubles");
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the confirmed field sizes",
        "2026-07-30",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed admission")
    .expect("report remains publishable");

    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}

#[test]
fn incomparable_units_are_not_combined_into_one_chart() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("squads");
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the confirmed field sizes",
        "2026-07-30",
        &catalog,
        &context,
        numeric_visualization_proposal(48.0, "squads", "expanded"),
    )
    .expect("typed admission")
    .expect("report remains publishable");

    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}

#[test]
fn inference_claim_cannot_masquerade_as_a_raw_chart_coordinate() {
    let context = one_track_context();
    let catalog = numeric_visualization_catalog("teams");
    let report = admit_deep_research_typed_report_proposal_at(
        "compare the confirmed field sizes",
        "2026-07-30",
        &catalog,
        &context,
        numeric_visualization_proposal(48.0, "teams", "answer"),
    )
    .expect("typed admission")
    .expect("report remains publishable");

    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}

#[test]
fn report_without_chartable_data_still_publishes_without_decorative_graphics() {
    let context = one_track_context();
    let catalog = catalog(vec![source(
        "source-1",
        "Focused record",
        "https://example.test/focused",
        "The focused record establishes the complete bounded answer.",
    )]);
    let proposal = with_narrative(
        serde_json::json!({
            "report_language": "en",
            "labels": labels(),
            "claims": [fact(
                "focused-answer",
                "direct_answer",
                "The focused record establishes the complete bounded answer.",
                "source-1",
            )],
            "relations": [],
            "gaps": [],
            "visualizations": []
        }),
        &context,
    );
    let report = admit_deep_research_typed_report_proposal_at(
        "focused query",
        "2026-07-30",
        &catalog,
        &context,
        proposal,
    )
    .expect("typed admission")
    .expect("text report");

    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}

#[test]
fn visualization_labels_in_another_language_are_omitted() {
    let context = DeepResearchReportContext {
        report_title: "赛制规模研究".to_string(),
        scope: DeepResearchReportScope::Focused,
        freshness_required: false,
        tracks: vec![serde_json::json!({
            "id": "request.answer",
            "title": "参赛规模变化",
            "focus": "核实参赛规模变化。",
            "material": true,
            "completion_criteria": ["参赛规模得到直接核实。"],
            "evidence_requirements": {
                "primary_source_required": false,
                "independent_corroboration_required": false
            }
        })],
    };
    let catalog = catalog(vec![
        source(
            "source-1",
            "基准记录",
            "https://example.test/baseline",
            "基准赛制的参赛规模为32支球队。",
        ),
        source(
            "source-2",
            "扩军记录",
            "https://example.test/expanded",
            "扩军赛制的参赛规模为48支球队。",
        ),
    ]);
    let proposal = serde_json::json!({
        "report_language": "zh",
        "labels": chinese_labels(),
        "claims": [
            fact("baseline", "finding", "基准赛制的参赛规模为32支球队。", "source-1"),
            fact("expanded", "finding", "扩军赛制的参赛规模为48支球队。", "source-2"),
            inference_for_dimension(
                "answer",
                "request.answer",
                "direct_answer",
                "conclusion",
                "已核实的参赛规模从32支球队扩大到48支球队。",
                &["baseline", "expanded"],
            )
        ],
        "relations": [],
        "gaps": [],
        "visualizations": [{
            "id": "field-comparison",
            "dimension_id": "request.answer",
            "kind": "bar",
            "title": "Confirmed field size comparison",
            "caption_claim_id": "answer",
            "x_axis": {
                "label": "Tournament format",
                "scale": "nominal",
                "evidence_measure": null,
                "unit": null
            },
            "y_axis": {
                "label": "Number of teams",
                "scale": "quantitative",
                "evidence_measure": "参赛规模",
                "unit": {"evidence_token": "支球队", "display_label": "teams"}
            },
            "points": [{
                "label": "Baseline",
                "x": {"value": "基准赛制", "verbatim": "基准赛制", "claim_id": "baseline"},
                "y": {"value": 32, "verbatim": "32支球队", "claim_id": "baseline"}
            }, {
                "label": "Expanded",
                "x": {"value": "扩军赛制", "verbatim": "扩军赛制", "claim_id": "expanded"},
                "y": {"value": 48, "verbatim": "48支球队", "claim_id": "expanded"}
            }]
        }]
    });
    let report = admit_deep_research_typed_report_proposal_in_language_at(
        "研究参赛规模变化",
        "2026-07-30",
        "zh",
        &catalog,
        &context,
        with_narrative(proposal, &context),
    )
    .expect("typed admission")
    .expect("Chinese report");

    assert!(report.markdown.contains("已核实的参赛规模"));
    assert!(!report
        .rendered_html
        .expect("typed HTML")
        .contains("data-a3s-visualization=\"v1\""));
}
