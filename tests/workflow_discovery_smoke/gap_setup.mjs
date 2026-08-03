const supplementalLoopContract = {
  cardinality: { gap_extractions: 2 },
  hard_caps: {
    max_gap_searches: 8,
    max_supplemental_fetches: 8,
  },
};
const referencePlan = {
  report_title: "Follow retained evidence references",
  research_scope: "comprehensive",
  freshness_required: false,
  workspace_evidence_required: false,
  tracks: [{
    id: "reference-track",
    title: "Reference track",
    focus: "Resolve the material question from a direct source.",
    material: true,
    questions: ["Which direct source resolves the material question?"],
    completion_criteria: ["A direct source resolves the material question."],
    evidence_requirements: {
      primary_source_required: true,
      independent_corroboration_required: false,
    },
  }],
  search_queries: ["direct source for the material question"],
  seed_urls: [],
  stop_conditions: [],
  budget: {
    retrieval_timeout_ms: 30_000,
    direct_searches: 1,
    direct_fetches: 2,
  },
};
const referenceDiscovery = {
  status: "success",
  candidates: [
    {
      candidate_id: "web-candidate-1",
      title: "Initially fetched source",
      url: "https://already.example.test/source",
      date: "",
      content: "A relevant secondary source.",
      engines: ["test-provider"],
      discovery: ["provider_result"],
      query_indexes: [0],
    },
    {
      candidate_id: "web-candidate-2",
      title: "Existing discovery candidate",
      url: "https://remaining.example.test/candidate",
      date: "",
      content: "An existing supplemental candidate.",
      engines: ["test-provider"],
      discovery: ["provider_result"],
      query_indexes: [0],
    },
  ],
  errors: [],
  metadata: {},
};
const bootstrapFetchRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Acquire exact-query evidence while planning is still running",
      execution_mode: "bootstrap_acquisition",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
    },
    step_outputs: { discover_web_sources: referenceDiscovery },
    step_failures: {},
  },
);
assert.equal(bootstrapFetchRequest.type, "schedule_steps");
assert.ok(bootstrapFetchRequest.steps.length > 0);
assert.ok(
  bootstrapFetchRequest.steps.every(
    (step) =>
      step.step_name === "retrieve_web_source" &&
      !step.step_id.includes("select") &&
      step.input.prefer_bounded_snapshot === true,
  ),
);
const eightFocusPlan = {
  ...referencePlan,
  tracks: Array.from({ length: 8 }, (_value, index) => ({
    ...referencePlan.tracks[0],
    id: `bounded-focus-${index + 1}`,
    title: `Bounded focus ${index + 1}`,
    focus: `Resolve bounded focus ${index + 1} from traceable evidence.`,
    questions: [`Which evidence resolves bounded focus ${index + 1}?`],
    completion_criteria: [
      `One source resolves bounded focus ${index + 1}.`,
    ],
  })),
};
const eightFocusSelectionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Resolve eight bounded research focuses",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: eightFocusPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: { discover_web_sources: referenceDiscovery },
    step_failures: {},
  },
);
assert.equal(eightFocusSelectionRequest.type, "schedule_step");
assert.equal(eightFocusSelectionRequest.step_id, "select_web_sources");
assert.equal(eightFocusSelectionRequest.input.timeout_ms, 180_000);
assert.equal(eightFocusSelectionRequest.retry.max_attempts, 1);
const eightFocusPacket = JSON.parse(
  eightFocusSelectionRequest.input.prompt.split(
    "CLOSED_WEB_DISCOVERY_PACKET=",
  )[1],
);
assert.equal(eightFocusPacket.focuses.length, 8);
const semanticAdmissionFetchRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Preserve complete retrieval after semantic source admission",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      discover_web_sources: referenceDiscovery,
      select_web_sources: {
        output: JSON.stringify({
          object: { candidate_ids: ["web-candidate-1"] },
        }),
      },
    },
    step_failures: {},
  },
);
assert.equal(semanticAdmissionFetchRequest.type, "schedule_steps");
assert.equal(semanticAdmissionFetchRequest.steps.length, 1);
assert.equal(
  semanticAdmissionFetchRequest.steps[0].step_name,
  "retrieve_web_source",
);
assert.equal(
  semanticAdmissionFetchRequest.steps[0].input.prefer_bounded_snapshot,
  false,
);
const fetchableFallbackDiscovery = {
  status: "success",
  candidates: [
    {
      candidate_id: "web-candidate-1",
      title: "Transport-unknown candidate",
      url: "https://transport-unknown.example.test/record",
      date: "",
      content: "A discovery snippet without a retained provider snapshot.",
      provider_text: "",
      engines: ["test-provider"],
      discovery: ["provider_result"],
      query_indexes: [0],
    },
    {
      candidate_id: "web-candidate-2",
      title: "Fetchable provider candidate",
      url: "https://fetchable-provider.example.test/record",
      date: "",
      content: "A discovery snippet with bounded provider source text.",
      provider_text:
        "A complete bounded provider snapshot remains available for fetched-text review.",
      engines: ["test-provider"],
      discovery: ["provider_result"],
      query_indexes: [0],
    },
  ],
  errors: [],
  metadata: {},
};
const fetchableFallbackRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Prefer recoverable transport during source admission fallback",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: { discover_web_sources: fetchableFallbackDiscovery },
    step_failures: {
      select_web_sources: {
        error: "simulated semantic source admission timeout",
      },
    },
  },
);
assert.equal(fetchableFallbackRequest.type, "schedule_steps");
assert.equal(
  fetchableFallbackRequest.steps[0].input.candidates[0].url,
  "https://fetchable-provider.example.test/record",
);
assert.equal(
  fetchableFallbackRequest.steps[0].input.prefer_bounded_snapshot,
  true,
);
const referenceRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "web-source-1",
      title: "Initially fetched source",
      url_or_path: "https://already.example.test/source",
      reliability: "Fetched source text.",
      chunks: [
        {
          chunk_id: "web-source-1:chunk:1",
          text: [
            "The retained evidence cites",
            "[a direct record](https://reference.example.test/primary),",
            "the already fetched",
            "[source](https://already.example.test/source),",
            "and an existing",
            "[candidate](https://remaining.example.test/candidate).",
            "An illustration",
            "![chart](https://media.example.test/chart.webp)",
            "is not a document reference.",
            "It repeats https://reference.example.test/primary.",
            "It also contains http://unsafe.example.test/plain.",
            "A split boundary ends with https://truncated.example.test/part",
          ].join(" "),
        },
        {
          chunk_id: "web-source-1:chunk:2",
          text:
            "Unselected text cites https://unselected.example.test/ignored.",
        },
      ],
    }],
  },
  errors: [],
  metadata: {},
};
const referenceBaseOutputs = {
  discover_web_sources: referenceDiscovery,
  select_web_sources: {
    output: JSON.stringify({
      object: { candidate_ids: ["web-candidate-1"] },
    }),
  },
  retrieve_web_source_1: referenceRetrieval,
};
const referenceSelectionRequest = await sandbox.__deepResearchRun(
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
    step_outputs: referenceBaseOutputs,
    step_failures: {},
  },
);
assert.equal(referenceSelectionRequest.type, "schedule_step");
assert.equal(referenceSelectionRequest.step_id, "select_evidence_chunks");

const largeShardRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "web-source-1",
      title: "Large fetched source",
      url_or_path: "https://already.example.test/source",
      reliability: "Fetched source text.",
      chunks: Array.from({ length: 64 }, (_value, index) => ({
        chunk_id: `web-source-1:chunk:${index + 1}`,
        text: `Evidence chunk ${index + 1} ${"x".repeat(4096)}`,
      })),
    }],
  },
  errors: [],
  metadata: {},
};
const largeShardRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from a large fetched source",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      retrieve_web_source_1: largeShardRetrieval,
    },
    step_failures: {},
  },
);
assert.equal(largeShardRequest.type, "schedule_steps");
assert.ok(largeShardRequest.steps.length > 1);
for (const step of largeShardRequest.steps) {
  assert.ok(
    Buffer.byteLength(step.input.prompt, "utf8") < 128 * 1024,
    `${step.step_id} exceeded the runtime prompt byte ceiling`,
  );
}

const expandedCatalogRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: Array.from({ length: 11 }, (_source, sourceIndex) => ({
      source_id: `expanded-source-${sourceIndex + 1}`,
      title: `Expanded catalog source ${sourceIndex + 1}`,
      url_or_path: `https://expanded-${sourceIndex + 1}.example.test/source`,
      reliability: "Fetched source text.",
      chunks: Array.from({ length: 120 }, (_chunk, chunkIndex) => ({
        chunk_id: `expanded-source-${sourceIndex + 1}:chunk:${chunkIndex + 1}`,
        text: `Bounded evidence ${sourceIndex + 1}.${chunkIndex + 1}.`,
      })),
    })),
  },
  errors: [],
  metadata: {},
};
const expandedCatalogRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Reduce a complete expanded evidence catalog",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      retrieve_web_source_1: expandedCatalogRetrieval,
    },
    step_failures: {},
  },
);
assert.equal(expandedCatalogRequest.type, "schedule_steps");
assert.ok(expandedCatalogRequest.steps.length > 0);
assert.ok(
  expandedCatalogRequest.steps.every((step) =>
    step.step_id.startsWith("select_evidence_chunks_shard_")
  ),
);
const expandedShardChunkIds = expandedCatalogRequest.steps.flatMap((step) => {
  assert.ok(
    Buffer.byteLength(step.input.prompt, "utf8") < 128 * 1024,
    `${step.step_id} exceeded the runtime prompt byte ceiling`,
  );
  const packet = JSON.parse(
    step.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
  );
  return packet.sources.flatMap((source) =>
    source.chunks.map((chunk) => chunk.chunk_id)
  );
});
assert.equal(expandedShardChunkIds.length, 11 * 120);
assert.equal(new Set(expandedShardChunkIds).size, expandedShardChunkIds.length);

