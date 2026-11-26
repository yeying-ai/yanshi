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
    Html, IntoResponse, Redirect, Response,
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
use crate::local_questions::{
  LocalQuestionResolveError, LocalRunQuestionHandler, LocalRunQuestionRegistry,
};

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
  #[serde(default)]
  pub sha256: Option<String>,
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
  #[serde(default)]
  sha256: Option<String>,
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
      sha256: value.sha256.clone(),
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
      sha256: value.sha256,
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
  #[serde(default)]
  notebook_id: Option<Uuid>,
  #[serde(default)]
  notebook_kind: Option<String>,
}

pub(crate) type RunCompletionHook =
  Arc<dyn Fn(Uuid, &str, &[LocalArtifactSummary], Option<&serde_json::Value>) + Send + Sync>;

struct RunEntry {
  record: LocalRunRecord,
  events: Vec<LocalStreamedEvent>,
  /// False for hydrated terminal runs until their `events.jsonl` is lazily read.
  events_loaded: bool,
  artifacts: Vec<LocalArtifactSummary>,
  tx: broadcast::Sender<LocalStreamedEvent>,
  cancel_token: FlowCancellationToken,
  debug_controller: Option<FlowDebugController>,
  attempt_start_seq: i64,
  model: Option<String>,
  cost_limit_usd: Option<f64>,
  notebook_id: Option<Uuid>,
  notebook_kind: Option<String>,
  on_complete: Option<RunCompletionHook>,
}

type TenantPreferencesMap = BTreeMap<String, BTreeMap<String, serde_json::Value>>;

#[derive(Clone)]
pub struct LocalEngineState {
  runs: Arc<Mutex<HashMap<Uuid, RunEntry>>>,
  preferences: Arc<Mutex<TenantPreferencesMap>>,
  questions: LocalRunQuestionRegistry,
  harness: crate::local_harness::LocalHarnessState,
  notebooks: crate::local_notebooks::LocalNotebooksState,
  base_dir: PathBuf,
  api_token: Option<Arc<str>>,
}

impl LocalEngineState {
  pub fn new(base_dir: PathBuf) -> Self {
    let initial_prefs = load_preferences_from_disk(&base_dir);
    let hydrated_runs = hydrate_runs_from_disk(&base_dir);
    let questions = LocalRunQuestionRegistry::new();
    let harness = crate::local_harness::LocalHarnessState::new(base_dir.clone());
    let notebooks = crate::local_notebooks::LocalNotebooksState::new(base_dir.clone());
    Self {
      runs: Arc::new(Mutex::new(hydrated_runs)),
      preferences: Arc::new(Mutex::new(initial_prefs)),
      questions,
      harness,
      notebooks,
      base_dir,
      api_token: None,
    }
  }

  pub fn with_api_token(mut self, token: impl Into<String>) -> Self {
    let trimmed = token.into();
    let trimmed = trimmed.trim();
    if !trimmed.is_empty() {
      self.api_token = Some(Arc::from(trimmed));
    }
    self
  }

  pub fn api_token(&self) -> Option<&str> {
    self.api_token.as_deref()
  }

  pub fn base_dir(&self) -> &Path {
    &self.base_dir
  }

  pub fn questions(&self) -> &LocalRunQuestionRegistry {
    &self.questions
  }

  pub fn harness(&self) -> &crate::local_harness::LocalHarnessState {
    &self.harness
  }

