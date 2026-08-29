fn render_typed_visualizations_into_html(
    html: &str,
    visualizations: &[AdmittedTypedVisualization],
    context: &DeepResearchReportContext,
) -> String {
    let mut output = html.to_string();
    let mut figure_ordinal = 0usize;
    for (dimension_index, track) in context.tracks.iter().enumerate() {
        let Some(dimension_id) = track.get("id").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(visualization) = visualizations
            .iter()
            .find(|visualization| visualization.dimension_id == dimension_id)
        else {
            continue;
        };
        let marker = format!("<section id=\"dimension-{}\"", dimension_index + 1);
        let Some(section_start) = output.find(&marker) else {
            continue;
        };
        let Some(section_end_relative) = output[section_start..].find("</section>") else {
            continue;
        };
        let section_end = section_start + section_end_relative;
        let section = &output[section_start..section_end];
        let insertion_relative = [
            "<details class=\"traceability\"",
            "<div class=\"relations\"",
            "<aside class=\"limitations\"",
            "<div class=\"retained-excerpts\"",
        ]
        .into_iter()
        .filter_map(|candidate| section.find(candidate))
        .min()
        .unwrap_or(section.len());
        figure_ordinal += 1;
        output.insert_str(
            section_start + insertion_relative,
            &render_typed_visualization(visualization, figure_ordinal),
        );
    }
    output
}

fn render_typed_visualization(
    visualization: &AdmittedTypedVisualization,
    ordinal: usize,
) -> String {
    let title_id = format!("visualization-{ordinal}-title");
    let description_id = format!("visualization-{ordinal}-description");
    let axis_summary = format!(
        "{} · {}",
        typed_visual_axis_label(&visualization.x_axis),
        typed_visual_axis_label(&visualization.y_axis)
    );
    let description = typed_visual_accessible_description(visualization);
    let svg = match visualization.kind {
        TypedWireVisualizationKind::Bar => render_typed_bar_chart(visualization),
        TypedWireVisualizationKind::Line => render_typed_line_chart(visualization),
        TypedWireVisualizationKind::Scatter => render_typed_scatter_chart(visualization),
        TypedWireVisualizationKind::Timeline => render_typed_timeline(visualization),
        TypedWireVisualizationKind::Kpi => render_typed_kpi_chart(visualization),
    };
    let caption = visualization.caption.as_ref().map_or_else(String::new, |(text, number)| {
        format!(
            "<figcaption><a class=\"citation\" href=\"#claim-{number}\">[{number}]</a> {}</figcaption>\n",
            typed_visual_escape(text)
        )
    });
    format!(
        "<figure class=\"research-visual research-visual--{kind}\" data-a3s-visualization=\"v1\" data-a3s-chart-kind=\"{kind}\" data-a3s-x-scale=\"{x_scale}\" data-a3s-y-scale=\"{y_scale}\" aria-labelledby=\"{title_id}\">\n<div class=\"research-visual__header\"><h3 id=\"{title_id}\">{title}</h3><p>{axis_summary}</p></div>\n<div class=\"research-visual__plot\"><svg viewBox=\"0 0 760 400\" role=\"img\" aria-labelledby=\"{title_id} {description_id}\" preserveAspectRatio=\"xMidYMid meet\"><title>{title}</title><desc id=\"{description_id}\">{description}</desc>{svg}</svg></div>\n{table}{caption}</figure>\n",
        kind = visualization.kind.as_str(),
        x_scale = visualization.x_axis.scale.as_str(),
        y_scale = visualization.y_axis.scale.as_str(),
        title = typed_visual_escape(&visualization.title),
        axis_summary = typed_visual_escape(&axis_summary),
        description = typed_visual_escape(&description),
        table = render_typed_visual_data_table(visualization),
    )
}

