//! Zero-Postgres embedded loopback gateway for the Tauri Desktop app.
//!
//! Speaks the exact `/v1/workflows/inspect`, `/v1/runs`, `/v1/runs/:id/debug`,
//! `/v1/runs/:id/events`, `/v1/runs/:id/artifacts`, and `/v1/runs/:id/preview`
//! contract consumed by `yanshi-ui`, backed directly by `yanshi-config` and
//! `yanshi-core` so desktop users can visually debug or one-click execute
//! workflows offline without installing PostgreSQL.

use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
  Json, Router,
  body::{Body, Bytes},
  extract::{Path as RoutePath, Query, State},
  http::{HeaderValue, StatusCode, header},
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
use yanshi_config::inspect::{InspectWorkflowResponse, inspect_workflow};
use yanshi_core::{
  FlowCancellationToken, FlowDebugController, FlowDebugSnapshot, FlowExecutionConfig, FlowExt,
  events::{EventListener, WorkflowEvent},
  run_context::collect_file_outputs,
  value::FlowValue,
};
use yanshi_llm::Yanshi;

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

struct RunEntry {
  record: LocalRunRecord,
  events: Vec<LocalStreamedEvent>,
  artifacts: Vec<LocalArtifactSummary>,
  tx: broadcast::Sender<LocalStreamedEvent>,
  cancel_token: FlowCancellationToken,
  debug_controller: Option<FlowDebugController>,
}

#[derive(Clone)]
pub struct LocalEngineState {
  runs: Arc<Mutex<HashMap<Uuid, RunEntry>>>,
  base_dir: PathBuf,
}

impl LocalEngineState {
  pub fn new(base_dir: PathBuf) -> Self {
    Self {
      runs: Arc::new(Mutex::new(HashMap::new())),
      base_dir,
    }
  }

