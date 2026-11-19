use crate::events::DebugPausePhase;
use crate::resource_limits::ResourceLimits;
use crate::retry::RetryPolicy;
use crate::value::FlowValue;
use serde::{Deserialize, Serialize};
use std::{
  collections::{BTreeSet, HashMap},
  path::PathBuf,
  sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
  },
};
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowExecutionMode {
  Serial,
  Concurrent,
}

// V1.4: `Eq` dropped (kept `PartialEq`) once `node_retry_policy` was
// added -- `RetryPolicy` only supports `PartialEq` (it nests an `f64`
// backoff multiplier). Nothing in the workspace relied on `Eq`
// specifically (no `HashSet<FlowExecutionConfig>` etc.).
#[derive(Debug, Clone, PartialEq)]
pub struct FlowExecutionConfig {
  pub mode: FlowExecutionMode,
  pub max_concurrency: usize,
  pub fail_fast: bool,
  pub continue_on_skip: bool,
  pub run_base_dir: Option<PathBuf>,
  pub cancellation_token: Option<FlowCancellationToken>,
  /// V1.4: optional Flow-wide timeout applied to every node's execution
  /// (`Standard`/`Map`/`While` alike), inside the executor itself.
  /// `None` (default) is the pre-V1.4 behavior: no Flow-level timeout.
  /// Complements (does not replace) `yanshi-config`'s per-node YAML
  /// `timeout_ms` (T3.1's `TimeoutRetryNode`), which wraps a node
  /// *before* it ever reaches this executor and so has no visibility
  /// into `WorkflowEvent`.
  pub node_timeout_ms: Option<u64>,
  /// V1.4: optional Flow-wide retry policy applied to every node's
  /// execution on a transient failure (network / timeout /
  /// rate-limit-class, per [`RetryPolicy`]'s default classification).
  /// `None` (default) is the pre-V1.4 behavior: no Flow-level retry.
  /// Every attempt emits `WorkflowEvent::RetryAttempt` (see
  /// `FlowExecutor::execute_node_with_reliability` in `flow.rs`), unlike
  /// `yanshi-config`'s per-node YAML `max_retries`, which retries
  /// silently from the executor's point of view.
  pub node_retry_policy: Option<RetryPolicy>,
  /// W5.3: optional Flow-wide advisory state-pool size limit. `None`
  /// (default) is unchanged pre-W5.3 behavior: no limit checked. When
  /// set, every `notify_state_size` call compares the current estimated
  /// state pool size (already computed for the passive
  /// `StateSizeObserver`) against [`ResourceLimits::exceeds_state_limit`]
  /// and emits `WorkflowEvent::ResourceWarning` when exceeded. This is
  /// advisory-only — no eviction, no rejection, nothing is ever removed
  /// from the state pool. A prior wire-in attempt (`state_monitor.rs`'s
  /// LRU eviction, removed in W5.3) was unsafe: a node's output can be a
  /// real dependency for any later node via `input_mapping` regardless of
  /// how recently it was "accessed", unlike a cache entry.
  pub resource_limits: Option<ResourceLimits>,
  /// Y2.3: USD spend cap for the run, from the YAML top-level
  /// `cost_limit_usd` or the gateway request. Billed nodes record their
  /// cost (`run_context::record_cost_usd`); once the run's total has
  /// passed the limit, every node that has not started yet is refused
  /// with `YanshiError::CostLimitExceeded` and one
  /// `WorkflowEvent::CostLimitExceeded` is emitted. The node that crosses
  /// the limit finishes; nodes already running concurrently finish too.
  pub cost_limit_usd: Option<f64>,
  /// Optional interactive debug controller enabling single-step execution
  /// and pre-node breakpoints (`None` in release mode — zero overhead).
  pub debug_controller: Option<FlowDebugController>,
  /// Optional interactive question handler enabling HITL `ask_user` prompts
  /// inside workflow nodes (`None` in headless / batch runs).
  pub question_handler: Option<crate::run_context::RunQuestionHandle>,
  /// Optional host-path ceiling enforced at runtime by modality/file readers
  /// (`None` in local CLI / SDK; `Some(roots)` on multi-tenant gateway runs,
  /// where local reads must stay within the run's artifact root or `roots`).
  pub allowed_path_roots: Option<Vec<PathBuf>>,
}