  pub fn notebooks(&self) -> &crate::local_notebooks::LocalNotebooksState {
    &self.notebooks
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

  /// Lazily hydrate `events.jsonl` from disk into memory for a previously
  /// persisted run without holding `self.runs` during filesystem I/O.
  async fn ensure_run_events_loaded_async(&self, run_id: Uuid) {
    let needs_load = {
      let guard = self.lock_runs();
      guard.get(&run_id).is_some_and(|entry| !entry.events_loaded)
    };
    if !needs_load {
      return;
    }
    let path = self.base_dir.join(run_id.to_string()).join("events.jsonl");
    let loaded = tokio::task::spawn_blocking(move || read_run_events_jsonl(&path))
      .await
      .unwrap_or_default();
    let mut guard = self.lock_runs();
    if let Some(entry) = guard.get_mut(&run_id)
      && !entry.events_loaded
    {
      entry.events = loaded;
      entry.events_loaded = true;
    }
  }

  /// Sync variant used by synchronous helper paths (e.g. attach-run).
  fn ensure_run_events_loaded_sync(&self, run_id: Uuid) {
    let needs_load = {
      let guard = self.lock_runs();
      guard.get(&run_id).is_some_and(|entry| !entry.events_loaded)
    };
    if !needs_load {
      return;
    }
    let path = self.base_dir.join(run_id.to_string()).join("events.jsonl");
    let loaded = read_run_events_jsonl(&path);
    let mut guard = self.lock_runs();
    if let Some(entry) = guard.get_mut(&run_id)
      && !entry.events_loaded
    {
      entry.events = loaded;
      entry.events_loaded = true;
    }
  }

  /// Return true if any DAG run or Harness session is currently non-terminal.
  pub fn has_active_work(&self) -> bool {
    let active_run = self
      .lock_runs()
      .values()
      .any(|entry| !is_terminal_status(&entry.record.status));
    active_run || self.harness.has_active_sessions()
  }

  /// Gracefully cancel every in-flight DAG run and Harness session and flush
  /// their cancelled metadata to disk before process exit.
  pub fn cancel_all_active(&self) -> (usize, usize) {
    let mut to_flush: Vec<(PathBuf, PersistedRunMeta, Option<LocalStreamedEvent>)> = Vec::new();
    {
      let mut guard = self.lock_runs();
      for (&run_id, entry) in guard.iter_mut() {
        if is_terminal_status(&entry.record.status) {
          continue;
        }
        entry.cancel_token.cancel();
        self.questions.clear_run(run_id);
        if let Some(ctrl) = entry.debug_controller.take() {
          ctrl.abort();
        }
        entry.record.status = "cancelled".to_string();
        entry.record.finished_at = Some(Utc::now());
        let synthetic = push_synthetic_terminal_event(entry, "workflow.cancelled", None);
        let run_dir = self.base_dir.join(run_id.to_string());
        let meta = PersistedRunMeta::from_entry(entry);
        to_flush.push((run_dir, meta, synthetic));
      }
    }

    let cancelled_runs = to_flush.len();
    for (run_dir, meta, synthetic_ev) in to_flush {
      if let Err(err) = write_persisted_run_meta_sync(&run_dir, &meta) {
        tracing::warn!(run_dir = %run_dir.display(), error = %err, "failed to flush cancelled run meta on shutdown");
      }
      if let Some(ev) = synthetic_ev
        && let Err(err) = append_run_event_jsonl_sync(&run_dir, &ev)
      {
        tracing::warn!(run_dir = %run_dir.display(), error = %err, "failed to flush cancelled run event on shutdown");
      }
    }

    let cancelled_sessions = self.harness.cancel_all_active();
    (cancelled_runs, cancelled_sessions)
  }
}

impl PersistedRunMeta {
  fn from_entry(entry: &RunEntry) -> Self {
    Self {
      record: entry.record.clone(),
      artifacts: entry
        .artifacts
        .iter()
        .map(PersistedArtifactRecord::from)
        .collect(),
      model: entry.model.clone(),
      cost_limit_usd: entry.cost_limit_usd,
      notebook_id: entry.notebook_id,
      notebook_kind: entry.notebook_kind.clone(),
    }
  }
}

fn write_persisted_run_meta_sync(run_dir: &Path, meta: &PersistedRunMeta) -> std::io::Result<()> {
  std::fs::create_dir_all(run_dir)?;
  let bytes = serde_json::to_vec_pretty(meta).map_err(std::io::Error::other)?;
  let tmp = run_dir.join(format!("run_meta.json.tmp.{}", Uuid::new_v4()));
  let dst = run_dir.join("run_meta.json");
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

async fn write_persisted_run_meta_async(
  run_dir: PathBuf,
  meta: PersistedRunMeta,
) -> std::io::Result<()> {
  tokio::task::spawn_blocking(move || write_persisted_run_meta_sync(&run_dir, &meta))
    .await
    .map_err(std::io::Error::other)?
}

fn checkpoint_dir_for_local_run(base_dir: &Path, run_id: Uuid) -> PathBuf {
  base_dir.join(run_id.to_string()).join("checkpoint")
}

fn write_run_meta_sync(run_dir: &Path, entry: &RunEntry) -> std::io::Result<()> {
  let meta = PersistedRunMeta::from_entry(entry);
  write_persisted_run_meta_sync(run_dir, &meta)
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
    let attempt_start_seq = 0;
    let (tx, _rx) = broadcast::channel(512);
    let entry = RunEntry {
      record: meta.record,
      events: Vec::new(),
      events_loaded: false,
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
      notebook_id: meta.notebook_id,
      notebook_kind: meta.notebook_kind,
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

pub fn default_studio_runs_dir() -> PathBuf {
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

const BUNDLED_PRICING_YAML: &str = include_str!("../../../config/pricing.prod.yml");

pub(crate) fn should_seed_bundled_pricing_env(
  explicit_env: Option<&str>,
  home_dir: Option<&Path>,
) -> bool {
  if explicit_env.is_some_and(|val| !val.trim().is_empty()) {
    return false;
  }
  if home_dir.is_some_and(|home| home.join(".yanshi").join("pricing.yml").is_file()) {
    return false;
  }
  true
}

pub(crate) fn materialize_bundled_pricing_table(base_dir: &Path) -> std::io::Result<PathBuf> {
  std::fs::create_dir_all(base_dir)?;
  let dst = base_dir.join("_bundled_pricing.yml");
  let tmp = base_dir.join(format!("_bundled_pricing.yml.tmp.{}", Uuid::new_v4()));
  std::fs::write(&tmp, BUNDLED_PRICING_YAML.as_bytes())?;
  std::fs::rename(tmp, &dst)?;
  Ok(dst)
}

pub(crate) fn load_user_dotenv() {
  static DOTENV_ONCE: std::sync::Once = std::sync::Once::new();
  DOTENV_ONCE.call_once(|| {
    let _ = dotenvy::dotenv();
    if let Some(home) = dirs::home_dir() {
      let user_env = home.join(".yanshi").join(".env");
      if user_env.is_file() {
        let _ = dotenvy::from_path(user_env);
      }
    }
  });
}

/// Initialize process environment (`~/.yanshi/.env` + bundled baseline
/// `pricing.prod.yml` fallback) synchronously on the main thread before Tokio
/// or Tauri worker threads are spawned.
pub fn bootstrap_process_env() {
  load_user_dotenv();
  let explicit = std::env::var("YANSHI_PRICING_TABLE").ok();
  let home = dirs::home_dir();
  if should_seed_bundled_pricing_env(explicit.as_deref(), home.as_deref()) {
    let base_dir = default_studio_runs_dir();
    if let Ok(path) = materialize_bundled_pricing_table(&base_dir) {
      // SAFETY: `bootstrap_process_env` runs synchronously at the start of
      // `main()` before any Tokio or Tauri worker threads exist.
      unsafe {
        std::env::set_var("YANSHI_PRICING_TABLE", path);
      }
    }
  }
}

pub(crate) async fn ensure_llm_initialized() -> anyhow::Result<()> {
  static INIT: tokio::sync::OnceCell<()> = tokio::sync::OnceCell::const_new();
  INIT
    .get_or_try_init(|| async {
      load_user_dotenv();
      Yanshi::ensure_init().await
    })
    .await?;
  Ok(())
}

const EMBEDDED_INDEX_HTML: &str = include_str!("../../dist/index.html");
const EMBEDDED_APP_JS: &str = include_str!("../../dist/assets/app.js");
const EMBEDDED_STYLES_CSS: &str = include_str!("../../dist/assets/styles.css");

/// Generate a per-launch 256-bit loopback bearer token, honoring an explicit
/// non-empty `YANSHI_API_TOKEN` env override when supplied by the operator.
pub fn generate_loopback_api_token() -> String {
  if let Ok(explicit) = std::env::var("YANSHI_API_TOKEN") {
    let trimmed = explicit.trim();
    if !trimmed.is_empty() {
      return trimmed.to_string();
    }
  }
  format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

/// Start the embedded loopback engine on `127.0.0.1:0` with bearer-token
/// authentication enabled and return `(base_url, api_token, state)`.
pub async fn start_authenticated_loopback_engine_with_state(
  custom_base_dir: Option<PathBuf>,
) -> anyhow::Result<(String, String, LocalEngineState)> {
  let base_dir = custom_base_dir.unwrap_or_else(default_studio_runs_dir);
  tokio::fs::create_dir_all(&base_dir).await?;
  let token = generate_loopback_api_token();
  let state = LocalEngineState::new(base_dir).with_api_token(token.clone());
  let app = build_local_router(state.clone());

  let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
  let addr: SocketAddr = listener.local_addr()?;
  tokio::spawn(async move {
    let _ = axum::serve(listener, app).await;
  });
  Ok((format!("http://127.0.0.1:{}", addr.port()), token, state))
}

/// Start the embedded loopback engine on `127.0.0.1:0` with bearer-token
/// authentication enabled and return `(base_url, api_token)`.
pub async fn start_authenticated_loopback_engine(
  custom_base_dir: Option<PathBuf>,
) -> anyhow::Result<(String, String)> {
  let (url, token, _state) =
    start_authenticated_loopback_engine_with_state(custom_base_dir).await?;
  Ok((url, token))
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

/// Bind the zero-Postgres local engine + embedded SPA on an explicit loopback
/// socket (`127.0.0.1:<port>` or `[::1]:<port>`) with bearer authentication,
/// returning the `LocalEngineState` handle for graceful shutdown cancellation.
pub async fn bind_authenticated_local_web_listener_with_state(
  bind: SocketAddr,
  custom_base_dir: Option<PathBuf>,
) -> anyhow::Result<(tokio::net::TcpListener, Router, String, LocalEngineState)> {
  if !bind.ip().is_loopback() {
    anyhow::bail!(
      "refusing to bind local web engine on non-loopback address {bind}; \
       use 127.0.0.1:<port> or [::1]:<port> (or `yanshi serve` with YANSHI_API_TOKEN)"
    );
  }
  let base_dir = custom_base_dir.unwrap_or_else(default_studio_runs_dir);
  tokio::fs::create_dir_all(&base_dir).await?;
  let token = generate_loopback_api_token();
  let state = LocalEngineState::new(base_dir).with_api_token(token.clone());
  let app = build_local_router(state.clone());
  let listener = tokio::net::TcpListener::bind(bind).await?;
  Ok((listener, app, token, state))
}

/// Bind the zero-Postgres local engine + embedded SPA on an explicit loopback
/// socket (`127.0.0.1:<port>` or `[::1]:<port>`) with bearer authentication.
pub async fn bind_authenticated_local_web_listener(
  bind: SocketAddr,
  custom_base_dir: Option<PathBuf>,
) -> anyhow::Result<(tokio::net::TcpListener, Router, String)> {
  let (listener, app, token, _state) =
    bind_authenticated_local_web_listener_with_state(bind, custom_base_dir).await?;
  Ok((listener, app, token))
}

/// Bind the zero-Postgres local engine + embedded SPA on an explicit loopback
/// socket (`127.0.0.1:<port>` or `[::1]:<port>`) until Ctrl-C / termination.
///
/// Non-loopback bind addresses are rejected fail-closed because the local
/// engine runs without bearer-token authentication.
pub async fn bind_local_web_listener(
  bind: SocketAddr,
  custom_base_dir: Option<PathBuf>,
) -> anyhow::Result<(tokio::net::TcpListener, Router)> {
  if !bind.ip().is_loopback() {
    anyhow::bail!(
      "refusing to bind unauthenticated local web engine on non-loopback address {bind}; \
       use 127.0.0.1:<port> or [::1]:<port> (or `yanshi serve` with YANSHI_API_TOKEN)"
    );
  }
  let base_dir = custom_base_dir.unwrap_or_else(default_studio_runs_dir);
  tokio::fs::create_dir_all(&base_dir).await?;
  let state = LocalEngineState::new(base_dir);
  let app = build_local_router(state);
  let listener = tokio::net::TcpListener::bind(bind).await?;
  Ok((listener, app))
}

fn constant_time_str_eq(a: &str, b: &str) -> bool {
  let ab = a.as_bytes();
  let bb = b.as_bytes();
  let mut diff = ab.len() ^ bb.len();
  let max_len = ab.len().max(bb.len());
  for idx in 0..max_len {
    let lhs = ab.get(idx).copied().unwrap_or(0);
    let rhs = bb.get(idx).copied().unwrap_or(0);
    diff |= usize::from(lhs ^ rhs);
  }
  diff == 0
}

async fn require_local_api_token(
  State(state): State<LocalEngineState>,
  req: axum::extract::Request,
  next: axum::middleware::Next,
) -> Response {
  let Some(expected) = state.api_token() else {
    return next.run(req).await;
  };
  let provided = req
    .headers()
    .get(header::AUTHORIZATION)
    .and_then(|v| v.to_str().ok())
    .and_then(|raw| {
      raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
    })
    .map(str::trim);
  match provided {
    Some(tok) if constant_time_str_eq(tok, expected) => next.run(req).await,
    _ => (
      StatusCode::UNAUTHORIZED,
      Json(serde_json::json!({
        "error": {
          "code": "unauthorized",
          "message": "missing or invalid bearer token"
        }
      })),
    )
      .into_response(),
  }
}

fn is_trusted_local_origin(origin: &HeaderValue, req_parts: &axum::http::request::Parts) -> bool {
  let Ok(orig_str) = origin.to_str() else {
    return false;
  };
  let trimmed = orig_str.trim().trim_end_matches('/');
  if matches!(
    trimmed,
    "tauri://localhost"
      | "http://tauri.localhost"
      | "https://tauri.localhost"
      | "http://127.0.0.1:5173"
      | "http://localhost:5173"
  ) {
    return true;
  }
  if let Some(host) = req_parts
    .headers
    .get(header::HOST)
    .and_then(|v| v.to_str().ok())
  {
    let same_http = format!("http://{}", host.trim());
    if trimmed.eq_ignore_ascii_case(&same_http) {
      return true;
    }
  }
  false
}

fn build_local_cors_layer() -> CorsLayer {
  use axum::http::Method;
  use tower_http::cors::AllowOrigin;
  CorsLayer::new()
    .allow_origin(AllowOrigin::predicate(is_trusted_local_origin))
    .allow_methods([
      Method::GET,
      Method::POST,
      Method::PUT,
      Method::PATCH,
      Method::DELETE,
      Method::HEAD,
      Method::OPTIONS,
    ])
    .allow_headers([
      header::AUTHORIZATION,
      header::CONTENT_TYPE,
      header::ACCEPT,
      header::HeaderName::from_static("x-yanshi-tenant"),
      header::HeaderName::from_static("last-event-id"),
    ])
}

pub fn build_local_router(state: LocalEngineState) -> Router {
  let harness_router = crate::local_harness::build_harness_router(state.harness.clone());
  let notebooks_router = crate::local_notebooks::build_notebooks_router(state.clone());
  let ui_router = build_embedded_ui_router(state.clone());
  let v1_router = Router::new()
    .route("/v1/diagnostics", get(get_diagnostics))
    .route("/v1/models/config", get(get_models_config))
    .route(
      "/v1/models/providers/:name",
      axum::routing::put(put_model_provider).delete(delete_model_provider),
    )
    .route("/v1/models/providers/:name/test", post(test_model_provider))
    .route(
      "/v1/models/items/:name",
      axum::routing::put(put_model_item).delete(delete_model_item),
    )
    .route("/v1/desktop/pick-directory", post(desktop_pick_directory))
    .route("/v1/desktop/open-artifact", post(desktop_open_artifact))
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
    .route("/v1/runs/:id/questions", get(list_run_questions))
    .route(
      "/v1/runs/:id/questions/:question_id",
      post(answer_run_question),
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
    .with_state(state.clone())
    .merge(harness_router)
    .merge(notebooks_router)
    .route_layer(axum::middleware::from_fn_with_state(
      state,
      require_local_api_token,
    ));

  Router::new()
    .route("/health", get(health))
    .merge(v1_router)
    .merge(ui_router)
    .layer(build_local_cors_layer())
}

fn build_embedded_ui_router(state: LocalEngineState) -> Router {
  Router::new()
    .route("/", get(|| async { Redirect::temporary("/ui/studio") }))
    .route("/ui", get(embedded_index_html))
    .route("/ui/", get(embedded_index_html))
    .route("/ui/studio", get(embedded_index_html))
    .route("/ui/studio/", get(embedded_index_html))
    .route("/ui/notebooks", get(embedded_index_html))
    .route("/ui/notebooks/", get(embedded_index_html))
    .route("/ui/notebooks/:id", get(embedded_index_html))
    .route("/ui/app", get(embedded_index_html))
    .route("/ui/app/", get(embedded_index_html))
    .route("/ui/console", get(embedded_index_html))
    .route("/ui/console/", get(embedded_index_html))
    .route("/ui/runs/new", get(embedded_index_html))
    .route("/ui/runs/:id/compare", get(embedded_index_html))
    .route("/ui/harness/sessions", get(embedded_index_html))
    .route("/ui/harness/sessions/", get(embedded_index_html))
    .route("/ui/harness/sessions/new", get(embedded_index_html))
    .route("/ui/harness/sessions/:id", get(embedded_index_html))
    .route("/ui/diagnostics", get(embedded_index_html))
    .route("/ui/assets/app.js", get(embedded_app_js))
    .route("/ui/assets/styles.css", get(embedded_styles_css))
    .route("/ui-dx", get(serve_dx_ui_index))
    .route("/ui-dx/", get(serve_dx_ui_index))
    .route("/ui-dx/*asset_path", get(serve_dx_ui_asset))
    .with_state(state)
}

const DX_UI_STRICT_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; connect-src 'self' http://127.0.0.1:* http://localhost:*; frame-src 'self' blob:; object-src 'none'; base-uri 'self'";

fn resolve_dx_ui_dir() -> Option<std::path::PathBuf> {
  if let Ok(raw) = std::env::var("YANSHI_DX_UI_DIR") {
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
      let candidate = std::path::PathBuf::from(trimmed);
      if candidate.is_dir() {
        return Some(candidate);
      }
    }
  }
  let candidates = [
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .join("..")
      .join("..")
      .join("yanshi-ui-dx/dist/public"),
    std::path::PathBuf::from("yanshi-ui-dx/dist/public"),
  ];
  candidates.into_iter().find(|p| p.is_dir())
}

fn sanitize_dx_asset_path(raw: &str) -> Option<std::path::PathBuf> {
  let trimmed = raw.trim_start_matches('/');
  if trimmed.is_empty() {
    return Some(std::path::PathBuf::new());
  }
  let mut clean = std::path::PathBuf::new();
  for comp in std::path::Path::new(trimmed).components() {
    match comp {
      std::path::Component::Normal(seg) => clean.push(seg),
      std::path::Component::CurDir => {}
      std::path::Component::ParentDir
      | std::path::Component::RootDir
      | std::path::Component::Prefix(_) => return None,
    }
  }
  Some(clean)
}

fn dx_asset_content_type(path: &std::path::Path) -> &'static str {
  match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
    "html" => "text/html; charset=utf-8",
    "js" | "mjs" => "application/javascript; charset=utf-8",
    "css" => "text/css; charset=utf-8",
    "wasm" => "application/wasm",
    "svg" => "image/svg+xml",
    "png" => "image/png",
    "json" => "application/json; charset=utf-8",
    _ => "application/octet-stream",
  }
}

async fn serve_dx_ui_index() -> Response {
  serve_dx_ui_rel("").await
}

async fn serve_dx_ui_asset(
  axum::extract::Path(asset_path): axum::extract::Path<String>,
) -> Response {
  serve_dx_ui_rel(&asset_path).await
}

async fn serve_dx_ui_rel(raw_rel: &str) -> Response {
  let Some(base_dir) = resolve_dx_ui_dir() else {
    return (
      StatusCode::NOT_FOUND,
      [(header::CONTENT_TYPE, "text/plain; charset=utf-8")],
      "Dioxus UI dist not built yet; run ./yanshi-ui-dx/scripts/build.sh or set YANSHI_DX_UI_DIR.",
    )
      .into_response();
  };
  let Some(rel) = sanitize_dx_asset_path(raw_rel) else {
    return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
  };
  let mut target = if rel.as_os_str().is_empty() {
    base_dir.join("index.html")
  } else {
    base_dir.join(&rel)
  };
  let looks_like_file = rel.extension().is_some();
  if !target.is_file() {
    if looks_like_file {
      return (StatusCode::NOT_FOUND, "asset not found").into_response();
    }
    target = base_dir.join("index.html");
  }
  let Ok(bytes) = tokio::fs::read(&target).await else {
    return (StatusCode::NOT_FOUND, "asset unreadable").into_response();
  };
  let content_type = dx_asset_content_type(&target);
  let cache = if content_type.starts_with("text/html") {
    "no-store"
  } else {
    "public, max-age=300"
  };
  (
    StatusCode::OK,
    [
      (header::CONTENT_TYPE, content_type),
      (header::CACHE_CONTROL, cache),
      (header::CONTENT_SECURITY_POLICY, DX_UI_STRICT_CSP),
    ],
    Body::from(bytes),
  )
    .into_response()
}

async fn embedded_index_html(State(state): State<LocalEngineState>) -> Response {
  let body = match state.api_token() {
    Some(tok) => {
      let escaped = serde_json::to_string(tok).unwrap_or_else(|_| "\"\"".to_string());
      let snippet = format!("<script>window.__YANSHI_API_TOKEN__ = {escaped};</script></head>");
      if EMBEDDED_INDEX_HTML.contains("</head>") {
        EMBEDDED_INDEX_HTML.replacen("</head>", &snippet, 1)
      } else {
        format!("<script>window.__YANSHI_API_TOKEN__ = {escaped};</script>{EMBEDDED_INDEX_HTML}")
      }
    }
    None => EMBEDDED_INDEX_HTML.to_string(),
  };
  (
    StatusCode::OK,
    [(header::CACHE_CONTROL, "no-store")],
    Html(body),
  )
    .into_response()
}

async fn embedded_app_js() -> Response {
  embedded_asset_response(EMBEDDED_APP_JS, "application/javascript; charset=utf-8")
}

async fn embedded_styles_css() -> Response {
  embedded_asset_response(EMBEDDED_STYLES_CSS, "text/css; charset=utf-8")
}

fn embedded_asset_response(body: &'static str, content_type: &'static str) -> Response {
  (
    StatusCode::OK,
    [
      (header::CONTENT_TYPE, content_type),
      (header::CACHE_CONTROL, "no-cache"),
    ],
    Body::from(body),
  )
    .into_response()
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
  offset: Option<usize>,
  status: Option<String>,
}

async fn list_runs(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  Query(query): Query<ListRunsQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let cap = query.limit.unwrap_or(20).clamp(1, 100);
  let offset = query.offset.unwrap_or(0);
  let status_filter = query
    .status
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty());
  if let Some(st) = status_filter
    && !matches!(
      st.to_ascii_lowercase().as_str(),
      "queued" | "running" | "succeeded" | "failed" | "cancelled"
    )
  {
    return error_json(
      StatusCode::BAD_REQUEST,
      &format!(
        "invalid status filter '{st}'; expected queued, running, succeeded, failed, or cancelled"
      ),
    );
  }
  let mut list: Vec<LocalRunRecord> = state
    .lock_runs()
    .values()
    .filter(|e| {
      e.record.tenant_id == tenant
        && status_filter.is_none_or(|st| e.record.status.eq_ignore_ascii_case(st))
    })
    .map(|e| e.record.clone())
    .collect();
  list.sort_by_key(|a| std::cmp::Reverse(a.started_at));
  let paged: Vec<LocalRunRecord> = list.into_iter().skip(offset).take(cap).collect();
  Json(serde_json::json!({ "runs": paged })).into_response()
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
  max_concurrency: Option<usize>,
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
  pub max_concurrency: Option<usize>,
  pub debug_controller: Option<FlowDebugController>,
  pub notebook_id: Option<Uuid>,
  pub notebook_kind: Option<String>,
  pub on_complete: Option<RunCompletionHook>,
}

pub(crate) struct ImportedRunRegistration {
  pub run_id: Uuid,
  pub tenant_id: String,
  pub workflow: String,
  pub run_dir: PathBuf,
  pub notebook_id: Uuid,
  pub notebook_kind: String,
  pub artifacts: Vec<LocalArtifactSummary>,
}

pub(crate) struct CompletedRunForAttach {
  pub record: LocalRunRecord,
  pub artifacts: Vec<LocalArtifactSummary>,
  pub deliverable: Option<serde_json::Value>,
  pub notebook_kind: Option<String>,
}

impl LocalEngineState {
  pub(crate) fn register_imported_run(&self, reg: ImportedRunRegistration) {
    let now = Utc::now();
    let record = LocalRunRecord {
      id: reg.run_id,
      workflow: reg.workflow,
      status: "succeeded".to_string(),
      tenant_id: reg.tenant_id,
      started_at: now,
      finished_at: Some(now),
      run_dir: Some(reg.run_dir.display().to_string()),
      error: None,
    };
    let (tx, _rx) = broadcast::channel(16);
    let entry = RunEntry {
      record,
      events: Vec::new(),
      events_loaded: true,
      artifacts: reg.artifacts,
      tx,
      cancel_token: FlowCancellationToken::new(),
      debug_controller: None,
      attempt_start_seq: 0,
      model: None,
      cost_limit_usd: None,
      notebook_id: Some(reg.notebook_id),
      notebook_kind: Some(reg.notebook_kind),
      on_complete: None,
    };
    let _ = write_run_meta_sync(&reg.run_dir, &entry);
    self.lock_runs().insert(reg.run_id, entry);
  }

  pub(crate) fn get_completed_run_for_attach(
    &self,
    tenant_id: &str,
    run_id: Uuid,
  ) -> Result<CompletedRunForAttach, (StatusCode, String)> {
    self.ensure_run_events_loaded_sync(run_id);
    let (record, artifacts, events, nb_kind) = {
      let guard = self.lock_runs();
      let Some(entry) = guard
        .get(&run_id)
        .filter(|e| e.record.tenant_id == tenant_id)
      else {
        return Err((StatusCode::NOT_FOUND, format!("run {run_id} not found")));
      };
      if entry.record.status != "succeeded" {
        return Err((
          StatusCode::BAD_REQUEST,
          format!(
            "run {run_id} has status '{}'; only succeeded runs can be attached to a notebook",
            entry.record.status
          ),
        ));
      }
      if entry.artifacts.is_empty() {
        return Err((
          StatusCode::BAD_REQUEST,
          format!("run {run_id} produced no downloadable artifacts to attach"),
        ));
      }
      (
        entry.record.clone(),
        entry.artifacts.clone(),
        entry.events.clone(),
        entry.notebook_kind.clone(),
      )
    };

    let mut deliverable = None;
    if let Some(ref dir_str) = record.run_dir {
      let step_path = Path::new(dir_str).join("deliverable_outputs.json");
      if let Ok(bytes) = std::fs::read(&step_path)
        && let Ok(result) =
          serde_json::from_slice::<yanshi_core::async_node::AsyncNodeResult>(&bytes)
      {
        let mut state_map = HashMap::new();
        state_map.insert("deliverable".to_string(), result);
        deliverable = yanshi_config::content::extract_deliverable_from_state(&state_map);
      }
    }
    if deliverable.is_none() {
      for ev in events.iter().rev() {
        if (ev.kind == "node.output.captured" || ev.kind == "node_output_captured")
          && ev.payload.get("node_id").and_then(|v| v.as_str()) == Some("deliverable")
          && let Some(output) = ev.payload.get("output")
          && let Some(unwrapped) = unwrap_local_captured_deliverable(output)
        {
          deliverable = Some(unwrapped);
          break;
        }
      }
    }
    if deliverable.is_none()
      && let Some(ref dir_str) = record.run_dir
    {
      let deliv_dir = Path::new(dir_str).join("artifacts").join("deliverable");
      if let Ok(rd) = std::fs::read_dir(deliv_dir) {
        for item in rd.flatten() {
          let p = item.path();
          if p.extension().and_then(|e| e.to_str()) == Some("json")
            && let Ok(bytes) = std::fs::read(&p)
            && let Ok(val) = serde_json::from_slice::<serde_json::Value>(&bytes)
            && val.get("kind").is_some()
            && val.get("title").is_some()
          {
            deliverable = Some(val);
            break;
          }
        }
      }
    }

    Ok(CompletedRunForAttach {
      record,
      artifacts,
      deliverable,
      notebook_kind: nb_kind,
    })
  }
}

fn unwrap_local_flow_json(val: &serde_json::Value) -> &serde_json::Value {
  if val.get("type").and_then(|t| t.as_str()) == Some("json")
    && let Some(inner) = val.get("value")
  {
    return inner;
  }
  val
}

fn unwrap_local_captured_deliverable(output: &serde_json::Value) -> Option<serde_json::Value> {
  if let Some(raw_deliv) = output.get("deliverable") {
    let candidate = unwrap_local_flow_json(raw_deliv);
    if candidate.get("kind").is_some() && candidate.get("title").is_some() {
      return Some(candidate.clone());
    }
  }
  if let Some(raw_out) = output.get("output") {
    let inner_out = unwrap_local_flow_json(raw_out);
    if let Some(nested) = inner_out.get("deliverable") {
      let candidate = unwrap_local_flow_json(nested);
      if candidate.get("kind").is_some() && candidate.get("title").is_some() {
        return Some(candidate.clone());
      }
    }
    if inner_out.get("kind").is_some() && inner_out.get("title").is_some() {
      return Some(inner_out.clone());
    }
  }
  let direct = unwrap_local_flow_json(output);
  if direct.get("kind").is_some() && direct.get("title").is_some() {
    return Some(direct.clone());
  }
  None
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
      events_loaded: true,
      artifacts: Vec::new(),
      tx,
      cancel_token: cancel_token.clone(),
      debug_controller: params.debug_controller.clone(),
      attempt_start_seq: 0,
      model: params.model.clone(),
      cost_limit_usd: params.cost_limit_usd,
      notebook_id: params.notebook_id,
      notebook_kind: params.notebook_kind,
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
        params.max_concurrency,
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
  if matches!(req.max_concurrency, Some(0)) {
    return error_json(StatusCode::BAD_REQUEST, "max_concurrency must be >= 1");
  }
  let header_tenant = extract_tenant(&headers);
  if let Some(explicit) = req
    .tenant_id
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    && explicit != header_tenant
  {
    return error_json(
      StatusCode::FORBIDDEN,
      &format!(
        "request body tenant_id '{explicit}' does not match X-Yanshi-Tenant '{header_tenant}'"
      ),
    );
  }
  let resolved_tenant = header_tenant;

  let parsed_def = match yanshi_config::executor::parse_workflow_definition(&req.workflow) {
    Ok(def) => def,
    Err(err) => {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!("invalid workflow YAML: {err}"),
      );
    }
  };
  for key in req.inputs.keys() {
    if !parsed_def.inputs.contains_key(key) {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "unknown input '{key}': workflow '{}' declares {:?}",
          parsed_def.name,
          parsed_def.inputs.keys().collect::<Vec<_>>()
        ),
      );
    }
  }

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
      max_concurrency: req.max_concurrency,
      debug_controller,
      notebook_id: None,
      notebook_kind: None,
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
  max_concurrency: Option<usize>,
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

    let concurrency = max_concurrency.unwrap_or(8).clamp(1, 32);
    let mut exec_cfg = FlowExecutionConfig::concurrent(concurrency)
      .with_run_base_dir(base_dir.clone())
      .with_cancellation_token(cancel_token.clone())
      .with_question_handler(Arc::new(LocalRunQuestionHandler::new(
        run_id,
        state.questions.clone(),
        cancel_token.clone(),
      )));
    if let Some(ref m) = model_override {
      exec_cfg = exec_cfg.with_model_override(m.clone());
    }
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
          |recorded| {
            let mut merged = match recorded.cost_limit_usd {
              Some(limit) => exec_cfg.with_cost_limit_usd(limit),
              None => exec_cfg,
            };
            if let Some(saved_model) = recorded.model_override {
              merged = merged.with_model_override(saved_model);
            }
            merged
          },
        )
        .await?
    } else {
      let mut initial_inputs: HashMap<String, FlowValue> = raw_inputs
        .into_iter()
        .map(|(k, v)| (k, FlowValue::Json(v)))
        .collect();
      yanshi_config::executor::seed_chat_model_from_override(
        &flow_def,
        &mut initial_inputs,
        model_override.as_deref(),
      );
      yanshi_config::executor::apply_declared_inputs(&flow_def, &mut initial_inputs)?;
      flow
        .execute_from_inputs_with_id_and_config(run_id.to_string(), initial_inputs, exec_cfg)
        .await?
    };
    Ok(final_state)
  }
  .await;

  state.questions.clear_run(run_id);
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
        let sha256 = tokio::fs::read(&file_out.path).await.ok().map(|bytes| {
          use sha2::{Digest as _, Sha256};
          format!("{:x}", Sha256::digest(&bytes))
        });
        discovered_artifacts.push(LocalArtifactSummary {
          id: Uuid::new_v4(),
          node_id: file_out.node_id,
          name,
          mime_type: file_out
            .mime_type
            .unwrap_or_else(|| infer_mime(&file_out.path).to_string()),
          size_bytes: meta.len(),
          sha256,
          created_at: Utc::now(),
          file_path: file_out.path,
        });
      }
    }
  }

  let preview_status = match &outcome {
    Ok(pool) => {
      if cancel_token.is_cancelled() {
        "cancelled"
      } else if first_pool_error(pool).is_some() {
        "failed"
      } else {
        "succeeded"
      }
    }
    Err(_) => {
      if cancel_token.is_cancelled() {
        "cancelled"
      } else {
        "failed"
      }
    }
  };
  let extracted_deliverable = outcome
    .as_ref()
    .ok()
    .and_then(yanshi_config::content::extract_deliverable_from_state);

  let mut synthetic_to_persist: Option<LocalStreamedEvent> = None;
  let (completion_hook, persisted_nb, tenant_id, final_artifacts, meta_to_persist) = {
    let mut guard = state.lock_runs();
    if let Some(entry) = guard.get_mut(&run_id) {
      entry.debug_controller = None;
      for candidate in discovered_artifacts {
        if let Some(existing) = entry
          .artifacts
          .iter_mut()
          .find(|a| a.file_path == candidate.file_path)
        {
          existing.size_bytes = candidate.size_bytes;
          existing.sha256 = candidate.sha256;
          existing.mime_type = candidate.mime_type;
        } else {
          entry.artifacts.push(candidate);
        }
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
      let meta_snapshot = PersistedRunMeta::from_entry(entry);
      (
        entry.on_complete.clone(),
        entry.notebook_id.zip(entry.notebook_kind.clone()),
        entry.record.tenant_id.clone(),
        entry.artifacts.clone(),
        Some(meta_snapshot),
      )
    } else {
      (None, None, String::new(), Vec::new(), None)
    }
  };

  if let Some(meta) = meta_to_persist {
    let _ = write_persisted_run_meta_async(run_dir.clone(), meta).await;
  }

  if let Some(hook) = completion_hook {
    hook(
      run_id,
      preview_status,
      &final_artifacts,
      extracted_deliverable.as_ref(),
    );
  } else if preview_status == "succeeded"
    && let Some((nb_id, nb_kind)) = persisted_nb
  {
    state.notebooks().attach_run_artifacts(
      &tenant_id,
      nb_id,
      &nb_kind,
      run_id,
      &final_artifacts,
      extracted_deliverable.as_ref(),
    );
  }

  if let Some(ev) = synthetic_to_persist {
    let _ = append_run_event_jsonl_sync(&run_dir, &ev);
  }
}

