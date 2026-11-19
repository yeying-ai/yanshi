//! Zero-Postgres Harness Mode runtime & HTTP API for the Tauri Desktop app.
//!
//! Mirrors the `/v1/harness/sessions/*` contract served by `yanshi-server`,
//! backed directly by `yanshi-harness`, `yanshi-agents`, `yanshi-tools`,
//! `FileLoopCheckpointer`, and local JSON/JSONL persistence under
//! `<base_dir>/_harness/` so desktop users can run governed ReAct /
//! Plan-Execute agents, approve tool calls, and answer `ask_user` interrupts
//! without installing PostgreSQL.

use std::collections::HashMap;
use std::convert::Infallible;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use axum::{
  Json, Router,
  extract::{Path as RoutePath, Query, State},
  http::{HeaderMap, StatusCode},
  response::{
    IntoResponse, Response,
    sse::{Event as SseEvent, KeepAlive, Sse},
  },
  routing::{get, post},
};
use chrono::{DateTime, Utc};
use futures::stream::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::{Semaphore, broadcast, oneshot};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tracing::warn;
use uuid::Uuid;

use yanshi_agent_spi::checkpoint::{AgentLoopCheckpoint, AgentLoopCheckpointer, LoopRuntimeKind};
use yanshi_agents::{
  checkpoint::FileLoopCheckpointer,
  plan_execute::{PlanExecuteAgent, PlanExecuteConfig},
  react::{ReActAgent, ReActConfig},
  runtime::{AgentCancellationToken, AgentRuntime, AgentStopReason, RuntimeLimits},
};
use yanshi_harness::{
  ApprovalDecision, ApprovalOutcome, ApprovalProvider, ApprovalRequest, ApprovalScope,
  HarnessError, HarnessEvent, HarnessEventBody, HarnessEventSink, HarnessProfile,
  HarnessRunOptions, HarnessRuntime, HarnessRuntimeKind, HookConfig, SeqAllocator, SinkChain,
  StopReason, StoppedPayload, default_providers, wrap_registry,
};
use yanshi_memory::{MemoryStore, SessionMemory, SqliteMemory};
use yanshi_skills::{SkillBuilder, SkillLoader};
use yanshi_tools::ToolRegistry;

use crate::local_engine::{ensure_llm_initialized, error_json, extract_tenant};

