use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::language::{
    infer_deep_research_output_language, validate_deep_research_output_language,
};
use crate::planner::deep_research_loop_contract_for_language;
use crate::report::validate_deep_research_run_id;
use crate::workflow::retrieval_workflow_source;

const MAX_QUERY_CHARS: usize = 16_000;
const MAX_WORKSPACE_SOURCE_HINTS: usize = 8;
const MAX_WORKSPACE_SOURCE_PATH_CHARS: usize = 1_200;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceScope {
    LocalOnly,
    #[default]
    WebAndWorkspace,
}

impl EvidenceScope {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LocalOnly => "local_only",
            Self::WebAndWorkspace => "web_and_workspace",
        }
    }

    pub const fn network_enabled(self) -> bool {
        matches!(self, Self::WebAndWorkspace)
    }

    fn planner_label(self) -> &'static str {
        match self {
            Self::LocalOnly => "offline/local-only evidence",
            Self::WebAndWorkspace => {
                "web available; workspace only when the request explicitly depends on local artifacts"
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSourceHint {
    pub path: String,
}

impl WorkspaceSourceHint {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    fn normalized_path(&self) -> Result<String, String> {
        let path = self.path.trim().replace('\\', "/");
        if path.is_empty()
            || path.chars().count() > MAX_WORKSPACE_SOURCE_PATH_CHARS
            || path.starts_with('/')
            || path.contains('\0')
        {
            return Err("workspace source hints require a bounded relative path".to_string());
        }
        let components = path.split('/').collect::<Vec<_>>();
        if components
            .iter()
            .any(|component| component.is_empty() || matches!(*component, "." | ".."))
        {
            return Err(
                "workspace source hints must not contain empty, current, or parent components"
                    .to_string(),
            );
        }
        Ok(components.join("/"))
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeepResearchRequestLimits {
    pub max_tracks: u8,
    pub local_max_steps: u8,
    pub workflow_timeout_ms: u64,
    pub max_tool_calls: u16,
    pub max_output_bytes: usize,
}

impl Default for DeepResearchRequestLimits {
    fn default() -> Self {
        Self {
            max_tracks: 4,
            local_max_steps: 4,
            workflow_timeout_ms: 600_000,
            max_tool_calls: 240,
            max_output_bytes: 2 * 1024 * 1024,
        }
    }
}

impl DeepResearchRequestLimits {
    fn validate(self) -> Result<(), String> {
        if !(1..=4).contains(&self.max_tracks)
            || !(1..=4).contains(&self.local_max_steps)
            || !(30_000..=600_000).contains(&self.workflow_timeout_ms)
            || !(4..=240).contains(&self.max_tool_calls)
            || !(256 * 1024..=2 * 1024 * 1024).contains(&self.max_output_bytes)
        {
            return Err(
                "DeepResearch request limits exceed the closed retrieval safety envelope"
                    .to_string(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DeepResearchRequest {
    pub run_id: String,
    pub query: String,
    #[serde(default = "default_output_language")]
    pub output_language: String,
    pub current_date: String,
    pub evidence_scope: EvidenceScope,
    #[serde(default)]
    pub workspace_source_hints: Vec<WorkspaceSourceHint>,
    #[serde(default)]
    pub limits: DeepResearchRequestLimits,
}

impl DeepResearchRequest {
    pub fn new(
        run_id: impl Into<String>,
        query: impl Into<String>,
        evidence_scope: EvidenceScope,
    ) -> Self {
        let query = query.into();
        Self {
            run_id: run_id.into(),
            output_language: infer_deep_research_output_language(&query),
            query,
            current_date: chrono::Local::now().date_naive().to_string(),
            evidence_scope,
            workspace_source_hints: Vec::new(),
            limits: DeepResearchRequestLimits::default(),
        }
    }

    pub fn with_current_date(mut self, current_date: impl Into<String>) -> Self {
        self.current_date = current_date.into();
        self
    }

    pub fn with_output_language(mut self, output_language: impl Into<String>) -> Self {
        self.output_language = output_language.into();
        self
    }

    pub fn with_workspace_source_hints(
        mut self,
        workspace_source_hints: Vec<WorkspaceSourceHint>,
    ) -> Self {
        self.workspace_source_hints = workspace_source_hints;
        self
    }

    pub fn with_limits(mut self, limits: DeepResearchRequestLimits) -> Self {
        self.limits = limits;
        self
    }

    pub fn validate(&self) -> Result<(), String> {
        validate_deep_research_run_id(&self.run_id)?;
        let query = self.query.trim();
        if query.is_empty() || query.chars().count() > MAX_QUERY_CHARS {
            return Err("DeepResearch query must be non-empty and bounded".to_string());
        }
        validate_deep_research_output_language(&self.output_language)?;
        chrono::NaiveDate::parse_from_str(self.current_date.trim(), "%Y-%m-%d")
            .map_err(|_| "DeepResearch current date must use YYYY-MM-DD".to_string())?;
        self.limits.validate()?;
        if self.workspace_source_hints.len() > MAX_WORKSPACE_SOURCE_HINTS {
            return Err(format!(
                "DeepResearch accepts at most {MAX_WORKSPACE_SOURCE_HINTS} workspace source hints"
            ));
        }
        let mut paths = BTreeSet::new();
        for hint in &self.workspace_source_hints {
            let path = hint.normalized_path()?;
            if !paths.insert(path) {
                return Err("DeepResearch workspace source hints must be unique".to_string());
            }
        }
        Ok(())
    }

    pub fn to_workflow_arguments(&self) -> Result<Value, String> {
        self.validate()?;
        let hints = self
            .workspace_source_hints
            .iter()
            .map(|hint| {
                hint.normalized_path()
                    .map(|path| serde_json::json!({ "path": path }))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let loop_contract = deep_research_loop_contract_for_language(
            self.query.trim(),
            self.current_date.trim(),
            self.evidence_scope.planner_label(),
            usize::from(self.limits.max_tracks),
            self.output_language.trim(),
        );
        Ok(serde_json::json!({
            "source": retrieval_workflow_source(),
            "run_id": self.run_id,
            "input": {
                "query": self.query.trim(),
                "output_language": self.output_language.trim(),
                "inquiry_host_managed": true,
                "current_date": self.current_date.trim(),
                "loop_contract": loop_contract,
                "evidence_scope": self.evidence_scope.as_str(),
                "workspace_source_hints": hints,
                "local_max_steps": self.limits.local_max_steps,
                "workflow_timeout_ms": self.limits.workflow_timeout_ms,
            },
            "limits": {
                "timeoutMs": self.limits.workflow_timeout_ms,
                "maxToolCalls": self.limits.max_tool_calls,
                "maxOutputBytes": self.limits.max_output_bytes,
                "maxConcurrentGenerations": 2,
            }
        }))
    }
}

fn default_output_language() -> String {
    "en".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_request_builds_the_closed_workflow_contract() {
        let request = DeepResearchRequest::new(
            "run-20260725-01",
            "Assess the migration",
            EvidenceScope::LocalOnly,
        )
        .with_current_date("2026-07-25")
        .with_workspace_source_hints(vec![WorkspaceSourceHint::new("docs/migration.md")]);

        let arguments = request
            .to_workflow_arguments()
            .expect("typed request should compile");

        assert_eq!(arguments["run_id"], "run-20260725-01");
        assert_eq!(arguments["input"]["evidence_scope"], "local_only");
        assert_eq!(
            arguments["input"]["workspace_source_hints"][0]["path"],
            "docs/migration.md"
        );
        assert_eq!(
            arguments["input"]["loop_contract"]["goal"],
            "Assess the migration"
        );
        assert_eq!(arguments["limits"]["maxToolCalls"], 240);
        assert_eq!(arguments["limits"]["maxConcurrentGenerations"], 2);
        assert_eq!(arguments["input"]["output_language"], "en");
        assert!(arguments["input"]["loop_contract"]["planner"]["prompt"]
            .as_str()
            .is_some_and(|prompt| prompt.contains("Output language: en")));
    }

    #[test]
    fn typed_request_infers_and_pins_the_users_output_language() {
        let inferred = DeepResearchRequest::new(
            "run-zh",
            "比较 A3S Code TUI 与 Web 的深度研究实现",
            EvidenceScope::WebAndWorkspace,
        )
        .with_current_date("2026-07-25");
        let inferred_arguments = inferred
            .to_workflow_arguments()
            .expect("Chinese request should compile");

        assert_eq!(inferred.output_language, "zh");
        assert_eq!(inferred_arguments["input"]["output_language"], "zh");
        let planner_prompt = inferred_arguments["input"]["loop_contract"]["planner"]["prompt"]
            .as_str()
            .expect("planner prompt");
        assert!(planner_prompt.contains("Output language: zh"));
        assert!(planner_prompt
            .contains("Supplemental queries may use the language of the strongest likely source"));

        let mixed_product_names = DeepResearchRequest::new(
            "run-zh-products",
            "基于一手资料，比较 Stanford STORM、OpenAI deep research 与 A3S DeepResearch 的方法与局限。",
            EvidenceScope::WebAndWorkspace,
        );
        assert_eq!(mixed_product_names.output_language, "zh");

        let explicit = DeepResearchRequest::new(
            "run-fr",
            "Compare the release policies",
            EvidenceScope::WebAndWorkspace,
        )
        .with_current_date("2026-07-25")
        .with_output_language("fr-CA");
        assert_eq!(
            explicit
                .to_workflow_arguments()
                .expect("explicit language should compile")["input"]["output_language"],
            "fr-CA"
        );
    }

    #[test]
    fn typed_request_rejects_an_invalid_output_language() {
        let request =
            DeepResearchRequest::new("run-1", "Inspect context", EvidenceScope::WebAndWorkspace)
                .with_current_date("2026-07-25")
                .with_output_language("../zh");

        assert!(request.validate().is_err());
    }

    #[test]
    fn typed_request_rejects_unsafe_or_duplicate_workspace_hints() {
        let unsafe_request =
            DeepResearchRequest::new("run-1", "Inspect context", EvidenceScope::WebAndWorkspace)
                .with_current_date("2026-07-25")
                .with_workspace_source_hints(vec![WorkspaceSourceHint::new("../secret")]);
        assert!(unsafe_request.validate().is_err());

        let duplicate_request =
            DeepResearchRequest::new("run-1", "Inspect context", EvidenceScope::WebAndWorkspace)
                .with_current_date("2026-07-25")
                .with_workspace_source_hints(vec![
                    WorkspaceSourceHint::new("docs/context.md"),
                    WorkspaceSourceHint::new("docs/context.md"),
                ]);
        assert!(duplicate_request.validate().is_err());
    }

    #[test]
    fn typed_request_rejects_an_unsafe_run_identity() {
        let request = DeepResearchRequest::new("../run", "query", EvidenceScope::WebAndWorkspace)
            .with_current_date("2026-07-25");
        assert!(request.validate().is_err());
    }
}
