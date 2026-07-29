for (const [stepId, expectedStage] of [
  ["generate_gap_queries", "Typed gap-query generation"],
  [
    "select_supplemental_web_sources_2",
    "Semantic supplemental web source selection",
  ],
]) {
  await assert.rejects(
    sandbox.__deepResearchRun(
      {
        async tool(name) {
          assert.equal(name, "generate_object");
          return { exitCode: 1, output: "simulated typed generation failure" };
        },
      },
      {
        kind: "step",
        step_id: stepId,
        step_name: "generate_object",
        input: {},
      },
    ),
    new RegExp(expectedStage),
  );
}

async function discover(searchOutput) {
  const ctx = {
    async tool(name, request) {
      assert.equal(name, "batch");
      assert.equal(request.invocations.length, 1);
      const invocation = request.invocations[0];
      assert.equal(invocation.args.full_text_bytes, 8 * 1024);
      const outputBytes = new TextEncoder().encode(searchOutput).byteLength;
      return {
        output: `--- [1: ${invocation.tool} · ${invocation.id}] ---\n${searchOutput}\n`,
        metadata: {
          results: [
            {
              index: 0,
              id: invocation.id,
              tool: invocation.tool,
              success: true,
              output_bytes: outputBytes,
              metadata: {
                selected_engines: ["test-provider"],
                engine_selection_source: "test",
              },
            },
          ],
        },
      };
    },
  };
  return sandbox.__deepResearchRun(ctx, {
    kind: "step",
    step_name: "discover_web_sources",
    input: {
      plan: {
        budget: {
          direct_searches: 1,
          direct_fetches: 4,
        },
        search_queries: ["structural workflow smoke query"],
      },
      search_timeout_secs: 1,
    },
  });
}

const structured = await discover(
  JSON.stringify([
    {
      title: "First result",
      url: "https://one.example.test/record?revision=7",
      full_text: "Provider-returned source text that is long enough for closed evidence review.",
    },
    {
      title: "Second result",
      url: "https://two.example.test/record?view=complete",
    },
  ]),
);
assert.equal(structured.status, "success");
assert.equal(structured.candidates.length, 2);
assert.equal(
  structured.candidates[0].url,
  "https://one.example.test/record?revision=7",
);
assert.equal(
  structured.candidates[0].provider_text,
  "Provider-returned source text that is long enough for closed evidence review.",
);

const providerSnapshotFailure = "direct transport unavailable";
const providerSnapshotResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "batch");
      assert.equal(request.invocations.length, 1);
      const invocation = request.invocations[0];
      return {
        output:
          `--- [1: ${invocation.tool} · ${invocation.id}] ---\n` +
          `ERROR: ${providerSnapshotFailure}\n`,
        metadata: {
          results: [{
            index: 0,
            id: invocation.id,
            tool: invocation.tool,
            success: false,
            output_bytes: new TextEncoder().encode(providerSnapshotFailure)
              .byteLength,
          }],
        },
      };
    },
  },
  {
    kind: "step",
    step_name: "retrieve_web_source",
    input: {
      plan: {
        tracks: [{
          id: "transport-neutral",
          title: "Transport-neutral evidence",
          focus: "Assess one traceable record.",
          material: true,
          questions: ["What does the record establish?"],
          completion_criteria: ["The record establishes the requested fact."],
          evidence_requirements: {
            primary_source_required: false,
            independent_corroboration_required: false,
          },
        }],
      },
      candidates: [{
        candidate_id: "candidate-1",
        title: "Traceable record",
        url: "https://snapshot.example.test/record",
        engines: ["anonymous-provider"],
        provider_text:
          "This provider-returned source text contains a traceable substantive record for semantic review.",
      }],
      source_id_prefix: "snapshot-source",
      source_index_offset: 0,
      fetch_timeout_secs: 1,
    },
  },
);
assert.equal(providerSnapshotResult.packet.sources.length, 1);
assert.match(
  providerSnapshotResult.packet.sources[0].reliability,
  /Search-provider source-text snapshot/,
);
assert.match(
  providerSnapshotResult.packet.sources[0].chunks[0].text,
  /traceable substantive record/,
);
assert.equal(providerSnapshotResult.metadata.provider_full_text_count, 1);