impl FlowExecutionConfig {
  /// The base directory each run's `<run_id>/` dir is created under:
  /// `run_base_dir` when set, else `~/.yanshi/runs`.
  pub fn resolved_run_base_dir(&self) -> crate::Result<PathBuf> {
    match &self.run_base_dir {
      Some(dir) => Ok(dir.clone()),
      None => dirs::home_dir()
        .map(|home| home.join(".yanshi").join("runs"))
        .ok_or_else(|| crate::YanshiError::ConfigurationError {
          message: "Could not find home directory".to_string(),
        }),
    }
  }

  pub fn serial() -> Self {
    Self::default()
  }

  pub fn concurrent(max_concurrency: usize) -> Self {
    Self {
      mode: FlowExecutionMode::Concurrent,
      max_concurrency: max_concurrency.max(1),
      fail_fast: true,
      continue_on_skip: true,
      run_base_dir: None,
      cancellation_token: None,
      node_timeout_ms: None,
      node_retry_policy: None,
      resource_limits: None,
      cost_limit_usd: None,
      debug_controller: None,
      question_handler: None,
      allowed_path_roots: None,
    }
  }

  /// Attach an interactive [`crate::run_context::RunQuestionHandler`] for
  /// HITL `ask_user` prompts emitted by workflow nodes.
  pub fn with_question_handler(
    mut self,
    handler: Arc<dyn crate::run_context::RunQuestionHandler>,
  ) -> Self {
    self.question_handler = Some(crate::run_context::RunQuestionHandle(handler));
    self
  }

  /// Attach an interactive [`FlowDebugController`] (forces deterministic
  /// serial stepping so operator inspection sees one node at a time).
  pub fn with_debug_controller(mut self, controller: FlowDebugController) -> Self {
    self.mode = FlowExecutionMode::Serial;
    self.max_concurrency = 1;
    self.debug_controller = Some(controller);
    self
  }

  /// Y2.3: cap the run's recorded spend (see [`Self::cost_limit_usd`]).
  pub fn with_cost_limit_usd(mut self, limit_usd: f64) -> Self {
    self.cost_limit_usd = Some(limit_usd);
    self
  }

  pub fn with_run_base_dir(mut self, run_base_dir: impl Into<PathBuf>) -> Self {
    self.run_base_dir = Some(run_base_dir.into());
    self
  }

  pub fn with_cancellation_token(mut self, token: FlowCancellationToken) -> Self {
    self.cancellation_token = Some(token);
    self
  }

  /// V1.4: opt into a Flow-wide per-node timeout.
  pub fn with_node_timeout_ms(mut self, timeout_ms: u64) -> Self {
    self.node_timeout_ms = Some(timeout_ms);
    self
  }

  /// V1.4: opt into a Flow-wide per-node retry policy.
  pub fn with_node_retry_policy(mut self, policy: RetryPolicy) -> Self {
    self.node_retry_policy = Some(policy);
    self
  }

  /// W5.3: opt into an advisory Flow-wide state-pool size limit.
  pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
    self.resource_limits = Some(limits);
    self
  }

  /// Confine runtime local media/file reads to this run's artifact root plus `roots`.
  pub fn with_allowed_path_roots(mut self, roots: Vec<PathBuf>) -> Self {
    self.allowed_path_roots = Some(roots);
    self
  }
}

impl Default for FlowExecutionConfig {
  fn default() -> Self {
    Self {
      mode: FlowExecutionMode::Serial,
      max_concurrency: 1,
      fail_fast: true,
      continue_on_skip: true,
      run_base_dir: None,
      cancellation_token: None,
      node_timeout_ms: None,
      node_retry_policy: None,
      resource_limits: None,
      cost_limit_usd: None,
      debug_controller: None,
      question_handler: None,
      allowed_path_roots: None,
    }
  }
}