fn first_pool_error(
  pool: &HashMap<String, yanshi_core::async_node::AsyncNodeResult>,
) -> Option<String> {
  use yanshi_core::FlowStateExt as _;
  pool
    .first_node_failure()
    .map(|(node_id, err)| format!("node '{node_id}' failed: {err}"))
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
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = state.lock_runs();
  match guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) {
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

#[derive(Debug, Deserialize)]
struct AnswerRunQuestionRequest {
  answer: String,
}

async fn list_run_questions(
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
      return error_json(StatusCode::NOT_FOUND, &format!("run {run_id} not found"));
    }
  }
  let pending = state.questions.list(run_id);
  Json(serde_json::json!({ "pending": pending })).into_response()
}

async fn answer_run_question(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((run_id, question_id)): RoutePath<(Uuid, String)>,
  Json(body): Json<AnswerRunQuestionRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_runs();
    if !guard
      .get(&run_id)
      .is_some_and(|entry| entry.record.tenant_id == tenant)
    {
      return error_json(StatusCode::NOT_FOUND, &format!("run {run_id} not found"));
    }
  }
  let trimmed = body.answer.trim();
  if trimmed.is_empty() {
    return error_json(StatusCode::BAD_REQUEST, "answer must not be empty");
  }

  match state
    .questions
    .answer(run_id, &question_id, trimmed.to_string())
  {
    Ok(()) => Json(serde_json::json!({
      "run_id": run_id,
      "question_id": question_id,
      "resolved": true,
    }))
    .into_response(),
    Err(LocalQuestionResolveError::WaiterGone) => error_json(
      StatusCode::BAD_REQUEST,
      &format!("question {question_id} cannot be answered: node future already dropped"),
    ),
    Err(LocalQuestionResolveError::NotFound) => error_json(
      StatusCode::NOT_FOUND,
      &format!("no pending question {question_id} for run {run_id}"),
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
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
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
    let tenant = extract_tenant(&headers);
    let run_dir = state.base_dir.join(run_id.to_string());
    let (record, was_active, synthetic_ev, meta_snapshot) = {
      let mut guard = state.lock_runs();
      let Some(entry) = guard
        .get_mut(&run_id)
        .filter(|e| e.record.tenant_id == tenant)
      else {
        return error_json(StatusCode::NOT_FOUND, "run not found");
      };
      let was_active = !is_terminal_status(&entry.record.status);
      entry.cancel_token.cancel();
      state.questions.clear_run(run_id);
      if let Some(ctrl) = entry.debug_controller.take() {
        ctrl.abort();
      }
      let mut synthetic = None;
      if was_active {
        entry.record.status = "cancelled".to_string();
        entry.record.finished_at = Some(Utc::now());
        synthetic = push_synthetic_terminal_event(entry, "workflow.cancelled", None);
      }
      let meta = PersistedRunMeta::from_entry(entry);
      (entry.record.clone(), was_active, synthetic, meta)
    };
    let _ = write_persisted_run_meta_async(run_dir.clone(), meta_snapshot).await;
    if let Some(ev) = synthetic_ev {
      let _ = append_run_event_jsonl_sync(&run_dir, &ev);
    }
    let mut payload = serde_json::to_value(&record).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = payload.as_object_mut() {
      obj.insert(
        "run".to_string(),
        serde_json::to_value(&record).unwrap_or_default(),
      );
      obj.insert("cancelled".to_string(), serde_json::Value::Bool(was_active));
    }
    return Json(payload).into_response();
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
    state.ensure_run_events_loaded_async(run_id).await;
    let (
      reopened_record,
      workflow_yaml,
      model_override,
      cost_limit_usd,
      cancel_token,
      meta_snapshot,
    ) = {
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
      let meta = PersistedRunMeta::from_entry(entry);
      (
        entry.record.clone(),
        entry.record.workflow.clone(),
        entry.model.clone(),
        entry.cost_limit_usd,
        new_cancel,
        meta,
      )
    };
    let _ = write_persisted_run_meta_async(run_dir, meta_snapshot).await;

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
        None,
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
    let tenant = extract_tenant(&headers);
    let req: DebugControlRequest = match serde_json::from_slice(&body) {
      Ok(parsed) => parsed,
      Err(err) => return error_json(StatusCode::BAD_REQUEST, &format!("invalid JSON: {err}")),
    };
    let guard = state.lock_runs();
    let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    let Some(ctrl) = entry.debug_controller.clone() else {
      return error_json(
        StatusCode::BAD_REQUEST,
        "run does not have an active debug controller",
      );
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalFilterOp {
  Gt,
  Ge,
  Lt,
  Le,
  Eq,
  Ne,
}

#[derive(Debug, Clone)]
enum LocalFilterClause {
  KindEquals(String),
  KindNotEquals(String),
  KindContains(String),
  Step { op: LocalFilterOp, threshold: i64 },
}

fn strip_prefix_ci_local<'a>(input: &'a str, prefix: &str) -> Option<&'a str> {
  let head = input.get(..prefix.len())?;
  if head.eq_ignore_ascii_case(prefix) {
    input.get(prefix.len()..)
  } else {
    None
  }
}

fn split_and_local(input: &str) -> Vec<&str> {
  let lower = input.to_ascii_lowercase();
  let bytes = lower.as_bytes();
  let mut out = Vec::new();
  let mut start = 0usize;
  let mut i = 0usize;
  while i + 5 <= bytes.len() {
    if bytes[i].is_ascii_whitespace()
      && bytes[i + 1] == b'a'
      && bytes[i + 2] == b'n'
      && bytes[i + 3] == b'd'
      && bytes[i + 4].is_ascii_whitespace()
    {
      out.push(&input[start..i]);
      start = i + 5;
      i = start;
      continue;
    }
    i += 1;
  }
  out.push(&input[start..]);
  out
}

fn parse_local_event_filter(raw: Option<&str>) -> Result<Vec<LocalFilterClause>, String> {
  let Some(input) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
    return Ok(Vec::new());
  };
  let mut clauses = Vec::new();
  for piece_raw in split_and_local(input) {
    let piece = piece_raw.trim();
    if piece.is_empty() {
      return Err("empty clause between AND".to_string());
    }
    if let Some(rest) = strip_prefix_ci_local(piece, "kind") {
      let rest = rest.trim_start();
      if let Some(val) = rest.strip_prefix('~') {
        let v = val.trim();
        if v.is_empty() || v.contains(char::is_whitespace) {
          return Err(format!(
            "clause '{piece}': kind~ value must be a non-empty non-whitespace token"
          ));
        }
        clauses.push(LocalFilterClause::KindContains(v.to_ascii_lowercase()));
        continue;
      }
      if let Some(val) = rest.strip_prefix("!=") {
        let v = val.trim();
        if v.is_empty() || v.contains(char::is_whitespace) {
          return Err(format!(
            "clause '{piece}': kind!= value must be a non-empty non-whitespace token"
          ));
        }
        clauses.push(LocalFilterClause::KindNotEquals(v.to_ascii_lowercase()));
        continue;
      }
      if let Some(val) = rest.strip_prefix('=') {
        let v = val.trim();
        if v.is_empty() || v.contains(char::is_whitespace) {
          return Err(format!(
            "clause '{piece}': kind= value must be a non-empty non-whitespace token"
          ));
        }
        clauses.push(LocalFilterClause::KindEquals(v.to_ascii_lowercase()));
        continue;
      }
      return Err(format!(
        "clause '{piece}': 'kind' must be followed by '=', '!=', or '~'"
      ));
    }
    if let Some(rest) = strip_prefix_ci_local(piece, "step") {
      let rest = rest.trim_start();
      let ops = [
        (">=", LocalFilterOp::Ge),
        ("<=", LocalFilterOp::Le),
        ("!=", LocalFilterOp::Ne),
        (">", LocalFilterOp::Gt),
        ("<", LocalFilterOp::Lt),
        ("=", LocalFilterOp::Eq),
      ];
      let mut parsed_step = None;
      for (sym, op) in ops {
        if let Some(num) = rest.strip_prefix(sym) {
          let threshold = num.trim().parse::<i64>().map_err(|e| {
            format!(
              "clause '{piece}': step threshold '{}' is not a valid i64: {e}",
              num.trim()
            )
          })?;
          parsed_step = Some(LocalFilterClause::Step { op, threshold });
          break;
        }
      }
      if let Some(cl) = parsed_step {
        clauses.push(cl);
        continue;
      }
      return Err(format!(
        "clause '{piece}': 'step' must be followed by one of >=, <=, !=, >, <, ="
      ));
    }
    return Err(format!(
      "clause '{piece}' did not match kind=..., kind!=..., kind~..., or step<op>N"
    ));
  }
  Ok(clauses)
}

