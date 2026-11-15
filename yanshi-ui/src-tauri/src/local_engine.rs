//! Zero-Postgres embedded loopback gateway for the Tauri Desktop app.
//!
//! Speaks the exact `/v1/workflows/inspect`, `/v1/runs`, `/v1/runs/:id/debug`,
//! `/v1/runs/:id/events`, `/v1/runs/:id/artifacts`, and `/v1/runs/:id/preview`
//! contract consumed by `yanshi-ui`, backed directly by `yanshi-config` and
//! `yanshi-core` so desktop users can visually debug or one-click execute
//! workflows offline without installing PostgreSQL.

use std::collections::{BTreeMap, HashMap};
use std::convert::Infallible;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
  Json, Router,
  body::{Body, Bytes},
  extract::{Path as RoutePath, Query, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{
    IntoResponse, Response,
    sse::{Event as SseEvent, KeepAlive, Sse},
  },
  routing::{get, post},
};
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tokio_util::io::ReaderStream;
use tower_http::cors::CorsLayer;
use uuid::Uuid;
use yanshi_config::{
  diagnostics::{DoctorProfile, build_report},
  inspect::{InspectWorkflowResponse, inspect_workflow},
};
use yanshi_core::{
  CheckpointConfig, CheckpointManager, FlowCancellationToken, FlowDebugController,
  FlowDebugSnapshot, FlowExecutionConfig, FlowExt, ResumePlanOptions, build_resume_plan,
  events::{EventListener, WorkflowEvent},
  run_context::collect_file_outputs,
  value::FlowValue,
};
use yanshi_harness::{ApprovalDecision, ApprovalOutcome, ApprovalScope};
use yanshi_llm::Yanshi;

use crate::local_harness::LocalApprovalResolveError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalRunRecord {
  pub id: Uuid,
  pub workflow: String,
  pub status: String,
  pub tenant_id: String,
  pub started_at: DateTime<Utc>,
  pub finished_at: Option<DateTime<Utc>>,
  pub run_dir: Option<String>,
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStreamedEvent {
  pub run_id: Uuid,
  pub seq: i64,
  pub kind: String,
  pub payload: serde_json::Value,
  pub ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalArtifactSummary {
  pub id: Uuid,
  pub node_id: String,
  pub name: String,
  pub mime_type: String,
  pub size_bytes: u64,
  pub created_at: DateTime<Utc>,
  #[serde(skip)]
  pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedArtifactRecord {
  id: Uuid,
  node_id: String,
  name: String,
  mime_type: String,
  size_bytes: u64,
  created_at: DateTime<Utc>,
  file_path: PathBuf,
}

impl From<&LocalArtifactSummary> for PersistedArtifactRecord {
  fn from(value: &LocalArtifactSummary) -> Self {
    Self {
      id: value.id,
      node_id: value.node_id.clone(),
      name: value.name.clone(),
      mime_type: value.mime_type.clone(),
      size_bytes: value.size_bytes,
      created_at: value.created_at,
      file_path: value.file_path.clone(),
    }
  }
}

impl From<PersistedArtifactRecord> for LocalArtifactSummary {
  fn from(value: PersistedArtifactRecord) -> Self {
    Self {
      id: value.id,
      node_id: value.node_id,
      name: value.name,
      mime_type: value.mime_type,
      size_bytes: value.size_bytes,
      created_at: value.created_at,
      file_path: value.file_path,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedRunMeta {
  record: LocalRunRecord,
  #[serde(default)]
  artifacts: Vec<PersistedArtifactRecord>,
  #[serde(default)]
  model: Option<String>,
  #[serde(default)]
  cost_limit_usd: Option<f64>,
}

pub(crate) type RunCompletionHook = Arc<dyn Fn(Uuid, &str, &[LocalArtifactSummary]) + Send + Sync>;

struct RunEntry {
  record: LocalRunRecord,
  events: Vec<LocalStreamedEvent>,
  artifacts: Vec<LocalArtifactSummary>,
  tx: broadcast::Sender<LocalStreamedEvent>,
  cancel_token: FlowCancellationToken,
  debug_controller: Option<FlowDebugController>,
  attempt_start_seq: i64,
  model: Option<String>,
  cost_limit_usd: Option<f64>,
  on_complete: Option<RunCompletionHook>,
}

type TenantPreferencesMap = BTreeMap<String, BTreeMap<String, serde_json::Value>>;

#[derive(Clone)]
pub struct LocalEngineState {
  runs: Arc<Mutex<HashMap<Uuid, RunEntry>>>,
  preferences: Arc<Mutex<TenantPreferencesMap>>,
  harness: crate::local_harness::LocalHarnessState,
  base_dir: PathBuf,
}

impl LocalEngineState {
  pub fn new(base_dir: PathBuf) -> Self {
    let initial_prefs = load_preferences_from_disk(&base_dir);
    let hydrated_runs = hydrate_runs_from_disk(&base_dir);
    let harness = crate::local_harness::LocalHarnessState::new(base_dir.clone());
    Self {
      runs: Arc::new(Mutex::new(hydrated_runs)),
      preferences: Arc::new(Mutex::new(initial_prefs)),
      harness,
      base_dir,
    }
  }

  pub fn base_dir(&self) -> &Path {
    &self.base_dir
  }

  pub fn harness(&self) -> &crate::local_harness::LocalHarnessState {
    &self.harness
  }

  fn lock_runs(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, RunEntry>> {
    match self.runs.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }

  fn lock_preferences(&self) -> std::sync::MutexGuard<'_, TenantPreferencesMap> {
    match self.preferences.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }
}

fn checkpoint_dir_for_local_run(base_dir: &Path, run_id: Uuid) -> PathBuf {
  base_dir.join(run_id.to_string()).join("checkpoint")
}

fn write_run_meta_sync(run_dir: &Path, entry: &RunEntry) -> std::io::Result<()> {
  std::fs::create_dir_all(run_dir)?;
  let meta = PersistedRunMeta {
    record: entry.record.clone(),
    artifacts: entry
      .artifacts
      .iter()
      .map(PersistedArtifactRecord::from)
      .collect(),
    model: entry.model.clone(),
    cost_limit_usd: entry.cost_limit_usd,
  };
  let bytes = serde_json::to_vec_pretty(&meta).map_err(std::io::Error::other)?;
  let tmp = run_dir.join(format!("run_meta.json.tmp.{}", Uuid::new_v4()));
  let dst = run_dir.join("run_meta.json");
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

fn append_run_event_jsonl_sync(run_dir: &Path, event: &LocalStreamedEvent) -> std::io::Result<()> {
  std::fs::create_dir_all(run_dir)?;
  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(run_dir.join("events.jsonl"))?;
  let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
  writeln!(file, "{line}")
}

fn read_run_events_jsonl(path: &Path) -> Vec<LocalStreamedEvent> {
  let Ok(text) = std::fs::read_to_string(path) else {
    return Vec::new();
  };
  let mut events: Vec<LocalStreamedEvent> = text
    .lines()
    .filter(|line| !line.trim().is_empty())
    .filter_map(|line| serde_json::from_str::<LocalStreamedEvent>(line).ok())
    .collect();
  events.sort_by_key(|ev| ev.seq);
  events
}

fn hydrate_runs_from_disk(base_dir: &Path) -> HashMap<Uuid, RunEntry> {
  let mut out = HashMap::new();
  let Ok(read_dir) = std::fs::read_dir(base_dir) else {
    return out;
  };
  for item in read_dir.flatten() {
    let path = item.path();
    if !path.is_dir() {
      continue;
    }
    let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) else {
      continue;
    };
    let Ok(run_id) = Uuid::parse_str(dir_name) else {
      continue;
    };
    let meta_path = path.join("run_meta.json");
    let Ok(bytes) = std::fs::read(&meta_path) else {
      continue;
    };
    let Ok(mut meta) = serde_json::from_slice::<PersistedRunMeta>(&bytes) else {
      continue;
    };
    let mut needs_rewrite = false;
    if !is_terminal_status(&meta.record.status) {
      meta.record.status = "failed".to_string();
      meta.record.finished_at = Some(Utc::now());
      meta.record.error = Some("desktop_restarted_while_running".to_string());
      needs_rewrite = true;
    }
    let events = read_run_events_jsonl(&path.join("events.jsonl"));
    let attempt_start_seq = 0;
    let (tx, _rx) = broadcast::channel(512);
    let entry = RunEntry {
      record: meta.record,
      events,
      artifacts: meta
        .artifacts
        .into_iter()
        .map(LocalArtifactSummary::from)
        .collect(),
      tx,
      cancel_token: FlowCancellationToken::new(),
      debug_controller: None,
      attempt_start_seq,
      model: meta.model,
      cost_limit_usd: meta.cost_limit_usd,
      on_complete: None,
    };
    if needs_rewrite {
      let _ = write_run_meta_sync(&path, &entry);
    }
    out.insert(run_id, entry);
  }
  out
}

fn preferences_file_path(base_dir: &Path) -> PathBuf {
  base_dir.join("_preferences.json")
}

fn load_preferences_from_disk(base_dir: &Path) -> TenantPreferencesMap {
  let path = preferences_file_path(base_dir);
  let Ok(bytes) = std::fs::read(&path) else {
    return BTreeMap::new();
  };
  serde_json::from_slice(&bytes).unwrap_or_default()
}

pub(crate) fn extract_tenant(headers: &HeaderMap) -> String {
  headers
    .get("x-yanshi-tenant")
    .and_then(|v| v.to_str().ok())
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or("default")
    .to_string()
}

fn default_studio_runs_dir() -> PathBuf {
  if let Ok(explicit) = std::env::var("YANSHI_RUN_DIR") {
    let trimmed = explicit.trim();
    if !trimmed.is_empty() {
      return PathBuf::from(trimmed);
    }
  }
  dirs::home_dir()
    .unwrap_or_else(std::env::temp_dir)
    .join(".yanshi")
    .join("studio-runs")
}

pub(crate) fn load_user_dotenv() {
  let _ = dotenvy::dotenv();
  if let Some(home) = dirs::home_dir() {
    let user_env = home.join(".yanshi").join(".env");
    if user_env.is_file() {
      let _ = dotenvy::from_path(user_env);
    }
  }
}

pub(crate) async fn ensure_llm_initialized() -> anyhow::Result<()> {
  static INIT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
  INIT
    .get_or_try_init(|| async {
      load_user_dotenv();
      Yanshi::init().await
    })
    .await?;
  Ok(())
}

/// Start the embedded loopback engine on `127.0.0.1:0` and return its base URL
/// (e.g. `http://127.0.0.1:54123`).
pub async fn start_loopback_engine(custom_base_dir: Option<PathBuf>) -> anyhow::Result<String> {
  let base_dir = custom_base_dir.unwrap_or_else(default_studio_runs_dir);
  tokio::fs::create_dir_all(&base_dir).await?;
  let state = LocalEngineState::new(base_dir);
  let app = build_local_router(state);

  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
  let addr: SocketAddr = listener.local_addr()?;
  tokio::spawn(async move {
    let _ = axum::serve(listener, app).await;
  });
  Ok(format!("http://127.0.0.1:{}", addr.port()))
}

pub fn build_local_router(state: LocalEngineState) -> Router {
  let harness_router = crate::local_harness::build_harness_router(state.harness.clone());
  Router::new()
    .route("/health", get(health))
    .route("/v1/diagnostics", get(get_diagnostics))
    .route(
      "/v1/preferences",
      get(list_preferences).put(put_preferences),
    )
    .route("/v1/workflows/inspect", post(inspect_route))
    .route("/v1/runs", get(list_runs).post(create_run))
    .route("/v1/runs/:id", get(get_run).post(post_run_action))
    .route("/v1/runs/:id/resume-plan", get(get_run_resume_plan))
    .route("/v1/runs/:id/approvals", get(list_run_approvals))
    .route(
      "/v1/runs/:id/approvals/:request_id",
      post(decide_run_approval),
    )
    .route("/v1/runs/:id/debug", get(get_run_debug))
    .route("/v1/runs/:id/events", get(stream_events))
    .route("/v1/runs/:id/events/history", get(events_history))
    .route("/v1/runs/:id/artifacts", get(list_artifacts))
    .route("/v1/runs/:id/artifacts.zip", get(download_artifacts_zip))
    .route(
      "/v1/runs/:id/artifacts/:artifact_id",
      get(download_artifact),
    )
    .route("/v1/runs/:id/preview", get(preview_file))
    .with_state(state)
    .merge(harness_router)
    .layer(CorsLayer::permissive())
}

async fn health() -> impl IntoResponse {
  Json(serde_json::json!({ "status": "ok", "engine": "tauri-local" }))
}

async fn get_diagnostics() -> Response {
  load_user_dotenv();
  let report = build_report(DoctorProfile::Local, None, false, false, (None, Vec::new())).await;
  match serde_json::to_value(&report) {
    Ok(value) => (StatusCode::OK, Json(value)).into_response(),
    Err(err) => error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to serialize diagnostics report: {err}"),
    ),
  }
}

#[derive(Debug, Deserialize, Serialize)]
struct PreferencesEnvelope {
  preferences: BTreeMap<String, serde_json::Value>,
}

const MAX_PREFERENCE_VALUE_BYTES: usize = 16 * 1024;

fn is_valid_preference_key(key: &str) -> bool {
  let len = key.len();
  if !(1..=128).contains(&len) {
    return false;
  }
  key
    .bytes()
    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-' || b == b':')
}

fn looks_like_token(value: &serde_json::Value) -> Option<&'static str> {
  let serde_json::Value::String(s) = value else {
    return None;
  };
  let trimmed = s.trim();
  if trimmed.is_empty() {
    return None;
  }
  let lower = trimmed.to_lowercase();
  if lower.starts_with("bearer ") {
    return Some("Bearer-prefixed");
  }
  for prefix in ["sk-", "sk_", "ant-", "api_", "ghp_", "ghs_", "gho_"] {
    if trimmed.starts_with(prefix) && trimmed.len() >= prefix.len() + 16 {
      return Some("API-key-shaped");
    }
  }
  if trimmed.len() >= 32 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
    return Some("long hex digest");
  }
  if trimmed.len() >= 40
    && trimmed.chars().all(|c| {
      c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '_' || c == '-' || c == '='
    })
    && !trimmed.chars().any(|c| c == ' ')
  {
    let alpha = trimmed.chars().filter(|c| c.is_alphabetic()).count();
    let digit = trimmed.chars().filter(|c| c.is_ascii_digit()).count();
    if alpha > 0 && digit > 0 {
      return Some("long opaque token-shaped string");
    }
  }
  None
}

async fn list_preferences(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
) -> impl IntoResponse {
  let tenant = extract_tenant(&headers);
  let preferences = state
    .lock_preferences()
    .get(&tenant)
    .cloned()
    .unwrap_or_default();
  Json(PreferencesEnvelope { preferences })
}

async fn put_preferences(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  Json(body): Json<PreferencesEnvelope>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let mut validated: Vec<(String, serde_json::Value)> = Vec::with_capacity(body.preferences.len());
  for (key, value) in body.preferences {
    if !is_valid_preference_key(&key) {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!("preference key '{key}' is invalid; must match ^[a-zA-Z0-9_.\\-:]{{1,128}}$"),
      );
    }
    let serialized = match serde_json::to_vec(&value) {
      Ok(bytes) => bytes,
      Err(err) => return error_json(StatusCode::BAD_REQUEST, &err.to_string()),
    };
    if serialized.len() > MAX_PREFERENCE_VALUE_BYTES {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "preference value for '{key}' is too large ({} bytes; max {MAX_PREFERENCE_VALUE_BYTES})",
          serialized.len()
        ),
      );
    }
    if let Some(reason) = looks_like_token(&value) {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "preference value for '{key}' looks like a secret token ({reason}); refusing to store"
        ),
      );
    }
    validated.push((key, value));
  }

  let (tenant_snapshot, full_snapshot) = {
    let mut guard = state.lock_preferences();
    let tenant_map = guard.entry(tenant).or_default();
    for (key, value) in validated {
      tenant_map.insert(key, value);
    }
    (tenant_map.clone(), guard.clone())
  };

  if let Err(err) = persist_preferences_to_disk(&state.base_dir, &full_snapshot).await {
    return error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist preferences: {err}"),
    );
  }

  (
    StatusCode::OK,
    Json(PreferencesEnvelope {
      preferences: tenant_snapshot,
    }),
  )
    .into_response()
}

