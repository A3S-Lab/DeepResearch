use super::*;

fn assessed_inquiry_workflow() -> String {
    use a3s::research::{
        replay, CompletionCriterionAssessment, ContractAssessmentStatus, EvidenceRef, InquiryEvent,
        InquiryLimits, Question, ResearchContractAssessment, ResearchMethod, ResearchObligation,
        ResearchObligationAssessment, StopConditionAssessment,
    };

    let mut core = Question::queued(
        "question:core",
        None,
        "What does the accepted evidence establish?",
    );
    core.obligation_ids = vec!["obligation:core".to_string()];
    let mut context = Question::queued(
        "question:context",
        None,
        "Which supporting limitation can be established?",
    );
    context.obligation_ids = vec!["obligation:core".to_string()];
    context.material = false;
    let events = vec![
        InquiryEvent::StrategySelected {
            method: ResearchMethod::Focused,
        },
        InquiryEvent::ResearchObligationsCommitted {
            obligations: vec![ResearchObligation::new(
                "obligation:core",
                "Evidence-backed answer",
                "Establish the core answer and bound unavailable context",
                true,
                vec!["The core answer is traceable to accepted evidence".to_string()],
            )],
            stop_conditions: vec![
                "Every planned question is answered or explicitly bounded".to_string()
            ],
        },
        InquiryEvent::QuestionsQueued {
            questions: vec![core, context],
        },
        InquiryEvent::EvidenceAccepted {
            evidence: EvidenceRef::new(
                "evidence:core",
                vec!["claim:core".to_string()],
                vec!["source:core".to_string()],
            ),
        },
        InquiryEvent::QuestionAnswered {
            question_id: "question:core".to_string(),
            answer: "The accepted source supports the core answer.".to_string(),
            evidence_ids: vec!["evidence:core".to_string()],
        },
        InquiryEvent::QuestionBounded {
            question_id: "question:context".to_string(),
            reason: "The available evidence does not establish the supporting limitation."
                .to_string(),
        },
        InquiryEvent::ResearchContractAssessed {
            assessment: ResearchContractAssessment {
                obligations: vec![ResearchObligationAssessment {
                    obligation_id: "obligation:core".to_string(),
                    criteria: vec![CompletionCriterionAssessment {
                        criterion_index: 0,
                        status: ContractAssessmentStatus::Satisfied,
                        rationale: "The accepted evidence satisfies the core criterion."
                            .to_string(),
                        evidence_ids: vec!["evidence:core".to_string()],
                    }],
                    primary_source: None,
                    independent_corroboration: None,
                }],
                stop_conditions: vec![StopConditionAssessment {
                    condition_index: 0,
                    status: ContractAssessmentStatus::Satisfied,
                    rationale: "Both questions reached a closed terminal state.".to_string(),
                    evidence_ids: vec!["evidence:core".to_string()],
                }],
                diagnostics: Vec::new(),
            },
        },
    ];
    let state = replay(&events, &InquiryLimits::default()).expect("valid assessed inquiry fixture");
    serde_json::json!({
        "mode": "inquiry_collection_wave",
        "execution": {
            "mode": "collect_only",
            "terminal_authority": "host_inquiry_reducer"
        },
        "query": "must not be copied into report_context",
        "plan": {
            "report_title": "Reader-facing title",
            "answer_shape": "must not survive",
            "execution_route": "must not survive",
            "phases": ["must not survive"],
            "tracks": [{
                "id": "obligation:core",
                "title": "Evidence-backed answer",
                "focus": "Establish the core answer and bound unavailable context",
                "material": true
            }],
            "stop_conditions": [
                "Every planned question is answered or explicitly bounded"
            ],
            "search_queries": ["internal retrieval instruction"]
        },
        "checker": {
            "decision": "finalize",
            "verified_findings": ["legacy checker finding must not survive"]
        },
        "verification": {
            "status": "completed",
            "checker_completed": true,
            "error": "legacy verification detail must not survive"
        },
        "inquiry": {
            "events": events,
            "state": state
        }
    })
    .to_string()
}