const DEFAULT_PROFILE: &str = "local";
const DEFAULT_RUNTIME_KIND: &str = "react";
const DEFAULT_MODEL: &str = "kimi-k2.6";
const DEFAULT_MAX_STEPS: usize = 15;
const MAX_STEPS_CEILING: usize = 50;
const MAX_CONCURRENT_SESSIONS: usize = 32;
const DEFAULT_APPROVAL_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalHarnessSession {
  pub id: Uuid,
  pub tenant_id: String,
  pub status: String,
  pub user_input: String,
  pub workspace_root: String,
  pub profile: String,
  pub runtime_kind: String,
  pub model: String,
  pub skill_name: Option<String>,
  pub started_at: DateTime<Utc>,
  pub finished_at: Option<DateTime<Utc>>,
  pub final_answer: Option<String>,
  pub error: Option<String>,
  #[serde(default)]
  pub pending_question: Option<String>,
  #[serde(default)]
  pub pending_question_step_index: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub cost_limit_usd: Option<f64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub max_steps: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalStreamedHarnessEvent {
  pub session_id: Uuid,
  pub seq: i64,
  pub kind: String,
  pub payload: serde_json::Value,
  pub ts: DateTime<Utc>,
}

struct SessionEntry {
  session: LocalHarnessSession,
  events: Vec<LocalStreamedHarnessEvent>,
  tx: broadcast::Sender<LocalStreamedHarnessEvent>,
}

#[derive(Clone)]
pub struct LocalHarnessState {
  sessions: Arc<Mutex<HashMap<Uuid, SessionEntry>>>,
  approvals: LocalApprovalRegistry,
  cancellations: LocalCancellationRegistry,
  concurrency_limit: Arc<Semaphore>,
  harness_dir: PathBuf,
}

impl LocalHarnessState {
  pub fn new(base_dir: PathBuf) -> Self {
    let harness_dir = base_dir.join("_harness");
    let hydrated = hydrate_sessions_from_disk(&harness_dir);
    Self {
      sessions: Arc::new(Mutex::new(hydrated)),
      approvals: LocalApprovalRegistry::default(),
      cancellations: LocalCancellationRegistry::default(),
      concurrency_limit: Arc::new(Semaphore::new(MAX_CONCURRENT_SESSIONS)),
      harness_dir,
    }
  }

  fn lock_sessions(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, SessionEntry>> {
    match self.sessions.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }

  fn checkpoints_dir(&self) -> PathBuf {
    self.harness_dir.join("_loop_checkpoints")
  }

  fn memory_db_path(&self) -> PathBuf {
    self.harness_dir.join("_memory.sqlite")
  }

  fn session_dir(&self, session_id: Uuid) -> PathBuf {
    self.harness_dir.join(session_id.to_string())
  }

  pub(crate) fn approvals(&self) -> &LocalApprovalRegistry {
    &self.approvals
  }
}

fn is_terminal_harness_status(status: &str) -> bool {
  matches!(status, "completed" | "failed" | "cancelled")
}

fn hydrate_sessions_from_disk(harness_dir: &Path) -> HashMap<Uuid, SessionEntry> {
  let mut out = HashMap::new();
  let Ok(read_dir) = std::fs::read_dir(harness_dir) else {
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
    let Ok(session_id) = Uuid::parse_str(dir_name) else {
      continue;
    };
    let session_file = path.join("session.json");
    let Ok(bytes) = std::fs::read(&session_file) else {
      continue;
    };
    let Ok(mut session) = serde_json::from_slice::<LocalHarnessSession>(&bytes) else {
      continue;
    };
    if session.status == "running" {
      session.status = "failed".to_string();
      session.finished_at = Some(Utc::now());
      session.error = Some("desktop_restarted_while_running".to_string());
      let _ = write_session_sync(&path, &session);
    }
    let events = read_events_jsonl(&path.join("events.jsonl"));
    let (tx, _rx) = broadcast::channel(256);
    out.insert(
      session_id,
      SessionEntry {
        session,
        events,
        tx,
      },
    );
  }
  out
}

fn read_events_jsonl(path: &Path) -> Vec<LocalStreamedHarnessEvent> {
  let Ok(text) = std::fs::read_to_string(path) else {
    return Vec::new();
  };
  let mut events: Vec<LocalStreamedHarnessEvent> = text
    .lines()
    .filter(|line| !line.trim().is_empty())
    .filter_map(|line| serde_json::from_str::<LocalStreamedHarnessEvent>(line).ok())
    .collect();
  events.sort_by_key(|ev| ev.seq);
  events
}

fn write_session_sync(session_dir: &Path, session: &LocalHarnessSession) -> std::io::Result<()> {
  std::fs::create_dir_all(session_dir)?;
  let bytes = serde_json::to_vec_pretty(session).map_err(std::io::Error::other)?;
  let tmp = session_dir.join(format!("session.json.tmp.{}", Uuid::new_v4()));
  let dst = session_dir.join("session.json");
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

fn append_event_jsonl_sync(
  session_dir: &Path,
  event: &LocalStreamedHarnessEvent,
) -> std::io::Result<()> {
  std::fs::create_dir_all(session_dir)?;
  let mut file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(session_dir.join("events.jsonl"))?;
  let line = serde_json::to_string(event).map_err(std::io::Error::other)?;
  writeln!(file, "{line}")
}

fn clear_events_jsonl_sync(session_dir: &Path) -> std::io::Result<()> {
  let path = session_dir.join("events.jsonl");
  match std::fs::remove_file(path) {
    Ok(()) => Ok(()),
    Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
    Err(err) => Err(err),
  }
}

// ── Cancellation Registry ────────────────────────────────────────────

#[derive(Clone, Default)]
struct LocalCancellationRegistry {
  inner: Arc<Mutex<HashMap<Uuid, (u64, AgentCancellationToken)>>>,
  next_generation: Arc<AtomicU64>,
}

impl LocalCancellationRegistry {
  fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, (u64, AgentCancellationToken)>> {
    match self.inner.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }

  fn register(&self, id: Uuid) -> CancellationGuard {
    let generation = self.next_generation.fetch_add(1, Ordering::Relaxed);
    let token = AgentCancellationToken::new();
    self.lock().insert(id, (generation, token.clone()));
    CancellationGuard {
      registry: self.clone(),
      id,
      generation,
      token,
    }
  }

  fn cancel(&self, id: Uuid) -> bool {
    match self.lock().get(&id) {
      Some((_, token)) => {
        token.cancel();
        true
      }
      None => false,
    }
  }
}

struct CancellationGuard {
  registry: LocalCancellationRegistry,
  id: Uuid,
  generation: u64,
  token: AgentCancellationToken,
}

impl CancellationGuard {
  fn token(&self) -> AgentCancellationToken {
    self.token.clone()
  }
}

impl Drop for CancellationGuard {
  fn drop(&mut self) {
    let mut map = self.registry.lock();
    if map
      .get(&self.id)
      .is_some_and(|(gen_id, _)| *gen_id == self.generation)
    {
      map.remove(&self.id);
    }
  }
}

// ── Approval Registry & Provider ─────────────────────────────────────

struct ParkedApproval {
  request: ApprovalRequest,
  responder: oneshot::Sender<ApprovalDecision>,
}

#[derive(Clone, Default)]
pub(crate) struct LocalApprovalRegistry {
  entries: Arc<Mutex<HashMap<(String, String), ParkedApproval>>>,
}

#[derive(Debug)]
pub(crate) enum LocalApprovalResolveError {
  NotFound,
  ProviderGone,
}

impl LocalApprovalRegistry {
  fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), ParkedApproval>> {
    match self.entries.lock() {
      Ok(guard) => guard,
      Err(poisoned) => poisoned.into_inner(),
    }
  }

  pub(crate) fn park(&self, request: ApprovalRequest) -> oneshot::Receiver<ApprovalDecision> {
    let (tx, rx) = oneshot::channel();
    let key = (request.session_id.clone(), request.id.clone());
    self.lock().insert(
      key,
      ParkedApproval {
        request,
        responder: tx,
      },
    );
    rx
  }

  fn drop_pending(&self, session_id: &str, request_id: &str) {
    self
      .lock()
      .remove(&(session_id.to_string(), request_id.to_string()));
  }

  pub(crate) fn list(&self, session_id: &str) -> Vec<ApprovalRequest> {
    let mut items: Vec<ApprovalRequest> = self
      .lock()
      .iter()
      .filter(|((sess, _), _)| sess == session_id)
      .map(|(_, entry)| entry.request.clone())
      .collect();
    items.sort_by_key(|req| req.requested_at);
    items
  }

  pub(crate) fn decide(
    &self,
    session_id: &str,
    request_id: &str,
    decision: ApprovalDecision,
  ) -> Result<(), LocalApprovalResolveError> {
    let Some(entry) = self
      .lock()
      .remove(&(session_id.to_string(), request_id.to_string()))
    else {
      return Err(LocalApprovalResolveError::NotFound);
    };
    entry
      .responder
      .send(decision)
      .map_err(|_| LocalApprovalResolveError::ProviderGone)
  }
}

struct LocalApprovalProvider {
  registry: LocalApprovalRegistry,
  default_timeout: Duration,
}

impl LocalApprovalProvider {
  fn new(registry: LocalApprovalRegistry, default_timeout: Duration) -> Self {
    Self {
      registry,
      default_timeout,
    }
  }
}

#[async_trait]
impl ApprovalProvider for LocalApprovalProvider {
  fn name(&self) -> &str {
    "tauri-local"
  }

  async fn request(&self, request: ApprovalRequest) -> Result<ApprovalDecision, HarnessError> {
    let session_id = request.session_id.clone();
    let request_id = request.id.clone();
    let deadline = request
      .expires_at
      .map(|exp| (exp - Utc::now()).to_std().unwrap_or_default())
      .filter(|d| !d.is_zero())
      .unwrap_or(self.default_timeout);

    let rx = self.registry.park(request);
    match tokio::time::timeout(deadline, rx).await {
      Ok(Ok(decision)) => Ok(decision),
      Ok(Err(_dropped)) => {
        self.registry.drop_pending(&session_id, &request_id);
        Ok(ApprovalDecision {
          request_id,
          decision: ApprovalOutcome::DenyAndStop,
          scope: ApprovalScope::Once,
          decided_by: "desktop:dropped".to_string(),
          decided_at: Utc::now(),
          reason: Some("approval channel closed before decision".to_string()),
        })
      }
      Err(_timeout) => {
        self.registry.drop_pending(&session_id, &request_id);
        Err(HarnessError::ApprovalTimeout {
          timeout_ms: deadline.as_millis() as u64,
        })
      }
    }
  }
}

// ── Event Sink ───────────────────────────────────────────────────────

struct LocalHarnessEventSink {
  state: LocalHarnessState,
}

impl LocalHarnessEventSink {
  fn new(state: LocalHarnessState) -> Self {
    Self { state }
  }
}

fn harness_event_kind(body: &HarnessEventBody) -> &'static str {
  match body {
    HarnessEventBody::SessionStarted(_) => "session_started",
    HarnessEventBody::StepStarted(_) => "step_started",
    HarnessEventBody::ToolCallRequested(_) => "tool_call_requested",
    HarnessEventBody::ApprovalRequested(_) => "approval_requested",
    HarnessEventBody::ApprovalDecided(_) => "approval_decided",
    HarnessEventBody::ToolCallCompleted(_) => "tool_call_completed",
    HarnessEventBody::TokenDelta(_) => "token_delta",
    HarnessEventBody::InterruptRequested(_) => "interrupt_requested",
    HarnessEventBody::InterruptAnswered(_) => "interrupt_answered",
    HarnessEventBody::BackgroundTaskUpdated(_) => "background_task_updated",
    HarnessEventBody::MemorySummaryAdded(_) => "memory_summary_added",
    HarnessEventBody::Stopped(_) => "stopped",
  }
}

#[async_trait]
impl HarnessEventSink for LocalHarnessEventSink {
  fn name(&self) -> &str {
    "tauri-local"
  }

  async fn write(&self, event: &HarnessEvent) -> Result<(), HarnessError> {
    let Ok(session_id) = Uuid::parse_str(&event.session_id) else {
      return Ok(());
    };
    let kind = harness_event_kind(&event.body);
    let payload = serde_json::to_value(&event.body).unwrap_or(serde_json::Value::Null);
    let streamed = LocalStreamedHarnessEvent {
      session_id,
      seq: event.seq as i64,
      kind: kind.to_string(),
      payload,
      ts: event.ts,
    };

    // V2.2 contract: `token_delta` is live-only (never persisted, never in `/events/history`).
    let persist = kind != "token_delta";
    let session_dir = self.state.session_dir(session_id);
    {
      let mut guard = self.state.lock_sessions();
      if let Some(entry) = guard.get_mut(&session_id) {
        if persist {
          entry.events.push(streamed.clone());
        }
        let _ = entry.tx.send(streamed.clone());
      }
    }
    if persist && let Err(err) = append_event_jsonl_sync(&session_dir, &streamed) {
      warn!(session_id = %session_id, error = %err, "failed to persist harness event");
    }
    Ok(())
  }
}

// ── Router & HTTP Handlers ───────────────────────────────────────────

pub fn build_harness_router(state: LocalHarnessState) -> Router {
  Router::new()
    .route(
      "/v1/harness/sessions",
      get(list_harness_sessions).post(submit_harness_session),
    )
    .route(
      "/v1/harness/sessions/:id",
      get(get_harness_session).post(post_harness_session_action),
    )
    .route(
      "/v1/harness/sessions/:id/events",
      get(stream_harness_events),
    )
    .route(
      "/v1/harness/sessions/:id/events/history",
      get(list_harness_events),
    )
    .route(
      "/v1/harness/sessions/:id/approvals",
      get(list_pending_approvals),
    )
    .route(
      "/v1/harness/sessions/:id/approvals/:request_id",
      post(decide_approval),
    )
    .route(
      "/v1/harness/sessions/:id/interrupt",
      get(get_pending_interrupt),
    )
    .route(
      "/v1/harness/sessions/:id/interrupt/answer",
      post(answer_interrupt),
    )
    .with_state(state)
}

#[derive(Debug, Deserialize)]
struct CreateHarnessSessionRequest {
  #[serde(default)]
  tenant_id: Option<String>,
  user_input: String,
  workspace_root: String,
  #[serde(default)]
  profile: Option<String>,
  #[serde(default)]
  runtime_kind: Option<String>,
  #[serde(default)]
  model: Option<String>,
  #[serde(default)]
  skill_name: Option<String>,
  #[serde(default)]
  cost_limit_usd: Option<f64>,
  #[serde(default)]
  max_steps: Option<usize>,
}

fn validate_workspace_root(workspace_root: &str) -> Result<(), String> {
  if workspace_root.contains('\0') {
    return Err("`workspace_root` must not contain NUL bytes".to_string());
  }
  let path = Path::new(workspace_root);
  if path == Path::new("/") {
    return Err("`workspace_root` cannot be `/` (filesystem root)".to_string());
  }
  Ok(())
}

async fn submit_harness_session(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  Json(req): Json<CreateHarnessSessionRequest>,
) -> Response {
  let user_input = req.user_input.trim();
  if user_input.is_empty() {
    return error_json(
      StatusCode::BAD_REQUEST,
      "`user_input` must be a non-empty prompt",
    );
  }
  let workspace_root = req.workspace_root.trim();
  if workspace_root.is_empty() {
    return error_json(
      StatusCode::BAD_REQUEST,
      "`workspace_root` must be a non-empty path",
    );
  }
  if let Err(reason) = validate_workspace_root(workspace_root) {
    return error_json(StatusCode::BAD_REQUEST, &reason);
  }

  let tenant_id = extract_tenant(&headers);
  if let Some(body_tenant) = req
    .tenant_id
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    && body_tenant != tenant_id
  {
    return error_json(
      StatusCode::FORBIDDEN,
      &format!(
        "request body tenant_id '{body_tenant}' does not match X-Yanshi-Tenant '{tenant_id}'"
      ),
    );
  }

  let profile = req
    .profile
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or(DEFAULT_PROFILE)
    .to_string();
  let runtime_kind = req
    .runtime_kind
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or(DEFAULT_RUNTIME_KIND)
    .to_string();
  if !matches!(runtime_kind.as_str(), "react" | "plan_execute") {
    return error_json(
      StatusCode::BAD_REQUEST,
      &format!(
        "runtime_kind '{runtime_kind}' is not supported (expected 'react' or 'plan_execute')"
      ),
    );
  }
  let skill_name = req
    .skill_name
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_string);
  if runtime_kind == "plan_execute" && skill_name.is_some() {
    return error_json(
      StatusCode::BAD_REQUEST,
      "skill-backed sessions run ReAct; 'plan_execute' cannot be combined with skill_name",
    );
  }
  let model = req
    .model
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or(DEFAULT_MODEL)
    .to_string();

  let session_id = Uuid::new_v4();
  let session = LocalHarnessSession {
    id: session_id,
    tenant_id,
    status: "running".to_string(),
    user_input: user_input.to_string(),
    workspace_root: workspace_root.to_string(),
    profile: profile.clone(),
    runtime_kind: runtime_kind.clone(),
    model: model.clone(),
    skill_name: skill_name.clone(),
    started_at: Utc::now(),
    finished_at: None,
    final_answer: None,
    error: None,
    pending_question: None,
    pending_question_step_index: None,
    cost_limit_usd: req.cost_limit_usd,
    max_steps: req.max_steps,
  };

  let session_dir = state.session_dir(session_id);
  if let Err(err) = write_session_sync(&session_dir, &session) {
    return error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist harness session: {err}"),
    );
  }

  let (tx, _rx) = broadcast::channel(256);
  state.lock_sessions().insert(
    session_id,
    SessionEntry {
      session,
      events: Vec::new(),
      tx,
    },
  );

  let skill_dir = skill_name
    .as_deref()
    .and_then(|name| resolve_skill_dir(workspace_root, name));
  let cancel_guard = state.cancellations.register(session_id);
  let state_bg = state.clone();
  let inputs = SessionRunInputs {
    session_id,
    user_input: user_input.to_string(),
    workspace_root: workspace_root.to_string(),
    profile,
    runtime_kind,
    model,
    skill_name,
    skill_dir,
    cost_limit_usd: req.cost_limit_usd,
    max_steps: req.max_steps,
    initial_seq: 0,
    cancellation_token: cancel_guard.token(),
  };
  tokio::spawn(async move {
    let _guard = cancel_guard;
    execute_session(state_bg, inputs).await;
  });

  (
    StatusCode::OK,
    Json(serde_json::json!({
      "session_id": session_id,
      "status": "running",
    })),
  )
    .into_response()
}

