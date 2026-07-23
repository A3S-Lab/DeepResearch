#[test]
fn schema_requires_irrelevant_diagnostic_links_to_be_empty() {
    let state = assessed_state();
    let schema = research_contract_assessment_json_schema(&state).expect("schema");
    let dispositions = schema["properties"]["diagnostics"]["properties"]["diagnostic:gap"]
        ["oneOf"]
        .as_array()
        .expect("disposition variants");
    let irrelevant = dispositions
        .iter()
        .find(|variant| variant["properties"]["disposition"]["enum"][0] == "irrelevant")
        .expect("irrelevant disposition schema");
    assert_eq!(irrelevant["properties"]["obligation_ids"]["maxItems"], 0);
    assert_eq!(irrelevant["properties"]["evidence_ids"]["maxItems"], 0);
}

#[test]
fn diagnostic_schema_is_closed_over_its_parent_obligation_path() {
    let state = assessed_state();
    let schema = research_contract_assessment_json_schema(&state).expect("schema");
    let dispositions = schema["properties"]["diagnostics"]["properties"]["diagnostic:gap"]
        ["oneOf"]
        .as_array()
        .expect("disposition variants");
    let resolved = dispositions
        .iter()
        .find(|variant| variant["properties"]["disposition"]["enum"][0] == "resolved")
        .expect("resolved disposition schema");
    assert_eq!(
        resolved["properties"]["obligation_ids"]["items"]["enum"],
        serde_json::json!(["obligation:core"])
    );
    assert_eq!(resolved["properties"]["obligation_ids"]["minItems"], 1);
    assert_eq!(resolved["properties"]["obligation_ids"]["maxItems"], 1);
    assert_eq!(
        resolved["properties"]["evidence_ids"]["items"]["enum"],
        serde_json::json!(["evidence:resolution"]),
        "the parent and unrelated evidence must not be offered as resolvers"
    );

    let bounded = dispositions
        .iter()
        .find(|variant| variant["properties"]["disposition"]["enum"][0] == "bounded")
        .expect("bounded disposition schema");
    assert_eq!(
        bounded["properties"]["evidence_ids"]["items"]["enum"],
        serde_json::json!(["evidence:core"])
    );
    assert_eq!(bounded["properties"]["evidence_ids"]["minItems"], 1);
    assert_eq!(bounded["properties"]["evidence_ids"]["maxItems"], 1);
}

#[test]
fn diagnostic_schema_omits_resolved_without_a_distinct_traceable_resolver() {
    let mut state = assessed_state();
    state.questions[0].evidence_ids = vec!["evidence:core".to_string()];
    let schema = research_contract_assessment_json_schema(&state).expect("schema");
    let dispositions = schema["properties"]["diagnostics"]["properties"]["diagnostic:gap"]
        ["oneOf"]
        .as_array()
        .expect("disposition variants");
    assert!(dispositions
        .iter()
        .all(|variant| { variant["properties"]["disposition"]["enum"][0] != "resolved" }));
}

#[test]
fn bounded_material_diagnostic_prevents_false_convergence() {
    let mut state = assessed_state();
    let value = assessment(DiagnosticDisposition::Bounded, &["evidence:core"]);
    validate_research_contract_assessment(&state, &value).expect("valid assessment");
    state.contract_assessment = Some(value);
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Qualified)
    );
}

#[test]
fn malformed_irrelevant_links_are_conservatively_bounded_by_the_host() {
    let mut state = assessed_state();
    let value = assessment(DiagnosticDisposition::Irrelevant, &["evidence:core"]);
    validate_research_contract_assessment(&state, &value)
        .expect_err("the strict validator must reject contradictory irrelevant links");

    let event = research_contract_assessment_event(&state, value)
        .expect("the event boundary should repair the known model shape");
    let InquiryEvent::ResearchContractAssessed { assessment } = &event else {
        panic!("expected a research contract assessment event");
    };
    let diagnostic = &assessment.diagnostics[0];
    assert_eq!(diagnostic.disposition, DiagnosticDisposition::Bounded);
    assert_eq!(diagnostic.obligation_ids, ["obligation:core"]);
    assert_eq!(diagnostic.evidence_ids, ["evidence:core"]);

    state
        .apply(&event, &InquiryLimits::default())
        .expect("event");
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Qualified)
    );
}

#[test]
fn parent_evidence_cannot_resolve_its_own_diagnostic() {
    let state = assessed_state();
    let value = assessment(DiagnosticDisposition::Resolved, &["evidence:core"]);
    let error = validate_research_contract_assessment(&state, &value)
        .expect_err("parent evidence must not resolve its own diagnostic");
    assert!(error.message().contains("different traceable evidence"));
}

