fn assessed_report_depth_workflow() -> String {
    use a3s::research::{
        replay, CompletionCriterionAssessment, ContractAssessmentStatus, EvidenceRef,
        InquiryEvent, InquiryLimits, Question, ResearchContractAssessment, ResearchMethod,
        ResearchObligation, ResearchObligationAssessment, StopConditionAssessment,
    };

    let obligations = vec![
        ResearchObligation::new(
            "obligation:mechanism",
            "Mechanism and causes",
            "Establish the documented mechanism and causes",
            true,
            vec!["The mechanism is supported by traceable evidence".to_string()],
        ),
        ResearchObligation::new(
            "obligation:consequences",
            "Counterevidence and consequences",
            "Establish counterevidence and consequences",
            true,
            vec!["Consequences are supported by traceable evidence".to_string()],
        ),
    ];
    let mut mechanism = Question::queued(
        "question:mechanism",
        None,
        "What does the evidence establish about the mechanism?",
    );
    mechanism.obligation_ids = vec!["obligation:mechanism".to_string()];
    let mut consequences = Question::queued(
        "question:consequences",
        None,
        "What does the evidence establish about the consequences?",
    );
    consequences.obligation_ids = vec!["obligation:consequences".to_string()];
    let events = vec![
        InquiryEvent::StrategySelected {
            method: ResearchMethod::Focused,
        },
        InquiryEvent::ResearchObligationsCommitted {
            obligations,
            stop_conditions: vec![
                "Both material obligations are closed by accepted evidence".to_string(),
            ],
        },
        InquiryEvent::QuestionsQueued {
            questions: vec![mechanism, consequences],
        },
        InquiryEvent::EvidenceAccepted {
            evidence: EvidenceRef::new(
                "evidence:mechanism",
                vec!["claim:mechanism".to_string()],
                vec!["source:mechanism".to_string()],
            ),
        },
        InquiryEvent::EvidenceAccepted {
            evidence: EvidenceRef::new(
                "evidence:consequences",
                vec!["claim:consequences".to_string()],
                vec!["source:consequences".to_string()],
            ),
        },
        InquiryEvent::QuestionAnswered {
            question_id: "question:mechanism".to_string(),
            answer: "The accepted evidence establishes the mechanism.".to_string(),
            evidence_ids: vec!["evidence:mechanism".to_string()],
        },
        InquiryEvent::QuestionAnswered {
            question_id: "question:consequences".to_string(),
            answer: "The accepted evidence establishes the consequences.".to_string(),
            evidence_ids: vec!["evidence:consequences".to_string()],
        },
        InquiryEvent::ResearchContractAssessed {
            assessment: ResearchContractAssessment {
                obligations: vec![
                    ResearchObligationAssessment {
                        obligation_id: "obligation:mechanism".to_string(),
                        criteria: vec![CompletionCriterionAssessment {
                            criterion_index: 0,
                            status: ContractAssessmentStatus::Satisfied,
                            rationale: "The mechanism evidence satisfies the criterion."
                                .to_string(),
                            evidence_ids: vec!["evidence:mechanism".to_string()],
                        }],
                        primary_source: None,
                        independent_corroboration: None,
                    },
                    ResearchObligationAssessment {
                        obligation_id: "obligation:consequences".to_string(),
                        criteria: vec![CompletionCriterionAssessment {
                            criterion_index: 0,
                            status: ContractAssessmentStatus::Satisfied,
                            rationale: "The consequences evidence satisfies the criterion."
                                .to_string(),
                            evidence_ids: vec!["evidence:consequences".to_string()],
                        }],
                        primary_source: None,
                        independent_corroboration: None,
                    },
                ],
                stop_conditions: vec![StopConditionAssessment {
                    condition_index: 0,
                    status: ContractAssessmentStatus::Satisfied,
                    rationale: "Both material obligations are evidence-answered.".to_string(),
                    evidence_ids: vec![
                        "evidence:mechanism".to_string(),
                        "evidence:consequences".to_string(),
                    ],
                }],
                diagnostics: Vec::new(),
            },
        },
    ];
    let state =
        replay(&events, &InquiryLimits::default()).expect("valid report-depth inquiry fixture");
    serde_json::json!({
        "mode": "inquiry_collection_wave",
        "execution": {
            "mode": "collect_only",
            "terminal_authority": "host_inquiry_reducer"
        },
        "inquiry": {
            "events": events,
            "state": state
        }
    })
    .to_string()
}