#[derive(Debug, Deserialize, Default)]
struct ListHarnessSessionsQuery {
  limit: Option<usize>,
}

async fn list_harness_sessions(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  Query(query): Query<ListHarnessSessionsQuery>,
) -> impl IntoResponse {
  let tenant = extract_tenant(&headers);
  let limit = query.limit.unwrap_or(25).clamp(1, 100);
  let mut sessions: Vec<LocalHarnessSession> = state
    .lock_sessions()
    .values()
    .filter(|entry| entry.session.tenant_id == tenant)
    .map(|entry| entry.session.clone())
    .collect();
  sessions.sort_by_key(|s| std::cmp::Reverse(s.started_at));
  sessions.truncate(limit);
  Json(serde_json::json!({ "sessions": sessions }))
}

async fn get_harness_session(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = state.lock_sessions();
  let Some(entry) = guard.get(&id) else {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {id} not found"),
    );
  };
  if entry.session.tenant_id != tenant {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {id} not found"),
    );
  }
  (StatusCode::OK, Json(entry.session.clone())).into_response()
}

#[derive(Debug, Deserialize, Serialize, Default, Clone, Copy, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum ResumeMode {
  #[default]
  Rerun,
  Append,
}

#[derive(Debug, Deserialize, Default)]
struct ResumeHarnessSessionRequest {
  #[serde(default)]
  user_input: Option<String>,
  #[serde(default)]
  mode: ResumeMode,
}

#[derive(Debug, Serialize)]
struct CancelHarnessSessionResponse {
  #[serde(flatten)]
  session: LocalHarnessSession,
  cancelled: bool,
}

#[derive(Debug, Serialize)]
struct ResumeHarnessSessionResponse {
  #[serde(flatten)]
  session: LocalHarnessSession,
  resumed: bool,
  mode: ResumeMode,
}

async fn post_harness_session_action(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(id_action): RoutePath<String>,
  body: Option<Json<ResumeHarnessSessionRequest>>,
) -> Response {
  if let Some(raw_id) = id_action.strip_suffix(":cancel") {
    return cancel_harness_session(state, headers, raw_id).await;
  }
  if let Some(raw_id) = id_action.strip_suffix(":resume") {
    let req = body.map(|Json(v)| v).unwrap_or_default();
    return resume_harness_session(state, headers, raw_id, req).await;
  }
  error_json(
    StatusCode::BAD_REQUEST,
    "harness session action route must end with :cancel or :resume",
  )
}

