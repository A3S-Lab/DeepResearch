const mergedSelection = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Combine independently collected evidence",
      execution_mode: "collect_only",
      evidence_scope: "local_only",
      research_plan: {
        report_title: "Independent packet identity",
        research_scope: "focused",
        freshness_required: false,
        workspace_evidence_required: true,
        tracks: [
          {
            id: "workspace",
            title: "Workspace",
            focus: "Combine independently collected evidence",
            material: true,
            questions: ["Which evidence answers the request?"],
            completion_criteria: ["Traceable evidence answers the request."],
            evidence_requirements: {
              primary_source_required: false,
              independent_corroboration_required: false,
            },
          },
        ],
        search_queries: [],
        seed_urls: [],
        stop_conditions: [],
        budget: {
          retrieval_timeout_ms: 30_000,
          direct_searches: 0,
          direct_fetches: 0,
        },
      },
      bootstrap_acquisition: {
        status: "success",
        packet: {
          version: 1,
          focuses: [],
          sources: [
            {
              source_id: "local-source-1",
              title: "src/bootstrap.rs",
              url_or_path: "src/bootstrap.rs",
              reliability: "Host-restored bootstrap evidence.",
              chunks: [
                {
                  chunk_id: "local-source-1:chunk:1",
                  text: "Bootstrap evidence.",
                },
              ],
            },
          ],
        },
        errors: [],
        metadata: {},
      },
    },
    step_outputs: {
      retrieve_local: {
        status: "success",
        packet: {
          version: 1,
          focuses: [],
          sources: [
            {
              source_id: "local-source-1",
              title: "src/followup.rs",
              url_or_path: "src/followup.rs",
              reliability: "Host-restored follow-up evidence.",
              chunks: [
                {
                  chunk_id: "local-source-1:chunk:1",
                  text: "Follow-up evidence.",
                },
              ],
            },
          ],
        },
        errors: [],
        metadata: {},
      },
    },
    step_failures: {},
  },
);
assert.equal(mergedSelection.type, "schedule_step");
assert.equal(mergedSelection.step_id, "select_evidence_chunks");
assert.match(
  mergedSelection.input.prompt,
  /A workspace source establishes its contents, but not that it belongs to an active build/,
);
const mergedPacket = JSON.parse(
  mergedSelection.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
);
assert.equal(mergedPacket.sources.length, 2);
assert.equal(
  new Set(mergedPacket.sources.map((source) => source.source_id)).size,
  2,
);
assert.equal(
  new Set(
    mergedPacket.sources.flatMap((source) =>
      source.chunks.map((chunk) => chunk.chunk_id),
    ),
  ).size,
  2,
);
function successfulReadBatch(request) {
  const sections = request.invocations.map((invocation, index) => {
    const offset = invocation.args.offset;
    const output =
      `${offset + 1}\tEvidence from ${invocation.args.file_path} at ${offset}`;
    return {
      text: `--- [${index + 1}: ${invocation.tool} · ${invocation.id}] ---\n${output}\n`,
      metadata: {
        index,
        id: invocation.id,
        tool: invocation.tool,
        success: true,
        output_bytes: new TextEncoder().encode(output).byteLength,
        metadata: {
          source_anchors: [invocation.args.file_path],
          range: {
            offset,
            returned_lines: 1,
          },
        },
      },
    };
  });
  return {
    output: sections.map((section) => section.text).join(""),
    metadata: {
      results: sections.map((section) => section.metadata),
    },
  };
}

