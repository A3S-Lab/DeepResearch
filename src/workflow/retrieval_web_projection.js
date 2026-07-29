  const webSourceFetchSteps = (
    stepIdPrefix,
    plan,
    candidates,
    sourceIdPrefix,
    fetchTimeoutSecs,
    retry,
    preferBoundedSnapshot
  ) => (Array.isArray(candidates) ? candidates : [])
    .slice(0, MAX_SOURCES)
    .map((candidate, index) => ({
      step_id: `${stepIdPrefix}${index + 1}`,
      step_name: STEP_WEB_SOURCE,
      input: {
        plan,
        candidates: [candidate],
        discovery_errors: [],
        discovery_metadata: {},
        source_selection_mode: "per_source_effect",
        source_id_prefix: sourceIdPrefix,
        source_index_offset: index,
        fetch_timeout_secs: fetchTimeoutSecs,
        prefer_bounded_snapshot: preferBoundedSnapshot === true,
      },
      retry,
    }));

  const webRetrievalFromSourceSteps = (settings) => {
    const plan = object(settings.plan);
    const candidates = Array.isArray(settings.candidates)
      ? settings.candidates.slice(0, MAX_SOURCES)
      : [];
    const outputs = object(settings.outputs);
    const failures = object(settings.failures);
    const retrievals = candidates.map((_candidate, index) => {
      const stepId = `${settings.step_id_prefix}${index + 1}`;
      return outputs[stepId] || {
        status: "failed",
        packet: null,
        errors: [
          failures[stepId] && failures[stepId].error ||
            `Source effect ${index + 1} did not complete.`,
        ],
        metadata: {},
      };
    });
    const admission = combinedEvidencePacket(
      plan,
      retrievals,
      settings.catalog_source_prefix
    );
    const errors = uniqueStrings([
      ...(Array.isArray(settings.discovery_errors)
        ? settings.discovery_errors
        : []),
      ...retrievals.flatMap((retrieval) =>
        Array.isArray(retrieval.errors) ? retrieval.errors : []
      ),
      admission.error || "",
    ]);
    const metadataTotal = (field) => retrievals.reduce(
      (total, retrieval) => {
        const value = Number(object(retrieval.metadata)[field]);
        return total + (Number.isSafeInteger(value) && value >= 0 ? value : 0);
      },
      0
    );
    return {
      status: admission.packet
        ? (errors.length > 0 ? "partial" : "success")
        : "failed",
      packet: admission.packet,
      errors: errors.slice(0, 12),
      metadata: Object.assign({}, object(settings.discovery_metadata), {
        source_selection_mode: String(
          settings.source_selection_mode || "unknown"
        ),
        selected_candidate_count: candidates.length,
        completed_source_effect_count: retrievals.filter((_retrieval, index) =>
          Boolean(outputs[`${settings.step_id_prefix}${index + 1}`])
        ).length,
        fetched_count: admission.source_count,
        transport_retry_count: metadataTotal("transport_retry_count"),
        transport_retry_success_count:
          metadataTotal("transport_retry_success_count"),
        batch_output_recovery_count:
          metadataTotal("batch_output_recovery_count"),
        document_range_count: metadataTotal("document_range_count"),
        provider_full_text_count: metadataTotal("provider_full_text_count"),
        catalog_chunk_count: admission.chunk_count,
      }),
    };
  };
