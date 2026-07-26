pub(crate) const REPORT_CSS: &str = r#"
:root {
  color-scheme: light;
  --a3s-bg: #f7f7f8;
  --a3s-panel: #ffffff;
  --a3s-panel-soft: #f2f3f5;
  --a3s-panel-strong: #e9ebef;
  --a3s-ink: #17181a;
  --a3s-muted: #71757d;
  --a3s-faint: #a1a5ad;
  --a3s-line: #e2e4e8;
  --a3s-line-strong: #d3d6dc;
  --a3s-action: #242424;
  --a3s-action-ink: #ffffff;
  --a3s-blue: #2864e8;
  --a3s-warning: #8a5700;
  --a3s-warning-bg: #fff8e8;
  --a3s-code-bg: #1f2023;
  --a3s-code-ink: #f2f3f5;
  --a3s-font: ui-sans-serif, -apple-system, BlinkMacSystemFont, "Segoe UI",
    "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei",
    "Noto Sans CJK SC", sans-serif;
  --a3s-mono: "SFMono-Regular", Consolas, "Liberation Mono", Menlo, monospace;
}

* {
  box-sizing: border-box;
}

html {
  scroll-behavior: auto;
  scroll-padding-top: 24px;
}

body {
  margin: 0;
  color: var(--a3s-ink);
  background: var(--a3s-bg);
  font-family: var(--a3s-font);
  font-size: 15px;
  line-height: 1.72;
  overflow-wrap: anywhere;
  text-rendering: optimizeLegibility;
}

a {
  color: var(--a3s-blue);
  text-decoration-thickness: 1px;
  text-underline-offset: 0.2em;
}

a:hover {
  text-decoration-thickness: 2px;
}

:focus-visible {
  outline: 2px solid var(--a3s-blue);
  outline-offset: 3px;
}

.skip-link {
  position: fixed;
  top: -64px;
  left: 16px;
  z-index: 20;
  padding: 9px 13px;
  color: var(--a3s-action-ink);
  background: var(--a3s-action);
  border-radius: 7px;
  font-size: 13px;
  font-weight: 600;
  text-decoration: none;
}

.skip-link:focus {
  top: 16px;
}

.report-shell {
  display: grid;
  width: min(1480px, calc(100% - 48px));
  grid-template-columns: 176px minmax(0, 1fr) 232px;
  gap: 16px;
  align-items: start;
  margin: 32px auto 64px;
}

.report-column {
  grid-column: 2;
  grid-row: 1;
  min-width: 0;
  overflow: clip;
  background: var(--a3s-panel);
  border: 1px solid var(--a3s-line);
  border-radius: 14px;
  box-shadow: 0 1px 2px rgba(23, 24, 26, 0.04);
}

.report-hero {
  padding: 52px 64px 40px;
  background: var(--a3s-panel);
  border-bottom: 1px solid var(--a3s-line);
}

