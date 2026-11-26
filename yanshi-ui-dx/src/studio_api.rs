//! Typed client, SSE event reducer, media/deliverable extractor, event filter,
//! and trace-comparison helpers for Workflow Studio, Classic Console & Run Compare.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::api::{
  PendingApproval, encode_uri_component, join_api_url, request_json, resolve_api_base,
};
use crate::notebook_api::{
  MediaPreviewItem, MediaPreviewKind, NotebookArtifact, append_inline_query, classify_media_path,
  is_run_terminal,
};
use crate::studio_yaml::InspectWorkflowResponse;

// ── Run, Event, Debug, Resume & Question Domain Types ────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunRecord {
  pub id: String,
  #[serde(default)]
  pub workflow: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub tenant_id: Option<String>,
  #[serde(default)]
  pub started_at: Option<String>,
  #[serde(default)]
  pub finished_at: Option<String>,
  #[serde(default)]
  pub run_dir: Option<String>,
  #[serde(default)]
  pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct StreamedEvent {
  #[serde(default)]
  pub run_id: String,
  #[serde(default)]
  pub seq: i64,
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub payload: serde_json::Value,
  #[serde(default)]
  pub ts: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSummary {
  pub id: String,
  #[serde(default)]
  pub node_id: String,
  pub name: String,
  #[serde(default)]
  pub mime_type: Option<String>,
  #[serde(default)]
  pub size_bytes: Option<u64>,
  #[serde(default)]
  pub sha256: Option<String>,
  #[serde(default)]
  pub created_at: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDebugPause {
  pub node_id: String,
  #[serde(default)]
  pub phase: String,
  #[serde(default)]
  pub next_nodes: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlowDebugSnapshot {
  #[serde(default)]
  pub step_every_node: bool,
  #[serde(default)]
  pub breakpoints: Vec<String>,
  #[serde(default)]
  pub paused_at: Option<FlowDebugPause>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugStatusResponse {
  #[serde(default)]
  pub run_id: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub active: bool,
  #[serde(default)]
  pub snapshot: Option<FlowDebugSnapshot>,
  #[serde(default)]
  pub resumed: Option<bool>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeToolCall {
  #[serde(default)]
  pub node_id: String,
  #[serde(default)]
  pub tool_call_id: String,
  #[serde(default)]
  pub tool: String,
  #[serde(default)]
  pub step_index: usize,
  #[serde(default)]
  pub idempotency: String,
  #[serde(default)]
  pub decision: String,
  #[serde(default)]
  pub reason: String,
  #[serde(default)]
  pub has_recorded_result: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumeSummary {
  #[serde(default)]
  pub total: usize,
  #[serde(default)]
  pub to_replay: usize,
  #[serde(default)]
  pub to_skip: usize,
  #[serde(default)]
  pub requires_manual: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResumePlan {
  #[serde(default)]
  pub schema_version: u32,
  #[serde(default)]
  pub workflow_id: String,
  #[serde(default)]
  pub last_completed_node: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub created_at: String,
  #[serde(default)]
  pub tool_calls: Vec<ResumeToolCall>,
  #[serde(default)]
  pub summary: ResumeSummary,
  #[serde(default)]
  pub force_replay: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingRunQuestion {
  pub question_id: String,
  #[serde(default)]
  pub node_id: String,
  #[serde(default)]
  pub question: String,
  #[serde(default)]
  pub created_at: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DebugResumeOverrides {
  pub input_override: Option<serde_json::Map<String, serde_json::Value>>,
  pub output_override: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct AttachRunResponse {
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub run_id: String,
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub attached_count: usize,
  #[serde(default)]
  pub artifacts: Vec<NotebookArtifact>,
}

// ── Live Telemetry & Event Reducer ───────────────────────────────────────────

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StudioNodeRunStatus {
  #[default]
  Idle,
  Running,
  PausedBefore,
  PausedAfter,
  Succeeded,
  Skipped,
  Failed,
}

impl StudioNodeRunStatus {
  pub const fn label(self) -> &'static str {
    match self {
      Self::Idle => "待命",
      Self::Running => "运行中…",
      Self::PausedBefore => "⏸ 执行前断点",
      Self::PausedAfter => "⏸ 单步暂停 · 待核查",
      Self::Succeeded => "✓ 已完成",
      Self::Skipped => "↷ 条件跳过",
      Self::Failed => "✕ 失败",
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeRuntimeTelemetry {
  pub status: StudioNodeRunStatus,
  pub duration_ms: Option<u64>,
  pub cost_usd: Option<f64>,
  pub has_input: bool,
  pub has_output: bool,
  pub error: Option<String>,
  pub skip_reason: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CostLimitExceededInfo {
  pub node_id: String,
  pub spent_usd: f64,
  pub limit_usd: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReducedWorkflowState {
  pub telemetry_by_node: HashMap<String, NodeRuntimeTelemetry>,
  pub node_inputs: HashMap<String, serde_json::Value>,
  pub node_outputs: HashMap<String, serde_json::Value>,
  pub paused_at: Option<FlowDebugPause>,
  pub workflow_error: Option<String>,
  pub workflow_duration_ms: Option<u64>,
  pub total_cost_usd: f64,
  pub cost_limit_exceeded: Option<CostLimitExceededInfo>,
}

pub fn format_spend_usd(usd: f64) -> String {
  if !usd.is_finite() || usd <= 0.0 {
    return "$0.00".to_string();
  }
  if usd < 0.01 {
    return format!("${usd:.4}");
  }
  if usd < 1.0 {
    return format!("${usd:.3}");
  }
  format!("${usd:.2}")
}

pub fn format_usd_badge(usd: f64) -> String {
  if usd > 0.0 && usd < 0.001 {
    return "<$0.001".to_string();
  }
  if usd < 0.1 {
    format!("${usd:.4}")
  } else {
    format!("${usd:.3}")
  }
}

pub fn reduce_workflow_events(events: &[StreamedEvent]) -> ReducedWorkflowState {
  let mut state = ReducedWorkflowState::default();

  for ev in events {
    let payload = ev.payload.as_object();
    let node_id = payload
      .and_then(|p| p.get("node_id"))
      .and_then(serde_json::Value::as_str);

    match ev.kind.as_str() {
      "workflow.started" | "checkpoint.restored" => {
        state.workflow_error = None;
        state.cost_limit_exceeded = None;
      }
      "node.started" => {
        if let Some(nid) = node_id {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = StudioNodeRunStatus::Running;
          entry.error = None;
        }
      }
      "node.input.captured" => {
        if let Some(nid) = node_id {
          if let Some(inp) = payload.and_then(|p| p.get("input")) {
            state.node_inputs.insert(nid.to_string(), inp.clone());
          }
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.has_input = true;
        }
      }
      "node.output.captured" => {
        if let Some(nid) = node_id {
          if let Some(out) = payload.and_then(|p| p.get("output")) {
            state.node_outputs.insert(nid.to_string(), out.clone());
          }
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.has_output = true;
        }
      }
      "node.completed" => {
        if let Some(nid) = node_id {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = StudioNodeRunStatus::Succeeded;
          if let Some(ms) = payload
            .and_then(|p| p.get("duration_ms"))
            .and_then(serde_json::Value::as_u64)
          {
            entry.duration_ms = Some(ms);
          }
        }
      }
      "node.cost" => {
        let cost = payload
          .and_then(|p| p.get("cost_usd"))
          .and_then(serde_json::Value::as_f64)
          .filter(|v| v.is_finite());
        let total = payload
          .and_then(|p| p.get("total_cost_usd"))
          .and_then(serde_json::Value::as_f64)
          .filter(|v| v.is_finite());
        if let (Some(nid), Some(c)) = (node_id, cost) {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.cost_usd = Some(c);
        }
        if let Some(t) = total {
          state.total_cost_usd = state.total_cost_usd.max(t);
        } else if let Some(c) = cost {
          state.total_cost_usd += c;
        }
      }
      "cost.limit_exceeded" => {
        let spent = payload
          .and_then(|p| p.get("spent_usd"))
          .and_then(serde_json::Value::as_f64)
          .filter(|v| v.is_finite())
          .unwrap_or(state.total_cost_usd);
        let limit = payload
          .and_then(|p| p.get("limit_usd"))
          .and_then(serde_json::Value::as_f64)
          .filter(|v| v.is_finite())
          .unwrap_or(0.0);
        state.total_cost_usd = state.total_cost_usd.max(spent);
        state.cost_limit_exceeded = Some(CostLimitExceededInfo {
          node_id: node_id.unwrap_or("unknown").to_string(),
          spent_usd: spent,
          limit_usd: limit,
        });
      }
      "node.skipped" => {
        if let Some(nid) = node_id {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = StudioNodeRunStatus::Skipped;
          entry.skip_reason = payload
            .and_then(|p| p.get("reason"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        }
      }
      "node.failed" => {
        if let Some(nid) = node_id {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = StudioNodeRunStatus::Failed;
          entry.error = payload
            .and_then(|p| p.get("error"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
          if let Some(ms) = payload
            .and_then(|p| p.get("duration_ms"))
            .and_then(serde_json::Value::as_u64)
          {
            entry.duration_ms = Some(ms);
          }
        }
      }
      "node.debug.paused" => {
        if let Some(nid) = node_id {
          let phase = if payload
            .and_then(|p| p.get("phase"))
            .and_then(serde_json::Value::as_str)
            == Some("before_node")
          {
            "before_node"
          } else {
            "after_node"
          };
          let next_nodes: Vec<String> = payload
            .and_then(|p| p.get("next_nodes"))
            .and_then(serde_json::Value::as_array)
            .map(|arr| {
              arr
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect()
            })
            .unwrap_or_default();
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = if phase == "before_node" {
            StudioNodeRunStatus::PausedBefore
          } else {
            StudioNodeRunStatus::PausedAfter
          };
          state.paused_at = Some(FlowDebugPause {
            node_id: nid.to_string(),
            phase: phase.to_string(),
            next_nodes,
          });
        }
      }
      "node.debug.resumed" => {
        if let Some(nid) = node_id {
          let entry = state.telemetry_by_node.entry(nid.to_string()).or_default();
          entry.status = if entry.status == StudioNodeRunStatus::PausedBefore {
            StudioNodeRunStatus::Running
          } else {
            StudioNodeRunStatus::Succeeded
          };
          if state
            .paused_at
            .as_ref()
            .is_some_and(|pause| pause.node_id == nid)
          {
            state.paused_at = None;
          }
        } else {
          state.paused_at = None;
        }
      }
      "workflow.completed" => {
        state.paused_at = None;
        state.workflow_error = None;
        if let Some(ms) = payload
          .and_then(|p| p.get("duration_ms"))
          .and_then(serde_json::Value::as_u64)
        {
          state.workflow_duration_ms = Some(ms);
        }
      }
      "workflow.failed" => {
        state.paused_at = None;
        if let Some(err) = payload
          .and_then(|p| p.get("error"))
          .and_then(serde_json::Value::as_str)
        {
          state.workflow_error = Some(err.to_string());
        }
        if let Some(ms) = payload
          .and_then(|p| p.get("duration_ms"))
          .and_then(serde_json::Value::as_u64)
        {
          state.workflow_duration_ms = Some(ms);
        }
      }
      "workflow.cancelled" => {
        state.paused_at = None;
      }
      _ => {}
    }
  }

  state
}

// ── Captured FlowValue Helpers ───────────────────────────────────────────────

pub fn unwrap_captured_flow_json(
  raw: Option<&serde_json::Value>,
) -> serde_json::Map<String, serde_json::Value> {
  let Some(obj) = raw.and_then(serde_json::Value::as_object) else {
    return serde_json::Map::new();
  };
  let mut out = serde_json::Map::new();
  for (key, val) in obj {
    if let Some(inner) = val.as_object()
      && inner.get("type").and_then(serde_json::Value::as_str) == Some("json")
      && let Some(unwrapped) = inner.get("value")
    {
      out.insert(key.clone(), unwrapped.clone());
      continue;
    }
    out.insert(key.clone(), val.clone());
  }
  out
}

pub fn parse_debug_override_object(
  raw_text: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
  let trimmed = raw_text.trim();
  if trimmed.is_empty() {
    return Err("覆盖载荷不能为空，请输入合法 JSON 对象".to_string());
  }
  let parsed: serde_json::Value =
    serde_json::from_str(trimmed).map_err(|err| format!("JSON 解析失败：{err}"))?;
  match parsed {
    serde_json::Value::Object(map) => Ok(map),
    _ => Err("覆盖载荷顶层必须是 JSON 对象（如 {\"output\": \"...\"}）".to_string()),
  }
}

pub fn extract_final_text_summary(
  execution_order: &[String],
  node_outputs: &HashMap<String, serde_json::Value>,
) -> Option<(String, String)> {
  for nid in execution_order.iter().rev() {
    let raw_out = node_outputs.get(nid)?;
    let unwrapped = unwrap_captured_flow_json(Some(raw_out));
    if let Some(text) = unwrapped
      .get("output")
      .and_then(serde_json::Value::as_str)
      .filter(|s| !s.trim().is_empty())
    {
      return Some((nid.clone(), text.to_string()));
    }
  }
  None
}

pub fn is_terminal_run_record(run: &RunRecord) -> bool {
  is_run_terminal(&run.status)
}

// ── Media Extractor & Deliverables Collector ─────────────────────────────────

fn looks_like_run_file_path(s: &str) -> bool {
  if s.is_empty() || s.len() > 1024 || s.contains('\n') {
    return false;
  }
  let lower = s.to_ascii_lowercase();
  if lower.starts_with("http://") || lower.starts_with("https://") {
    return false;
  }
  if s.contains("/artifacts/") || s.starts_with("artifacts/") {
    return classify_media_path(s, None).is_some();
  }
  (s.starts_with('/') || s.contains('/')) && classify_media_path(s, None).is_some()
}

fn basename_of(path: &str) -> String {
  path
    .replace('\\', "/")
    .split('/')
    .rfind(|seg| !seg.is_empty())
    .unwrap_or(path)
    .to_string()
}

pub fn build_preview_api_path(run_id: &str, file_path: &str) -> String {
  format!(
    "/v1/runs/{}/preview?path={}",
    encode_uri_component(run_id),
    encode_uri_component(file_path)
  )
}

fn node_media_priority(kind: MediaPreviewKind) -> u8 {
  match kind {
    MediaPreviewKind::Video => 0,
    MediaPreviewKind::Image => 1,
    MediaPreviewKind::Html => 2,
    MediaPreviewKind::Pdf => 3,
    MediaPreviewKind::Table => 4,
    MediaPreviewKind::Audio => 5,
    MediaPreviewKind::Markdown => 6,
    MediaPreviewKind::Document => 7,
    MediaPreviewKind::Subtitle => 8,
  }
}

fn deliverable_priority(kind: MediaPreviewKind) -> u8 {
  match kind {
    MediaPreviewKind::Video => 0,
    MediaPreviewKind::Html => 1,
    MediaPreviewKind::Pdf => 2,
    MediaPreviewKind::Table => 3,
    MediaPreviewKind::Document => 4,
    MediaPreviewKind::Markdown => 5,
    MediaPreviewKind::Image => 6,
    MediaPreviewKind::Audio => 7,
    MediaPreviewKind::Subtitle => 8,
  }
}

pub fn extract_node_media_previews(
  run_id: &str,
  node_id: &str,
  output: Option<&serde_json::Value>,
) -> Vec<MediaPreviewItem> {
  if run_id.is_empty() {
    return Vec::new();
  }
  let Some(val) = output else {
    return Vec::new();
  };

  let mut seen = HashSet::new();
  let mut items = Vec::new();
  let api_base = resolve_api_base();

  fn walk(
    val: &serde_json::Value,
    depth: usize,
    run_id: &str,
    node_id: &str,
    api_base: &str,
    seen: &mut HashSet<String>,
    items: &mut Vec<MediaPreviewItem>,
  ) {
    if depth > 8 {
      return;
    }
    let mut push = |file_path: &str, hint_label: String, mime: Option<&str>| {
      let trimmed = file_path.trim();
      if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
        return;
      }
      let Some(kind) = classify_media_path(trimmed, mime) else {
        return;
      };
      let api_path = build_preview_api_path(run_id, trimmed);
      items.push(MediaPreviewItem {
        key: format!("{node_id}:{trimmed}"),
        node_id: node_id.to_string(),
        label: if hint_label.is_empty() {
          basename_of(trimmed)
        } else {
          hint_label
        },
        kind,
        direct_url: join_api_url(api_base, &api_path),
        api_path,
        mime_type: mime.map(str::to_string),
        size_bytes: None,
      });
    };

    match val {
      serde_json::Value::String(s) => {
        if looks_like_run_file_path(s) {
          push(s, basename_of(s), None);
        }
      }
      serde_json::Value::Array(arr) => {
        for entry in arr {
          walk(entry, depth + 1, run_id, node_id, api_base, seen, items);
        }
      }
      serde_json::Value::Object(obj) => {
        if let Some(path) = obj.get("path").and_then(serde_json::Value::as_str) {
          let mime = obj.get("mime_type").and_then(serde_json::Value::as_str);
          if looks_like_run_file_path(path) || mime.is_some() {
            push(path, basename_of(path), mime);
          }
        }
        for (_, v) in obj {
          walk(v, depth + 1, run_id, node_id, api_base, seen, items);
        }
      }
      _ => {}
    }
  }

  walk(val, 0, run_id, node_id, &api_base, &mut seen, &mut items);
  items.sort_by(|a, b| {
    node_media_priority(a.kind)
      .cmp(&node_media_priority(b.kind))
      .then_with(|| a.key.cmp(&b.key))
  });
  items
}

pub fn collect_run_deliverables(
  run_id: &str,
  artifacts: &[ArtifactSummary],
  node_outputs: &HashMap<String, serde_json::Value>,
) -> Vec<MediaPreviewItem> {
  if run_id.is_empty() {
    return Vec::new();
  }
  let api_base = resolve_api_base();
  let mut results = Vec::new();
  let mut seen_names = HashSet::new();

  for art in artifacts {
    let kind = classify_media_path(&art.name, art.mime_type.as_deref())
      .unwrap_or(MediaPreviewKind::Document);
    let api_path = format!(
      "/v1/runs/{}/artifacts/{}",
      encode_uri_component(run_id),
      encode_uri_component(&art.id)
    );
    seen_names.insert(format!("{}:{}", art.node_id, art.name));
    results.push(MediaPreviewItem {
      key: format!("artifact:{}", art.id),
      node_id: art.node_id.clone(),
      label: art.name.clone(),
      kind,
      direct_url: join_api_url(&api_base, &append_inline_query(&api_path)),
      api_path,
      mime_type: art.mime_type.clone(),
      size_bytes: art.size_bytes,
    });
  }

  for (node_id, output) in node_outputs {
    for item in extract_node_media_previews(run_id, node_id, Some(output)) {
      let dedup_key = format!("{node_id}:{}", item.label);
      if seen_names.contains(&dedup_key) {
        if matches!(
          item.kind,
          MediaPreviewKind::Video
            | MediaPreviewKind::Image
            | MediaPreviewKind::Html
            | MediaPreviewKind::Pdf
            | MediaPreviewKind::Audio
        ) && let Some(existing) = results
          .iter_mut()
          .find(|r| r.node_id == *node_id && r.label == item.label)
        {
          existing.api_path = item.api_path;
          existing.direct_url = item.direct_url;
        }
        continue;
      }
      seen_names.insert(dedup_key);
      results.push(item);
    }
  }

  results.sort_by(|a, b| {
    deliverable_priority(a.kind)
      .cmp(&deliverable_priority(b.kind))
      .then_with(|| a.label.cmp(&b.label))
  });
  results
}

// ── Classic Console Event Filter & Trace Comparison Helpers ──────────────────

pub fn event_node_id(event: &StreamedEvent) -> Option<String> {
  let obj = event.payload.as_object()?;
  obj
    .get("node_id")
    .or_else(|| obj.get("node"))
    .and_then(serde_json::Value::as_str)
    .map(str::to_string)
}

pub fn event_tone(kind: &str) -> &'static str {
  let lower = kind.to_ascii_lowercase();
  if lower.contains("fail") || lower.contains("error") || lower.contains("exceeded") {
    "error"
  } else if lower.contains("complete") || lower.contains("succeed") {
    "ok"
  } else if lower.contains("tool") {
    "tool"
  } else if lower.contains("agent") || lower.contains("llm") {
    "agent"
  } else if lower.contains("pause") || lower.contains("approval") || lower.contains("question") {
    "warn"
  } else {
    "neutral"
  }
}

/// Evaluates lightweight console filter expressions such as:
/// `kind=node.completed AND seq>2` or plain substring matches.
pub fn matches_event_filter(event: &StreamedEvent, expr: &str) -> bool {
  let trimmed = expr.trim();
  if trimmed.is_empty() {
    return true;
  }
  for clause in trimmed.split(" AND ") {
    let clause = clause.trim();
    if clause.is_empty() {
      continue;
    }
    if let Some((lhs, rhs)) = clause.split_once(">=") {
      let Some(val) = extract_numeric_field(event, lhs.trim()) else {
        return false;
      };
      let Ok(target) = rhs.trim().parse::<f64>() else {
        return false;
      };
      if val < target {
        return false;
      }
    } else if let Some((lhs, rhs)) = clause.split_once("<=") {
      let Some(val) = extract_numeric_field(event, lhs.trim()) else {
        return false;
      };
      let Ok(target) = rhs.trim().parse::<f64>() else {
        return false;
      };
      if val > target {
        return false;
      }
    } else if let Some((lhs, rhs)) = clause.split_once('>') {
      let Some(val) = extract_numeric_field(event, lhs.trim()) else {
        return false;
      };
      let Ok(target) = rhs.trim().parse::<f64>() else {
        return false;
      };
      if val <= target {
        return false;
      }
    } else if let Some((lhs, rhs)) = clause.split_once('<') {
      let Some(val) = extract_numeric_field(event, lhs.trim()) else {
        return false;
      };
      let Ok(target) = rhs.trim().parse::<f64>() else {
        return false;
      };
      if val >= target {
        return false;
      }
    } else if let Some((lhs, rhs)) = clause.split_once("!=") {
      let actual = extract_string_field(event, lhs.trim()).unwrap_or_default();
      if actual.eq_ignore_ascii_case(rhs.trim().trim_matches('"')) {
        return false;
      }
    } else if let Some((lhs, rhs)) = clause.split_once('=') {
      let actual = extract_string_field(event, lhs.trim()).unwrap_or_default();
      let want = rhs.trim().trim_matches('"');
      if !actual.to_ascii_lowercase().contains(&want.to_ascii_lowercase()) {
        return false;
      }
    } else {
      let needle = clause.to_ascii_lowercase();
      let in_kind = event.kind.to_ascii_lowercase().contains(&needle);
      let in_node = event_node_id(event)
        .is_some_and(|n| n.to_ascii_lowercase().contains(&needle));
      let in_payload = event.payload.to_string().to_ascii_lowercase().contains(&needle);
      if !in_kind && !in_node && !in_payload {
        return false;
      }
    }
  }
  true
}

fn extract_numeric_field(event: &StreamedEvent, field: &str) -> Option<f64> {
  match field {
    "seq" => Some(event.seq as f64),
    "step" | "step_index" => event
      .payload
      .as_object()
      .and_then(|o| o.get("step_index").or_else(|| o.get("step")))
      .and_then(serde_json::Value::as_f64),
    other => event
      .payload
      .as_object()
      .and_then(|o| o.get(other))
      .and_then(serde_json::Value::as_f64),
  }
}

fn extract_string_field(event: &StreamedEvent, field: &str) -> Option<String> {
  match field {
    "kind" => Some(event.kind.clone()),
    "node" | "node_id" => event_node_id(event),
    other => event
      .payload
      .as_object()
      .and_then(|o| o.get(other))
      .map(|v| v.as_str().map(str::to_string).unwrap_or_else(|| v.to_string())),
  }
}

pub fn compare_key(event: &StreamedEvent) -> String {
  let step = event
    .payload
    .as_object()
    .and_then(|o| o.get("step_index"))
    .and_then(serde_json::Value::as_i64)
    .unwrap_or(event.seq);
  format!("{}#{step}", event.kind)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RunCompareSummary {
  pub event_count: usize,
  pub tool_call_count: usize,
  pub final_answer: Option<String>,
  pub node_count: usize,
}

pub fn summarise_events(events: &[StreamedEvent]) -> RunCompareSummary {
  let tool_call_count = events
    .iter()
    .filter(|e| e.kind.to_ascii_lowercase().contains("tool_call"))
    .count();
  let nodes: HashSet<String> = events.iter().filter_map(event_node_id).collect();
  let final_answer = events.iter().rev().find_map(|e| {
    let k = e.kind.to_ascii_lowercase();
    if matches!(
      k.as_str(),
      "run_completed" | "final_answer" | "stopped" | "workflow.completed"
    ) {
      let obj = e.payload.as_object()?;
      obj
        .get("answer")
        .or_else(|| obj.get("final_answer"))
        .or_else(|| obj.get("message"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
    } else {
      None
    }
  });
  RunCompareSummary {
    event_count: events.len(),
    tool_call_count,
    final_answer,
    node_count: nodes.len(),
  }
}

// ── Async REST Client ────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ListRunsEnvelope {
  #[serde(default)]
  runs: Vec<RunRecord>,
}

#[derive(Deserialize)]
struct RunEnvelope {
  #[serde(default)]
  run: Option<RunRecord>,
  #[serde(default)]
  id: Option<String>,
  #[serde(default)]
  workflow: Option<String>,
  #[serde(default)]
  status: Option<String>,
  #[serde(default)]
  tenant_id: Option<String>,
  #[serde(default)]
  started_at: Option<String>,
  #[serde(default)]
  finished_at: Option<String>,
  #[serde(default)]
  run_dir: Option<String>,
  #[serde(default)]
  error: Option<String>,
}

impl RunEnvelope {
  fn into_record(self) -> RunRecord {
    if let Some(run) = self.run {
      return run;
    }
    RunRecord {
      id: self.id.unwrap_or_default(),
      workflow: self.workflow.unwrap_or_default(),
      status: self.status.unwrap_or_default(),
      tenant_id: self.tenant_id,
      started_at: self.started_at,
      finished_at: self.finished_at,
      run_dir: self.run_dir,
      error: self.error,
    }
  }
}

#[derive(Deserialize)]
struct CreateRunEnvelope {
  run_id: String,
}

#[derive(Deserialize)]
struct ListArtifactsEnvelope {
  #[serde(default)]
  artifacts: Vec<ArtifactSummary>,
}

#[derive(Deserialize)]
struct RunApprovalsEnvelope {
  #[serde(default)]
  approvals: Vec<PendingApproval>,
}

#[derive(Deserialize)]
struct RunQuestionsEnvelope {
  #[serde(default)]
  pending: Vec<PendingRunQuestion>,
}

#[derive(Deserialize)]
struct PreferencesEnvelope {
  #[serde(default)]
  preferences: Option<serde_json::Value>,
}

pub async fn inspect_workflow_server(
  workflow_yaml: &str,
  token: &str,
  tenant: &str,
) -> Result<InspectWorkflowResponse, String> {
  let body = serde_json::json!({ "workflow": workflow_yaml });
  request_json("POST", "/v1/workflows/inspect", token, tenant, Some(&body)).await
}

pub async fn list_runs(tenant: &str, limit: usize, token: &str) -> Result<Vec<RunRecord>, String> {
  let t = if tenant.trim().is_empty() {
    "default"
  } else {
    tenant.trim()
  };
  let path = format!("/v1/runs?tenant_id={}&limit={limit}", encode_uri_component(t));
  let env: ListRunsEnvelope = request_json("GET", &path, token, t, None).await?;
  Ok(env.runs)
}

pub async fn get_run(run_id: &str, tenant: &str, token: &str) -> Result<RunRecord, String> {
  let path = format!("/v1/runs/{}", encode_uri_component(run_id));
  let env: RunEnvelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.into_record())
}

#[derive(Clone, Debug)]
pub struct LaunchStudioRunRequest {
  pub tenant_id: String,
  pub workflow_yaml: String,
  pub inputs: serde_json::Map<String, serde_json::Value>,
  pub model: Option<String>,
  pub cost_limit_usd: Option<f64>,
  pub run_mode: String,
  pub step_every_node: bool,
  pub breakpoints: Vec<String>,
}

pub async fn launch_studio_run(req: &LaunchStudioRunRequest, token: &str) -> Result<String, String> {
  let tenant = if req.tenant_id.trim().is_empty() {
    "default"
  } else {
    req.tenant_id.trim()
  };
  let mut body = serde_json::Map::new();
  body.insert("tenant_id".to_string(), serde_json::json!(tenant));
  body.insert("workflow".to_string(), serde_json::json!(req.workflow_yaml));
  body.insert(
    "inputs".to_string(),
    serde_json::Value::Object(req.inputs.clone()),
  );
  if let Some(m) = req.model.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
    body.insert("model".to_string(), serde_json::json!(m));
  }
  if let Some(limit) = req.cost_limit_usd {
    body.insert("cost_limit_usd".to_string(), serde_json::json!(limit));
  }
  body.insert("run_mode".to_string(), serde_json::json!(req.run_mode));
  body.insert(
    "step_every_node".to_string(),
    serde_json::json!(req.step_every_node),
  );
  body.insert(
    "breakpoints".to_string(),
    serde_json::json!(req.breakpoints),
  );

  let payload = serde_json::Value::Object(body);
  let created: CreateRunEnvelope =
    request_json("POST", "/v1/runs", token, tenant, Some(&payload)).await?;
  Ok(created.run_id)
}

pub async fn cancel_run(run_id: &str, tenant: &str, token: &str) -> Result<RunRecord, String> {
  let path = format!("/v1/runs/{}:cancel", encode_uri_component(run_id));
  let env: RunEnvelope = request_json("POST", &path, token, tenant, None).await?;
  Ok(env.into_record())
}

pub async fn get_run_events_history(
  run_id: &str,
  limit: Option<usize>,
  tenant: &str,
  token: &str,
) -> Result<Vec<StreamedEvent>, String> {
  let path = match limit {
    Some(n) => format!(
      "/v1/runs/{}/events/history?limit={n}",
      encode_uri_component(run_id)
    ),
    None => format!("/v1/runs/{}/events/history", encode_uri_component(run_id)),
  };
  let raw: serde_json::Value = request_json("GET", &path, token, tenant, None).await?;
  if let Some(arr) = raw.as_array() {
    serde_json::from_value(serde_json::Value::Array(arr.clone()))
      .map_err(|err| format!("parse events history: {err}"))
  } else if let Some(events) = raw.get("events").and_then(serde_json::Value::as_array) {
    serde_json::from_value(serde_json::Value::Array(events.clone()))
      .map_err(|err| format!("parse events envelope: {err}"))
  } else {
    Ok(Vec::new())
  }
}

pub async fn get_run_artifacts(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<Vec<ArtifactSummary>, String> {
  let path = format!("/v1/runs/{}/artifacts", encode_uri_component(run_id));
  let env: ListArtifactsEnvelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.artifacts)
}

pub async fn get_debug_status(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<DebugStatusResponse, String> {
  let path = format!("/v1/runs/{}/debug", encode_uri_component(run_id));
  request_json("GET", &path, token, tenant, None).await
}

pub async fn send_debug_command(
  run_id: &str,
  payload: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<DebugStatusResponse, String> {
  let path = format!("/v1/runs/{}:debug", encode_uri_component(run_id));
  request_json("POST", &path, token, tenant, Some(payload)).await
}

pub async fn get_resume_plan(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<ResumePlan, String> {
  let path = format!("/v1/runs/{}/resume-plan", encode_uri_component(run_id));
  request_json("GET", &path, token, tenant, None).await
}

pub async fn resume_run(run_id: &str, tenant: &str, token: &str) -> Result<RunRecord, String> {
  let path = format!("/v1/runs/{}:resume", encode_uri_component(run_id));
  let env: RunEnvelope = request_json("POST", &path, token, tenant, None).await?;
  Ok(env.into_record())
}

pub async fn list_run_approvals(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<Vec<PendingApproval>, String> {
  let path = format!("/v1/runs/{}/approvals", encode_uri_component(run_id));
  let env: RunApprovalsEnvelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.approvals)
}

pub async fn decide_run_approval(
  run_id: &str,
  request_id: &str,
  decision: &str,
  scope: &str,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let path = format!(
    "/v1/runs/{}/approvals/{}",
    encode_uri_component(run_id),
    encode_uri_component(request_id)
  );
  let body = serde_json::json!({
    "decision": decision,
    "scope": scope,
    "decided_by": "user:studio"
  });
  let _: serde_json::Value = request_json("POST", &path, token, tenant, Some(&body)).await?;
  Ok(())
}

pub async fn list_run_questions(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<Vec<PendingRunQuestion>, String> {
  let path = format!("/v1/runs/{}/questions", encode_uri_component(run_id));
  let env: RunQuestionsEnvelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.pending)
}

pub async fn answer_run_question(
  run_id: &str,
  question_id: &str,
  answer: &str,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let path = format!(
    "/v1/runs/{}/questions/{}",
    encode_uri_component(run_id),
    encode_uri_component(question_id)
  );
  let body = serde_json::json!({ "answer": answer });
  let _: serde_json::Value = request_json("POST", &path, token, tenant, Some(&body)).await?;
  Ok(())
}

pub async fn attach_run_to_notebook(
  notebook_id: &str,
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<AttachRunResponse, String> {
  let path = format!(
    "/v1/notebooks/{}/artifacts",
    encode_uri_component(notebook_id)
  );
  let body = serde_json::json!({ "run_id": run_id });
  request_json("POST", &path, token, tenant, Some(&body)).await
}

pub async fn fetch_preferences(
  tenant: &str,
  token: &str,
) -> Result<Option<serde_json::Value>, String> {
  let env: PreferencesEnvelope = request_json("GET", "/v1/preferences", token, tenant, None).await?;
  Ok(env.preferences)
}

pub async fn put_preferences(
  patch: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let body = serde_json::json!({ "preferences": patch });
  let _: serde_json::Value = request_json("PUT", "/v1/preferences", token, tenant, Some(&body)).await?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn reducer_tracks_debug_pause_costs_io_and_completion() {
    let events = vec![
      StreamedEvent {
        run_id: "r1".into(),
        seq: 1,
        kind: "workflow.started".into(),
        payload: serde_json::json!({}),
        ts: "2026-11-26T00:00:00Z".into(),
      },
      StreamedEvent {
        run_id: "r1".into(),
        seq: 2,
        kind: "node.started".into(),
        payload: serde_json::json!({ "node_id": "n1" }),
        ts: "2026-11-26T00:00:01Z".into(),
      },
      StreamedEvent {
        run_id: "r1".into(),
        seq: 3,
        kind: "node.input.captured".into(),
        payload: serde_json::json!({ "node_id": "n1", "input": { "topic": { "type": "json", "value": "Rust" } } }),
        ts: "2026-11-26T00:00:01Z".into(),
      },
      StreamedEvent {
        run_id: "r1".into(),
        seq: 4,
        kind: "node.output.captured".into(),
        payload: serde_json::json!({ "node_id": "n1", "output": { "output": { "type": "json", "value": "Done" } } }),
        ts: "2026-11-26T00:00:02Z".into(),
      },
      StreamedEvent {
        run_id: "r1".into(),
        seq: 5,
        kind: "node.cost".into(),
        payload: serde_json::json!({ "node_id": "n1", "cost_usd": 0.0025, "total_cost_usd": 0.0025 }),
        ts: "2026-11-26T00:00:02Z".into(),
      },
      StreamedEvent {
        run_id: "r1".into(),
        seq: 6,
        kind: "node.debug.paused".into(),
        payload: serde_json::json!({ "node_id": "n1", "phase": "after_node", "next_nodes": ["n2"] }),
        ts: "2026-11-26T00:00:02Z".into(),
      },
    ];

    let reduced = reduce_workflow_events(&events);
    let t = reduced.telemetry_by_node.get("n1").expect("telemetry");
    assert_eq!(t.status, StudioNodeRunStatus::PausedAfter);
    assert_eq!(t.cost_usd, Some(0.0025));
    assert!(t.has_input && t.has_output);
    assert_eq!(reduced.total_cost_usd, 0.0025);
    assert_eq!(
      reduced.paused_at.as_ref().map(|p| p.node_id.as_str()),
      Some("n1")
    );

    let summary = extract_final_text_summary(&["n1".into()], &reduced.node_outputs);
    assert_eq!(summary, Some(("n1".into(), "Done".into())));
  }

  #[test]
  fn media_extractor_finds_nested_files_and_upgrades_inline_preview() {
    let output = serde_json::json!({
      "slides_html": { "type": "file", "path": "/tmp/run1/artifacts/deck.html", "mime_type": "text/html" },
      "frames": ["/tmp/run1/artifacts/slide_01.png"]
    });
    let previews = extract_node_media_previews("run1", "render_preview", Some(&output));
    assert_eq!(previews.len(), 2);
    assert_eq!(previews[0].kind, MediaPreviewKind::Image);
    assert_eq!(previews[1].kind, MediaPreviewKind::Html);

    let artifacts = vec![ArtifactSummary {
      id: "art-1".into(),
      node_id: "render_preview".into(),
      name: "deck.html".into(),
      mime_type: Some("text/html".into()),
      size_bytes: Some(2048),
      ..Default::default()
    }];
    let mut outputs = HashMap::new();
    outputs.insert("render_preview".into(), output);
    let deliverables = collect_run_deliverables("run1", &artifacts, &outputs);
    assert_eq!(deliverables.len(), 2);
    let html_item = deliverables
      .iter()
      .find(|d| d.label == "deck.html")
      .expect("deck.html");
    assert!(html_item.api_path.contains("/preview?path="));
  }

  #[test]
  fn console_filter_and_compare_summary_work() {
    let ev = StreamedEvent {
      run_id: "r1".into(),
      seq: 7,
      kind: "tool_call_completed".into(),
      payload: serde_json::json!({ "node_id": "agent_1", "step_index": 3 }),
      ts: "2026-11-26T00:00:05Z".into(),
    };
    assert!(matches_event_filter(&ev, "kind=tool_call AND step>2"));
    assert!(!matches_event_filter(&ev, "kind=tool_call AND step>5"));
    assert_eq!(compare_key(&ev), "tool_call_completed#3");
    let sum = summarise_events(&[ev]);
    assert_eq!(sum.tool_call_count, 1);
    assert_eq!(sum.node_count, 1);
  }
}
