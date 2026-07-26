use std::future::Future;

use futures::future::{self, Either};
use serde_json::{Map, Value};

use super::{
    DeepResearchCancellation, DeepResearchEngine, DeepResearchEngineError, DeepResearchEvent,
    DeepResearchLifecycle, DeepResearchRequest, DeepResearchResult, DeepResearchRun,
    GenerationRequest, GenerationStage, PublicationOutcome, PublicationRequest, ResearchProgress,
    ResearchStage, WorkflowRequest, WorkflowStage,
};
use crate::planner::{
    attach_bootstrap_acquisition, bootstrap_workflow_args, host_fallback_plan,
    host_plan_from_outline, validated_loop_planner, workflow_args_with_plan, PlannedInquiry,
};
use crate::report::{
    admit_deep_research_typed_report_draft_in_language_at,
    apply_deep_research_typed_editorial_plan, canonical_workflow_output,
    deep_research_report_context_from_plan, deep_research_report_slug,
    deep_research_source_catalog, deep_research_typed_editorial_prompt,
    deep_research_typed_editorial_schema,
    deep_research_typed_report_proposal_prompt_in_language_at,
    deep_research_typed_report_proposal_schema_for_language, DeepResearchEvidenceFirstPublication,
    DeepResearchPublicationQuality,
};

