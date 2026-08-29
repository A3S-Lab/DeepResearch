pub(crate) const REPORT_VISUAL_CSS: &str = r#"
.research-visual {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  margin: 30px 0 28px;
  overflow: clip;
  background: var(--a3s-panel);
  border: 1px solid var(--a3s-line);
  border-radius: 10px;
}

.research-visual__header {
  display: flex;
  gap: 16px;
  align-items: baseline;
  justify-content: space-between;
  padding: 18px 20px 14px;
  border-bottom: 1px solid var(--a3s-line);
}

.report-section .research-visual__header h3 {
  margin: 0;
  font-size: 15px;
  line-height: 22px;
}

.report-section .research-visual__header p {
  flex: 0 1 auto;
  margin: 0;
  color: var(--a3s-muted);
  font-size: 11px;
  line-height: 17px;
  text-align: right;
}

.research-visual__plot {
  width: 100%;
  min-width: 0;
  max-width: 100%;
  padding: 12px 14px 4px;
  overflow: hidden;
  background: var(--a3s-panel);
}

.research-visual svg {
  display: block;
  width: 100%;
  height: auto;
  min-height: 280px;
  max-height: 440px;
  font-family: var(--a3s-font);
}

.chart-grid {
  stroke: var(--a3s-line);
  stroke-width: 1;
}

.chart-axis,
.chart-mark--timeline line {
  stroke: var(--a3s-line-strong);
  stroke-width: 1.5;
}

.chart-line {
  fill: none;
  stroke: var(--a3s-blue);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 2.5;
}

.chart-mark rect,
.chart-mark circle {
  fill: var(--a3s-blue);
}

.chart-mark--point circle,
.chart-mark--timeline circle {
  stroke: var(--a3s-panel);
  stroke-width: 2;
}

.chart-tick,
.chart-category,
.chart-axis-title,
.chart-point-label,
.chart-kpi__label {
  fill: var(--a3s-muted);
  font-size: 11px;
}

.chart-value {
  fill: var(--a3s-ink);
  font-size: 11px;
  font-weight: 600;
}

.chart-axis-title {
  font-weight: 600;
}

.chart-kpi rect {
  fill: var(--a3s-panel-soft);
  stroke: var(--a3s-line);
  stroke-width: 1;
}

.chart-kpi__value {
  fill: var(--a3s-ink);
  font-size: 24px;
  font-weight: 650;
  letter-spacing: -0.02em;
}

.chart-kpi__label {
  font-size: 12px;
  font-weight: 600;
}

.research-visual__table {
  width: auto;
  min-width: 0;
  max-width: calc(100% - 40px);
  margin: 0 20px 18px;
}

.research-visual__table table {
  min-width: 0;
  font-size: 12px;
}

.research-visual__table th,
.research-visual__table td {
  width: 50%;
  padding: 9px 11px;
}

.research-visual__table tbody th {
  background: var(--a3s-panel);
  font-weight: 600;
}

.research-visual__raw {
  display: block;
  margin-top: 2px;
  color: var(--a3s-muted);
  font-size: 10px;
  font-weight: 400;
  line-height: 16px;
}

.report-section .research-visual figcaption {
  max-width: none;
  margin: 0;
  padding: 13px 20px 15px;
  color: var(--a3s-muted);
  background: var(--a3s-panel-soft);
  border-top: 1px solid var(--a3s-line);
  font-size: 12px;
  line-height: 19px;
}

@media (max-width: 640px) {
  .research-visual__header {
    display: block;
    padding: 15px 16px 12px;
  }

  .report-section .research-visual__header p {
    margin-top: 4px;
    text-align: left;
  }

  .research-visual__plot {
    padding: 8px 4px 0;
    overflow-x: auto;
  }

  .research-visual svg {
    width: 700px;
    max-width: none;
  }

  .research-visual__table {
    max-width: calc(100% - 32px);
    margin: 0 16px 16px;
  }
}

@media print {
  .research-visual__plot {
    padding-right: 0;
    padding-left: 0;
  }

  .research-visual svg {
    max-height: 110mm;
  }
}
"#;