async fn cancel_harness_session(
  state: LocalHarnessState,
  headers: HeaderMap,
  raw_id: &str,
) -> Response {
  let Ok(id) = Uuid::parse_str(raw_id) else {
    return error_json(
      StatusCode::BAD_REQUEST,
      &format!("invalid harness session id '{raw_id}'"),
    );
  };
  let tenant = extract_tenant(&headers);

  let (already_terminal, snapshot, cancel_event) = {
    let mut guard = state.lock_sessions();
    let Some(entry) = guard.get_mut(&id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {id} not found"),
      );
    }
    if is_terminal_harness_status(&entry.session.status) {
      (true, entry.session.clone(), None)
    } else {
      entry.session.status = "cancelled".to_string();
      entry.session.finished_at = Some(Utc::now());
      entry.session.error = Some("cancel requested".to_string());
      entry.session.pending_question = None;
      entry.session.pending_question_step_index = None;

      let next_seq = entry
        .events
        .last()
        .map(|ev| ev.seq.saturating_add(1))
        .unwrap_or(0);
      let stopped = LocalStreamedHarnessEvent {
        session_id: id,
        seq: next_seq,
        kind: "stopped".to_string(),
        payload: serde_json::json!({
          "reason": "cancelled",
          "source": "operator",
        }),
        ts: Utc::now(),
      };
      entry.events.push(stopped.clone());
      let _ = entry.tx.send(stopped.clone());
      (false, entry.session.clone(), Some(stopped))
    }
  };

  if already_terminal {
    return (
      StatusCode::OK,
      Json(CancelHarnessSessionResponse {
        session: snapshot,
        cancelled: false,
      }),
    )
      .into_response();
  }

  state.cancellations.cancel(id);
  let session_dir = state.session_dir(id);
  let _ = write_session_sync(&session_dir, &snapshot);
  if let Some(ev) = &cancel_event {
    let _ = append_event_jsonl_sync(&session_dir, ev);
  }

  (
    StatusCode::OK,
    Json(CancelHarnessSessionResponse {
      session: snapshot,
      cancelled: true,
    }),
  )
    .into_response()
}

async fn resume_harness_session(
  state: LocalHarnessState,
  headers: HeaderMap,
  raw_id: &str,
  body: ResumeHarnessSessionRequest,
) -> Response {
  let Ok(id) = Uuid::parse_str(raw_id) else {
    return error_json(
      StatusCode::BAD_REQUEST,
      &format!("invalid harness session id '{raw_id}'"),
    );
  };
  let tenant = extract_tenant(&headers);

  let (updated_session, initial_seq) = {
    let mut guard = state.lock_sessions();
    let Some(entry) = guard.get_mut(&id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {id} not found"),
      );
    }
    if !is_terminal_harness_status(&entry.session.status) {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "harness session {id} is still {}; cancel before resuming",
          entry.session.status
        ),
      );
    }

    let next_prompt = body
      .user_input
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .map(str::to_string)
      .unwrap_or_else(|| entry.session.user_input.clone());

    let seq_offset = match body.mode {
      ResumeMode::Rerun => {
        entry.events.clear();
        0
      }
      ResumeMode::Append => entry
        .events
        .last()
        .map(|ev| (ev.seq.max(0) as u64).saturating_add(1))
        .unwrap_or(0),
    };

    entry.session.user_input = next_prompt;
    entry.session.status = "running".to_string();
    entry.session.finished_at = None;
    entry.session.final_answer = None;
    entry.session.error = None;
    entry.session.pending_question = None;
    entry.session.pending_question_step_index = None;

    (entry.session.clone(), seq_offset)
  };

  let session_dir = state.session_dir(id);
  if body.mode == ResumeMode::Rerun {
    let _ = clear_events_jsonl_sync(&session_dir);
  }
  let _ = write_session_sync(&session_dir, &updated_session);

  let skill_dir = updated_session
    .skill_name
    .as_deref()
    .and_then(|name| resolve_skill_dir(&updated_session.workspace_root, name));
  let cancel_guard = state.cancellations.register(id);
  let state_bg = state.clone();
  let inputs = SessionRunInputs {
    session_id: id,
    user_input: updated_session.user_input.clone(),
    workspace_root: updated_session.workspace_root.clone(),
    profile: updated_session.profile.clone(),
    runtime_kind: updated_session.runtime_kind.clone(),
    model: updated_session.model.clone(),
    skill_name: updated_session.skill_name.clone(),
    skill_dir,
    cost_limit_usd: updated_session.cost_limit_usd,
    max_steps: updated_session.max_steps,
    initial_seq,
    cancellation_token: cancel_guard.token(),
  };
  tokio::spawn(async move {
    let _guard = cancel_guard;
    execute_session(state_bg, inputs).await;
  });

  (
    StatusCode::OK,
    Json(ResumeHarnessSessionResponse {
      session: updated_session,
      resumed: true,
      mode: body.mode,
    }),
  )
    .into_response()
}

#[derive(Debug, Deserialize, Default)]
struct HarnessEventsQuery {
  after_seq: Option<i64>,
}

async fn list_harness_events(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(session_id): RoutePath<Uuid>,
  Query(query): Query<HarnessEventsQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let after_seq = query.after_seq.unwrap_or(-1);
  let guard = state.lock_sessions();
  let Some(entry) = guard.get(&session_id) else {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {session_id} not found"),
    );
  };
  if entry.session.tenant_id != tenant {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {session_id} not found"),
    );
  }
  let filtered: Vec<LocalStreamedHarnessEvent> = entry
    .events
    .iter()
    .filter(|ev| ev.seq > after_seq)
    .take(1_000)
    .cloned()
    .collect();
  (StatusCode::OK, Json(filtered)).into_response()
}

async fn stream_harness_events(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(session_id): RoutePath<Uuid>,
  Query(query): Query<HarnessEventsQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let after_seq = query.after_seq.unwrap_or(-1);
  let (backfill, rx, already_terminal) = {
    let guard = state.lock_sessions();
    let Some(entry) = guard.get(&session_id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    }
    let rx = entry.tx.subscribe();
    let backfill: Vec<LocalStreamedHarnessEvent> = entry
      .events
      .iter()
      .filter(|ev| ev.seq > after_seq)
      .cloned()
      .collect();
    let terminal = is_terminal_harness_status(&entry.session.status);
    (backfill, rx, terminal)
  };

  let max_backfill_seq = backfill.last().map(|ev| ev.seq).unwrap_or(after_seq);
  let backfill_stream = futures::stream::iter(backfill).map(sse_from_harness_event);

  if already_terminal {
    return Sse::new(backfill_stream.map(Ok::<_, Infallible>))
      .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
      .into_response();
  }

  let live_stream = BroadcastStream::new(rx)
    .filter_map(move |res| {
      futures::future::ready(match res {
        Ok(ev) if ev.kind == "token_delta" || ev.seq > max_backfill_seq => Some(ev),
        Ok(_) => None,
        Err(BroadcastStreamRecvError::Lagged(_)) => None,
      })
    })
    .scan(false, |done, ev| {
      if *done {
        return futures::future::ready(None);
      }
      if ev.kind == "stopped" {
        *done = true;
      }
      futures::future::ready(Some(sse_from_harness_event(ev)))
    });

  Sse::new(backfill_stream.chain(live_stream).map(Ok::<_, Infallible>))
    .keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
    .into_response()
}

fn sse_from_harness_event(event: LocalStreamedHarnessEvent) -> SseEvent {
  let data = serde_json::to_string(&event).unwrap_or_else(|_| "{}".to_string());
  SseEvent::default()
    .id(event.seq.to_string())
    .event(event.kind)
    .data(data)
}

async fn list_pending_approvals(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(session_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_sessions();
    let Some(entry) = guard.get(&session_id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    }
  }
  let approvals = state.approvals.list(&session_id.to_string());
  (
    StatusCode::OK,
    Json(serde_json::json!({ "approvals": approvals })),
  )
    .into_response()
}

#[derive(Debug, Deserialize)]
struct ApprovalDecisionRequest {
  decision: ApprovalOutcome,
  #[serde(default)]
  scope: Option<ApprovalScope>,
  #[serde(default)]
  decided_by: Option<String>,
  #[serde(default)]
  reason: Option<String>,
}