async fn persist_preferences_to_disk(
  base_dir: &Path,
  snapshot: &TenantPreferencesMap,
) -> std::io::Result<()> {
  tokio::fs::create_dir_all(base_dir).await?;
  let bytes = serde_json::to_vec_pretty(snapshot).map_err(std::io::Error::other)?;
  let final_path = preferences_file_path(base_dir);
  let tmp_path = base_dir.join(format!("_preferences.json.tmp.{}", Uuid::new_v4()));
  tokio::fs::write(&tmp_path, bytes).await?;
  tokio::fs::rename(&tmp_path, &final_path).await
}

#[derive(Debug, Deserialize)]
struct InspectRequest {
  workflow: String,
}

async fn inspect_route(Json(req): Json<InspectRequest>) -> Json<InspectWorkflowResponse> {
  Json(inspect_workflow(&req.workflow))
}

#[derive(Debug, Deserialize, Default)]
struct ListRunsQuery {
  limit: Option<usize>,
}

async fn list_runs(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  Query(query): Query<ListRunsQuery>,
) -> impl IntoResponse {
  let tenant = extract_tenant(&headers);
  let cap = query.limit.unwrap_or(20).clamp(1, 100);
  let mut list: Vec<LocalRunRecord> = state
    .lock_runs()
    .values()
    .filter(|e| e.record.tenant_id == tenant)
    .map(|e| e.record.clone())
    .collect();
  list.sort_by_key(|a| std::cmp::Reverse(a.started_at));
  list.truncate(cap);
  Json(serde_json::json!({ "runs": list }))
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum RunMode {
  #[default]
  Release,
  Debug,
}

#[derive(Debug, Deserialize)]
struct CreateRunRequest {
  #[serde(default)]
  tenant_id: Option<String>,
  workflow: String,
  #[serde(default)]
  inputs: serde_json::Map<String, serde_json::Value>,
  #[serde(default)]
  model: Option<String>,
  #[serde(default)]
  cost_limit_usd: Option<f64>,
  #[serde(default)]
  run_mode: RunMode,
  #[serde(default)]
  breakpoints: Vec<String>,
  #[serde(default)]
  step_every_node: Option<bool>,
}

pub(crate) struct SpawnLocalRunParams {
  pub tenant_id: String,
  pub workflow: String,
  pub inputs: serde_json::Map<String, serde_json::Value>,
  pub model: Option<String>,
  pub cost_limit_usd: Option<f64>,
  pub debug_controller: Option<FlowDebugController>,
  pub on_complete: Option<RunCompletionHook>,
}

impl LocalEngineState {
  pub(crate) async fn spawn_workflow_run(
    &self,
    params: SpawnLocalRunParams,
  ) -> Result<Uuid, String> {
    let run_id = Uuid::new_v4();
    let run_dir = self.base_dir.join(run_id.to_string());
    tokio::fs::create_dir_all(&run_dir)
      .await
      .map_err(|err| format!("failed to create run directory: {err}"))?;

    let cancel_token = FlowCancellationToken::new();
    let (tx, _rx) = broadcast::channel(512);
    let record = LocalRunRecord {
      id: run_id,
      workflow: params.workflow.clone(),
      status: "running".to_string(),
      tenant_id: params.tenant_id,
      started_at: Utc::now(),
      finished_at: None,
      run_dir: Some(run_dir.display().to_string()),
      error: None,
    };

    let entry = RunEntry {
      record,
      events: Vec::new(),
      artifacts: Vec::new(),
      tx,
      cancel_token: cancel_token.clone(),
      debug_controller: params.debug_controller.clone(),
      attempt_start_seq: 0,
      model: params.model.clone(),
      cost_limit_usd: params.cost_limit_usd,
      on_complete: params.on_complete,
    };
    let _ = write_run_meta_sync(&run_dir, &entry);
    self.lock_runs().insert(run_id, entry);

    let state_bg = self.clone();
    let base_dir_bg = self.base_dir.clone();
    tokio::spawn(async move {
      execute_local_flow(
        state_bg,
        base_dir_bg,
        run_id,
        params.workflow,
        params.inputs,
        params.model,
        params.cost_limit_usd,
        false,
        cancel_token,
        params.debug_controller,
      )
      .await;
    });

    Ok(run_id)
  }
}

async fn create_run(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  Json(req): Json<CreateRunRequest>,
) -> Response {
  if req.workflow.trim().is_empty() {
    return error_json(StatusCode::BAD_REQUEST, "workflow YAML cannot be empty");
  }
  if let Some(limit) = req.cost_limit_usd
    && (!limit.is_finite() || limit <= 0.0)
  {
    return error_json(
      StatusCode::BAD_REQUEST,
      "cost_limit_usd must be a finite number greater than 0",
    );
  }
  let header_tenant = extract_tenant(&headers);
  let resolved_tenant = match req
    .tenant_id
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    Some(explicit) if headers.contains_key("x-yanshi-tenant") && explicit != header_tenant => {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "request body tenant_id '{explicit}' conflicts with X-Yanshi-Tenant header '{header_tenant}'"
        ),
      );
    }
    Some(explicit) => explicit.to_string(),
    None => header_tenant,
  };

  let debug_enabled = req.run_mode == RunMode::Debug
    || !req.breakpoints.is_empty()
    || req.step_every_node == Some(true);
  let debug_controller = if debug_enabled {
    let step_each = req
      .step_every_node
      .unwrap_or(req.run_mode == RunMode::Debug);
    Some(FlowDebugController::new(step_each, req.breakpoints))
  } else {
    None
  };

  let run_id = match state
    .spawn_workflow_run(SpawnLocalRunParams {
      tenant_id: resolved_tenant,
      workflow: req.workflow,
      inputs: req.inputs,
      model: req.model,
      cost_limit_usd: req.cost_limit_usd,
      debug_controller,
      on_complete: None,
    })
    .await
  {
    Ok(id) => id,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };

  (
    StatusCode::OK,
    Json(serde_json::json!({
      "run_id": run_id,
      "status": "queued",
    })),
  )
    .into_response()
}