fn render_typed_visual_data_table(visualization: &AdmittedTypedVisualization) -> String {
    let mut rows = String::new();
    for point in &visualization.points {
        let x_label = typed_visual_coordinate_label(&point.x, &visualization.x_axis);
        let raw_x = if typed_visual_contains_case_insensitive(&point.label, &x_label)
            && typed_visual_contains_case_insensitive(&x_label, &point.label)
        {
            String::new()
        } else {
            format!(
                "<span class=\"research-visual__raw\">{}</span>",
                typed_visual_escape(&x_label)
            )
        };
        let x_claim_link = if point.x.claim_number == point.y.claim_number {
            String::new()
        } else {
            typed_visual_claim_link(point.x.claim_number)
        };
        rows.push_str("<tr>");
        rows.push_str(&format!(
            "<th scope=\"row\">{}{raw_x}{x_claim_link}</th>",
            typed_visual_escape(&point.label),
        ));
        rows.push_str(&format!(
            "<td>{}{}</td>",
            typed_visual_escape(&typed_visual_coordinate_label(
                &point.y,
                &visualization.y_axis,
            )),
            typed_visual_claim_link(point.y.claim_number),
        ));
        rows.push_str("</tr>\n");
    }
    format!(
        "<div class=\"table-wrap research-visual__table\" role=\"region\" aria-label=\"{title}\" tabindex=\"0\"><table><caption class=\"sr-only\">{title}</caption><thead><tr><th scope=\"col\">{x}</th><th scope=\"col\">{y}</th></tr></thead><tbody>{rows}</tbody></table></div>\n",
        title = typed_visual_escape(&visualization.title),
        x = typed_visual_escape(&typed_visual_axis_label(&visualization.x_axis)),
        y = typed_visual_escape(&typed_visual_axis_label(&visualization.y_axis)),
    )
}

fn render_typed_bar_chart(visualization: &AdmittedTypedVisualization) -> String {
    let values = typed_visual_axis_numbers(&visualization.points, false);
    let Some((minimum, maximum)) = typed_visual_domain(&values, true) else {
        return String::new();
    };
    let mut output = render_typed_quantitative_grid(
        minimum,
        maximum,
        &visualization.y_axis,
        false,
    );
    let plot_left = 84.0;
    let plot_right = 724.0;
    let plot_top = 34.0;
    let plot_bottom = 318.0;
    let slot = (plot_right - plot_left) / visualization.points.len() as f64;
    let bar_width = (slot * 0.58).min(72.0);
    let zero_y = typed_visual_scale(0.0, minimum, maximum, plot_bottom, plot_top);
    for (index, point) in visualization.points.iter().enumerate() {
        let Some(value) = typed_visual_number(&point.y) else {
            continue;
        };
        let value_y = typed_visual_scale(value, minimum, maximum, plot_bottom, plot_top);
        let x = plot_left + slot * index as f64 + (slot - bar_width) / 2.0;
        let y = value_y.min(zero_y);
        let height = (value_y - zero_y).abs().max(1.0);
        output.push_str(&format!(
            "<g class=\"chart-mark\"><title>{}: {}</title><rect x=\"{x:.2}\" y=\"{y:.2}\" width=\"{bar_width:.2}\" height=\"{height:.2}\" rx=\"5\"></rect><text class=\"chart-value\" x=\"{center:.2}\" y=\"{label_y:.2}\" text-anchor=\"middle\">{value_label}</text><text class=\"chart-category\" x=\"{center:.2}\" y=\"346\" text-anchor=\"middle\">{category}</text></g>",
            typed_visual_escape(&point.label),
            typed_visual_escape(&typed_visual_coordinate_label(&point.y, &visualization.y_axis)),
            center = x + bar_width / 2.0,
            label_y = if value >= 0.0 { y - 8.0 } else { y + height + 17.0 },
            value_label = typed_visual_escape(&typed_visual_format_number(value)),
            category = typed_visual_escape(&typed_visual_truncate(&point.label, 16)),
        ));
    }
    output.push_str(&render_typed_axis_titles(visualization));
    output
}

