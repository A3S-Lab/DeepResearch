import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

const workflowFiles = [
  "src/workflow/retrieval_foundation.js",
  "src/workflow/retrieval_web.js",
  "src/workflow/retrieval_selection.js",
  "src/workflow/retrieval_reduction.js",
  "src/workflow/retrieval_materialization.js",
  "src/workflow/retrieval_loop.js",
  "src/workflow/retrieval_local.js",
  "src/workflow/retrieval_local_collection.js",
  "src/workflow/retrieval_execution.js",
];

const sourceParts = await Promise.all(
  workflowFiles.map((path) => readFile(path, "utf8")),
);
const compactSource = sourceParts
  .join("")
  .split(/\r?\n/)
  .map((line) => line.trim())
  .filter((line) => line.length > 0 && !line.startsWith("//"))
  .map((line) => (/[;,{]$/.test(line) ? line : `${line}\n`))
  .join("");

const sandbox = {};
vm.runInNewContext(
  `${compactSource}\nglobalThis.__deepResearchRun = run;`,
  sandbox,
  { timeout: 5_000 },
);
assert.equal(typeof sandbox.__deepResearchRun, "function");

async function discover(searchOutput) {
  const ctx = {
    async tool(name, request) {
      assert.equal(name, "batch");
      assert.equal(request.invocations.length, 1);
      const invocation = request.invocations[0];
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

const maximumSearchQueries = Array.from(
  { length: 4 },
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
      assert.equal(request.invocations.length, 4);
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
          direct_searches: 4,
          direct_fetches: 8,
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
assert.equal(maximumDiscovery.metadata.provider_candidate_count, 64);
assert.equal(maximumDiscovery.metadata.candidate_count, 67);
assert.equal(maximumDiscovery.candidates.length, 67);
assert.equal(
  maximumDiscovery.candidates.at(-1).url,
  "https://wide-4.example.test/record-16",
);

const unstructured = await discover(
  "https://three.example.test/record?format=full",
);
assert.equal(unstructured.status, "failed");
assert.equal(unstructured.candidates.length, 0);

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
  const selectedChunks = packet.sources.map((source) => source.chunks[0]);
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
const referenceSupplementalRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
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
assert.equal(referenceSupplementalRequest.type, "schedule_step");
assert.equal(
  referenceSupplementalRequest.step_id,
  "select_supplemental_web_sources",
);
assert.ok(
  referenceSupplementalRequest.input.prompt.includes(
    "Do not collapse candidates that address different missing completion criteria",
  ),
);
const referenceSupplementalPacket = JSON.parse(
  referenceSupplementalRequest.input.prompt
    .split("CLOSED_SUPPLEMENTAL_DISCOVERY_PACKET=")[1],
);
assert.equal(
  referenceSupplementalPacket.candidates.filter(
    (candidate) =>
      candidate.url === "https://reference.example.test/primary",
  ).length,
  1,
);
assert.equal(
  referenceSupplementalPacket.candidates.filter(
    (candidate) =>
      candidate.url === "https://remaining.example.test/candidate",
  ).length,
  1,
);
assert.ok(
  !referenceSupplementalPacket.candidates.some(
    (candidate) =>
      candidate.url === "https://already.example.test/source" ||
      candidate.url === "http://unsafe.example.test/plain" ||
      candidate.url === "https://truncated.example.test/part" ||
      candidate.url === "https://unselected.example.test/ignored",
  ),
);

const directReferenceCandidate = referenceSupplementalPacket.candidates.find(
  (candidate) =>
    candidate.url === "https://reference.example.test/primary",
);
assert.ok(directReferenceCandidate);
const supplementalRetrieval = {
  status: "success",
  packet: {
    version: 1,
    focuses: [],
    sources: [{
      source_id: "web-source-1",
      title: "Direct supplemental record",
      url_or_path: "https://reference.example.test/primary",
      reliability: "Fetched source text.",
      chunks: [{
        chunk_id: "web-source-1:chunk:1",
        text: "The direct supplemental record fully resolves the material criterion.",
      }],
    }],
  },
  errors: [],
  metadata: {},
};
const supplementalSelectionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
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
      select_supplemental_web_sources: {
        output: JSON.stringify({
          object: {
            candidate_ids: [directReferenceCandidate.candidate_id],
          },
        }),
      },
      retrieve_supplemental_web_source_1: supplementalRetrieval,
    },
    step_failures: {},
  },
);
assert.equal(supplementalSelectionRequest.type, "schedule_step");
assert.equal(
  supplementalSelectionRequest.step_id,
  "select_supplemental_evidence_chunks",
);
const supplementalEvidencePacket = JSON.parse(
  supplementalSelectionRequest.input.prompt.split("CLOSED_EVIDENCE_PACKET=")[1],
);
assert.equal(
  supplementalEvidencePacket.sources[0].source_id,
  "supplemental-catalog-source-1",
);
const supplementalSource = supplementalEvidencePacket.sources[0];
const supplementalFocus = supplementalEvidencePacket.focuses[0];
const completedSupplementalRun = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
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
      select_supplemental_web_sources: {
        output: JSON.stringify({
          object: {
            candidate_ids: [directReferenceCandidate.candidate_id],
          },
        }),
      },
      retrieve_supplemental_web_source_1: supplementalRetrieval,
      select_supplemental_evidence_chunks: {
        output: JSON.stringify({
          object: {
            chunk_ids: [supplementalSource.chunks[0].chunk_id],
            source_coverage: [{
              source_id: supplementalSource.source_id,
              obligation_id: supplementalFocus.obligation_id,
              completion_criterion_indexes: [0],
              roles: {
                supporting: true,
                primary: true,
                independent: false,
              },
            }],
            source_relevance: [{
              source_id: supplementalSource.source_id,
              obligation_id: supplementalFocus.obligation_id,
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
