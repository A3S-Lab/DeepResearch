# A3S DeepResearch

`a3s-deep-research` is the evidence-first, domain-agnostic deep research engine
used by A3S products.

The engine is intentionally independent of CLI, TUI, and web presentation
layers. Product integrations provide model, search, fetch, persistence,
progress, and artifact-opening adapters. The engine owns:

- bounded semantic research planning;
- exact-query bootstrap and supplemental retrieval orchestration;
- source sanitization and evidence admission;
- closed-evidence report generation contracts;
- deterministic quality and citation gates; and
- Markdown and HTML research artifacts.

Topic dictionaries, named-entity branches, and domain-specific query or report
fast paths do not belong in this repository. Domain examples may appear only
as black-box regression fixtures.

## Status

The engine is being extracted from the A3S CLI. The first integration target is
the `a3s code` DeepResearch mode.

## License

MIT