/// Process-local cancellation signal for Flow execution.
#[derive(Debug, Clone)]
pub struct FlowCancellationToken {
  cancelled: Arc<AtomicBool>,
  notify: Arc<Notify>,
}

impl FlowCancellationToken {
  pub fn new() -> Self {
    Self {
      cancelled: Arc::new(AtomicBool::new(false)),
      notify: Arc::new(Notify::new()),
    }
  }

  pub fn cancel(&self) {
    self.cancelled.store(true, Ordering::SeqCst);
    self.notify.notify_waiters();
  }

  pub fn is_cancelled(&self) -> bool {
    self.cancelled.load(Ordering::SeqCst)
  }

  /// Resolves immediately if already cancelled, or awaits [`Self::cancel`].
  pub async fn cancelled(&self) {
    loop {
      let notified = self.notify.notified();
      if self.is_cancelled() {
        return;
      }
      notified.await;
    }
  }
}

impl Default for FlowCancellationToken {
  fn default() -> Self {
    Self::new()
  }
}

impl PartialEq for FlowCancellationToken {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.cancelled, &other.cancelled) || self.is_cancelled() == other.is_cancelled()
  }
}

impl Eq for FlowCancellationToken {}

/// Details of an active pause boundary on a debuggable Flow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDebugPause {
  pub node_id: String,
  pub phase: DebugPausePhase,
  pub next_nodes: Vec<String>,
}

/// Read-only snapshot of [`FlowDebugController`] state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDebugSnapshot {
  pub step_every_node: bool,
  pub breakpoints: Vec<String>,
  pub paused_at: Option<FlowDebugPause>,
}

/// Optional input/output override payload supplied when resuming a paused debug step.
///
/// - At [`DebugPausePhase::BeforeNode`]:
///   - `input_override`: replaces the resolved `inputs` map passed into the node.
///   - `output_override`: skips real node execution entirely (Mock injection) and
///     records `Ok(output_override)` as the node's result.
/// - At [`DebugPausePhase::AfterNode`]:
///   - `output_override`: replaces the node's captured output in `state_pool`,
///     on-disk step result, and checkpoint so downstream nodes consume the edited data.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DebugStepPatch {
  pub input_override: Option<HashMap<String, FlowValue>>,
  pub output_override: Option<HashMap<String, FlowValue>>,
}

impl DebugStepPatch {
  pub fn is_empty(&self) -> bool {
    self.input_override.is_none() && self.output_override.is_none()
  }

  /// Convert optional JSON object maps (from HTTP debug endpoints) into [`FlowValue`] maps.
  ///
  /// Both tagged `FlowValue` envelopes (`{"type":"json","value":...}`,
  /// `{"type":"file","path":...}`, `{"type":"url","url":...}`) and plain untagged
  /// JSON values are accepted seamlessly.
  pub fn from_json_maps(
    input_override: Option<serde_json::Map<String, serde_json::Value>>,
    output_override: Option<serde_json::Map<String, serde_json::Value>>,
  ) -> Self {
    Self {
      input_override: input_override.map(json_object_to_flow_map),
      output_override: output_override.map(json_object_to_flow_map),
    }
  }
}

fn json_object_to_flow_map(
  obj: serde_json::Map<String, serde_json::Value>,
) -> HashMap<String, FlowValue> {
  obj
    .into_iter()
    .map(|(k, v)| (k, json_value_to_flow_value(v)))
    .collect()
}