struct InMemoryEventListener {
  run_id: Uuid,
  run_dir: PathBuf,
  state: LocalEngineState,
}

impl EventListener for InMemoryEventListener {
  fn on_event(&self, event: &WorkflowEvent) {
    let kind = event.event_type().to_string();
    let payload = workflow_event_to_json(event);
    let streamed_opt = {
      let mut guard = self.state.lock_runs();
      guard.get_mut(&self.run_id).map(|entry| {
        let seq = entry.events.last().map_or(0, |last| last.seq + 1);
        let streamed = LocalStreamedEvent {
          run_id: self.run_id,
          seq,
          kind,
          payload,
          ts: Utc::now(),
        };
        entry.events.push(streamed.clone());
        let _ = entry.tx.send(streamed.clone());
        streamed
      })
    };
    if let Some(streamed) = streamed_opt {
      let _ = append_run_event_jsonl_sync(&self.run_dir, &streamed);
    }
  }
}

#[allow(clippy::too_many_arguments)]
async fn execute_local_flow(
  state: LocalEngineState,
  base_dir: PathBuf,
  run_id: Uuid,
  workflow_yaml: String,
  raw_inputs: serde_json::Map<String, serde_json::Value>,
  model_override: Option<String>,
  cost_limit_usd: Option<f64>,
  resume: bool,
  cancel_token: FlowCancellationToken,
  debug_controller: Option<FlowDebugController>,
) {
  let run_dir = base_dir.join(run_id.to_string());
  let outcome: anyhow::Result<HashMap<String, yanshi_core::async_node::AsyncNodeResult>> = async {
    let _ = ensure_llm_initialized().await;
    let flow_def = yanshi_config::executor::parse_workflow_definition(&workflow_yaml)?;
    let mut flow =
      yanshi_config::executor::build_flow_from_definition(&flow_def, model_override.as_deref())?;

    let listener = InMemoryEventListener {
      run_id,
      run_dir: run_dir.clone(),
      state: state.clone(),
    };
    flow = flow.with_event_listener(Arc::new(listener));

    let mut exec_cfg = FlowExecutionConfig::concurrent(8)
      .with_run_base_dir(base_dir.clone())
      .with_cancellation_token(cancel_token.clone());
    if let Some(limit) = flow_def
      .effective_cost_limit(cost_limit_usd)
      .map_err(anyhow::Error::msg)?
    {
      exec_cfg = exec_cfg.with_cost_limit_usd(limit);
    }
    if let Some(ctrl) = debug_controller.clone() {
      exec_cfg = exec_cfg.with_debug_controller(ctrl);
    }

    flow = flow.with_checkpointing(
      CheckpointConfig::default()
        .with_checkpoint_dir(checkpoint_dir_for_local_run(&base_dir, run_id)),
    )?;

    let final_state = if resume {
      flow
        .resume_with_config(
          &run_id.to_string(),
          &ResumePlanOptions::default(),
          |recorded| match recorded.cost_limit_usd {
            Some(limit) => exec_cfg.with_cost_limit_usd(limit),
            None => exec_cfg,
          },
        )
        .await?
    } else {
      let mut initial_inputs: HashMap<String, FlowValue> = raw_inputs
        .into_iter()
        .map(|(k, v)| (k, FlowValue::Json(v)))
        .collect();
      yanshi_config::executor::apply_declared_inputs(&flow_def, &mut initial_inputs)?;
      flow
        .execute_from_inputs_with_id_and_config(run_id.to_string(), initial_inputs, exec_cfg)
        .await?
    };
    Ok(final_state)
  }
  .await;

  if let Some(ctrl) = &debug_controller {
    ctrl.abort();
  }

  let mut discovered_artifacts = Vec::new();
  if let Ok(ref pool) = outcome {
    for file_out in collect_file_outputs(pool, &base_dir) {
      if let Ok(meta) = tokio::fs::metadata(&file_out.path).await {
        let name = file_out
          .path
          .file_name()
          .map(|n| n.to_string_lossy().to_string())
          .unwrap_or_else(|| file_out.output_key.clone());
        discovered_artifacts.push(LocalArtifactSummary {
          id: Uuid::new_v4(),
          node_id: file_out.node_id,
          name,
          mime_type: file_out
            .mime_type
            .unwrap_or_else(|| infer_mime(&file_out.path).to_string()),
          size_bytes: meta.len(),
          created_at: Utc::now(),
          file_path: file_out.path,
        });
      }
    }
  }

  let mut completion_notify: Option<(RunCompletionHook, String, Vec<LocalArtifactSummary>)> = None;
  let mut synthetic_to_persist: Option<LocalStreamedEvent> = None;
  {
    let mut guard = state.lock_runs();
    if let Some(entry) = guard.get_mut(&run_id) {
      entry.debug_controller = None;
      if !discovered_artifacts.is_empty() {
        entry.artifacts = discovered_artifacts;
      }
      entry.record.finished_at = Some(Utc::now());
      match outcome {
        Ok(pool) => {
          if cancel_token.is_cancelled() {
            entry.record.status = "cancelled".to_string();
          } else if let Some(node_err) = first_pool_error(&pool) {
            entry.record.status = "failed".to_string();
            entry.record.error = Some(node_err.clone());
            synthetic_to_persist =
              push_synthetic_terminal_event(entry, "workflow.failed", Some(&node_err));
          } else {
            entry.record.status = "succeeded".to_string();
            entry.record.error = None;
          }
        }
        Err(err) => {
          if cancel_token.is_cancelled() {
            entry.record.status = "cancelled".to_string();
          } else {
            let msg = err.to_string();
            entry.record.status = "failed".to_string();
            entry.record.error = Some(msg.clone());
            synthetic_to_persist =
              push_synthetic_terminal_event(entry, "workflow.failed", Some(&msg));
          }
        }
      }
      let _ = write_run_meta_sync(&run_dir, entry);
      if let Some(hook) = entry.on_complete.clone() {
        completion_notify = Some((hook, entry.record.status.clone(), entry.artifacts.clone()));
      }
    }
  }

  if let Some(ev) = synthetic_to_persist {
    let _ = append_run_event_jsonl_sync(&run_dir, &ev);
  }
  if let Some((hook, final_status, final_artifacts)) = completion_notify {
    hook(run_id, &final_status, &final_artifacts);
  }
}

fn first_pool_error(
  pool: &HashMap<String, yanshi_core::async_node::AsyncNodeResult>,
) -> Option<String> {
  for (node_id, res) in pool {
    if let Err(err) = res {
      if matches!(err, yanshi_core::error::YanshiError::NodeSkipped) {
        continue;
      }
      return Some(format!("node '{node_id}' failed: {err}"));
    }
  }
  None
}

fn push_synthetic_terminal_event(
  entry: &mut RunEntry,
  kind: &str,
  err_msg: Option<&str>,
) -> Option<LocalStreamedEvent> {
  if entry
    .events
    .iter()
    .any(|e| e.seq >= entry.attempt_start_seq && is_terminal_event_kind(&e.kind))
  {
    return None;
  }
  let seq = entry.events.last().map_or(0, |last| last.seq + 1);
  let ev = LocalStreamedEvent {
    run_id: entry.record.id,
    seq,
    kind: kind.to_string(),
    payload: serde_json::json!({
      "workflow_id": entry.record.id.to_string(),
      "error": err_msg,
    }),
    ts: Utc::now(),
  };
  entry.events.push(ev.clone());
  let _ = entry.tx.send(ev.clone());
  Some(ev)
}

async fn get_run(
  State(state): State<LocalEngineState>,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let guard = state.lock_runs();
  match guard.get(&run_id) {
    Some(entry) => Json(entry.record.clone()).into_response(),
    None => error_json(StatusCode::NOT_FOUND, "run not found"),
  }
}