fn matches_local_event(ev: &LocalStreamedEvent, clauses: &[LocalFilterClause]) -> bool {
  if clauses.is_empty() {
    return true;
  }
  let kind_lower = ev.kind.to_ascii_lowercase();
  for clause in clauses {
    match clause {
      LocalFilterClause::KindEquals(v) if &kind_lower != v => return false,
      LocalFilterClause::KindNotEquals(v) if &kind_lower == v => return false,
      LocalFilterClause::KindContains(v) if !kind_lower.contains(v.as_str()) => return false,
      LocalFilterClause::Step { op, threshold } => {
        let step = ["step_index", "step", "seq"]
          .iter()
          .find_map(|k| ev.payload.get(*k).and_then(|v| v.as_i64()))
          .unwrap_or(ev.seq);
        let ok = match op {
          LocalFilterOp::Gt => step > *threshold,
          LocalFilterOp::Ge => step >= *threshold,
          LocalFilterOp::Lt => step < *threshold,
          LocalFilterOp::Le => step <= *threshold,
          LocalFilterOp::Eq => step == *threshold,
          LocalFilterOp::Ne => step != *threshold,
        };
        if !ok {
          return false;
        }
      }
      _ => {}
    }
  }
  true
}

#[derive(Debug, Deserialize, Default)]
struct EventsQuery {
  after_seq: Option<i64>,
  #[serde(default)]
  filter: Option<String>,
}