fn render_typed_line_chart(visualization: &AdmittedTypedVisualization) -> String {
    let values = typed_visual_axis_numbers(&visualization.points, false);
    let Some((minimum, maximum)) = typed_visual_domain(&values, true) else {
        return String::new();
    };
    let mut output = render_typed_quantitative_grid(
        minimum,
        maximum,
        &visualization.y_axis,
        false,
    );
    let left = 96.0;
    let right = 712.0;
    let top = 34.0;
    let bottom = 318.0;
    let step = if visualization.points.len() == 1 {
        0.0
    } else {
        (right - left) / (visualization.points.len() - 1) as f64
    };
    let coordinates = visualization
        .points
        .iter()
        .enumerate()
        .filter_map(|(index, point)| {
            let value = typed_visual_number(&point.y)?;
            Some((
                left + step * index as f64,
                typed_visual_scale(value, minimum, maximum, bottom, top),
                value,
                point,
            ))
        })
        .collect::<Vec<_>>();
    let polyline = coordinates
        .iter()
        .map(|(x, y, _, _)| format!("{x:.2},{y:.2}"))
        .collect::<Vec<_>>()
        .join(" ");
    output.push_str(&format!(
        "<polyline class=\"chart-line\" points=\"{polyline}\"></polyline>"
    ));
    for (x, y, value, point) in coordinates {
        output.push_str(&format!(
            "<g class=\"chart-mark chart-mark--point\"><title>{}: {}</title><circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"5\"></circle><text class=\"chart-value\" x=\"{x:.2}\" y=\"{label_y:.2}\" text-anchor=\"middle\">{value}</text><text class=\"chart-category\" x=\"{x:.2}\" y=\"346\" text-anchor=\"middle\">{category}</text></g>",
            typed_visual_escape(&point.label),
            typed_visual_escape(&typed_visual_coordinate_label(&point.y, &visualization.y_axis)),
            label_y = y - 11.0,
            value = typed_visual_escape(&typed_visual_format_number(value)),
            category = typed_visual_escape(&typed_visual_truncate(&point.label, 14)),
        ));
    }
    output.push_str(&render_typed_axis_titles(visualization));
    output
}

fn render_typed_scatter_chart(visualization: &AdmittedTypedVisualization) -> String {
    let x_values = typed_visual_axis_numbers(&visualization.points, true);
    let y_values = typed_visual_axis_numbers(&visualization.points, false);
    let Some((x_minimum, x_maximum)) = typed_visual_domain(&x_values, false) else {
        return String::new();
    };
    let Some((y_minimum, y_maximum)) = typed_visual_domain(&y_values, false) else {
        return String::new();
    };
    let mut output = render_typed_quantitative_grid(
        y_minimum,
        y_maximum,
        &visualization.y_axis,
        false,
    );
    output.push_str(&render_typed_quantitative_grid(
        x_minimum,
        x_maximum,
        &visualization.x_axis,
        true,
    ));
    for point in &visualization.points {
        let (Some(x_value), Some(y_value)) =
            (typed_visual_number(&point.x), typed_visual_number(&point.y))
        else {
            continue;
        };
        let x = typed_visual_scale(x_value, x_minimum, x_maximum, 84.0, 724.0);
        let y = typed_visual_scale(y_value, y_minimum, y_maximum, 318.0, 34.0);
        output.push_str(&format!(
            "<g class=\"chart-mark chart-mark--point\"><title>{}: {}, {}</title><circle cx=\"{x:.2}\" cy=\"{y:.2}\" r=\"6\"></circle><text class=\"chart-point-label\" x=\"{label_x:.2}\" y=\"{label_y:.2}\">{label}</text></g>",
            typed_visual_escape(&point.label),
            typed_visual_escape(&typed_visual_format_number(x_value)),
            typed_visual_escape(&typed_visual_format_number(y_value)),
            label_x = x + 9.0,
            label_y = y - 8.0,
            label = typed_visual_escape(&typed_visual_truncate(&point.label, 18)),
        ));
    }
    output.push_str(&render_typed_axis_titles(visualization));
    output
}