#[derive(Debug, Deserialize, Default)]
struct ResumePlanQuery {
  #[serde(default)]
  force_replay: bool,
}

async fn get_run_resume_plan(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<ResumePlanQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_runs();
    let Some(_entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
  }

  let ckpt_dir = checkpoint_dir_for_local_run(&state.base_dir, run_id);
  let manager =
    match CheckpointManager::new(CheckpointConfig::default().with_checkpoint_dir(ckpt_dir)) {
      Ok(mgr) => mgr,
      Err(err) => {
        return error_json(
          StatusCode::INTERNAL_SERVER_ERROR,
          &format!("checkpoint manager init failed: {err}"),
        );
      }
    };

  let checkpoint = match manager.load_latest_checkpoint(&run_id.to_string()).await {
    Ok(Some(ckpt)) => ckpt,
    Ok(None) => {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("no checkpoint found for run {run_id}"),
      );
    }
    Err(err) => {
      return error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("failed to load checkpoint: {err}"),
      );
    }
  };

  match build_resume_plan(
    &checkpoint,
    &ResumePlanOptions {
      force_replay: query.force_replay,
    },
  ) {
    Ok(plan) => Json(plan).into_response(),
    Err(err) => error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to build resume plan: {err}"),
    ),
  }
}

#[derive(Debug, Deserialize)]
struct RunApprovalDecisionRequest {
  decision: ApprovalOutcome,
  #[serde(default)]
  scope: Option<ApprovalScope>,
  #[serde(default)]
  decided_by: Option<String>,
  #[serde(default)]
  reason: Option<String>,
}

async fn list_run_approvals(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_runs();
    if !guard
      .get(&run_id)
      .is_some_and(|entry| entry.record.tenant_id == tenant)
    {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    }
  }
  let approvals = state.harness.approvals().list(&run_id.to_string());
  Json(serde_json::json!({ "approvals": approvals })).into_response()
}

async fn decide_run_approval(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((run_id, request_id)): RoutePath<(Uuid, String)>,
  Json(body): Json<RunApprovalDecisionRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_runs();
    if !guard
      .get(&run_id)
      .is_some_and(|entry| entry.record.tenant_id == tenant)
    {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    }
  }

  let decision = ApprovalDecision {
    request_id: request_id.clone(),
    decision: body.decision,
    scope: body.scope.unwrap_or(ApprovalScope::Once),
    decided_by: body
      .decided_by
      .unwrap_or_else(|| "user:desktop".to_string()),
    decided_at: Utc::now(),
    reason: body.reason,
  };

  match state
    .harness
    .approvals()
    .decide(&run_id.to_string(), &request_id, decision)
  {
    Ok(()) => Json(serde_json::json!({
      "run_id": run_id,
      "request_id": request_id,
      "resolved": true,
    }))
    .into_response(),
    Err(LocalApprovalResolveError::NotFound) => error_json(
      StatusCode::NOT_FOUND,
      &format!("pending approval '{request_id}' not found for run {run_id}"),
    ),
    Err(LocalApprovalResolveError::ProviderGone) => error_json(
      StatusCode::BAD_REQUEST,
      &format!("approval {request_id} cannot be decided: provider future already dropped"),
    ),
  }
}

#[derive(Debug, Serialize)]
struct DebugStatusResponse {
  run_id: Uuid,
  status: String,
  active: bool,
  snapshot: Option<FlowDebugSnapshot>,
  #[serde(skip_serializing_if = "Option::is_none")]
  resumed: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum DebugControlRequest {
  Step {
    #[serde(default)]
    input_override: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    output_override: Option<serde_json::Map<String, serde_json::Value>>,
  },
  Continue {
    #[serde(default)]
    input_override: Option<serde_json::Map<String, serde_json::Value>>,
    #[serde(default)]
    output_override: Option<serde_json::Map<String, serde_json::Value>>,
  },
  SetBreakpoints {
    #[serde(default)]
    breakpoints: Vec<String>,
    #[serde(default)]
    step_every_node: Option<bool>,
  },
}

async fn get_run_debug(
  State(state): State<LocalEngineState>,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id) else {
    return error_json(StatusCode::NOT_FOUND, "run not found");
  };
  let snapshot = entry.debug_controller.as_ref().map(|c| c.snapshot());
  Json(DebugStatusResponse {
    run_id,
    status: entry.record.status.clone(),
    active: snapshot.is_some(),
    snapshot,
    resumed: None,
  })
  .into_response()
}

async fn post_run_action(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(raw_segment): RoutePath<String>,
  body: Bytes,
) -> Response {
  if let Some(raw_id) = raw_segment.strip_suffix(":cancel") {
    let Ok(run_id) = Uuid::parse_str(raw_id) else {
      return error_json(StatusCode::BAD_REQUEST, "invalid run UUID");
    };
    let run_dir = state.base_dir.join(run_id.to_string());
    let (record, synthetic_ev) = {
      let mut guard = state.lock_runs();
      let Some(entry) = guard.get_mut(&run_id) else {
        return error_json(StatusCode::NOT_FOUND, "run not found");
      };
      entry.cancel_token.cancel();
      if let Some(ctrl) = entry.debug_controller.take() {
        ctrl.abort();
      }
      let mut synthetic = None;
      if !is_terminal_status(&entry.record.status) {
        entry.record.status = "cancelled".to_string();
        entry.record.finished_at = Some(Utc::now());
        synthetic = push_synthetic_terminal_event(entry, "workflow.cancelled", None);
      }
      let _ = write_run_meta_sync(&run_dir, entry);
      (entry.record.clone(), synthetic)
    };
    if let Some(ev) = synthetic_ev {
      let _ = append_run_event_jsonl_sync(&run_dir, &ev);
    }
    return Json(serde_json::json!({
      "run": record,
      "cancelled": true,
    }))
    .into_response();
  }

  if let Some(raw_id) = raw_segment.strip_suffix(":resume") {
    let Ok(run_id) = Uuid::parse_str(raw_id) else {
      return error_json(StatusCode::BAD_REQUEST, "invalid run UUID");
    };
    let tenant = extract_tenant(&headers);
    {
      let guard = state.lock_runs();
      let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
        return error_json(StatusCode::NOT_FOUND, "run not found");
      };
      if entry.record.workflow.starts_with("@skill:") {
        return error_json(
          StatusCode::BAD_REQUEST,
          "skill runs cannot be resumed; submit the skill again",
        );
      }
      if entry.record.status != "failed" && entry.record.status != "cancelled" {
        return error_json(
          StatusCode::BAD_REQUEST,
          &format!(
            "run {run_id} is '{}'; only failed or cancelled runs can be resumed",
            entry.record.status
          ),
        );
      }
    }

    let ckpt_dir = checkpoint_dir_for_local_run(&state.base_dir, run_id);
    let has_ckpt =
      match CheckpointManager::new(CheckpointConfig::default().with_checkpoint_dir(ckpt_dir)) {
        Ok(mgr) => match mgr.load_latest_checkpoint(&run_id.to_string()).await {
          Ok(opt) => opt.is_some(),
          Err(err) => {
            return error_json(
              StatusCode::INTERNAL_SERVER_ERROR,
              &format!("failed to load checkpoint: {err}"),
            );
          }
        },
        Err(err) => {
          return error_json(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("checkpoint manager init failed: {err}"),
          );
        }
      };
    if !has_ckpt {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!("run {run_id} has no checkpoint on this gateway to resume from"),
      );
    }

    let run_dir = state.base_dir.join(run_id.to_string());
    let (reopened_record, workflow_yaml, model_override, cost_limit_usd, cancel_token) = {
      let mut guard = state.lock_runs();
      let Some(entry) = guard
        .get_mut(&run_id)
        .filter(|e| e.record.tenant_id == tenant)
      else {
        return error_json(StatusCode::NOT_FOUND, "run not found");
      };
      if entry.record.status != "failed" && entry.record.status != "cancelled" {
        return error_json(
          StatusCode::BAD_REQUEST,
          &format!("run {run_id} is no longer failed or cancelled"),
        );
      }
      let next_seq = entry.events.last().map_or(0, |last| last.seq + 1);
      let new_cancel = FlowCancellationToken::new();
      entry.attempt_start_seq = next_seq;
      entry.cancel_token = new_cancel.clone();
      entry.debug_controller = None;
      entry.record.status = "running".to_string();
      entry.record.finished_at = None;
      entry.record.error = None;
      let _ = write_run_meta_sync(&run_dir, entry);
      (
        entry.record.clone(),
        entry.record.workflow.clone(),
        entry.model.clone(),
        entry.cost_limit_usd,
        new_cancel,
      )
    };

    let state_bg = state.clone();
    let base_dir_bg = state.base_dir.clone();
    tokio::spawn(async move {
      execute_local_flow(
        state_bg,
        base_dir_bg,
        run_id,
        workflow_yaml,
        serde_json::Map::new(),
        model_override,
        cost_limit_usd,
        true,
        cancel_token,
        None,
      )
      .await;
    });

    return Json(reopened_record).into_response();
  }

  if let Some(raw_id) = raw_segment.strip_suffix(":debug") {
    let Ok(run_id) = Uuid::parse_str(raw_id) else {
      return error_json(StatusCode::BAD_REQUEST, "invalid run UUID");
    };
    let req: DebugControlRequest = match serde_json::from_slice(&body) {
      Ok(parsed) => parsed,
      Err(err) => return error_json(StatusCode::BAD_REQUEST, &format!("invalid JSON: {err}")),
    };
    let guard = state.lock_runs();
    let Some(entry) = guard.get(&run_id) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    let Some(ctrl) = entry.debug_controller.clone() else {
      return Json(DebugStatusResponse {
        run_id,
        status: entry.record.status.clone(),
        active: false,
        snapshot: None,
        resumed: Some(false),
      })
      .into_response();
    };
    let status = entry.record.status.clone();
    drop(guard);

    let resumed = match req {
      DebugControlRequest::Step {
        input_override,
        output_override,
      } => {
        let patch = yanshi_core::DebugStepPatch::from_json_maps(input_override, output_override);
        ctrl.step_with_patch(patch)
      }
      DebugControlRequest::Continue {
        input_override,
        output_override,
      } => {
        let patch = yanshi_core::DebugStepPatch::from_json_maps(input_override, output_override);
        ctrl.continue_with_patch(patch)
      }
      DebugControlRequest::SetBreakpoints {
        breakpoints,
        step_every_node,
      } => {
        ctrl.set_breakpoints(breakpoints);
        if let Some(step_each) = step_every_node {
          ctrl.set_step_every_node(step_each);
        }
        false
      }
    };

    return Json(DebugStatusResponse {
      run_id,
      status,
      active: true,
      snapshot: Some(ctrl.snapshot()),
      resumed: Some(resumed),
    })
    .into_response();
  }

  error_json(
    StatusCode::NOT_FOUND,
    "unsupported run action (expected :cancel, :resume, or :debug)",
  )
}