#[test]
fn unrelated_evidence_cannot_resolve_a_diagnostic() {
    let state = assessed_state();
    let value = assessment(DiagnosticDisposition::Resolved, &["evidence:unrelated"]);
    let error = validate_research_contract_assessment(&state, &value)
        .expect_err("unrelated evidence must not resolve a diagnostic");
    assert!(error.message().contains("linked obligation path"));
}

#[test]
fn distinct_traceable_evidence_allows_resolved_diagnostic() {
    let mut state = assessed_state();
    let value = assessment(DiagnosticDisposition::Resolved, &["evidence:resolution"]);
    validate_research_contract_assessment(&state, &value).expect("valid assessment");
    state.contract_assessment = Some(value);
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Satisfied)
    );
}

#[test]
fn host_derivation_preserves_source_quality_as_bounded_without_typed_roles() {
    let mut state = quality_state();
    for evidence in state.evidence_catalog.values_mut() {
        evidence.source_coverage.clear();
    }
    let assessment = derive_research_contract_assessment(&state).expect("derived assessment");
    let obligation = &assessment.obligations[0];
    assert_eq!(
        obligation.criteria[0].status,
        ContractAssessmentStatus::Satisfied
    );
    assert_eq!(
        obligation.primary_source.as_ref().unwrap().status,
        ContractAssessmentStatus::Bounded
    );
    assert_eq!(
        obligation
            .independent_corroboration
            .as_ref()
            .unwrap()
            .status,
        ContractAssessmentStatus::Bounded
    );
    assert!(obligation
        .primary_source
        .as_ref()
        .unwrap()
        .rationale
        .contains("will not infer"));

    let event =
        research_contract_assessment_event(&state, assessment).expect("assessment event");
    state
        .apply(&event, &InquiryLimits::default())
        .expect("apply assessment");
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Qualified)
    );
}

#[test]
fn host_derivation_satisfies_quality_from_typed_source_coverage() {
    let mut state = quality_state();
    let assessment = derive_research_contract_assessment(&state).expect("derived assessment");
    let obligation = &assessment.obligations[0];
    assert_eq!(
        obligation.primary_source.as_ref().unwrap().status,
        ContractAssessmentStatus::Satisfied
    );
    assert_eq!(
        obligation
            .independent_corroboration
            .as_ref()
            .unwrap()
            .status,
        ContractAssessmentStatus::Satisfied
    );
    assert_eq!(
        obligation
            .independent_corroboration
            .as_ref()
            .unwrap()
            .source_ids,
        ["source:corroborating", "source:primary"]
    );

    let event =
        research_contract_assessment_event(&state, assessment).expect("assessment event");
    state
        .apply(&event, &InquiryLimits::default())
        .expect("apply assessment");
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Satisfied)
    );
}

#[test]
fn one_typed_independent_source_remains_bounded() {
    let mut state = quality_state();
    state
        .evidence_catalog
        .get_mut("evidence:corroborating")
        .unwrap()
        .source_coverage
        .clear();

    let assessment = derive_research_contract_assessment(&state).expect("derived assessment");
    assert_eq!(
        assessment.obligations[0]
            .independent_corroboration
            .as_ref()
            .unwrap()
            .status,
        ContractAssessmentStatus::Bounded
    );
}

