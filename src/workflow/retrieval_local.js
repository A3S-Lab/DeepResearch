  const localRetrievalSchema = {
    type: "object",
    additionalProperties: false,
    properties: {
      sources: {
        type: "array",
        minItems: 1,
        maxItems: MAX_LOCAL_SOURCES,
        items: {
          type: "object",
          additionalProperties: false,
          properties: {
            url_or_path: { type: "string", minLength: 1, maxLength: 1200 },
            ranges: {
              type: "array",
              minItems: 1,
              maxItems: MAX_LOCAL_RANGES,
              items: {
                type: "object",
                additionalProperties: false,
                properties: {
                  offset: {
                    type: "integer",
                    minimum: 0,
                    maximum: 1000000,
                  },
                  limit: {
                    type: "integer",
                    minimum: 1,
                    maximum: MAX_LOCAL_RANGE_LINES,
                  },
                },
                required: ["offset", "limit"],
              },
            },
          },
          required: ["url_or_path", "ranges"],
        },
      },
    },
    required: ["sources"],
  };
  const normalizeLocalPath = (value) => String(value || "")
    .trim()
    .replace(/\\/g, "/")
    .replace(/^\.\//, "")
    .replace(/\/+/g, "/");
  const observedLocalAnchor = (reported, anchors) => {
    const candidate = normalizeLocalPath(reported);
    if (!candidate) {
      return "";
    }
    for (const anchor of anchors) {
      const observed = normalizeLocalPath(anchor.url_or_path);
      if (candidate === observed) {
        return observed;
      }
    }
    return "";
  };
  const cleanLocalReadText = (value, offset, returnedLines) => {
    if (
      !Number.isSafeInteger(offset) ||
      offset < 0 ||
      !Number.isSafeInteger(returnedLines) ||
      returnedLines <= 0
    ) {
      return "";
    }
    const framedLines = String(value || "")
      .replace(/\r\n?/g, "\n")
      .split("\n")
      .slice(0, returnedLines);
    if (framedLines.length !== returnedLines) {
      return "";
    }
    const restored = [];
    for (let index = 0; index < framedLines.length; index += 1) {
      const line = framedLines[index];
      const separator = line.indexOf("\t");
      if (
        separator < 0 ||
        Number(line.slice(0, separator).trim()) !== offset + index + 1
      ) {
        return "";
      }
      restored.push(line.slice(separator + 1));
    }
    return restored.join("\n").trim();
  };