fn benchmark_publication_workflow(qualified: bool) -> String {
    use a3s::research::{
        replay, CompletionCriterionAssessment, ContractAssessmentStatus, EvidenceRef,
        InquiryEvent, InquiryLimits, OutlineSection, Question, ResearchContractAssessment,
        ResearchMethod, ResearchObligation, ResearchObligationAssessment, ResearchOutline,
        StopConditionAssessment,
    };

    let mut workflow = serde_json::json!({
        "plan": {
            "report_title": "Benchmark boundary",
            "tracks": [{
                "id": "obligation:benchmark-boundary",
                "title": "Benchmark boundary",
                "focus": "Establish only the documented benchmark boundary",
                "material": true,
                "completion_criteria": [
                    "The documented range is retained without inventing a product threshold"
                ],
                "evidence_requirements": {
                    "primary_source_required": false,
                    "independent_corroboration_required": false
                }
            }],
            "stop_conditions": [
                "The documented range is established and any decision threshold remains bounded"
            ]
        },
        "research": {
            "status": "success",
            "results": [{
                "success": true,
                "structured": {
                    "summary": "The benchmark has a documented range.",
                    "sources": [{
                        "title": "Published benchmark",
                        "url_or_path": "https://example.com/benchmark",
                        "quote_or_fact": "The benchmark covers 1M-10M vectors.",
                        "reliability": "Published source"
                    }],
                    "key_evidence": ["The benchmark covers 1M-10M vectors."],
                    "contradictions": [],
                    "gaps": [],
                    "confidence": "medium"
                }
            }]
        }
    });
    let accepted = accepted_evidence_ledger(&workflow.to_string(), None)
        .into_iter()
        .next()
        .expect("benchmark evidence");
    let claim_ids = accepted
        .claims
        .iter()
        .map(|claim| claim.id.clone())
        .collect::<Vec<_>>();
    let source_ids = accepted
        .sources
        .iter()
        .map(|source| source.id.clone())
        .collect::<Vec<_>>();
    let evidence_id = accepted.id.clone();
    let obligation_id = "obligation:benchmark-boundary";
    let question_id = "question:benchmark-boundary";
    let section_id = "section:benchmark-boundary";
    let mut question = Question::queued(
        question_id,
        None,
        "What benchmark boundary does the accepted source establish?",
    );
    question.obligation_ids = vec![obligation_id.to_string()];
    let status = if qualified {
        ContractAssessmentStatus::Bounded
    } else {
        ContractAssessmentStatus::Satisfied
    };
    let events = vec![
        InquiryEvent::StrategySelected {
            method: ResearchMethod::Focused,
        },
        InquiryEvent::ResearchObligationsCommitted {
            obligations: vec![ResearchObligation::new(
                obligation_id,
                "Benchmark boundary",
                "Establish only the documented benchmark boundary",
                true,
                vec![
                    "The documented range is retained without inventing a product threshold"
                        .to_string(),
                ],
            )],
            stop_conditions: vec![
                "The documented range is established and any decision threshold remains bounded"
                    .to_string(),
            ],
        },
        InquiryEvent::QuestionsQueued {
            questions: vec![question],
        },
        InquiryEvent::EvidenceAccepted {
            evidence: EvidenceRef::new(
                evidence_id.clone(),
                claim_ids.clone(),
                source_ids.clone(),
            ),
        },
        InquiryEvent::QuestionAnswered {
            question_id: question_id.to_string(),
            answer: "The accepted evidence establishes a 1M-10M benchmark range.".to_string(),
            evidence_ids: vec![evidence_id.clone()],
        },
        InquiryEvent::ResearchContractAssessed {
            assessment: ResearchContractAssessment {
                obligations: vec![ResearchObligationAssessment {
                    obligation_id: obligation_id.to_string(),
                    criteria: vec![CompletionCriterionAssessment {
                        criterion_index: 0,
                        status,
                        rationale: if qualified {
                            "The range is supported, but it does not establish a product decision threshold."
                        } else {
                            "The accepted evidence establishes the requested documented range."
                        }
                        .to_string(),
                        evidence_ids: vec![evidence_id.clone()],
                    }],
                    primary_source: None,
                    independent_corroboration: None,
                }],
                stop_conditions: vec![StopConditionAssessment {
                    condition_index: 0,
                    status,
                    rationale: if qualified {
                        "No accepted evidence establishes a below-range product threshold."
                    } else {
                        "The requested documented range is established."
                    }
                    .to_string(),
                    evidence_ids: vec![evidence_id.clone()],
                }],
                diagnostics: Vec::new(),
            },
        },
        InquiryEvent::OutlineCommitted {
            outline: ResearchOutline {
                sections: vec![OutlineSection {
                    id: section_id.to_string(),
                    heading: "Evidence".to_string(),
                    purpose: "State the supported range and its decision boundary.".to_string(),
                    perspective_ids: Vec::new(),
                    question_ids: vec![question_id.to_string()],
                    claim_ids: claim_ids.clone(),
                    source_ids: source_ids.clone(),
                    composition_hint: "Lead with the supported range.".to_string(),
                }],
            },
        },
        InquiryEvent::SectionDrafted {
            section_id: section_id.to_string(),
            content: "The published benchmark covers 1M-10M vectors.".to_string(),
            citation_ids: claim_ids
                .iter()
                .chain(source_ids.iter())
                .cloned()
                .collect(),
        },
        InquiryEvent::AuditCompleted {
            passed: true,
            issues: Vec::new(),
        },
    ];
    let state = replay(&events, &InquiryLimits::default())
        .expect("valid benchmark publication inquiry");
    workflow["mode"] = serde_json::json!("inquiry_collection_wave");
    workflow["execution"] = serde_json::json!({
        "mode": "collect_only",
        "terminal_authority": "host_inquiry_reducer"
    });
    workflow["inquiry"] = serde_json::json!({
        "events": events,
        "state": state
    });
    workflow.to_string()
}