async fn decide_approval(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath((session_id, request_id)): RoutePath<(Uuid, String)>,
  Json(body): Json<ApprovalDecisionRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = state.lock_sessions();
    let Some(entry) = guard.get(&session_id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
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
    .approvals
    .decide(&session_id.to_string(), &request_id, decision)
  {
    Ok(()) => (
      StatusCode::OK,
      Json(serde_json::json!({
        "session_id": session_id,
        "request_id": request_id,
        "resolved": true,
      })),
    )
      .into_response(),
    Err(LocalApprovalResolveError::NotFound) => error_json(
      StatusCode::NOT_FOUND,
      &format!("pending approval '{request_id}' not found for session {session_id}"),
    ),
    Err(LocalApprovalResolveError::ProviderGone) => error_json(
      StatusCode::CONFLICT,
      "approval provider finished before decision arrived",
    ),
  }
}

async fn get_pending_interrupt(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(session_id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = state.lock_sessions();
  let Some(entry) = guard.get(&session_id) else {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {session_id} not found"),
    );
  };
  if entry.session.tenant_id != tenant {
    return error_json(
      StatusCode::NOT_FOUND,
      &format!("harness session {session_id} not found"),
    );
  }
  let pending = entry.session.pending_question.as_ref().map(|question| {
    serde_json::json!({
      "question": question,
      "step_index": entry.session.pending_question_step_index.unwrap_or(0),
    })
  });
  (
    StatusCode::OK,
    Json(serde_json::json!({ "pending": pending })),
  )
    .into_response()
}

#[derive(Debug, Deserialize)]
struct AnswerInterruptRequest {
  answer: String,
}

#[derive(Debug, Serialize)]
struct AnswerInterruptResponse {
  #[serde(flatten)]
  session: LocalHarnessSession,
  resumed: bool,
}

async fn answer_interrupt(
  State(state): State<LocalHarnessState>,
  headers: HeaderMap,
  RoutePath(session_id): RoutePath<Uuid>,
  Json(body): Json<AnswerInterruptRequest>,
) -> Response {
  let answer = body.answer.trim();
  if answer.is_empty() {
    return error_json(
      StatusCode::BAD_REQUEST,
      "`answer` must be a non-empty string",
    );
  }
  let tenant = extract_tenant(&headers);

  let (updated_session, initial_seq) = {
    let mut guard = state.lock_sessions();
    let Some(entry) = guard.get_mut(&session_id) else {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    };
    if entry.session.tenant_id != tenant {
      return error_json(
        StatusCode::NOT_FOUND,
        &format!("harness session {session_id} not found"),
      );
    }
    if entry.session.status != "awaiting_input" {
      return error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "harness session {session_id} is '{}' (expected 'awaiting_input')",
          entry.session.status
        ),
      );
    }

    let next_seq = entry
      .events
      .last()
      .map(|ev| (ev.seq.max(0) as u64).saturating_add(1))
      .unwrap_or(0);
    entry.session.status = "running".to_string();
    entry.session.pending_question = None;
    entry.session.pending_question_step_index = None;
    (entry.session.clone(), next_seq)
  };

  let session_dir = state.session_dir(session_id);
  let _ = write_session_sync(&session_dir, &updated_session);

  let skill_dir = updated_session
    .skill_name
    .as_deref()
    .and_then(|name| resolve_skill_dir(&updated_session.workspace_root, name));
  let cancel_guard = state.cancellations.register(session_id);
  let state_bg = state.clone();
  let answer_owned = answer.to_string();
  let inputs = SessionRunInputs {
    session_id,
    user_input: updated_session.user_input.clone(),
    workspace_root: updated_session.workspace_root.clone(),
    profile: updated_session.profile.clone(),
    runtime_kind: updated_session.runtime_kind.clone(),
    model: updated_session.model.clone(),
    skill_name: updated_session.skill_name.clone(),
    skill_dir,
    cost_limit_usd: updated_session.cost_limit_usd,
    max_steps: updated_session.max_steps,
    initial_seq,
    cancellation_token: cancel_guard.token(),
  };
  tokio::spawn(async move {
    let _guard = cancel_guard;
    resume_interrupt_session(state_bg, inputs, answer_owned).await;
  });

  (
    StatusCode::OK,
    Json(AnswerInterruptResponse {
      session: updated_session,
      resumed: true,
    }),
  )
    .into_response()
}

// ── Background Harness Executor ──────────────────────────────────────

#[derive(Clone)]
struct SessionRunInputs {
  session_id: Uuid,
  user_input: String,
  workspace_root: String,
  profile: String,
  runtime_kind: String,
  model: String,
  skill_name: Option<String>,
  skill_dir: Option<PathBuf>,
  cost_limit_usd: Option<f64>,
  max_steps: Option<usize>,
  initial_seq: u64,
  cancellation_token: AgentCancellationToken,
}

fn resolve_max_steps(requested: Option<usize>) -> usize {
  requested
    .unwrap_or(DEFAULT_MAX_STEPS)
    .min(MAX_STEPS_CEILING)
}

fn parse_profile(value: &str) -> HarnessProfile {
  match value {
    "dev" => HarnessProfile::Dev,
    "production" => HarnessProfile::Production,
    _ => HarnessProfile::Local,
  }
}

fn parse_runtime_kind(value: &str) -> HarnessRuntimeKind {
  match value {
    "plan_execute" => HarnessRuntimeKind::PlanExecute,
    _ => HarnessRuntimeKind::React,
  }
}

fn session_agent_kind(value: &str) -> LoopRuntimeKind {
  match value {
    "plan_execute" => LoopRuntimeKind::PlanExecute,
    _ => LoopRuntimeKind::React,
  }
}

fn resolve_skill_dir(workspace_root: &str, skill_name: &str) -> Option<PathBuf> {
  if skill_name.is_empty()
    || skill_name.contains('/')
    || skill_name.contains('\\')
    || skill_name == "."
    || skill_name == ".."
  {
    return None;
  }
  let mut candidates = vec![
    Path::new(workspace_root).join("skills").join(skill_name),
    Path::new(workspace_root)
      .join(".yanshi")
      .join("skills")
      .join(skill_name),
  ];
  if let Some(home) = dirs::home_dir() {
    candidates.push(home.join(".yanshi").join("skills").join(skill_name));
  }
  candidates
    .into_iter()
    .find(|dir| dir.join("SKILL.md").is_file() || dir.join("skill.toml").is_file())
}

async fn execute_session(state: LocalHarnessState, inputs: SessionRunInputs) {
  let Ok(_permit) = state.concurrency_limit.clone().acquire_owned().await else {
    return;
  };
  let session_id = inputs.session_id;
  match execute_session_inner(state.clone(), inputs).await {
    Ok(result) => apply_run_result(&state, session_id, &result),
    Err(err) => apply_run_failure(&state, session_id, &err.to_string()).await,
  }
}

async fn resume_interrupt_session(
  state: LocalHarnessState,
  inputs: SessionRunInputs,
  answer: String,
) {
  let Ok(_permit) = state.concurrency_limit.clone().acquire_owned().await else {
    return;
  };
  let session_id = inputs.session_id;
  match resume_interrupt_inner(state.clone(), inputs, answer).await {
    Ok(result) => apply_run_result(&state, session_id, &result),
    Err(err) => apply_run_failure(&state, session_id, &err.to_string()).await,
  }
}

async fn execute_session_inner(
  state: LocalHarnessState,
  inputs: SessionRunInputs,
) -> anyhow::Result<yanshi_harness::HarnessRunResult> {
  ensure_llm_initialized().await?;
  let join = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()?;
    rt.block_on(run_harness_on_current_thread(state, inputs))
  });
  join
    .await
    .map_err(|err| anyhow::anyhow!("harness task panicked: {err}"))?
}

async fn resume_interrupt_inner(
  state: LocalHarnessState,
  inputs: SessionRunInputs,
  answer: String,
) -> anyhow::Result<yanshi_harness::HarnessRunResult> {
  ensure_llm_initialized().await?;
  let checkpointer = FileLoopCheckpointer::new(state.checkpoints_dir())
    .map_err(|err| anyhow::anyhow!("failed to open loop checkpoint dir: {err}"))?;
  let checkpoint = checkpointer
    .load(&inputs.session_id.to_string())
    .await
    .map_err(|err| anyhow::anyhow!("failed to load loop checkpoint: {err}"))?
    .ok_or_else(|| anyhow::anyhow!("no loop checkpoint found for session {}", inputs.session_id))?;

  let join = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()?;
    rt.block_on(resume_harness_on_current_thread(
      state, inputs, checkpoint, answer,
    ))
  });
  join
    .await
    .map_err(|err| anyhow::anyhow!("harness resume task panicked: {err}"))?
}

async fn open_local_memory(db_path: &Path) -> Box<dyn MemoryStore> {
  if let Some(parent) = db_path.parent() {
    let _ = std::fs::create_dir_all(parent);
  }
  match SqliteMemory::open(db_path).await {
    Ok(store) => Box::new(store),
    Err(err) => {
      warn!(error = %err, "falling back to in-memory SessionMemory for local harness");
      Box::new(SessionMemory::default_window())
    }
  }
}

