//! Skill catalog + skill-routes (`GET /v1/skills`, `POST /v1/skills/{name}:run`).
//!
//! The catalog wraps `yanshi-skills::SkillRegistryIndex`. "Submit a
//! skill run" creates a `runs` row with the workflow column set to the
//! `@skill:<name>` marker (optionally followed by `\n---\n<user input>`)
//! and dispatches to the same `RunExecutor` that handles `/v1/runs` —
//! `RunContext::skill_dir` carries the resolved manifest directory
//! alongside it so `FlowRunExecutor` (W0.2, `crate::runs::skill_execute`)
//! builds and runs a real `SkillBuilder`-constructed agent instead of
//! trying to parse the marker as a Flow definition.

use axum::{
  Json,
  extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};
use yanshi_skills::{ResolvedSkillRegistryEntry, SkillError, SkillRegistryIndex};

use crate::AppState;
use crate::error::{ApiError, JsonReq};
use crate::runs::{CreateRunResponse, RetentionOverrides, RunOptions};
use crate::tenant::TenantId;
use axum::Extension;

/// Read-only view of a skill registry, suitable for serving over HTTP.
///
/// Holds the index plus the path it was loaded from so `resolve_skill`
/// can re-use the manifest's relative path conventions.
#[derive(Clone, Debug, Default)]
pub struct SkillCatalog {
  inner: Arc<SkillCatalogInner>,
}

#[derive(Debug, Default)]
struct SkillCatalogInner {
  index_path: Option<PathBuf>,
  index: Option<SkillRegistryIndex>,
}

impl SkillCatalog {
  /// Empty catalog used when no skills are configured. `GET /v1/skills`
  /// returns `[]` and skill-run requests fail with 404.
  pub fn empty() -> Self {
    Self::default()
  }

  /// Load from `YANSHI_SKILLS_INDEX` (path to `skills.index.toml`).
  /// Returns an empty catalog when the env var is unset; logs a warning
  /// when the env var is set but loading fails so operators can see config
  /// errors without crash-looping the gateway.
  pub fn from_env() -> Self {
    let Ok(raw) = std::env::var("YANSHI_SKILLS_INDEX") else {
      return Self::empty();
    };
    let path = PathBuf::from(raw);
    match SkillRegistryIndex::load(&path) {
      Ok(index) => {
        info!(
          path = %path.display(),
          skill_count = index.entries().len(),
          "loaded skill catalog"
        );
        Self {
          inner: Arc::new(SkillCatalogInner {
            index_path: Some(path),
            index: Some(index),
          }),
        }
      }
      Err(e) => {
        warn!(
          path = %path.display(),
          error = %e,
          "failed to load YANSHI_SKILLS_INDEX; serving empty skill catalog"
        );
        Self::empty()
      }
    }
  }

  /// Construct from an already-parsed index. Used by tests.
  pub fn from_index(index: SkillRegistryIndex, index_path: PathBuf) -> Self {
    Self {
      inner: Arc::new(SkillCatalogInner {
        index_path: Some(index_path),
        index: Some(index),
      }),
    }
  }

  pub fn entries(&self) -> Vec<SkillEntry> {
    self
      .inner
      .index
      .as_ref()
      .map(|idx| idx.entries().iter().map(SkillEntry::from).collect())
      .unwrap_or_default()
  }

  /// Resolve a skill by name or alias. `None` when the catalog is empty
  /// or the name is unknown.
  pub fn resolve(&self, name: &str) -> Option<ResolvedSkillRegistryEntry> {
    let inner = self.inner.as_ref();
    let index = inner.index.as_ref()?;
    let path = inner.index_path.as_deref()?;
    match index.resolve_skill(name, path) {
      Ok(entry) => Some(entry),
      Err(SkillError::ValidationError { .. }) => None,
      Err(e) => {
        warn!(skill = %name, error = %e, "skill resolution failed");
        None
      }
    }
  }
}

