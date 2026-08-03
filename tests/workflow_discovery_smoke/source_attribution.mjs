const attributionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: completedSupplementalInput,
    step_outputs: completedSupplementalStepOutputs,
    step_failures: {},
  },
);
assert.equal(attributionRequest.type, "schedule_step");
assert.equal(attributionRequest.step_id, "attribute_selected_sources");
assert.deepEqual(
  Array.from(
    JSON.parse(
      attributionRequest.input.prompt.split(
        "CLOSED_SOURCE_ATTRIBUTION_PACKET=",
      )[1],
    ).sources,
    (source) => source.source_id,
  ),
  ["catalog-source-1", "supplemental-catalog-source-1"],
);
assert.deepEqual(
  Object.keys(
    JSON.parse(
      attributionRequest.input.prompt.split(
        "CLOSED_SOURCE_ATTRIBUTION_PACKET=",
      )[1],
    ).sources[0],
  ).sort(),
  ["excerpts", "source_id", "title"],
);

const completedSupplementalRun = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: completedSupplementalInput,
    step_outputs: {
      ...completedSupplementalStepOutputs,
      attribute_selected_sources: {
        output: JSON.stringify({
          object: {
            attribution_groups: [{
              group_id: "original-record",
              source_ids: ["catalog-source-1"],
            }, {
              group_id: "separate-record",
              source_ids: ["supplemental-catalog-source-1"],
            }],
            independent_group_pairs: [{
              group_ids: ["original-record", "separate-record"],
            }],
          },
        }),
      },
    },
    step_failures: {},
  },
);
assert.equal(completedSupplementalRun.type, "complete");
const completedSources = completedSupplementalRun.output.research.results.flatMap(
  (result) => result.structured.sources,
);
assert.deepEqual(
  Array.from(completedSources, (source) => source.source_id),
  ["catalog-source-1", "supplemental-catalog-source-1"],
);
assert.equal(
  completedSupplementalRun.output.research.metadata.source_attribution_status,
  "verified",
);
assert.deepEqual(
  JSON.parse(JSON.stringify(
    completedSupplementalRun.output.research.metadata.source_attribution,
  )),
  {
    version: 1,
    groups: [{
      group_id: "attribution-group-1",
      source_ids: ["catalog-source-1"],
    }, {
      group_id: "attribution-group-2",
      source_ids: ["supplemental-catalog-source-1"],
    }],
    independent_group_pairs: [{
      group_ids: ["attribution-group-1", "attribution-group-2"],
    }],
  },
);

const invalidAttributionRun = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: completedSupplementalInput,
    step_outputs: {
      ...completedSupplementalStepOutputs,
      attribute_selected_sources: {
        output: JSON.stringify({
          object: {
            attribution_groups: [{
              group_id: "first",
              source_ids: ["catalog-source-1"],
            }, {
              group_id: "duplicate",
              source_ids: ["catalog-source-1"],
            }],
            independent_group_pairs: [{
              group_ids: ["first", "duplicate"],
            }],
          },
        }),
      },
    },
    step_failures: {},
  },
);
assert.equal(invalidAttributionRun.type, "complete");
assert.equal(invalidAttributionRun.output.research.status, "incomplete");
assert.equal(
  invalidAttributionRun.output.research.metadata.source_attribution_status,
  "unavailable",
);
assert.equal(
  Object.hasOwn(
    invalidAttributionRun.output.research.metadata,
    "source_attribution",
  ),
  false,
);
assert.match(
  invalidAttributionRun.output.research.warnings.collection_errors.join(" "),
  /multiply assigned source ID/,
);

