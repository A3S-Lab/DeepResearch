import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import vm from "node:vm";

const workflowFiles = [
  "src/workflow/retrieval_foundation.js",
  "src/workflow/retrieval_web.js",
  "src/workflow/retrieval_source_selection.js",
  "src/workflow/retrieval_web_projection.js",
  "src/workflow/retrieval_selection.js",
  "src/workflow/retrieval_reduction.js",
  "src/workflow/retrieval_materialization.js",
  "src/workflow/retrieval_gap.js",
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

const caseFiles = [
  "tests/workflow_discovery_smoke/discovery_transport.mjs",
  "tests/workflow_discovery_smoke/selection_local.mjs",
  "tests/workflow_discovery_smoke/gap_setup.mjs",
  "tests/workflow_discovery_smoke/criterion_gap.mjs",
  "tests/workflow_discovery_smoke/supplemental_rounds.mjs",
];

const caseSource = (
  await Promise.all(caseFiles.map((path) => readFile(path, "utf8")))
).join("\n");

await vm.runInNewContext(
  `(async () => {\n${caseSource}\n})()`,
  { assert, sandbox, TextEncoder, Buffer },
  {
    filename: "tests/workflow_discovery_smoke/cases.mjs",
    timeout: 5_000,
  },
);
