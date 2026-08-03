const criterionGapPlan = {
  ...referencePlan,
  tracks: Array.from({ length: 3 }, (_value, trackIndex) => ({
    ...referencePlan.tracks[0],
    id: `criterion-track-${trackIndex + 1}`,
    title: `Criterion track ${trackIndex + 1}`,
    focus: `Resolve every criterion for track ${trackIndex + 1}.`,
    completion_criteria: Array.from(
      { length: 3 },
      (_criterion, criterionIndex) =>
        `Track ${trackIndex + 1} criterion ${criterionIndex + 1} is established.`,
    ),
  })),
};
const criterionGapQueryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve every atomic completion criterion",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: criterionGapPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [referenceSource.chunks[0].chunk_id],
            source_coverage: [],
            source_relevance: [{
              source_id: referenceSource.source_id,
              obligation_id: criterionGapPlan.tracks[0].id,
            }],
          },
        }),
      },
      checkpoint_initial_retrieval: {},
    },
    step_failures: {},
  },
);
assert.equal(criterionGapQueryRequest.step_id, "generate_gap_queries");
const criterionGapPacket = JSON.parse(
  criterionGapQueryRequest.input.prompt.split("TYPED_GAP_QUERY_PACKET=")[1],
);
assert.equal("focuses" in criterionGapPacket, false);
assert.deepEqual(Array.from(criterionGapPacket.initial_attempts), []);
assert.deepEqual(
  Array.from(criterionGapPacket.coverage_gaps, (gap) => [
    gap.obligation_id,
    Array.from(gap.missing_completion_criterion_indexes),
  ]),
  [
    ["criterion-track-1", [0]],
    ["criterion-track-2", [0]],
    ["criterion-track-3", [0]],
    ["criterion-track-1", [1]],
  ],
);
const criterionGapQueries = {
  output: JSON.stringify({
    object: {
      queries: Array.from(
        { length: 4 },
        (_value, index) => `atomic criterion query ${index + 1}`,
      ),
    },
  }),
};
const criterionGapDiscovery = {
  status: "success",
  candidates: Array.from({ length: 4 }, (_value, index) => ({
    candidate_id: `criterion-gap-candidate-${index + 1}`,
    title: `Atomic criterion record ${index + 1}`,
    url: `https://criterion-gap-${index + 1}.example.test/record`,
    date: "",
    content: `Candidate evidence for atomic criterion ${index + 1}.`,
    engines: ["test-provider"],
    discovery: ["provider_result"],
    query_indexes: [index],
  })),
  errors: [],
  metadata: {},
};
const criterionGapSelectorRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve every atomic completion criterion",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: criterionGapPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [referenceSource.chunks[0].chunk_id],
            source_coverage: [],
            source_relevance: [{
              source_id: referenceSource.source_id,
              obligation_id: criterionGapPlan.tracks[0].id,
            }],
          },
        }),
      },
      checkpoint_initial_retrieval: {},
      generate_gap_queries: criterionGapQueries,
      discover_gap_web_sources: criterionGapDiscovery,
    },
    step_failures: {},
  },
);
assert.equal(criterionGapSelectorRequest.type, "schedule_step");
assert.equal(
  criterionGapSelectorRequest.step_id,
  "select_supplemental_web_sources",
);
const criterionGapSelectorPacket = JSON.parse(
  criterionGapSelectorRequest.input.prompt.split(
    "CLOSED_SUPPLEMENTAL_DISCOVERY_PACKET=",
  )[1],
);
assert.deepEqual(
  Array.from(criterionGapSelectorPacket.coverage_gaps, (gap) => [
    gap.obligation_id,
    Array.from(gap.missing_completion_criterion_indexes),
  ]),
  [
    ["criterion-track-1", [0]],
    ["criterion-track-2", [0]],
    ["criterion-track-3", [0]],
    ["criterion-track-1", [1]],
  ],
);
assert.deepEqual(
  Array.from(
    criterionGapSelectorPacket.focuses,
    (focus) => focus.obligation_id,
  ),
  ["criterion-track-1", "criterion-track-2", "criterion-track-3"],
);