async fn events_history(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<EventsQuery>,
) -> Response {
  let clauses = match parse_local_event_filter(query.filter.as_deref()) {
    Ok(c) => c,
    Err(msg) => return error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let tenant = extract_tenant(&headers);
  let after = query.after_seq.unwrap_or(-1);
  state.ensure_run_events_loaded_async(run_id).await;
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
    return error_json(StatusCode::NOT_FOUND, "run not found");
  };
  let filtered: Vec<LocalStreamedEvent> = entry
    .events
    .iter()
    .filter(|e| e.seq > after && matches_local_event(e, &clauses))
    .cloned()
    .collect();
  Json(filtered).into_response()
}

async fn stream_events(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<EventsQuery>,
) -> Response {
  let clauses = match parse_local_event_filter(query.filter.as_deref()) {
    Ok(c) => c,
    Err(msg) => return error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let tenant = extract_tenant(&headers);
  let after = query.after_seq.unwrap_or(-1);
  state.ensure_run_events_loaded_async(run_id).await;
  let (backfill, last_backfill_seq, rx, already_done) = {
    let guard = state.lock_runs();
    let Some(entry) = guard.get(&run_id).filter(|e| e.record.tenant_id == tenant) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    let rx = entry.tx.subscribe();
    let raw_after: Vec<&LocalStreamedEvent> =
      entry.events.iter().filter(|e| e.seq > after).collect();
    let last_seq = raw_after.last().map(|e| e.seq).unwrap_or(after);
    let attempt_start = entry.attempt_start_seq;
    let done = is_terminal_status(&entry.record.status)
      || raw_after
        .iter()
        .any(|e| e.seq >= attempt_start && is_terminal_event_kind(&e.kind));
    let backfill: Vec<LocalStreamedEvent> = raw_after
      .into_iter()
      .filter(|e| matches_local_event(e, &clauses))
      .cloned()
      .collect();
    (backfill, last_seq, rx, done)
  };

  let backfill_stream = futures::stream::iter(backfill);

  let live_stream = if already_done {
    futures::future::Either::Left(futures::stream::empty())
  } else {
    let clauses_live = clauses;
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
      })
      .filter(move |ev| futures::future::ready(matches_local_event(ev, &clauses_live)));
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

#[derive(Debug, Default, Deserialize)]
struct DownloadArtifactQuery {
  #[serde(default)]
  inline: Option<String>,
}

fn is_inline_query_flag(raw: Option<&str>) -> bool {
  matches!(
    raw.map(str::trim).map(str::to_ascii_lowercase).as_deref(),
    Some("1" | "true" | "yes")
  )
}

async fn download_artifact(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((run_id, artifact_id)): RoutePath<(Uuid, Uuid)>,
  Query(query): Query<DownloadArtifactQuery>,
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
  serve_file_with_disposition(
    &artifact.file_path,
    &artifact.mime_type,
    Some(&artifact.name),
    is_inline_query_flag(query.inline.as_deref()),
  )
  .await
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

#[derive(Debug, Default, Deserialize)]
struct DeleteProviderQuery {
  #[serde(default)]
  clear_api_key: Option<bool>,
}

fn desktop_models_config_dir() -> Result<PathBuf, String> {
  yanshi_llm::config::user_store::default_yanshi_config_dir().map_err(|err| err.to_string())
}

async fn get_models_config() -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  match tokio::task::spawn_blocking(move || {
    yanshi_llm::config::user_store::get_model_config_snapshot(&dir)
  })
  .await
  {
    Ok(Ok(snapshot)) => Json(snapshot).into_response(),
    Ok(Err(err)) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &err.to_string()),
    Err(err) => error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("models config task failed: {err}"),
    ),
  }
}