const localTaskRequests = [];
await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      assert.equal(name, "task");
      localTaskRequests.push(request);
      if (localTaskRequests.length === 1) {
        throw new Error("first track failed");
      }
      return { metadata: { results: [] } };
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Inspect several material workspace tracks",
      max_steps: 4,
      plan: {
        tracks: [
          {
            id: "track-1",
            title: "First track",
            focus: "Inspect the first material track.",
            material: true,
          },
          {
            id: "track-2",
            title: "Second track",
            focus: "Inspect the second material track.",
            material: true,
          },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.equal(localTaskRequests.length, 2);
for (const [index, task] of localTaskRequests.entries()) {
  assert.equal(task.max_steps, 5);
  assert.equal(
    task.output_schema.properties.sources.maxItems,
    8,
  );
  assert.match(
    task.prompt,
    /at most 8 paths and at most 3 non-overlapping ranges per path/,
  );
  assert.match(
    task.prompt,
    /must contain only an exact file path copied from a successful tool result/,
  );
  assert.match(
    task.prompt,
    /Follow requested transitions through concrete call sites/,
  );
  assert.match(
    task.prompt,
    /Similar code in another tree and path names alone do not establish/,
  );
  assert.match(task.prompt, new RegExp(`track-${index + 1}`));
  assert.doesNotMatch(task.prompt, new RegExp(`track-${2 - index}`));
}

let localDiscoveryTask = 0;
let localReadPaths = [];
const fairLocalResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      if (name === "task") {
        localDiscoveryTask += 1;
        const prefix = localDiscoveryTask === 1 ? "a" : "b";
        return {
          metadata: {
            success: true,
            source_anchors: [{
              tool: "read",
              url_or_path: `src/${prefix}-5.rs`,
            }],
            structured: {
              sources: Array.from({ length: 5 }, (_, index) => ({
                url_or_path: `src/${prefix}-${index + 1}.rs`,
                ranges: [{ offset: 0, limit: 1 }],
              })),
            },
          },
        };
      }
      assert.equal(name, "batch");
      localReadPaths = request.invocations.map(
        (invocation) => invocation.args.file_path,
      );
      return successfulReadBatch(request);
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Inspect two independent workspace tracks",
      max_steps: 4,
      plan: {
        tracks: [
          {
            id: "track-a",
            title: "Track A",
            focus: "Inspect track A.",
            material: true,
          },
          {
            id: "track-b",
            title: "Track B",
            focus: "Inspect track B.",
            material: true,
          },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.deepEqual(Array.from(localReadPaths), [
  "src/a-5.rs",
  "src/b-5.rs",
  "src/a-1.rs",
  "src/b-1.rs",
  "src/a-2.rs",
  "src/b-2.rs",
  "src/a-3.rs",
  "src/b-3.rs",
]);
assert.equal(fairLocalResult.packet.sources.length, 8);
assert.equal(fairLocalResult.metadata.observed_source_count, 8);

let hintedReadPaths = [];
const hintedLocalResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      if (name === "task") {
        assert.match(request.prompt, /docs\/explicit-context\.md/);
        return { metadata: { results: [] } };
      }
      assert.equal(name, "batch");
      hintedReadPaths = request.invocations.map(
        (invocation) => invocation.args.file_path,
      );
      assert.equal(request.invocations[0].args.offset, 0);
      assert.equal(request.invocations[0].args.limit, 240);
      return successfulReadBatch(request);
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Use the explicit context",
      max_steps: 4,
      source_hints: [{ path: "docs/explicit-context.md" }],
      plan: {
        tracks: [
          { id: "workspace", focus: "Inspect explicit context.", material: true },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.deepEqual(Array.from(hintedReadPaths), ["docs/explicit-context.md"]);
assert.equal(hintedLocalResult.metadata.hinted_source_count, 1);
assert.equal(hintedLocalResult.metadata.restored_hint_count, 1);
assert.equal(
  hintedLocalResult.packet.sources[0].url_or_path,
  "docs/explicit-context.md",
);

let duplicateRangeTask = 0;
let duplicateReadOffsets = [];
const duplicateRangeResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      if (name === "task") {
        const firstOffset = duplicateRangeTask * 3;
        duplicateRangeTask += 1;
        return {
          metadata: {
            success: true,
            structured: {
              sources: [{
                url_or_path: "src/shared.rs",
                ranges: Array.from({ length: 3 }, (_, index) => ({
                  offset: firstOffset + index,
                  limit: 1,
                })),
              }],
            },
          },
        };
      }
      assert.equal(name, "batch");
      duplicateReadOffsets = request.invocations.map(
        (invocation) => invocation.args.offset,
      );
      return successfulReadBatch(request);
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Inspect one source proposed by two independent tracks",
      max_steps: 4,
      plan: {
        tracks: [
          { id: "track-a", focus: "Inspect track A.", material: true },
          { id: "track-b", focus: "Inspect track B.", material: true },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.deepEqual(Array.from(duplicateReadOffsets), [0, 1, 2]);
assert.equal(duplicateRangeResult.packet.sources.length, 1);
assert.equal(duplicateRangeResult.metadata.observed_source_count, 1);
assert.equal(duplicateRangeResult.metadata.requested_range_count, 3);
assert.equal(duplicateRangeResult.metadata.read_range_count, 3);
assert.ok(
  duplicateRangeResult.errors.includes(
    "Local retrieval omitted duplicate-source ranges beyond the closed per-source range limit of 3.",
  ),
);

const partialRestoreResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      if (name === "task") {
        return {
          metadata: {
            success: true,
            structured: {
              sources: [
                {
                  url_or_path: "src/restored.rs",
                  ranges: [{ offset: 0, limit: 1 }],
                },
                {
                  url_or_path: "src/unrestored.rs",
                  ranges: [{ offset: 0, limit: 1 }],
                },
              ],
            },
          },
        };
      }
      assert.equal(name, "batch");
      const batch = successfulReadBatch(request);
      batch.metadata.results[1].success = false;
      return batch;
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Count only workspace sources restored by the host",
      max_steps: 4,
      plan: {
        tracks: [
          { id: "workspace", focus: "Inspect workspace evidence.", material: true },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.equal(partialRestoreResult.packet.sources.length, 1);
assert.equal(partialRestoreResult.metadata.observed_source_count, 1);
assert.equal(partialRestoreResult.metadata.requested_range_count, 2);
assert.equal(partialRestoreResult.metadata.read_range_count, 1);

let truncatedReadBatchCalls = 0;
const recoveredReadResult = await sandbox.__deepResearchRun(
  {
    async tool(name, request) {
      if (name === "task") {
        return {
          metadata: {
            success: true,
            structured: {
              sources: [
                {
                  url_or_path: "src/first.rs",
                  ranges: [{ offset: 0, limit: 1 }],
                },
                {
                  url_or_path: "src/second.rs",
                  ranges: [{ offset: 0, limit: 1 }],
                },
              ],
            },
          },
        };
      }
      assert.equal(name, "batch");
      truncatedReadBatchCalls += 1;
      const batch = successfulReadBatch(request);
      if (request.invocations.length === 2) {
        batch.output = batch.output.split("--- [2:")[0];
      }
      return batch;
    },
  },
  {
    kind: "step",
    step_name: "retrieve_local",
    input: {
      query: "Recover a truncated local read batch",
      max_steps: 4,
      plan: {
        tracks: [
          { id: "workspace", focus: "Inspect workspace evidence.", material: true },
        ],
        stop_conditions: [],
      },
    },
  },
);
assert.equal(truncatedReadBatchCalls, 2);
assert.equal(recoveredReadResult.status, "success");
assert.equal(recoveredReadResult.packet.sources.length, 2);
assert.equal(recoveredReadResult.metadata.observed_source_count, 2);
assert.equal(recoveredReadResult.metadata.requested_range_count, 2);
assert.equal(recoveredReadResult.metadata.read_range_count, 2);

const splitCoveragePlan = {
  report_title: "Split criterion coverage",
  research_scope: "comprehensive",
  freshness_required: false,
  workspace_evidence_required: true,
  tracks: [{
    id: "workspace",
    title: "Workspace",
    focus: "Trace one source across two completion criteria.",
    material: true,
    questions: ["Which source resolves both transitions?"],
    completion_criteria: [
      "The first transition is established.",
      "The second transition is established.",
    ],
    evidence_requirements: {
      primary_source_required: true,
      independent_corroboration_required: false,
    },
  }],
  search_queries: [],
  seed_urls: [],
  stop_conditions: [],
  budget: {
    retrieval_timeout_ms: 30_000,
    direct_searches: 0,
    direct_fetches: 0,
  },
};
const splitCoverageBootstrap = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "bootstrap-source",
      title: "src/runtime.rs",
      url_or_path: "src/runtime.rs",
      reliability: "Host-restored workspace evidence.",
      chunks: [{
        chunk_id: "bootstrap-source:chunk:1",
        text: "This source establishes both transitions.",
      }],
    }],
  },
  errors: [],
  metadata: {},
};
const splitCoverageLocal = {
  status: "failed",
  packet: null,
  errors: [],
  metadata: {},
};
const splitCoverageRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Trace both transitions",
      execution_mode: "collect_only",
      evidence_scope: "local_only",
      research_plan: splitCoveragePlan,
      bootstrap_acquisition: splitCoverageBootstrap,
    },
    step_outputs: {
      retrieve_local: splitCoverageLocal,
    },
    step_failures: {},
  },
);
assert.equal(splitCoverageRequest.type, "schedule_step");
assert.equal(splitCoverageRequest.step_id, "select_evidence_chunks");
const splitCoveragePacket = JSON.parse(
  splitCoverageRequest.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
);
const splitCoverageSource = splitCoveragePacket.sources[0];
const splitCoverageFocus = splitCoveragePacket.focuses[0];
const splitCoverageResult = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Trace both transitions",
      execution_mode: "collect_only",
      evidence_scope: "local_only",
      research_plan: splitCoveragePlan,
      bootstrap_acquisition: splitCoverageBootstrap,
    },
    step_outputs: {
      retrieve_local: splitCoverageLocal,
      select_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [splitCoverageSource.chunks[0].chunk_id],
            source_coverage: [
              {
                source_id: splitCoverageSource.source_id,
                obligation_id: splitCoverageFocus.obligation_id,
                completion_criterion_indexes: [0],
                roles: {
                  supporting: true,
                  primary: true,
                  independent: false,
                },
              },
              {
                source_id: splitCoverageSource.source_id,
                obligation_id: splitCoverageFocus.obligation_id,
                completion_criterion_indexes: [1],
                roles: {
                  supporting: true,
                  primary: true,
                  independent: false,
                },
              },
            ],
            source_relevance: [{
              source_id: splitCoverageSource.source_id,
              obligation_id: splitCoverageFocus.obligation_id,
            }],
          },
        }),
      },
    },
    step_failures: {},
  },
);
assert.equal(splitCoverageResult.type, "schedule_step");
assert.equal(splitCoverageResult.step_id, "checkpoint_initial_retrieval");
assert.equal(splitCoverageResult.input.research.metadata.source_count, 1);
assert.deepEqual(
  Array.from(
    splitCoverageResult.input.research.results[0]
      .structured.source_coverage[0].completion_criterion_indexes,
  ),
  [0, 1],
);