const perCriterionRolePlan = {
  ...referencePlan,
  tracks: [{
    ...referencePlan.tracks[0],
    id: "per-criterion-role-track",
    title: "Per-criterion role track",
    focus: "Resolve two criteria with criterion-local source roles.",
    completion_criteria: [
      "The first criterion is established.",
      "The second criterion is established.",
    ],
    evidence_requirements: {
      primary_source_required: true,
      independent_corroboration_required: true,
    },
  }],
};
const perCriterionRoleRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [
      {
        source_id: "role-source-primary",
        title: "Primary record",
        url_or_path: "https://role-primary.example.test/record",
        reliability: "Fetched source text.",
        chunks: [{
          chunk_id: "role-source-primary:chunk:1",
          text: "The original record establishes both requested criteria.",
        }],
      },
      {
        source_id: "role-source-independent-one",
        title: "First independent record",
        url_or_path: "https://role-independent-one.example.test/record",
        reliability: "Fetched source text.",
        chunks: [{
          chunk_id: "role-source-independent-one:chunk:1",
          text: "An independently attributable record corroborates only the first criterion.",
        }],
      },
      {
        source_id: "role-source-independent-two",
        title: "Second independent record",
        url_or_path: "https://role-independent-two.example.test/record",
        reliability: "Fetched source text.",
        chunks: [{
          chunk_id: "role-source-independent-two:chunk:1",
          text: "Another independently attributable record corroborates only the first criterion.",
        }],
      },
    ],
  },
  errors: [],
  metadata: {},
};
const perCriterionRoleBaseOutputs = {
  discover_web_sources: referenceDiscovery,
  select_web_sources: {
    output: JSON.stringify({
      object: { candidate_ids: ["web-candidate-1"] },
    }),
  },
  retrieve_web_source_1: perCriterionRoleRetrieval,
};
const perCriterionRoleSelectionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve criterion-local source roles",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: perCriterionRolePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: perCriterionRoleBaseOutputs,
    step_failures: {},
  },
);
assert.equal(perCriterionRoleSelectionRequest.type, "schedule_step");
assert.equal(
  perCriterionRoleSelectionRequest.step_id,
  "select_evidence_chunks",
);
const perCriterionRolePacket = JSON.parse(
  perCriterionRoleSelectionRequest.input.prompt.split(
    "CLOSED_EVIDENCE_PACKET=",
  )[1],
);
const [primaryRoleSource, firstIndependentRoleSource,
  secondIndependentRoleSource] = perCriterionRolePacket.sources;
const perCriterionRoleFocus = perCriterionRolePacket.focuses[0];
const perCriterionRoleSelectorOutput = {
  output: JSON.stringify({
    object: {
      chunk_ids: perCriterionRolePacket.sources.flatMap((source) =>
        source.chunks.map((chunk) => chunk.chunk_id)
      ),
      source_coverage: [
        {
          source_id: primaryRoleSource.source_id,
          obligation_id: perCriterionRoleFocus.obligation_id,
          completion_criterion_indexes: [0, 1],
          roles: {
            supporting: true,
            primary: true,
            independent: false,
          },
        },
        ...[firstIndependentRoleSource, secondIndependentRoleSource].map(
          (source) => ({
            source_id: source.source_id,
            obligation_id: perCriterionRoleFocus.obligation_id,
            completion_criterion_indexes: [0],
            roles: {
              supporting: true,
              primary: false,
              independent: true,
            },
          }),
        ),
      ],
      source_relevance: perCriterionRolePacket.sources.map((source) => ({
        source_id: source.source_id,
        obligation_id: perCriterionRoleFocus.obligation_id,
      })),
    },
  }),
};
const perCriterionRoleGapRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve criterion-local source roles",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: perCriterionRolePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...perCriterionRoleBaseOutputs,
      select_evidence_chunks: perCriterionRoleSelectorOutput,
      checkpoint_initial_retrieval: {},
    },
    step_failures: {},
  },
);
assert.equal(perCriterionRoleGapRequest.type, "schedule_step");
assert.equal(perCriterionRoleGapRequest.step_id, "generate_gap_queries");
const perCriterionRoleGapPacket = JSON.parse(
  perCriterionRoleGapRequest.input.prompt.split(
    "TYPED_GAP_QUERY_PACKET=",
  )[1],
);
assert.equal(perCriterionRoleGapPacket.coverage_gaps.length, 1);
assert.deepEqual(
  Array.from(
    perCriterionRoleGapPacket.coverage_gaps[0]
      .missing_completion_criterion_indexes,
  ),
  [],
);
assert.deepEqual(
  Array.from(
    perCriterionRoleGapPacket.coverage_gaps[0].missing_roles[0]
      .completion_criterion_indexes,
  ),
  [1],
);
assert.equal(
  perCriterionRoleGapPacket.coverage_gaps[0].missing_roles[0].role,
  "independent",
);

