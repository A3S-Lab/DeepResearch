#[test]
fn raw_acquisition_sources_remain_audit_only_without_semantic_projection() {
    let query = "Assess the migration decision";
    let output = fallback_source_backed_fixture(
        query,
        serde_json::json!([
            source_fixture(
                "bootstrap-web-source-1",
                "Published migration note",
                "https://example.test/migration-note",
                "The migration note records the proposed support boundary."
            ),
            source_fixture(
                "bootstrap-web-source-2",
                "Workspace migration note",
                "evidence/migration-note.md",
                "The workspace note records the proposed support boundary."
            )
        ]),
    );
    let workspace = tempfile::tempdir().expect("create source-backed workspace");

    let artifacts = materialize_deep_research_source_backed_report(
        workspace.path(),
        query,
        &output.to_string(),
        None,
    )
    .expect("materialize source-backed report")
    .expect("source-backed artifacts");
    let markdown = std::fs::read_to_string(artifacts.markdown).expect("read Markdown");
    let html = std::fs::read_to_string(artifacts.html).expect("read HTML");

    assert!(
        markdown.contains("Claim eligibility: not eligible for conclusions"),
        "{markdown}"
    );
    assert!(
        markdown.contains("did not pass the run's structured evidence-admission boundary"),
        "{markdown}"
    );
    assert_eq!(
        markdown
            .matches("Claim eligibility: not eligible for conclusions")
            .count(),
        2
    );
    assert!(
        html.contains("Claim eligibility: not eligible for conclusions"),
        "{html}"
    );
    assert_eq!(
        html.matches("Claim eligibility: not eligible for conclusions")
            .count(),
        2
    );
    assert!(html.contains("report-degraded"), "{html}");
}
#[test]
fn rejects_cross_query_catalog_replay() {
    let output = source_backed_fixture(
        "original query",
        serde_json::json!([source_fixture(
            "bootstrap-web-source-1",
            "Source",
            "https://example.test/source",
            "Traceable source content."
        )]),
    );
    let error = deep_research_source_catalog("different query", &output.to_string(), None)
        .expect_err("cross-query source replay must fail");
    assert!(error.contains("different query"));
}

#[test]
fn no_evidence_report_uses_the_users_language_and_is_rediscoverable() {
    let workspace = tempfile::tempdir().expect("create no-evidence workspace");
    let query = "核查 Nimbus 当前备份策略";
    let artifacts = materialize_deep_research_no_evidence_report(workspace.path(), query)
        .expect("materialize no-evidence report");
    let markdown = std::fs::read_to_string(&artifacts.markdown).expect("read Markdown");
    let html = std::fs::read_to_string(&artifacts.html).expect("read HTML");

    assert!(markdown.contains(NO_EVIDENCE_ARTIFACT_MARKER));
    assert!(html.contains(NO_EVIDENCE_ARTIFACT_MARKER));
    assert!(markdown.contains("## 证据状态"));
    assert!(markdown.contains("检索失败不等于相关事实不存在"));
    assert!(markdown.contains("## 来源"));
    assert!(html.contains("<html lang=\"zh\">"));
    assert!(!markdown.contains("workflow"));
    assert!(!markdown.contains("model"));

    let slug = deep_research_report_slug(query);
    let output = evidence_first_publication_fixture(query, &slug, "no_evidence");
    let published =
        deep_research_evidence_first_published_report(workspace.path(), query, &output.to_string())
            .expect("rediscover no-evidence publication")
            .expect("published no-evidence report");

    assert_eq!(
        published.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(published.artifacts, artifacts);
}

#[test]
fn run_scoped_artifacts_isolate_concurrent_equal_queries() {
    let workspace = tempfile::tempdir().expect("create run-scoped workspace");
    let query = "Assess the same release boundary";
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
        ..DeepResearchPublicationQuality::default()
    };
    let first =
        materialize_deep_research_no_evidence_report_for_run(workspace.path(), "run-a", query)
            .expect("materialize first run");
    let second =
        materialize_deep_research_no_evidence_report_for_run(workspace.path(), "run-b", query)
            .expect("materialize second run");

    assert_ne!(first, second);
    assert!(first
        .html
        .ends_with(".a3s/research/artifacts/run-a/index.html"));
    assert!(second
        .html
        .ends_with(".a3s/research/artifacts/run-b/index.html"));
    record_deep_research_publication_receipt(
        workspace.path(),
        query,
        "run-a",
        DeepResearchEvidenceFirstPublication::NoEvidence,
        quality,
        &first,
    )
    .expect("record first run receipt");
    record_deep_research_publication_receipt(
        workspace.path(),
        query,
        "run-b",
        DeepResearchEvidenceFirstPublication::NoEvidence,
        quality,
        &second,
    )
    .expect("record second run receipt");

    let recovered_first =
        recover_deep_research_publication_receipt(workspace.path(), query, "run-a")
            .expect("recover first run")
            .expect("first run receipt");
    let recovered_second =
        recover_deep_research_publication_receipt(workspace.path(), query, "run-b")
            .expect("recover second run")
            .expect("second run receipt");
    assert_eq!(recovered_first.artifacts, first);
    assert_eq!(recovered_second.artifacts, second);
    assert!(materialize_deep_research_no_evidence_report_for_run(
        workspace.path(),
        "../escape",
        query,
    )
    .is_err());
}

