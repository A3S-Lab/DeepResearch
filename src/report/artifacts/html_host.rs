//! Fixed host chrome for standalone DeepResearch HTML artifacts.
//!
//! The report body remains entirely data-driven. This module is the only
//! authority allowed to add executable content to a report artifact.

use std::borrow::Cow;

pub(crate) const REPORT_HOST_SCRIPT_ATTRIBUTE: &str = r#"data-a3s-report-host="v1""#;
pub(crate) const REPORT_HOST_UI_ATTRIBUTE: &str = r#"data-a3s-report-host-ui="v1""#;

pub(crate) const REPORT_HOST_SCRIPT: &str = r##"(() => {
  "use strict";

  const host = document.querySelector('[data-a3s-report-host-ui="v1"]');
  if (!host) return;

  const editButton = host.querySelector('[data-a3s-action="edit"]');
  const saveButton = host.querySelector('[data-a3s-action="save"]');
  const printButton = host.querySelector('[data-a3s-action="print"]');
  const status = host.querySelector('[data-a3s-report-status]');
  const editableRegions = Array.from(document.querySelectorAll('[data-a3s-editable-region]'));
  const tocLinks = Array.from(document.querySelectorAll('.report-nav a[href^="#"]'));
  const tocEntries = tocLinks.map((link) => ({
    link,
    target: document.getElementById(link.getAttribute('href').slice(1))
  })).filter((entry) => entry.target);
  let editing = false;
  let tocFrame = 0;

  const announce = (state) => {
    if (status && status.dataset[state]) status.textContent = status.dataset[state];
  };

  const updateEditControl = (root, active) => {
    const button = root.querySelector('[data-a3s-action="edit"]');
    if (!button) return;
    const label = button.querySelector('[data-a3s-action-label]');
    const text = active ? button.dataset.labelDone : button.dataset.labelEdit;
    button.setAttribute('aria-pressed', String(active));
    button.setAttribute('aria-label', text);
    button.setAttribute('title', text);
    if (label) label.textContent = text;
  };

  const setEditing = (active) => {
    editing = active;
    document.body.classList.toggle('is-editing', active);
    document.body.dataset.a3sReportState = active ? 'editing' : 'readonly';
    editableRegions.forEach((region) => {
      region.setAttribute('contenteditable', String(active));
      region.setAttribute('spellcheck', String(active));
    });
    updateEditControl(host, active);
    announce(active ? 'editing' : 'readonly');
    if (active && editableRegions[0]) editableRegions[0].focus({ preventScroll: true });
  };

  const safeUrl = (value, source) => {
    const normalized = value.trim().toLowerCase();
    if (source) {
      return normalized.startsWith('https://') ||
        normalized.startsWith('http://') ||
        normalized.startsWith('data:image/');
    }
    return normalized.startsWith('#') ||
      normalized.startsWith('https://') ||
      normalized.startsWith('http://') ||
      normalized.startsWith('mailto:');
  };

  const sanitizeEditableClone = (clone) => {
    clone.querySelectorAll('[data-a3s-editable-region]').forEach((region) => {
      region.removeAttribute('contenteditable');
      region.removeAttribute('spellcheck');
      region.querySelectorAll('script,style,iframe,object,embed,link,meta,base,form,input,button,textarea,select').forEach((node) => node.remove());
      region.querySelectorAll('*').forEach((node) => {
        Array.from(node.attributes).forEach((attribute) => {
          const name = attribute.name.toLowerCase();
          if (name.startsWith('on') || name === 'srcdoc' || name === 'style') {
            node.removeAttribute(attribute.name);
          } else if (name === 'href' && !safeUrl(attribute.value, false)) {
            node.removeAttribute(attribute.name);
          } else if (name === 'src' && !safeUrl(attribute.value, true)) {
            node.removeAttribute(attribute.name);
          }
        });
      });
    });
  };

  const normalizeExportState = (clone) => {
    const body = clone.querySelector('body');
    if (body) {
      body.classList.remove('is-editing');
      body.dataset.a3sReportState = 'readonly';
    }
    const clonedHost = clone.querySelector('[data-a3s-report-host-ui="v1"]');
    if (clonedHost) {
      updateEditControl(clonedHost, false);
      const clonedStatus = clonedHost.querySelector('[data-a3s-report-status]');
      if (clonedStatus) clonedStatus.textContent = clonedStatus.dataset.readonly;
    }
    clone.querySelectorAll('.report-nav a[aria-current]').forEach((link) => link.removeAttribute('aria-current'));
    sanitizeEditableClone(clone);
  };

  const reportFileName = () => {
    const title = (document.querySelector('.report-hero h1')?.textContent || document.title || 'a3s-research-report')
      .trim()
      .replace(/[\\/:*?"<>|\u0000-\u001f]/g, '-')
      .replace(/\s+/g, ' ')
      .slice(0, 80)
      .replace(/[. ]+$/g, '');
    return `${title || 'a3s-research-report'}.html`;
  };

  const saveHtml = () => {
    try {
      const clone = document.documentElement.cloneNode(true);
      normalizeExportState(clone);
      const blob = new Blob([`<!doctype html>\n${clone.outerHTML}`], { type: 'text/html;charset=utf-8' });
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = reportFileName();
      link.hidden = true;
      document.body.appendChild(link);
      link.click();
      link.remove();
      setTimeout(() => URL.revokeObjectURL(url), 0);
      announce('saved');
    } catch (_error) {
      announce('error');
    }
  };

  const updateToc = () => {
    tocFrame = 0;
    if (!tocEntries.length) return;
    let current = tocEntries[0];
    tocEntries.forEach((entry) => {
      if (entry.target.getBoundingClientRect().top <= 144) current = entry;
    });
    tocLinks.forEach((link) => link.removeAttribute('aria-current'));
    current.link.setAttribute('aria-current', 'location');
  };

  const scheduleTocUpdate = () => {
    if (!tocFrame) tocFrame = requestAnimationFrame(updateToc);
  };

  const navigateToSection = (event, entry) => {
    event.preventDefault();
    const href = entry.link.getAttribute('href');
    entry.target.scrollIntoView({
      behavior: 'auto',
      block: 'start'
    });
    history.replaceState(null, '', href);
    tocLinks.forEach((link) => link.removeAttribute('aria-current'));
    entry.link.setAttribute('aria-current', 'location');
    scheduleTocUpdate();
  };

  const syncEditedLabels = (event) => {
    if (!editing || !event.target.closest('[data-a3s-editable-region]')) return;
    const title = document.querySelector('.report-hero h1');
    if (title?.textContent.trim()) document.title = title.textContent.trim();
    tocEntries.forEach((entry) => {
      const heading = entry.target.querySelector('h2');
      const label = entry.link.querySelector('.report-nav__text');
      if (heading && label && heading.textContent.trim()) label.textContent = heading.textContent.trim();
    });
  };

  editButton?.addEventListener('click', () => setEditing(!editing));
  saveButton?.addEventListener('click', saveHtml);
  printButton?.addEventListener('click', () => window.print());
  document.addEventListener('input', syncEditedLabels);
  tocEntries.forEach((entry) => {
    entry.link.addEventListener('click', (event) => navigateToSection(event, entry));
  });
  window.addEventListener('scroll', scheduleTocUpdate, { passive: true });
  window.addEventListener('resize', scheduleTocUpdate, { passive: true });
  window.addEventListener('hashchange', scheduleTocUpdate);
  updateToc();
})();"##;