const fullGapLoopContract = {
  cardinality: { gap_extractions: 4 },
  hard_caps: {
    max_gap_searches: 24,
    max_supplemental_fetches: 32,
  },
};
const fullCriterionGapPlan = {
  ...criterionGapPlan,
  tracks: Array.from({ length: 8 }, (_value, trackIndex) => ({
    ...criterionGapPlan.tracks[trackIndex % criterionGapPlan.tracks.length],
    id: `full-criterion-track-${trackIndex + 1}`,
    title: `Full criterion track ${trackIndex + 1}`,
    focus: `Resolve every criterion for full track ${trackIndex + 1}.`,
  })),
};
const fullCriterionGapQueryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve the maximum bounded set of atomic criteria",
      loop_contract: fullGapLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: fullCriterionGapPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [referenceSource.chunks[0].chunk_id],
            source_coverage: [],
            source_relevance: [{
              source_id: referenceSource.source_id,
              obligation_id: fullCriterionGapPlan.tracks[0].id,
            }],
          },
        }),
      },
      checkpoint_initial_retrieval: {},
    },
    step_failures: {},
  },
);
assert.equal(
  fullCriterionGapQueryRequest.input.schema.properties.queries.maxItems,
  6,
);
const fullCriterionGapPacket = JSON.parse(
  fullCriterionGapQueryRequest.input.prompt.split(
    "TYPED_GAP_QUERY_PACKET=",
  )[1],
);
assert.equal(fullCriterionGapPacket.coverage_gaps.length, 6);
assert.deepEqual(
  Array.from(fullCriterionGapPacket.coverage_gaps, (gap) =>
    gap.missing_completion_criterion_indexes[0]
  ),
  Array(6).fill(0),
);
const referenceGapQueries = {
  output: JSON.stringify({
    object: {
      queries: ["site:fresh.example.test fresh direct evidence query"],
    },
  }),
};
const referenceGapDiscoveryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [referenceSource.chunks[0].chunk_id],
            source_coverage: [],
            source_relevance: [{
              source_id: referenceSource.source_id,
              obligation_id: referenceFocus.obligation_id,
            }],
          },
        }),
      },
      checkpoint_initial_retrieval: {},
      generate_gap_queries: referenceGapQueries,
    },
    step_failures: {},
  },
);
assert.equal(referenceGapDiscoveryRequest.type, "schedule_step");
assert.equal(referenceGapDiscoveryRequest.step_id, "discover_gap_web_sources");
assert.deepEqual(
  Array.from(referenceGapDiscoveryRequest.input.plan.search_queries),
  ["fresh.example.test fresh direct evidence query"],
);
const referenceGapDiscovery = {
  status: "success",
  candidates: [{
    candidate_id: "web-candidate-1",
    title: "Fresh gap candidate",
    url: "https://fresh.example.test/direct",
    date: "",
    content: "A fresh candidate discovered from the typed gap.",
    engines: ["test-provider"],
    discovery: ["provider_result"],
    query_indexes: [0],
  }],
  errors: [],
  metadata: {},
};
const referenceSupplementalControlOutputs = {
  generate_gap_queries: referenceGapQueries,
  discover_gap_web_sources: referenceGapDiscovery,
};
const referenceSupplementalRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [referenceSource.chunks[0].chunk_id],
            source_coverage: [],
            source_relevance: [{
              source_id: referenceSource.source_id,
              obligation_id: referenceFocus.obligation_id,
            }],
          },
        }),
      },
      checkpoint_initial_retrieval: {},
      ...referenceSupplementalControlOutputs,
    },
    step_failures: {},
  },
);
assert.equal(referenceSupplementalRequest.type, "schedule_step");
assert.equal(
  referenceSupplementalRequest.step_id,
  "select_supplemental_web_sources",
);
assert.equal(referenceSupplementalRequest.input.timeout_ms, 180_000);
assert.equal(referenceSupplementalRequest.retry.max_attempts, 1);
assert.ok(
  referenceSupplementalRequest.input.schema.properties.candidate_ids.maxItems <=
    supplementalLoopContract.hard_caps.max_supplemental_fetches / 2,
);
assert.ok(
  referenceSupplementalRequest.input.prompt.includes(
    "Do not collapse candidates that address different missing completion criteria",
  ),
);