#[derive(Debug, Deserialize, Default)]
struct EventsQuery {
  after_seq: Option<i64>,
}

async fn events_history(
  State(state): State<LocalEngineState>,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<EventsQuery>,
) -> Response {
  let after = query.after_seq.unwrap_or(-1);
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id) else {
    return error_json(StatusCode::NOT_FOUND, "run not found");
  };
  let filtered: Vec<LocalStreamedEvent> = entry
    .events
    .iter()
    .filter(|e| e.seq > after)
    .cloned()
    .collect();
  Json(filtered).into_response()
}

async fn stream_events(
  State(state): State<LocalEngineState>,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<EventsQuery>,
) -> Response {
  let after = query.after_seq.unwrap_or(-1);
  let (backfill, rx, already_done) = {
    let guard = state.lock_runs();
    let Some(entry) = guard.get(&run_id) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    let rx = entry.tx.subscribe();
    let backfill: Vec<LocalStreamedEvent> = entry
      .events
      .iter()
      .filter(|e| e.seq > after)
      .cloned()
      .collect();
    let attempt_start = entry.attempt_start_seq;
    let done = is_terminal_status(&entry.record.status)
      || backfill
        .iter()
        .any(|e| e.seq >= attempt_start && is_terminal_event_kind(&e.kind));
    (backfill, rx, done)
  };

  let last_backfill_seq = backfill.last().map(|e| e.seq).unwrap_or(after);
  let backfill_stream = futures::stream::iter(backfill);

  let live_stream = if already_done {
    futures::future::Either::Left(futures::stream::empty())
  } else {
    let stream = BroadcastStream::new(rx)
      .filter_map(move |item| {
        let out = match item {
          Ok(ev) if ev.seq > last_backfill_seq => Some(ev),
          Ok(_) | Err(BroadcastStreamRecvError::Lagged(_)) => None,
        };
        futures::future::ready(out)
      })
      .scan(false, |terminated, ev| {
        if *terminated {
          return futures::future::ready(None);
        }
        if is_terminal_event_kind(&ev.kind) {
          *terminated = true;
        }
        futures::future::ready(Some(ev))
      });
    futures::future::Either::Right(stream)
  };

  let merged = backfill_stream.chain(live_stream).map(|ev| {
    let json = serde_json::to_string(&ev).unwrap_or_else(|_| "{}".to_string());
    Ok::<_, Infallible>(SseEvent::default().data(json))
  });

  Sse::new(merged)
    .keep_alive(
      KeepAlive::new()
        .interval(Duration::from_secs(15))
        .text("keep-alive"),
    )
    .into_response()
}

async fn list_artifacts(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
    return error_json(StatusCode::NOT_FOUND, "run not found");
  };
  Json(serde_json::json!({
    "run_id": run_id,
    "artifacts": entry.artifacts,
  }))
  .into_response()
}

async fn download_artifact(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((run_id, artifact_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let found = {
    let guard = state.lock_runs();
    guard
      .get(&run_id)
      .filter(|e| e.record.tenant_id == tenant)
      .and_then(|e| e.artifacts.iter().find(|a| a.id == artifact_id).cloned())
  };
  let Some(artifact) = found else {
    return error_json(StatusCode::NOT_FOUND, "artifact not found");
  };
  serve_file_inline(&artifact.file_path, &artifact.mime_type).await
}

#[derive(Debug, Clone)]
struct LocalZipEntry {
  node_id: String,
  name: String,
  mime_type: String,
  path: PathBuf,
}

fn sanitize_local_zip_segment(raw: &str, fallback: &str) -> String {
  let cleaned: String = raw
    .chars()
    .map(|c| {
      if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
        '_'
      } else {
        c
      }
    })
    .collect::<String>()
    .trim_matches(|c: char| c == '.' || c.is_whitespace())
    .to_string();
  if cleaned.is_empty() {
    fallback.to_string()
  } else {
    cleaned
  }
}

fn deduplicate_local_zip_path(
  seen: &mut std::collections::HashSet<String>,
  dir: &str,
  filename: &str,
) -> String {
  let primary = format!("{dir}/{filename}");
  if seen.insert(primary.clone()) {
    return primary;
  }
  let (stem, ext) = match filename.rsplit_once('.') {
    Some((s, e)) if !s.is_empty() => (s, Some(e)),
    _ => (filename, None),
  };
  let mut idx = 2usize;
  loop {
    let candidate = match ext {
      Some(e) => format!("{dir}/{stem}_{idx}.{e}"),
      None => format!("{dir}/{stem}_{idx}"),
    };
    if seen.insert(candidate.clone()) {
      return candidate;
    }
    idx += 1;
  }
}

fn discover_local_run_files(run_dir: &Path) -> Vec<LocalZipEntry> {
  let root = run_dir.join("artifacts");
  let Ok(node_dirs) = std::fs::read_dir(&root) else {
    return Vec::new();
  };
  let mut items = Vec::new();
  let mut sorted_nodes: Vec<_> = node_dirs.filter_map(|e| e.ok()).collect();
  sorted_nodes.sort_by_key(|e| e.file_name());
  for node_entry in sorted_nodes {
    let Ok(ft) = node_entry.file_type() else {
      continue;
    };
    if !ft.is_dir() {
      continue;
    }
    let node_id = node_entry.file_name().to_string_lossy().to_string();
    let Ok(files) = std::fs::read_dir(node_entry.path()) else {
      continue;
    };
    let mut sorted_files: Vec<_> = files.filter_map(|e| e.ok()).collect();
    sorted_files.sort_by_key(|e| e.file_name());
    for file_entry in sorted_files {
      let path = file_entry.path();
      if path.is_file() {
        let name = file_entry.file_name().to_string_lossy().to_string();
        items.push(LocalZipEntry {
          node_id: node_id.clone(),
          name,
          mime_type: infer_mime(&path).to_string(),
          path,
        });
      }
    }
  }
  items
}

fn build_local_artifacts_zip_blocking(
  run_id: Uuid,
  sources: Vec<LocalZipEntry>,
) -> std::io::Result<Option<(std::fs::File, u64)>> {
  use sha2::{Digest, Sha256};
  use std::io::{Read, Seek, SeekFrom, Write};

  let tmp = tempfile::tempfile()?;
  let mut writer = zip::ZipWriter::new(tmp);
  let options =
    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

  let mut seen = std::collections::HashSet::new();
  seen.insert("artifacts.json".to_string());
  let mut manifest_items = Vec::new();
  let mut buf = vec![0u8; 64 * 1024];

  for src in sources {
    let mut file = match std::fs::File::open(&src.path) {
      Ok(f) => f,
      Err(_) => continue,
    };
    let safe_node = sanitize_local_zip_segment(&src.node_id, "node");
    let safe_name = sanitize_local_zip_segment(&src.name, "output.bin");
    let entry_path = deduplicate_local_zip_path(&mut seen, &safe_node, &safe_name);

    writer
      .start_file(&entry_path, options)
      .map_err(std::io::Error::other)?;
    let mut hasher = Sha256::new();
    let mut size_bytes: u64 = 0;
    loop {
      let n = file.read(&mut buf)?;
      if n == 0 {
        break;
      }
      writer.write_all(&buf[..n])?;
      hasher.update(&buf[..n]);
      size_bytes += n as u64;
    }
    manifest_items.push(serde_json::json!({
      "node_id": src.node_id,
      "name": safe_name,
      "entry_path": entry_path,
      "mime_type": src.mime_type,
      "size_bytes": size_bytes,
      "sha256": format!("{:x}", hasher.finalize()),
    }));
  }

  if manifest_items.is_empty() {
    return Ok(None);
  }

  let manifest = serde_json::json!({
    "run_id": run_id,
    "artifacts": manifest_items,
  });
  let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(std::io::Error::other)?;
  writer
    .start_file("artifacts.json", options)
    .map_err(std::io::Error::other)?;
  writer.write_all(&manifest_bytes)?;

  let mut finished = writer.finish().map_err(std::io::Error::other)?;
  let total_len = finished.seek(SeekFrom::End(0))?;
  finished.seek(SeekFrom::Start(0))?;
  Ok(Some((finished, total_len)))
}

async fn download_artifacts_zip(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let registered = {
    let guard = state.lock_runs();
    let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    entry
      .artifacts
      .iter()
      .map(|a| LocalZipEntry {
        node_id: a.node_id.clone(),
        name: a.name.clone(),
        mime_type: a.mime_type.clone(),
        path: a.file_path.clone(),
      })
      .collect::<Vec<_>>()
  };

  let run_dir = state.base_dir.join(run_id.to_string());
  let sources = if !registered.is_empty() {
    registered
  } else {
    match tokio::task::spawn_blocking(move || discover_local_run_files(&run_dir)).await {
      Ok(found) => found,
      Err(err) => {
        return error_json(
          StatusCode::INTERNAL_SERVER_ERROR,
          &format!("artifact scan task failed: {err}"),
        );
      }
    }
  };

  if sources.is_empty() {
    return error_json(StatusCode::NOT_FOUND, "run has no downloadable artifacts");
  }

  let packed =
    match tokio::task::spawn_blocking(move || build_local_artifacts_zip_blocking(run_id, sources))
      .await
    {
      Ok(Ok(Some(v))) => v,
      Ok(Ok(None)) => {
        return error_json(StatusCode::NOT_FOUND, "no readable artifact files on disk");
      }
      Ok(Err(err)) => {
        return error_json(
          StatusCode::INTERNAL_SERVER_ERROR,
          &format!("failed to build zip archive: {err}"),
        );
      }
      Err(err) => {
        return error_json(
          StatusCode::INTERNAL_SERVER_ERROR,
          &format!("zip task panicked: {err}"),
        );
      }
    };

  let (std_file, total_len) = packed;
  let async_file = tokio::fs::File::from_std(std_file);
  let short_id = &run_id.to_string()[..8];
  let disposition = format!("attachment; filename=\"yanshi-run-{short_id}-artifacts.zip\"");

  let mut res = Response::new(Body::from_stream(ReaderStream::new(async_file)));
  res.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/zip"),
  );
  res
    .headers_mut()
    .insert(header::CONTENT_LENGTH, HeaderValue::from(total_len));
  if let Ok(val) = HeaderValue::from_str(&disposition) {
    res.headers_mut().insert(header::CONTENT_DISPOSITION, val);
  }
  res
}