async fn build_local_agent(
  state: &LocalHarnessState,
  inputs: &SessionRunInputs,
  hook_config: HookConfig,
  kind: LoopRuntimeKind,
) -> anyhow::Result<Box<dyn AgentRuntime>> {
  let workspace_path = Path::new(&inputs.workspace_root);
  match kind {
    LoopRuntimeKind::PlanExecute => {
      let tools = yanshi_tools::default_governed_registry(workspace_path)
        .map_err(|err| anyhow::anyhow!("failed to build governed tool registry: {err}"))?;
      let registry = wrap_registry(tools, hook_config);
      let memory = open_local_memory(&state.memory_db_path()).await;
      let plan_cfg = PlanExecuteConfig::new(&inputs.model);
      Ok(Box::new(PlanExecuteAgent::new(plan_cfg, memory, registry)))
    }
    LoopRuntimeKind::React => {
      if let Some(skill_dir) = inputs.skill_dir.as_deref() {
        let manifest = SkillLoader::load(skill_dir)
          .map_err(|err| anyhow::anyhow!("failed to load skill manifest: {err}"))?;
        let _warnings = SkillLoader::validate(&manifest, skill_dir)
          .map_err(|err| anyhow::anyhow!("skill validation failed: {err}"))?;
        let mut agent =
          SkillBuilder::build_with_project_root(&manifest, skill_dir, Some(workspace_path))
            .await
            .map_err(|err| anyhow::anyhow!("failed to build skill agent: {err}"))?;
        let mut snapshot = ToolRegistry::new();
        for tool in agent.tools().list() {
          snapshot.register(tool);
        }
        agent = agent.with_tools(wrap_registry(snapshot, hook_config));
        return Ok(Box::new(agent));
      }

      let tools = yanshi_tools::default_governed_registry(workspace_path)
        .map_err(|err| anyhow::anyhow!("failed to build governed tool registry: {err}"))?;
      let registry = wrap_registry(tools, hook_config);
      let memory = open_local_memory(&state.memory_db_path()).await;
      let react_cfg = ReActConfig::new(&inputs.model);
      Ok(Box::new(ReActAgent::new(react_cfg, memory, registry)))
    }
  }
}

async fn run_harness_on_current_thread(
  state: LocalHarnessState,
  inputs: SessionRunInputs,
) -> anyhow::Result<yanshi_harness::HarnessRunResult> {
  let session_id_str = inputs.session_id.to_string();
  let profile = parse_profile(&inputs.profile);
  let runtime_kind = parse_runtime_kind(&inputs.runtime_kind);

  let sink: Arc<dyn HarnessEventSink> = Arc::new(LocalHarnessEventSink::new(state.clone()));
  let sinks = SinkChain::new().push(sink.clone());
  let seq_allocator = SeqAllocator::with_initial(inputs.initial_seq);
  let approval_provider: Arc<dyn ApprovalProvider> = Arc::new(LocalApprovalProvider::new(
    state.approvals.clone(),
    DEFAULT_APPROVAL_TIMEOUT,
  ));

  let hook_config = HookConfig::new(session_id_str.clone(), approval_provider, sinks)
    .with_profile(profile)
    .with_seq_allocator(seq_allocator.clone())
    .with_approval_timeout(DEFAULT_APPROVAL_TIMEOUT);

  let agent = build_local_agent(
    &state,
    &inputs,
    hook_config,
    session_agent_kind(&inputs.runtime_kind),
  )
  .await?;

  let checkpointer: Arc<dyn AgentLoopCheckpointer> = Arc::new(
    FileLoopCheckpointer::new(state.checkpoints_dir())
      .map_err(|err| anyhow::anyhow!("failed to initialize FileLoopCheckpointer: {err}"))?,
  );

  let mut runtime = HarnessRuntime::new(agent)
    .with_event_sink(sink)
    .with_context_providers(default_providers())
    .with_seq_allocator(seq_allocator);

  let options = HarnessRunOptions::new(
    inputs.user_input,
    PathBuf::from(&inputs.workspace_root),
    inputs.model,
  )
  .with_profile(profile)
  .with_runtime_kind(runtime_kind)
  .with_session_id(session_id_str)
  .with_cancellation_token(inputs.cancellation_token)
  .with_loop_checkpointer(checkpointer)
  .with_limits(RuntimeLimits {
    max_steps: Some(resolve_max_steps(inputs.max_steps)),
    cost_limit_usd: inputs.cost_limit_usd,
    ..Default::default()
  });
  let options = match inputs.skill_name {
    Some(skill) => options.with_skill_name(skill),
    None => options,
  };

  let result = runtime.run(options).await?;
  Ok(result)
}

async fn resume_harness_on_current_thread(
  state: LocalHarnessState,
  inputs: SessionRunInputs,
  checkpoint: AgentLoopCheckpoint,
  answer: String,
) -> anyhow::Result<yanshi_harness::HarnessRunResult> {
  let session_id_str = inputs.session_id.to_string();
  let profile = parse_profile(&inputs.profile);

  let sink: Arc<dyn HarnessEventSink> = Arc::new(LocalHarnessEventSink::new(state.clone()));
  let sinks = SinkChain::new().push(sink.clone());
  let seq_allocator = SeqAllocator::with_initial(inputs.initial_seq);
  let approval_provider: Arc<dyn ApprovalProvider> = Arc::new(LocalApprovalProvider::new(
    state.approvals.clone(),
    DEFAULT_APPROVAL_TIMEOUT,
  ));

  let hook_config = HookConfig::new(session_id_str, approval_provider, sinks)
    .with_profile(profile)
    .with_seq_allocator(seq_allocator.clone())
    .with_approval_timeout(DEFAULT_APPROVAL_TIMEOUT);

  let agent = build_local_agent(&state, &inputs, hook_config, checkpoint.runtime_kind).await?;

  let checkpointer: Arc<dyn AgentLoopCheckpointer> = Arc::new(
    FileLoopCheckpointer::new(state.checkpoints_dir())
      .map_err(|err| anyhow::anyhow!("failed to initialize FileLoopCheckpointer: {err}"))?,
  );

  let mut runtime = HarnessRuntime::new(agent)
    .with_event_sink(sink)
    .with_seq_allocator(seq_allocator);

  let options = HarnessRunOptions::new(
    inputs.user_input,
    PathBuf::from(&inputs.workspace_root),
    inputs.model,
  )
  .with_cancellation_token(inputs.cancellation_token)
  .with_loop_checkpointer(checkpointer)
  .with_limits(RuntimeLimits {
    max_steps: Some(resolve_max_steps(inputs.max_steps)),
    cost_limit_usd: inputs.cost_limit_usd,
    ..Default::default()
  });

  let result = runtime
    .resume_from_interrupt(options, checkpoint, answer)
    .await?;
  Ok(result)
}