const adaptiveAttributionLoopContract = {
  cardinality: { gap_extractions: 1 },
  hard_caps: {
    max_gap_searches: 4,
    max_supplemental_fetches: 2,
  },
};
const adaptiveAttributionPlan = {
  ...referencePlan,
  tracks: [{
    ...referencePlan.tracks[0],
    id: "adaptive-attribution-track",
    evidence_requirements: {
      primary_source_required: true,
      independent_corroboration_required: true,
    },
  }],
};
const adaptiveAttributionInput = {
  query: "Replace derivative corroboration with independent evidence",
  loop_contract: adaptiveAttributionLoopContract,
  execution_mode: "collect_only",
  evidence_scope: "web_and_workspace",
  research_plan: adaptiveAttributionPlan,
  bootstrap_acquisition: {},
};
const adaptiveInitialRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "original-record",
      title: "Original accountable record",
      url_or_path: "https://adaptive-original.example.test/record",
      reliability: "Fetched source text.",
      chunks: [{
        chunk_id: "original-record:chunk:1",
        text:
          "The accountable issuing authority's original record establishes the material criterion.",
      }],
    }, {
      source_id: "derivative-record",
      title: "Republication of the original accountable record",
      url_or_path: "https://adaptive-derivative.example.test/copy",
      reliability: "Fetched source text.",
      chunks: [{
        chunk_id: "derivative-record:chunk:1",
        text:
          "This record identifies itself as a verbatim republication of the same issuing authority's original record.",
      }],
    }],
  },
  errors: [],
  metadata: {},
};
const adaptiveInitialBaseOutputs = {
  ...referenceBaseOutputs,
  retrieve_web_source_1: adaptiveInitialRetrieval,
};
const adaptiveInitialSelectionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: adaptiveInitialBaseOutputs,
    step_failures: {},
  },
);
assert.equal(adaptiveInitialSelectionRequest.type, "schedule_step");
assert.equal(
  adaptiveInitialSelectionRequest.step_id,
  "select_evidence_chunks",
);
const adaptiveInitialPacket = JSON.parse(
  adaptiveInitialSelectionRequest.input.prompt.split(
    "CLOSED_EVIDENCE_PACKET=",
  )[1],
);
const [adaptivePrimarySource, adaptiveDerivativeSource] =
  adaptiveInitialPacket.sources;
