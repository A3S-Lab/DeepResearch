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

const unstructured = await discover(
  "https://three.example.test/record?format=full",
);
assert.equal(unstructured.status, "failed");
assert.equal(unstructured.candidates.length, 0);