async fn put_model_provider(
  RoutePath(name): RoutePath<String>,
  Json(patch): Json<yanshi_llm::config::user_store::UpsertProviderPatch>,
) -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  match yanshi_llm::config::user_store::upsert_provider_config(&dir, &name, patch, true).await {
    Ok(snapshot) => Json(snapshot).into_response(),
    Err(err) => error_json(StatusCode::BAD_REQUEST, &err.to_string()),
  }
}

async fn delete_model_provider(
  RoutePath(name): RoutePath<String>,
  Query(query): Query<DeleteProviderQuery>,
) -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  let clear_key = query.clear_api_key.unwrap_or(false);
  match yanshi_llm::config::user_store::delete_provider_config(&dir, &name, clear_key, true).await {
    Ok(snapshot) => Json(snapshot).into_response(),
    Err(err) => error_json(StatusCode::BAD_REQUEST, &err.to_string()),
  }
}

async fn put_model_item(
  RoutePath(name): RoutePath<String>,
  Json(patch): Json<yanshi_llm::config::user_store::UpsertModelPatch>,
) -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  match yanshi_llm::config::user_store::upsert_model_config(&dir, &name, patch, true).await {
    Ok(snapshot) => Json(snapshot).into_response(),
    Err(err) => error_json(StatusCode::BAD_REQUEST, &err.to_string()),
  }
}

async fn delete_model_item(RoutePath(name): RoutePath<String>) -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  match yanshi_llm::config::user_store::delete_model_config(&dir, &name, true).await {
    Ok(snapshot) => Json(snapshot).into_response(),
    Err(err) => error_json(StatusCode::BAD_REQUEST, &err.to_string()),
  }
}

async fn test_model_provider(
  RoutePath(name): RoutePath<String>,
  Json(req): Json<yanshi_llm::config::user_store::TestProviderRequest>,
) -> Response {
  let dir = match desktop_models_config_dir() {
    Ok(d) => d,
    Err(msg) => return error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };
  match yanshi_llm::config::user_store::test_provider_connection(&dir, &name, req).await {
    Ok(result) => Json(result).into_response(),
    Err(err) => error_json(StatusCode::BAD_REQUEST, &err.to_string()),
  }
}

#[derive(Debug, Deserialize)]
struct DesktopPickDirectoryRequest {
  #[serde(default)]
  initial_dir: Option<String>,
}

async fn desktop_pick_directory(Json(req): Json<DesktopPickDirectoryRequest>) -> Response {
  let initial = req
    .initial_dir
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(PathBuf::from);
  match tokio::task::spawn_blocking(move || {
    crate::runtime_guard::pick_directory_native(initial.as_deref())
  })
  .await
  {
    Ok(Ok(Some(chosen))) => Json(serde_json::json!({
      "path": chosen.display().to_string(),
      "cancelled": false,
    }))
    .into_response(),
    Ok(Ok(None)) => Json(serde_json::json!({
      "path": null,
      "cancelled": true,
    }))
    .into_response(),
    Ok(Err(msg)) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
    Err(err) => error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("folder picker task failed: {err}"),
    ),
  }
}

#[derive(Debug, Deserialize)]
struct DesktopOpenArtifactRequest {
  api_path: String,
  #[serde(default)]
  mode: Option<String>,
}

fn resolve_run_artifact_api_path(
  state: &LocalEngineState,
  tenant: &str,
  api_path: &str,
) -> Result<PathBuf, (StatusCode, String)> {
  let trimmed = api_path.trim();
  let parsed = url::Url::parse(&format!("http://127.0.0.1{trimmed}"))
    .or_else(|_| url::Url::parse(trimmed))
    .map_err(|_| {
      (
        StatusCode::BAD_REQUEST,
        "invalid artifact api_path".to_string(),
      )
    })?;
  let segments: Vec<&str> = parsed.path().trim_matches('/').split('/').collect();
  if segments.len() < 4 || segments[0] != "v1" || segments[1] != "runs" {
    return Err((
      StatusCode::BAD_REQUEST,
      "api_path must point to a run preview or artifact endpoint".to_string(),
    ));
  }
  let run_id = Uuid::parse_str(segments[2]).map_err(|_| {
    (
      StatusCode::BAD_REQUEST,
      "invalid run UUID in api_path".to_string(),
    )
  })?;

  if segments.len() == 4 && segments[3] == "preview" {
    {
      let guard = state.lock_runs();
      if !guard
        .get(&run_id)
        .is_some_and(|e| e.record.tenant_id == tenant)
      {
        return Err((StatusCode::NOT_FOUND, "run not found".to_string()));
      }
    }
    let rel_path = parsed
      .query_pairs()
      .find(|(k, _)| k == "path")
      .map(|(_, v)| v.into_owned())
      .ok_or_else(|| {
        (
          StatusCode::BAD_REQUEST,
          "preview api_path is missing `path` query parameter".to_string(),
        )
      })?;
    let run_dir = state.base_dir.join(run_id.to_string());
    return resolve_confined_path(&run_dir, &rel_path)
      .map_err(|msg| (StatusCode::BAD_REQUEST, msg));
  }

  if segments.len() == 5 && segments[3] == "artifacts" {
    let artifact_id = Uuid::parse_str(segments[4]).map_err(|_| {
      (
        StatusCode::BAD_REQUEST,
        "invalid artifact UUID in api_path".to_string(),
      )
    })?;
    let found = {
      let guard = state.lock_runs();
      guard
        .get(&run_id)
        .filter(|e| e.record.tenant_id == tenant)
        .and_then(|e| e.artifacts.iter().find(|a| a.id == artifact_id).cloned())
    };
    let Some(artifact) = found else {
      return Err((StatusCode::NOT_FOUND, "artifact not found".to_string()));
    };
    let run_dir = state.base_dir.join(run_id.to_string());
    let canonical_root = std::fs::canonicalize(&run_dir)
      .map_err(|err| (StatusCode::NOT_FOUND, format!("run dir not found: {err}")))?;
    let canonical_file = std::fs::canonicalize(&artifact.file_path).map_err(|err| {
      (
        StatusCode::NOT_FOUND,
        format!("artifact file not found: {err}"),
      )
    })?;
    if !canonical_file.starts_with(&canonical_root) {
      return Err((
        StatusCode::BAD_REQUEST,
        "artifact path escapes run directory".to_string(),
      ));
    }
    return Ok(canonical_file);
  }

  Err((
    StatusCode::BAD_REQUEST,
    "unsupported artifact api_path".to_string(),
  ))
}

async fn desktop_open_artifact(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  Json(req): Json<DesktopOpenArtifactRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let reveal = matches!(
    req.mode.as_deref().map(str::trim),
    Some("reveal" | "folder" | "finder")
  );
  let resolved = match resolve_run_artifact_api_path(&state, &tenant, &req.api_path) {
    Ok(p) => p,
    Err((code, msg)) => return error_json(code, &msg),
  };
  let display = resolved.display().to_string();
  match tokio::task::spawn_blocking(move || {
    crate::runtime_guard::open_or_reveal_path_native(&resolved, reveal)
  })
  .await
  {
    Ok(Ok(())) => Json(serde_json::json!({
      "ok": true,
      "path": display,
      "mode": if reveal { "reveal" } else { "open" },
    }))
    .into_response(),
    Ok(Err(msg)) => error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
    Err(err) => error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("native opener task failed: {err}"),
    ),
  }
}

#[derive(Debug, Deserialize)]
struct PreviewQuery {
  path: String,
}

async fn preview_file(
  State(state): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(run_id): RoutePath<Uuid>,
  Query(query): Query<PreviewQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_runs();
    if !guard
      .get(&run_id)
      .is_some_and(|e| e.record.tenant_id == tenant)
    {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    }
  }
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
  let fallback_name = path.file_name().map(|n| n.to_string_lossy().to_string());
  serve_file_with_disposition(path, mime_type, fallback_name.as_deref(), true).await
}

