async function run(ctx, inputs) {
  const MAX_SOURCES = 24;
  const MAX_CATALOG_SOURCES = 32;
  const MAX_PLANNER_COMPLETION_CRITERIA = 3;
  const MAX_RESEARCH_FOCUSES = Math.floor(
    MAX_SOURCES / MAX_PLANNER_COMPLETION_CRITERIA
  );
  const MAX_SEARCH_QUERIES = 16;
  const MAX_GAP_ROUNDS = 4;
  const MAX_RESULTS_PER_SEARCH = 16;
  const MAX_PROVIDER_FULL_TEXT_BYTES = 8 * 1024;
  const MAX_PROVIDER_TEXT_CHARS = MAX_PROVIDER_FULL_TEXT_BYTES;
  const MAX_SEED_URLS = 3;
  const MAX_DISCOVERY_CANDIDATES =
    MAX_SEARCH_QUERIES * MAX_RESULTS_PER_SEARCH + MAX_SEED_URLS;
  // Keep a complete bounded catalog without positional sampling. Source-aware
  // selector packets below keep every admitted chunk inside the generation
  // transport limit before the existing exact-ID reduction pass.
  // Raw catalogs are reduced through byte-bounded selector shards before any
  // excerpt reaches synthesis. Keep a power-of-two aggregate fuse large
  // enough for the complete bounded source catalog instead of coupling it to
  // the older four-focus acquisition shape.
  const MAX_CHUNKS = 2048;
  const MAX_CHUNK_CHARS = 700;
  // Candidate admission becomes materially less reliable before the
  // transport's hard 128 KiB ceiling. Preserve every candidate identity while
  // reducing discovery metadata early enough to keep semantic source
  // selection inside the empirically stable generation envelope.
  const MAX_GENERATION_PROMPT_BYTES = 64 * 1024;
  // Source admission uses smaller complete candidate shards once one prompt
  // crosses the latency envelope observed to finish reliably. Each shard
  // retains full closed identities, and a small second-stage selector reduces
  // their semantic union to the transport budget.
  const MAX_WEB_SOURCE_SELECTOR_SHARD_CANDIDATES = 24;
  const WEB_SOURCE_SELECTOR_OVERSAMPLE_FACTOR = 2;
  // Small catalogs use one direct selector. Larger catalogs first split each
  // source into complete byte-bounded units, then pack units from distinct
  // sources together. A large source may span packets, while the later exact-ID
  // source reduction still chooses its strongest excerpts across every unit.
  const MAX_DIRECT_SELECTOR_CHUNKS = 10;
  const MAX_SELECTOR_SHARD_CANDIDATES = 4;
  // Use most of the safe prompt envelope on the normal path, but cap the
  // number of independently judged sources so schema cardinality does not
  // grow with a dense catalog. A failed optimistic shard is repartitioned
  // losslessly into the smaller recovery envelope below. This keeps complete
  // source text review practical without making one large generation the only
  // way to retain evidence.
  const MAX_SELECTOR_SHARD_PACKET_BYTES = 96 * 1024;
  const MAX_SELECTOR_RECOVERY_PACKET_BYTES = 32 * 1024;
  const MAX_SELECTOR_SHARD_SOURCES = 4;
  const MAX_EXCERPTS_PER_SOURCE = 4;
  const MAX_EXCERPT_CHARS_PER_SOURCE = 2800;
  const MAX_DOCUMENT_RANGES = 3;
  const MAX_HTML_RANGES = 2;
  const MAX_LOCAL_SOURCES = 8;
  const MAX_LOCAL_RANGES = 3;
  const MAX_LOCAL_RANGE_LINES = 240;
  // Planned and supplemental candidate catalogs can contain every result from
  // the bounded query set. Give one failover-aware admission enough active
  // time to judge that complete catalog, while keeping exactly one outer
  // attempt. Bootstrap acquisition does not use this model-backed selector;
  // it deterministically fetches a small provenance-balanced candidate set.
  const WEB_SOURCE_SELECTION_ACTIVE_TIMEOUT_MS = 180_000;
  // Gap-query packets can carry every unresolved atomic criterion and are
  // materially larger than candidate-ID selectors. Give one failover-aware
  // generation enough active time without multiplying the retrieval stage by
  // an outer retry of the same request.
  const GAP_QUERY_GENERATION_ACTIVE_TIMEOUT_MS = 180_000;
  const MODEL_GENERATION_ACTIVE_TIMEOUT_MS = 300_000;
  // A real primary-source selector exceeded 210 seconds. Keep exactly one
  // attempt so a slow source cannot starve later siblings, but allow the same
  // 270-second long-tail bound used by other closed-evidence generations.
  const MODEL_GENERATION_SHARD_ACTIVE_TIMEOUT_MS = 270_000;
  const STEP_DISCOVER_WEB = "discover_web_sources";
  const STEP_SELECT_WEB = "select_web_sources";
  const STEP_SELECT_WEB_SHARD_PREFIX = "select_web_sources_shard_";
  const STEP_WEB_SOURCE = "retrieve_web_source";
  const STEP_WEB_SOURCE_PREFIX = "retrieve_web_source_";
  const STEP_LOCAL = "retrieve_local";
  const STEP_SELECT = "select_evidence_chunks";
  const STEP_SELECT_SHARD_PREFIX = "select_evidence_chunks_shard_";
  const STEP_SELECT_SHARD_RECOVERY_PREFIX =
    "select_evidence_chunks_shard_recovery_";
  const STEP_SELECT_SOURCE_PREFIX = "select_evidence_chunks_source_";
  const STEP_CHECKPOINT_BOOTSTRAP = "checkpoint_bootstrap_acquisition";
  const STEP_CHECKPOINT_INITIAL = "checkpoint_initial_retrieval";
  const STEP_SELECT_SUPPLEMENTAL_WEB = "select_supplemental_web_sources";
  const STEP_SELECT_SUPPLEMENTAL_WEB_SHARD_PREFIX =
    "select_supplemental_web_sources_shard_";
  const STEP_GENERATE_GAP_QUERIES = "generate_gap_queries";
  const STEP_DISCOVER_GAP_WEB = "discover_gap_web_sources";
  const STEP_SUPPLEMENTAL_WEB_SOURCE_PREFIX =
    "retrieve_supplemental_web_source_";
  const STEP_SELECT_SUPPLEMENTAL = "select_supplemental_evidence_chunks";
  const STEP_SELECT_SUPPLEMENTAL_SHARD_PREFIX =
    "select_supplemental_evidence_chunks_shard_";
  const STEP_SELECT_SUPPLEMENTAL_SHARD_RECOVERY_PREFIX =
    "select_supplemental_evidence_chunks_shard_recovery_";
  const STEP_SELECT_SUPPLEMENTAL_SOURCE_PREFIX =
    "select_supplemental_evidence_chunks_source_";

  const object = (value) =>
    value && typeof value === "object" && !Array.isArray(value) ? value : {};
  const nonEmpty = (value) => typeof value === "string" && value.trim().length > 0;
  const clamp = (value, minimum, maximum, fallback) => {
    const number = Number(value);
    return Number.isFinite(number)
      ? Math.max(minimum, Math.min(maximum, Math.floor(number)))
      : fallback;
  };
  const bounded = (value, maximum) => {
    const compact = String(value || "").replace(/\s+/g, " ").trim();
    const characters = Array.from(compact);
    return characters.length <= maximum
      ? compact
      : `${characters.slice(0, Math.max(0, maximum - 1)).join("")}…`;
  };
  const boundedText = (value, maximum) => {
    const text = String(value || "")
      .replace(/\u0000/g, "")
      .replace(/\r\n?/g, "\n")
      .trim();
    const characters = Array.from(text);
    return characters.length <= maximum
      ? text
      : characters.slice(0, maximum).join("");
  };
  const portableSearchQuery = (value) =>
    String(value || "")
      .split(/\s+/)
      .filter(Boolean)
      .map((token) => /^site:/i.test(token) && token.length > 5
        ? token.slice(5)
        : token)
      .join(" ");
  const utf8ByteLength = (value) => {
    let bytes = 0;
    for (const character of String(value || "")) {
      const codePoint = character.codePointAt(0);
      bytes += codePoint <= 0x7f
        ? 1
        : codePoint <= 0x7ff
        ? 2
        : codePoint <= 0xffff
        ? 3
        : 4;
    }
    return bytes;
  };
  const utf8Prefix = (value, requestedBytes) => {
    const text = String(value || "");
    if (!Number.isSafeInteger(requestedBytes) || requestedBytes < 0) {
      return { text: "", code_units: 0, complete: false };
    }
    if (requestedBytes === 0) {
      return { text: "", code_units: 0, complete: true };
    }
    let bytes = 0;
    let codeUnits = 0;
    for (const character of text) {
      const characterBytes = utf8ByteLength(character);
      if (bytes + characterBytes > requestedBytes) {
        return {
          text: text.slice(0, codeUnits),
          code_units: codeUnits,
          complete: false,
        };
      }
      bytes += characterBytes;
      codeUnits += character.length;
      if (bytes === requestedBytes) {
        return {
          text: text.slice(0, codeUnits),
          code_units: codeUnits,
          complete: true,
        };
      }
    }
    return {
      text: text.slice(0, codeUnits),
      code_units: codeUnits,
      complete: bytes === requestedBytes,
    };
  };
  const artifactIsTruncated = (artifact) => {
    const value = object(artifact);
    const originalBytes = Number(value.original_bytes);
    const shownBytes = Number(value.shown_bytes);
    return Number.isSafeInteger(originalBytes) &&
      Number.isSafeInteger(shownBytes) &&
      originalBytes > shownBytes &&
      shownBytes >= 0;
  };
  const uniqueStrings = (values) => {
    const seen = new Set();
    const result = [];
    for (const value of values || []) {
      const text = typeof value === "string" ? value.trim() : "";
      if (!text || seen.has(text)) {
        continue;
      }
      seen.add(text);
      result.push(text);
    }
    return result;
  };
  const errorText = (error) =>
    bounded(error && error.message ? error.message : error, 600) ||
    "the tool returned no diagnostic";
  const toolExitCode = (result) => Number(
    result && (result.exitCode ?? result.exit_code)
  ) || 0;
  const cleanUrl = (value) => {
    let url = String(value || "").trim();
    while (/[.,;:!?\]}]$/.test(url)) {
      url = url.slice(0, -1);
    }
    while (url.endsWith(")")) {
      const openings = (url.match(/\(/g) || []).length;
      const closings = (url.match(/\)/g) || []).length;
      if (closings <= openings) {
        break;
      }
      url = url.slice(0, -1);
    }
    if (
      !/^https?:\/\/[^/\s]+(?:\/[^\s]*)?$/i.test(url) ||
      /[\u2026\uFFFD{}<>]/u.test(url)
    ) {
      return "";
    }
    return url;
  };
  const urlParts = (value) => {
    const url = cleanUrl(value);
    const match = url.match(/^(https?):\/\/([^/?#]+)([^#]*)/i);
    if (!match) {
      return null;
    }
    const scheme = match[1].toLowerCase();
    let authority = match[2].split("@").pop().toLowerCase();
    if (
      (scheme === "https" && authority.endsWith(":443")) ||
      (scheme === "http" && authority.endsWith(":80"))
    ) {
      authority = authority.replace(/:\d+$/, "");
    }
    return {
      url,
      scheme,
      authority,
      suffix: match[3] || "/",
    };
  };
  const canonicalUrl = (value) => {
    const parsed = urlParts(value);
    if (!parsed) {
      return "";
    }
    const suffix = parsed.suffix || "/";
    return `${parsed.scheme}://${parsed.authority}${suffix}`.replace(/\/+$/, "");
  };
  const urlHost = (value) => {
    const parsed = urlParts(value);
    if (!parsed) {
      return "";
    }
    if (parsed.authority.startsWith("[")) {
      const end = parsed.authority.indexOf("]");
      return end >= 0 ? parsed.authority.slice(0, end + 1) : "";
    }
    return parsed.authority.replace(/:\d+$/, "");
  };
  // Fetch the exact closed-catalog URL. Provider- or topic-specific URL
  // rewriting would make transport routing depend on host/path vocabulary and
  // can silently replace the artifact selected by the semantic contract.
  const fetchUrl = (value) => cleanUrl(value);

  const batchSections = (batch, expectedCount) => {
    const results = batch && batch.metadata && Array.isArray(batch.metadata.results)
      ? batch.metadata.results
      : [];
    let text = String(batch && batch.output || "");
    const batchArtifact = object(
      batch && batch.metadata && batch.metadata.artifact
    );
    if (artifactIsTruncated(batchArtifact)) {
      text = utf8Prefix(text, Number(batchArtifact.shown_bytes)).text;
    }
    const sections = Array.from({ length: expectedCount }, () => ({
      output: "",
      complete: false,
    }));
    let cursor = 0;
    for (let position = 0; position < expectedCount; position += 1) {
      const metadata = results.find(
        (item) => Number(item && item.index) === position
      );
      if (!metadata) {
        break;
      }
      const correlationId = typeof metadata.id === "string"
        ? metadata.id
        : null;
      const tool = String(metadata.tool || "");
      const label = correlationId === null
        ? tool
        : `${tool} · ${correlationId}`;
      const header = `--- [${position + 1}: ${label}] ---\n`;
      if (text.slice(cursor, cursor + header.length) !== header) {
        break;
      }
      cursor += header.length;
      if (metadata.success !== true) {
        const errorPrefix = "ERROR: ";
        if (text.slice(cursor, cursor + errorPrefix.length) !== errorPrefix) {
          break;
        }
        cursor += errorPrefix.length;
      }
      const outputBytes = Number(metadata.output_bytes);
      const prefix = utf8Prefix(text.slice(cursor), outputBytes);
      sections[position] = {
        output: prefix.text,
        complete: prefix.complete,
      };
      cursor += prefix.code_units;
      if (!prefix.complete) {
        break;
      }
      if (text.slice(cursor, cursor + 2) === "\r\n") {
        cursor += 2;
      } else if (text[cursor] === "\n") {
        cursor += 1;
      } else {
        break;
      }
    }
    return sections;
  };
  const batchChild = (batch, sections, index) => {
    const results = batch && batch.metadata && Array.isArray(batch.metadata.results)
      ? batch.metadata.results
      : [];
    const metadata = results.find((item) => Number(item && item.index) === index);
    const section = sections[index] || { output: "", complete: false };
    const output = section.output;
    const childMetadata = object(metadata && metadata.metadata);
    const childArtifact = object(childMetadata.artifact);
    const outputBytes = Number(metadata && metadata.output_bytes);
    const outputTruncated = Boolean(
      metadata &&
      metadata.success === true &&
      Number.isSafeInteger(outputBytes) &&
      outputBytes > 0 &&
      (
        artifactIsTruncated(childArtifact) ||
        section.complete !== true
      )
    );
    return {
      success: metadata ? metadata.success === true : false,
      output,
      metadata: childMetadata,
      error_kind: metadata && metadata.error_kind,
      output_truncated: outputTruncated,
    };
  };
  const invokeBatch = async (invocations, maximumConcurrency) => {
    if (invocations.length === 0) {
      return { batch: null, children: [] };
    }
    const batch = await ctx.tool("batch", {
      invocations,
      max_concurrency: Math.max(
        1,
        Math.min(maximumConcurrency, invocations.length)
      ),
    });
    const sections = batchSections(batch, invocations.length);
    return {
      batch,
      children: invocations.map((_invocation, index) =>
        batchChild(batch, sections, index)
      ),
    };
  };
  const invokeBatchWithOutputRecovery = async (
    invocations,
    maximumConcurrency
  ) => {
    const initial = await invokeBatch(invocations, maximumConcurrency);
    const children = initial.children.slice();
    const recoveryErrors = [];
    let recoveryCount = 0;
    for (let index = 0; index < children.length; index += 1) {
      if (!children[index] || children[index].output_truncated !== true) {
        continue;
      }
      try {
        const recovered = await invokeBatch([invocations[index]], 1);
        children[index] = recovered.children[0] || children[index];
        recoveryCount += 1;
      } catch (error) {
        recoveryErrors.push(
          `Batch output recovery ${index + 1} failed: ${errorText(error)}`
        );
      }
    }
    return {
      batch: initial.batch,
      children,
      output_recovery_count: recoveryCount,
      output_recovery_errors: recoveryErrors,
    };
  };

  const parseSearchResults = (output) => {
    const text = String(output || "").trim();
    if (!text) {
      return [];
    }
    try {
      const parsed = JSON.parse(text);
      const values = Array.isArray(parsed)
        ? parsed
        : (parsed && Array.isArray(parsed.results) ? parsed.results : []);
      return values
        .filter((item) => item && typeof item === "object")
        .map((item) => ({
          title: bounded(item.title || "", 220),
          url: cleanUrl(item.url || item.url_or_path),
          date: bounded(
            item.published_date || item.publication_date || item.date || "",
            100
          ),
          content: bounded(item.content || item.snippet || "", 600),
          provider_text: boundedText(
            typeof item.full_text === "string" ? item.full_text : "",
            MAX_PROVIDER_TEXT_CHARS
          ),
          engines: uniqueStrings(Array.isArray(item.engines) ? item.engines : [])
            .slice(0, 4),
        }))
        .filter((item) => item.url);
    } catch (_error) {
      return [];
    }
  };
  const documentRange = (metadata) => {
    const rawRange = metadata && metadata.range;
    if (!rawRange || typeof rawRange !== "object" || Array.isArray(rawRange)) {
      return null;
    }
    if (rawRange.offset === null || rawRange.offset === undefined) {
      return null;
    }
    const offset = Number(rawRange.offset);
    if (!Number.isSafeInteger(offset) || offset < 0) {
      return null;
    }
    const range = object(rawRange);
    const returnedChars = Number(range.returned_chars);
    if (!Number.isSafeInteger(returnedChars) || returnedChars < 0) {
      return null;
    }
    const nextOffset = range.next_offset === null || range.next_offset === undefined
      ? null
      : Number(range.next_offset);
    if (
      nextOffset !== null &&
      (!Number.isSafeInteger(nextOffset) || nextOffset <= offset)
    ) {
      return null;
    }
    const eof = range.eof === true;
    if (eof !== (nextOffset === null)) {
      return null;
    }
    if (nextOffset !== null && nextOffset !== offset + returnedChars) {
      return null;
    }
    return {
      offset,
      returned_chars: returnedChars,
      next_offset: nextOffset,
      eof,
    };
  };
  const extractedDocument = (metadata) => {
    const kind = String(metadata && metadata.document_kind || "").toLowerCase();
    const contentType = String(metadata && metadata.content_type || "").toLowerCase();
    return kind === "pdf" || kind === "document" ||
      /^application\/pdf(?:;|$)/.test(contentType);
  };
  const serializedContainerPrefixEnd = (line) => {
    const start = line.search(/\S/);
    if (start < 0 || !["{", "["].includes(line[start])) {
      return null;
    }
    const closers = [];
    let quoted = false;
    let escaped = false;
    for (let index = start; index < line.length; index += 1) {
      const character = line[index];
      if (quoted) {
        if (escaped) {
          escaped = false;
        } else if (character === "\\") {
          escaped = true;
        } else if (character === "\"") {
          quoted = false;
        }
        continue;
      }
      if (character === "\"") {
        quoted = true;
        continue;
      }
      if (character === "{" || character === "[") {
        closers.push(character === "{" ? "}" : "]");
        continue;
      }
      if (character === "}" || character === "]") {
        if (closers.pop() !== character) {
          return null;
        }
        if (closers.length === 0) {
          return { start, end: index + 1 };
        }
      }
    }
    return null;
  };
  const stripOversizedSerializedPrefix = (value) =>
    String(value || "")
      .split("\n")
      .map((line) => {
        const prefix = serializedContainerPrefixEnd(line);
        if (
          !prefix ||
          prefix.end - prefix.start <= MAX_CHUNK_CHARS
        ) {
          return line;
        }
        const suffix = line.slice(prefix.end).trimStart();
        if (Array.from(suffix).length < 30) {
          return line;
        }
        return `${line.slice(0, prefix.start)}${suffix}`;
      })
      .join("\n");
  const decodeEscapedCodeUnits = (value) =>
    String(value || "")
      .replace(/\\+u([0-9a-fA-F]{4})/g, (_match, digits) =>
        String.fromCharCode(Number.parseInt(digits, 16))
      )
      .replace(/\\+n/g, "\n")
      .replace(/\\+r/g, "\n")
      .replace(/\\+t/g, " ")
      .replace(/\\+"/g, "\"");
  const visibleTextFromSerializedMarkup = (line) => {
    const start = line.search(/\S/);
    if (
      start < 0 ||
      !["{", "["].includes(line[start]) ||
      Array.from(line).length <= MAX_CHUNK_CHARS
    ) {
      return "";
    }
    const decoded = decodeEscapedCodeUnits(line);
    const markupStart = decoded.indexOf("<");
    const markupEnd = decoded.lastIndexOf(">");
    if (markupStart < 0 || markupEnd <= markupStart) {
      return "";
    }
    const visible = decoded
      .slice(markupStart, markupEnd + 1)
      .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "\n")
      .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, "\n")
      .replace(/<noscript\b[^>]*>[\s\S]*?<\/noscript>/gi, "\n")
      .replace(/<br\b[^>]*>/gi, "\n")
      .replace(/<\/[^>]+>/g, "\n")
      .replace(/<[^>]+>/g, " ")
      .replace(/&#x([0-9a-fA-F]+);/g, (entity, digits) => {
        const codePoint = Number.parseInt(digits, 16);
        return codePoint <= 0x10ffff
          ? String.fromCodePoint(codePoint)
          : entity;
      })
      .replace(/&#([0-9]+);/g, (entity, digits) => {
        const codePoint = Number.parseInt(digits, 10);
        return codePoint <= 0x10ffff
          ? String.fromCodePoint(codePoint)
          : entity;
      })
      .replace(/&(amp|lt|gt|quot|apos);/g, (_entity, name) => ({
        amp: "&",
        lt: "<",
        gt: ">",
        quot: "\"",
        apos: "'",
      })[name])
      .replace(/[ \t]+/g, " ")
      .replace(/ *\n */g, "\n")
      .replace(/\n{3,}/g, "\n\n")
      .trim();
    return Array.from(visible).length >= 30 ? visible : "";
  };
  const structurallyVisibleFetchedText = (value) =>
    stripOversizedSerializedPrefix(value)
      .split("\n")
      .map((line) => visibleTextFromSerializedMarkup(line) || line)
      .join("\n");
  const cleanFetchedText = (value, returnedChars) => {
    const characters = Array.from(String(value || ""));
    if (
      !Number.isSafeInteger(returnedChars) ||
      returnedChars <= 0 ||
      characters.length < returnedChars
    ) {
      return "";
    }
    // Only serialization shape, balanced delimiters, and byte budgets decide
    // whether hidden transport state is unwrapped. No query, host, language,
    // publisher, path, or vocabulary participates in this boundary.
    const text = structurallyVisibleFetchedText(
      characters
        .slice(0, returnedChars)
        .join("")
        .replace(/\r\n?/g, "\n")
        .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, "\n")
        .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, "\n")
        .replace(/<noscript\b[^>]*>[\s\S]*?<\/noscript>/gi, "\n")
    );
    const seen = new Set();
    return text
      .split(/\n+/)
      .map((line) => line.replace(/\s+/g, " ").trim())
      .filter((line) => {
        if (!line || seen.has(line)) return false;
        seen.add(line);
        return true;
      })
      .join("\n")
      .trim();
  };
  const substantive = (value) => {
    const visible = String(value || "")
      .replace(/<script[\s\S]*?<\/script>/gi, " ")
      .replace(/<style[\s\S]*?<\/style>/gi, " ")
      .replace(/<[^>]+>/g, " ")
      .replace(/\s+/g, " ")
      .trim();
    return Array.from(visible).length >= 30;
  };
  const transientFetchFailure = (child) => {
    const errorKind = child && child.error_kind;
    const kind = errorKind && typeof errorKind === "object"
      ? errorKind.type
      : null;
    return Boolean(
      child &&
      (kind === "timeout" || kind === "transport")
    );
  };

  const evidenceLines = (value) => String(value || "")
    .split(/\n+/)
    .map((line) => line.replace(/\s+/g, " ").trim())
    .filter((line) => Array.from(line).length >= 12);
  const splitLongText = (value) => {
    const characters = Array.from(String(value || ""));
    const chunks = [];
    for (let offset = 0; offset < characters.length; offset += MAX_CHUNK_CHARS) {
      // Chunk boundaries are positional transport budgets. They never inspect
      // punctuation, words, language, script, entities, or topic vocabulary.
      const chunk = characters
        .slice(offset, offset + MAX_CHUNK_CHARS)
        .join("")
        .trim();
      if (chunk) {
        chunks.push(chunk);
      }
    }
    return chunks;
  };
  const sourceChunks = (values, sourceId) => {
    const segments = Array.isArray(values) ? values : [values];
    const chunks = [];
    let pending = "";
    const retain = () => {
      const text = bounded(pending, MAX_CHUNK_CHARS);
      if (text) {
        chunks.push({
          chunk_id: `${sourceId}:chunk:${chunks.length + 1}`,
          text,
        });
      }
      pending = "";
    };
    for (const segment of segments) {
      const units = evidenceLines(segment).flatMap(splitLongText);
      for (const unit of units) {
        const candidate = pending ? `${pending} ${unit}` : unit;
        if (Array.from(candidate).length > MAX_CHUNK_CHARS) {
          retain();
          pending = unit;
        } else {
          pending = candidate;
        }
      }
      // A provider range is a structural retrieval boundary. Keep it as a
      // distinct semantic-selection unit without turning it into another pass.
      retain();
    }
    return chunks;
  };
  const structuredFeedSegments = (values) => {
    const segments = Array.isArray(values) ? values : [values];
    const text = segments.map((value) => String(value || "")).join("\n");
    if (!/<feed(?:\s|>)/i.test(text) || !/<entry(?:\s|>)/i.test(text)) {
      return segments;
    }
    const starts = [];
    const entryPattern = /<entry(?:\s|>)/gi;
    let match = null;
    while ((match = entryPattern.exec(text)) !== null) {
      starts.push(match.index);
    }
    if (starts.length === 0) {
      return segments;
    }
    const boundedSegments = [];
    const header = text.slice(0, starts[0]).trim();
    if (header) {
      boundedSegments.push(header);
    }
    for (let index = 0; index < starts.length; index += 1) {
      const end = index + 1 < starts.length ? starts[index + 1] : text.length;
      const entry = text.slice(starts[index], end).trim();
      if (entry) {
        boundedSegments.push(entry);
      }
    }
    return boundedSegments;
  };
  const planFocuses = (plan) => {
    const tracks = Array.isArray(plan.tracks) ? plan.tracks : [];
    return tracks.slice(0, MAX_RESEARCH_FOCUSES).map((track, index) => {
      const item = object(track);
      const questions = (Array.isArray(item.questions) ? item.questions : [])
        .map((question) => {
          if (nonEmpty(question)) {
            return {
              question: question.trim(),
              role: "",
              completion_criterion_indexes: [],
            };
          }
          const structured = object(question);
          return nonEmpty(structured.question)
            ? {
                question: structured.question.trim(),
                role: nonEmpty(structured.role) ? structured.role.trim() : "",
                completion_criterion_indexes: Array.isArray(
                    structured.completion_criterion_indexes
                  )
                  ? structured.completion_criterion_indexes.filter(
                      Number.isSafeInteger
                    )
                  : [],
              }
            : null;
        })
        .filter(Boolean)
        .slice(0, 4);
      const completionCriteria = Array.isArray(item.completion_criteria)
        ? item.completion_criteria
            .filter(nonEmpty)
            .slice(0, MAX_PLANNER_COMPLETION_CRITERIA)
        : [];
      const evidenceRequirements = object(item.evidence_requirements);
      return {
        focus_index: index,
        obligation_id: bounded(item.id, 64),
        material: item.material === true,
        completion_criteria: completionCriteria.map((criterion) =>
          bounded(criterion, 400)
        ),
        evidence_requirements: {
          primary_source_required:
            evidenceRequirements.primary_source_required === true,
          independent_corroboration_required:
            evidenceRequirements.independent_corroboration_required === true,
        },
        research_questions: questions,
        focus: bounded(
          uniqueStrings([
            item.title,
            item.focus,
            ...questions.map((question) => question.question),
          ]).join(": "),
          900
        ),
      };
    }).filter((item) => item.obligation_id && item.focus);
  };
  const webEvidencePacket = (
    plan,
    fetched,
    sourcePrefix,
    sourceIndexOffset
  ) => {
    const focuses = planFocuses(plan);
    const prefix = nonEmpty(sourcePrefix) ? sourcePrefix : "web-source";
    const indexOffset = Number.isSafeInteger(sourceIndexOffset) &&
        sourceIndexOffset >= 0
      ? sourceIndexOffset
      : 0;
    const candidates = fetched
      .filter((item) => item.ok && substantive(item.text))
      .slice(0, MAX_SOURCES)
      .map((item, index) => {
        const sourceId = `${prefix}-${indexOffset + index + 1}`;
        const chunks = sourceChunks(
          structuredFeedSegments(item.segments || [item.text]),
          sourceId
        );
        if (chunks.length === 0) {
          return null;
        }
        return {
          source_id: sourceId,
          title: item.title || urlHost(item.url) || "Fetched source",
          url_or_path: item.url,
          // Provider dates are discovery metadata and may describe an index,
          // crawl, or documentation build rather than publication. Only the
          // fetched text may establish a date in closed evidence.
          reliability: item.retrieval_mode === "provider_full_text"
            ? item.provider_snapshot_preferred === true
              ? `Search-provider source-text snapshot${item.engines.length > 0
                ? ` returned via ${item.engines.join(", ")}`
                : ""}; retained after deterministic bounded candidate admission, and authority and claim fit require closed-evidence review.`
              : `Search-provider source-text snapshot${item.engines.length > 0
                ? ` returned via ${item.engines.join(", ")}`
                : ""}; direct transport was unavailable, and authority and claim fit require closed-evidence review.`
            : item.retrieval_mode === "bounded_direct_fetch"
            ? `Bounded initial fetched source-text snapshot${item.engines.length > 0
              ? ` discovered via ${item.engines.join(", ")}`
              : ""}; retained after deterministic bounded candidate admission, and authority and claim fit require closed-evidence review.`
            : `Fetched source text${item.engines.length > 0
              ? ` discovered via ${item.engines.join(", ")}`
              : ""}; authority and claim fit require closed-evidence review.`,
          chunks,
        };
      })
      .filter(Boolean);
    const chunkCount = candidates.reduce(
      (total, source) => total + source.chunks.length,
      0
    );
    if (focuses.length === 0 || candidates.length === 0) {
      return {
        packet: null,
        chunk_count: chunkCount,
        error: "",
      };
    }
    if (chunkCount > MAX_CHUNKS) {
      return {
        packet: null,
        chunk_count: chunkCount,
        error: `Fetched evidence produced ${chunkCount} chunks, exceeding the closed catalog limit of ${MAX_CHUNKS}; no fetched text was promoted.`,
      };
    }
    return {
      packet: {
        version: 1,
        focuses,
        sources: candidates,
      },
      chunk_count: chunkCount,
      error: "",
    };
  };
  const combinedEvidencePacket = (plan, retrievals, sourcePrefix) => {
    const focuses = planFocuses(plan);
    const prefix = nonEmpty(sourcePrefix)
      ? sourcePrefix
      : "catalog-source";
    const rawSources = retrievals
      .filter((retrieval) => retrieval && retrieval.packet)
      .flatMap((retrieval) => retrieval.packet.sources || []);
    const mergedSources = [];
    const sourceIndexByAnchor = new Map();
    for (let sourceIndex = 0; sourceIndex < rawSources.length; sourceIndex += 1) {
      const source = rawSources[sourceIndex];
      const anchor = String(source.url_or_path || "").trim();
      const identity = canonicalUrl(anchor) ||
        (anchor ? `path:${anchor}` : `unanchored:${sourceIndex}`);
      const existingIndex = sourceIndexByAnchor.get(identity);
      if (existingIndex === undefined) {
        sourceIndexByAnchor.set(identity, mergedSources.length);
        mergedSources.push(Object.assign({}, source, {
          chunks: Array.isArray(source.chunks) ? [...source.chunks] : [],
        }));
        continue;
      }
      const existing = mergedSources[existingIndex];
      const observedTexts = new Set(
        existing.chunks.map((chunk) => String(chunk.text || ""))
      );
      for (const chunk of Array.isArray(source.chunks) ? source.chunks : []) {
        const text = String(chunk.text || "");
        if (!text || observedTexts.has(text)) {
          continue;
        }
        observedTexts.add(text);
        existing.chunks.push(chunk);
      }
      if (!nonEmpty(existing.title) && nonEmpty(source.title)) {
        existing.title = source.title;
      }
    }
    const sources = mergedSources.map((source, sourceIndex) => {
        const sourceId = `${prefix}-${sourceIndex + 1}`;
        const chunks = Array.isArray(source.chunks)
          ? source.chunks.map((chunk, chunkIndex) =>
              Object.assign({}, chunk, {
                chunk_id: `${sourceId}:chunk:${chunkIndex + 1}`,
              })
            )
          : [];
        return Object.assign({}, source, {
          source_id: sourceId,
          chunks,
        });
      });
    const chunkCount = sources.reduce(
      (total, source) =>
        total + (Array.isArray(source.chunks) ? source.chunks.length : 0),
      0
    );
    if (focuses.length === 0 || sources.length === 0) {
      return {
        packet: null,
        source_count: sources.length,
        chunk_count: chunkCount,
        error: "",
      };
    }
    if (sources.length > MAX_CATALOG_SOURCES || chunkCount > MAX_CHUNKS) {
      return {
        packet: null,
        source_count: sources.length,
        chunk_count: chunkCount,
        error: `Retrieved evidence produced ${sources.length} sources and ${chunkCount} chunks, exceeding the complete closed catalog limit of ${MAX_CATALOG_SOURCES} sources and ${MAX_CHUNKS} chunks; no retrieved text was promoted.`,
      };
    }
    return {
      packet: {
        version: 1,
        focuses,
        sources,
      },
      source_count: sources.length,
      chunk_count: chunkCount,
      error: "",
    };
  };