#[test]
fn publication_rediscovery_validates_both_artifact_paths() {
    let workspace = tempfile::tempdir().expect("create publication workspace");
    let query = "Which release is supported?";
    let acquisition = source_backed_fixture(
        query,
        serde_json::json!([source_fixture(
            "bootstrap-web-source-1",
            "Release policy",
            "https://docs.example.test/policy",
            "Version 2 receives fixes through September 2027."
        )]),
    );
    let artifacts = materialize_deep_research_source_backed_report(
        workspace.path(),
        query,
        &acquisition.to_string(),
        None,
    )
    .expect("materialize source-backed report")
    .expect("source-backed artifacts");
    let slug = deep_research_report_slug(query);
    let output = evidence_first_publication_fixture(query, &slug, "source_backed");

    let published =
        deep_research_evidence_first_published_report(workspace.path(), query, &output.to_string())
            .expect("rediscover source-backed publication")
            .expect("published source-backed report");
    assert_eq!(published.artifacts, artifacts);

    let mut forged_success = output.clone();
    forged_success["publication"]["status"] = serde_json::json!("synthesized");
    forged_success["publication"]["quality"] = serde_json::json!({
        "direct_answer_count": 1,
        "finding_count": 1,
        "accepted_claim_count": 2,
        "cited_source_count": 1,
        "substantive_character_count": 120,
        "relevant_source_count": 1,
        "source_count": 1
    });
    let error = deep_research_evidence_first_published_report(
        workspace.path(),
        query,
        &forged_success.to_string(),
    )
    .expect_err("a source snapshot must never validate as a synthesized report");
    assert!(error.contains("content validation"), "{error}");

    let mut tampered = output;
    tampered["publication"]["markdown"] =
        serde_json::json!(format!(".a3s/research/{slug}/other.md"));
    let error = deep_research_evidence_first_published_report(
        workspace.path(),
        query,
        &tampered.to_string(),
    )
    .expect_err("unexpected Markdown artifact must be rejected");
    assert!(error.contains("unexpected artifact"));
}

#[test]
fn ineligible_audit_sources_do_not_poison_synthesized_quality_metrics() {
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
        direct_answer_count: 1,
        finding_count: 1,
        accepted_claim_count: 2,
        accepted_relation_count: 0,
        accepted_derivation_count: 0,
        accepted_basis_edge_count: 0,
        analytical_claim_count: 0,
        cross_source_synthesis_count: 0,
        resolved_material_dimension_count: 0,
        deeply_analyzed_dimension_count: 0,
        accepted_gap_count: 0,
        cited_source_count: 1,
        substantive_character_count: 120,
        relevant_source_count: 1,
        source_count: 5,
    };

    validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Synthesized,
        quality,
    )
    .expect("one semantically admitted source may coexist with audit-only sources");

    let invalid = DeepResearchPublicationQuality {
        cited_source_count: 2,
        ..quality
    };
    assert!(validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Synthesized,
        invalid,
    )
    .is_err());
}