#[derive(Debug, Deserialize)]
struct PreviewQuery {
  path: String,
}

async fn preview_file(
  State(state): State<LocalEngineState>,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<PreviewQuery>,
) -> Response {
  let run_dir = state.base_dir.join(run_id.to_string());
  let resolved = match resolve_confined_path(&run_dir, &query.path) {
    Ok(p) => p,
    Err(msg) => return error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let mime = infer_mime(&resolved);
  serve_file_inline(&resolved, mime).await
}

fn resolve_confined_path(run_dir: &Path, requested: &str) -> Result<PathBuf, String> {
  let trimmed = requested.trim();
  if trimmed.is_empty() {
    return Err("preview path cannot be empty".to_string());
  }
  let req_path = Path::new(trimmed);
  for comp in req_path.components() {
    if matches!(comp, Component::ParentDir) {
      return Err("parent directory '..' traversal is forbidden".to_string());
    }
  }
  let candidate = if req_path.is_absolute() {
    if !req_path.starts_with(run_dir) {
      return Err("path lies outside this run's directory".to_string());
    }
    req_path.to_path_buf()
  } else {
    run_dir.join(req_path)
  };
  let canon_root = std::fs::canonicalize(run_dir)
    .map_err(|_| "run directory does not exist on disk".to_string())?;
  let canon_file =
    std::fs::canonicalize(&candidate).map_err(|_| "preview file not found".to_string())?;
  if !canon_file.starts_with(&canon_root) {
    return Err("symlink escapes run directory".to_string());
  }
  Ok(canon_file)
}

async fn serve_file_inline(path: &Path, mime_type: &str) -> Response {
  let file = match tokio::fs::File::open(path).await {
    Ok(f) => f,
    Err(_) => return error_json(StatusCode::NOT_FOUND, "file not found"),
  };
  let len = file.metadata().await.ok().map(|m| m.len());
  let body = Body::from_stream(ReaderStream::new(file));

  let mut res = Response::new(body);
  let content_type = HeaderValue::from_str(mime_type)
    .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
  res.headers_mut().insert(header::CONTENT_TYPE, content_type);
  res.headers_mut().insert(
    header::CONTENT_DISPOSITION,
    HeaderValue::from_static("inline"),
  );
  res.headers_mut().insert(
    header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
  );
  if mime_type.starts_with("text/html") {
    res.headers_mut().insert(
      header::CONTENT_SECURITY_POLICY,
      HeaderValue::from_static("sandbox allow-scripts"),
    );
  }
  if let Some(size) = len
    && let Ok(v) = HeaderValue::from_str(&size.to_string())
  {
    res.headers_mut().insert(header::CONTENT_LENGTH, v);
  }
  res
}

fn infer_mime(path: &Path) -> &'static str {
  match path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("")
    .to_ascii_lowercase()
    .as_str()
  {
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "webp" => "image/webp",
    "svg" => "image/svg+xml",
    "wav" => "audio/wav",
    "mp3" => "audio/mpeg",
    "mp4" => "video/mp4",
    "webm" => "video/webm",
    "html" | "htm" => "text/html; charset=utf-8",
    "srt" => "application/x-subrip; charset=utf-8",
    "vtt" => "text/vtt; charset=utf-8",
    "pdf" => "application/pdf",
    "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "json" => "application/json; charset=utf-8",
    _ => "application/octet-stream",
  }
}

fn is_terminal_status(status: &str) -> bool {
  matches!(status, "succeeded" | "failed" | "cancelled")
}

fn is_terminal_event_kind(kind: &str) -> bool {
  matches!(
    kind,
    "workflow.completed" | "workflow.failed" | "workflow.cancelled"
  )
}

fn workflow_event_to_json(event: &WorkflowEvent) -> serde_json::Value {
  use WorkflowEvent as W;
  match event {
    W::WorkflowStarted { workflow_id, .. } => serde_json::json!({ "workflow_id": workflow_id }),
    W::WorkflowCompleted {
      workflow_id,
      duration,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "duration_ms": duration.as_millis() as u64,
    }),
    W::WorkflowFailed {
      workflow_id,
      error,
      duration,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "error": error,
      "duration_ms": duration.as_millis() as u64,
    }),
    W::WorkflowCancelled {
      workflow_id,
      reason,
      duration,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "reason": reason,
      "duration_ms": duration.as_millis() as u64,
    }),
    W::NodeStarted {
      workflow_id,
      node_id,
      ..
    } => serde_json::json!({ "workflow_id": workflow_id, "node_id": node_id }),
    W::NodeCompleted {
      workflow_id,
      node_id,
      duration,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "duration_ms": duration.as_millis() as u64,
    }),
    W::NodeInputCaptured {
      workflow_id,
      node_id,
      input,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "input": input,
    }),
    W::NodeOutputCaptured {
      workflow_id,
      node_id,
      output,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "output": output,
    }),
    W::NodeDebugPaused {
      workflow_id,
      node_id,
      phase,
      next_nodes,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "phase": phase,
      "next_nodes": next_nodes,
    }),
    W::NodeDebugResumed {
      workflow_id,
      node_id,
      action,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "action": action,
    }),
    W::NodeFailed {
      workflow_id,
      node_id,
      error,
      duration,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "error": error,
      "duration_ms": duration.as_millis() as u64,
    }),
    W::NodeSkipped {
      workflow_id,
      node_id,
      reason,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "reason": reason,
    }),
    W::CheckpointSaved {
      workflow_id,
      checkpoint_id,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "checkpoint_id": checkpoint_id,
    }),
    W::CheckpointRestored {
      workflow_id,
      checkpoint_id,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "checkpoint_id": checkpoint_id,
    }),
    W::RetryAttempt {
      workflow_id,
      node_id,
      attempt,
      max_attempts,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "attempt": attempt,
      "max_attempts": max_attempts,
    }),
    W::ResourceWarning {
      workflow_id,
      resource_type,
      usage,
      limit,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "resource_type": resource_type,
      "usage": usage,
      "limit": limit,
    }),
    W::NodeProgress {
      workflow_id,
      node_id,
      kind,
      detail,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "kind": kind,
      "detail": detail,
    }),
    W::NodeCostRecorded {
      workflow_id,
      node_id,
      cost_usd,
      total_cost_usd,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "cost_usd": cost_usd,
      "total_cost_usd": total_cost_usd,
    }),
    W::CostLimitExceeded {
      workflow_id,
      node_id,
      spent_usd,
      limit_usd,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "spent_usd": spent_usd,
      "limit_usd": limit_usd,
    }),
    W::LLMPromptSent {
      workflow_id,
      node_id,
      model,
      provider,
      temperature,
      max_tokens,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "model": model,
      "provider": provider,
      "temperature": temperature,
      "max_tokens": max_tokens,
    }),
    W::LLMResponseReceived {
      workflow_id,
      node_id,
      model,
      duration,
      usage,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "model": model,
      "duration_ms": duration.as_millis() as u64,
      "usage": usage.as_ref().map(|u| serde_json::json!({
        "prompt_tokens": u.prompt_tokens,
        "completion_tokens": u.completion_tokens,
        "total_tokens": u.total_tokens,
      })),
    }),
    W::ResumeDecisionRecorded {
      workflow_id,
      node_id,
      tool_call_id,
      tool,
      step_index,
      idempotency,
      decision,
      reason,
      force_replay,
      ..
    } => serde_json::json!({
      "workflow_id": workflow_id,
      "node_id": node_id,
      "resume": {
        "tool_call_id": tool_call_id,
        "tool": tool,
        "step_index": step_index,
        "idempotency": idempotency,
        "decision": decision,
        "reason": reason,
        "force_replay": force_replay,
      },
    }),
    other => serde_json::json!({
      "workflow_id": other.workflow_id(),
      "summary": other.to_string(),
    }),
  }
}

pub(crate) fn error_json(status: StatusCode, message: &str) -> Response {
  (status, Json(serde_json::json!({ "error": message }))).into_response()
}

#[cfg(test)]
mod tests {
  use super::*;

  const SAMPLE_YAML: &str = r#"name: local_engine_step_test
inputs:
  topic:
    default: "Rust Traits"
nodes:
  - id: first
    type: template
    parameters:
      template: "Step 1: {{ topic }}"
  - id: second
    type: template
    dependencies: ["first"]
    input_mapping:
      prev: "{{ nodes.first.outputs.output }}"
    parameters:
      template: "Step 2 from ({{ prev }})"
"#;

  #[tokio::test]
  async fn local_engine_supports_inspect_and_step_by_step_debug() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start local engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. Inspect workflow
    let inspect_res: InspectWorkflowResponse = client
      .post(format!("{base_url}/v1/workflows/inspect"))
      .json(&serde_json::json!({ "workflow": SAMPLE_YAML }))
      .send()
      .await
      .expect("inspect request")
      .json()
      .await
      .expect("inspect json");
    assert!(inspect_res.valid);
    assert_eq!(inspect_res.nodes.len(), 2);