/// Public-facing skill metadata. Mirrors `SkillRegistryEntry` but skips
/// implementation detail fields (manifest path, sha256, etc.) so the wire
/// format stays stable across registry implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillEntry {
  pub name: String,
  pub version: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub aliases: Vec<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub channel: Option<String>,
}

impl From<&yanshi_skills::SkillRegistryEntry> for SkillEntry {
  fn from(value: &yanshi_skills::SkillRegistryEntry) -> Self {
    Self {
      name: value.name.clone(),
      version: value.version.clone(),
      aliases: value.aliases.clone(),
      channel: value.channel.clone(),
    }
  }
}

#[derive(Debug, Serialize)]
pub struct ListSkillsResponse {
  pub skills: Vec<SkillEntry>,
}

/// `GET /v1/skills` — list installed skills (with version + aliases).
pub async fn list_skills(State(state): State<AppState>) -> Json<ListSkillsResponse> {
  Json(ListSkillsResponse {
    skills: state.skills.entries(),
  })
}

#[derive(Debug, Deserialize)]
pub struct RunSkillRequest {
  /// Free-form input forwarded to the skill agent. Optional so callers
  /// can fire-and-forget skills that don't need user input or supply
  /// structured `inputs` instead.
  #[serde(default)]
  pub input: Option<String>,
  /// Structured key-value inputs (matching `yanshi skill run --input KEY=VALUE`),
  /// appended deterministically as `Inputs:\n- key: value`.
  #[serde(default)]
  pub inputs: serde_json::Map<String, serde_json::Value>,
  /// Optional model override for the skill agent and its `[[workflows]]` tools.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional USD spend cap propagated to the skill's `[[workflows]]` tools.
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
  #[serde(default)]
  pub retention_overrides: Option<RetentionOverrides>,
  #[serde(default)]
  pub tenant_id: Option<String>,
}

/// Format optional free-form `input` + structured `inputs` into the agent
/// user message, mirroring `yanshi-cli`'s `skill run` prompt assembly.
fn compose_skill_prompt(
  input: Option<&str>,
  inputs: &serde_json::Map<String, serde_json::Value>,
) -> Option<String> {
  let mut keys: Vec<&String> = inputs.keys().collect();
  keys.sort();
  let facts: Vec<String> = keys
    .into_iter()
    .map(|key| {
      let rendered = match &inputs[key] {
        serde_json::Value::String(s) => s.trim().to_string(),
        other => other.to_string(),
      };
      format!("- {}: {}", key.trim(), rendered)
    })
    .collect();
  let trimmed = input.map(str::trim).filter(|s| !s.is_empty());
  match (trimmed, facts.is_empty()) {
    (Some(msg), true) => Some(msg.to_string()),
    (Some(msg), false) => Some(format!("{msg}\n\nInputs:\n{}", facts.join("\n"))),
    (None, false) => Some(format!(
      "Produce what these inputs ask for.\n\nInputs:\n{}",
      facts.join("\n")
    )),
    (None, true) => None,
  }
}