fn json_value_to_flow_value(val: serde_json::Value) -> FlowValue {
  if let serde_json::Value::Object(ref map) = val {
    match map.get("type").and_then(serde_json::Value::as_str) {
      Some("json") if map.contains_key("value") => {
        return FlowValue::Json(map.get("value").cloned().unwrap_or(serde_json::Value::Null));
      }
      Some("file") => {
        if let Some(path) = map.get("path").and_then(serde_json::Value::as_str) {
          let mime_type = map
            .get("mime_type")
            .and_then(serde_json::Value::as_str)
            .map(String::from);
          return FlowValue::File {
            path: PathBuf::from(path),
            mime_type,
          };
        }
      }
      Some("url") => {
        if let Some(url) = map.get("url").and_then(serde_json::Value::as_str) {
          let mime_type = map
            .get("mime_type")
            .and_then(serde_json::Value::as_str)
            .map(String::from);
          return FlowValue::Url {
            url: url.to_string(),
            mime_type,
          };
        }
      }
      _ => {}
    }
  }
  FlowValue::Json(val)
}

#[derive(Debug)]
struct DebugState {
  step_every_node: bool,
  breakpoints: BTreeSet<String>,
  paused_at: Option<FlowDebugPause>,
  resume_action: Option<&'static str>,
  pending_patch: DebugStepPatch,
  aborted: bool,
}

/// Interactive step-by-step debug gate for [`crate::Flow`] execution.
///
/// Mirrors [`FlowCancellationToken`] as a cheap, `Clone`-able handle backed
/// by `Arc`. When attached to [`FlowExecutionConfig::debug_controller`], the
/// executor pauses after each top-level node (when `step_every_node` is true)
/// and/or before nodes listed in `breakpoints`, waking on [`Self::step`],
/// [`Self::continue_run`], [`Self::abort`], or [`FlowCancellationToken::cancel`].
#[derive(Debug, Clone)]
pub struct FlowDebugController {
  state: Arc<Mutex<DebugState>>,
  notify: Arc<Notify>,
}

impl FlowDebugController {
  /// Create a controller that pauses after every executed node.
  pub fn step_every_node() -> Self {
    Self::new(true, std::iter::empty::<String>())
  }

  /// Create a controller with explicit initial `step_every_node` and pre-node breakpoints.
  pub fn new<I, S>(step_every_node: bool, breakpoints: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    Self {
      state: Arc::new(Mutex::new(DebugState {
        step_every_node,
        breakpoints: breakpoints.into_iter().map(Into::into).collect(),
        paused_at: None,
        resume_action: None,
        pending_patch: DebugStepPatch::default(),
        aborted: false,
      })),
      notify: Arc::new(Notify::new()),
    }
  }