    // 2. Submit in debug mode (`step_every_node: true`)
    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/runs"))
      .json(&serde_json::json!({
        "workflow": SAMPLE_YAML,
        "inputs": { "topic": "Zero-Cost Abstractions" },
        "run_mode": "debug",
        "step_every_node": true,
      }))
      .send()
      .await
      .expect("create run")
      .json()
      .await
      .expect("create run json");
    let run_id = created["run_id"].as_str().expect("run_id").to_string();

    // Wait until paused after `first`
    let mut paused_node = None;
    for _ in 0..50 {
      let dbg: serde_json::Value = client
        .get(format!("{base_url}/v1/runs/{run_id}/debug"))
        .send()
        .await
        .expect("get debug")
        .json()
        .await
        .expect("debug json");
      if let Some(nid) = dbg["snapshot"]["paused_at"]["node_id"].as_str() {
        paused_node = Some(nid.to_string());
        break;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(paused_node.as_deref(), Some("first"));

    // Continue run to completion while overriding `first`'s output for `second`
    let cont_res = client
      .post(format!("{base_url}/v1/runs/{run_id}:debug"))
      .json(&serde_json::json!({
        "action": "continue",
        "output_override": {
          "output": "Overridden Step 1"
        }
      }))
      .send()
      .await
      .expect("continue debug");
    assert!(cont_res.status().is_success());

    // Wait for `succeeded`
    let mut final_status = String::new();
    for _ in 0..50 {
      let run_json: serde_json::Value = client
        .get(format!("{base_url}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get run")
        .json()
        .await
        .expect("run json");
      final_status = run_json["status"].as_str().unwrap_or("").to_string();
      if final_status == "succeeded" {
        break;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(final_status, "succeeded");

    // Verify captured node I/O events exist in history
    let events: Vec<LocalStreamedEvent> = client
      .get(format!("{base_url}/v1/runs/{run_id}/events/history"))
      .send()
      .await
      .expect("events history")
      .json()
      .await
      .expect("events json");
    assert!(events.iter().any(|e| e.kind == "node.input.captured"));
    assert!(events.iter().any(|e| e.kind == "node.output.captured"));
    let second_outputs: Vec<_> = events
      .iter()
      .filter(|e| e.kind == "node.output.captured" && e.payload["node_id"] == "second")
      .collect();
    assert!(!second_outputs.is_empty());
    let serialized = serde_json::to_string(&second_outputs.last()).expect("serialize second out");
    assert!(serialized.contains("Step 2 from (Overridden Step 1)"));
  }

  #[tokio::test]
  async fn local_engine_inspects_tech_slides_video_and_previews_rendered_slide() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start local engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. Inspect the real `examples/generators/tech_slides_video.yml` workflow
    let flagship_path =
      Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/generators/tech_slides_video.yml");
    let flagship_yaml =
      std::fs::read_to_string(&flagship_path).expect("read tech_slides_video.yml");
    let inspect_res: InspectWorkflowResponse = client
      .post(format!("{base_url}/v1/workflows/inspect"))
      .json(&serde_json::json!({ "workflow": flagship_yaml }))
      .send()
      .await
      .expect("inspect flagship")
      .json()
      .await
      .expect("inspect flagship json");
    assert!(inspect_res.valid);
    assert_eq!(inspect_res.nodes.len(), 11);

    // 2. Run a 2-node one-click `release` slide-rendering workflow end-to-end
    let slide_yaml = r#"name: smoke_slide_release
nodes:
  - id: deck_json
    type: template
    parameters:
      output_format: json
      template: |
        {
          "deck": {
            "title": "Rust Trait Dispatch",
            "theme": "dark",
            "slides": [
              {
                "title": "Static vs Dynamic",
                "left": {
                  "kind": "code",
                  "language": "rust",
                  "code": "fn run(x: &impl Draw) { x.draw(); }"
                },
                "takeaway": "Zero-cost by default.",
                "notes": "Static dispatch monomorphizes at compile time."
              }
            ]
          }
        }
  - id: slides
    type: render_slides
    dependencies: ["deck_json"]
    input_mapping:
      deck: "{{ nodes.deck_json.outputs.deck }}"
    parameters:
      width: 1280
      progressive: false
"#;

    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/runs"))
      .json(&serde_json::json!({
        "workflow": slide_yaml,
        "run_mode": "release",
      }))
      .send()
      .await
      .expect("create release run")
      .json()
      .await
      .expect("create release json");
    let run_id = created["run_id"].as_str().expect("run_id").to_string();

    let mut final_status = String::new();
    let mut final_err = String::new();
    for _ in 0..100 {
      let run_json: serde_json::Value = client
        .get(format!("{base_url}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get run")
        .json()
        .await
        .expect("run json");
      final_status = run_json["status"].as_str().unwrap_or("").to_string();
      final_err = run_json["error"].as_str().unwrap_or("").to_string();
      if is_terminal_status(&final_status) {
        break;
      }
      tokio::time::sleep(Duration::from_millis(40)).await;
    }
    assert_eq!(
      final_status, "succeeded",
      "unexpected run error: {final_err}"
    );

    // Verify artifacts & sandboxed HTML preview
    let arts: serde_json::Value = client
      .get(format!("{base_url}/v1/runs/{run_id}/artifacts"))
      .send()
      .await
      .expect("list artifacts")
      .json()
      .await
      .expect("artifacts json");
    let list = arts["artifacts"].as_array().expect("artifacts array");
    assert!(!list.is_empty());
    let html_art = list
      .iter()
      .find(|a| a["mime_type"].as_str() == Some("text/html"))
      .expect("html artifact");
    let html_name = html_art["name"].as_str().expect("html artifact name");

    let html_preview = client
      .get(format!(
        "{base_url}/v1/runs/{run_id}/preview?path=artifacts/slides/{html_name}"
      ))
      .send()
      .await
      .expect("preview html");
    assert_eq!(html_preview.status(), StatusCode::OK);
    assert_eq!(
      html_preview
        .headers()
        .get("content-security-policy")
        .and_then(|v| v.to_str().ok()),
      Some("sandbox allow-scripts")
    );

    // Verify batch `.zip` download & cross-tenant 404 isolation
    let cross_tenant_zip = client
      .get(format!("{base_url}/v1/runs/{run_id}/artifacts.zip"))
      .header("X-Yanshi-Tenant", "other-tenant")
      .send()
      .await
      .expect("cross tenant zip");
    assert_eq!(cross_tenant_zip.status(), StatusCode::NOT_FOUND);

    let zip_res = client
      .get(format!("{base_url}/v1/runs/{run_id}/artifacts.zip"))
      .send()
      .await
      .expect("download artifacts zip");
    assert_eq!(zip_res.status(), StatusCode::OK);
    assert_eq!(
      zip_res
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok()),
      Some("application/zip")
    );
    let zip_bytes = zip_res.bytes().await.expect("zip bytes");
    let mut zip_archive =
      zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).expect("valid zip archive");
    assert!(zip_archive.len() >= 2);
    assert!(
      zip_archive.by_name("artifacts.json").is_ok(),
      "zip archive should contain artifacts.json manifest"
    );
  }

  #[tokio::test]
  async fn local_engine_serves_diagnostics_without_leaking_tokens() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start local engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    let key = "YANSHI_API_TOKEN";
    let previous = std::env::var(key).ok();
    let secret = "sk-local-engine-diagnostics-secret-never-leak";
    // SAFETY: test-local env mutation restored before assertions.
    unsafe {
      std::env::set_var(key, secret);
    }

    let res = client
      .get(format!("{base_url}/v1/diagnostics"))
      .send()
      .await;

    // SAFETY: restore env before asserting.
    unsafe {
      match previous {
        Some(val) => std::env::set_var(key, val),
        None => std::env::remove_var(key),
      }
    }

    let res = res.expect("diagnostics request");
    assert_eq!(res.status(), StatusCode::OK);
    let json: serde_json::Value = res.json().await.expect("diagnostics json");
    let status = json["status"].as_str().expect("status string");
    assert!(matches!(status, "ok" | "warning" | "fail"));
    let serialized = serde_json::to_string(&json).expect("serialize");
    assert!(
      !serialized.contains(secret),
      "diagnostics response leaked YANSHI_API_TOKEN value"
    );
    assert_eq!(
      json
        .pointer("/environment/yanshi_api_token_set")
        .and_then(|v| v.as_bool()),
      Some(true)
    );
  }

  #[tokio::test]
  async fn local_engine_persists_tenant_preferences_and_rejects_secrets() {
    let temp = tempfile::tempdir().expect("tempdir");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start local engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. Starts empty
    let initial: serde_json::Value = client
      .get(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .send()
      .await
      .expect("get initial prefs")
      .json()
      .await
      .expect("initial json");
    assert_eq!(initial, serde_json::json!({ "preferences": {} }));

    // 2. Upsert preferences for tenant `alpha`
    let put_res = client
      .put(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .json(&serde_json::json!({
        "preferences": {
          "theme": "light",
          "ui.run-list.page-size": 50
        }
      }))
      .send()
      .await
      .expect("put prefs");
    assert_eq!(put_res.status(), StatusCode::OK);
    let saved: serde_json::Value = put_res.json().await.expect("put json");
    assert_eq!(saved["preferences"]["theme"], "light");
    assert_eq!(saved["preferences"]["ui.run-list.page-size"], 50);

    // 3. Tenant `beta` stays isolated
    let beta: serde_json::Value = client
      .get(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "beta")
      .send()
      .await
      .expect("get beta prefs")
      .json()
      .await
      .expect("beta json");
    assert_eq!(beta, serde_json::json!({ "preferences": {} }));

    // 4. Rejects token-shaped secret values and aborts batch atomically
    let secret_attempt = client
      .put(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .json(&serde_json::json!({
        "preferences": {
          "theme": "dark",
          "leaked_token": "sk-abcdefghijklmnopqrstuvwxyz0123456789"
        }
      }))
      .send()
      .await
      .expect("secret put");
    assert_eq!(secret_attempt.status(), StatusCode::BAD_REQUEST);

    // `theme` stays `"light"` because the rejected batch was atomic
    let after_reject: serde_json::Value = client
      .get(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .send()
      .await
      .expect("get alpha after reject")
      .json()
      .await
      .expect("alpha json");
    assert_eq!(after_reject["preferences"]["theme"], "light");

    // 5. Rejects invalid key names
    let invalid_key = client
      .put(format!("{base_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .json(&serde_json::json!({
        "preferences": {
          "invalid key with spaces": true
        }
      }))
      .send()
      .await
      .expect("invalid key put");
    assert_eq!(invalid_key.status(), StatusCode::BAD_REQUEST);

    // 6. Restart loopback engine against the same directory and verify persistence
    let restarted_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("restart local engine");
    let reloaded: serde_json::Value = client
      .get(format!("{restarted_url}/v1/preferences"))
      .header("X-Yanshi-Tenant", "alpha")
      .send()
      .await
      .expect("get reloaded prefs")
      .json()
      .await
      .expect("reloaded json");
    assert_eq!(reloaded["preferences"]["theme"], "light");
    assert_eq!(reloaded["preferences"]["ui.run-list.page-size"], 50);
  }

  #[tokio::test]
  async fn local_engine_supports_checkpoint_resume_plan_and_restart_recovery() {
    let temp_runs = tempfile::tempdir().expect("temp_runs");
    let temp_ws = tempfile::tempdir().expect("temp_ws");
    let canon_ws = std::fs::canonicalize(temp_ws.path()).expect("canonicalize ws");
    let step1_file = canon_ws.join("step1_marker.txt");
    let gate_file = canon_ws.join("gate_input.txt");

    let workflow = format!(
      r#"name: resume_checkpoint_test
nodes:
  - id: first
    type: file
    parameters:
      allowed_paths:
        - "{ws}"
      operation: write
      path: "{step1}"
      content: "written_on_attempt_1"
  - id: second
    type: file
    dependencies: ["first"]
    parameters:
      allowed_paths:
        - "{ws}"
      operation: read
      path: "{gate}"
  - id: third
    type: template
    dependencies: ["second"]
    input_mapping:
      body: "{{{{ nodes.second.outputs.content }}}}"
    parameters:
      template: "Final: {{{{ body }}}}"
"#,
      ws = canon_ws.display(),
      step1 = step1_file.display(),
      gate = gate_file.display(),
    );

    let base_url = start_loopback_engine(Some(temp_runs.path().to_path_buf()))
      .await
      .expect("start engine");
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. First run fails on `second` because `gate_input.txt` does not exist yet
    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/runs"))
      .header("X-Yanshi-Tenant", "team-resume")
      .json(&serde_json::json!({
        "workflow": workflow,
        "cost_limit_usd": 2.5,
      }))
      .send()
      .await
      .expect("create run")
      .json()
      .await
      .expect("create json");
    let run_id = created["run_id"].as_str().expect("run_id").to_string();

    let mut status = String::new();
    for _ in 0..80 {
      let run_json: serde_json::Value = client
        .get(format!("{base_url}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get run")
        .json()
        .await
        .expect("run json");
      status = run_json["status"].as_str().unwrap_or("").to_string();
      if is_terminal_status(&status) {
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(status, "failed");
    assert_eq!(
      std::fs::read_to_string(&step1_file).expect("step1 written"),
      "written_on_attempt_1"
    );

    // Mutate `step1_marker.txt` so we can prove `first` is NOT re-executed on resume,
    // and create `gate_input.txt` so `second` + `third` succeed on resume.
    std::fs::write(&step1_file, "preserved_across_resume").expect("mutate step1");
    std::fs::write(&gate_file, "gate_opened_payload").expect("write gate");

    // 2. Restart a brand-new loopback engine over the same directory to verify `run_meta.json` reload
    let restarted_url = start_loopback_engine(Some(temp_runs.path().to_path_buf()))
      .await
      .expect("restart engine");

    // Cross-tenant resume-plan is 404
    let wrong_tenant_plan = client
      .get(format!("{restarted_url}/v1/runs/{run_id}/resume-plan"))
      .header("X-Yanshi-Tenant", "intruder")
      .send()
      .await
      .expect("wrong tenant plan");
    assert_eq!(wrong_tenant_plan.status(), StatusCode::NOT_FOUND);

    // Owner tenant gets valid ResumePlan showing `first` completed
    let plan_res = client
      .get(format!("{restarted_url}/v1/runs/{run_id}/resume-plan"))
      .header("X-Yanshi-Tenant", "team-resume")
      .send()
      .await
      .expect("get resume plan");
    assert_eq!(plan_res.status(), StatusCode::OK);
    let plan_json: serde_json::Value = plan_res.json().await.expect("plan json");
    assert_eq!(plan_json["workflow_id"], run_id);
    assert_eq!(plan_json["last_completed_node"], "first");

    // 3. Resume the failed run via `POST /v1/runs/{id}:resume`
    let resume_res = client
      .post(format!("{restarted_url}/v1/runs/{run_id}:resume"))
      .header("X-Yanshi-Tenant", "team-resume")
      .send()
      .await
      .expect("post resume");
    assert_eq!(resume_res.status(), StatusCode::OK);

    let mut resumed_status = String::new();
    for _ in 0..80 {
      let run_json: serde_json::Value = client
        .get(format!("{restarted_url}/v1/runs/{run_id}"))
        .send()
        .await
        .expect("get resumed run")
        .json()
        .await
        .expect("resumed run json");
      resumed_status = run_json["status"].as_str().unwrap_or("").to_string();
      if is_terminal_status(&resumed_status) {
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(resumed_status, "succeeded");

    // `first` was NOT re-executed
    assert_eq!(
      std::fs::read_to_string(&step1_file).expect("read step1 after resume"),
      "preserved_across_resume"
    );

    // Events continue monotonically across both attempts and include `third`'s final output
    let events: Vec<LocalStreamedEvent> = client
      .get(format!("{restarted_url}/v1/runs/{run_id}/events/history"))
      .send()
      .await
      .expect("events history")
      .json()
      .await
      .expect("events json");
    for pair in events.windows(2) {
      assert!(
        pair[0].seq < pair[1].seq,
        "event seq must strictly increase"
      );
    }
    assert_eq!(
      events
        .iter()
        .filter(|e| e.kind == "node.started" && e.payload["node_id"] == "first")
        .count(),
      1,
      "`first` must only execute once across initial run + checkpoint resume"
    );
    assert!(events.iter().any(|e| e.kind == "workflow.completed"));
    let third_outputs: Vec<_> = events
      .iter()
      .filter(|e| e.kind == "node.output.captured" && e.payload["node_id"] == "third")
      .collect();
    assert!(!third_outputs.is_empty());
    let out_str = serde_json::to_string(&third_outputs.last()).expect("serialize third out");
    assert!(out_str.contains("Final: gate_opened_payload"));
  }

  #[tokio::test]
  async fn local_engine_supports_run_level_approvals_and_rejects_bad_cost_limit() {
    use yanshi_harness::{ApprovalRequest, ApprovalRisk};

    let temp = tempfile::tempdir().expect("tempdir");
    let state = LocalEngineState::new(temp.path().to_path_buf());
    let approvals_reg = state.harness().approvals().clone();
    let app = build_local_router(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let port = listener.local_addr().expect("addr").port();
    tokio::spawn(async move {
      let _ = axum::serve(listener, app).await;
    });
    let base_url = format!("http://127.0.0.1:{port}");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // Rejects non-positive `cost_limit_usd`
    let bad_cost = client
      .post(format!("{base_url}/v1/runs"))
      .json(&serde_json::json!({
        "workflow": SAMPLE_YAML,
        "cost_limit_usd": -0.5,
      }))
      .send()
      .await
      .expect("bad cost");
    assert_eq!(bad_cost.status(), StatusCode::BAD_REQUEST);

    // Submit a paused debug run so we can park and resolve a run-scoped approval
    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/runs"))
      .header("X-Yanshi-Tenant", "ops")
      .json(&serde_json::json!({
        "workflow": SAMPLE_YAML,
        "run_mode": "debug",
        "step_every_node": true,
      }))
      .send()
      .await
      .expect("create run")
      .json()
      .await
      .expect("create json");
    let run_id = created["run_id"].as_str().expect("run_id").to_string();

    let rx = approvals_reg.park(ApprovalRequest {
      id: "run-req-1".to_string(),
      session_id: run_id.clone(),
      step_index: 0,
      tool: "shell".to_string(),
      source: None,
      permissions: Vec::new(),
      idempotency: Default::default(),
      effect: Default::default(),
      params_summary: serde_json::json!({ "command": "ls -la" }),
      risk: ApprovalRisk::High,
      reason: "workflow tool approval".to_string(),
      requested_at: Utc::now(),
      expires_at: None,
    });

    // Cross-tenant cannot list run approvals
    let cross_list = client
      .get(format!("{base_url}/v1/runs/{run_id}/approvals"))
      .header("X-Yanshi-Tenant", "other")
      .send()
      .await
      .expect("cross list");
    assert_eq!(cross_list.status(), StatusCode::NOT_FOUND);

    // Owner tenant sees the parked approval and resolves it
    let list_json: serde_json::Value = client
      .get(format!("{base_url}/v1/runs/{run_id}/approvals"))
      .header("X-Yanshi-Tenant", "ops")
      .send()
      .await
      .expect("list approvals")
      .json()
      .await
      .expect("list json");
    assert_eq!(list_json["approvals"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_json["approvals"][0]["id"], "run-req-1");

    let decide_res: serde_json::Value = client
      .post(format!("{base_url}/v1/runs/{run_id}/approvals/run-req-1"))
      .header("X-Yanshi-Tenant", "ops")
      .json(&serde_json::json!({
        "decision": "allow",
        "scope": "once",
        "decided_by": "studio-ui",
      }))
      .send()
      .await
      .expect("decide run approval")
      .json()
      .await
      .expect("decide json");
    assert_eq!(decide_res["resolved"], true);
    let decision = rx.await.expect("approval oneshot");
    assert_eq!(decision.decision, ApprovalOutcome::Allow);

    // Clean up paused debug run
    let _ = client
      .post(format!("{base_url}/v1/runs/{run_id}:cancel"))
      .send()
      .await;
  }
}
