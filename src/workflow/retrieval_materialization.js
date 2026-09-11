  // When model-backed chunk selection times out or returns an unusable
  // object, promote a bounded closed-catalog excerpt set so acquired text can
  // still become claim-eligible. Coverage stays empty: this path never claims
  // full criterion closure. Relevance edges use focus-token overlap against
  // retained excerpt text, falling back to the first focus only when no token
  // matches — still closed IDs, never open-world routing.
  const closedCatalogDeterministicSelection = (packet) => {
    if (
      !packet ||
      !Array.isArray(packet.sources) ||
      packet.sources.length === 0 ||
      !Array.isArray(packet.focuses) ||
      packet.focuses.length === 0
    ) {
      return null;
    }
    const defaultObligation = packet.focuses[0] &&
      typeof packet.focuses[0].obligation_id === "string"
      ? packet.focuses[0].obligation_id
      : "";
    if (!defaultObligation) {
      return null;
    }
    const tokenize = (text) => String(text || "")
      .toLowerCase()
      .split(/[^a-z0-9\u4e00-\u9fff]+/)
      .filter((token) => token.length > 2);
    const chunkIds = [];
    const sourceRelevance = [];
    for (const source of packet.sources) {
      if (!source || !Array.isArray(source.chunks) || source.chunks.length === 0) {
        continue;
      }
      const sourceId = typeof source.source_id === "string"
        ? source.source_id
        : "";
      if (!sourceId) {
        continue;
      }
      let retainedChars = 0;
      let retainedCount = 0;
      const retainedTexts = [];
      for (const chunk of source.chunks) {
        if (retainedCount >= MAX_EXCERPTS_PER_SOURCE) {
          break;
        }
        const chunkId = typeof chunk.chunk_id === "string"
          ? chunk.chunk_id
          : "";
        const quote = String(chunk.text || "");
        const quoteChars = Array.from(quote).length;
        if (
          !chunkId ||
          !quote ||
          quoteChars > MAX_CHUNK_CHARS ||
          retainedChars + quoteChars > MAX_EXCERPT_CHARS_PER_SOURCE
        ) {
          continue;
        }
        chunkIds.push(chunkId);
        retainedTexts.push(quote);
        retainedChars += quoteChars;
        retainedCount += 1;
      }
      if (retainedCount === 0) {
        continue;
      }
      const blob = retainedTexts.join(" ").toLowerCase();
      let bestObligation = defaultObligation;
      let bestScore = 0;
      for (const focus of packet.focuses) {
        const obligationId = focus &&
          typeof focus.obligation_id === "string"
          ? focus.obligation_id
          : "";
        if (!obligationId) {
          continue;
        }
        let score = 0;
        for (const token of tokenize(focus.focus)) {
          if (blob.includes(token)) {
            score += 1;
          }
        }
        if (score > bestScore) {
          bestScore = score;
          bestObligation = obligationId;
        }
      }
      sourceRelevance.push({
        source_id: sourceId,
        obligation_id: bestObligation,
      });
    }
    if (chunkIds.length === 0 || sourceRelevance.length === 0) {
      return null;
    }
    return {
      chunk_ids: chunkIds,
      source_coverage: [],
      source_relevance: sourceRelevance,
    };
  };

  const resolveClosedEvidenceSelection = (packet, semanticSelection, errors) => {
    if (
      semanticSelection &&
      Array.isArray(semanticSelection.chunk_ids) &&
      semanticSelection.chunk_ids.length > 0
    ) {
      return { selector: semanticSelection, used_fallback: false };
    }
    const fallback = closedCatalogDeterministicSelection(packet);
    if (!fallback) {
      return { selector: semanticSelection, used_fallback: false };
    }
    if (Array.isArray(errors)) {
      errors.push(
        "Semantic chunk selection did not complete; closed-catalog deterministic excerpts were promoted for claim-eligible publication."
      );
    }
    return { selector: fallback, used_fallback: true };
  };

  const materializeEvidence = (packet, selector, errors, metadata) => {
    const boundedErrors = uniqueStrings(errors).slice(0, 16);
    if (!packet || !selector || !Array.isArray(selector.chunk_ids)) {
      return {
        status: "failed",
        results: [],
        errors: uniqueStrings([
          ...boundedErrors,
          "Retrieved text was not promoted because semantic chunk selection did not complete.",
        ]),
        metadata: Object.assign({}, metadata, {
          evidence_selection_mode: "semantic_chunk_ids",
          source_count: 0,
          selection_count: 0,
        }),
      };
    }
    const chunkById = new Map();
    for (const source of packet.sources) {
      for (const chunk of source.chunks) {
        chunkById.set(chunk.chunk_id, { source, chunk });
      }
    }
    const selectionFocus = bounded(
      packet.focuses
        .map((focus) => focus && focus.focus)
        .filter(nonEmpty)
        .join(" | "),
      300
    ) || "Research plan evidence";
    const selectedBySource = new Map();
    const seenChunkIds = new Set();
    let invalidSelection = "";
    for (const chunkId of selector.chunk_ids) {
      const selected = chunkById.get(chunkId);
      if (!selected) {
        invalidSelection =
          "Semantic chunk selection returned an ID outside the closed catalog.";
        break;
      }
      const { source, chunk } = selected;
      const retained = selectedBySource.get(source.source_id) || [];
      const retainedChars = retained.reduce(
        (total, item) => total + Array.from(item.quote_or_fact).length,
        0
      );
      if (
        seenChunkIds.has(chunkId) ||
        retained.length >= MAX_EXCERPTS_PER_SOURCE ||
        retainedChars >= MAX_EXCERPT_CHARS_PER_SOURCE
      ) {
        invalidSelection =
          "Semantic chunk selection violated the closed selection limits.";
        break;
      }
      const quote = String(chunk.text || "");
      if (
        !quote ||
        Array.from(quote).length > MAX_CHUNK_CHARS ||
        retainedChars + Array.from(quote).length >
          MAX_EXCERPT_CHARS_PER_SOURCE
      ) {
        invalidSelection =
          "Semantic chunk selection referenced an invalid bounded chunk.";
        break;
      }
      seenChunkIds.add(chunkId);
      retained.push({
        focus: selectionFocus,
        quote_or_fact: quote,
      });
      selectedBySource.set(source.source_id, retained);
    }
    if (invalidSelection) {
      return {
        status: "failed",
        results: [],
        errors: uniqueStrings([...boundedErrors, invalidSelection]),
        metadata: Object.assign({}, metadata, {
          evidence_selection_mode: "semantic_chunk_ids",
          source_count: 0,
          selection_count: 0,
        }),
      };
    }
    const sourceCoverage = validatedSourceCoverage(
      packet,
      selector,
      seenChunkIds
    );
    if (sourceCoverage.error) {
      return {
        status: "failed",
        results: [],
        errors: uniqueStrings([...boundedErrors, sourceCoverage.error]),
        metadata: Object.assign({}, metadata, {
          evidence_selection_mode: "semantic_chunk_ids_with_typed_coverage",
          source_count: 0,
          selection_count: 0,
          source_coverage_count: 0,
        }),
      };
    }
    const sourceRelevance = validatedSourceRelevance(
      packet,
      selector,
      seenChunkIds
    );
    if (sourceRelevance.error) {
      return {
        status: "failed",
        results: [],
        errors: uniqueStrings([...boundedErrors, sourceRelevance.error]),
        metadata: Object.assign({}, metadata, {
          evidence_selection_mode: "semantic_chunk_ids_with_typed_relevance",
          source_count: 0,
          selection_count: 0,
          source_coverage_count: 0,
          source_relevance_count: 0,
        }),
      };
    }
    const durableSourceCoverage = sourceCoverage.bindings.map((binding) => {
      const roles = object(binding.roles);
      return Object.assign({}, binding, {
        roles: ["supporting", "primary", "independent"].filter((role) =>
          roles[role] === true
        ),
      });
    });
    const sources = packet.sources.map((source) => {
      const excerpts = selectedBySource.get(source.source_id) || [];
      if (excerpts.length === 0) {
        return null;
      }
      return {
        source_id: source.source_id,
        title: source.title,
        url_or_path: source.url_or_path,
        quote_or_fact: excerpts[0].quote_or_fact,
        evidence_excerpts: excerpts,
        date: source.date,
        reliability: source.reliability,
      };
    }).filter(Boolean);
    if (sources.length === 0) {
      return {
        status: "failed",
        results: [],
        errors: uniqueStrings([
          ...boundedErrors,
          "Semantic chunk selection retained no source text.",
        ]),
        metadata: Object.assign({}, metadata, {
          evidence_selection_mode: "semantic_chunk_ids",
          source_count: 0,
          selection_count: 0,
        }),
      };
    }
    const facts = sources.flatMap((source) =>
      source.evidence_excerpts.map((excerpt) => excerpt.quote_or_fact)
    );
    const results = sources.map((source) => {
      const sourceFacts = uniqueStrings(
        source.evidence_excerpts.map((excerpt) => excerpt.quote_or_fact)
      );
      return {
        task_id: `evidence_retrieval:${source.source_id}`,
        agent: "workflow",
        success: true,
        structured: {
          summary: `Semantic selection retained ${sourceFacts.length} fetched evidence chunk(s) from one source.`,
          sources: [source],
          source_coverage: durableSourceCoverage.filter((binding) =>
            binding.source_id === source.source_id
          ),
          source_relevance: sourceRelevance.bindings.filter((binding) =>
            binding.source_id === source.source_id
          ),
          relevant_obligation_ids: sourceRelevance.bindings
            .filter((binding) => binding.source_id === source.source_id)
            .map((binding) => binding.obligation_id),
          key_evidence: sourceFacts,
          contradictions: [],
          confidence: "Closed-evidence review required; source text was restored from the closed catalog by semantic chunk ID.",
          gaps: [],
        },
      };
    });
    return {
      status: boundedErrors.length > 0 ? "partial" : "success",
      results,
      errors: boundedErrors,
      metadata: Object.assign({}, metadata, {
        evidence_selection_mode: "semantic_chunk_ids_with_typed_coverage",
        source_count: sources.length,
        selection_count: facts.length,
        source_coverage_count: durableSourceCoverage.length,
        source_relevance_count: sourceRelevance.bindings.length,
      }),
    };
  };

  const materializedSourceCoverage = (selection) =>
    (Array.isArray(selection && selection.results) ? selection.results : [])
      .flatMap((result) => {
        const structured = object(result && result.structured);
        return Array.isArray(structured.source_coverage)
          ? structured.source_coverage
          : [];
      });

  const materializedSourceCount = (selection) => new Set(
    (Array.isArray(selection && selection.results) ? selection.results : [])
      .flatMap((result) => {
        const structured = object(result && result.structured);
        return Array.isArray(structured.sources) ? structured.sources : [];
      })
      .map((source) => String(source && source.source_id || ""))
      .filter(nonEmpty)
  ).size;

  const researchResult = (selection) => {
    const results = Array.isArray(selection.results) ? selection.results : [];
    const errors = Array.isArray(selection.errors) ? selection.errors : [];
    const status = selection.status === "success"
      ? "success"
      : (results.length > 0 ? "incomplete" : "failed");
    return {
      tool: "web_search/web_fetch/read",
      algorithm:
        "plan_discover_semantic_admit_retrieve_typed_coverage_supplement_attribute",
      status,
      metadata: Object.assign({}, object(selection.metadata), {
        result_count: results.length,
        source_count: results.reduce(
          (total, result) =>
            total + (
              result && result.structured && Array.isArray(result.structured.sources)
                ? result.structured.sources.length
                : 0
            ),
          0
        ),
        evidence_selection_mode: "semantic_chunk_ids_with_typed_coverage",
      }),
      results,
      warnings: errors.length > 0
        ? { collection_errors: errors }
        : undefined,
    };
  };

  const initialRetrievalCheckpointOutput = (query, plan, selection) => ({
    query,
    mode: "inquiry_collection",
    plan,
    research: researchResult(selection),
    execution: {
      mode: "collect_only",
      terminal_authority: "host_inquiry_reducer",
      note: "The initial closed-evidence portfolio was durably checkpointed before the optional supplemental pass. Closed-evidence review and convergence remain host-owned.",
    },
  });