const adaptiveFocus = adaptiveInitialPacket.focuses[0];
const adaptiveInitialSelection = {
  output: JSON.stringify({
    object: {
      chunk_ids: adaptiveInitialPacket.sources.flatMap((source) =>
        source.chunks.map((chunk) => chunk.chunk_id)
      ),
      source_coverage: [{
        source_id: adaptivePrimarySource.source_id,
        obligation_id: adaptiveFocus.obligation_id,
        completion_criterion_indexes: [0],
        roles: {
          supporting: true,
          primary: true,
          independent: false,
        },
      }, {
        source_id: adaptiveDerivativeSource.source_id,
        obligation_id: adaptiveFocus.obligation_id,
        completion_criterion_indexes: [0],
        roles: {
          supporting: true,
          primary: false,
          independent: true,
        },
      }],
      source_relevance: adaptiveInitialPacket.sources.map((source) => ({
        source_id: source.source_id,
        obligation_id: adaptiveFocus.obligation_id,
      })),
    },
  }),
};
const adaptiveSelectedOutputs = {
  ...adaptiveInitialBaseOutputs,
  select_evidence_chunks: adaptiveInitialSelection,
  checkpoint_initial_retrieval: {},
};
const adaptiveInitialAttributionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: adaptiveSelectedOutputs,
    step_failures: {},
  },
);
assert.equal(adaptiveInitialAttributionRequest.type, "schedule_step");
assert.equal(
  adaptiveInitialAttributionRequest.step_id,
  "attribute_selected_sources_round_1",
);
const adaptiveAttributionFailureRun = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: adaptiveSelectedOutputs,
    step_failures: {
      attribute_selected_sources_round_1: {
        error: "simulated closed attribution timeout",
      },
    },
  },
);
assert.equal(adaptiveAttributionFailureRun.type, "complete");
assert.equal(
  adaptiveAttributionFailureRun.output.research.metadata
    .source_attribution_status,
  "unavailable",
);
assert.equal(
  adaptiveAttributionFailureRun.output.research.metadata
    .typed_coverage_gap_count,
  1,
);
assert.equal(
  adaptiveAttributionFailureRun.output.research.metadata
    .supplemental_retrieval_attempted,
  false,
);
assert.match(
  adaptiveAttributionFailureRun.output.research.warnings
    .collection_errors.join(" "),
  /simulated closed attribution timeout/,
);
const adaptiveSameOriginAttribution = {
  output: JSON.stringify({
    object: {
      attribution_groups: [{
        group_id: "shared-accountable-origin",
        source_ids: [
          adaptivePrimarySource.source_id,
          adaptiveDerivativeSource.source_id,
        ],
      }],
      independent_group_pairs: [],
    },
  }),
};
const adaptiveAttributedOutputs = {
  ...adaptiveSelectedOutputs,
  attribute_selected_sources_round_1: adaptiveSameOriginAttribution,
};
const adaptiveGapQueryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: adaptiveAttributedOutputs,
    step_failures: {},
  },
);
assert.equal(adaptiveGapQueryRequest.type, "schedule_step");
assert.equal(adaptiveGapQueryRequest.step_id, "generate_gap_queries");
const adaptiveGapPacket = JSON.parse(
  adaptiveGapQueryRequest.input.prompt.split("TYPED_GAP_QUERY_PACKET=")[1],
);
assert.equal(adaptiveGapPacket.coverage_gaps.length, 1);
assert.deepEqual(
  Array.from(adaptiveGapPacket.coverage_gaps[0].missing_roles, (role) =>
    role.role
  ),
  ["independent"],
);
assert.deepEqual(
  Array.from(
    adaptiveGapPacket.coverage_gaps[0]
      .missing_completion_criterion_indexes,
  ),
  [],
);
assert.ok(
  adaptiveGapQueryRequest.input.prompt.includes(
    "target a separately accountable origin",
  ),
);
const adaptiveGapQueries = {
  output: JSON.stringify({
    object: { queries: ["separately accountable corroborating record"] },
  }),
};
const adaptiveNoChangeRetry = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      ...adaptiveAttributionInput,
      loop_contract: {
        ...adaptiveAttributionLoopContract,
        cardinality: { gap_extractions: 2 },
      },
    },
    step_outputs: {
      ...adaptiveAttributedOutputs,
      generate_gap_queries: adaptiveGapQueries,
      discover_gap_web_sources: {
        status: "failed",
        candidates: [],
        errors: ["No independently attributable candidate was retained."],
        metadata: {},
      },
      select_supplemental_web_sources: {
        output: JSON.stringify({ object: { candidate_ids: [] } }),
      },
    },
    step_failures: {},
  },
);
assert.equal(adaptiveNoChangeRetry.type, "schedule_step");
assert.equal(adaptiveNoChangeRetry.step_id, "generate_gap_queries_2");
const adaptiveGapDiscoveryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: {
      ...adaptiveAttributedOutputs,
      generate_gap_queries: adaptiveGapQueries,
    },
    step_failures: {},
  },
);
assert.equal(adaptiveGapDiscoveryRequest.type, "schedule_step");
assert.equal(
  adaptiveGapDiscoveryRequest.step_id,
  "discover_gap_web_sources",
);
const adaptiveGapDiscovery = {
  status: "success",
  candidates: [{
    candidate_id: "adaptive-independent-candidate",
    title: "Separately accountable review",
    url: "https://adaptive-independent.example.test/review",
    date: "",
    content:
      "A separately authored review corroborates the material criterion.",
    provider_text: "",
    engines: ["test-provider"],
    discovery: ["provider_result"],
    query_indexes: [0],
  }],
  errors: [],
  metadata: {},
};
const adaptiveAdmissionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: {
      ...adaptiveAttributedOutputs,
      generate_gap_queries: adaptiveGapQueries,
      discover_gap_web_sources: adaptiveGapDiscovery,
    },
    step_failures: {},
  },
);
assert.equal(adaptiveAdmissionRequest.type, "schedule_step");
assert.equal(
  adaptiveAdmissionRequest.step_id,
  "select_supplemental_web_sources",
);
const adaptiveAdmissionPacket = JSON.parse(
  adaptiveAdmissionRequest.input.prompt.split(
    "CLOSED_SUPPLEMENTAL_DISCOVERY_PACKET=",
  )[1],
);
const adaptiveIndependentCandidate = adaptiveAdmissionPacket.candidates.find(
  (candidate) =>
    candidate.url === "https://adaptive-independent.example.test/review",
);
assert.ok(adaptiveIndependentCandidate);
const adaptiveSourceSelection = {
  output: JSON.stringify({
    object: { candidate_ids: [adaptiveIndependentCandidate.candidate_id] },
  }),
};
const adaptiveFetchRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: {
      ...adaptiveAttributedOutputs,
      generate_gap_queries: adaptiveGapQueries,
      discover_gap_web_sources: adaptiveGapDiscovery,
      select_supplemental_web_sources: adaptiveSourceSelection,
    },
    step_failures: {},
  },
);
assert.equal(adaptiveFetchRequest.type, "schedule_steps");
assert.equal(adaptiveFetchRequest.steps.length, 1);
assert.equal(
  adaptiveFetchRequest.steps[0].step_id,
  "retrieve_supplemental_web_source_1",
);
const adaptiveSupplementalRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "independent-record",
      title: "Separately accountable review",
      url_or_path: "https://adaptive-independent.example.test/review",
      reliability: "Fetched source text.",
      chunks: [{
        chunk_id: "independent-record:chunk:1",
        text:
          "A separately accountable author independently evaluates and corroborates the material criterion.",
      }],
    }],
  },
  errors: [],
  metadata: {},
};
const adaptiveFetchedOutputs = {
  ...adaptiveAttributedOutputs,
  generate_gap_queries: adaptiveGapQueries,
  discover_gap_web_sources: adaptiveGapDiscovery,
  select_supplemental_web_sources: adaptiveSourceSelection,
  retrieve_supplemental_web_source_1: adaptiveSupplementalRetrieval,
};
const adaptiveSupplementalSelectionRequest =
  await sandbox.__deepResearchRun(
    {},
    {
      kind: "workflow",
      input: adaptiveAttributionInput,
      step_outputs: adaptiveFetchedOutputs,
      step_failures: {},
    },
  );