  fn lock_state(&self) -> std::sync::MutexGuard<'_, DebugState> {
    self
      .state
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
  }

  /// Return the current debug gate state.
  pub fn snapshot(&self) -> FlowDebugSnapshot {
    let guard = self.lock_state();
    FlowDebugSnapshot {
      step_every_node: guard.step_every_node,
      breakpoints: guard.breakpoints.iter().cloned().collect(),
      paused_at: guard.paused_at.clone(),
    }
  }

  /// Replace the set of pre-execution (`BeforeNode`) breakpoint node IDs.
  pub fn set_breakpoints<I, S>(&self, breakpoints: I)
  where
    I: IntoIterator<Item = S>,
    S: Into<String>,
  {
    let mut guard = self.lock_state();
    guard.breakpoints = breakpoints.into_iter().map(Into::into).collect();
  }

  /// Enable or disable automatic pausing after every completed node.
  pub fn set_step_every_node(&self, enabled: bool) {
    let mut guard = self.lock_state();
    guard.step_every_node = enabled;
  }

  /// Advance one node boundary and keep single-step pausing active.
  /// Returns `true` if the controller was currently paused.
  pub fn step(&self) -> bool {
    self.step_with_patch(DebugStepPatch::default())
  }

  /// Advance one node boundary applying `patch` to the currently paused gate.
  /// Returns `true` if the controller was currently paused.
  pub fn step_with_patch(&self, patch: DebugStepPatch) -> bool {
    let was_paused = {
      let mut guard = self.lock_state();
      let paused = guard.paused_at.take().is_some();
      guard.step_every_node = true;
      if paused {
        guard.resume_action = Some("step");
        guard.pending_patch = patch;
      }
      paused
    };
    if was_paused {
      self.notify.notify_waiters();
    }
    was_paused
  }

  /// Switch out of single-step mode and resume execution until completion
  /// (or the next explicit `BeforeNode` breakpoint).
  /// Returns `true` if the controller was currently paused.
  pub fn continue_run(&self) -> bool {
    self.continue_with_patch(DebugStepPatch::default())
  }

  /// Switch out of single-step mode applying `patch` to the currently paused gate.
  /// Returns `true` if the controller was currently paused.
  pub fn continue_with_patch(&self, patch: DebugStepPatch) -> bool {
    let was_paused = {
      let mut guard = self.lock_state();
      let paused = guard.paused_at.take().is_some();
      guard.step_every_node = false;
      if paused {
        guard.resume_action = Some("continue");
        guard.pending_patch = patch;
      }
      paused
    };
    if was_paused {
      self.notify.notify_waiters();
    }
    was_paused
  }

  /// Abort any parked debug gate immediately.
  pub fn abort(&self) {
    {
      let mut guard = self.lock_state();
      guard.aborted = true;
      guard.paused_at = None;
      guard.pending_patch = DebugStepPatch::default();
      guard.resume_action = Some("abort");
    }
    self.notify.notify_waiters();
  }

  /// Called by the Flow executor at `BeforeNode` and `AfterNode` boundaries.
  pub(crate) async fn gate(
    &self,
    node_id: &str,
    phase: DebugPausePhase,
    next_nodes: Vec<String>,
    cancellation_token: Option<&FlowCancellationToken>,
    on_paused: impl FnOnce(&FlowDebugPause),
    on_resumed: impl FnOnce(&'static str),
  ) -> crate::Result<DebugStepPatch> {
    let pause_info = {
      let mut guard = self.lock_state();
      if guard.aborted {
        return Err(crate::YanshiError::TaskCancelled);
      }
      let should_pause = match phase {
        DebugPausePhase::BeforeNode => guard.breakpoints.contains(node_id),
        DebugPausePhase::AfterNode => guard.step_every_node,
      };
      if !should_pause {
        return Ok(DebugStepPatch::default());
      }
      let pause = FlowDebugPause {
        node_id: node_id.to_string(),
        phase,
        next_nodes,
      };
      guard.paused_at = Some(pause.clone());
      guard.resume_action = None;
      guard.pending_patch = DebugStepPatch::default();
      pause
    };

    on_paused(&pause_info);

    let (action, patch) = loop {
      let notified = self.notify.notified();
      {
        let mut guard = self.lock_state();
        if guard.aborted {
          guard.paused_at = None;
          guard.pending_patch = DebugStepPatch::default();
          return Err(crate::YanshiError::TaskCancelled);
        }
        if let Some(act) = guard.resume_action.take() {
          let patch = std::mem::take(&mut guard.pending_patch);
          if act == "abort" {
            return Err(crate::YanshiError::TaskCancelled);
          }
          break (act, patch);
        }
      }

      if let Some(token) = cancellation_token {
        if token.is_cancelled() {
          let mut guard = self.lock_state();
          guard.paused_at = None;
          guard.pending_patch = DebugStepPatch::default();
          return Err(crate::YanshiError::TaskCancelled);
        }
        tokio::select! {
          biased;
          _ = token.cancelled() => {
            let mut guard = self.lock_state();
            guard.paused_at = None;
            guard.pending_patch = DebugStepPatch::default();
            return Err(crate::YanshiError::TaskCancelled);
          }
          _ = notified => {}
        }
      } else {
        notified.await;
      }
    };

    on_resumed(action);
    Ok(patch)
  }
}

impl PartialEq for FlowDebugController {
  fn eq(&self, other: &Self) -> bool {
    Arc::ptr_eq(&self.state, &other.state)
  }
}

impl Eq for FlowDebugController {}
