  const remainingWebCandidates = (discovery, excludedCandidates) => {
    const excludedIds = new Set(
      (excludedCandidates || []).map((candidate) => candidate.candidate_id)
    );
    return (Array.isArray(discovery && discovery.candidates)
      ? discovery.candidates
      : []
    ).filter((candidate) => !excludedIds.has(candidate.candidate_id));
  };

  const retainedEvidenceReferenceCandidates = (
    discovery,
    packet,
    semanticSelection
  ) => {
    const isMarkdownImageTarget = (text, urlStart) => {
      if (urlStart < 2 || text.slice(urlStart - 2, urlStart) !== "](") {
        return false;
      }
      const labelOpen = text.lastIndexOf("[", urlStart - 2);
      return labelOpen > 0 && text[labelOpen - 1] === "!";
    };
    if (
      !packet ||
      !Array.isArray(packet.sources) ||
      !semanticSelection ||
      !Array.isArray(semanticSelection.chunk_ids)
    ) {
      return [];
    }
    const selectedChunkIds = new Set(semanticSelection.chunk_ids);
    const knownUrls = new Set([
      ...(Array.isArray(discovery && discovery.candidates)
        ? discovery.candidates
        : []),
      ...packet.sources,
    ].map((item) => canonicalUrl(item.url || item.url_or_path)).filter(nonEmpty));
    const references = [];
    const referenceUrls = new Set();
    for (const source of packet.sources) {
      for (const chunk of Array.isArray(source.chunks) ? source.chunks : []) {
        if (!selectedChunkIds.has(chunk.chunk_id)) {
          continue;
        }
        const text = String(chunk.text || "");
        const pattern = /https:\/\/[^\s<>"']+/gi;
        let match = null;
        while ((match = pattern.exec(text)) !== null) {
          const rawUrl = match[0];
          if (
            isMarkdownImageTarget(text, match.index) ||
            match.index + rawUrl.length === text.length &&
            !/[.,;:!?\]})]$/.test(rawUrl)
          ) {
            continue;
          }
          const url = cleanUrl(rawUrl.replace(/\\([()])/g, "$1"));
          const key = canonicalUrl(url);
          if (
            !url ||
            !/^https:\/\//i.test(url) ||
            !key ||
            knownUrls.has(key) ||
            referenceUrls.has(key)
          ) {
            continue;
          }
          referenceUrls.add(key);
          references.push({
            candidate_id: `evidence-reference-${references.length + 1}`,
            title: "",
            url,
            date: "",
            content: bounded(chunk.text, 600),
            engines: [],
            discovery: ["retained_evidence_reference"],
            query_indexes: [],
          });
          if (references.length >= MAX_DISCOVERY_CANDIDATES) {
            return references;
          }
        }
      }
    }
    return references;
  };

  const initialWebSourceAttempts = (
    initialCandidates,
    packet,
    semanticSelection
  ) => {
    const selectedChunkIds = new Set(
      semanticSelection && Array.isArray(semanticSelection.chunk_ids)
        ? semanticSelection.chunk_ids
        : []
    );
    const sources = packet && Array.isArray(packet.sources)
      ? packet.sources
      : [];
    return (Array.isArray(initialCandidates) ? initialCandidates : []).map(
      (candidate) => {
        const candidateUrls = new Set([
          canonicalUrl(candidate.url),
          canonicalUrl(fetchUrl(candidate.url)),
        ].filter(nonEmpty));
        const source = sources.find((item) =>
          candidateUrls.has(canonicalUrl(item.url_or_path))
        );
        const retained = source && Array.isArray(source.chunks) &&
          source.chunks.some((chunk) => selectedChunkIds.has(chunk.chunk_id));
        return {
          candidate_id: candidate.candidate_id,
          url: candidate.url,
          title: candidate.title || "",
          outcome: !source
            ? "fetch_failed"
            : (retained ? "retained" : "selection_empty"),
        };
      }
    );
  };

  const supplementalWebCandidates = (
    discovery,
    gapDiscovery,
    excludedCandidates,
    packet,
    semanticSelection,
    activeQueries,
    priorQueries
  ) => {
    const withDiscoveryQueries = (candidate, queries) =>
      Object.assign({}, candidate, {
        discovery_queries: uniqueStrings(
          (Array.isArray(candidate && candidate.query_indexes)
            ? candidate.query_indexes
            : [])
            .filter((index) => Number.isSafeInteger(index) && index >= 0)
            .map((index) => Array.isArray(queries) ? queries[index] : "")
            .filter(nonEmpty)
        ),
      });
    const excludedUrls = new Set(
      [
        ...(excludedCandidates || []).map((candidate) => candidate.url),
        ...(packet && Array.isArray(packet.sources)
          ? packet.sources.map((source) => source.url_or_path)
          : []),
      ]
        .map(canonicalUrl)
        .filter(nonEmpty)
    );
    const candidates = [
      ...retainedEvidenceReferenceCandidates(
        discovery,
        packet,
        semanticSelection
      ),
      ...(Array.isArray(gapDiscovery && gapDiscovery.candidates)
        ? gapDiscovery.candidates.map((candidate) =>
            withDiscoveryQueries(candidate, activeQueries)
          )
        : []),
      ...remainingWebCandidates(discovery, excludedCandidates).map(
        (candidate) => withDiscoveryQueries(candidate, priorQueries)
      ),
    ];
    const seenUrls = new Set();
    return candidates
      .filter((candidate) => {
        const url = canonicalUrl(candidate && candidate.url);
        return url && !excludedUrls.has(url) && !seenUrls.has(url) && seenUrls.add(url);
      })
      .slice(0, MAX_DISCOVERY_CANDIDATES)
      .map((candidate, index) => Object.assign({}, candidate, {
        candidate_id: `supplemental-candidate-${index + 1}`,
      }));
  };

  const packetForCoverageGaps = (packet, coverageGaps) => {
    if (!packet) {
      return null;
    }
    if (coverageGaps.length === 0) {
      return packet;
    }
    const obligationIds = new Set(
      coverageGaps.map((gap) => gap.obligation_id)
    );
    const focuses = packet.focuses.filter((focus) =>
      obligationIds.has(focus.obligation_id)
    );
    return focuses.length > 0
      ? {
          version: packet.version,
          focuses,
          sources: packet.sources,
        }
      : null;
  };

  const combineMaterializedSelections = (primary, supplemental) => {
    if (!supplemental) {
      return primary;
    }
    const primaryResults = Array.isArray(primary.results) ? primary.results : [];
    const supplementalResults = Array.isArray(supplemental.results)
      ? supplemental.results
      : [];
    const errors = uniqueStrings([
      ...(Array.isArray(primary.errors) ? primary.errors : []),
      ...(Array.isArray(supplemental.errors) ? supplemental.errors : []),
    ]);
    const results = [...primaryResults, ...supplementalResults];
    return {
      status: results.length === 0
        ? "failed"
        : (errors.length > 0 ? "partial" : "success"),
      results,
      errors,
      metadata: Object.assign({}, object(primary.metadata), {
        retrieval_pass_count: Math.max(
          Number(object(primary.metadata).retrieval_pass_count) || 1,
          Number(object(supplemental.metadata).retrieval_pass) || 2
        ),
        supplemental: object(supplemental.metadata),
      }),
    };
  };

  const supplementalRoundStepIds = (round) => {
    const step = (base) => round === 1 ? base : `${base}_${round}`;
    const prefix = (base) => round === 1 ? base : `${base}${round}_`;
    return {
      generate_gap_queries: step(STEP_GENERATE_GAP_QUERIES),
      discover_gap_web: step(STEP_DISCOVER_GAP_WEB),
      select_web: step(STEP_SELECT_SUPPLEMENTAL_WEB),
      select_web_shard_prefix: prefix(
        STEP_SELECT_SUPPLEMENTAL_WEB_SHARD_PREFIX
      ),
      web_source_prefix: prefix(STEP_SUPPLEMENTAL_WEB_SOURCE_PREFIX),
      select_chunks: step(STEP_SELECT_SUPPLEMENTAL),
      select_shard_prefix: prefix(STEP_SELECT_SUPPLEMENTAL_SHARD_PREFIX),
      select_shard_recovery_prefix: prefix(
        STEP_SELECT_SUPPLEMENTAL_SHARD_RECOVERY_PREFIX
      ),
      select_source_prefix: prefix(STEP_SELECT_SUPPLEMENTAL_SOURCE_PREFIX),
    };
  };

  const supplementalCoverageRound = (settings) => {
    const round = clamp(settings.round, 1, MAX_GAP_ROUNDS, 1);
    const fetchBudget = clamp(settings.fetch_budget, 0, MAX_SOURCES, 0);
    const queryBudget = clamp(settings.query_budget, 0, MAX_SEARCH_QUERIES, 0);
    const stepIds = supplementalRoundStepIds(round);
    const plan = object(settings.plan);
    const packet = settings.packet;
    const semanticSelection = settings.semantic_selection;
    const outputs = object(settings.outputs);
    const failures = object(settings.failures);
    if (!settings.needs_web || settings.enabled === false) {
      return {
        schedule: null,
        selection: null,
        coverage_gaps: [],
        attempted: false,
        fetch_count: 0,
        query_count: 0,
      };
    }
    const hasInitialSelection = Boolean(
      packet &&
      semanticSelection &&
      Array.isArray(semanticSelection.chunk_ids)
    );
    const initialChunkIds = new Set(
      hasInitialSelection ? semanticSelection.chunk_ids : []
    );
    let initialCoverageBindings = Array.isArray(settings.coverage_bindings)
      ? settings.coverage_bindings
      : [];
    if (!Array.isArray(settings.coverage_bindings) && hasInitialSelection) {
      const initialCoverage = validatedSourceCoverage(
        packet,
        semanticSelection,
        initialChunkIds
      );
      if (!initialCoverage.error) {
        initialCoverageBindings = initialCoverage.bindings;
      }
    }
    const coverageGaps = typedCoverageGaps(plan, initialCoverageBindings);
    const queryCoverageGaps = prioritizedCoverageGaps(
      plan,
      coverageGaps,
      round,
      queryBudget
    );
    const roundCoverageGaps = queryCoverageGaps.length > 0
      ? queryCoverageGaps
      : coverageGaps;
    const initialCandidates = Array.isArray(settings.initial_candidates)
      ? settings.initial_candidates
      : [];
    const excludedCandidates = Array.isArray(settings.excluded_candidates)
      ? settings.excluded_candidates
      : initialCandidates;
    const initialAttempts = initialWebSourceAttempts(
      initialCandidates,
      packet,
      semanticSelection
    );
    const operationalGapCount = initialAttempts.filter((attempt) =>
      attempt.outcome !== "retained"
    ).length;
    if (coverageGaps.length === 0 && operationalGapCount === 0) {
      return {
        schedule: null,
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: false,
        fetch_count: 0,
        query_count: 0,
      };
    }
    if (fetchBudget === 0) {
      return {
        schedule: null,
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: false,
        fetch_count: 0,
        query_count: 0,
      };
    }
    if (
      queryBudget > 0 &&
      !outputs[stepIds.generate_gap_queries] &&
      !failures[stepIds.generate_gap_queries]
    ) {
      return {
        schedule: {
          type: "schedule_step",
          step_id: stepIds.generate_gap_queries,
          step_name: "generate_object",
          input: gapQueryGeneratorInput(
            plan,
            queryCoverageGaps,
            operationalGapCount,
            initialAttempts,
            queryBudget,
            round
          ),
          retry: settings.gap_query_generation_retry,
        },
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: true,
      };
    }
    const generatedGapQueries = queryBudget > 0
      ? validatedGapQueries(
          plan,
          structuredOutput(outputs[stepIds.generate_gap_queries]),
          queryBudget
        )
      : { queries: [], error: "" };
    const fallbackQueries = generatedGapQueries.queries.length === 0
      ? fallbackGapQueries(plan, queryCoverageGaps, queryBudget)
      : [];
    const gapQueries = fallbackQueries.length > 0
      ? {
          queries: fallbackQueries,
          error: uniqueStrings([
            generatedGapQueries.error || "",
            "Typed gap-query generation used the deterministic atomic-criterion fallback.",
          ]).join(" "),
        }
      : generatedGapQueries;
    if (
      gapQueries.queries.length > 0 &&
      !outputs[stepIds.discover_gap_web] &&
      !failures[stepIds.discover_gap_web]
    ) {
      return {
        schedule: {
          type: "schedule_step",
          step_id: stepIds.discover_gap_web,
          step_name: STEP_DISCOVER_WEB,
          input: {
            query: settings.query,
            plan: gapDiscoveryPlan(plan, gapQueries.queries),
            search_timeout_secs: 12,
          },
          retry: settings.retrieval_retry,
        },
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: true,
      };
    }
    const gapQueryFailure = failures[stepIds.generate_gap_queries] &&
      (failures[stepIds.generate_gap_queries].error ||
        "gap-query generation failed");
    const gapDiscoveryFailure = failures[stepIds.discover_gap_web] &&
      (failures[stepIds.discover_gap_web].error ||
        "gap-query discovery failed");
    const gapDiscovery = outputs[stepIds.discover_gap_web] || {
      status: "failed",
      candidates: [],
      errors: uniqueStrings([
        gapQueries.error || "",
        gapQueryFailure || "",
        gapDiscoveryFailure || "",
      ]),
      metadata: {},
    };
    const remainingCandidates = supplementalWebCandidates(
      settings.web_discovery,
      gapDiscovery,
      excludedCandidates,
      packet,
      semanticSelection,
      gapQueries.queries,
      Array.isArray(plan.search_queries) ? plan.search_queries : []
    );
    const fetchLimit = Math.min(fetchBudget, remainingCandidates.length);
    if (fetchLimit === 0 || remainingCandidates.length === 0) {
      return {
        schedule: null,
        selection: {
          status: "failed",
          results: [],
          errors: uniqueStrings([
            gapQueries.error || "",
            gapQueryFailure || "",
            gapDiscoveryFailure || "",
            ...(Array.isArray(gapDiscovery.errors) ? gapDiscovery.errors : []),
            "Gap-directed discovery retained no new fetchable candidate.",
          ]),
          metadata: {
            retrieval_pass: round + 1,
            coverage_gap_count: coverageGaps.length,
            operational_gap_count: operationalGapCount,
            generated_gap_query_count: gapQueries.queries.length,
            gap_search_budget: queryBudget,
            supplemental_fetch_budget: fetchBudget,
            supplemental_fetch_count: 0,
            source_count: 0,
            selection_count: 0,
          },
        },
        coverage_gaps: coverageGaps,
        attempted: true,
        attempted_candidates: [],
        fetch_count: 0,
        query_count: gapQueries.queries.length,
        queries: gapQueries.queries,
      };
    }

    // A supplemental pass closes typed coverage or replaces evidence lost to
    // an initial fetch/source-selection failure, so it always uses semantic
    // admission even when every remaining candidate would fit.
    const needsSourceSelection = remainingCandidates.length > 0;
    const directSupplementalSelectorInput = needsSourceSelection
      ? supplementalWebSelectorInput(
          plan,
          remainingCandidates,
          roundCoverageGaps,
          fetchLimit,
          operationalGapCount,
          initialAttempts,
          gapQueries.queries
        )
      : null;
    const supplementalWebShardEntries = needsSourceSelection
      ? sourceSelectionShardEntries(
          remainingCandidates,
          fetchLimit,
          stepIds.select_web_shard_prefix
        )
      : [];
    if (supplementalWebShardEntries.length > 0) {
      const pendingShardSteps = supplementalWebShardEntries
        .map((entry) => ({
          step_id: entry.step_id,
          step_name: "generate_object",
          input: supplementalWebSelectorInput(
            plan,
            entry.candidates,
            roundCoverageGaps,
            entry.selection_limit,
            operationalGapCount,
            initialAttempts,
            gapQueries.queries,
            { allow_empty: true }
          ),
          retry: settings.semantic_web_selection_retry,
        }))
        .filter((step) =>
          !outputs[step.step_id] && !failures[step.step_id]
        );
      if (pendingShardSteps.length > 0) {
        return {
          schedule: { type: "schedule_steps", steps: pendingShardSteps },
          selection: null,
          coverage_gaps: coverageGaps,
          attempted: true,
        };
      }
    }
    const supplementalWebShardUnion = supplementalWebShardEntries.length > 0
      ? sourceSelectionShardUnion(
          supplementalWebShardEntries,
          outputs,
          failures,
          (entry) => boundedSupplementalDiscoveryFallback(
            entry.candidates,
            entry.selection_limit
          )
        )
      : null;
    const supplementalReductionCandidates = supplementalWebShardUnion
      ? supplementalWebShardUnion.candidates
      : remainingCandidates;
    const needsSupplementalSourceReduction = Boolean(
      supplementalWebShardUnion &&
      supplementalReductionCandidates.length > fetchLimit
    );
    if (
      needsSourceSelection &&
      (supplementalWebShardEntries.length === 0 ||
        needsSupplementalSourceReduction) &&
      !outputs[stepIds.select_web] &&
      !failures[stepIds.select_web]
    ) {
      return {
        schedule: {
          type: "schedule_step",
          step_id: stepIds.select_web,
          step_name: "generate_object",
          input: needsSupplementalSourceReduction
            ? supplementalWebSelectorInput(
                plan,
                supplementalReductionCandidates,
                roundCoverageGaps,
                fetchLimit,
                operationalGapCount,
                initialAttempts,
                gapQueries.queries
              )
            : directSupplementalSelectorInput,
          retry: settings.semantic_web_selection_retry,
        },
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: true,
      };
    }
    const sourceSelectorFailure = failures[stepIds.select_web] &&
      (failures[stepIds.select_web].error ||
        "supplemental source selection failed");
    let sourceSelection;
    if (supplementalWebShardUnion) {
      if (needsSupplementalSourceReduction) {
        sourceSelection = selectedSupplementalWebCandidates(
          supplementalReductionCandidates,
          structuredOutput(outputs[stepIds.select_web]),
          fetchLimit,
          sourceSelectorFailure
        );
        if (
          supplementalWebShardUnion.fallback_count > 0 &&
          sourceSelection.mode === "semantic_supplemental_candidate_ids"
        ) {
          sourceSelection.mode = "bounded_supplemental_discovery_fallback";
        }
      } else {
        sourceSelection = {
          candidates: supplementalReductionCandidates,
          mode: supplementalWebShardUnion.fallback_count > 0
            ? "bounded_supplemental_discovery_fallback"
            : "semantic_supplemental_candidate_shards",
          error: uniqueStrings(supplementalWebShardUnion.errors).join(" "),
        };
      }
    } else {
      sourceSelection = selectedSupplementalWebCandidates(
        remainingCandidates,
        structuredOutput(outputs[stepIds.select_web]),
        fetchLimit,
        sourceSelectorFailure
      );
    }
    const supplementalDiscoveryMetadata = {
      coverage_gap_count: coverageGaps.length,
      operational_gap_count: operationalGapCount,
      generated_gap_query_count: gapQueries.queries.length,
      gap_search_budget: queryBudget,
      gap_discovery_candidate_count: Array.isArray(gapDiscovery.candidates)
        ? gapDiscovery.candidates.length
        : 0,
      failed_candidate_count: initialAttempts.filter((attempt) =>
        attempt.outcome === "fetch_failed"
      ).length,
      supplemental_fetch_limit: fetchLimit,
      supplemental_fetch_budget: fetchBudget,
      supplemental_fetch_count: sourceSelection.candidates.length,
    };
    const supplementalWebSteps = webSourceFetchSteps(
      stepIds.web_source_prefix,
      plan,
      sourceSelection.candidates,
      "supplemental-web-source",
      20,
      settings.retrieval_retry,
      sourceSelection.mode === "bounded_supplemental_discovery_fallback"
    );
    const pendingSupplementalWebSteps = supplementalWebSteps.filter((step) =>
      !outputs[step.step_id] && !failures[step.step_id]
    );
    if (pendingSupplementalWebSteps.length > 0) {
      return {
        schedule: {
          type: "schedule_steps",
          steps: pendingSupplementalWebSteps,
        },
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: true,
        attempted_candidates: sourceSelection.candidates,
        fetch_count: sourceSelection.candidates.length,
        query_count: gapQueries.queries.length,
        queries: gapQueries.queries,
      };
    }
    const retrieval = webRetrievalFromSourceSteps({
      step_id_prefix: stepIds.web_source_prefix,
      plan,
      candidates: sourceSelection.candidates,
      catalog_source_prefix: round === 1
        ? "supplemental-catalog-source"
        : `supplemental-${round}-catalog-source`,
      outputs,
      failures,
      discovery_errors: uniqueStrings([
        gapQueries.error || "",
        gapQueryFailure || "",
        gapDiscoveryFailure || "",
        ...(Array.isArray(gapDiscovery.errors) ? gapDiscovery.errors : []),
        ...(supplementalWebShardUnion
          ? supplementalWebShardUnion.errors
          : []),
        sourceSelectorFailure || "",
        sourceSelection.error || "",
      ]),
      discovery_metadata: supplementalDiscoveryMetadata,
      source_selection_mode: sourceSelection.mode,
    });
    const supplementalPacket = packetForCoverageGaps(
      retrieval.packet,
      roundCoverageGaps
    );
    if (!supplementalPacket) {
      return {
        schedule: null,
        selection: {
          status: "failed",
          results: [],
          errors: uniqueStrings([
            ...(Array.isArray(retrieval.errors) ? retrieval.errors : []),
            "The supplemental pass retained no closed evidence packet for its typed coverage gaps.",
          ]),
          metadata: Object.assign({}, object(retrieval.metadata), {
            coverage_gap_count: coverageGaps.length,
            operational_gap_count: operationalGapCount,
            source_count: 0,
            selection_count: 0,
          }),
        },
        coverage_gaps: coverageGaps,
        attempted: true,
        attempted_candidates: sourceSelection.candidates,
        fetch_count: sourceSelection.candidates.length,
        query_count: gapQueries.queries.length,
        queries: gapQueries.queries,
      };
    }

    const chunkCount = supplementalPacket.sources.reduce(
      (total, source) => total + source.chunks.length,
      0
    );
    const selectorShards = selectorShardPackets(supplementalPacket);
    const usesSelectorShards = chunkCount > MAX_DIRECT_SELECTOR_CHUNKS;
    if (usesSelectorShards) {
      const pendingShardSteps = selectorShards
        .map((shard, index) => ({
          step_id: `${stepIds.select_shard_prefix}${index + 1}`,
          step_name: "generate_object",
          input: selectorInput(shard, { shard: true }),
          retry: settings.semantic_shard_selection_retry,
        }))
        .filter((step) =>
          !outputs[step.step_id] && !failures[step.step_id]
        );
      if (pendingShardSteps.length > 0) {
        return {
          schedule: {
            type: "schedule_steps",
            steps: pendingShardSteps,
          },
          selection: null,
          coverage_gaps: coverageGaps,
          attempted: true,
        };
      }
    }
    const selectorShardRecoveries = usesSelectorShards
      ? selectorShardRecoveryEntries(
          selectorShards,
          failures,
          stepIds.select_shard_prefix,
          stepIds.select_shard_recovery_prefix
        )
      : [];
    if (selectorShardRecoveries.length > 0) {
      const pendingRecoverySteps = selectorShardRecoveries
        .map((entry) => ({
          step_id: entry.step_id,
          step_name: "generate_object",
          input: selectorInput(entry.packet, { shard: true }),
          retry: settings.semantic_shard_selection_retry,
        }))
        .filter((step) =>
          !outputs[step.step_id] && !failures[step.step_id]
        );
      if (pendingRecoverySteps.length > 0) {
        return {
          schedule: {
            type: "schedule_steps",
            steps: pendingRecoverySteps,
          },
          selection: null,
          coverage_gaps: coverageGaps,
          attempted: true,
        };
      }
    }
    const selectorReductionEntries = usesSelectorShards
      ? selectorShardReductionEntries(
          selectorShards,
          failures,
          stepIds.select_shard_prefix,
          selectorShardRecoveries
        )
      : [];
    const shardReduction = usesSelectorShards
      ? reducedSelectorPacket(
          supplementalPacket,
          selectorReductionEntries,
          outputs,
          failures
        )
      : {
          packet: supplementalPacket,
          candidate_count: chunkCount,
          source_coverage: [],
          error: "",
        };
    const sourceReductionPackets = usesSelectorShards
      ? selectorSourceReductionPackets(shardReduction.packet)
      : [];
    if (usesSelectorShards && shardReduction.packet) {
      const pendingSourceSteps = sourceReductionPackets
        .map((reduction, index) => ({
          step_id: `${stepIds.select_source_prefix}${index + 1}`,
          step_name: "generate_object",
          input: selectorInput(reduction.packet, {
            source_reduction: true,
          }),
          retry: settings.semantic_selection_retry,
        }))
        .filter((step) =>
          !outputs[step.step_id] && !failures[step.step_id]
        );
      if (pendingSourceSteps.length > 0) {
        return {
          schedule: {
            type: "schedule_steps",
            steps: pendingSourceSteps,
          },
          selection: null,
          coverage_gaps: coverageGaps,
          attempted: true,
        };
      }
    }
    const sourceReduction = usesSelectorShards
      ? reducedSourcePacket(
          shardReduction.packet,
          sourceReductionPackets,
          outputs,
          failures,
          shardReduction.source_coverage,
          shardReduction.source_relevance,
          stepIds.select_source_prefix
        )
      : shardReduction;
    if (
      !usesSelectorShards &&
      sourceReduction.packet &&
      !outputs[stepIds.select_chunks] &&
      !failures[stepIds.select_chunks]
    ) {
      return {
        schedule: {
          type: "schedule_step",
          step_id: stepIds.select_chunks,
          step_name: "generate_object",
          input: selectorInput(sourceReduction.packet, { shard: false }),
          retry: settings.semantic_selection_retry,
        },
        selection: null,
        coverage_gaps: coverageGaps,
        attempted: true,
      };
    }
    const selectorFailure = !usesSelectorShards &&
      failures[stepIds.select_chunks] &&
      (failures[stepIds.select_chunks].error ||
        "supplemental semantic chunk selection failed");
    const supplementalSemanticSelection =
      usesSelectorShards && sourceReduction.packet
        ? {
            chunk_ids: sourceReduction.packet.sources.flatMap((source) =>
              source.chunks.map((chunk) => chunk.chunk_id)
            ),
            source_coverage: sourceReduction.source_coverage,
            source_relevance: sourceReduction.source_relevance,
          }
        : structuredOutput(outputs[stepIds.select_chunks]);
    const errors = uniqueStrings([
      ...(Array.isArray(retrieval.errors) ? retrieval.errors : []),
      shardReduction.error || "",
      sourceReduction.error || "",
      selectorFailure || "",
    ]);
    return {
      schedule: null,
      selection: materializeEvidence(
        supplementalPacket,
        supplementalSemanticSelection,
        errors,
        {
          retrieval_pass: round + 1,
          coverage_gap_count: coverageGaps.length,
          operational_gap_count: operationalGapCount,
          supplemental_fetch_budget: fetchBudget,
          supplemental_fetch_count: sourceSelection.candidates.length,
          gap_search_budget: queryBudget,
          generated_gap_query_count: gapQueries.queries.length,
          catalog_source_count: supplementalPacket.sources.length,
          catalog_chunk_count: chunkCount,
          semantic_selection_shard_count: usesSelectorShards
            ? selectorShards.length
            : 1,
          semantic_selection_recovery_shard_count:
            selectorShardRecoveries.length,
          semantic_selection_candidate_count: shardReduction.candidate_count,
          semantic_selection_failed_shard_count:
            shardReduction.failed_shard_count || 0,
          semantic_selection_source_reduction_count:
            sourceReductionPackets.length,
          semantic_selection_failed_source_reduction_count:
            sourceReduction.failed_source_reduction_count || 0,
          semantic_selection_materialized_count:
            sourceReduction.candidate_count,
          web: object(retrieval.metadata),
        }
      ),
      coverage_gaps: coverageGaps,
      attempted: true,
      attempted_candidates: sourceSelection.candidates,
      fetch_count: sourceSelection.candidates.length,
      query_count: gapQueries.queries.length,
      queries: gapQueries.queries,
    };
  };