const failedOptimisticShard = largeShardRequest.steps[0];
const completedOptimisticShardOutputs = Object.fromEntries(
  largeShardRequest.steps.slice(1).map((step) => [
    step.step_id,
    {
      output: JSON.stringify({
        object: {
          chunk_ids: [],
          source_coverage: [],
          source_relevance: [],
        },
      }),
    },
  ]),
);
const largeShardRecoveryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Recover one failed optimistic evidence shard",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      retrieve_web_source_1: largeShardRetrieval,
      ...completedOptimisticShardOutputs,
    },
    step_failures: {
      [failedOptimisticShard.step_id]: {
        error: "simulated optimistic shard failure",
      },
    },
  },
);
assert.equal(largeShardRecoveryRequest.type, "schedule_steps");
assert.ok(largeShardRecoveryRequest.steps.length > 1);
assert.ok(
  largeShardRecoveryRequest.steps.every((step) =>
    step.step_id.startsWith("select_evidence_chunks_shard_recovery_1_")
  ),
);
const failedOptimisticPacket = JSON.parse(
  failedOptimisticShard.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
);
const failedOptimisticChunkIds = failedOptimisticPacket.sources.flatMap(
  (source) => source.chunks.map((chunk) => chunk.chunk_id),
);
const recoveredOptimisticChunkIds = largeShardRecoveryRequest.steps.flatMap(
  (step) => {
    assert.ok(
      Buffer.byteLength(step.input.prompt, "utf8") < 48 * 1024,
      `${step.step_id} exceeded the recovery generation prompt budget`,
    );
    const packet = JSON.parse(
      step.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
    );
    return packet.sources.flatMap((source) =>
      source.chunks.map((chunk) => chunk.chunk_id)
    );
  },
);
assert.equal(
  recoveredOptimisticChunkIds.sort().join("\n"),
  failedOptimisticChunkIds.sort().join("\n"),
);
assert.equal(
  new Set(recoveredOptimisticChunkIds).size,
  recoveredOptimisticChunkIds.length,
);
const reliableShardSourceCount = 11;
const reliableShardChunksPerSource = 6;
const reliableShardRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: Array.from(
      { length: reliableShardSourceCount },
      (_source, sourceIndex) => ({
        source_id: `reliable-source-${sourceIndex + 1}`,
        title: `Reliable shard source ${sourceIndex + 1}`,
        url_or_path:
          `https://reliable-${sourceIndex + 1}.example.test/source`,
        reliability: "Fetched source text.",
        chunks: Array.from(
          { length: reliableShardChunksPerSource },
          (_chunk, chunkIndex) => ({
            chunk_id:
              `reliable-source-${sourceIndex + 1}:chunk:${chunkIndex + 1}`,
            text:
              `Bounded source ${sourceIndex + 1}.${chunkIndex + 1} ` +
              "r".repeat(660),
          }),
        ),
      }),
    ),
  },
  errors: [],
  metadata: {},
};
const reliableShardRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select a reliable complete cross-source catalog",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...referenceBaseOutputs,
      retrieve_web_source_1: reliableShardRetrieval,
    },
    step_failures: {},
  },
);
assert.equal(reliableShardRequest.type, "schedule_steps");
assert.ok(reliableShardRequest.steps.length > 1);
assert.ok(reliableShardRequest.steps.length < reliableShardSourceCount);
const reliableShardChunkIds = reliableShardRequest.steps.flatMap((step) => {
  assert.ok(
    Buffer.byteLength(step.input.prompt, "utf8") < 48 * 1024,
    `${step.step_id} exceeded the reliable generation prompt budget`,
  );
  const packet = JSON.parse(
    step.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
  );
  assert.ok(packet.sources.length <= 4);
  return packet.sources.flatMap((source) =>
    source.chunks.map((chunk) => chunk.chunk_id)
  );
});
assert.equal(
  reliableShardChunkIds.length,
  reliableShardSourceCount * reliableShardChunksPerSource,
);
assert.equal(
  new Set(reliableShardChunkIds).size,
  reliableShardChunkIds.length,
);