assert.equal(adaptiveSupplementalSelectionRequest.type, "schedule_step");
assert.equal(
  adaptiveSupplementalSelectionRequest.step_id,
  "select_supplemental_evidence_chunks",
);
const adaptiveSupplementalPacket = JSON.parse(
  adaptiveSupplementalSelectionRequest.input.prompt.split(
    "CLOSED_EVIDENCE_PACKET=",
  )[1],
);
const adaptiveIndependentSource = adaptiveSupplementalPacket.sources[0];
const adaptiveSupplementalFocus = adaptiveSupplementalPacket.focuses[0];
const adaptiveSupplementalSelection = {
  output: JSON.stringify({
    object: {
      chunk_ids: [adaptiveIndependentSource.chunks[0].chunk_id],
      source_coverage: [{
        source_id: adaptiveIndependentSource.source_id,
        obligation_id: adaptiveSupplementalFocus.obligation_id,
        completion_criterion_indexes: [0],
        roles: {
          supporting: true,
          primary: false,
          independent: true,
        },
      }],
      source_relevance: [{
        source_id: adaptiveIndependentSource.source_id,
        obligation_id: adaptiveSupplementalFocus.obligation_id,
      }],
    },
  }),
};
const adaptiveCompletedRetrievalOutputs = {
  ...adaptiveFetchedOutputs,
  select_supplemental_evidence_chunks: adaptiveSupplementalSelection,
};
const adaptiveFinalAttributionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: adaptiveCompletedRetrievalOutputs,
    step_failures: {},
  },
);
assert.equal(adaptiveFinalAttributionRequest.type, "schedule_step");
assert.equal(
  adaptiveFinalAttributionRequest.step_id,
  "attribute_selected_sources",
);
const adaptiveFinalAttributionPacket = JSON.parse(
  adaptiveFinalAttributionRequest.input.prompt.split(
    "CLOSED_SOURCE_ATTRIBUTION_PACKET=",
  )[1],
);
assert.deepEqual(
  Array.from(adaptiveFinalAttributionPacket.sources, (source) =>
    source.source_id
  ),
  [
    adaptivePrimarySource.source_id,
    adaptiveDerivativeSource.source_id,
    adaptiveIndependentSource.source_id,
  ],
);
const adaptiveFinalAttribution = {
  output: JSON.stringify({
    object: {
      attribution_groups: [{
        group_id: "shared-accountable-origin",
        source_ids: [
          adaptivePrimarySource.source_id,
          adaptiveDerivativeSource.source_id,
        ],
      }, {
        group_id: "separate-accountable-origin",
        source_ids: [adaptiveIndependentSource.source_id],
      }],
      independent_group_pairs: [{
        group_ids: [
          "shared-accountable-origin",
          "separate-accountable-origin",
        ],
      }],
    },
  }),
};
const adaptiveCompletedRun = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: adaptiveAttributionInput,
    step_outputs: {
      ...adaptiveCompletedRetrievalOutputs,
      attribute_selected_sources: adaptiveFinalAttribution,
    },
    step_failures: {},
  },
);
assert.equal(adaptiveCompletedRun.type, "complete");
assert.equal(
  adaptiveCompletedRun.output.research.metadata.typed_coverage_gap_count,
  0,
);
assert.equal(
  adaptiveCompletedRun.output.research.metadata
    .supplemental_retrieval_attempted,
  true,
);
assert.equal(
  adaptiveCompletedRun.output.research.metadata.source_attribution_status,
  "verified",
);
assert.equal(
  adaptiveCompletedRun.output.research.metadata
    .source_attribution_independent_pair_count,
  1,
);