impl DeepResearchEngine<'_> {
    pub async fn execute(
        &self,
        workflow_args: Value,
    ) -> Result<DeepResearchRun, DeepResearchEngineError> {
        self.execute_internal(workflow_args, &DeepResearchCancellation::new())
            .await
    }

    pub async fn execute_request(
        &self,
        request: DeepResearchRequest,
        cancellation: DeepResearchCancellation,
    ) -> Result<DeepResearchResult, DeepResearchEngineError> {
        request
            .validate()
            .map_err(DeepResearchEngineError::Contract)?;
        let run_id = request.run_id.clone();
        let query = request.query.trim().to_string();
        self.event(DeepResearchEvent::RunStarted {
            run_id: run_id.clone(),
            query: query.clone(),
        })
        .await?;
        if cancellation.is_cancelled() {
            let _ = self
                .event(DeepResearchEvent::RunCancelled {
                    run_id: run_id.clone(),
                })
                .await;
            return Err(DeepResearchEngineError::Cancelled);
        }
        let workflow_args = request
            .to_workflow_arguments()
            .map_err(DeepResearchEngineError::Contract)?;
        match self.execute_internal(workflow_args, &cancellation).await {
            Ok(run) => {
                let publication = PublicationOutcome::from(run.publication);
                let result = DeepResearchResult {
                    run_id: run_id.clone(),
                    query,
                    lifecycle: DeepResearchLifecycle::Completed,
                    publication,
                    quality: run.quality,
                    artifacts: run.artifacts,
                    output: typed_result_output(run.output, publication),
                };
                self.event(DeepResearchEvent::PublicationCompleted {
                    run_id: run_id.clone(),
                    outcome: result.publication,
                    quality: result.quality,
                    artifacts: result.artifacts.clone(),
                })
                .await?;
                self.event(DeepResearchEvent::RunCompleted {
                    run_id,
                    outcome: result.publication,
                })
                .await?;
                Ok(result)
            }
            Err(DeepResearchEngineError::Cancelled) => {
                let _ = self.event(DeepResearchEvent::RunCancelled { run_id }).await;
                Err(DeepResearchEngineError::Cancelled)
            }
            Err(error) => {
                let _ = self
                    .event(DeepResearchEvent::RunFailed {
                        run_id,
                        message: bounded_error(&error.to_string()),
                    })
                    .await;
                Err(error)
            }
        }
    }

    async fn execute_internal(
        &self,
        workflow_args: Value,
        cancellation: &DeepResearchCancellation,
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
        let output_language = workflow_args
            .pointer("/input/output_language")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|language| !language.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| crate::language::infer_deep_research_output_language(&query));
        crate::language::validate_deep_research_output_language(&output_language)
            .map_err(DeepResearchEngineError::Contract)?;
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

        ensure_not_cancelled(cancellation)?;
        self.progress(
            root_run_id,
            ResearchProgress::Started(ResearchStage::Planning),
        )
        .await?;
        self.progress(
            root_run_id,
            ResearchProgress::Started(ResearchStage::BootstrapRetrieval),
        )
        .await?;
        let bootstrap_args =
            bootstrap_workflow_args(workflow_args.clone(), &format!("{root_run_id}-bootstrap"))
                .map_err(DeepResearchEngineError::Contract)?;
        let planner = self.generate_plan(&workflow_args, cancellation);
        let bootstrap = self.workflow.execute_workflow(WorkflowRequest {
            stage: WorkflowStage::Bootstrap,
            arguments: bootstrap_args,
            timeout_ms: limits.bootstrap_stage_timeout_ms,
        });
        let (planned, bootstrap) =
            await_or_cancel(cancellation, future::join(planner, bootstrap)).await?;

        let (plan, planning_mode, planning_error) = match planned {
            Ok(plan) => {
                self.progress(
                    root_run_id,
                    ResearchProgress::Completed(ResearchStage::Planning),
                )
                .await?;
                (plan, "semantic", None)
            }
            Err(error) => {
                let error = bounded_error(&error.to_string());
                self.progress(
                    root_run_id,
                    ResearchProgress::Degraded {
                        stage: ResearchStage::Planning,
                        reason: error.clone(),
                    },
                )
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

        let (
            bootstrap_output,
            bootstrap_metadata,
            bootstrap_acquisition,
            bootstrap_catalog,
            bootstrap_error,
        ) = match bootstrap {
            Ok(result) => {
                let metadata = result.metadata;
                let canonical = canonical_workflow_output(&result.output, metadata.as_ref());
                match deep_research_source_catalog(&query, &canonical, metadata.as_ref()) {
                    Ok(catalog) => {
                        self.progress(
                            root_run_id,
                            ResearchProgress::Completed(ResearchStage::BootstrapRetrieval),
                        )
                        .await?;
                        let acquisition = catalog
                            .as_ref()
                            .and_then(|_| bootstrap_acquisition_value(&canonical, &query));
                        (canonical, metadata, acquisition, catalog, None)
                    }
                    Err(error) => {
                        let error = bounded_error(&error);
                        self.progress(
                            root_run_id,
                            ResearchProgress::Degraded {
                                stage: ResearchStage::BootstrapRetrieval,
                                reason: error.clone(),
                            },
                        )
                        .await?;
                        (canonical, metadata, None, None, Some(error))
                    }
                }
            }
            Err(error) => {
                let error = bounded_error(&error);
                self.progress(
                    root_run_id,
                    ResearchProgress::Degraded {
                        stage: ResearchStage::BootstrapRetrieval,
                        reason: error.clone(),
                    },
                )
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

        ensure_not_cancelled(cancellation)?;
        self.progress(
            root_run_id,
            ResearchProgress::Started(ResearchStage::PlannedRetrieval),
        )
        .await?;
        let planned_retrieval = await_or_cancel(
            cancellation,
            self.workflow.execute_workflow(WorkflowRequest {
                stage: WorkflowStage::PlannedRetrieval,
                arguments: planned_args,
                timeout_ms: limits.planned_retrieval_stage_timeout_ms,
            }),
        )
        .await?;
        let (planned_output, planned_metadata, planned_catalog, planned_error) =
            match planned_retrieval {
                Ok(result) => {
                    let metadata = result.metadata;
                    let canonical = canonical_workflow_output(&result.output, metadata.as_ref());
                    match deep_research_source_catalog(&query, &canonical, metadata.as_ref()) {
                        Ok(catalog) => {
                            self.progress(
                                root_run_id,
                                ResearchProgress::Completed(ResearchStage::PlannedRetrieval),
                            )
                            .await?;
                            (Some(canonical), metadata, catalog, None)
                        }
                        Err(error) => {
                            let error = bounded_error(&error);
                            self.progress(
                                root_run_id,
                                ResearchProgress::Degraded {
                                    stage: ResearchStage::PlannedRetrieval,
                                    reason: error.clone(),
                                },
                            )
                            .await?;
                            (Some(canonical), metadata, None, Some(error))
                        }
                    }
                }
                Err(error) => {
                    let error = bounded_error(&error);
                    self.progress(
                        root_run_id,
                        ResearchProgress::Degraded {
                            stage: ResearchStage::PlannedRetrieval,
                            reason: error.clone(),
                        },
                    )
                    .await?;
                    (None, None, None, Some(error))
                }
            };
        let planned_catalog = planned_catalog
            .filter(|catalog| catalog.sources.iter().any(|source| source.claim_eligible));
        let bootstrap_catalog = bootstrap_catalog
            .filter(|catalog| catalog.sources.iter().any(|source| source.claim_eligible));

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
        let mut editorial_error = None;
        let mut accepted_block_count = 0usize;
        let mut rejected_block_count = 0usize;
        let mut direct_answer_block_count = 0usize;
        let mut finding_block_count = 0usize;
        let mut accepted_claim_count = 0usize;
        let mut accepted_relation_count = 0usize;
        let mut accepted_derivation_count = 0usize;
        let mut accepted_basis_edge_count = 0usize;
        let mut analytical_claim_count = 0usize;
        let mut cross_source_synthesis_count = 0usize;
        let mut resolved_material_dimension_count = 0usize;
        let mut deeply_analyzed_dimension_count = 0usize;
        let mut accepted_gap_count = 0usize;
        let mut cited_source_count = 0usize;
        let mut substantive_character_count = 0usize;

        ensure_not_cancelled(cancellation)?;
        self.progress(
            root_run_id,
            ResearchProgress::Started(ResearchStage::SourcePublication),
        )
        .await?;
        let artifacts = if let Some(catalog) = catalog.as_ref() {
            let source_backed_quality = DeepResearchPublicationQuality {
                research_scope: report_context.scope,
                direct_answer_count: 0,
                finding_count: 0,
                accepted_claim_count: 0,
                accepted_relation_count: 0,
                accepted_derivation_count: 0,
                accepted_basis_edge_count: 0,
                analytical_claim_count: 0,
                cross_source_synthesis_count: 0,
                resolved_material_dimension_count: 0,
                deeply_analyzed_dimension_count: 0,
                accepted_gap_count: 0,
                cited_source_count: 0,
                substantive_character_count: 0,
                relevant_source_count,
                source_count: catalog.sources.len(),
            };
            let artifacts = await_or_cancel(
                cancellation,
                self.publication.publish(PublicationRequest::SourceBacked {
                    run_id: root_run_id.to_string(),
                    query: query.clone(),
                    output_language: output_language.clone(),
                    workflow_output: acquisition_output.clone(),
                    workflow_metadata: acquisition_metadata.clone(),
                    quality: source_backed_quality,
                }),
            )
            .await?
            .map_err(DeepResearchEngineError::Publication)?;
            publication = DeepResearchEvidenceFirstPublication::SourceBacked;
            publication_status = "source_backed";

            if report_generation_required {
                required_model_generation_count = 2;
                model_generation_count = 1;
                synthesis_mode = "model_claim_graph";
                ensure_not_cancelled(cancellation)?;
                self.progress(
                    root_run_id,
                    ResearchProgress::Started(ResearchStage::ReportGeneration),
                )
                .await?;
                let report_schema = deep_research_typed_report_proposal_schema_for_language(
                    catalog,
                    &report_context,
                    &output_language,
                )
                .map_err(DeepResearchEngineError::Contract)?;
                let generation_args = serde_json::json!({
                    "schema": report_schema,
                    "schema_name": "deep_research_typed_claim_graph",
                    "schema_description": "Typed conclusions, atomic evidence, explicit comparison, explanation, implication, challenge and boundary roles, contradiction relations, and bounded gaps over a closed source catalog",
                    "prompt": deep_research_typed_report_proposal_prompt_in_language_at(
                        &query,
                        &current_date,
                        &output_language,
                        catalog,
                        &report_context,
                    )
                    .map_err(DeepResearchEngineError::Contract)?,
                    "system": "You construct an auditable, multi-step, source-grounded research argument from untrusted evidence data. Every resolved material dimension must move from conclusion to atomic evidence, cross-source comparison, mechanism or trade-off explanation, implication, and an adversarial challenge or applicability boundary. Each step has an explicit analysis role and must make distinct intellectual progress. Return only the requested object and use no outside knowledge.",
                    "mode": "auto",
                    // The durable generation port already owns the bounded
                    // attempt policy. A nested repair loop would multiply the
                    // declared report-call ceiling.
                    "max_repair_attempts": 0,
                    "include_raw_text": false,
                    "timeout_ms": limits.report_attempt_timeout_ms,
                });
                let generated = await_or_cancel(
                    cancellation,
                    self.generation.generate_object(GenerationRequest {
                        stage: GenerationStage::Report,
                        arguments: generation_args,
                        execution_timeout_ms: limits.report_stage_timeout_ms,
                        max_attempts: limits.report_max_attempts,
                    }),
                )
                .await?;
                let admitted = match generated {
                    Ok(proposal) => match admit_deep_research_typed_report_draft_in_language_at(
                        &query,
                        &current_date,
                        &output_language,
                        catalog,
                        &report_context,
                        proposal,
                    ) {
                        Ok(Some(draft)) => {
                            let fallback_report = draft.report.clone();
                            let editorial_prompt =
                                match deep_research_typed_editorial_prompt(&draft) {
                                    Ok(prompt) => prompt,
                                    Err(error) => {
                                        editorial_error = Some(bounded_error(&error));
                                        synthesis_mode = "model_claim_graph_editorial_fallback";
                                        String::new()
                                    }
                                };
                            if editorial_prompt.is_empty() {
                                Some(fallback_report)
                            } else {
                                model_generation_count += 1;
                                let editorial_args = serde_json::json!({
                                    "schema": deep_research_typed_editorial_schema(&draft),
                                    "schema_name": "deep_research_typed_editorial_plan",
                                    "schema_description": "Evidence-preserving section headings, paragraph grouping, purposes, and topologically valid reading order over already admitted claims",
                                    "prompt": editorial_prompt,
                                    "system": "You edit the reading flow of an admitted research argument. You may select headings, paragraph groupings, purposes, and a premise-preserving claim order only. Never add, remove, rewrite, summarize, or merge a claim. Return only the requested object.",
                                    "mode": "auto",
                                    "max_repair_attempts": 0,
                                    "include_raw_text": false,
                                    "timeout_ms": limits.report_attempt_timeout_ms,
                                });
                                let editorial = await_or_cancel(
                                    cancellation,
                                    self.generation.generate_object(GenerationRequest {
                                        stage: GenerationStage::Editorial,
                                        arguments: editorial_args,
                                        execution_timeout_ms: limits.report_stage_timeout_ms,
                                        max_attempts: limits.report_max_attempts,
                                    }),
                                )
                                .await?;
                                match editorial {
                                    Ok(editorial) => {
                                        match apply_deep_research_typed_editorial_plan(
                                            &query,
                                            &current_date,
                                            &output_language,
                                            catalog,
                                            &report_context,
                                            draft,
                                            editorial,
                                        ) {
                                            Ok(report) => {
                                                synthesis_mode = "model_claim_graph_editorial";
                                                Some(report)
                                            }
                                            Err(error) => {
                                                editorial_error = Some(bounded_error(&error));
                                                synthesis_mode =
                                                    "model_claim_graph_editorial_fallback";
                                                Some(fallback_report)
                                            }
                                        }
                                    }
                                    Err(error) => {
                                        editorial_error = Some(bounded_error(&error));
                                        synthesis_mode = "model_claim_graph_editorial_fallback";
                                        Some(fallback_report)
                                    }
                                }
                            }
                        }
                        Ok(None) => None,
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
                    let report_publication = report.publication;
                    let report_accepted_block_count = report.accepted_block_count;
                    let report_rejected_block_count = report.rejected_block_count;
                    let report_direct_answer_block_count = report.direct_answer_block_count;
                    let report_finding_block_count = report.finding_block_count;
                    let report_accepted_claim_count = report.accepted_claim_count;
                    let report_accepted_relation_count = report.accepted_relation_count;
                    let report_accepted_derivation_count = report.accepted_derivation_count;
                    let report_accepted_basis_edge_count = report.accepted_basis_edge_count;
                    let report_analytical_claim_count = report.analytical_claim_count;
                    let report_cross_source_synthesis_count = report.cross_source_synthesis_count;
                    let report_resolved_material_dimension_count =
                        report.resolved_material_dimension_count;
                    let report_deeply_analyzed_dimension_count =
                        report.deeply_analyzed_dimension_count;
                    let report_accepted_gap_count = report.accepted_gap_count;
                    let report_cited_source_count = report.cited_source_count;
                    let report_substantive_character_count = report.substantive_character_count;
                    self.progress(
                        root_run_id,
                        ResearchProgress::Completed(ResearchStage::ReportGeneration),
                    )
                    .await?;
                    ensure_not_cancelled(cancellation)?;
                    self.progress(
                        root_run_id,
                        ResearchProgress::Started(ResearchStage::FinalPublication),
                    )
                    .await?;
                    let final_publication = await_or_cancel(
                        cancellation,
                        self.publication.publish(PublicationRequest::Synthesized {
                            run_id: root_run_id.to_string(),
                            query: query.clone(),
                            output_language: output_language.clone(),
                            report,
                            publication: report_publication,
                            quality: DeepResearchPublicationQuality {
                                research_scope: report_context.scope,
                                direct_answer_count: report_direct_answer_block_count,
                                finding_count: report_finding_block_count,
                                accepted_claim_count: report_accepted_claim_count,
                                accepted_relation_count: report_accepted_relation_count,
                                accepted_derivation_count: report_accepted_derivation_count,
                                accepted_basis_edge_count: report_accepted_basis_edge_count,
                                analytical_claim_count: report_analytical_claim_count,
                                cross_source_synthesis_count: report_cross_source_synthesis_count,
                                resolved_material_dimension_count:
                                    report_resolved_material_dimension_count,
                                deeply_analyzed_dimension_count:
                                    report_deeply_analyzed_dimension_count,
                                accepted_gap_count: report_accepted_gap_count,
                                cited_source_count: report_cited_source_count,
                                substantive_character_count: report_substantive_character_count,
                                relevant_source_count,
                                source_count: catalog.sources.len(),
                            },
                        }),
                    )
                    .await?;
                    match final_publication {
                        Ok(final_artifacts) => {
                            accepted_block_count = report_accepted_block_count;
                            rejected_block_count = report_rejected_block_count;
                            direct_answer_block_count = report_direct_answer_block_count;
                            finding_block_count = report_finding_block_count;
                            accepted_claim_count = report_accepted_claim_count;
                            accepted_relation_count = report_accepted_relation_count;
                            accepted_derivation_count = report_accepted_derivation_count;
                            accepted_basis_edge_count = report_accepted_basis_edge_count;
                            analytical_claim_count = report_analytical_claim_count;
                            cross_source_synthesis_count = report_cross_source_synthesis_count;
                            resolved_material_dimension_count =
                                report_resolved_material_dimension_count;
                            deeply_analyzed_dimension_count =
                                report_deeply_analyzed_dimension_count;
                            accepted_gap_count = report_accepted_gap_count;
                            cited_source_count = report_cited_source_count;
                            substantive_character_count = report_substantive_character_count;
                            self.progress(
                                root_run_id,
                                ResearchProgress::Completed(ResearchStage::FinalPublication),
                            )
                            .await?;
                            publication = report_publication;
                            publication_status = match report_publication {
                                DeepResearchEvidenceFirstPublication::Synthesized => "synthesized",
                                DeepResearchEvidenceFirstPublication::Qualified => "qualified",
                                DeepResearchEvidenceFirstPublication::SourceBacked => {
                                    "source_backed"
                                }
                                DeepResearchEvidenceFirstPublication::NoEvidence => "no_evidence",
                            };
                            final_artifacts
                        }
                        Err(error) => {
                            let final_error = bounded_error(&error);
                            report_error = Some(final_error.clone());
                            self.progress(
                                root_run_id,
                                ResearchProgress::Degraded {
                                    stage: ResearchStage::FinalPublication,
                                    reason: final_error.clone(),
                                },
                            )
                            .await?;

                            // The synthesized publisher can fail after touching
                            // the report pair but before its receipt becomes
                            // durable. Re-publish the closed source snapshot so
                            // the returned artifact kind and receipt agree.
                            await_or_cancel(
                                cancellation,
                                self.publication.publish(PublicationRequest::SourceBacked {
                                    run_id: root_run_id.to_string(),
                                    query: query.clone(),
                                    output_language: output_language.clone(),
                                    workflow_output: acquisition_output.clone(),
                                    workflow_metadata: acquisition_metadata.clone(),
                                    quality: source_backed_quality,
                                }),
                            )
                                .await?
                                .map_err(|recovery_error| {
                                    DeepResearchEngineError::Publication(format!(
                                        "final publication failed: {final_error}; source-backed recovery failed: {}",
                                        bounded_error(&recovery_error)
                                    ))
                                })?
                        }
                    }
                } else {
                    if report_error.is_none() {
                        report_error = Some(
                            "the report proposal did not satisfy the query-scoped answer, evidence, independent-source, and depth gates"
                                .to_string(),
                        );
                    }
                    self.progress(
                        root_run_id,
                        ResearchProgress::Degraded {
                            stage: ResearchStage::ReportGeneration,
                            reason: report_error.clone().unwrap_or_default(),
                        },
                    )
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
            await_or_cancel(
                cancellation,
                self.publication.publish(PublicationRequest::NoEvidence {
                    run_id: root_run_id.to_string(),
                    query: query.clone(),
                    output_language: output_language.clone(),
                    quality: DeepResearchPublicationQuality {
                        research_scope: report_context.scope,
                        direct_answer_count: 0,
                        finding_count: 0,
                        accepted_claim_count: 0,
                        accepted_relation_count: 0,
                        accepted_derivation_count: 0,
                        accepted_basis_edge_count: 0,
                        analytical_claim_count: 0,
                        cross_source_synthesis_count: 0,
                        resolved_material_dimension_count: 0,
                        deeply_analyzed_dimension_count: 0,
                        accepted_gap_count: 0,
                        cited_source_count: 0,
                        substantive_character_count: 0,
                        relevant_source_count: 0,
                        source_count: 0,
                    },
                }),
            )
            .await?
            .map_err(DeepResearchEngineError::Publication)?
        };
        ensure_not_cancelled(cancellation)?;
        self.progress(
            root_run_id,
            ResearchProgress::Completed(ResearchStage::SourcePublication),
        )
        .await?;

        let acquisition = bootstrap_acquisition.unwrap_or_else(|| {
            serde_json::from_str::<Value>(&acquisition_output)
                .ok()
                .and_then(|value| value.get("acquisition").cloned())
                .unwrap_or(Value::Null)
        });
        let quality = DeepResearchPublicationQuality {
            research_scope: report_context.scope,
            direct_answer_count: direct_answer_block_count,
            finding_count: finding_block_count,
            accepted_claim_count,
            accepted_relation_count,
            accepted_derivation_count,
            accepted_basis_edge_count,
            analytical_claim_count,
            cross_source_synthesis_count,
            resolved_material_dimension_count,
            deeply_analyzed_dimension_count,
            accepted_gap_count,
            cited_source_count,
            substantive_character_count,
            relevant_source_count,
            source_count: catalog.as_ref().map_or(0, |catalog| catalog.sources.len()),
        };
        let output = serde_json::json!({
            "query": query,
            "output_language": output_language,
            "mode": "evidence_first_report",
            "acquisition": acquisition,
            "research": {
                "status": match publication {
                    DeepResearchEvidenceFirstPublication::Synthesized => "success",
                    DeepResearchEvidenceFirstPublication::Qualified => "partial_success",
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
                    "accepted_relation_count": accepted_relation_count,
                    "accepted_derivation_count": accepted_derivation_count,
                    "accepted_basis_edge_count": accepted_basis_edge_count,
                    "analytical_claim_count": analytical_claim_count,
                    "cross_source_synthesis_count": cross_source_synthesis_count,
                    "resolved_material_dimension_count": resolved_material_dimension_count,
                    "deeply_analyzed_dimension_count": deeply_analyzed_dimension_count,
                    "accepted_gap_count": accepted_gap_count,
                    "cited_source_count": cited_source_count,
                    "substantive_character_count": substantive_character_count,
                    "relevant_source_count": relevant_source_count,
                    "source_count": catalog.as_ref().map_or(0, |catalog| catalog.sources.len()),
                },
                "warnings": {
                    "report_error": report_error,
                    "editorial_error": editorial_error,
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
                    "accepted_relation_count": accepted_relation_count,
                    "accepted_derivation_count": accepted_derivation_count,
                    "accepted_basis_edge_count": accepted_basis_edge_count,
                    "analytical_claim_count": analytical_claim_count,
                    "cross_source_synthesis_count": cross_source_synthesis_count,
                    "resolved_material_dimension_count": resolved_material_dimension_count,
                    "deeply_analyzed_dimension_count": deeply_analyzed_dimension_count,
                    "accepted_gap_count": accepted_gap_count,
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
                "maximum_report_generation_count": usize::from(limits.report_max_attempts)
                    * required_model_generation_count,
            }
        });
        Ok(DeepResearchRun {
            output,
            artifacts,
            publication,
            quality,
        })
    }

    async fn generate_plan(
        &self,
        workflow_args: &Value,
        cancellation: &DeepResearchCancellation,
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
            "max_repair_attempts": 1,
            "include_raw_text": false,
            "timeout_ms": outline_timeout_ms,
        });
        let outline = await_or_cancel(
            cancellation,
            self.generation.generate_object(GenerationRequest {
                stage: GenerationStage::Planning,
                arguments,
                execution_timeout_ms: self.limits.planner_stage_timeout_ms(outline_timeout_ms),
                max_attempts: self.limits.planner_max_attempts,
            }),
        )
        .await?
        .map_err(|message| DeepResearchEngineError::Stage {
            stage: GenerationStage::Planning.label(),
            message,
        })?;
        host_plan_from_outline(workflow_args, outline).map_err(DeepResearchEngineError::Contract)
    }
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