.eyebrow {
  margin: 0 0 14px;
  color: var(--a3s-blue);
  font-size: 11px;
  font-weight: 650;
  line-height: 16px;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.report-hero h1 {
  max-width: 27ch;
  margin: 0;
  color: var(--a3s-ink);
  font-size: clamp(28px, 4vw, 38px);
  font-weight: 650;
  line-height: 1.22;
  letter-spacing: -0.025em;
}

.report-thesis,
.hero-thesis {
  max-width: 72ch;
  margin: 22px 0 0;
  color: var(--a3s-muted);
  font-size: 16px;
  line-height: 1.75;
}

.signal-row {
  display: flex;
  flex-wrap: wrap;
  gap: 7px 18px;
  margin-top: 24px;
  color: var(--a3s-muted);
  font-size: 12px;
  line-height: 18px;
}

.signal-row span {
  display: inline-flex;
  align-items: center;
  gap: 7px;
}

.signal-row span::before {
  width: 5px;
  height: 5px;
  content: "";
  background: var(--a3s-ink);
  border-radius: 50%;
}

.report-degraded .report-hero {
  box-shadow: inset 4px 0 0 var(--a3s-warning);
}

.report-degraded .eyebrow {
  color: var(--a3s-warning);
}

.report-nav {
  position: sticky;
  top: 24px;
  z-index: 5;
  grid-column: 3;
  grid-row: 1;
  align-self: start;
  max-height: calc(100vh - 48px);
  margin: 0;
  padding: 14px;
  overflow: auto;
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid var(--a3s-line);
  border-radius: 10px;
  backdrop-filter: blur(12px);
  scrollbar-color: var(--a3s-line-strong) transparent;
  scrollbar-width: thin;
}

.report-nav__context {
  min-width: 0;
  padding: 2px 6px 12px;
  border-bottom: 1px solid var(--a3s-line);
}

.report-nav__context span {
  color: var(--a3s-ink);
  font-size: 12px;
  font-weight: 650;
  line-height: 18px;
}

.report-nav__track {
  display: grid;
  min-width: 0;
  gap: 2px;
  padding-top: 8px;
}

.report-nav a {
  position: relative;
  display: grid;
  grid-template-columns: 22px minmax(0, 1fr);
  gap: 6px;
  align-items: start;
  min-height: 38px;
  min-width: 0;
  padding: 8px 8px 8px 10px;
  color: var(--a3s-muted);
  border-radius: 7px;
  font-size: 11px;
  line-height: 17px;
  text-decoration: none;
}

.report-nav a::after {
  position: absolute;
  top: 8px;
  bottom: 8px;
  left: 0;
  width: 2px;
  content: "";
  background: var(--a3s-blue);
  transform: scaleY(0);
  transform-origin: center;
}

.report-nav a:hover {
  color: var(--a3s-ink);
  background: var(--a3s-panel-soft);
}

.report-nav a[aria-current="location"] {
  color: var(--a3s-ink);
  background: color-mix(in srgb, var(--a3s-blue) 5%, var(--a3s-panel));
  font-weight: 600;
}

.report-nav a[aria-current="location"]::after {
  transform: scaleY(1);
}

.report-nav__index {
  color: var(--a3s-faint);
  font-family: var(--a3s-mono);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.report-nav a[aria-current="location"] .report-nav__index {
  color: var(--a3s-blue);
}

.report-nav__text {
  overflow-wrap: anywhere;
  text-wrap: pretty;
}

main {
  min-width: 0;
  padding: 0 64px 56px;
}

article {
  min-width: 0;
  max-width: 100%;
}

.report-section {
  position: relative;
  min-width: 0;
  padding: 40px 0;
  border-bottom: 1px solid var(--a3s-line);
  scroll-margin-top: 24px;
}

.report-section:last-child {
  border-bottom: 0;
}

.report-section h2 {
  max-width: 34ch;
  margin: 0 0 22px;
  color: var(--a3s-ink);
  font-size: 22px;
  font-weight: 650;
  line-height: 30px;
}

.report-section h3 {
  margin: 28px 0 12px;
  color: var(--a3s-ink);
  font-size: 16px;
  font-weight: 600;
  line-height: 24px;
}

.report-section h4 {
  margin: 22px 0 8px;
  font-size: 14px;
  font-weight: 600;
  line-height: 21px;
}

.report-section p,
.report-section li {
  max-width: 78ch;
}

.report-section p {
  margin: 0 0 16px;
}

.report-section ul,
.report-section ol {
  margin: 14px 0 20px;
  padding-left: 1.5rem;
}

.report-section li {
  margin: 8px 0;
}

.section-index {
  margin-bottom: 8px;
  color: var(--a3s-faint);
  font-family: var(--a3s-mono);
  font-size: 11px;
  line-height: 16px;
}

.direct-answer,
.section--lead {
  margin: 32px 0 0;
  padding: 24px 26px;
  background: var(--a3s-panel-soft);
  border: 1px solid var(--a3s-line);
  border-radius: 10px;
}

.direct-answer + .report-section,
.section--lead + .report-section {
  margin-top: 8px;
}

.narrative,
.section-body,
.prose {
  min-width: 0;
  max-width: 78ch;
}

.report-paragraph {
  margin: 0 0 20px;
  font-size: 15px;
  line-height: 1.82;
  text-wrap: pretty;
}

.report-paragraph:last-child {
  margin-bottom: 0;
}

.report-paragraph--implication {
  padding-left: 15px;
  border-left: 2px solid var(--a3s-line-strong);
}

.claim-sentence {
  scroll-margin-top: 24px;
}

.traceability {
  margin-top: 24px;
  padding: 12px 14px;
  background: var(--a3s-panel-soft);
  border: 1px solid var(--a3s-line);
  border-radius: 8px;
}

.traceability summary {
  color: var(--a3s-muted);
  cursor: pointer;
  font-size: 12px;
  font-weight: 600;
  line-height: 20px;
}

.traceability[open] summary {
  margin-bottom: 10px;
  color: var(--a3s-ink);
}

.traceability ol {
  margin: 0;
  padding-left: 1.25rem;
}

.traceability li {
  margin: 7px 0;
  color: var(--a3s-muted);
  font-size: 12px;
  line-height: 19px;
}

.traceability__basis,
.traceability__derivation {
  display: block;
  margin-top: 3px;
}

.citation {
  display: inline-flex;
  min-width: 1.6rem;
  justify-content: center;
  font-size: 0.82em;
  font-weight: 600;
  text-decoration: none;
}

.coverage {
  color: var(--a3s-muted);
}

.relations,
.limitations {
  margin-top: 24px;
  padding: 16px 18px;
  color: #654300;
  background: var(--a3s-warning-bg);
  border: 1px solid color-mix(in srgb, var(--a3s-warning) 24%, var(--a3s-line));
  border-radius: 8px;
}

.relations h3,
.limitations h3 {
  margin-top: 0;
  color: var(--a3s-warning);
}

.retained-excerpts {
  margin-top: 28px;
  padding-top: 6px;
  border-top: 1px solid var(--a3s-line);
}

.source-excerpt {
  margin: 18px 0;
}

.source-excerpt h4 {
  margin-bottom: 8px;
}

blockquote {
  margin: 20px 0;
  padding: 2px 0 2px 18px;
  color: var(--a3s-muted);
  border-left: 3px solid var(--a3s-line-strong);
}

pre {
  max-width: 100%;
  margin: 12px 0 20px;
  padding: 14px 16px;
  overflow: auto;
  color: var(--a3s-code-ink);
  background: var(--a3s-code-bg);
  border-radius: 8px;
  font-size: 12px;
  line-height: 19px;
  white-space: pre-wrap;
}

code {
  font-family: var(--a3s-mono);
}

:not(pre) > code {
  padding: 0.12em 0.32em;
  background: var(--a3s-panel-soft);
  border: 1px solid var(--a3s-line);
  border-radius: 4px;
  font-size: 0.9em;
}

.table-wrap {
  position: relative;
  width: 100%;
  margin: 22px 0;
  overflow-x: auto;
  border: 1px solid var(--a3s-line);
  border-radius: 8px;
  overscroll-behavior-inline: contain;
}

.table-wrap::after {
  position: sticky;
  right: 8px;
  bottom: 6px;
  display: none;
  float: right;
  padding: 3px 7px;
  color: var(--a3s-muted);
  background: rgba(255, 255, 255, 0.94);
  border: 1px solid var(--a3s-line);
  border-radius: 5px;
  content: var(--table-scroll-hint);
  font-size: 10px;
}

table {
  width: 100%;
  min-width: 720px;
  border-collapse: collapse;
  font-size: 13px;
  line-height: 21px;
}

th,
td {
  padding: 11px 13px;
  text-align: left;
  vertical-align: top;
  border-right: 1px solid var(--a3s-line);
  border-bottom: 1px solid var(--a3s-line);
}

th:last-child,
td:last-child {
  border-right: 0;
}

tr:last-child td {
  border-bottom: 0;
}

th {
  color: var(--a3s-ink);
  background: var(--a3s-panel-soft);
  font-weight: 600;
}

.sources ol {
  padding-left: 1.55rem;
}

.sources li {
  padding: 8px 0 12px;
  border-bottom: 1px solid var(--a3s-line);
}

.sources li:last-child {
  border-bottom: 0;
}

.source-meta {
  display: block;
  margin-top: 4px;
  color: var(--a3s-muted);
  font-size: 12px;
  line-height: 18px;
}

.key-points-list,
.timeline-list,
.process-list {
  border-top: 1px solid var(--a3s-line);
}

.key-point,
.timeline-entry,
.process-step {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr);
  gap: 12px;
  padding: 18px 0;
  border-bottom: 1px solid var(--a3s-line);
}

.key-point-number,
.timeline-marker,
.process-number {
  color: var(--a3s-faint);
  font-family: var(--a3s-mono);
  font-size: 11px;
  line-height: 20px;
}

.composition-content h3 {
  margin-top: 0;
}

.composition-comparison,
.composition-evidence,
.composition-source-ledger {
  width: 100%;
}

.composition-source-ledger .section-body > ul,
.composition-evidence .section-body > ul {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0 24px;
  padding: 0;
  list-style: none;
}

.composition-source-ledger .section-body > ul > li,
.composition-evidence .section-body > ul > li {
  min-width: 0;
  margin: 0;
  padding: 14px 0;
  border-top: 1px solid var(--a3s-line);
}

.footer-note {
  margin: 0;
  padding: 18px 64px;
  color: var(--a3s-muted);
  background: var(--a3s-panel-soft);
  border-top: 1px solid var(--a3s-line);
  font-size: 11px;
  line-height: 17px;
}

@media (max-width: 1180px) and (min-width: 821px) {
  .report-shell {
    width: min(100% - 24px, 1120px);
    grid-template-columns: 56px minmax(0, 1fr) 208px;
    gap: 12px;
    margin-top: 12px;
  }
}

@media (max-width: 820px) {
  .report-shell {
    display: flex;
    width: min(100% - 24px, 780px);
    flex-direction: column;
    gap: 8px;
    margin: 12px auto 40px;
  }

  .report-hero {
    padding: 36px 32px 30px;
  }

  .report-nav {
    position: static;
    order: -1;
    width: 100%;
    max-height: none;
    padding: 11px 12px;
    overflow: hidden;
  }

  .report-nav__context {
    padding: 0 4px 8px;
  }

  .report-nav__track {
    display: flex;
    gap: 4px;
    padding-top: 7px;
    overflow-x: auto;
    overscroll-behavior-inline: contain;
    scrollbar-color: var(--a3s-line-strong) transparent;
    scrollbar-width: thin;
  }

  .report-nav a {
    flex: 0 0 min(220px, 72vw);
    min-height: 36px;
    padding: 7px 7px 7px 9px;
  }

  main {
    padding: 0 32px 40px;
  }

  .footer-note {
    padding: 16px 32px;
  }

  .composition-source-ledger .section-body > ul,
  .composition-evidence .section-body > ul {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 640px) {
  html {
    scroll-padding-top: 16px;
  }

  .report-shell {
    width: 100%;
    margin: 0;
  }

  .report-column {
    border-right: 0;
    border-left: 0;
    border-radius: 0;
  }

  .report-hero {
    padding: 30px 20px 26px;
  }

  .report-hero h1 {
    font-size: 28px;
    line-height: 36px;
  }

  .report-thesis,
  .hero-thesis {
    margin-top: 16px;
    font-size: 15px;
  }

  .report-nav {
    align-self: stretch;
    margin: 8px;
    width: auto;
  }

  main {
    padding: 0 20px 32px;
  }

  .report-section {
    padding: 32px 0;
  }

  .direct-answer,
  .section--lead {
    margin-top: 20px;
    padding: 20px;
  }

  .table-wrap::after {
    display: block;
  }

  .key-point,
  .timeline-entry,
  .process-step {
    grid-template-columns: 26px minmax(0, 1fr);
    gap: 8px;
  }

  .footer-note {
    padding: 15px 20px;
  }
}

@media (prefers-reduced-motion: reduce) {
  html {
    scroll-behavior: auto;
  }

  *,
  *::before,
  *::after {
    scroll-behavior: auto !important;
    transition-duration: 0.01ms !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
  }
}

@media print {
  :root {
    --a3s-bg: #ffffff;
    --a3s-panel: #ffffff;
    --a3s-panel-soft: #ffffff;
    --a3s-ink: #000000;
    --a3s-muted: #444444;
    --a3s-line: #d8d8d8;
  }

  @page {
    margin: 16mm 17mm;
  }

  body {
    color: #000000;
    background: #ffffff;
    font-size: 10.5pt;
  }

  .skip-link,
  .report-nav,
  .footer-note {
    display: none;
  }

  .report-shell {
    display: block;
    width: 100%;
    margin: 0;
    overflow: visible;
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }

  .report-column {
    overflow: visible;
    border: 0;
    border-radius: 0;
    box-shadow: none;
  }

  .report-hero {
    padding: 0 0 10mm;
    border-bottom: 1px solid #bdbdbd;
    box-shadow: none;
  }

  .report-hero h1 {
    max-width: none;
    font-size: 25pt;
  }

  main {
    padding: 0;
  }

  article,
  .report-section,
  .section-body {
    width: 100%;
    max-width: 100%;
    min-width: 0;
  }

  .report-section {
    padding: 8mm 0;
    break-inside: auto;
  }

  .direct-answer,
  .section--lead {
    padding: 6mm;
    border: 1px solid #cfcfcf;
  }

  .report-paragraph,
  .traceability,
  .limitations,
  .relations {
    break-inside: avoid;
    background: #ffffff;
  }

  .key-point,
  .timeline-entry,
  .process-step,
  tr {
    break-inside: avoid;
  }

  .table-wrap {
    overflow: visible;
    border: 1px solid #cfcfcf;
  }

  .composition-source-ledger .section-body > ul,
  .composition-evidence .section-body > ul {
    grid-template-columns: 1fr;
  }

  .composition-source-ledger .section-body > ul > li,
  .composition-evidence .section-body > ul > li {
    break-inside: avoid;
  }

  .table-wrap::after {
    display: none;
  }

  table {
    min-width: 0;
    font-size: 8.5pt;
  }

  pre {
    color: #000000;
    background: #f3f3f3;
    border: 1px solid #d8d8d8;
  }

  a {
    color: #000000;
    text-decoration: none;
  }
}
"#;