/// `POST /v1/skills/{name}:run` — submit a skill execution as a run.
///
/// Resolution is best-effort: the path matches `:run` literally so the
/// route conflicts with no existing skill name. If the catalog has no
/// entry for `name`, return 404. Otherwise persist a queued `runs` row
/// with `workflow = "@skill:<name>"` (+ optional `\n---\n<input>`) via
/// `runs::admit_and_dispatch` so skill runs share the same
/// per-tenant concurrency / submission-rate admission control, `run_dir`
/// retention tracking, and `question_registry` as `/v1/runs`.
pub async fn run_skill(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(name): Path<String>,
  JsonReq(req): JsonReq<RunSkillRequest>,
) -> Result<Json<CreateRunResponse>, ApiError> {
  // Path is registered as `/v1/skills/:name_run`; strip the trailing
  // `:run` suffix to recover the bare skill name. Failing the suffix
  // check is a 400 because the route literal can't actually mismatch
  // here — but defensive code is cheap.
  let skill_name = name
    .strip_suffix(":run")
    .ok_or_else(|| ApiError::BadRequest("skill route must end with :run".into()))?;

  let resolved = state.skills.resolve(skill_name).ok_or_else(|| {
    ApiError::NotFound(format!(
      "skill '{}' not installed (configure YANSHI_SKILLS_INDEX)",
      skill_name
    ))
  })?;

  if let Some(limit) = req.cost_limit_usd {
    yanshi_config::config::v2::validate_cost_limit(limit).map_err(ApiError::BadRequest)?;
  }
  let (events_retention_days, artifacts_retention_days) = match req.retention_overrides {
    Some(overrides) => {
      if let Err(msg) = overrides.validate() {
        return Err(ApiError::BadRequest(msg.into()));
      }
      overrides.into_pair()
    }
    None => (None, None),
  };

  let tenant_id = tenant.as_str().to_string();
  // Q1.4.3: body tenant_id is no longer authoritative; if present it
  // must match the auth-bound tenant.
  if let Some(body_tenant) = &req.tenant_id
    && body_tenant != &tenant_id
  {
    return Err(ApiError::TenantMismatch(format!(
      "request body tenant_id '{body_tenant}' does not match authenticated tenant '{tenant_id}'"
    )));
  }

  // Y8.5 / AA1: a skill's `[[workflows]]` run in this process as agent tools,
  // so both their static definitions and any structured `req.inputs` meet the
  // same gateway reach policy as a workflow submitted to /v1/runs.
  // A manifest that does not load is left to the executor to report.
  if let Ok(manifest) = yanshi_skills::SkillLoader::load(&resolved.path) {
    let definitions =
      yanshi_config::skill_workflows::load_definitions(&manifest, &resolved.path)
        .map_err(|e| ApiError::BadRequest(format!("skill '{}': {e:#}", resolved.name)))?;
    for (wf_name, definition) in &definitions {
      state
        .gateway_policy
        .check_workflow(definition)
        .and_then(|()| {
          state
            .gateway_policy
            .check_run_inputs(definition, &req.inputs)
        })
        .map_err(|e| {
          ApiError::BadRequest(format!(
            "skill '{}' workflow tool '{wf_name}': {e}",
            resolved.name
          ))
        })?;
    }
  }

  let composed_input = compose_skill_prompt(req.input.as_deref(), &req.inputs);
  let workflow = match composed_input.as_deref() {
    Some(prompt) if !prompt.is_empty() => format!("@skill:{}\n---\n{}", resolved.name, prompt),
    _ => format!("@skill:{}", resolved.name),
  };

  let run = crate::runs::admit_and_dispatch(
    &state,
    state.executor.clone(),
    tenant_id,
    workflow,
    RunOptions {
      cost_limit_usd: req.cost_limit_usd,
      inputs: req.inputs,
      model: req.model,
      skill_dir: Some(resolved.path),
      events_retention_days,
      artifacts_retention_days,
      ..Default::default()
    },
  )
  .await?;

  Ok(Json(CreateRunResponse {
    run_id: run.id,
    status: "queued",
  }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn empty_catalog_lists_no_skills() {
    let catalog = SkillCatalog::empty();
    assert!(catalog.entries().is_empty());
    assert!(catalog.resolve("anything").is_none());
  }

  #[test]
  fn compose_skill_prompt_formats_message_and_structured_inputs() {
    let empty = serde_json::Map::new();
    assert_eq!(compose_skill_prompt(None, &empty), None);
    assert_eq!(compose_skill_prompt(Some("hi"), &empty), Some("hi".into()));

    let inputs = json!({"want": "report,data_table", "sections": 4})
      .as_object()
      .cloned()
      .expect("object");
    assert_eq!(
      compose_skill_prompt(None, &inputs),
      Some(
        "Produce what these inputs ask for.\n\nInputs:\n- sections: 4\n- want: report,data_table"
          .into()
      )
    );
    assert_eq!(
      compose_skill_prompt(Some("Summarise"), &inputs),
      Some("Summarise\n\nInputs:\n- sections: 4\n- want: report,data_table".into())
    );
  }
}