pub(crate) const REPORT_HOST_CSS: &str = r#"
.report-menu {
  position: sticky;
  top: 24px;
  z-index: 6;
  grid-column: 1;
  grid-row: 1;
  align-self: start;
  min-width: 0;
  padding: 14px;
  color: var(--a3s-ink);
  background: rgba(255, 255, 255, 0.96);
  border: 1px solid var(--a3s-line);
  border-radius: 10px;
  backdrop-filter: blur(12px);
}

.report-menu__brand {
  display: grid;
  gap: 1px;
  padding: 2px 6px 13px;
  border-bottom: 1px solid var(--a3s-line);
}

.report-menu__brand span {
  color: var(--a3s-blue);
  font-family: var(--a3s-mono);
  font-size: 10px;
  font-weight: 700;
  letter-spacing: 0.08em;
}

.report-menu__brand strong {
  font-size: 13px;
  font-weight: 650;
  line-height: 20px;
}

.report-menu__meta {
  display: grid;
  gap: 5px;
  padding: 12px 6px 10px;
  color: var(--a3s-muted);
  font-size: 11px;
  line-height: 17px;
}

.report-menu__meta span {
  display: grid;
  grid-template-columns: 6px minmax(0, 1fr);
  gap: 7px;
  align-items: start;
}

.report-menu__meta span::before {
  width: 5px;
  height: 5px;
  margin-top: 6px;
  content: "";
  background: var(--a3s-ink);
  border-radius: 50%;
}

.report-menu__actions {
  display: grid;
  gap: 3px;
  margin-top: 4px;
}