fn render_typed_timeline(visualization: &AdmittedTypedVisualization) -> String {
    let left = 72.0;
    let right = 728.0;
    let center = 198.0;
    let step = (right - left) / (visualization.points.len() - 1) as f64;
    let mut output = format!(
        "<line class=\"chart-axis chart-timeline__line\" x1=\"{left}\" y1=\"{center}\" x2=\"{right}\" y2=\"{center}\"></line>"
    );
    for (index, point) in visualization.points.iter().enumerate() {
        let x = left + index as f64 * step;
        let above = index % 2 == 0;
        let connector_end = if above { 118.0 } else { 278.0 };
        let date_y = if above { 99.0 } else { 318.0 };
        let label_y = if above { 78.0 } else { 341.0 };
        output.push_str(&format!(
            "<g class=\"chart-mark chart-mark--timeline\"><title>{}: {}</title><line x1=\"{x:.2}\" y1=\"{center}\" x2=\"{x:.2}\" y2=\"{connector_end}\"></line><circle cx=\"{x:.2}\" cy=\"{center}\" r=\"6\"></circle><text class=\"chart-value\" x=\"{x:.2}\" y=\"{date_y}\" text-anchor=\"middle\">{date}</text><text class=\"chart-point-label\" x=\"{x:.2}\" y=\"{label_y}\" text-anchor=\"middle\">{label}</text></g>",
            typed_visual_escape(&point.label),
            typed_visual_escape(&typed_visual_coordinate_label(&point.x, &visualization.x_axis)),
            date = typed_visual_escape(&typed_visual_truncate(&typed_visual_coordinate_label(&point.x, &visualization.x_axis), 16)),
            label = typed_visual_escape(&typed_visual_truncate(&point.label, 16)),
        ));
    }
    output.push_str(&format!(
        "<text class=\"chart-axis-title\" x=\"400\" y=\"382\" text-anchor=\"middle\">{}</text>",
        typed_visual_escape(&visualization.x_axis.label)
    ));
    output
}

fn render_typed_kpi_chart(visualization: &AdmittedTypedVisualization) -> String {
    let gap = 14.0;
    let left = 38.0;
    let available = 684.0;
    let width = (available - gap * (visualization.points.len() - 1) as f64)
        / visualization.points.len() as f64;
    let mut output = String::new();
    for (index, point) in visualization.points.iter().enumerate() {
        let x = left + index as f64 * (width + gap);
        let value = typed_visual_coordinate_label(&point.y, &visualization.y_axis);
        output.push_str(&format!(
            "<g class=\"chart-kpi\"><title>{}: {}</title><rect x=\"{x:.2}\" y=\"72\" width=\"{width:.2}\" height=\"232\" rx=\"12\"></rect><text class=\"chart-kpi__value\" x=\"{center:.2}\" y=\"174\" text-anchor=\"middle\">{value}</text><text class=\"chart-kpi__label\" x=\"{center:.2}\" y=\"220\" text-anchor=\"middle\">{label}</text></g>",
            typed_visual_escape(&point.label),
            typed_visual_escape(&value),
            center = x + width / 2.0,
            value = typed_visual_escape(&typed_visual_truncate(&value, 18)),
            label = typed_visual_escape(&typed_visual_truncate(&point.label, 18)),
        ));
    }
    output.push_str(&format!(
        "<text class=\"chart-axis-title\" x=\"400\" y=\"352\" text-anchor=\"middle\">{}</text>",
        typed_visual_escape(&typed_visual_axis_label(&visualization.y_axis))
    ));
    output
}

fn render_typed_quantitative_grid(
    minimum: f64,
    maximum: f64,
    axis: &TypedWireVisualAxis,
    horizontal: bool,
) -> String {
    let mut output = String::new();
    for index in 0..=4 {
        let fraction = index as f64 / 4.0;
        let value = minimum + (maximum - minimum) * fraction;
        if horizontal {
            let x = 84.0 + (724.0 - 84.0) * fraction;
            output.push_str(&format!(
                "<line class=\"chart-grid\" x1=\"{x:.2}\" y1=\"34\" x2=\"{x:.2}\" y2=\"318\"></line><text class=\"chart-tick\" x=\"{x:.2}\" y=\"341\" text-anchor=\"middle\">{}</text>",
                typed_visual_escape(&typed_visual_format_number(value))
            ));
        } else {
            let y = 318.0 - (318.0 - 34.0) * fraction;
            output.push_str(&format!(
                "<line class=\"chart-grid\" x1=\"84\" y1=\"{y:.2}\" x2=\"724\" y2=\"{y:.2}\"></line><text class=\"chart-tick\" x=\"74\" y=\"{label_y:.2}\" text-anchor=\"end\">{}</text>",
                typed_visual_escape(&typed_visual_format_number(value)),
                label_y = y + 4.0,
            ));
        }
    }
    let _ = axis;
    output
}

