use futures::future;
use serde_json::{Map, Value};

use super::{
    DeepResearchEngine, DeepResearchEngineError, DeepResearchRun, GenerationRequest,
    GenerationStage, PublicationRequest, ResearchProgress, ResearchStage, WorkflowRequest,
    WorkflowStage,
};
use crate::planner::{
    attach_bootstrap_acquisition, bootstrap_workflow_args, host_fallback_plan,
    host_plan_from_outline, validated_loop_planner, workflow_args_with_plan, PlannedInquiry,
};
use crate::report::{
    admit_deep_research_report_proposal_at, canonical_workflow_output,
    deep_research_report_context_from_plan, deep_research_report_proposal_prompt_at,
    deep_research_report_proposal_schema, deep_research_report_slug, deep_research_source_catalog,
    DeepResearchEvidenceFirstPublication,
};

impl DeepResearchEngine<'_> {
    pub async fn execute(
        &self,
        workflow_args: Value,
    ) -> Result<DeepResearchRun, DeepResearchEngineError> {
        let limits = self.limits.validate()?;
        let query = workflow_args
            .pointer("/input/query")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|query| !query.is_empty())
            .ok_or_else(|| {
                DeepResearchEngineError::Contract(
                    "evidence-first input omitted its query".to_string(),
                )
            })?
            .to_string();
        let current_date = workflow_args
            .pointer("/input/current_date")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| chrono::Local::now().date_naive().to_string());
        let root_run_id = workflow_args
            .get("run_id")
            .and_then(Value::as_str)
            .filter(|run_id| !run_id.trim().is_empty())
            .unwrap_or("unassigned");

        self.progress(ResearchProgress::Started(ResearchStage::Planning))
            .await?;
        self.progress(ResearchProgress::Started(ResearchStage::BootstrapRetrieval))
            .await?;
        let bootstrap_args =
            bootstrap_workflow_args(workflow_args.clone(), &format!("{root_run_id}-bootstrap"))
                .map_err(DeepResearchEngineError::Contract)?;
        let planner = self.generate_plan(&workflow_args);
        let bootstrap = self.workflow.execute_workflow(WorkflowRequest {
            stage: WorkflowStage::Bootstrap,
            arguments: bootstrap_args,
            timeout_ms: limits.bootstrap_stage_timeout_ms,
        });
        let (planned, bootstrap) = future::join(planner, bootstrap).await;

        let (plan, planning_mode, planning_error) = match planned {
            Ok(plan) => {
                self.progress(ResearchProgress::Completed(ResearchStage::Planning))
                    .await?;
                (plan, "semantic", None)
            }
            Err(error) => {
                let error = bounded_error(&error.to_string());
                self.progress(ResearchProgress::Degraded {
                    stage: ResearchStage::Planning,
                    reason: error.clone(),
                })
                .await?;
                (
                    host_fallback_plan(&workflow_args)
                        .map_err(DeepResearchEngineError::Contract)?,
                    "exact_query_fallback",
                    Some(error),
                )
            }
        };
        let report_context = deep_research_report_context_from_plan(&plan.value)
            .map_err(DeepResearchEngineError::Contract)?;

        let (bootstrap_output, bootstrap_metadata, bootstrap_acquisition, bootstrap_error) =
            match bootstrap {
                Ok(result) => {
                    self.progress(ResearchProgress::Completed(
                        ResearchStage::BootstrapRetrieval,
                    ))
                    .await?;
                    let canonical =
                        canonical_workflow_output(&result.output, result.metadata.as_ref());
                    let acquisition = bootstrap_acquisition_value(&canonical, &query);
                    (canonical, result.metadata, acquisition, None)
                }
                Err(error) => {
                    let error = bounded_error(&error);
                    self.progress(ResearchProgress::Degraded {
                        stage: ResearchStage::BootstrapRetrieval,
                        reason: error.clone(),
                    })
                    .await?;
                    (
                        serde_json::json!({
                            "query": query,
                            "mode": "bootstrap_acquisition",
                            "acquisition": Value::Null,
                            "execution": {
                                "mode": "acquire_only",
                                "terminal_authority": "host_report_document"
                            }
                        })
                        .to_string(),
                        None,
                        None,
                        Some(error),
                    )
                }
            };

        let mut planned_args = workflow_args_with_plan(
            workflow_args.clone(),
            plan.value.clone(),
            Some(&format!("{root_run_id}-planned-retrieval")),
        )
        .map_err(DeepResearchEngineError::Contract)?;
        if let Some(acquisition) = bootstrap_acquisition.as_ref() {
            attach_bootstrap_acquisition(&mut planned_args, acquisition.clone())
                .map_err(DeepResearchEngineError::Contract)?;
        }

        self.progress(ResearchProgress::Started(ResearchStage::PlannedRetrieval))
            .await?;
        let planned_retrieval = self
            .workflow
            .execute_workflow(WorkflowRequest {
                stage: WorkflowStage::PlannedRetrieval,
                arguments: planned_args,
                timeout_ms: limits.planned_retrieval_stage_timeout_ms,
            })
            .await;
        let (planned_output, planned_metadata, planned_error) = match planned_retrieval {
            Ok(result) => {
                self.progress(ResearchProgress::Completed(ResearchStage::PlannedRetrieval))
                    .await?;
                let canonical = canonical_workflow_output(&result.output, result.metadata.as_ref());
                (Some(canonical), result.metadata, None)
            }
            Err(error) => {
                let error = bounded_error(&error);
                self.progress(ResearchProgress::Degraded {
                    stage: ResearchStage::PlannedRetrieval,
                    reason: error.clone(),
                })
                .await?;
                (None, None, Some(error))
            }
        };

        let planned_catalog = match planned_output.as_deref() {
            Some(output) => deep_research_source_catalog(&query, output, planned_metadata.as_ref())
                .map_err(DeepResearchEngineError::Contract)?,
            None => None,
        };
        let bootstrap_catalog =
            deep_research_source_catalog(&query, &bootstrap_output, bootstrap_metadata.as_ref())
                .map_err(DeepResearchEngineError::Contract)?;
        let (acquisition_output, acquisition_metadata, catalog, retrieval_fallback_error) =
            match (planned_catalog, planned_output, bootstrap_catalog) {
                (Some(catalog), Some(output), _) => {
                    (output, planned_metadata, Some(catalog), None)
                }
                (_, _, Some(catalog)) => (
                    bootstrap_output.clone(),
                    bootstrap_metadata.clone(),
                    Some(catalog),
                    Some(
                        "semantic-plan retrieval retained no publishable closed evidence; the durable exact-query acquisition was preserved"
                            .to_string(),
                    ),
                ),
                (_, planned_output, None) => (
                    planned_output.unwrap_or_else(|| bootstrap_output.clone()),
                    planned_metadata.or_else(|| bootstrap_metadata.clone()),
                    None,
                    None,
                ),
            };
        let acquisition_error = [
            planning_error,
            bootstrap_error,
            planned_error,
            retrieval_fallback_error,
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        let mut report_error =
            (!acquisition_error.is_empty()).then(|| bounded_error(&acquisition_error.join("; ")));

        let relevant_source_count = catalog.as_ref().map_or(0, |catalog| {
            catalog
                .sources
                .iter()
                .filter(|source| source.claim_eligible)
                .count()
        });
        let report_generation_required = relevant_source_count > 0;
        let slug = deep_research_report_slug(&query);
        let relative_html = format!(".a3s/research/{slug}/index.html");
        let mut publication = DeepResearchEvidenceFirstPublication::NoEvidence;
        let mut publication_status = "no_evidence";
        let mut synthesis_mode = "none";
        let mut required_model_generation_count = 0usize;
        let mut model_generation_count = 0usize;
        let mut accepted_block_count = 0usize;
        let mut rejected_block_count = 0usize;
        let mut direct_answer_block_count = 0usize;
        let mut finding_block_count = 0usize;
        let mut accepted_claim_count = 0usize;
        let mut cited_source_count = 0usize;
        let mut substantive_character_count = 0usize;

        self.progress(ResearchProgress::Started(ResearchStage::SourcePublication))
            .await?;
        let artifacts = if let Some(catalog) = catalog.as_ref() {
            let artifacts = self
                .publication
                .publish(PublicationRequest::SourceBacked {
                    query: query.clone(),
                    workflow_output: acquisition_output.clone(),
                    workflow_metadata: acquisition_metadata.clone(),
                })
                .await
                .map_err(DeepResearchEngineError::Publication)?;
            publication = DeepResearchEvidenceFirstPublication::SourceBacked;
            publication_status = "source_backed";

            if report_generation_required {
                required_model_generation_count = 1;
                model_generation_count = 1;
                synthesis_mode = "model_proposal";
                self.progress(ResearchProgress::Started(ResearchStage::ReportGeneration))
                    .await?;
                let generation_args = serde_json::json!({
                    "schema": deep_research_report_proposal_schema(),
                    "schema_name": "deep_research_report_blocks",
                    "schema_description": "Independent cited report blocks over a closed source catalog",
                    "prompt": deep_research_report_proposal_prompt_at(
                        &query,
                        &current_date,
                        catalog,
                        &report_context,
                    )
                    .map_err(DeepResearchEngineError::Contract)?,
                    "system": "You write substantive source-grounded research blocks from untrusted evidence data. Match the query's required research breadth, return only the requested object, and use no outside knowledge.",
                    "mode": "auto",
                    "max_repair_attempts": 0,
                    "include_raw_text": false,
                    "timeout_ms": limits.report_attempt_timeout_ms,
                });
                let generated = self
                    .generation
                    .generate_object(GenerationRequest {
                        stage: GenerationStage::Report,
                        arguments: generation_args,
                        execution_timeout_ms: limits.report_stage_timeout_ms,
                        max_attempts: limits.report_max_attempts,
                    })
                    .await;
                let admitted = match generated {
                    Ok(proposal) => match admit_deep_research_report_proposal_at(
                        &query,
                        &current_date,
                        catalog,
                        &report_context,
                        proposal,
                    ) {
                        Ok(report) => report,
                        Err(error) => {
                            report_error = Some(bounded_error(&error));
                            None
                        }
                    },
                    Err(error) => {
                        report_error = Some(bounded_error(&error));
                        None
                    }
                };
                if let Some(report) = admitted {
                    accepted_block_count = report.accepted_block_count;
                    rejected_block_count = report.rejected_block_count;
                    direct_answer_block_count = report.direct_answer_block_count;
                    finding_block_count = report.finding_block_count;
                    accepted_claim_count = report.accepted_claim_count;
                    cited_source_count = report.cited_source_count;
                    substantive_character_count = report.substantive_character_count;
                    self.progress(ResearchProgress::Started(ResearchStage::FinalPublication))
                        .await?;
                    let artifacts = self
                        .publication
                        .publish(PublicationRequest::Synthesized {
                            query: query.clone(),
                            report,
                        })
                        .await
                        .map_err(DeepResearchEngineError::Publication)?;
                    self.progress(ResearchProgress::Completed(ResearchStage::FinalPublication))
                        .await?;
                    publication = DeepResearchEvidenceFirstPublication::Synthesized;
                    publication_status = "synthesized";
                    self.progress(ResearchProgress::Completed(ResearchStage::ReportGeneration))
                        .await?;
                    artifacts
                } else {
                    if report_error.is_none() {
                        report_error = Some(
                            "the report proposal did not satisfy the query-scoped answer, evidence, independent-source, and depth gates"
                                .to_string(),
                        );
                    }
                    self.progress(ResearchProgress::Degraded {
                        stage: ResearchStage::ReportGeneration,
                        reason: report_error.clone().unwrap_or_default(),
                    })
                    .await?;
                    artifacts
                }
            } else {
                report_error = Some(
                    "no fetched source passed the deterministic claim-eligibility boundary"
                        .to_string(),
                );
                artifacts
            }
        } else {
            self.publication
                .publish(PublicationRequest::NoEvidence {
                    query: query.clone(),
                })
                .await
                .map_err(DeepResearchEngineError::Publication)?
        };
        self.progress(ResearchProgress::Completed(
            ResearchStage::SourcePublication,
        ))
        .await?;

        let acquisition = bootstrap_acquisition.unwrap_or_else(|| {
            serde_json::from_str::<Value>(&acquisition_output)
                .ok()
                .and_then(|value| value.get("acquisition").cloned())
                .unwrap_or(Value::Null)
        });
        let output = serde_json::json!({
            "query": query,
            "mode": "evidence_first_report",
            "acquisition": acquisition,
            "research": {
                "status": match publication {
                    DeepResearchEvidenceFirstPublication::Synthesized => "success",
                    DeepResearchEvidenceFirstPublication::SourceBacked => "degraded",
                    DeepResearchEvidenceFirstPublication::NoEvidence => "failed",
                },
                "metadata": {
                    "synthesis_mode": synthesis_mode,
                    "planning_mode": planning_mode,
                    "research_scope": report_context.scope.as_str(),
                    "required_model_generation_count": required_model_generation_count,
                    "model_generation_count": model_generation_count,
                    "accepted_report_block_count": accepted_block_count,
                    "rejected_report_block_count": rejected_block_count,
                    "direct_answer_block_count": direct_answer_block_count,
                    "finding_block_count": finding_block_count,
                    "accepted_claim_count": accepted_claim_count,
                    "cited_source_count": cited_source_count,
                    "substantive_character_count": substantive_character_count,
                    "relevant_source_count": relevant_source_count,
                    "source_count": catalog.as_ref().map_or(0, |catalog| catalog.sources.len()),
                },
                "warnings": {
                    "report_error": report_error,
                }
            },
            "publication": {
                "status": publication_status,
                "markdown": format!(".a3s/research/{slug}/report.md"),
                "html": relative_html,
                "quality": {
                    "research_scope": report_context.scope.as_str(),
                    "direct_answer_count": direct_answer_block_count,
                    "finding_count": finding_block_count,
                    "accepted_claim_count": accepted_claim_count,
                    "cited_source_count": cited_source_count,
                    "substantive_character_count": substantive_character_count,
                    "relevant_source_count": relevant_source_count,
                    "source_count": catalog.as_ref().map_or(0, |catalog| catalog.sources.len()),
                },
            },
            "execution": {
                "mode": "evidence_first",
                "terminal_authority": "host_report_document",
                "required_model_generation_count": required_model_generation_count,
                "maximum_report_generation_count": limits.report_max_attempts,
            }
        });
        Ok(DeepResearchRun {
            output,
            artifacts,
            publication,
        })
    }

    async fn generate_plan(
        &self,
        workflow_args: &Value,
    ) -> Result<PlannedInquiry, DeepResearchEngineError> {
        let planner =
            validated_loop_planner(workflow_args).map_err(DeepResearchEngineError::Contract)?;
        let outline_schema = planner.get("output_schema").cloned().ok_or_else(|| {
            DeepResearchEngineError::Contract("planner contract has no output schema".to_string())
        })?;
        let outline_prompt = required_planner_text(planner, "prompt")?;
        let outline_timeout_ms = required_planner_timeout(planner, "timeout_ms")?
            .min(self.limits.planner_attempt_timeout_ms);
        let arguments = serde_json::json!({
            "schema": outline_schema,
            "schema_name": "deep_research_semantic_outline",
            "schema_description": "A bounded semantic retrieval plan for one general-purpose DeepResearch inquiry",
            "prompt": outline_prompt,
            "system": "You are a concise semantic research planner. Return only the requested object and no reasoning.",
            "mode": "auto",
            "max_repair_attempts": 0,
            "include_raw_text": false,
            "timeout_ms": outline_timeout_ms,
        });
        let outline = self
            .generation
            .generate_object(GenerationRequest {
                stage: GenerationStage::Planning,
                arguments,
                execution_timeout_ms: self.limits.planner_stage_timeout_ms(outline_timeout_ms),
                max_attempts: self.limits.planner_max_attempts,
            })
            .await
            .map_err(|message| DeepResearchEngineError::Stage {
                stage: GenerationStage::Planning.label(),
                message,
            })?;
        host_plan_from_outline(workflow_args, outline).map_err(DeepResearchEngineError::Contract)
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