#[test]
fn generated_report_depth_gate_requires_every_exact_inquiry_obligation_id() {
    let workflow = assessed_report_depth_workflow();
    let coverage = |obligation_id: &str| ReportTrackCoverage {
        obligation_id: obligation_id.to_string(),
        status: ReportTrackStatus::Answered,
        finding: format!("A supported finding for {obligation_id}."),
        interpretation: format!("The evidence explains why {obligation_id} matters."),
        implication: "The finding changes the reader's decision boundary.".to_string(),
        uncertainty: "The conclusion remains bounded by source recency.".to_string(),
    };
    let mut generated = GeneratedDeepResearchReport {
        markdown: "# Report\n\nA substantive source-backed report body with analysis, implications, confidence, and limitations.\n\n## Sources\n\n- https://example.com/source"
            .to_string(),
        editorial: ReportEditorialPlan {
            thesis: "The evidence supports a bounded answer to the investigation.".to_string(),
            track_coverage: vec![coverage("obligation:mechanism")],
        },
        presentation: ReportPresentation {
            rationale: "An analytical composition fits the causal comparison and decision audience."
                .to_string(),
            ..ReportPresentation::default()
        },
    };

    let error = validate_generated_report_depth(&generated, &workflow).unwrap_err();
    assert!(
        error.contains("obligation:consequences"),
        "{error}"
    );

    generated
        .editorial
        .track_coverage
        .push(coverage("obligation:consequences"));
    validate_generated_report_depth(&generated, &workflow).unwrap();

    generated.editorial.track_coverage[1].obligation_id =
        "counterevidence-and-consequences".to_string();
    let error = validate_generated_report_depth(&generated, &workflow).unwrap_err();
    assert!(error.contains("unknown obligation ID"), "{error}");
}