const preferredProviderSnapshotResult = await sandbox.__deepResearchRun(
  {
    async tool() {
      assert.fail("a preferred provider snapshot must not start direct transport");
    },
  },
  {
    kind: "step",
    step_name: "retrieve_web_source",
    input: {
      plan: {
        tracks: [{
          id: "bounded-fallback",
          title: "Bounded fallback evidence",
          focus: "Review one bounded discovery fallback source.",
          material: true,
          questions: ["What does the bounded source establish?"],
          completion_criteria: ["The source establishes one traceable fact."],
          evidence_requirements: {
            primary_source_required: false,
            independent_corroboration_required: false,
          },
        }],
      },
      candidates: [{
        candidate_id: "candidate-1",
        title: "Bounded provider record",
        url: "https://snapshot.example.test/bounded-record",
        engines: ["anonymous-provider"],
        provider_text:
          "This bounded provider snapshot contains enough traceable source text for the later closed semantic review.",
      }],
      source_id_prefix: "bounded-source",
      source_index_offset: 0,
      fetch_timeout_secs: 1,
      prefer_bounded_snapshot: true,
    },
  },
);
assert.equal(preferredProviderSnapshotResult.status, "success");
assert.equal(
  preferredProviderSnapshotResult.metadata.preferred_provider_full_text_count,
  1,
);
assert.equal(preferredProviderSnapshotResult.metadata.document_range_count, 0);
assert.match(
  preferredProviderSnapshotResult.packet.sources[0].chunks[0].text,
  /bounded provider snapshot/,
);
assert.match(
  preferredProviderSnapshotResult.packet.sources[0].reliability,
  /deterministic bounded candidate admission/,
);

const boundedDirectInitialText = "B".repeat(9_000);
let boundedDirectBatchCalls = 0;
const boundedDirectSnapshotResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "batch");
      boundedDirectBatchCalls += 1;
      assert.equal(request.invocations.length, 1);
      assert.equal(request.invocations[0].args.offset, undefined);
      return {
        output:
          `--- [1: web_fetch · fetch-1] ---\n${boundedDirectInitialText}\n`,
        metadata: {
          results: [{
            index: 0,
            id: "fetch-1",
            tool: "web_fetch",
            success: true,
            output_bytes: Buffer.byteLength(boundedDirectInitialText, "utf8"),
            metadata: {
              content_type: "text/html",
              range: {
                offset: 0,
                returned_chars: boundedDirectInitialText.length,
                next_offset: boundedDirectInitialText.length,
                eof: false,
              },
            },
          }],
        },
      };
    },
  },
  {
    kind: "step",
    step_name: "retrieve_web_source",
    input: {
      plan: {
        tracks: [{
          id: "bounded-direct-fallback",
          title: "Bounded direct fallback evidence",
          focus: "Review one bounded initial source range.",
          material: true,
          questions: ["What does the bounded initial range establish?"],
          completion_criteria: ["The initial range establishes one fact."],
          evidence_requirements: {
            primary_source_required: false,
            independent_corroboration_required: false,
          },
        }],
      },
      candidates: [{
        candidate_id: "candidate-1",
        title: "Bounded direct record",
        url: "https://direct.example.test/bounded-record",
        engines: ["anonymous-provider"],
        provider_text: "",
      }],
      source_id_prefix: "bounded-direct-source",
      source_index_offset: 0,
      fetch_timeout_secs: 1,
      prefer_bounded_snapshot: true,
    },
  },
);
assert.equal(boundedDirectSnapshotResult.status, "success");
assert.equal(boundedDirectBatchCalls, 1);
assert.equal(boundedDirectSnapshotResult.metadata.bounded_direct_text_count, 1);
assert.equal(boundedDirectSnapshotResult.metadata.document_range_count, 1);
assert.equal(boundedDirectSnapshotResult.packet.sources[0].chunks.length, 12);
assert.equal(
  boundedDirectSnapshotResult.packet.sources[0].chunks.reduce(
    (total, chunk) => total + Array.from(chunk.text).length,
    0,
  ),
  8 * 1024,
);
assert.match(
  boundedDirectSnapshotResult.packet.sources[0].reliability,
  /Bounded initial fetched source-text snapshot/,
);