.report-menu__action {
  display: grid;
  width: 100%;
  min-width: 0;
  min-height: 36px;
  grid-template-columns: 24px minmax(0, 1fr);
  gap: 7px;
  align-items: center;
  padding: 5px 8px 5px 6px;
  color: var(--a3s-muted);
  background: transparent;
  border: 1px solid transparent;
  border-radius: 7px;
  font: inherit;
  font-size: 12px;
  line-height: 18px;
  text-align: left;
  cursor: pointer;
}

.report-menu__action:hover {
  color: var(--a3s-ink);
  background: var(--a3s-panel-soft);
}

.report-menu__action[aria-pressed="true"] {
  color: var(--a3s-blue);
  background: color-mix(in srgb, var(--a3s-blue) 6%, var(--a3s-panel));
  border-color: color-mix(in srgb, var(--a3s-blue) 20%, var(--a3s-line));
}

.report-menu__icon {
  display: grid;
  width: 24px;
  height: 24px;
  place-items: center;
  border: 1px solid var(--a3s-line);
  border-radius: 6px;
  background: var(--a3s-panel);
}

.report-menu__icon svg {
  width: 14px;
  height: 14px;
  fill: none;
  stroke: currentColor;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 1.75;
}

.report-menu__status {
  margin: 12px 6px 0;
  padding-top: 10px;
  color: var(--a3s-muted);
  border-top: 1px solid var(--a3s-line);
  font-size: 11px;
  line-height: 17px;
}

.report-menu__hint {
  margin: 4px 6px 0;
  color: var(--a3s-faint);
  font-size: 10px;
  line-height: 16px;
}

[data-a3s-editable-region] {
  outline: 0 solid transparent;
  outline-offset: -1px;
}

.is-editing [data-a3s-editable-region] {
  outline: 1px solid color-mix(in srgb, var(--a3s-blue) 42%, var(--a3s-line));
  background-color: color-mix(in srgb, var(--a3s-blue) 1.5%, var(--a3s-panel));
  caret-color: var(--a3s-blue);
}

.is-editing [data-a3s-editable-region]:focus {
  outline-color: var(--a3s-blue);
}

@media (max-width: 1180px) and (min-width: 821px) {
  .report-menu {
    padding: 9px;
  }

  .report-menu__brand {
    display: flex;
    justify-content: center;
    padding: 5px 0 12px;
  }

  .report-menu__brand strong,
  .report-menu__meta,
  .report-menu__action [data-a3s-action-label],
  .report-menu__status,
  .report-menu__hint {
    display: none;
  }

  .report-menu__action {
    width: 36px;
    min-height: 36px;
    grid-template-columns: 1fr;
    padding: 5px;
  }
}

@media (max-width: 820px) {
  .report-menu {
    position: static;
    order: -2;
    align-self: stretch;
    display: grid;
    width: auto;
    grid-template-columns: minmax(112px, 1fr) auto;
    gap: 8px 16px;
    align-items: center;
    padding: 10px 12px;
  }

  .report-menu__brand {
    padding: 0;
    border: 0;
  }

  .report-menu__meta,
  .report-menu__status,
  .report-menu__hint {
    display: none;
  }

  .report-menu__actions {
    display: flex;
    gap: 4px;
    margin: 0;
  }

  .report-menu__action {
    width: 36px;
    min-height: 36px;
    grid-template-columns: 1fr;
    padding: 5px;
  }

  .report-menu__action [data-a3s-action-label] {
    display: none;
  }
}

@media (max-width: 640px) {
  .report-menu {
    margin: 8px 8px 0;
  }
}

@media print {
  .report-menu {
    display: none;
  }

  [data-a3s-editable-region] {
    outline: 0 !important;
    background: transparent !important;
  }
}
"#;

#[derive(Clone, Copy)]
struct ReportHostLabels {
    menu: &'static str,
    product: &'static str,
    actions: &'static str,
    edit: &'static str,
    done: &'static str,
    save: &'static str,
    print: &'static str,
    readonly: &'static str,
    editing: &'static str,
    saved: &'static str,
    error: &'static str,
    hint: &'static str,
}