fn apply_run_result(
  state: &LocalHarnessState,
  session_id: Uuid,
  result: &yanshi_harness::HarnessRunResult,
) {
  let snapshot = {
    let mut guard = state.lock_sessions();
    let Some(entry) = guard.get_mut(&session_id) else {
      return;
    };
    if is_terminal_harness_status(&entry.session.status) {
      return;
    }

    if let AgentStopReason::AwaitingInput { question } = &result.stop_reason {
      let step_index = result
        .inner
        .steps
        .last()
        .map(|step| step.index as i64)
        .unwrap_or(0);
      entry.session.status = "awaiting_input".to_string();
      entry.session.pending_question = Some(question.clone());
      entry.session.pending_question_step_index = Some(step_index);
      Some(entry.session.clone())
    } else {
      let (status, final_answer, error) = match &result.stop_reason {
        AgentStopReason::FinalAnswer | AgentStopReason::StopCondition { .. } => {
          ("completed", result.answer.clone(), None)
        }
        AgentStopReason::MaxSteps { max_steps } => (
          "failed",
          result.answer.clone(),
          Some(format!("max_steps_reached:{max_steps}")),
        ),
        AgentStopReason::MaxToolCalls { max_tool_calls } => (
          "failed",
          result.answer.clone(),
          Some(format!("max_tool_calls_reached:{max_tool_calls}")),
        ),
        AgentStopReason::Timeout { timeout_ms } => (
          "failed",
          result.answer.clone(),
          Some(format!("timeout:{timeout_ms}ms")),
        ),
        AgentStopReason::Cancelled { message } => {
          ("cancelled", None, Some(format!("cancelled:{message}")))
        }
        AgentStopReason::TokenBudgetExceeded { used, budget } => (
          "failed",
          result.answer.clone(),
          Some(format!("token_budget_exceeded:{used}/{budget}")),
        ),
        AgentStopReason::CostLimitExceeded {
          used_usd,
          budget_usd,
        } => (
          "failed",
          result.answer.clone(),
          Some(format!(
            "cost_limit_exceeded:${used_usd:.4}/${budget_usd:.4}"
          )),
        ),
        AgentStopReason::LoopDetected { tool, repeats } => (
          "failed",
          result.answer.clone(),
          Some(format!("loop_detected:{tool}x{repeats}")),
        ),
        AgentStopReason::Error { message } => {
          ("failed", None, Some(format!("agent_error:{message}")))
        }
        AgentStopReason::AwaitingInput { .. } => unreachable!("handled in AwaitingInput branch"),
        AgentStopReason::ApprovalDenied { message } => {
          ("failed", None, Some(format!("approval_denied:{message}")))
        }
      };

      entry.session.status = status.to_string();
      entry.session.finished_at = Some(Utc::now());
      entry.session.final_answer = final_answer;
      entry.session.error = error;
      entry.session.pending_question = None;
      entry.session.pending_question_step_index = None;
      Some(entry.session.clone())
    }
  };

  if let Some(session) = snapshot {
    let _ = write_session_sync(&state.session_dir(session_id), &session);
  }
}