#[test]
fn host_derivation_maps_each_question_to_its_typed_criterion_edge() {
    let limits = InquiryLimits::default();
    let obligation = ResearchObligation::new(
        "obligation:mapped",
        "Mapped coverage",
        "Exercise structural question-to-criterion coverage",
        true,
        vec![
            "The first criterion has direct evidence".to_string(),
            "The second criterion is explicitly bounded".to_string(),
        ],
    );
    let mut state = InquiryState::default();
    for event in [
        InquiryEvent::StrategySelected {
            method: ResearchMethod::Focused,
        },
        InquiryEvent::ResearchObligationsCommitted {
            obligations: vec![obligation],
            stop_conditions: vec!["Every material edge is terminal".to_string()],
        },
    ] {
        state.apply(&event, &limits).expect("contract prefix");
    }
    let mut first = Question::queued("question:first", None, "Resolve criterion zero");
    first.obligation_ids = vec!["obligation:mapped".to_string()];
    first.completion_criterion_indexes = vec![0];
    let mut second = Question::queued("question:second", None, "Resolve criterion one");
    second.obligation_ids = vec!["obligation:mapped".to_string()];
    second.completion_criterion_indexes = vec![1];
    state
        .apply(
            &InquiryEvent::QuestionsQueued {
                questions: vec![first, second],
            },
            &limits,
        )
        .expect("mapped questions");
    state
        .apply(
            &InquiryEvent::EvidenceAccepted {
                evidence: EvidenceRef::new(
                    "evidence:mapped",
                    vec!["claim:mapped".to_string()],
                    vec!["source:mapped".to_string()],
                ),
            },
            &limits,
        )
        .expect("mapped evidence");
    state
        .apply(
            &InquiryEvent::QuestionAnswered {
                question_id: "question:first".to_string(),
                answer: "The accepted evidence resolves criterion zero.".to_string(),
                evidence_ids: vec!["evidence:mapped".to_string()],
            },
            &limits,
        )
        .expect("mapped answer");
    state
        .apply(
            &InquiryEvent::QuestionBounded {
                question_id: "question:second".to_string(),
                reason: "The closed packet does not resolve criterion one.".to_string(),
            },
            &limits,
        )
        .expect("mapped bound");

    let assessment = derive_research_contract_assessment(&state).expect("derived assessment");
    assert_eq!(
        assessment.obligations[0].criteria[0].status,
        ContractAssessmentStatus::Satisfied
    );
    assert_eq!(
        assessment.obligations[0].criteria[1].status,
        ContractAssessmentStatus::Bounded
    );
    assert_eq!(
        assessment.obligations[0].criteria[1].evidence_ids,
        ["evidence:mapped"]
    );
    assert_eq!(
        assessment.stop_conditions[0].status,
        ContractAssessmentStatus::Bounded
    );
}

#[test]
fn partial_answer_retains_material_evidence_and_derives_qualified_contract() {
    let limits = InquiryLimits::default();
    let obligation = ResearchObligation::new(
        "obligation:partial",
        "Partially supported material finding",
        "Retain supported facts while bounding the missing comparison edge",
        true,
        vec!["The available comparison is traceable and its gap is explicit".to_string()],
    );
    let mut state = InquiryState::default();
    for event in [
        InquiryEvent::StrategySelected {
            method: ResearchMethod::Focused,
        },
        InquiryEvent::ResearchObligationsCommitted {
            obligations: vec![obligation],
            stop_conditions: vec![
                "The material finding is traceable or explicitly qualified".to_string()
            ],
        },
    ] {
        state.apply(&event, &limits).expect("partial prefix");
    }
    let mut question = Question::queued(
        "question:partial",
        None,
        "Which comparison facts are supported and what remains unknown?",
    );
    question.obligation_ids = vec!["obligation:partial".to_string()];
    state
        .apply(
            &InquiryEvent::QuestionsQueued {
                questions: vec![question],
            },
            &limits,
        )
        .expect("partial question");
    state
        .apply(
            &InquiryEvent::EvidenceAccepted {
                evidence: EvidenceRef::new(
                    "evidence:partial",
                    vec!["claim:partial".to_string()],
                    vec!["source:partial".to_string()],
                ),
            },
            &limits,
        )
        .expect("partial evidence");
    state
        .apply(
            &InquiryEvent::QuestionPartiallyAnswered {
                question_id: "question:partial".to_string(),
                answer: "The retained evidence establishes the dominant supported path."
                    .to_string(),
                limitation:
                    "The packet does not establish the remaining named compatibility cases."
                        .to_string(),
                evidence_ids: vec!["evidence:partial".to_string()],
            },
            &limits,
        )
        .expect("partial answer");

    assert_eq!(state.phase, InquiryPhase::Outlining);
    assert_eq!(state.questions[0].status, QuestionStatus::Answered);
    assert_eq!(
        state.questions[0].bound_reason.as_deref(),
        Some("The packet does not establish the remaining named compatibility cases.")
    );
    assert_eq!(state.questions[0].evidence_ids, ["evidence:partial"]);
    assert!(material_evidence_floor(&state));

    let assessment = derive_research_contract_assessment(&state).expect("partial assessment");
    assert_eq!(
        assessment.obligations[0].criteria[0].status,
        ContractAssessmentStatus::Bounded
    );
    assert_eq!(
        assessment.obligations[0].criteria[0].evidence_ids,
        ["evidence:partial"]
    );
    assert_eq!(
        assessment.stop_conditions[0].status,
        ContractAssessmentStatus::Bounded
    );
    let event = research_contract_assessment_event(&state, assessment)
        .expect("partial assessment event");
    state
        .apply(&event, &limits)
        .expect("apply partial assessment");
    assert_eq!(
        research_contract_outcome(&state),
        Some(ResearchContractOutcome::Qualified)
    );
}