fn render_typed_axis_titles(visualization: &AdmittedTypedVisualization) -> String {
    format!(
        "<text class=\"chart-axis-title\" x=\"404\" y=\"382\" text-anchor=\"middle\">{}</text><text class=\"chart-axis-title\" x=\"18\" y=\"176\" text-anchor=\"middle\" transform=\"rotate(-90 18 176)\">{}</text>",
        typed_visual_escape(&typed_visual_axis_label(&visualization.x_axis)),
        typed_visual_escape(&typed_visual_axis_label(&visualization.y_axis)),
    )
}

fn typed_visual_axis_numbers(points: &[AdmittedVisualPoint], x_axis: bool) -> Vec<f64> {
    points
        .iter()
        .filter_map(|point| {
            typed_visual_number(if x_axis { &point.x } else { &point.y })
        })
        .collect()
}

fn typed_visual_number(coordinate: &AdmittedVisualCoordinate) -> Option<f64> {
    match coordinate.value {
        AdmittedVisualValue::Number(value) => Some(value),
        AdmittedVisualValue::Text(_) => None,
    }
}

fn typed_visual_domain(values: &[f64], include_zero: bool) -> Option<(f64, f64)> {
    let mut minimum = values.iter().copied().reduce(f64::min)?;
    let mut maximum = values.iter().copied().reduce(f64::max)?;
    if include_zero {
        minimum = minimum.min(0.0);
        maximum = maximum.max(0.0);
    }
    if typed_visual_numbers_equal(minimum, maximum) {
        let padding = minimum.abs().max(1.0) * 0.1;
        minimum -= padding;
        maximum += padding;
    } else if !include_zero {
        let padding = (maximum - minimum) * 0.08;
        minimum -= padding;
        maximum += padding;
    }
    Some((minimum, maximum))
}

fn typed_visual_scale(value: f64, minimum: f64, maximum: f64, start: f64, end: f64) -> f64 {
    start + (value - minimum) / (maximum - minimum) * (end - start)
}

fn typed_visual_axis_label(axis: &TypedWireVisualAxis) -> String {
    axis.unit.as_ref().map_or_else(
        || axis.label.clone(),
        |unit| format!("{} ({})", axis.label, unit.display_label),
    )
}

fn typed_visual_coordinate_label(
    coordinate: &AdmittedVisualCoordinate,
    axis: &TypedWireVisualAxis,
) -> String {
    match &coordinate.value {
        AdmittedVisualValue::Text(value) => value.clone(),
        AdmittedVisualValue::Number(value) => axis.unit.as_ref().map_or_else(
            || typed_visual_format_number(*value),
            |unit| format!("{} {}", typed_visual_format_number(*value), unit.display_label),
        ),
    }
}

fn typed_visual_accessible_description(visualization: &AdmittedTypedVisualization) -> String {
    let points = visualization
        .points
        .iter()
        .map(|point| {
            format!(
                "{}: {}, {}",
                point.label,
                typed_visual_coordinate_label(&point.x, &visualization.x_axis),
                typed_visual_coordinate_label(&point.y, &visualization.y_axis),
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    format!("{}. {points}", visualization.title)
}

fn typed_visual_claim_link(number: usize) -> String {
    format!(
        " <a class=\"citation\" href=\"#claim-{number}\">[{number}]</a>"
    )
}

fn typed_visual_format_number(value: f64) -> String {
    if typed_visual_numbers_equal(value, value.round()) && value.abs() < 1e15 {
        return format!("{:.0}", value);
    }
    let formatted = format!("{value:.4}");
    formatted.trim_end_matches('0').trim_end_matches('.').to_string()
}

fn typed_visual_truncate(value: &str, maximum: usize) -> String {
    if value.chars().count() <= maximum {
        return value.to_string();
    }
    let mut shortened = value.chars().take(maximum.saturating_sub(1)).collect::<String>();
    shortened.push('…');
    shortened
}

fn typed_visual_escape(value: &str) -> String {
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