async fn serve_file_with_disposition(
  path: &Path,
  mime_type: &str,
  download_name: Option<&str>,
  inline: bool,
) -> Response {
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

  let safe_name: String = download_name
    .or_else(|| path.file_name().and_then(|n| n.to_str()))
    .unwrap_or("artifact")
    .chars()
    .filter(|c| !c.is_control() && *c != '"' && *c != '\\' && *c != '/')
    .collect();
  let mode = if inline { "inline" } else { "attachment" };
  let disposition = format!("{mode}; filename=\"{safe_name}\"");
  if let Ok(val) = HeaderValue::from_str(&disposition) {
    res.headers_mut().insert(header::CONTENT_DISPOSITION, val);
  }
  res.headers_mut().insert(
    header::X_CONTENT_TYPE_OPTIONS,
    HeaderValue::from_static("nosniff"),
  );
  let lowered_mime = mime_type.trim().to_ascii_lowercase();
  if lowered_mime.starts_with("text/html") {
    res.headers_mut().insert(
      header::CONTENT_SECURITY_POLICY,
      HeaderValue::from_static("sandbox allow-scripts"),
    );
  } else if lowered_mime.starts_with("image/svg+xml") {
    res.headers_mut().insert(
      header::CONTENT_SECURITY_POLICY,
      HeaderValue::from_static(
        "sandbox; default-src 'none'; img-src data:; style-src 'unsafe-inline'",
      ),
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
    "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    "csv" => "text/csv; charset=utf-8",
    "tsv" => "text/tab-separated-values; charset=utf-8",
    "md" | "txt" | "log" | "yml" | "yaml" => "text/plain; charset=utf-8",
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
  let code = match status {
    StatusCode::BAD_REQUEST => "bad_request",
    StatusCode::UNAUTHORIZED => "unauthorized",
    StatusCode::FORBIDDEN => "forbidden",
    StatusCode::NOT_FOUND => "not_found",
    StatusCode::CONFLICT => "conflict",
    StatusCode::PAYLOAD_TOO_LARGE => "payload_too_large",
    StatusCode::UNPROCESSABLE_ENTITY => "bad_request",
    StatusCode::TOO_MANY_REQUESTS => "too_many_requests",
    _ => "internal_error",
  };
  (
    status,
    Json(serde_json::json!({
      "error": {
        "code": code,
        "message": message,
      }
    })),
  )
    .into_response()
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
    assert_eq!(inspect_res.nodes.len(), 20);
    assert!(
      inspect_res.nodes.iter().any(|n| n.id == "deliverable"),
      "flagship tech_slides_video.yml must expose deliverable node"
    );
    assert!(
      inspect_res
        .nodes
        .iter()
        .any(|n| n.node_type == "illustration"),
      "flagship tech_slides_video.yml must expose illustration node"
    );

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
      export_pptx: true
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
    assert!(
      list.iter().any(|a| {
        a["name"].as_str().is_some_and(|n| n.ends_with(".pptx"))
          && a["mime_type"].as_str()
            == Some("application/vnd.openxmlformats-officedocument.presentationml.presentation")
      }),
      "native OOXML .pptx artifact must be registered on desktop runs"
    );
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

    let wrong_tenant_get = client
      .get(format!("{base_url}/v1/runs/{run_id}"))
      .header("X-Yanshi-Tenant", "intruder")
      .send()
      .await
      .expect("wrong tenant get");
    assert_eq!(wrong_tenant_get.status(), StatusCode::NOT_FOUND);

    let mut status = String::new();
    for _ in 0..80 {
      let run_json: serde_json::Value = client
        .get(format!("{base_url}/v1/runs/{run_id}"))
        .header("X-Yanshi-Tenant", "team-resume")
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
        .header("X-Yanshi-Tenant", "team-resume")
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
      .header("X-Yanshi-Tenant", "team-resume")
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

  #[tokio::test]
  async fn local_engine_serves_embedded_spa_routes_and_rejects_public_bind() {
    let public_addr: SocketAddr = "0.0.0.0:18080".parse().expect("valid socket addr");
    let rejected = bind_local_web_listener(public_addr, None).await;
    assert!(
      rejected.is_err(),
      "non-loopback bind must be rejected fail-closed"
    );

    let temp = tempfile::tempdir().expect("tempdir");
    let loopback_addr: SocketAddr = "127.0.0.1:0".parse().expect("loopback addr");
    let (listener, app) = bind_local_web_listener(loopback_addr, Some(temp.path().to_path_buf()))
      .await
      .expect("bind loopback web listener");
    let bound = listener.local_addr().expect("local addr");
    tokio::spawn(async move {
      let _ = axum::serve(listener, app).await;
    });

    let base_url = format!("http://127.0.0.1:{}", bound.port());
    let client = reqwest::Client::builder()
      .no_proxy()
      .redirect(reqwest::redirect::Policy::none())
      .build()
      .expect("test client");

    let root_res = client
      .get(format!("{base_url}/"))
      .send()
      .await
      .expect("root");
    assert_eq!(root_res.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
      root_res
        .headers()
        .get(header::LOCATION)
        .and_then(|v| v.to_str().ok()),
      Some("/ui/studio")
    );

    for path in ["/ui/studio", "/ui/notebooks", "/ui/notebooks/demo-nb"] {
      let res = client
        .get(format!("{base_url}{path}"))
        .send()
        .await
        .expect("spa route");
      assert_eq!(
        res.status(),
        StatusCode::OK,
        "route {path} should return 200"
      );
      let html = res.text().await.expect("html text");
      assert!(html.contains("yanshi-debugger"));
    }

    let js_res = client
      .get(format!("{base_url}/ui/assets/app.js"))
      .send()
      .await
      .expect("app.js");
    assert_eq!(js_res.status(), StatusCode::OK);
    assert_eq!(
      js_res
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok()),
      Some("application/javascript; charset=utf-8")
    );
  }

  #[tokio::test]
  async fn local_engine_run_questions_enforce_tenant_and_unblock_handler() {
    use yanshi_core::run_context::RunQuestionHandler;

    let temp = tempfile::tempdir().expect("tempdir");
    let state = LocalEngineState::new(temp.path().to_path_buf());
    let questions_reg = state.questions().clone();
    let app = build_local_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let addr = listener.local_addr().expect("addr");
    tokio::spawn(async move {
      let _ = axum::serve(listener, app).await;
    });
    let base_url = format!("http://127.0.0.1:{}", addr.port());
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/runs"))
      .header("X-Yanshi-Tenant", "ops")
      .json(&serde_json::json!({
        "workflow": SAMPLE_YAML,
        "run_mode": "debug",
      }))
      .send()
      .await
      .expect("create run")
      .json()
      .await
      .expect("create json");
    let run_id_str = created["run_id"].as_str().expect("run_id").to_string();
    let run_id = Uuid::parse_str(&run_id_str).expect("valid uuid");

    let handler =
      LocalRunQuestionHandler::new(run_id, questions_reg.clone(), FlowCancellationToken::new());
    let waiter = tokio::spawn(async move {
      handler
        .ask_user("clarify_arch", "Use PostgreSQL or SQLite?")
        .await
    });

    // Wait briefly until the question is parked
    let mut question_id = String::new();
    for _ in 0..40 {
      let parked = questions_reg.list(run_id);
      if let Some(first) = parked.first() {
        question_id = first.question_id.clone();
        break;
      }
      tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(!question_id.is_empty(), "question should be parked");

    // Cross-tenant cannot list or answer
    let cross_list = client
      .get(format!("{base_url}/v1/runs/{run_id}/questions"))
      .header("X-Yanshi-Tenant", "other")
      .send()
      .await
      .expect("cross list");
    assert_eq!(cross_list.status(), StatusCode::NOT_FOUND);

    // Owner tenant sees pending question
    let list_json: serde_json::Value = client
      .get(format!("{base_url}/v1/runs/{run_id}/questions"))
      .header("X-Yanshi-Tenant", "ops")
      .send()
      .await
      .expect("list questions")
      .json()
      .await
      .expect("list json");
    assert_eq!(list_json["pending"].as_array().map(Vec::len), Some(1));
    assert_eq!(list_json["pending"][0]["node_id"], "clarify_arch");

    // Empty answer is rejected with 400
    let empty_ans = client
      .post(format!(
        "{base_url}/v1/runs/{run_id}/questions/{question_id}"
      ))
      .header("X-Yanshi-Tenant", "ops")
      .json(&serde_json::json!({ "answer": "   " }))
      .send()
      .await
      .expect("empty answer");
    assert_eq!(empty_ans.status(), StatusCode::BAD_REQUEST);

    // Valid answer unblocks the parked handler
    let ans_json: serde_json::Value = client
      .post(format!(
        "{base_url}/v1/runs/{run_id}/questions/{question_id}"
      ))
      .header("X-Yanshi-Tenant", "ops")
      .json(&serde_json::json!({ "answer": "PostgreSQL 17 with pgvector" }))
      .send()
      .await
      .expect("answer question")
      .json()
      .await
      .expect("answer json");
    assert_eq!(ans_json["resolved"], true);

    let resolved_text = waiter.await.expect("waiter task").expect("handler answer");
    assert_eq!(resolved_text, "PostgreSQL 17 with pgvector");

    let _ = client
      .post(format!("{base_url}/v1/runs/{run_id}:cancel"))
      .header("X-Yanshi-Tenant", "ops")
      .send()
      .await;
  }

  #[test]
  fn bundled_pricing_table_materializes_and_parses_production_catalog() {
    let temp = tempfile::tempdir().expect("tempdir");
    assert!(should_seed_bundled_pricing_env(None, Some(temp.path())));
    assert!(!should_seed_bundled_pricing_env(
      Some("/custom/pricing.yml"),
      Some(temp.path())
    ));

    let user_cfg_dir = temp.path().join(".yanshi");
    std::fs::create_dir_all(&user_cfg_dir).expect("mkdir .yanshi");
    std::fs::write(user_cfg_dir.join("pricing.yml"), "models: {}\n").expect("write user pricing");
    assert!(
      !should_seed_bundled_pricing_env(None, Some(temp.path())),
      "existing ~/.yanshi/pricing.yml must take precedence over bundled fallback"
    );

    let runs_dir = temp.path().join("studio-runs");
    let path = materialize_bundled_pricing_table(&runs_dir).expect("materialize bundled pricing");
    let table =
      yanshi_llm::pricing::PricingTable::load_from_yaml(&path).expect("parse pricing.prod.yml");
    assert!(table.default.is_some(), "default fallback row expected");
    assert!(
      table.models.len() >= 40,
      "expected full production model pricing catalog, got {}",
      table.models.len()
    );
    assert!(table.models.contains_key("gemini-3.1-flash-image"));
    assert!(table.models.contains_key("qwen3-tts-flash"));
  }

  #[test]
  fn first_pool_error_prefers_upstream_root_cause_over_dependency_cascade() {
    let mut pool = HashMap::new();
    for idx in 0..32 {
      let node_id = format!("downstream_{idx}");
      pool.insert(
        node_id.clone(),
        Err(yanshi_core::error::YanshiError::DependencyNotMet {
          node_id,
          dependency_id: "root_llm".to_string(),
        }),
      );
    }
    pool.insert(
      "skipped_branch".to_string(),
      Err(yanshi_core::error::YanshiError::NodeSkipped),
    );
    pool.insert(
      "root_llm".to_string(),
      Err(yanshi_core::error::YanshiError::NodeExecutionFailed {
        message: "provider quota exhausted".to_string(),
      }),
    );

    let msg = first_pool_error(&pool).expect("must extract error");
    assert!(
      msg.contains("root_llm") && msg.contains("provider quota exhausted"),
      "must report upstream root cause rather than random DependencyNotMet cascade: {msg}"
    );
  }

  #[tokio::test]
  async fn authenticated_loopback_enforces_bearer_token_strict_cors_and_unwraps_deliverable() {
    let temp = tempfile::tempdir().expect("tempdir");
    let (base_url, token) = start_authenticated_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start authenticated engine");
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("test client");

    // 1. Unauthenticated `/v1/runs` is rejected with 401.
    let unauth = client
      .get(format!("{base_url}/v1/runs"))
      .send()
      .await
      .expect("unauth get");
    assert_eq!(unauth.status(), StatusCode::UNAUTHORIZED);

    // 2. Authenticated `/v1/runs` succeeds, while `/ui/studio` embeds the session token.
    let ok = client
      .get(format!("{base_url}/v1/runs"))
      .bearer_auth(&token)
      .send()
      .await
      .expect("auth get");
    assert_eq!(ok.status(), StatusCode::OK);
    let ui_html = client
      .get(format!("{base_url}/ui/studio"))
      .send()
      .await
      .expect("ui get")
      .text()
      .await
      .expect("html text");
    assert!(ui_html.contains("window.__YANSHI_API_TOKEN__"));

    // 3. Evil cross-origin preflight receives NO Access-Control-Allow-Origin, whereas `tauri://localhost` is allowed.
    let evil_preflight = client
      .request(reqwest::Method::OPTIONS, format!("{base_url}/v1/runs"))
      .header("Origin", "https://evil.example")
      .header("Access-Control-Request-Method", "POST")
      .send()
      .await
      .expect("evil preflight");
    assert!(
      evil_preflight
        .headers()
        .get("access-control-allow-origin")
        .is_none()
    );

    let tauri_preflight = client
      .request(reqwest::Method::OPTIONS, format!("{base_url}/v1/runs"))
      .header("Origin", "tauri://localhost")
      .header("Access-Control-Request-Method", "POST")
      .send()
      .await
      .expect("tauri preflight");
    assert_eq!(
      tauri_preflight
        .headers()
        .get("access-control-allow-origin")
        .and_then(|v| v.to_str().ok()),
      Some("tauri://localhost")
    );

    // 4. Tenant mismatch returns 403 and undeclared input returns 400.
    let mismatch = client
      .post(format!("{base_url}/v1/runs"))
      .bearer_auth(&token)
      .json(&serde_json::json!({
        "tenant_id": "acme",
        "workflow": "name: T\nnodes:\n  - id: a\n    type: template\n    parameters:\n      template: ok"
      }))
      .send()
      .await
      .expect("tenant mismatch");
    assert_eq!(mismatch.status(), StatusCode::FORBIDDEN);

    let bad_input = client
      .post(format!("{base_url}/v1/runs"))
      .bearer_auth(&token)
      .json(&serde_json::json!({
        "workflow": "name: T\nnodes:\n  - id: a\n    type: template\n    parameters:\n      template: ok",
        "inputs": { "undeclared_key": 1 }
      }))
      .send()
      .await
      .expect("undeclared input");
    assert_eq!(bad_input.status(), StatusCode::BAD_REQUEST);

    // 5. `FlowValue::Json` envelope in `node.output.captured` is unwrapped cleanly.
    let wrapped = serde_json::json!({
      "deliverable": {
        "type": "json",
        "value": { "kind": "report", "title": "Studio Report", "files": [] }
      }
    });
    let unwrapped = unwrap_local_captured_deliverable(&wrapped).expect("unwrap");
    assert_eq!(unwrapped["kind"], "report");
    assert_eq!(unwrapped["title"], "Studio Report");
  }

  #[test]
  fn cancel_all_active_marks_running_entries_cancelled_and_persists_meta() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let state = LocalEngineState::new(tmp.path().to_path_buf());
    let run_id = Uuid::new_v4();
    let cancel_token = FlowCancellationToken::new();
    let (tx, _rx) = broadcast::channel(8);
    let record = LocalRunRecord {
      id: run_id,
      workflow: "name: shutdown-test".to_string(),
      status: "running".to_string(),
      tenant_id: "default".to_string(),
      started_at: Utc::now(),
      finished_at: None,
      run_dir: Some(tmp.path().join(run_id.to_string()).display().to_string()),
      error: None,
    };
    state.lock_runs().insert(
      run_id,
      RunEntry {
        record,
        events: Vec::new(),
        events_loaded: true,
        artifacts: Vec::new(),
        tx,
        cancel_token: cancel_token.clone(),
        debug_controller: None,
        attempt_start_seq: 0,
        model: None,
        cost_limit_usd: None,
        notebook_id: None,
        notebook_kind: None,
        on_complete: None,
      },
    );
    assert!(state.has_active_work());

    let (runs, sessions) = state.cancel_all_active();
    assert_eq!(runs, 1);
    assert_eq!(sessions, 0);
    assert!(cancel_token.is_cancelled());
    assert!(!state.has_active_work());

    let persisted_bytes =
      std::fs::read(tmp.path().join(run_id.to_string()).join("run_meta.json")).expect("meta file");
    let persisted: PersistedRunMeta = serde_json::from_slice(&persisted_bytes).expect("parse meta");
    assert_eq!(persisted.record.status, "cancelled");
    assert!(persisted.record.finished_at.is_some());
  }

  #[tokio::test]
  async fn hydrated_runs_lazy_load_events_on_demand() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run_id = Uuid::new_v4();
    let run_dir = tmp.path().join(run_id.to_string());
    let meta = PersistedRunMeta {
      record: LocalRunRecord {
        id: run_id,
        workflow: "name: lazy-events".to_string(),
        status: "succeeded".to_string(),
        tenant_id: "default".to_string(),
        started_at: Utc::now(),
        finished_at: Some(Utc::now()),
        run_dir: Some(run_dir.display().to_string()),
        error: None,
      },
      artifacts: Vec::new(),
      model: None,
      cost_limit_usd: None,
      notebook_id: None,
      notebook_kind: None,
    };
    write_persisted_run_meta_sync(&run_dir, &meta).expect("write meta");
    let ev = LocalStreamedEvent {
      run_id,
      seq: 0,
      kind: "workflow.completed".to_string(),
      payload: serde_json::json!({ "ok": true }),
      ts: Utc::now(),
    };
    append_run_event_jsonl_sync(&run_dir, &ev).expect("append event");

    let state = LocalEngineState::new(tmp.path().to_path_buf());
    {
      let guard = state.lock_runs();
      let entry = guard.get(&run_id).expect("hydrated run");
      assert!(!entry.events_loaded, "startup hydration must stay lazy");
      assert!(entry.events.is_empty());
    }

    state.ensure_run_events_loaded_async(run_id).await;
    {
      let guard = state.lock_runs();
      let entry = guard.get(&run_id).expect("hydrated run");
      assert!(entry.events_loaded);
      assert_eq!(entry.events.len(), 1);
      assert_eq!(entry.events[0].kind, "workflow.completed");
    }
  }

  #[test]
  fn desktop_open_artifact_resolver_enforces_tenant_and_run_dir_confinement() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let run_id = Uuid::new_v4();
    let run_dir = tmp.path().join(run_id.to_string());
    let sample = run_dir.join("outputs").join("deck.pptx");
    std::fs::create_dir_all(sample.parent().expect("parent")).expect("mkdir");
    std::fs::write(&sample, b"pptx-bytes").expect("write");

    let state = LocalEngineState::new(tmp.path().to_path_buf());
    let (tx, _rx) = broadcast::channel(8);
    let art_id = Uuid::new_v4();
    state.lock_runs().insert(
      run_id,
      RunEntry {
        record: LocalRunRecord {
          id: run_id,
          workflow: "name: opener-test".to_string(),
          status: "succeeded".to_string(),
          tenant_id: "default".to_string(),
          started_at: Utc::now(),
          finished_at: Some(Utc::now()),
          run_dir: Some(run_dir.display().to_string()),
          error: None,
        },
        events: Vec::new(),
        events_loaded: true,
        artifacts: vec![LocalArtifactSummary {
          id: art_id,
          node_id: "slides".to_string(),
          name: "deck.pptx".to_string(),
          mime_type: "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            .to_string(),
          size_bytes: 10,
          sha256: None,
          created_at: Utc::now(),
          file_path: sample.clone(),
        }],
        tx,
        cancel_token: FlowCancellationToken::new(),
        debug_controller: None,
        attempt_start_seq: 0,
        model: None,
        cost_limit_usd: None,
        notebook_id: None,
        notebook_kind: None,
        on_complete: None,
      },
    );

    // Valid preview path resolves inside run_dir.
    let preview_ok = resolve_run_artifact_api_path(
      &state,
      "default",
      &format!("/v1/runs/{run_id}/preview?path=outputs/deck.pptx"),
    )
    .expect("preview resolves");
    assert_eq!(preview_ok, std::fs::canonicalize(&sample).expect("canon"));

    // Valid artifact ID resolves inside run_dir.
    let art_ok = resolve_run_artifact_api_path(
      &state,
      "default",
      &format!("/v1/runs/{run_id}/artifacts/{art_id}"),
    )
    .expect("artifact resolves");
    assert_eq!(art_ok, std::fs::canonicalize(&sample).expect("canon"));

    // Path traversal is rejected.
    let traversal = resolve_run_artifact_api_path(
      &state,
      "default",
      &format!("/v1/runs/{run_id}/preview?path=../../etc/passwd"),
    );
    assert!(traversal.is_err());

    // Wrong tenant is rejected with 404.
    let wrong_tenant = resolve_run_artifact_api_path(
      &state,
      "other-tenant",
      &format!("/v1/runs/{run_id}/artifacts/{art_id}"),
    );
    assert_eq!(wrong_tenant.unwrap_err().0, StatusCode::NOT_FOUND);
  }

  #[test]
  fn dx_ui_path_sanitizer_and_mime_work() {
    assert_eq!(
      sanitize_dx_asset_path("assets/app.wasm"),
      Some(std::path::PathBuf::from("assets/app.wasm"))
    );
    assert!(sanitize_dx_asset_path("../secret").is_none());
    assert!(sanitize_dx_asset_path("assets/../../etc/passwd").is_none());
    assert_eq!(
      dx_asset_content_type(std::path::Path::new("app.wasm")),
      "application/wasm"
    );
    assert!(DX_UI_STRICT_CSP.contains("'wasm-unsafe-eval'"));
    assert!(!DX_UI_STRICT_CSP.contains("'unsafe-eval'"));
  }
}