#[test]
fn broad_publication_quality_requires_report_depth_metrics() {
    let shallow = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Comprehensive,
        direct_answer_count: 1,
        finding_count: 5,
        accepted_claim_count: 6,
        accepted_relation_count: 0,
        accepted_derivation_count: 0,
        accepted_basis_edge_count: 6,
        analytical_claim_count: 3,
        cross_source_synthesis_count: 1,
        resolved_material_dimension_count: 1,
        deeply_analyzed_dimension_count: 1,
        accepted_gap_count: 0,
        cited_source_count: 2,
        substantive_character_count: 1_199,
        relevant_source_count: 4,
        source_count: 4,
    };

    assert!(validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Synthesized,
        shallow,
    )
    .is_err());

    let fact_only = DeepResearchPublicationQuality {
        accepted_basis_edge_count: 0,
        analytical_claim_count: 0,
        cross_source_synthesis_count: 0,
        substantive_character_count: 1_200,
        ..shallow
    };
    assert!(
        validate_deep_research_publication_quality(
            DeepResearchEvidenceFirstPublication::Synthesized,
            fact_only,
        )
        .is_err(),
        "a durable fact inventory must not be rediscovered as deep synthesis"
    );

    validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Synthesized,
        DeepResearchPublicationQuality {
            substantive_character_count: 1_200,
            ..shallow
        },
    )
    .expect("a broad publication that meets every depth metric should pass");

    let all_bounded = DeepResearchPublicationQuality {
        resolved_material_dimension_count: 0,
        deeply_analyzed_dimension_count: 1,
        accepted_gap_count: 1,
        substantive_character_count: 1_200,
        ..shallow
    };
    assert!(
        validate_deep_research_publication_quality(
            DeepResearchEvidenceFirstPublication::Qualified,
            all_bounded,
        )
        .is_err(),
        "an all-bounded comprehensive report must never pass the commercial publication gate"
    );
    assert!(validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Synthesized,
        all_bounded,
    )
    .is_err());
}

#[test]
fn qualified_publication_requires_a_persisted_typed_gap() {
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
        direct_answer_count: 1,
        finding_count: 0,
        accepted_claim_count: 1,
        accepted_relation_count: 0,
        accepted_derivation_count: 0,
        accepted_basis_edge_count: 0,
        analytical_claim_count: 0,
        cross_source_synthesis_count: 0,
        resolved_material_dimension_count: 0,
        deeply_analyzed_dimension_count: 0,
        accepted_gap_count: 0,
        cited_source_count: 1,
        substantive_character_count: 80,
        relevant_source_count: 1,
        source_count: 1,
    };

    assert!(validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Qualified,
        quality,
    )
    .is_err());
    let qualified = DeepResearchPublicationQuality {
        accepted_gap_count: 1,
        ..quality
    };
    validate_deep_research_publication_quality(
        DeepResearchEvidenceFirstPublication::Qualified,
        qualified,
    )
    .expect("qualified status must be backed by an explicit typed evidence gap");
    assert!(
        validate_deep_research_publication_quality(
            DeepResearchEvidenceFirstPublication::Synthesized,
            qualified,
        )
        .is_err(),
        "a publication with an accepted gap must not claim synthesized completion"
    );
}

#[test]
fn publication_receipt_recovers_only_the_exact_run_and_artifact_generation() {
    let workspace = tempfile::tempdir().expect("create publication receipt workspace");
    let query = "Assess the current release boundary";
    let run_id = "research-publication-receipt";
    let artifacts = materialize_deep_research_no_evidence_report(workspace.path(), query)
        .expect("materialize no-evidence publication");
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
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
    };

    record_deep_research_publication_receipt(
        workspace.path(),
        query,
        run_id,
        DeepResearchEvidenceFirstPublication::NoEvidence,
        quality,
        &artifacts,
    )
    .expect("record exact publication receipt");

    let receipt_path = artifacts
        .html
        .parent()
        .expect("publication directory")
        .join("publication-receipt.json");
    let mut legacy_receipt: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&receipt_path).expect("read current publication receipt"),
    )
    .expect("decode current publication receipt");
    assert_eq!(legacy_receipt["schema_version"], 5);
    assert_eq!(legacy_receipt["output_language"], "en");
    assert!(recover_deep_research_publication_receipt_in_language(
        workspace.path(),
        query,
        "zh",
        run_id,
    )
    .expect("reject a receipt from another output language")
    .is_none());
    legacy_receipt["schema_version"] = serde_json::json!(1);
    legacy_receipt
        .as_object_mut()
        .expect("receipt object")
        .remove("output_language");
    let legacy_quality = legacy_receipt["quality"]
        .as_object_mut()
        .expect("receipt quality object");
    for field in [
        "accepted_relation_count",
        "accepted_derivation_count",
        "accepted_basis_edge_count",
        "analytical_claim_count",
        "cross_source_synthesis_count",
        "accepted_gap_count",
    ] {
        legacy_quality.remove(field);
    }
    std::fs::write(
        &receipt_path,
        serde_json::to_vec_pretty(&legacy_receipt).expect("encode version-1 receipt"),
    )
    .expect("write compatible version-1 receipt");

    assert!(recover_deep_research_publication_receipt_in_language(
        workspace.path(),
        query,
        "en",
        run_id,
    )
    .expect("language-bound recovery treats a legacy receipt as typed absence")
    .is_none());
    let recovered = recover_deep_research_publication_receipt(workspace.path(), query, run_id)
        .expect("recover exact publication receipt")
        .expect("receipt-backed publication");
    assert_eq!(recovered.artifacts, artifacts);
    assert_eq!(
        recovered.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(recovered.quality, quality);
    assert!(
        recover_deep_research_publication_receipt(workspace.path(), query, "another-run")
            .expect("reject another run without treating it as corruption")
            .is_none()
    );

    std::fs::write(
        &artifacts.markdown,
        "# Replaced report\n\nThe receipt must not authorize this generation.\n",
    )
    .expect("replace one artifact after receipt");
    assert!(
        recover_deep_research_publication_receipt(workspace.path(), query, run_id)
            .expect("digest mismatch is typed absence rather than content recovery")
            .is_none()
    );
}