#[test]
fn synthesis_payload_carries_only_plan_and_replayed_inquiry_context() {
    let evidence = AcceptedEvidence {
        id: "evidence:1".to_string(),
        summary: "A source-backed summary.".to_string(),
        confidence: Some("high".to_string()),
        sources: vec![AcceptedSource {
            id: "source:1".to_string(),
            anchor: "https://example.com/source".to_string(),
            title: Some("Source".to_string()),
            date: None,
            reliability: Some("Official".to_string()),
            quote_or_fact: Some("The primary finding is supported.".to_string()),
            evidence_excerpts: Vec::new(),
        }],
        claims: vec![],
        source_coverage: Vec::new(),
        relevant_obligation_ids: Vec::new(),
        contradictions: vec![],
        gaps: vec![],
    };
    let workflow = assessed_inquiry_workflow();
    let payload = synthesis_payload_with_context(&[evidence], &workflow);
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();

    let context = &payload["report_context"];
    assert_eq!(context["plan"]["report_title"], "Reader-facing title");
    assert_eq!(
        context["plan"]["tracks"][0],
        serde_json::json!({
            "id": "obligation:core",
            "title": "Evidence-backed answer",
            "focus": "Establish the core answer and bound unavailable context",
            "material": true
        })
    );
    assert_eq!(
        context["plan"]["stop_conditions"],
        serde_json::json!(["Every planned question is answered or explicitly bounded"])
    );
    assert_eq!(context["inquiry"]["contract_outcome"], "qualified");
    assert_eq!(
        context["inquiry"]["obligations"][0]["id"],
        "obligation:core"
    );
    assert_eq!(
        context["inquiry"]["contract_assessment"]["obligations"][0]["criteria"][0]["status"],
        "satisfied"
    );
    assert_eq!(context["inquiry"]["questions"][0]["status"], "answered");
    assert_eq!(
        context["inquiry"]["questions"][0]["answer"],
        "The accepted source supports the core answer."
    );
    assert_eq!(
        context["inquiry"]["questions"][0]["evidence_ids"],
        serde_json::json!(["evidence:core"])
    );
    assert_eq!(context["inquiry"]["questions"][1]["status"], "bounded");
    assert_eq!(
        context["inquiry"]["questions"][1]["bound_reason"],
        "The available evidence does not establish the supporting limitation."
    );

    for omitted in [
        "query",
        "answer_shape",
        "execution_route",
        "phases",
        "search_queries",
        "checker",
        "verification",
        "verified_findings",
        "unresolved_obligations",
        "publication_status",
    ] {
        assert!(
            !context.to_string().contains(omitted),
            "{omitted} leaked into report context: {context:#}"
        );
    }
    assert!(!context
        .to_string()
        .contains("legacy checker finding must not survive"));
    assert!(!context
        .to_string()
        .contains("legacy verification detail must not survive"));
}

#[test]
fn synthesis_payload_exposes_verification_facts_without_internal_publication_status() {
    let evidence = AcceptedEvidence {
        id: "evidence:1".to_string(),
        summary: "A traceable result survived the checker failure.".to_string(),
        confidence: Some("medium".to_string()),
        sources: vec![AcceptedSource {
            id: "source:1".to_string(),
            anchor: "https://example.com/source".to_string(),
            title: Some("Source".to_string()),
            date: None,
            reliability: Some("Official".to_string()),
            quote_or_fact: Some("A traceable result survived the checker failure.".to_string()),
            evidence_excerpts: Vec::new(),
        }],
        claims: vec![],
        source_coverage: Vec::new(),
        relevant_obligation_ids: Vec::new(),
        contradictions: vec![],
        gaps: vec![],
    };
    let workflow = serde_json::json!({
        "plan": { "report_title": "Qualified result" },
        "verification": {
            "status": "degraded",
            "checker_completed": false,
            "prior_checker_retained": true,
            "error": "internal provider failure must not leak"
        }
    })
    .to_string();

    let payload = synthesis_payload_with_context(&[evidence], &workflow);
    let payload: serde_json::Value = serde_json::from_str(&payload).unwrap();
    assert!(payload["report_context"]
        .get("publication_status")
        .is_none());
    assert_eq!(
        payload["report_context"]["verification"]["checker_completed"],
        false
    );
    assert!(!payload
        .to_string()
        .contains("internal provider failure must not leak"));
}
