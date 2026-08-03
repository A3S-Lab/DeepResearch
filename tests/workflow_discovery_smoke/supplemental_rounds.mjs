const referenceSupplementalPacket = JSON.parse(
  referenceSupplementalRequest.input.prompt
    .split("CLOSED_SUPPLEMENTAL_DISCOVERY_PACKET=")[1],
);
assert.ok(
  Buffer.byteLength(referenceSupplementalRequest.input.prompt, "utf8") <=
    64 * 1024,
);
assert.equal(
  referenceSupplementalPacket.candidate_metadata_projection
    .all_candidate_identities_preserved,
  true,
);
assert.deepEqual(
  Array.from(referenceSupplementalPacket.active_search_queries),
  ["fresh.example.test fresh direct evidence query"],
);
const freshSupplementalCandidate = referenceSupplementalPacket.candidates.find(
  (candidate) => candidate.url === "https://fresh.example.test/direct",
);
assert.deepEqual(
  Array.from(freshSupplementalCandidate.discovery_queries),
  ["fresh.example.test fresh direct evidence query"],
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
    (candidate) => candidate.url === "https://fresh.example.test/direct",
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
      candidate.url === "https://media.example.test/chart.webp" ||
      candidate.url === "https://truncated.example.test/part" ||
      candidate.url === "https://unselected.example.test/ignored",
  ),
);

const largeGapDiscovery = {
  status: "success",
  candidates: Array.from({ length: 80 }, (_value, index) => ({
    candidate_id: `large-gap-candidate-${index + 1}`,
    title: `Large gap candidate ${index + 1}`,
    url: `https://large-gap-${index + 1}.example.test/record`,
    date: "",
    content: `Bounded discovery context ${index + 1}`,
    provider_text: "",
    engines: ["test-provider"],
    discovery: ["provider_result"],
    query_indexes: [0],
  })),
  errors: [],
  metadata: {},
};
const largeSupplementalBaseOutputs = {
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
  discover_gap_web_sources: largeGapDiscovery,
};
const largeSupplementalShardRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from a complete large supplemental catalog",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: largeSupplementalBaseOutputs,
    step_failures: {},
  },
);
assert.equal(largeSupplementalShardRequest.type, "schedule_steps");
assert.ok(largeSupplementalShardRequest.steps.length > 1);
assert.ok(
  largeSupplementalShardRequest.steps.every((step) =>
    step.step_id.startsWith("select_supplemental_web_sources_shard_")
  ),
);
const largeSupplementalCandidateIds = [];
const largeSupplementalShardOutputs = {};
for (const step of largeSupplementalShardRequest.steps) {
  const packet = JSON.parse(
    step.input.prompt.split("CLOSED_SUPPLEMENTAL_DISCOVERY_PACKET=")[1],
  );
  assert.ok(packet.candidates.length <= 24);
  assert.ok(
    packet.candidate_metadata_projection.uniform_scale_percent > 0,
  );
  assert.ok(Buffer.byteLength(step.input.prompt, "utf8") <= 64 * 1024);
  largeSupplementalCandidateIds.push(
    ...packet.candidates.map((candidate) => candidate.candidate_id),
  );
  largeSupplementalShardOutputs[step.step_id] = {
    output: JSON.stringify({
      object: {
        candidate_ids: packet.candidates
          .slice(0, step.input.schema.properties.candidate_ids.maxItems)
          .map((candidate) => candidate.candidate_id),
      },
    }),
  };
}
assert.equal(largeSupplementalCandidateIds.length, 82);
assert.equal(new Set(largeSupplementalCandidateIds).size, 82);
const largeSupplementalReductionRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Select from a complete large supplemental catalog",
      loop_contract: supplementalLoopContract,
      execution_mode: "collect_only",
      evidence_scope: "web_and_workspace",
      research_plan: referencePlan,
      bootstrap_acquisition: {},
    },
    step_outputs: {
      ...largeSupplementalBaseOutputs,
      ...largeSupplementalShardOutputs,
    },
    step_failures: {},
  },
);
assert.equal(largeSupplementalReductionRequest.type, "schedule_step");
assert.equal(
  largeSupplementalReductionRequest.step_id,
  "select_supplemental_web_sources",
);
assert.ok(
  largeSupplementalReductionRequest.input.schema.properties.candidate_ids.items
    .enum.length >
      largeSupplementalReductionRequest.input.schema.properties.candidate_ids
        .maxItems,
);

