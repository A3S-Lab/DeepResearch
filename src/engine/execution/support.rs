use std::future::Future;

use futures::future::{self, Either};
use serde_json::Map;

fn incomplete_editorial_fallback(
    report: AdmittedDeepResearchReport,
) -> Option<AdmittedDeepResearchReport> {
    (report.publication != DeepResearchEvidenceFirstPublication::Synthesized).then_some(report)
}

fn ensure_not_cancelled(
    cancellation: &DeepResearchCancellation,
) -> Result<(), DeepResearchEngineError> {
    if cancellation.is_cancelled() {
        Err(DeepResearchEngineError::Cancelled)
    } else {
        Ok(())
    }
}

fn typed_result_output(mut output: Value, publication: PublicationOutcome) -> Value {
    if let Some(research) = output.get_mut("research").and_then(Value::as_object_mut) {
        research.insert(
            "status".to_string(),
            Value::String(publication_outcome_id(publication).to_string()),
        );
    }
    if let Some(metadata) = output.get_mut("publication").and_then(Value::as_object_mut) {
        metadata.remove("markdown");
        metadata.remove("html");
        metadata.insert(
            "artifact_kinds".to_string(),
            serde_json::json!(["markdown", "html"]),
        );
    }
    output
}

const fn publication_outcome_id(publication: PublicationOutcome) -> &'static str {
    match publication {
        PublicationOutcome::Synthesized => "synthesized",
        PublicationOutcome::Qualified => "qualified",
        PublicationOutcome::SourceBacked => "source_backed",
        PublicationOutcome::NoEvidence => "no_evidence",
    }
}

async fn await_or_cancel<T>(
    cancellation: &DeepResearchCancellation,
    future: impl Future<Output = T>,
) -> Result<T, DeepResearchEngineError> {
    ensure_not_cancelled(cancellation)?;
    let cancelled = cancellation.cancelled();
    futures::pin_mut!(future);
    futures::pin_mut!(cancelled);
    match future::select(future, cancelled).await {
        Either::Left((value, _)) => {
            ensure_not_cancelled(cancellation)?;
            Ok(value)
        }
        Either::Right(((), _)) => Err(DeepResearchEngineError::Cancelled),
    }
}

fn required_planner_text<'a>(
    planner: &'a Map<String, Value>,
    field: &str,
) -> Result<&'a str, DeepResearchEngineError> {
    planner
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            DeepResearchEngineError::Contract(format!(
                "planner contract has no non-empty `{field}`"
            ))
        })
}

fn required_planner_timeout(
    planner: &Map<String, Value>,
    field: &str,
) -> Result<u64, DeepResearchEngineError> {
    let value = planner.get(field).and_then(Value::as_u64).ok_or_else(|| {
        DeepResearchEngineError::Contract(format!("planner contract omitted integer `{field}`"))
    })?;
    if (1_000..=600_000).contains(&value) {
        Ok(value)
    } else {
        Err(DeepResearchEngineError::Contract(format!(
            "planner contract `{field}` must be between 1000 and 600000"
        )))
    }
}

fn bootstrap_acquisition_value(output: &str, expected_query: &str) -> Option<Value> {
    let value = serde_json::from_str::<Value>(output).ok()?;
    if value.get("query").and_then(Value::as_str) != Some(expected_query)
        || value.get("mode").and_then(Value::as_str) != Some("bootstrap_acquisition")
        || value
            .pointer("/execution/terminal_authority")
            .and_then(Value::as_str)
            != Some("host_inquiry_reducer")
    {
        return None;
    }
    let acquisition = value.get("acquisition")?.clone();
    let sources = acquisition.pointer("/packet/sources")?.as_array()?;
    if sources.is_empty() || sources.len() > 16 {
        return None;
    }
    let valid = sources.iter().all(|source| {
        source
            .get("source_id")
            .and_then(Value::as_str)
            .is_some_and(|id| !id.trim().is_empty())
            && source
                .get("url_or_path")
                .and_then(Value::as_str)
                .is_some_and(|anchor| !anchor.trim().is_empty())
            && source
                .get("chunks")
                .and_then(Value::as_array)
                .is_some_and(|chunks| {
                    !chunks.is_empty()
                        && chunks.iter().all(|chunk| {
                            chunk
                                .get("chunk_id")
                                .and_then(Value::as_str)
                                .is_some_and(|id| !id.trim().is_empty())
                                && chunk
                                    .get("text")
                                    .and_then(Value::as_str)
                                    .is_some_and(|text| !text.trim().is_empty())
                        })
                })
    });
    valid.then_some(acquisition)
}

fn bounded_error(error: &str) -> String {
    error
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(1_000)
        .collect()
}