const completeDirectRanges = ["F".repeat(9_000), "S".repeat(1_000)];
const completeDirectOffsets = [];
const completeDirectFetchResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "batch");
      assert.equal(request.invocations.length, 1);
      const invocation = request.invocations[0];
      const offset = invocation.args.offset ?? 0;
      completeDirectOffsets.push(offset);
      const rangeIndex = offset === 0 ? 0 : 1;
      const output = completeDirectRanges[rangeIndex];
      const eof = rangeIndex === completeDirectRanges.length - 1;
      return {
        output: `--- [1: web_fetch · ${invocation.id}] ---\n${output}\n`,
        metadata: {
          results: [{
            index: 0,
            id: invocation.id,
            tool: "web_fetch",
            success: true,
            output_bytes: Buffer.byteLength(output, "utf8"),
            metadata: {
              content_type: "text/html",
              range: {
                offset,
                returned_chars: output.length,
                next_offset: eof ? null : offset + output.length,
                eof,
              },
            },
          }],
        },
      };
    },
  },
  {
    kind: "step",
    step_name: "retrieve_web_source",
    input: {
      plan: {
        tracks: [{
          id: "complete-direct-fetch",
          title: "Complete direct evidence",
          focus: "Review every available source range.",
          material: true,
          questions: ["What does the complete source establish?"],
          completion_criteria: ["The complete source establishes one fact."],
          evidence_requirements: {
            primary_source_required: false,
            independent_corroboration_required: false,
          },
        }],
      },
      candidates: [{
        candidate_id: "candidate-1",
        title: "Complete direct record",
        url: "https://direct.example.test/complete-record",
        engines: ["anonymous-provider"],
        provider_text: "",
      }],
      source_id_prefix: "complete-direct-source",
      source_index_offset: 0,
      fetch_timeout_secs: 1,
      prefer_bounded_snapshot: false,
    },
  },
);
assert.equal(completeDirectFetchResult.status, "success");
assert.deepEqual(completeDirectOffsets, [0, 9_000]);
assert.equal(completeDirectFetchResult.metadata.bounded_direct_text_count, 0);
assert.equal(completeDirectFetchResult.metadata.document_range_count, 2);
assert.equal(
  completeDirectFetchResult.packet.sources[0].chunks.reduce(
    (total, chunk) => total + Array.from(chunk.text).length,
    0,
  ),
  10_000,
);
assert.doesNotMatch(
  completeDirectFetchResult.packet.sources[0].reliability,
  /Bounded initial/,
);

const maximumSearchQueries = Array.from(
  { length: 16 },
  (_, index) => `bounded discovery query ${index + 1}`,
);
const maximumSearchOutputs = maximumSearchQueries.map((_query, queryIndex) =>
  JSON.stringify(
    Array.from({ length: 16 }, (_, resultIndex) => ({
      title: `Result ${queryIndex + 1}-${resultIndex + 1}`,
      url:
        `https://wide-${queryIndex + 1}.example.test/record-${resultIndex + 1}`,
    })),
  )
);
const maximumDiscovery = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "batch");
      assert.equal(request.invocations.length, 16);
      return {
        output: request.invocations
          .map(
            (invocation, index) =>
              `--- [${index + 1}: ${invocation.tool} · ${invocation.id}] ---\n` +
              `${maximumSearchOutputs[index]}\n`,
          )
          .join(""),
        metadata: {
          results: request.invocations.map((invocation, index) => ({
            index,
            id: invocation.id,
            tool: invocation.tool,
            success: true,
            output_bytes: new TextEncoder().encode(
              maximumSearchOutputs[index],
            ).byteLength,
            metadata: {
              selected_engines: ["test-provider"],
              engine_selection_source: "test",
            },
          })),
        },
      };
    },
  },
  {
    kind: "step",
    step_name: "discover_web_sources",
    input: {
      plan: {
        budget: {
          direct_searches: 16,
          direct_fetches: 24,
        },
        search_queries: maximumSearchQueries,
        seed_urls: [
          "https://seed-one.example.test/",
          "https://seed-two.example.test/",
          "https://seed-three.example.test/",
        ],
      },
      search_timeout_secs: 1,
    },
  },
);
assert.equal(maximumDiscovery.status, "success");
assert.equal(maximumDiscovery.metadata.provider_candidate_count, 256);
assert.equal(maximumDiscovery.metadata.candidate_count, 259);
assert.equal(maximumDiscovery.candidates.length, 259);
assert.equal(
  maximumDiscovery.candidates.at(-1).url,
  "https://wide-16.example.test/record-16",
);