#[test]
fn generated_report_publication_does_not_rewrite_closed_evidence_prose_lexically() {
    let workspace = std::env::temp_dir().join(format!(
        "a3s-deepresearch-quantity-gate-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&workspace).unwrap();
    let query = "Compare the documented benchmark boundary";
    let workflow = benchmark_publication_workflow(false);
    let generated = GeneratedDeepResearchReport {
        markdown: "# Benchmark boundary\n\nUse the product below 1M vectors.\n\n## Evidence\n\nThe published benchmark covers 1M-10M vectors.\n\n## Sources\n\n- https://example.com/benchmark"
            .to_string(),
        editorial: ReportEditorialPlan {
            thesis: "The retained evidence supports a bounded benchmark comparison."
                .to_string(),
            track_coverage: vec![ReportTrackCoverage {
                obligation_id: "obligation:benchmark-boundary".to_string(),
                status: ReportTrackStatus::Bounded,
                finding: "The benchmark publishes a tested range.".to_string(),
                interpretation: "The tested range does not establish a lower threshold."
                    .to_string(),
                implication: "A product decision needs a workload-specific test.".to_string(),
                uncertainty: "No evidence establishes a below-1M cutoff.".to_string(),
            }],
        },
        presentation: ReportPresentation {
            rationale: "A compact analytical briefing fits a bounded benchmark decision."
                .to_string(),
            ..ReportPresentation::default()
        },
    };

    let artifacts = materialize_deep_research_completed_report_from_generation(
        &workspace, query, &generated, &workflow, None,
    )
    .expect("closed-evidence publication should not use language-specific text matching");
    let markdown = std::fs::read_to_string(&artifacts.markdown).unwrap();
    assert!(markdown.contains("below 1M"), "{markdown}");
    assert!(markdown.contains("covers 1M-10M"), "{markdown}");
    assert!(!markdown.contains("## Evidence boundary"), "{markdown}");

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn recovery_report_demotes_an_embedded_report_title() {
    let workspace = std::env::temp_dir().join(format!(
        "a3s-deepresearch-recovery-title-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&workspace).unwrap();
    let answer = "# A source-backed partial report\n\n## Findings\n\nThe collected evidence supports a bounded partial finding with enough explanation to preserve for the reader, but not enough coverage for a completed report.\n\n## Sources\n\n- https://example.com/partial\n\n## Limitations\n\nThe uncollected dimensions remain unknown and require a later retry.";

    let artifacts = materialize_deep_research_recovery_report(
        &workspace,
        "partial report with a title",
        answer,
        "workflow failed",
        None,
    )
    .expect("a useful partial synthesis should converge to a valid recovery artifact");
    let html = std::fs::read_to_string(&artifacts.html).unwrap();
    assert_eq!(html.to_ascii_lowercase().matches("<h1").count(), 1);
    assert!(html.contains("A source-backed partial report"));

    let _ = std::fs::remove_dir_all(workspace);
}

#[test]
fn recovery_report_preserves_safe_validation_feedback_without_classifying_its_words() {
    let result = deep_research_recovery_result_text(
        "report does not cite every source declared by its closed evidence plan",
        r#"{"mode":"direct_web_degraded","research":{"status":"partial_success"}}"#,
    );
    assert!(result.contains("does not cite every source"), "{result}");
}

#[test]
fn recovery_report_preflights_both_targets_before_replacing_either_file() {
    let workspace = std::env::temp_dir().join(format!(
        "a3s-deepresearch-pair-preflight-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let report_dir = workspace.join(".a3s/research/pair-preflight");
    std::fs::create_dir_all(&report_dir).unwrap();
    let old_markdown =
        "# Previous valid report\n\nThis content must survive a rejected HTML target.";
    std::fs::write(report_dir.join("report.md"), old_markdown).unwrap();
    let protected = workspace.join("protected.html");
    std::fs::write(&protected, "protected HTML").unwrap();
    std::fs::hard_link(&protected, report_dir.join("index.html")).unwrap();

    let error = materialize_deep_research_recovery_report(
        &workspace,
        "pair preflight",
        "recovery",
        "workflow failed",
        None,
    )
    .expect_err("an unsafe HTML target must reject the whole artifact pair");

    assert!(
        error.contains("hard-linked DeepResearch artifact"),
        "{error}"
    );
    assert_eq!(
        std::fs::read_to_string(report_dir.join("report.md")).unwrap(),
        old_markdown,
        "preflight failure must preserve the previous Markdown generation"
    );
    assert_eq!(
        std::fs::read_to_string(&protected).unwrap(),
        "protected HTML"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

#[test]
fn report_pair_recovery_rolls_back_an_interrupted_partial_generation() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_dir = workspace.path().join(".a3s/research/interrupted-pair");
    std::fs::create_dir_all(&report_dir).unwrap();
    let markdown_path = report_dir.join("report.md");
    let html_path = report_dir.join("index.html");
    std::fs::write(&markdown_path, b"previous markdown").unwrap();
    std::fs::write(&html_path, b"previous html").unwrap();

    simulate_research_report_pair_interruption_for_test(
        &markdown_path,
        b"replacement markdown",
        &html_path,
        b"replacement html",
        ResearchReportPairInterruption::AfterMarkdownReplacement,
    )
    .expect("simulate an interruption after the first replacement");

    assert_eq!(
        std::fs::read(&markdown_path).unwrap(),
        b"replacement markdown"
    );
    assert_eq!(std::fs::read(&html_path).unwrap(), b"previous html");

    recover_research_report_pair(&markdown_path, &html_path)
        .expect("restart recovery must restore the previous complete generation");

    assert_eq!(std::fs::read(&markdown_path).unwrap(), b"previous markdown");
    assert_eq!(std::fs::read(&html_path).unwrap(), b"previous html");
    assert_eq!(
        std::fs::read_dir(&report_dir).unwrap().count(),
        2,
        "successful recovery must remove the journal and transaction files"
    );
}

#[test]
fn report_pair_recovery_commits_a_fully_replaced_generation() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_dir = workspace.path().join(".a3s/research/committed-pair");
    std::fs::create_dir_all(&report_dir).unwrap();
    let markdown_path = report_dir.join("report.md");
    let html_path = report_dir.join("index.html");
    std::fs::write(&markdown_path, b"previous markdown").unwrap();
    std::fs::write(&html_path, b"previous html").unwrap();

    simulate_research_report_pair_interruption_for_test(
        &markdown_path,
        b"replacement markdown",
        &html_path,
        b"replacement html",
        ResearchReportPairInterruption::AfterHtmlReplacement,
    )
    .expect("simulate an interruption after both replacements");

    recover_research_report_pair(&markdown_path, &html_path)
        .expect("restart recovery must recognize the complete new generation");

    assert_eq!(
        std::fs::read(&markdown_path).unwrap(),
        b"replacement markdown"
    );
    assert_eq!(std::fs::read(&html_path).unwrap(), b"replacement html");
    assert_eq!(
        std::fs::read_dir(&report_dir).unwrap().count(),
        2,
        "successful recovery must remove the journal and transaction files"
    );
}

#[test]
fn report_resolution_recovers_an_interrupted_pair_before_validation() {
    fn report_pair(label: &str) -> (String, String) {
        let body = format!(
            "# {label}\n\n\
             ## Findings\n\n\
             {label} retains a substantive, source-backed finding whose complete generation \
             must remain consistent across both published artifact formats.\n\n\
             ## Sources\n\n\
             1. [Recorded source](https://example.test/recorded-source)\n\n\
             ## Limitations\n\n\
             The deterministic fixture validates publication recovery rather than domain truth.\n"
        );
        let markdown =
            markdown_with_artifact_kind(&body, DeepResearchArtifactKind::Synthesized).unwrap();
        let html = html_with_artifact_kind(
            &deep_research_completed_report_html(label, &body),
            DeepResearchArtifactKind::Synthesized,
        )
        .unwrap();
        (markdown, html)
    }

    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_dir = workspace.path().join(".a3s/research/restart-open");
    std::fs::create_dir_all(&report_dir).unwrap();
    let markdown_path = report_dir.join("report.md");
    let html_path = report_dir.join("index.html");
    let (previous_markdown, previous_html) = report_pair("Previous generation");
    let (replacement_markdown, replacement_html) = report_pair("Replacement generation");
    write_research_report_pair(
        &markdown_path,
        &previous_markdown,
        &html_path,
        &previous_html,
    )
    .unwrap();

    simulate_research_report_pair_interruption_for_test(
        &markdown_path,
        &replacement_markdown,
        &html_path,
        &replacement_html,
        ResearchReportPairInterruption::AfterMarkdownReplacement,
    )
    .expect("simulate an interruption before HTML replacement");

    let artifacts = research_report_artifacts_from_output(
        "A3S_RESEARCH_VIEW: .a3s/research/restart-open/index.html",
        workspace.path(),
    )
    .expect("artifact resolution must recover the previous complete pair");
    let resolved_markdown = std::fs::read_to_string(artifacts.markdown).unwrap();
    let resolved_html = std::fs::read_to_string(artifacts.html).unwrap();
    assert!(
        resolved_markdown.contains("Previous generation"),
        "{resolved_markdown}"
    );
    assert!(resolved_html.contains("Previous generation"), "{resolved_html}");
    assert!(!resolved_markdown.contains("Replacement generation"));
    assert!(!resolved_html.contains("Replacement generation"));
    assert_eq!(
        std::fs::read_dir(&report_dir).unwrap().count(),
        2,
        "artifact resolution must finish transaction cleanup"
    );
}

#[test]
fn report_resolution_removes_an_interrupted_first_generation() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_dir = workspace.path().join(".a3s/research/interrupted-first-pair");
    std::fs::create_dir_all(&report_dir).unwrap();
    let markdown_path = report_dir.join("report.md");
    let html_path = report_dir.join("index.html");

    simulate_research_report_pair_interruption_for_test(
        &markdown_path,
        b"uncommitted markdown",
        &html_path,
        b"uncommitted html",
        ResearchReportPairInterruption::AfterMarkdownReplacement,
    )
    .expect("simulate an interruption during the first publication");

    assert!(
        research_report_artifacts_from_output(
            "A3S_RESEARCH_VIEW: .a3s/research/interrupted-first-pair/index.html",
            workspace.path(),
        )
        .is_none(),
        "a partial first generation must not resolve as a report"
    );
    assert_eq!(
        std::fs::read_dir(&report_dir).unwrap().count(),
        0,
        "restart recovery must remove the incomplete generation and transaction files"
    );
}

#[test]
fn report_pair_recovery_rejects_transaction_path_escape() {
    let workspace = tempfile::tempdir().expect("temporary workspace");
    let report_dir = workspace.path().join(".a3s/research/invalid-transaction");
    std::fs::create_dir_all(&report_dir).unwrap();
    let markdown_path = report_dir.join("report.md");
    let html_path = report_dir.join("index.html");
    std::fs::write(&markdown_path, b"previous markdown").unwrap();
    std::fs::write(&html_path, b"previous html").unwrap();
    let protected = workspace.path().join("protected.md");
    std::fs::write(&protected, b"protected workspace content").unwrap();
    let digest = "0".repeat(64);
    let transaction = serde_json::json!({
        "version": 1,
        "markdown_name": "report.md",
        "html_name": "index.html",
        "staged_markdown_name": "../../protected.md",
        "staged_html_name": ".index.html.1.tmp",
        "previous_markdown_name": null,
        "previous_html_name": null,
        "new_markdown_sha256": digest,
        "new_html_sha256": digest,
        "previous_markdown_sha256": null,
        "previous_html_sha256": null
    });
    std::fs::write(
        report_dir.join(RESEARCH_REPORT_PAIR_TRANSACTION_FILE),
        serde_json::to_vec(&transaction).unwrap(),
    )
    .unwrap();

    let error = recover_research_report_pair(&markdown_path, &html_path)
        .expect_err("transaction paths must remain inside the report directory");

    assert!(
        error.contains("invalid DeepResearch report transaction file name"),
        "{error}"
    );
    assert_eq!(
        std::fs::read(&protected).unwrap(),
        b"protected workspace content"
    );
    assert_eq!(
        std::fs::read(&markdown_path).unwrap(),
        b"previous markdown"
    );
    assert_eq!(std::fs::read(&html_path).unwrap(), b"previous html");
}