const supplementalAdmissionFallback = await sandbox.__deepResearchRun(
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
    step_failures: {
      select_supplemental_web_sources: {
        error: "simulated supplemental source admission timeout",
      },
    },
  },
);
assert.equal(supplementalAdmissionFallback.type, "schedule_steps");
const supplementalFallbackUrls = Array.from(
  supplementalAdmissionFallback.steps,
  (step) => step.input.candidates[0].url,
);
assert.equal(
  supplementalFallbackUrls[0],
  "https://fresh.example.test/direct",
);
assert.deepEqual(
  new Set(supplementalFallbackUrls),
  new Set([
    "https://fresh.example.test/direct",
    "https://reference.example.test/primary",
    "https://remaining.example.test/candidate",
  ]),
);
assert.ok(
  supplementalAdmissionFallback.steps.every(
    (step) => step.input.prefer_bounded_snapshot === true,
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
const completedSupplementalInput = {
  query: "Follow a retained evidence reference",
  loop_contract: supplementalLoopContract,
  execution_mode: "collect_only",
  evidence_scope: "web_and_workspace",
  research_plan: referencePlan,
  bootstrap_acquisition: {},
};
const completedSupplementalStepOutputs = {
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
};
const secondGapQueryRequest = await sandbox.__deepResearchRun(
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
            source_coverage: [],
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
assert.equal(secondGapQueryRequest.type, "schedule_step");
assert.equal(secondGapQueryRequest.step_id, "generate_gap_queries_2");
assert.equal(secondGapQueryRequest.input.timeout_ms, 180_000);
assert.equal(secondGapQueryRequest.retry.max_attempts, 1);
const secondGapPacket = JSON.parse(
  secondGapQueryRequest.input.prompt.split("TYPED_GAP_QUERY_PACKET=")[1],
);
assert.equal(secondGapPacket.round, 2);
assert.ok(
  secondGapQueryRequest.input.prompt.includes(
    "Later rounds rotate priority across the declared material focuses",
  ),
);

const secondGapDiscoveryRequest = await sandbox.__deepResearchRun(
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
            source_coverage: [],
            source_relevance: [{
              source_id: supplementalSource.source_id,
              obligation_id: supplementalFocus.obligation_id,
            }],
          },
        }),
      },
      generate_gap_queries_2: {
        output: JSON.stringify({
          object: { queries: ["second distinct gap query"] },
        }),
      },
    },
    step_failures: {},
  },
);
assert.equal(secondGapDiscoveryRequest.type, "schedule_step");
assert.equal(secondGapDiscoveryRequest.step_id, "discover_gap_web_sources_2");
assert.deepEqual(
  Array.from(secondGapDiscoveryRequest.input.plan.search_queries),
  ["second distinct gap query"],
);

const thirdGapQueryRequest = await sandbox.__deepResearchRun(
  {},
  {
    kind: "workflow",
    input: {
      query: "Follow a retained evidence reference",
      loop_contract: fullGapLoopContract,
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
            source_coverage: [],
            source_relevance: [{
              source_id: supplementalSource.source_id,
              obligation_id: supplementalFocus.obligation_id,
            }],
          },
        }),
      },
      generate_gap_queries_2: {
        output: JSON.stringify({
          object: { queries: ["second distinct gap query"] },
        }),
      },
      discover_gap_web_sources_2: {
        status: "failed",
        candidates: [],
        errors: ["No fetchable candidate in this round."],
        metadata: {},
      },
    },
    step_failures: {},
  },
);
assert.equal(thirdGapQueryRequest.type, "schedule_step");
assert.equal(thirdGapQueryRequest.step_id, "generate_gap_queries_3");
assert.equal(thirdGapQueryRequest.input.timeout_ms, 180_000);
const thirdGapPacket = JSON.parse(
  thirdGapQueryRequest.input.prompt.split("TYPED_GAP_QUERY_PACKET=")[1],
);
assert.equal(thirdGapPacket.round, 3);