pub(crate) fn render_report_menu(
    language: &str,
    primary_status: &str,
    secondary_status: Option<&str>,
) -> String {
    let labels = report_host_labels(language);
    let secondary = secondary_status
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("<span>{}</span>", escape_html(value)))
        .unwrap_or_default();
    format!(
        r#"<aside class="report-menu" {ui_attribute} aria-label="{menu}">
<div class="report-menu__brand"><span>A3S</span><strong>{product}</strong></div>
<div class="report-menu__meta"><span>{primary_status}</span>{secondary}</div>
<div class="report-menu__actions" role="group" aria-label="{actions}">
<button class="report-menu__action" type="button" data-a3s-action="edit" data-label-edit="{edit}" data-label-done="{done}" aria-label="{edit}" aria-pressed="false" title="{edit}"><span class="report-menu__icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L8 18l-4 1 1-4Z"/></svg></span><span data-a3s-action-label>{edit}</span></button>
<button class="report-menu__action" type="button" data-a3s-action="save" aria-label="{save}" title="{save}"><span class="report-menu__icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M12 3v12"/><path d="m7 10 5 5 5-5"/><path d="M5 21h14"/></svg></span><span data-a3s-action-label>{save}</span></button>
<button class="report-menu__action" type="button" data-a3s-action="print" aria-label="{print}" title="{print}"><span class="report-menu__icon" aria-hidden="true"><svg viewBox="0 0 24 24"><path d="M6 9V2h12v7"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><path d="M6 14h12v8H6z"/></svg></span><span data-a3s-action-label>{print}</span></button>
</div>
<p class="report-menu__status" data-a3s-report-status data-readonly="{readonly}" data-editing="{editing}" data-saved="{saved}" data-error="{error}" aria-live="polite">{readonly}</p>
<p class="report-menu__hint">{hint}</p>
</aside>"#,
        ui_attribute = REPORT_HOST_UI_ATTRIBUTE,
        menu = labels.menu,
        product = labels.product,
        primary_status = escape_html(primary_status),
        actions = labels.actions,
        edit = labels.edit,
        done = labels.done,
        save = labels.save,
        print = labels.print,
        readonly = labels.readonly,
        editing = labels.editing,
        saved = labels.saved,
        error = labels.error,
        hint = labels.hint,
    )
}

pub(crate) fn report_host_script_element() -> String {
    format!(
        "<script {}>\n{}\n</script>",
        REPORT_HOST_SCRIPT_ATTRIBUTE, REPORT_HOST_SCRIPT
    )
}

pub(crate) fn document_has_only_fixed_host_script(document: &str) -> bool {
    let expected = report_host_script_element();
    if document.matches(&expected).count() != 1 {
        return false;
    }
    !document
        .replacen(&expected, "", 1)
        .to_ascii_lowercase()
        .contains("<script")
}

pub(crate) fn document_without_fixed_host_script(document: &str) -> Cow<'_, str> {
    let expected = report_host_script_element();
    if document.matches(&expected).count() == 1 {
        Cow::Owned(document.replacen(&expected, "", 1))
    } else {
        Cow::Borrowed(document)
    }
}

fn report_host_labels(language: &str) -> ReportHostLabels {
    if crate::language::primary_output_language(language) == "zh" {
        ReportHostLabels {
            menu: "报告菜单",
            product: "深度研究",
            actions: "报告操作",
            edit: "编辑报告",
            done: "完成编辑",
            save: "保存 HTML",
            print: "打印报告",
            readonly: "只读",
            editing: "编辑中 · 修改保留在当前页面",
            saved: "HTML 副本已保存",
            error: "保存失败，请重试",
            hint: "编辑后保存为新的单文件副本",
        }
    } else {
        ReportHostLabels {
            menu: "Report menu",
            product: "Deep Research",
            actions: "Report actions",
            edit: "Edit report",
            done: "Finish editing",
            save: "Save HTML",
            print: "Print report",
            readonly: "Read only",
            editing: "Editing · changes remain on this page",
            saved: "HTML copy saved",
            error: "Save failed; try again",
            hint: "Save an updated single-file copy after editing",
        }
    }
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_menu_uses_the_report_language_and_escapes_status_text() {
        let menu = render_report_menu("zh-CN", "<可追溯>", Some("边界 & 置信度"));

        assert!(menu.contains("编辑报告"));
        assert!(menu.contains("保存 HTML"));
        assert!(menu.contains("&lt;可追溯&gt;"));
        assert!(menu.contains("边界 &amp; 置信度"));
        assert!(!menu.contains("<可追溯>"));
    }

    #[test]
    fn fixed_script_contract_rejects_modified_or_additional_scripts() {
        let trusted = format!("<html><body>{}</body></html>", report_host_script_element());
        assert!(document_has_only_fixed_host_script(&trusted));
        assert!(!document_has_only_fixed_host_script(
            &trusted.replace("window.print()", "window.alert(1)")
        ));
        assert!(!document_has_only_fixed_host_script(&format!(
            "{trusted}<script>unsafe()</script>"
        )));
    }

    #[test]
    fn audit_projection_removes_only_one_exact_fixed_host_script() {
        let fixed = report_host_script_element();
        assert!(!document_without_fixed_host_script(&fixed).contains("getAttribute('href')"));

        let modified = fixed.replace("window.print()", "window.print(); const href = 'fake.md'");
        assert!(document_without_fixed_host_script(&modified).contains("fake.md"));
    }
}