const packedShardSourceCount = 12;
const packedShardRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: Array.from(
      { length: packedShardSourceCount },
      (_value, sourceIndex) => ({
        source_id: `web-source-${sourceIndex + 1}`,
        title: `Small fetched source ${sourceIndex + 1}`,
        url_or_path:
          `https://packed-${sourceIndex + 1}.example.test/source`,
        reliability: "Fetched source text.",
        chunks: Array.from({ length: 2 }, (_chunk, chunkIndex) => ({
          chunk_id:
            `web-source-${sourceIndex + 1}:chunk:${chunkIndex + 1}`,
          text:
            `Independent evidence ${sourceIndex + 1}.${chunkIndex + 1}.`,
        })),
      }),
    ),
  },
  errors: [],
  metadata: {},
};
const packedShardBaseOutputs = {
  ...referenceBaseOutputs,
  retrieve_web_source_1: packedShardRetrieval,
};
const packedShardRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from several small fetched sources",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: packedShardBaseOutputs,
    step_failures: {},
  },
);
assert.equal(packedShardRequest.type, "schedule_steps");
assert.ok(packedShardRequest.steps.length > 0);
assert.ok(packedShardRequest.steps.length < packedShardSourceCount);
const packedShardOutputs = {};
for (const step of packedShardRequest.steps) {
  assert.ok(
    Buffer.byteLength(step.input.prompt, "utf8") < 128 * 1024,
    `${step.step_id} exceeded the runtime prompt byte ceiling`,
  );
  const packet = JSON.parse(
    step.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
  );
  const distinctSourceCount = new Set(
    packet.sources.map((source) => source.source_id),
  ).size;
  const chunkCount = packet.sources.reduce(
    (total, source) => total + source.chunks.length,
    0,
  );
  assert.equal(
    step.input.schema.properties.chunk_ids.maxItems,
    Math.min(4 * distinctSourceCount, chunkCount),
  );
  const focus = packet.focuses[0];
  const selectedChunks = packet.sources.flatMap((source) => source.chunks);
  packedShardOutputs[step.step_id] = {
    output: JSON.stringify({
      object: {
        chunk_ids: selectedChunks.map((chunk) => chunk.chunk_id),
        source_coverage: [],
        source_relevance: packet.sources.map((source) => ({
          source_id: source.source_id,
          obligation_id: focus.obligation_id,
        })),
      },
    }),
  };
}
assert.ok(
  Object.values(packedShardOutputs).some((output) =>
    JSON.parse(output.output).object.chunk_ids.length > 4
  ),
);
const packedShardReduction = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from several small fetched sources",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...packedShardBaseOutputs,
      ...packedShardOutputs,
    },
    step_failures: {},
  },
);
assert.equal(packedShardReduction.type, "schedule_step");
assert.equal(packedShardReduction.step_id, "checkpoint_initial_retrieval");

const referencePacket = JSON.parse(
  referenceSelectionRequest.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
);
const referenceSource = referencePacket.sources[0];
const referenceFocus = referencePacket.focuses[0];
const referenceGapQueryRequest = await sandbox.__deepResearchRun(
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
    },
    step_failures: {},
  },
);
assert.equal(referenceGapQueryRequest.type, "schedule_step");
assert.equal(
  referenceGapQueryRequest.step_id,
  "generate_gap_queries",
);
assert.ok(
  referenceGapQueryRequest.input.prompt.includes(
    "Generate new search queries from the typed evidence gaps",
  ),
);
assert.ok(
  referenceGapQueryRequest.input.prompt.includes(
    "Do not use site:, Boolean OR",
  ),
);
assert.ok(
  referenceGapQueryRequest.input.prompt.includes(
    "one missing criterion, and one likely original record type",
  ),
);
assert.ok(
  referenceGapQueryRequest.input.prompt.includes(
    "Do not invent a report, dataset, audit, case",
  ),
);
assert.ok(
  referenceGapQueryRequest.input.prompt.includes(
    "failed earlier search is never evidence of non-disclosure",
  ),
);
assert.equal(
  referenceGapQueryRequest.input.schema.properties.queries.maxItems,
  4,
);
assert.equal(referenceGapQueryRequest.input.timeout_ms, 180_000);
assert.equal(referenceGapQueryRequest.retry.max_attempts, 1);

const deterministicGapFallbackRequest = await sandbox.__deepResearchRun(
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
    },
    step_failures: {
      generate_gap_queries: {
        error: "simulated gap-query generation timeout",
      },
    },
  },
);
assert.equal(deterministicGapFallbackRequest.type, "schedule_step");
assert.equal(
  deterministicGapFallbackRequest.step_id,
  "discover_gap_web_sources",
);
assert.deepEqual(
  Array.from(deterministicGapFallbackRequest.input.plan.search_queries),
  [referencePlan.tracks[0].completion_criteria[0]],
);