  fn lock_runs(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, RunEntry>> {
    match self.runs.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }
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

fn load_user_dotenv() {
  let _ = dotenvy::dotenv();
  if let Some(home) = dirs::home_dir() {
    let user_env = home.join(".yanshi").join(".env");
    if user_env.is_file() {
      let _ = dotenvy::from_path(user_env);
    }
  }
}

async fn ensure_llm_initialized() -> anyhow::Result<()> {
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
  Router::new()
    .route("/health", get(health))
    .route("/v1/workflows/inspect", post(inspect_route))
    .route("/v1/runs", get(list_runs).post(create_run))
    .route("/v1/runs/:id", get(get_run).post(post_run_action))
    .route("/v1/runs/:id/debug", get(get_run_debug))
    .route("/v1/runs/:id/events", get(stream_events))
    .route("/v1/runs/:id/events/history", get(events_history))
    .route("/v1/runs/:id/artifacts", get(list_artifacts))
    .route(
      "/v1/runs/:id/artifacts/:artifact_id",
      get(download_artifact),
    )
    .route("/v1/runs/:id/preview", get(preview_file))
    .layer(CorsLayer::permissive())
    .with_state(state)
}

async fn health() -> impl IntoResponse {
  Json(serde_json::json!({ "status": "ok", "engine": "tauri-local" }))
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
  Query(query): Query<ListRunsQuery>,
) -> impl IntoResponse {
  let cap = query.limit.unwrap_or(20).clamp(1, 100);
  let mut list: Vec<LocalRunRecord> = state
    .lock_runs()
    .values()
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
  run_mode: RunMode,
  #[serde(default)]
  breakpoints: Vec<String>,
  #[serde(default)]
  step_every_node: Option<bool>,
}

async fn create_run(
  State(state): State<LocalEngineState>,
  Json(req): Json<CreateRunRequest>,
) -> Response {
  if req.workflow.trim().is_empty() {
    return error_json(StatusCode::BAD_REQUEST, "workflow YAML cannot be empty");
  }

  let run_id = Uuid::new_v4();
  let run_dir = state.base_dir.join(run_id.to_string());
  if let Err(err) = tokio::fs::create_dir_all(&run_dir).await {
    return error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to create run directory: {err}"),
    );
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

  let cancel_token = FlowCancellationToken::new();
  let (tx, _rx) = broadcast::channel(512);
  let record = LocalRunRecord {
    id: run_id,
    workflow: req.workflow.clone(),
    status: "running".to_string(),
    tenant_id: req.tenant_id.unwrap_or_else(|| "default".to_string()),
    started_at: Utc::now(),
    finished_at: None,
    run_dir: Some(run_dir.display().to_string()),
    error: None,
  };

  state.lock_runs().insert(
    run_id,
    RunEntry {
      record,
      events: Vec::new(),
      artifacts: Vec::new(),
      tx,
      cancel_token: cancel_token.clone(),
      debug_controller: debug_controller.clone(),
    },
  );

  let state_bg = state.clone();
  let base_dir_bg = state.base_dir.clone();
  tokio::spawn(async move {
    execute_local_flow(
      state_bg,
      base_dir_bg,
      run_id,
      req.workflow,
      req.inputs,
      req.model,
      cancel_token,
      debug_controller,
    )
    .await;
  });

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
  state: LocalEngineState,
}

impl EventListener for InMemoryEventListener {
  fn on_event(&self, event: &WorkflowEvent) {
    let kind = event.event_type().to_string();
    let payload = workflow_event_to_json(event);
    let mut guard = self.state.lock_runs();
    if let Some(entry) = guard.get_mut(&self.run_id) {
      let seq = entry.events.len() as i64;
      let streamed = LocalStreamedEvent {
        run_id: self.run_id,
        seq,
        kind,
        payload,
        ts: Utc::now(),
      };
      entry.events.push(streamed.clone());
      let _ = entry.tx.send(streamed);
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
  cancel_token: FlowCancellationToken,
  debug_controller: Option<FlowDebugController>,
) {
  let outcome: anyhow::Result<HashMap<String, yanshi_core::async_node::AsyncNodeResult>> = async {
    let _ = ensure_llm_initialized().await;
    let flow_def = yanshi_config::executor::parse_workflow_definition(&workflow_yaml)?;
    let mut flow =
      yanshi_config::executor::build_flow_from_definition(&flow_def, model_override.as_deref())?;

    let mut initial_inputs: HashMap<String, FlowValue> = raw_inputs
      .into_iter()
      .map(|(k, v)| (k, FlowValue::Json(v)))
      .collect();
    yanshi_config::executor::apply_declared_inputs(&flow_def, &mut initial_inputs)?;

    let listener = InMemoryEventListener {
      run_id,
      state: state.clone(),
    };
    flow = flow.with_event_listener(Arc::new(listener));

    let mut exec_cfg = FlowExecutionConfig::concurrent(8)
      .with_run_base_dir(base_dir.clone())
      .with_cancellation_token(cancel_token.clone());
    if let Some(ctrl) = debug_controller.clone() {
      exec_cfg = exec_cfg.with_debug_controller(ctrl);
    }
    let final_state = flow
      .execute_from_inputs_with_id_and_config(run_id.to_string(), initial_inputs, exec_cfg)
      .await?;
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

  let mut guard = state.lock_runs();
  if let Some(entry) = guard.get_mut(&run_id) {
    entry.debug_controller = None;
    entry.artifacts = discovered_artifacts;
    entry.record.finished_at = Some(Utc::now());
    match outcome {
      Ok(pool) => {
        if cancel_token.is_cancelled() {
          entry.record.status = "cancelled".to_string();
        } else if let Some(node_err) = first_pool_error(&pool) {
          entry.record.status = "failed".to_string();
          entry.record.error = Some(node_err.clone());
          push_synthetic_terminal_event(entry, "workflow.failed", Some(&node_err));
        } else {
          entry.record.status = "succeeded".to_string();
        }
      }
      Err(err) => {
        if cancel_token.is_cancelled() {
          entry.record.status = "cancelled".to_string();
        } else {
          let msg = err.to_string();
          entry.record.status = "failed".to_string();
          entry.record.error = Some(msg.clone());
          push_synthetic_terminal_event(entry, "workflow.failed", Some(&msg));
        }
      }
    }
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

fn push_synthetic_terminal_event(entry: &mut RunEntry, kind: &str, err_msg: Option<&str>) {
  if entry.events.iter().any(|e| is_terminal_event_kind(&e.kind)) {
    return;
  }
  let seq = entry.events.len() as i64;
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
  let _ = entry.tx.send(ev);
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
  Step,
  Continue,
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
  RoutePath(raw_segment): RoutePath<String>,
  body: Bytes,
) -> Response {
  if let Some(raw_id) = raw_segment.strip_suffix(":cancel") {
    let Ok(run_id) = Uuid::parse_str(raw_id) else {
      return error_json(StatusCode::BAD_REQUEST, "invalid run UUID");
    };
    let mut guard = state.lock_runs();
    let Some(entry) = guard.get_mut(&run_id) else {
      return error_json(StatusCode::NOT_FOUND, "run not found");
    };
    entry.cancel_token.cancel();
    if let Some(ctrl) = entry.debug_controller.take() {
      ctrl.abort();
    }
    if !is_terminal_status(&entry.record.status) {
      entry.record.status = "cancelled".to_string();
      entry.record.finished_at = Some(Utc::now());
      push_synthetic_terminal_event(entry, "workflow.cancelled", None);
    }
    return Json(serde_json::json!({
      "run": entry.record,
      "cancelled": true,
    }))
    .into_response();
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
      DebugControlRequest::Step => ctrl.step(),
      DebugControlRequest::Continue => ctrl.continue_run(),
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
    "unsupported run action (expected :cancel or :debug)",
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
    let done = is_terminal_status(&entry.record.status)
      || backfill.iter().any(|e| is_terminal_event_kind(&e.kind));
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
  RoutePath(run_id): RoutePath<Uuid>,
) -> Response {
  let guard = state.lock_runs();
  let Some(entry) = guard.get(&run_id) else {
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
  RoutePath((run_id, artifact_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let found = {
    let guard = state.lock_runs();
    guard
      .get(&run_id)
      .and_then(|e| e.artifacts.iter().find(|a| a.id == artifact_id).cloned())
  };
  let Some(artifact) = found else {
    return error_json(StatusCode::NOT_FOUND, "artifact not found");
  };
  serve_file_inline(&artifact.file_path, &artifact.mime_type).await
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
    other => serde_json::json!({
      "workflow_id": other.workflow_id(),
      "summary": other.to_string(),
    }),
  }
}

fn error_json(status: StatusCode, message: &str) -> Response {
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

    // Continue run to completion
    let cont_res = client
      .post(format!("{base_url}/v1/runs/{run_id}:debug"))
      .json(&serde_json::json!({ "action": "continue" }))
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
  }
}