#[test]
fn exact_run_receipt_recovers_a_committed_publication_after_an_ambiguous_failure() {
    let workspace = tempfile::tempdir().expect("create receipt resolution workspace");
    let query = "Assess the current release boundary";
    let run_id = "receipt-resolution-run";
    let artifacts = materialize_deep_research_no_evidence_report(workspace.path(), query)
        .expect("materialize no-evidence publication");
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
        ..DeepResearchPublicationQuality::default()
    };
    record_deep_research_publication_receipt(
        workspace.path(),
        query,
        run_id,
        DeepResearchEvidenceFirstPublication::NoEvidence,
        quality,
        &artifacts,
    )
    .expect("record committed publication receipt");

    let resolved = resolve_deep_research_run_publication(
        workspace.path(),
        query,
        run_id,
        "publication port returned an ambiguous failure",
    )
    .expect("resolve exact committed publication")
    .expect("receipt-backed publication");

    assert_eq!(resolved.artifacts, artifacts);
    assert_eq!(
        resolved.publication,
        DeepResearchEvidenceFirstPublication::NoEvidence
    );
    assert_eq!(resolved.quality, quality);
}

#[test]
fn exact_run_receipt_rejects_a_conflicting_structured_publication() {
    let workspace = tempfile::tempdir().expect("create receipt conflict workspace");
    let query = "Assess the current release boundary";
    let run_id = "receipt-conflict-run";
    let artifacts = materialize_deep_research_no_evidence_report(workspace.path(), query)
        .expect("materialize no-evidence publication");
    let quality = DeepResearchPublicationQuality {
        research_scope: DeepResearchReportScope::Focused,
        ..DeepResearchPublicationQuality::default()
    };
    record_deep_research_publication_receipt(
        workspace.path(),
        query,
        run_id,
        DeepResearchEvidenceFirstPublication::NoEvidence,
        quality,
        &artifacts,
    )
    .expect("record committed publication receipt");
    let slug = deep_research_report_slug(query);
    let conflicting_output = serde_json::json!({
        "query": query,
        "output_language": "en",
        "mode": "evidence_first_report",
        "publication": {
            "status": "no_evidence",
            "markdown": format!(".a3s/research/{slug}/report.md"),
            "html": format!(".a3s/research/{slug}/index.html"),
            "quality": {
                "research_scope": "comprehensive",
                "direct_answer_count": 0,
                "finding_count": 0,
                "accepted_claim_count": 0,
                "accepted_relation_count": 0,
                "accepted_derivation_count": 0,
                "accepted_basis_edge_count": 0,
                "accepted_gap_count": 0,
                "cited_source_count": 0,
                "substantive_character_count": 0,
                "relevant_source_count": 0,
                "source_count": 0,
            },
        },
    })
    .to_string();

    let error =
        resolve_deep_research_run_publication(workspace.path(), query, run_id, &conflicting_output)
            .expect_err("conflicting durable authorities must fail closed");

    assert_eq!(
        error,
        "the workflow publication disagrees with the exact run receipt"
    );
}
