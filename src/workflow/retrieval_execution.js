  const generationStepLabel = (stepId) => {
    const id = String(stepId || "");
    if (id === STEP_SELECT_WEB || id.startsWith(STEP_SELECT_WEB_SHARD_PREFIX)) {
      return "Semantic web source selection";
    }
    if (
      id === STEP_SELECT_SUPPLEMENTAL_WEB ||
      id.startsWith(`${STEP_SELECT_SUPPLEMENTAL_WEB}_`)
    ) {
      return "Semantic supplemental web source selection";
    }
    if (
      id === STEP_GENERATE_GAP_QUERIES ||
      id.startsWith(`${STEP_GENERATE_GAP_QUERIES}_`)
    ) {
      return "Typed gap-query generation";
    }
    if (
      id === STEP_ATTRIBUTE_SOURCES ||
      id.startsWith(STEP_ATTRIBUTE_SOURCES_ROUND_PREFIX)
    ) {
      return "Closed source-attribution review";
    }
    return "Semantic chunk selection";
  };

  if (inputs.kind === "step") {
    if (inputs.step_name === STEP_DISCOVER_WEB) {
      return await discoverWeb(object(inputs.input));
    }
    if (inputs.step_name === STEP_WEB_SOURCE) {
      return await collectWeb(object(inputs.input));
    }
    if (inputs.step_name === STEP_LOCAL) {
      return await collectLocal(object(inputs.input));
    }
    if (inputs.step_name === STEP_CHECKPOINT_BOOTSTRAP) return object(inputs.input);
    if (inputs.step_name === STEP_CHECKPOINT_INITIAL) return object(inputs.input);
    if (inputs.step_name === "generate_object") {
      const result = await ctx.tool("generate_object", object(inputs.input));
      const exitCode = toolExitCode(result);
      if (exitCode !== 0) {
        const diagnostic = bounded(result && result.output, 600) ||
          "generate_object returned no diagnostic";
        const stage = generationStepLabel(inputs.step_id);
        throw new Error(
          `${stage} failed with exit code ${exitCode}: ${diagnostic}`
        );
      }
      return result;
    }
    return { error: `unknown retrieval step: ${String(inputs.step_name || "")}` };
  }
  if (inputs.kind !== "workflow") {
    return { error: `unknown DeepResearch retrieval invocation: ${String(inputs.kind || "")}` };
  }

  const input = object(inputs.input);
  const plan = object(input.research_plan);
  const query = String(input.query || "");
  const executionMode = String(input.execution_mode || "collect_only");
  const scope = input.evidence_scope === "local_only"
    ? "local_only"
    : "web_and_workspace";
  const needsWeb = scope === "web_and_workspace";
  const needsLocal = scope === "local_only" ||
    plan.workspace_evidence_required === true;
  const supplementalFetchBudget = needsWeb
    ? clamp(
        object(object(input.loop_contract).hard_caps).max_supplemental_fetches,
        0,
        MAX_CATALOG_SOURCES,
        0
      )
    : 0;
  const gapRoundCount = needsWeb
    ? clamp(
        object(object(input.loop_contract).cardinality).gap_extractions,
        0,
        MAX_GAP_ROUNDS,
        0
      )
    : 0;
  const gapSearchBudget = needsWeb
    ? clamp(
        object(object(input.loop_contract).hard_caps).max_gap_searches,
        0,
        MAX_SEARCH_QUERIES * gapRoundCount,
        0
      )
    : 0;
  const outputs = object(inputs.step_outputs);
  const failures = object(inputs.step_failures);
  const retrievalRetry = {
    max_attempts: 1,
    delay_ms: 0,
    on_exhausted: "continue_workflow",
  };
  const semanticSelectionRetry = {
    max_attempts: 2,
    delay_ms: 100,
    on_exhausted: "continue_workflow",
  };
  const webSourceSelectionRetry = {
    max_attempts: 1,
    delay_ms: 0,
    on_exhausted: "continue_workflow",
  };
  const gapQueryGenerationRetry = {
    max_attempts: 1,
    delay_ms: 0,
    on_exhausted: "continue_workflow",
  };
  const semanticShardSelectionRetry = {
    max_attempts: 1,
    delay_ms: 0,
    on_exhausted: "continue_workflow",
  };
  const sourceAttributionRetry = {
    max_attempts: 1,
    delay_ms: 0,
    on_exhausted: "continue_workflow",
  };

  if (!nonEmpty(query) || Object.keys(plan).length === 0) {
    return {
      type: "fail",
      error: "host-managed DeepResearch retrieval requires a query and validated research plan",
    };
  }
  if (executionMode === "bootstrap_acquisition") {
    if (
      needsWeb &&
      !outputs[STEP_DISCOVER_WEB] &&
      !failures[STEP_DISCOVER_WEB]
    ) {
      return {
        type: "schedule_step",
        step_id: STEP_DISCOVER_WEB,
        step_name: STEP_DISCOVER_WEB,
        input: {
          query,
          plan,
          search_timeout_secs: 12,
        },
        retry: retrievalRetry,
      };
    }
    const bootstrapDiscovery = needsWeb
      ? (outputs[STEP_DISCOVER_WEB] || {
          status: "failed",
          candidates: [],
          errors: [
            failures[STEP_DISCOVER_WEB] && failures[STEP_DISCOVER_WEB].error ||
              "bootstrap web discovery failed",
          ],
          metadata: {},
        })
      : null;
    const bootstrapSelection = needsWeb
      ? boundedDiscoveryFallback(plan, bootstrapDiscovery, "")
      : { candidates: [], mode: "none", error: "" };
    const bootstrapWebSteps = webSourceFetchSteps(
      STEP_WEB_SOURCE_PREFIX,
      plan,
      bootstrapSelection.candidates,
      "bootstrap-web-source",
      20,
      retrievalRetry,
      true
    );
    const pendingBootstrapWebSteps = bootstrapWebSteps.filter((step) =>
      !outputs[step.step_id] && !failures[step.step_id]
    );
    if (needsWeb && pendingBootstrapWebSteps.length > 0) {
      return {
        type: "schedule_steps",
        steps: pendingBootstrapWebSteps,
      };
    }
    if (needsLocal && !outputs[STEP_LOCAL] && !failures[STEP_LOCAL]) {
      return {
        type: "schedule_step",
        step_id: STEP_LOCAL,
        step_name: STEP_LOCAL,
        input: {
          query,
          plan,
          max_steps: input.local_max_steps,
          source_hints: input.workspace_source_hints,
        },
        retry: retrievalRetry,
      };
    }
    const bootstrapWeb = needsWeb
      ? webRetrievalFromSourceSteps({
          step_id_prefix: STEP_WEB_SOURCE_PREFIX,
          plan,
          candidates: bootstrapSelection.candidates,
          outputs,
          failures,
          discovery_errors: uniqueStrings([
            ...(Array.isArray(bootstrapDiscovery.errors)
              ? bootstrapDiscovery.errors
              : []),
            bootstrapSelection.error || "",
          ]),
          discovery_metadata: object(bootstrapDiscovery.metadata),
          source_selection_mode: bootstrapSelection.mode,
        })
      : null;
    const bootstrapLocal = needsLocal
      ? (outputs[STEP_LOCAL] || {
          status: "failed",
          packet: null,
          errors: [
            failures[STEP_LOCAL] && failures[STEP_LOCAL].error ||
              "bootstrap local retrieval did not complete",
          ],
          metadata: {},
        })
      : null;
    const bootstrapAdmission = combinedEvidencePacket(
      plan,
      [bootstrapWeb, bootstrapLocal]
    );
    const bootstrapErrors = uniqueStrings([
      ...(Array.isArray(bootstrapWeb && bootstrapWeb.errors)
        ? bootstrapWeb.errors
        : []),
      ...(Array.isArray(bootstrapLocal && bootstrapLocal.errors)
        ? bootstrapLocal.errors
        : []),
      bootstrapAdmission.error || "",
    ]);
    const bootstrapOutput = {
      query,
      mode: "bootstrap_acquisition",
      acquisition: {
        status: bootstrapAdmission.packet
          ? (bootstrapErrors.length > 0 ? "partial" : "success")
          : "failed",
        packet: bootstrapAdmission.packet,
        errors: bootstrapErrors,
        metadata: {
          source_selection_mode: bootstrapSelection.mode,
          source_count: bootstrapAdmission.source_count,
          chunk_count: bootstrapAdmission.chunk_count,
          web: bootstrapWeb ? object(bootstrapWeb.metadata) : undefined,
          local: bootstrapLocal ? object(bootstrapLocal.metadata) : undefined,
        },
      },
      execution: {
        mode: "acquire_only",
        terminal_authority: "host_inquiry_reducer",
        note: "Raw sources were durably acquired before semantic planning settled.",
      },
    };
    if (
      !outputs[STEP_CHECKPOINT_BOOTSTRAP] &&
      !failures[STEP_CHECKPOINT_BOOTSTRAP]
    ) {
      return {
        type: "schedule_step",
        step_id: STEP_CHECKPOINT_BOOTSTRAP,
        step_name: STEP_CHECKPOINT_BOOTSTRAP,
        input: bootstrapOutput,
        retry: retrievalRetry,
      };
    }
    return {
      type: "complete",
      output: outputs[STEP_CHECKPOINT_BOOTSTRAP] || bootstrapOutput,
    };
  }
  const bootstrapAcquisition = object(input.bootstrap_acquisition);
  const bootstrapPacket = object(bootstrapAcquisition.packet);
  const hasBootstrapPacket =
    Array.isArray(bootstrapPacket.sources) &&
    bootstrapPacket.sources.length > 0;
  const hasBootstrapWeb = needsWeb && hasBootstrapPacket;
  const plannedQueryCount = Array.isArray(plan.search_queries)
    ? plan.search_queries.length
    : 0;
  const plannedSeedCount = Array.isArray(plan.seed_urls)
    ? plan.seed_urls.length
    : 0;
  const skippedBootstrapQueryCount = hasBootstrapWeb && plannedQueryCount > 0
    ? 1
    : 0;
  const hasPlannedWebDiscovery = needsWeb &&
    (
      plannedQueryCount > skippedBootstrapQueryCount ||
      plannedSeedCount > 0
    );
  if (
    hasPlannedWebDiscovery &&
    !outputs[STEP_DISCOVER_WEB] &&
    !failures[STEP_DISCOVER_WEB]
  ) {
    return {
      type: "schedule_step",
      step_id: STEP_DISCOVER_WEB,
      step_name: STEP_DISCOVER_WEB,
      input: {
        query,
        plan,
        skip_query_count: skippedBootstrapQueryCount,
        search_timeout_secs: 12,
      },
      retry: retrievalRetry,
    };
  }
  const webDiscovery = hasPlannedWebDiscovery
    ? (outputs[STEP_DISCOVER_WEB] || {
        status: "failed",
        candidates: [],
        errors: [
          failures[STEP_DISCOVER_WEB] && failures[STEP_DISCOVER_WEB].error ||
            "web discovery failed",
        ],
        metadata: {},
      })
    : null;
  const discoveryCandidatesList = Array.isArray(
    webDiscovery && webDiscovery.candidates
  )
    ? webDiscovery.candidates
    : [];
  const fetchLimit = clamp(
    object(plan.budget).direct_fetches,
    0,
    MAX_SOURCES,
    4
  );
  const needsWebSourceSelection =
    hasPlannedWebDiscovery && fetchLimit > 0 &&
    discoveryCandidatesList.length > 0;
  const directWebSelectorInput = needsWebSourceSelection
    ? webSourceSelectorInput(plan, webDiscovery)
    : null;
  const webSourceShardEntries = needsWebSourceSelection
    ? sourceSelectionShardEntries(
        discoveryCandidatesList,
        fetchLimit,
        STEP_SELECT_WEB_SHARD_PREFIX
      )
    : [];
  if (webSourceShardEntries.length > 0) {
    const pendingShardSteps = webSourceShardEntries
      .map((entry) => ({
        step_id: entry.step_id,
        step_name: "generate_object",
        input: webSourceSelectorInput(
          plan,
          { candidates: entry.candidates },
          { fetch_limit: entry.selection_limit }
        ),
        retry: webSourceSelectionRetry,
      }))
      .filter((step) =>
        !outputs[step.step_id] && !failures[step.step_id]
      );
    if (pendingShardSteps.length > 0) {
      return { type: "schedule_steps", steps: pendingShardSteps };
    }
  }
  const webSourceShardUnion = webSourceShardEntries.length > 0
    ? sourceSelectionShardUnion(
        webSourceShardEntries,
        outputs,
        failures,
        (entry, error) => boundedDiscoveryFallback(
          Object.assign({}, plan, {
            budget: Object.assign({}, object(plan.budget), {
              direct_fetches: entry.selection_limit,
            }),
          }),
          { candidates: entry.candidates },
          error,
          entry.selection_limit
        )
      )
    : null;
  const webReductionDiscovery = webSourceShardUnion
    ? { candidates: webSourceShardUnion.candidates }
    : webDiscovery;
  const needsWebSourceReduction = Boolean(
    webSourceShardUnion && webSourceShardUnion.candidates.length > fetchLimit
  );
  if (
    needsWebSourceSelection &&
    (webSourceShardEntries.length === 0 || needsWebSourceReduction) &&
    !outputs[STEP_SELECT_WEB] &&
    !failures[STEP_SELECT_WEB]
  ) {
    return {
      type: "schedule_step",
      step_id: STEP_SELECT_WEB,
      step_name: "generate_object",
      input: needsWebSourceReduction
        ? webSourceSelectorInput(plan, webReductionDiscovery)
        : directWebSelectorInput,
      retry: webSourceSelectionRetry,
    };
  }
  const webSourceSelectorFailure = failures[STEP_SELECT_WEB] &&
    (failures[STEP_SELECT_WEB].error ||
      "semantic web source selection failed");
  let webSourceSelection = { candidates: [], mode: "none", error: "" };
  if (hasPlannedWebDiscovery && webSourceShardUnion) {
    if (needsWebSourceReduction) {
      const semanticReduction = webSourceSelectorFailure
        ? { candidates: [], error: webSourceSelectorFailure }
        : closedCandidateSelection(
            webSourceShardUnion.candidates,
            structuredOutput(outputs[STEP_SELECT_WEB]),
            fetchLimit
          );
      if (semanticReduction.error) {
        webSourceSelection = boundedDiscoveryFallback(
          plan,
          webReductionDiscovery,
          semanticReduction.error,
          fetchLimit
        );
      } else {
        webSourceSelection = {
          candidates: semanticReduction.candidates,
          mode: webSourceShardUnion.fallback_count > 0
            ? "bounded_discovery_fallback"
            : "semantic_candidate_shards",
          error: uniqueStrings(webSourceShardUnion.errors).join(" "),
        };
      }
    } else {
      webSourceSelection = {
        candidates: webSourceShardUnion.candidates,
        mode: webSourceShardUnion.fallback_count > 0
          ? "bounded_discovery_fallback"
          : "semantic_candidate_shards",
        error: uniqueStrings(webSourceShardUnion.errors).join(" "),
      };
    }
  } else if (hasPlannedWebDiscovery) {
    webSourceSelection = selectedWebCandidates(
      plan,
      webDiscovery,
      structuredOutput(outputs[STEP_SELECT_WEB]),
      webSourceSelectorFailure
    );
  }
  const plannedWebSteps = webSourceFetchSteps(
    STEP_WEB_SOURCE_PREFIX,
    plan,
    webSourceSelection.candidates,
    "web-source",
    20,
    retrievalRetry,
    webSourceSelection.mode === "bounded_discovery_fallback"
  );
  const pendingPlannedWebSteps = plannedWebSteps.filter((step) =>
    !outputs[step.step_id] && !failures[step.step_id]
  );
  if (hasPlannedWebDiscovery && pendingPlannedWebSteps.length > 0) {
    return {
      type: "schedule_steps",
      steps: pendingPlannedWebSteps,
    };
  }
  if (needsLocal && !outputs[STEP_LOCAL] && !failures[STEP_LOCAL]) {
    return {
      type: "schedule_step",
      step_id: STEP_LOCAL,
      step_name: STEP_LOCAL,
      input: {
        query,
        plan,
        max_steps: input.local_max_steps,
        source_hints: input.workspace_source_hints,
      },
      retry: retrievalRetry,
    };
  }

  const bootstrapRetrieval = hasBootstrapPacket
    ? {
        status: String(bootstrapAcquisition.status || "partial"),
        packet: bootstrapPacket,
        errors: Array.isArray(bootstrapAcquisition.errors)
          ? bootstrapAcquisition.errors
          : [],
        metadata: Object.assign({}, object(bootstrapAcquisition.metadata), {
          source_selection_mode: "bootstrap_packet",
          bootstrap_source_count: bootstrapPacket.sources.length,
        }),
      }
    : null;
  const webRetrieval = hasPlannedWebDiscovery
    ? webRetrievalFromSourceSteps({
        step_id_prefix: STEP_WEB_SOURCE_PREFIX,
        plan,
        candidates: webSourceSelection.candidates,
        outputs,
        failures,
        discovery_errors: uniqueStrings([
          ...(Array.isArray(webDiscovery.errors) ? webDiscovery.errors : []),
          webSourceSelectorFailure || "",
          webSourceSelection.error || "",
        ]),
        discovery_metadata: object(webDiscovery.metadata),
        source_selection_mode: webSourceSelection.mode,
      })
    : null;
  const localRetrieval = needsLocal
    ? (outputs[STEP_LOCAL] || {
        status: "failed",
        packet: null,
        errors: [
          failures[STEP_LOCAL] && failures[STEP_LOCAL].error ||
            "local retrieval failed",
        ],
        metadata: {},
      })
    : null;
  const admission = combinedEvidencePacket(
    plan,
    [bootstrapRetrieval, webRetrieval, localRetrieval]
  );
  const packet = admission.packet;
  const selectorShards = packet ? selectorShardPackets(packet) : [];
  const usesSelectorShards =
    packet && admission.chunk_count > MAX_DIRECT_SELECTOR_CHUNKS;
  if (usesSelectorShards) {
    const pendingShardSteps = selectorShards
      .map((shard, index) => ({
        step_id: `${STEP_SELECT_SHARD_PREFIX}${index + 1}`,
        step_name: "generate_object",
        input: selectorInput(shard, { shard: true }),
        retry: semanticShardSelectionRetry,
      }))
      .filter((step) =>
        !outputs[step.step_id] && !failures[step.step_id]
      );
    if (pendingShardSteps.length > 0) {
      return {
        type: "schedule_steps",
        steps: pendingShardSteps,
      };
    }
  }
  const selectorShardRecoveries = usesSelectorShards
    ? selectorShardRecoveryEntries(
        selectorShards,
        failures,
        STEP_SELECT_SHARD_PREFIX,
        STEP_SELECT_SHARD_RECOVERY_PREFIX
      )
    : [];
  if (selectorShardRecoveries.length > 0) {
    const pendingRecoverySteps = selectorShardRecoveries
      .map((entry) => ({
        step_id: entry.step_id,
        step_name: "generate_object",
        input: selectorInput(entry.packet, { shard: true }),
        retry: semanticShardSelectionRetry,
      }))
      .filter((step) =>
        !outputs[step.step_id] && !failures[step.step_id]
      );
    if (pendingRecoverySteps.length > 0) {
      return {
        type: "schedule_steps",
        steps: pendingRecoverySteps,
      };
    }
  }
  const selectorReductionEntries = usesSelectorShards
    ? selectorShardReductionEntries(
        selectorShards,
        failures,
        STEP_SELECT_SHARD_PREFIX,
        selectorShardRecoveries
      )
    : [];
  const shardReduction = usesSelectorShards
    ? reducedSelectorPacket(packet, selectorReductionEntries, outputs, failures)
    : {
        packet,
        candidate_count: admission.chunk_count,
        error: "",
      };
  const sourceReductionPackets = usesSelectorShards
    ? selectorSourceReductionPackets(shardReduction.packet)
    : [];
  if (usesSelectorShards && shardReduction.packet) {
    const pendingSourceSteps = sourceReductionPackets
      .map((reduction, index) => ({
        step_id: `${STEP_SELECT_SOURCE_PREFIX}${index + 1}`,
        step_name: "generate_object",
        input: selectorInput(reduction.packet, {
          source_reduction: true,
        }),
        retry: semanticSelectionRetry,
      }))
      .filter((step) =>
        !outputs[step.step_id] && !failures[step.step_id]
      );
    if (pendingSourceSteps.length > 0) {
      return {
        type: "schedule_steps",
        steps: pendingSourceSteps,
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
        STEP_SELECT_SOURCE_PREFIX
      )
    : shardReduction;
  if (
    !usesSelectorShards &&
    sourceReduction.packet &&
    !outputs[STEP_SELECT] &&
    !failures[STEP_SELECT]
  ) {
    return {
      type: "schedule_step",
      step_id: STEP_SELECT,
      step_name: "generate_object",
      input: selectorInput(sourceReduction.packet, { shard: false }),
      retry: semanticSelectionRetry,
    };
  }
  const selectorFailure = !usesSelectorShards && failures[STEP_SELECT] &&
    (failures[STEP_SELECT].error || "semantic chunk selection failed");
  const retrievalErrors = uniqueStrings([
    ...(Array.isArray(bootstrapRetrieval && bootstrapRetrieval.errors)
      ? bootstrapRetrieval.errors
      : []),
    ...(Array.isArray(webRetrieval && webRetrieval.errors)
      ? webRetrieval.errors
      : []),
    ...(Array.isArray(localRetrieval && localRetrieval.errors)
      ? localRetrieval.errors
      : []),
    admission.error || "",
    shardReduction.error || "",
    sourceReduction.error || "",
    selectorFailure || "",
  ]);
  const semanticSelection = usesSelectorShards && sourceReduction.packet
      ? {
        chunk_ids: sourceReduction.packet.sources.flatMap((source) =>
          source.chunks.map((chunk) => chunk.chunk_id)
        ),
        source_coverage: sourceReduction.source_coverage,
        source_relevance: sourceReduction.source_relevance,
      }
    : structuredOutput(outputs[STEP_SELECT]);
  const primarySelection = materializeEvidence(
    packet,
    semanticSelection,
    retrievalErrors,
    {
      catalog_source_count: admission.source_count,
      catalog_chunk_count: admission.chunk_count,
      semantic_selection_shard_count: usesSelectorShards
        ? selectorShards.length
        : 1,
      semantic_selection_recovery_shard_count:
        selectorShardRecoveries.length,
      semantic_selection_candidate_count: shardReduction.candidate_count,
      semantic_selection_failed_shard_count:
        shardReduction.failed_shard_count || 0,
      semantic_selection_source_reduction_count: sourceReductionPackets.length,
      semantic_selection_failed_source_reduction_count:
        sourceReduction.failed_source_reduction_count || 0,
      semantic_selection_materialized_count: sourceReduction.candidate_count,
      bootstrap_source_count: hasBootstrapPacket
        ? bootstrapPacket.sources.length
        : 0,
      bootstrap: bootstrapRetrieval
        ? object(bootstrapRetrieval.metadata)
        : undefined,
      web: webRetrieval ? object(webRetrieval.metadata) : undefined,
      local: localRetrieval ? object(localRetrieval.metadata) : undefined,
    }
  );
  primarySelection.metadata = Object.assign(
    {},
    object(primarySelection.metadata),
    {
      retrieval_pass_count: 1,
      typed_coverage_gap_count: typedCoverageGaps(
        plan,
        materializedSourceCoverage(primarySelection)
      ).length,
    }
  );
  const initialCheckpointOutput = initialRetrievalCheckpointOutput(
    query,
    plan,
    primarySelection
  );
  if (
    !outputs[STEP_CHECKPOINT_INITIAL] &&
    !failures[STEP_CHECKPOINT_INITIAL]
  ) {
    return {
      type: "schedule_step",
      step_id: STEP_CHECKPOINT_INITIAL,
      step_name: STEP_CHECKPOINT_INITIAL,
      input: initialCheckpointOutput,
      retry: retrievalRetry,
    };
  }
  let selection = primarySelection;
  let roundPlan = plan;
  let supplementalAttempted = false;
  let supplementalRoundCount = 0;
  let supplementalFetchCount = 0;
  let generatedGapQueryCount = 0;
  let reviewedAttributionIdentity = "";
  let reviewedAttributionContract = null;
  let reviewedAttributionError = "";
  const initialMaterializedSourceCount = materializedSourceCount(selection);
  const excludedCandidates = [
    ...(Array.isArray(webSourceSelection.candidates)
      ? webSourceSelection.candidates
      : []),
  ];
  for (let round = 1; round <= gapRoundCount; round += 1) {
    const coverageBindings = materializedSourceCoverage(selection);
    let coverageGaps = typedCoverageGaps(plan, coverageBindings);
    if (
      coverageGaps.length === 0 &&
      planNeedsIndependentAttribution(plan)
    ) {
      const attributionPacket = materializedAttributionPacket(selection);
      const attributionIdentity = JSON.stringify(attributionPacket);
      if (attributionIdentity !== reviewedAttributionIdentity) {
        const attributionStepId =
          `${STEP_ATTRIBUTE_SOURCES_ROUND_PREFIX}${round}`;
        const attributionReview = sourceAttributionReview(
          selection,
          attributionStepId,
          outputs,
          failures,
          sourceAttributionRetry
        );
        if (attributionReview.schedule) {
          return attributionReview.schedule;
        }
        reviewedAttributionIdentity = attributionIdentity;
        reviewedAttributionContract = attributionReview.contract;
        reviewedAttributionError = attributionReview.error;
      }
      if (reviewedAttributionContract) {
        coverageGaps = typedCoverageGaps(
          plan,
          coverageBindings,
          reviewedAttributionContract
        );
      }
    }
    if (coverageGaps.length === 0) {
      break;
    }
    const remainingRounds = gapRoundCount - round + 1;
    const remainingFetchBudget = Math.max(
      0,
      supplementalFetchBudget - supplementalFetchCount
    );
    const remainingQueryBudget = Math.max(
      0,
      gapSearchBudget - generatedGapQueryCount
    );
    const remainingSourceSlots = Math.max(
      0,
      MAX_CATALOG_SOURCES - materializedSourceCount(selection)
    );
    if (remainingFetchBudget === 0 || remainingSourceSlots === 0) {
      break;
    }
    const roundFetchBudget = Math.min(
      MAX_SOURCES,
      remainingSourceSlots,
      Math.ceil(remainingFetchBudget / remainingRounds)
    );
    const roundQueryBudget = Math.min(
      MAX_SEARCH_QUERIES,
      Math.ceil(remainingQueryBudget / remainingRounds)
    );
    const supplementalRound = supplementalCoverageRound({
      round,
      enabled: true,
      fetch_budget: roundFetchBudget,
      query_budget: roundQueryBudget,
      query,
      plan: roundPlan,
      needs_web: needsWeb,
      web_discovery: round === 1 ? webDiscovery : { candidates: [] },
      initial_candidates: round === 1
        ? webSourceSelection.candidates
        : [],
      excluded_candidates: excludedCandidates,
      packet: round === 1 ? packet : null,
      semantic_selection: round === 1 ? semanticSelection : null,
      coverage_bindings: coverageBindings,
      coverage_gaps: coverageGaps,
      outputs,
      failures,
      retrieval_retry: retrievalRetry,
      gap_query_generation_retry: gapQueryGenerationRetry,
      semantic_web_selection_retry: webSourceSelectionRetry,
      semantic_selection_retry: semanticSelectionRetry,
      semantic_shard_selection_retry: semanticShardSelectionRetry,
    });
    if (supplementalRound.schedule) {
      return supplementalRound.schedule;
    }
    if (!supplementalRound.attempted) {
      break;
    }
    supplementalAttempted = true;
    supplementalRoundCount += 1;
    supplementalFetchCount += Number(supplementalRound.fetch_count || 0);
    generatedGapQueryCount += Number(supplementalRound.query_count || 0);
    excludedCandidates.push(...(
      Array.isArray(supplementalRound.attempted_candidates)
        ? supplementalRound.attempted_candidates
        : []
    ));
    selection = combineMaterializedSelections(
      selection,
      supplementalRound.selection
    );
    const stepIds = supplementalRoundStepIds(round);
    const roundGapQueries = Array.isArray(supplementalRound.queries)
      ? supplementalRound.queries
      : validatedGapQueries(
          roundPlan,
          structuredOutput(outputs[stepIds.generate_gap_queries]),
          roundQueryBudget
        ).queries;
    roundPlan = Object.assign({}, roundPlan, {
      search_queries: uniqueStrings([
        ...(Array.isArray(roundPlan.search_queries)
          ? roundPlan.search_queries
          : []),
        ...roundGapQueries,
      ]),
    });
    if (
      Number(supplementalRound.fetch_count || 0) === 0 &&
      Number(supplementalRound.query_count || 0) === 0
    ) {
      break;
    }
  }
  const finalAttributionPacket = materializedAttributionPacket(selection);
  const finalAttributionIdentity = JSON.stringify(finalAttributionPacket);
  let finalAttributionContract = reviewedAttributionContract;
  let finalAttributionError = reviewedAttributionError;
  if (finalAttributionIdentity !== reviewedAttributionIdentity) {
    const attributionReview = sourceAttributionReview(
      selection,
      STEP_ATTRIBUTE_SOURCES,
      outputs,
      failures,
      sourceAttributionRetry
    );
    if (attributionReview.schedule) {
      return attributionReview.schedule;
    }
    finalAttributionContract = attributionReview.contract;
    finalAttributionError = attributionReview.error;
  }
  const finalCoverageAttribution = finalAttributionContract ||
    (finalAttributionPacket.sources.length > 0
      ? { groups: [], independent_group_pairs: [] }
      : undefined);
  const finalCoverageGaps = typedCoverageGaps(
    plan,
    materializedSourceCoverage(selection),
    finalCoverageAttribution
  );
  selection.metadata = Object.assign({}, object(selection.metadata), {
    typed_coverage_gap_count: finalCoverageGaps.length,
    supplemental_retrieval_attempted: supplementalAttempted,
    supplemental_retrieval_round_count: supplementalRoundCount,
    supplemental_fetch_count: supplementalFetchCount,
    supplemental_source_count: Math.max(
      0,
      materializedSourceCount(selection) - initialMaterializedSourceCount
    ),
    generated_gap_query_count: generatedGapQueryCount,
  });
  if (finalAttributionPacket.sources.length > 0) {
    selection = applySourceAttribution(
      selection,
      finalAttributionContract,
      finalAttributionError
    );
  }
  const research = researchResult(selection);
  return {
    type: "complete",
    output: {
      query,
      mode: "inquiry_collection",
      plan,
      research,
      execution: {
        mode: "collect_only",
        terminal_authority: "host_inquiry_reducer",
        note: supplementalAttempted
          ? `The host-planned retrieval pass and up to ${gapRoundCount} coverage-directed supplemental passes completed. Closed-evidence review and convergence remain host-owned.`
          : "The host-planned retrieval pass completed without a runnable typed-coverage supplement. Closed-evidence review and convergence remain host-owned.",
      },
    },
  };
}