async fn apply_run_failure(state: &LocalHarnessState, session_id: Uuid, err_msg: &str) {
  let next_seq_and_session = {
    let mut guard = state.lock_sessions();
    let Some(entry) = guard.get_mut(&session_id) else {
      return;
    };
    if is_terminal_harness_status(&entry.session.status) {
      return;
    }
    entry.session.status = "failed".to_string();
    entry.session.finished_at = Some(Utc::now());
    entry.session.error = Some(err_msg.to_string());
    entry.session.pending_question = None;
    entry.session.pending_question_step_index = None;
    let next_seq = entry
      .events
      .last()
      .map(|ev| (ev.seq.max(0) as u64).saturating_add(1))
      .unwrap_or(0);
    Some((next_seq, entry.session.clone()))
  };

  let Some((next_seq, session)) = next_seq_and_session else {
    return;
  };
  let _ = write_session_sync(&state.session_dir(session_id), &session);

  let sink = LocalHarnessEventSink::new(state.clone());
  let event = HarnessEvent {
    seq: next_seq,
    session_id: session_id.to_string(),
    ts: Utc::now(),
    body: HarnessEventBody::Stopped(StoppedPayload {
      reason: StopReason::Failed,
      final_answer: None,
      error: Some(err_msg.to_string()),
    }),
  };
  let _ = sink.write(&event).await;
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use crate::local_engine::{LocalEngineState, build_local_router, start_loopback_engine};
  use std::sync::OnceLock;
  use yanshi_harness::ApprovalRisk;
  use yanshi_llm::Yanshi;

  pub(crate) fn mock_env_lock() -> &'static tokio::sync::Mutex<()> {
    static LOCK: OnceLock<tokio::sync::Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| tokio::sync::Mutex::new(()))
  }

  pub(crate) async fn register_mock_model(model: &str) {
    let _ = ensure_llm_initialized().await;
    let config_path = std::env::temp_dir().join(format!(
      "yanshi-desktop-harness-mock-{}.yml",
      Uuid::new_v4()
    ));
    std::fs::write(
      &config_path,
      format!(
        r#"
models:
  {model}:
    vendor: mock
    type: text
    model_id: {model}
providers:
  mock:
    api_key_env: MOCK_API_KEY
"#
      ),
    )
    .expect("write mock model config");
    Yanshi::init_with_config(config_path.to_str().expect("utf8 path"))
      .await
      .expect("init mock model");
  }

  #[tokio::test]
  async fn local_harness_executes_react_session_persists_events_and_reloads_on_restart() {
    let _env_guard = mock_env_lock().lock().await;
    let model = format!("mock-desktop-react-{}", Uuid::new_v4());
    // SAFETY: serialized by `mock_env_lock()`.
    unsafe {
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![
          r#"{"thought":"summarize workspace","answer":"Workspace contains 2 modules."}"#,
          r#"{"thought":"follow up","answer":"Follow-up complete."}"#,
        ])
        .expect("serialize mock responses"),
      );
      std::env::remove_var("YANSHI_MOCK_DELAY_MS");
    }
    register_mock_model(&model).await;

    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = tempfile::tempdir().expect("workspace");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start loopback engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. Reject `/` workspace root
    let root_reject = client
      .post(format!("{base_url}/v1/harness/sessions"))
      .json(&serde_json::json!({
        "user_input": "hello",
        "workspace_root": "/",
        "model": model,
      }))
      .send()
      .await
      .expect("root workspace request");
    assert_eq!(root_reject.status(), StatusCode::BAD_REQUEST);

    // 2. Submit real ReAct session under tenant `team-a`
    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/harness/sessions"))
      .header("X-Yanshi-Tenant", "team-a")
      .json(&serde_json::json!({
        "tenant_id": "team-a",
        "user_input": "Summarize the workspace.",
        "workspace_root": workspace.path().display().to_string(),
        "profile": "local",
        "runtime_kind": "react",
        "model": model,
      }))
      .send()
      .await
      .expect("submit session")
      .json()
      .await
      .expect("submit json");
    let session_id = created["session_id"].as_str().expect("session_id");

    // Wait for completion
    let mut status = String::new();
    let mut final_answer = String::new();
    for _ in 0..80 {
      let detail: serde_json::Value = client
        .get(format!("{base_url}/v1/harness/sessions/{session_id}"))
        .header("X-Yanshi-Tenant", "team-a")
        .send()
        .await
        .expect("get session")
        .json()
        .await
        .expect("session json");
      status = detail["status"].as_str().unwrap_or("").to_string();
      final_answer = detail["final_answer"].as_str().unwrap_or("").to_string();
      if is_terminal_harness_status(&status) {
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(status, "completed");
    assert_eq!(final_answer, "Workspace contains 2 modules.");

    // 3. Tenant isolation: `team-b` sees empty list and 404 on `team-a`'s session
    let team_b_list: serde_json::Value = client
      .get(format!("{base_url}/v1/harness/sessions"))
      .header("X-Yanshi-Tenant", "team-b")
      .send()
      .await
      .expect("list team-b")
      .json()
      .await
      .expect("team-b json");
    assert_eq!(team_b_list["sessions"].as_array().map(Vec::len), Some(0));

    let cross_tenant = client
      .get(format!("{base_url}/v1/harness/sessions/{session_id}"))
      .header("X-Yanshi-Tenant", "team-b")
      .send()
      .await
      .expect("cross tenant get");
    assert_eq!(cross_tenant.status(), StatusCode::NOT_FOUND);

    // 4. Events history returns bare JSON array `[...]` with `session_started` and `stopped`
    let events: Vec<LocalStreamedHarnessEvent> = client
      .get(format!(
        "{base_url}/v1/harness/sessions/{session_id}/events/history"
      ))
      .header("X-Yanshi-Tenant", "team-a")
      .send()
      .await
      .expect("events history")
      .json()
      .await
      .expect("events history array");
    assert!(events.iter().any(|e| e.kind == "session_started"));
    assert!(events.iter().any(|e| e.kind == "stopped"));
    assert!(!events.iter().any(|e| e.kind == "token_delta"));
    let first_run_max_seq = events.last().map(|e| e.seq).unwrap_or(0);

    // 5. Resume in `append` mode — preserves prior events and continues monotonic `seq`
    let resume_res = client
      .post(format!(
        "{base_url}/v1/harness/sessions/{session_id}:resume"
      ))
      .header("X-Yanshi-Tenant", "team-a")
      .json(&serde_json::json!({
        "mode": "append",
        "user_input": "Now run follow-up."
      }))
      .send()
      .await
      .expect("resume session");
    assert_eq!(resume_res.status(), StatusCode::OK);

    for _ in 0..80 {
      let detail: serde_json::Value = client
        .get(format!("{base_url}/v1/harness/sessions/{session_id}"))
        .header("X-Yanshi-Tenant", "team-a")
        .send()
        .await
        .expect("get resumed session")
        .json()
        .await
        .expect("resumed json");
      if detail["status"].as_str() == Some("completed") {
        assert_eq!(detail["final_answer"].as_str(), Some("Follow-up complete."));
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }

    let appended_events: Vec<LocalStreamedHarnessEvent> = client
      .get(format!(
        "{base_url}/v1/harness/sessions/{session_id}/events/history"
      ))
      .header("X-Yanshi-Tenant", "team-a")
      .send()
      .await
      .expect("appended history")
      .json()
      .await
      .expect("appended array");
    assert!(
      appended_events
        .last()
        .is_some_and(|last| last.seq > first_run_max_seq)
    );

    // 6. Restart engine against the same directory and verify session & events survive
    let restarted_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("restart loopback engine");
    let reloaded_list: serde_json::Value = client
      .get(format!("{restarted_url}/v1/harness/sessions"))
      .header("X-Yanshi-Tenant", "team-a")
      .send()
      .await
      .expect("reloaded list")
      .json()
      .await
      .expect("reloaded list json");
    assert_eq!(reloaded_list["sessions"].as_array().map(Vec::len), Some(1));
  }

  #[tokio::test]
  async fn local_harness_supports_ask_user_interrupt_and_resume_with_checkpoint() {
    let _env_guard = mock_env_lock().lock().await;
    let model = format!("mock-desktop-interrupt-{}", Uuid::new_v4());
    // PlanExecuteAgent: planner returns `{"plan":[...ask_user...]}`, pauses at `awaiting_input`;
    // after `/interrupt/answer`, resumes from FileLoopCheckpointer and synthesizes final answer!
    // SAFETY: serialized by `mock_env_lock()`.
    unsafe {
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![
          r#"{"plan":[{"id":"1","description":"Ask target environment","tool":"ask_user","params":{"question":"Deploy to staging or prod?"}}]}"#,
        ])
        .expect("serialize interrupt responses"),
      );
      std::env::remove_var("YANSHI_MOCK_DELAY_MS");
    }
    register_mock_model(&model).await;

    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = tempfile::tempdir().expect("workspace");
    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start loopback engine");

    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/harness/sessions"))
      .json(&serde_json::json!({
        "user_input": "Prepare deployment",
        "workspace_root": workspace.path().display().to_string(),
        "runtime_kind": "plan_execute",
        "model": model,
      }))
      .send()
      .await
      .expect("submit plan_execute")
      .json()
      .await
      .expect("submit json");
    let session_id = created["session_id"].as_str().expect("session_id");

    // Wait until paused at `awaiting_input`
    let mut paused = false;
    for _ in 0..80 {
      let detail: serde_json::Value = client
        .get(format!("{base_url}/v1/harness/sessions/{session_id}"))
        .send()
        .await
        .expect("get session")
        .json()
        .await
        .expect("detail json");
      if detail["status"].as_str() == Some("awaiting_input") {
        paused = true;
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(paused, "session did not pause on ask_user");

    let interrupt: serde_json::Value = client
      .get(format!(
        "{base_url}/v1/harness/sessions/{session_id}/interrupt"
      ))
      .send()
      .await
      .expect("get interrupt")
      .json()
      .await
      .expect("interrupt json");
    assert_eq!(
      interrupt["pending"]["question"].as_str(),
      Some("Deploy to staging or prod?")
    );

    // Submit answer to resume from FileLoopCheckpointer
    let answer_res = client
      .post(format!(
        "{base_url}/v1/harness/sessions/{session_id}/interrupt/answer"
      ))
      .json(&serde_json::json!({ "answer": "staging" }))
      .send()
      .await
      .expect("post interrupt answer");
    assert_eq!(answer_res.status(), StatusCode::OK);

    let mut final_status = String::new();
    let mut final_answer = String::new();
    for _ in 0..80 {
      let detail: serde_json::Value = client
        .get(format!("{base_url}/v1/harness/sessions/{session_id}"))
        .send()
        .await
        .expect("get resumed session")
        .json()
        .await
        .expect("detail json");
      final_status = detail["status"].as_str().unwrap_or("").to_string();
      final_answer = detail["final_answer"].as_str().unwrap_or("").to_string();
      if is_terminal_harness_status(&final_status) {
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert_eq!(final_status, "completed");
    assert!(
      final_answer.contains("Deploy to staging or prod?: staging"),
      "unexpected final_answer: {final_answer}"
    );
  }

  #[tokio::test]
  async fn local_harness_parks_and_resolves_pending_approvals_and_cancel() {
    let _env_guard = mock_env_lock().lock().await;
    let model = format!("mock-desktop-slow-{}", Uuid::new_v4());
    // SAFETY: serialized by `mock_env_lock()`.
    unsafe {
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![r#"{"thought":"slow","answer":"done"}"#])
          .expect("serialize slow response"),
      );
      std::env::set_var("YANSHI_MOCK_DELAY_MS", "2000");
    }
    register_mock_model(&model).await;

    let temp = tempfile::tempdir().expect("tempdir");
    let workspace = tempfile::tempdir().expect("workspace");
    let state = LocalEngineState::new(temp.path().to_path_buf());
    let harness_state = state.harness().clone();
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

    let created: serde_json::Value = client
      .post(format!("{base_url}/v1/harness/sessions"))
      .json(&serde_json::json!({
        "user_input": "Long task",
        "workspace_root": workspace.path().display().to_string(),
        "model": model,
      }))
      .send()
      .await
      .expect("submit slow session")
      .json()
      .await
      .expect("submit json");
    let session_id = created["session_id"]
      .as_str()
      .expect("session_id")
      .to_string();

    // Park a pending approval on this session and resolve it via HTTP
    let approval_rx = harness_state.approvals.park(ApprovalRequest {
      id: "req-42".to_string(),
      session_id: session_id.clone(),
      step_index: 1,
      tool: "http".to_string(),
      source: None,
      permissions: Vec::new(),
      idempotency: Default::default(),
      effect: Default::default(),
      params_summary: serde_json::json!({ "url": "https://example.com" }),
      risk: ApprovalRisk::Medium,
      reason: "outbound HTTP request".to_string(),
      requested_at: Utc::now(),
      expires_at: None,
    });

    let listed: serde_json::Value = client
      .get(format!(
        "{base_url}/v1/harness/sessions/{session_id}/approvals"
      ))
      .send()
      .await
      .expect("list approvals")
      .json()
      .await
      .expect("approvals json");
    assert_eq!(listed["approvals"].as_array().map(Vec::len), Some(1));
    assert_eq!(listed["approvals"][0]["id"], "req-42");

    let decide_res = client
      .post(format!(
        "{base_url}/v1/harness/sessions/{session_id}/approvals/req-42"
      ))
      .json(&serde_json::json!({
        "decision": "allow",
        "scope": "session",
        "decided_by": "test-operator",
      }))
      .send()
      .await
      .expect("decide approval");
    assert_eq!(decide_res.status(), StatusCode::OK);

    let resolved = approval_rx.await.expect("oneshot resolved");
    assert_eq!(resolved.decision, ApprovalOutcome::Allow);
    assert_eq!(resolved.scope, ApprovalScope::Session);

    // Cancel the running session
    let cancel_res: serde_json::Value = client
      .post(format!(
        "{base_url}/v1/harness/sessions/{session_id}:cancel"
      ))
      .send()
      .await
      .expect("cancel session")
      .json()
      .await
      .expect("cancel json");
    assert_eq!(cancel_res["cancelled"], true);
    assert_eq!(cancel_res["status"], "cancelled");

    // SAFETY: reset delay env var for subsequent tests.
    unsafe {
      std::env::remove_var("YANSHI_MOCK_DELAY_MS");
    }
  }
}