const maximumSelectionPlan = {
  tracks: Array.from({ length: 8 }, (_track, trackIndex) => ({
    id: `maximum-track-${trackIndex + 1}`,
    title: `Maximum selector track ${trackIndex + 1}`,
    focus: `Resolve every bounded criterion for track ${trackIndex + 1}.`,
    material: true,
    questions: Array.from(
      { length: 4 },
      (_question, questionIndex) =>
        `Which source resolves question ${questionIndex + 1} for track ${trackIndex + 1}?`,
    ),
    completion_criteria: Array.from(
      { length: 3 },
      (_criterion, criterionIndex) =>
        `Criterion ${criterionIndex + 1} for track ${trackIndex + 1} is established.`,
    ),
    evidence_requirements: {
      primary_source_required: true,
      independent_corroboration_required: true,
    },
  })),
  search_queries: maximumSearchQueries,
  seed_urls: [
    "https://seed-one.example.test/",
    "https://seed-two.example.test/",
    "https://seed-three.example.test/",
  ],
  budget: {
    direct_searches: 16,
    direct_fetches: 24,
  },
};
const maximumSelectorRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from the complete maximum discovery catalog",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: maximumSelectionPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: { discover_web_sources: maximumDiscovery },
    step_failures: {},
  },
);
assert.equal(maximumSelectorRequest.type, "schedule_steps");
assert.ok(maximumSelectorRequest.steps.length > 1);
assert.ok(
  maximumSelectorRequest.steps.every((step) =>
    step.step_id.startsWith("select_web_sources_shard_")
  ),
);
const maximumSelectorCandidateIds = [];
const maximumSelectorShardOutputs = {};
for (const step of maximumSelectorRequest.steps) {
  assert.ok(
    Buffer.byteLength(step.input.prompt, "utf8") <= 64 * 1024,
  );
  const packet = JSON.parse(
    step.input.prompt.split("CLOSED_WEB_DISCOVERY_PACKET=")[1],
  );
  assert.equal(packet.focuses.length, 8);
  assert.ok(packet.candidates.length <= 24);
  assert.equal(
    packet.candidate_metadata_projection.all_candidate_identities_preserved,
    true,
  );
  assert.ok(
    packet.candidate_metadata_projection.uniform_scale_percent > 0,
  );
  maximumSelectorCandidateIds.push(
    ...packet.candidates.map((candidate) => candidate.candidate_id),
  );
  maximumSelectorShardOutputs[step.step_id] = {
    output: JSON.stringify({
      object: {
        candidate_ids: packet.candidates
          .slice(0, step.input.schema.properties.candidate_ids.maxItems)
          .map((candidate) => candidate.candidate_id),
      },
    }),
  };
}
assert.equal(maximumSelectorCandidateIds.length, 259);
assert.equal(new Set(maximumSelectorCandidateIds).size, 259);
const maximumSelectorReductionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from the complete maximum discovery catalog",
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: maximumSelectionPlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      discover_web_sources: maximumDiscovery,
      ...maximumSelectorShardOutputs,
    },
    step_failures: {},
  },
);
assert.equal(maximumSelectorReductionRequest.type, "schedule_step");
assert.equal(
  maximumSelectorReductionRequest.step_id,
  "select_web_sources",
);
assert.ok(
  maximumSelectorReductionRequest.input.schema.properties.candidate_ids.items
    .enum.length > maximumSelectionPlan.budget.direct_fetches,
);
assert.ok(
  Buffer.byteLength(
    maximumSelectorReductionRequest.input.prompt,
    "utf8",
  ) <= 64 * 1024,
);

const recoveredSearchOutputs = [
  JSON.stringify([{
    title: "Recovered first result",
    url: "https://recovered-one.example.test/record",
  }]),
  JSON.stringify([{
    title: "Recovered second result",
    url: "https://recovered-two.example.test/record",
  }]),
];
let searchBatchCalls = 0;
const recoveredDiscovery = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "batch");
      searchBatchCalls += 1;
      if (request.invocations.length === 2) {
        const first = request.invocations[0];
        return {
          output:
            `--- [1: ${first.tool} · ${first.id}] ---\n` +
            recoveredSearchOutputs[0].slice(0, 12),
          metadata: {
            results: request.invocations.map((invocation, index) => ({
              index,
              id: invocation.id,
              tool: invocation.tool,
              success: true,
              output_bytes: new TextEncoder().encode(
                recoveredSearchOutputs[index],
              ).byteLength,
              metadata: {},
            })),
          },
        };
      }
      assert.equal(request.invocations.length, 1);
      const invocation = request.invocations[0];
      const outputIndex = Number(invocation.id.split("-").at(-1)) - 1;
      const output = recoveredSearchOutputs[outputIndex];
      return {
        output:
          `--- [1: ${invocation.tool} · ${invocation.id}] ---\n${output}\n`,
        metadata: {
          results: [{
            index: 0,
            id: invocation.id,
            tool: invocation.tool,
            success: true,
            output_bytes: new TextEncoder().encode(output).byteLength,
            metadata: {},
          }],
        },
      };
    },
  },
  {
    kind: "step",
    step_name: "discover_web_sources",
    input: {
      plan: {
        budget: {
          direct_searches: 2,
          direct_fetches: 2,
        },
        search_queries: ["recover first", "recover second"],
      },
      search_timeout_secs: 1,
    },
  },
);
assert.equal(recoveredDiscovery.status, "success");
assert.equal(recoveredDiscovery.candidates.length, 2);
assert.equal(
  recoveredDiscovery.metadata.search_batch_output_recovery_count,
  2,
);
assert.equal(searchBatchCalls, 3);

const unstructured = await discover(
  "https://three.example.test/record?format=full",
);
assert.equal(unstructured.status, "failed");
assert.equal(unstructured.candidates.length, 0);
