//! Typed browser API bridge, SSE streaming, localStorage persistence, and
//! route helpers for Yanshi Studio's Dioxus 0.7 frontend.

use serde::{Deserialize, Serialize};

// ── Models & Providers ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderConfigItem {
  pub name: String,
  #[serde(default)]
  pub protocol: String,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub default_base_url: Option<String>,
  #[serde(default)]
  pub api_key_env: String,
  #[serde(default)]
  pub api_key_set: bool,
  #[serde(default)]
  pub api_key_masked: Option<String>,
  #[serde(default)]
  pub api_key_source: String,
  #[serde(default)]
  pub timeout_seconds: Option<u64>,
  #[serde(default)]
  pub requests_per_minute: Option<u32>,
  #[serde(default)]
  pub is_custom: bool,
  #[serde(default)]
  pub is_overridden: bool,
  #[serde(default)]
  pub model_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelConfigItem {
  pub name: String,
  #[serde(default)]
  pub vendor: String,
  #[serde(default)]
  pub model_id: String,
  #[serde(default)]
  pub model_type: String,
  #[serde(default)]
  pub accepts: Vec<String>,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub effective_base_url: Option<String>,
  #[serde(default)]
  pub temperature: Option<f64>,
  #[serde(default)]
  pub max_tokens: Option<u32>,
  #[serde(default)]
  pub supports_streaming: bool,
  #[serde(default)]
  pub supports_tools: bool,
  #[serde(default)]
  pub native_tool_calling: bool,
  #[serde(default)]
  pub supports_thinking: bool,
  #[serde(default)]
  pub thinking_kind: Option<String>,
  #[serde(default)]
  pub is_custom: bool,
  #[serde(default)]
  pub is_overridden: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ModelConfigSnapshot {
  #[serde(default)]
  pub config_dir: String,
  #[serde(default)]
  pub models_path: String,
  #[serde(default)]
  pub env_path: String,
  #[serde(default)]
  pub providers: Vec<ProviderConfigItem>,
  #[serde(default)]
  pub models: Vec<ModelConfigItem>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TestProviderOutcome {
  pub ok: bool,
  #[serde(default)]
  pub provider: String,
  #[serde(default)]
  pub protocol: String,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub latency_ms: u64,
  #[serde(default)]
  pub error: Option<String>,
}

// ── Diagnostics ──────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsConfigSection {
  #[serde(default)]
  pub error: Option<String>,
  #[serde(default)]
  pub missing_env_vars: Vec<String>,
  #[serde(default)]
  pub models: Option<usize>,
  #[serde(default)]
  pub providers: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsSecuritySection {
  #[serde(default)]
  pub profile: Option<String>,
  #[serde(default)]
  pub warning: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsSandboxSection {
  #[serde(default)]
  pub enforcing: bool,
  #[serde(default)]
  pub backend: Option<String>,
  #[serde(default)]
  pub enforcement: Option<String>,
  #[serde(default)]
  pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsDirCheck {
  #[serde(default)]
  pub path: String,
  #[serde(default)]
  pub source: String,
  #[serde(default)]
  pub exists: bool,
  #[serde(default)]
  pub writable: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsDiskSection {
  #[serde(default)]
  pub run_dir: Option<DiagnosticsDirCheck>,
  #[serde(default)]
  pub trace_dir: Option<DiagnosticsDirCheck>,
  #[serde(default)]
  pub marketplace_cache: Option<DiagnosticsDirCheck>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsEnvSection {
  #[serde(default)]
  pub yanshi_api_token_set: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticsReport {
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub version: Option<String>,
  #[serde(default)]
  pub profile: Option<String>,
  #[serde(default)]
  pub config: Option<DiagnosticsConfigSection>,
  #[serde(default)]
  pub security: Option<DiagnosticsSecuritySection>,
  #[serde(default)]
  pub sandbox: Option<DiagnosticsSandboxSection>,
  #[serde(default)]
  pub disk: Option<DiagnosticsDiskSection>,
  #[serde(default)]
  pub environment: Option<DiagnosticsEnvSection>,
}

// ── Harness Sessions ─────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessSession {
  pub id: String,
  #[serde(default)]
  pub tenant_id: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub profile: String,
  #[serde(default)]
  pub runtime_kind: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub skill_name: Option<String>,
  #[serde(default)]
  pub workspace_root: String,
  #[serde(default)]
  pub user_input: String,
  #[serde(default)]
  pub started_at: Option<String>,
  #[serde(default)]
  pub finished_at: Option<String>,
  #[serde(default)]
  pub final_answer: Option<String>,
  #[serde(default)]
  pub error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct HarnessEvent {
  #[serde(default)]
  pub seq: i64,
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub ts: Option<String>,
  #[serde(default)]
  pub payload: serde_json::Value,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PendingApproval {
  pub id: String,
  #[serde(default)]
  pub session_id: String,
  #[serde(default)]
  pub tool: String,
  #[serde(default)]
  pub risk: String,
  #[serde(default)]
  pub reason: String,
  #[serde(default)]
  pub step_index: usize,
  #[serde(default)]
  pub idempotency: Option<String>,
  #[serde(default)]
  pub requested_at: Option<String>,
  #[serde(default)]
  pub params_summary: serde_json::Value,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingInterrupt {
  #[serde(default)]
  pub step_index: usize,
  #[serde(default)]
  pub question: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SseFrame {
  pub event: String,
  pub data: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlobPreviewBundle {
  pub svg_url: String,
  pub html_url: String,
}

// ── Pure helpers (shared with host unit tests) ───────────────────────────────

/// Pure SSE chunk parser shared across WASM and host unit tests.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn parse_sse_chunk(pending: &mut String, chunk: &str) -> Vec<SseFrame> {
  pending.push_str(&chunk.replace("\r\n", "\n"));
  let mut frames = Vec::new();
  while let Some(sep_idx) = pending.find("\n\n") {
    let block = pending[..sep_idx].to_string();
    pending.drain(..sep_idx + 2);
    let mut event_name = String::from("message");
    let mut data_lines = Vec::new();
    for line in block.lines() {
      if let Some(rest) = line.strip_prefix("event:") {
        let trimmed = rest.trim();
        if !trimmed.is_empty() {
          event_name = trimmed.to_string();
        }
      } else if let Some(rest) = line.strip_prefix("data:") {
        data_lines.push(rest.trim_start().to_string());
      }
    }
    if !data_lines.is_empty() {
      frames.push(SseFrame {
        event: event_name,
        data: data_lines.join("\n"),
      });
    }
  }
  frames
}

/// Normalize an API base URL and path into a fetchable URL string.
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub fn join_api_url(base: &str, path: &str) -> String {
  if path.starts_with("http://") || path.starts_with("https://") {
    return path.to_string();
  }
  let trimmed_base = base.trim().trim_end_matches('/');
  let normalized_path = if path.starts_with('/') {
    path.to_string()
  } else {
    format!("/{path}")
  };
  if trimmed_base.is_empty() {
    normalized_path
  } else {
    format!("{trimmed_base}{normalized_path}")
  }
}

/// Minimal RFC 3986 path-segment percent encoder.
pub fn encode_uri_component(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len());
  for byte in raw.as_bytes() {
    if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
      out.push(char::from(*byte));
    } else {
      out.push_str(&format!("%{byte:02X}"));
    }
  }
  out
}

/// Mask an API bearer token down to its last 4 characters.
pub fn mask_token(raw: &str) -> String {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return "(none)".to_string();
  }
  let chars: Vec<char> = trimmed.chars().collect();
  if chars.len() <= 4 {
    return format!("••{trimmed}");
  }
  let tail: String = chars[chars.len() - 4..].iter().collect();
  format!("••••{tail}")
}

/// True when a Harness session status is terminal.
pub fn is_harness_terminal(status: &str) -> bool {
  matches!(
    status.to_ascii_lowercase().as_str(),
    "completed" | "failed" | "cancelled" | "canceled" | "stopped"
  )
}

/// Extract streaming token delta text from a `token_delta` Harness event payload.
pub fn extract_token_delta(payload: &serde_json::Value) -> Option<String> {
  payload
    .get("delta")
    .or_else(|| payload.get("text"))
    .or_else(|| payload.get("token"))
    .and_then(|v| v.as_str())
    .map(ToString::to_string)
}

// ── Browser / WASM implementation ────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
mod wasm_impl {
  use super::*;
  use js_sys::{Array, Reflect, Uint8Array};
  use wasm_bindgen::JsCast;
  use wasm_bindgen::JsValue;
  use wasm_bindgen_futures::JsFuture;
  use web_sys::{
    AbortController, Blob, BlobPropertyBag, Headers, ReadableStreamDefaultReader, Request,
    RequestInit, RequestMode, Response, TextDecoder, Url,
  };

  fn js_err(err: JsValue) -> String {
    err
      .as_string()
      .or_else(|| {
        Reflect::get(&err, &JsValue::from_str("message"))
          .ok()
          .and_then(|v| v.as_string())
      })
      .unwrap_or_else(|| format!("{err:?}"))
  }

  fn window_string_prop(key: &str) -> Option<String> {
    let win = web_sys::window()?;
    let val = Reflect::get(&win, &JsValue::from_str(key)).ok()?;
    val
      .as_string()
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
  }

  pub fn resolve_api_base() -> String {
    let stored = read_storage("yanshi.ui.apiBaseUrl", "");
    if !stored.trim().is_empty() {
      return stored.trim().trim_end_matches('/').to_string();
    }
    window_string_prop("__YANSHI_API_BASE__").unwrap_or_default()
  }

  pub fn resolve_embedded_token() -> String {
    window_string_prop("__YANSHI_API_TOKEN__").unwrap_or_default()
  }

  pub fn is_local_desktop_engine() -> bool {
    let Some(win) = web_sys::window() else {
      return false;
    };
    let has_tauri = Reflect::get(&win, &JsValue::from_str("__TAURI_INTERNALS__"))
      .ok()
      .is_some_and(|v| !v.is_undefined() && !v.is_null());
    has_tauri || !resolve_api_base().is_empty() || !resolve_embedded_token().is_empty()
  }

  pub fn read_storage(key: &str, fallback: &str) -> String {
    let Some(win) = web_sys::window() else {
      return fallback.to_string();
    };
    let Ok(Some(storage)) = win.local_storage() else {
      return fallback.to_string();
    };
    storage
      .get_item(key)
      .ok()
      .flatten()
      .unwrap_or_else(|| fallback.to_string())
  }

  pub fn write_storage(key: &str, value: &str) {
    if let Some(win) = web_sys::window()
      && let Ok(Some(storage)) = win.local_storage()
    {
      let _ = storage.set_item(key, value);
    }
  }

  pub fn current_location_path_and_search() -> (String, String) {
    let Some(win) = web_sys::window() else {
      return ("/ui-dx/diagnostics".to_string(), String::new());
    };
    let loc = win.location();
    let hash = loc.hash().unwrap_or_default();
    if let Some(stripped) = hash.strip_prefix("#/") {
      let full = format!("/{stripped}");
      if let Some((p, s)) = full.split_once('?') {
        return (p.to_string(), format!("?{s}"));
      }
      return (full, String::new());
    }
    let path = loc.pathname().unwrap_or_else(|_| "/ui-dx".to_string());
    let search = loc.search().unwrap_or_default();
    (path, search)
  }

  pub fn push_browser_route(path_with_search: &str) {
    let Some(win) = web_sys::window() else {
      return;
    };
    let use_hash = win
      .location()
      .protocol()
      .map(|p| p == "tauri:" || p == "file:")
      .unwrap_or(false);
    if use_hash {
      let target = format!("#{}", path_with_search);
      let _ = win.location().set_hash(&target);
      return;
    }
    if let Ok(history) = win.history() {
      let _ = history.push_state_with_url(&JsValue::NULL, "", Some(path_with_search));
    }
  }

  fn build_headers(
    token: &str,
    tenant: &str,
    content_type: Option<&str>,
    accept: Option<&str>,
  ) -> Result<Headers, String> {
    let headers = Headers::new().map_err(js_err)?;
    let effective_token = if token.trim().is_empty() {
      resolve_embedded_token()
    } else {
      token.trim().to_string()
    };
    if !effective_token.is_empty() {
      headers
        .set("Authorization", &format!("Bearer {effective_token}"))
        .map_err(js_err)?;
    }
    let trimmed_tenant = tenant.trim();
    if !trimmed_tenant.is_empty() {
      headers
        .set("X-Yanshi-Tenant", trimmed_tenant)
        .map_err(js_err)?;
    }
    if let Some(ct) = content_type {
      headers.set("Content-Type", ct).map_err(js_err)?;
    }
    if let Some(acc) = accept {
      headers.set("Accept", acc).map_err(js_err)?;
    }
    Ok(headers)
  }

  async fn send_request(
    method: &str,
    path: &str,
    token: &str,
    tenant: &str,
    json_body: Option<&str>,
    accept: Option<&str>,
    signal: Option<&web_sys::AbortSignal>,
  ) -> Result<Response, String> {
    let win = web_sys::window().ok_or_else(|| "window is unavailable".to_string())?;
    let url = join_api_url(&resolve_api_base(), path);
    let init = RequestInit::new();
    init.set_method(method);
    init.set_mode(RequestMode::Cors);
    let headers = build_headers(token, tenant, json_body.map(|_| "application/json"), accept)?;
    init.set_headers(&headers);
    if let Some(body) = json_body {
      init.set_body(&JsValue::from_str(body));
    }
    if let Some(sig) = signal {
      init.set_signal(Some(sig));
    }
    let request = Request::new_with_str_and_init(&url, &init).map_err(js_err)?;
    let resp_value = JsFuture::from(win.fetch_with_request(&request))
      .await
      .map_err(js_err)?;
    resp_value
      .dyn_into::<Response>()
      .map_err(|_| "unexpected non-Response value from fetch".to_string())
  }

  async fn read_response_text(resp: &Response) -> Result<String, String> {
    let promise = resp.text().map_err(js_err)?;
    Ok(
      JsFuture::from(promise)
        .await
        .map_err(js_err)?
        .as_string()
        .unwrap_or_default(),
    )
  }

  async fn request_json<T: for<'de> Deserialize<'de>>(
    method: &str,
    path: &str,
    token: &str,
    tenant: &str,
    payload: Option<&serde_json::Value>,
  ) -> Result<T, String> {
    let body_str = match payload {
      Some(val) => Some(serde_json::to_string(val).map_err(|e| e.to_string())?),
      None => None,
    };
    let resp = send_request(method, path, token, tenant, body_str.as_deref(), None, None).await?;
    let status = resp.status();
    let text = read_response_text(&resp).await?;
    if !resp.ok() {
      return Err(format!("HTTP {status}: {text}"));
    }
    serde_json::from_str::<T>(&text).map_err(|err| format!("{method} {path} invalid JSON: {err}"))
  }

  pub async fn fetch_models_config(
    token: &str,
    tenant: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    request_json("GET", "/v1/models/config", token, tenant, None).await
  }

  pub async fn put_provider(
    name: &str,
    payload: &serde_json::Value,
    token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    let path = format!("/v1/models/providers/{}", encode_uri_component(name));
    request_json("PUT", &path, token, "", Some(payload)).await
  }

  pub async fn delete_provider(
    name: &str,
    clear_api_key: bool,
    token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    let path = format!(
      "/v1/models/providers/{}?clear_api_key={}",
      encode_uri_component(name),
      if clear_api_key { "true" } else { "false" }
    );
    request_json("DELETE", &path, token, "", None).await
  }

  pub async fn test_provider(
    name: &str,
    payload: &serde_json::Value,
    token: &str,
  ) -> Result<TestProviderOutcome, String> {
    let path = format!("/v1/models/providers/{}/test", encode_uri_component(name));
    request_json("POST", &path, token, "", Some(payload)).await
  }

  pub async fn put_model_item(
    name: &str,
    payload: &serde_json::Value,
    token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    let path = format!("/v1/models/items/{}", encode_uri_component(name));
    request_json("PUT", &path, token, "", Some(payload)).await
  }

  pub async fn delete_model_item(name: &str, token: &str) -> Result<ModelConfigSnapshot, String> {
    let path = format!("/v1/models/items/{}", encode_uri_component(name));
    request_json("DELETE", &path, token, "", None).await
  }

  pub async fn fetch_diagnostics(token: &str) -> Result<DiagnosticsReport, String> {
    request_json("GET", "/v1/diagnostics", token, "", None).await
  }

  pub async fn submit_run(
    tenant: &str,
    profile: &str,
    workflow_yaml: &str,
    inputs: Option<serde_json::Value>,
    token: &str,
  ) -> Result<String, String> {
    let effective_tenant = if tenant.trim().is_empty() {
      "default"
    } else {
      tenant.trim()
    };
    let mut body = serde_json::json!({
      "tenant_id": effective_tenant,
      "workflow": workflow_yaml,
      "profile": profile,
    });
    if let Some(inp) = inputs
      && inp.is_object()
      && let Some(obj) = body.as_object_mut()
    {
      obj.insert("inputs".to_string(), inp);
    }
    let raw: serde_json::Value =
      request_json("POST", "/v1/runs", token, effective_tenant, Some(&body)).await?;
    raw
      .get("run_id")
      .or_else(|| raw.pointer("/run/id"))
      .or_else(|| raw.get("id"))
      .and_then(|v| v.as_str())
      .map(ToString::to_string)
      .ok_or_else(|| format!("POST /v1/runs missing run_id: {raw}"))
  }

  pub async fn list_harness_sessions(
    tenant: &str,
    token: &str,
  ) -> Result<Vec<HarnessSession>, String> {
    let effective_tenant = if tenant.trim().is_empty() {
      "default"
    } else {
      tenant.trim()
    };
    #[derive(Deserialize)]
    struct Envelope {
      #[serde(default)]
      sessions: Vec<HarnessSession>,
    }
    let env: Envelope =
      request_json("GET", "/v1/harness/sessions", token, effective_tenant, None).await?;
    Ok(env.sessions)
  }

  pub async fn create_harness_session(
    body: &serde_json::Value,
    tenant: &str,
    token: &str,
  ) -> Result<String, String> {
    let effective_tenant = if tenant.trim().is_empty() {
      "default"
    } else {
      tenant.trim()
    };
    let raw: serde_json::Value = request_json(
      "POST",
      "/v1/harness/sessions",
      token,
      effective_tenant,
      Some(body),
    )
    .await?;
    raw
      .get("session_id")
      .and_then(|v| v.as_str())
      .filter(|s| !s.is_empty())
      .map(ToString::to_string)
      .ok_or_else(|| format!("POST /v1/harness/sessions missing session_id: {raw}"))
  }

  pub async fn get_harness_session(
    session_id: &str,
    tenant: &str,
    token: &str,
  ) -> Result<HarnessSession, String> {
    let path = format!("/v1/harness/sessions/{}", encode_uri_component(session_id));
    request_json("GET", &path, token, tenant, None).await
  }

  pub async fn get_harness_events_history(
    session_id: &str,
    tenant: &str,
    token: &str,
  ) -> Result<Vec<HarnessEvent>, String> {
    let path = format!(
      "/v1/harness/sessions/{}/events/history",
      encode_uri_component(session_id)
    );
    request_json("GET", &path, token, tenant, None).await
  }

  pub async fn list_harness_approvals(
    session_id: &str,
    tenant: &str,
    token: &str,
  ) -> Result<Vec<PendingApproval>, String> {
    let path = format!(
      "/v1/harness/sessions/{}/approvals",
      encode_uri_component(session_id)
    );
    #[derive(Deserialize)]
    struct Envelope {
      #[serde(default)]
      approvals: Vec<PendingApproval>,
    }
    let env: Envelope = request_json("GET", &path, token, tenant, None).await?;
    Ok(env.approvals)
  }

  pub async fn decide_harness_approval(
    session_id: &str,
    request_id: &str,
    decision: &str,
    scope: &str,
    tenant: &str,
    token: &str,
  ) -> Result<(), String> {
    let path = format!(
      "/v1/harness/sessions/{}/approvals/{}",
      encode_uri_component(session_id),
      encode_uri_component(request_id)
    );
    let body = serde_json::json!({
      "decision": decision,
      "scope": scope,
      "decided_by": "ui-dx",
    });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = send_request("POST", &path, token, tenant, Some(&body_str), None, None).await?;
    if !resp.ok() {
      let text = read_response_text(&resp).await?;
      return Err(format!("HTTP {}: {text}", resp.status()));
    }
    Ok(())
  }

  pub async fn get_harness_interrupt(
    session_id: &str,
    tenant: &str,
    token: &str,
  ) -> Result<Option<PendingInterrupt>, String> {
    let path = format!(
      "/v1/harness/sessions/{}/interrupt",
      encode_uri_component(session_id)
    );
    #[derive(Deserialize)]
    struct Envelope {
      #[serde(default)]
      pending: Option<PendingInterrupt>,
    }
    let env: Envelope = request_json("GET", &path, token, tenant, None).await?;
    Ok(env.pending)
  }

  pub async fn answer_harness_interrupt(
    session_id: &str,
    answer: &str,
    tenant: &str,
    token: &str,
  ) -> Result<(), String> {
    let path = format!(
      "/v1/harness/sessions/{}/interrupt/answer",
      encode_uri_component(session_id)
    );
    let body = serde_json::json!({ "answer": answer });
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = send_request("POST", &path, token, tenant, Some(&body_str), None, None).await?;
    if !resp.ok() {
      let text = read_response_text(&resp).await?;
      return Err(format!("HTTP {}: {text}", resp.status()));
    }
    Ok(())
  }

  pub async fn cancel_harness_session(
    session_id: &str,
    tenant: &str,
    token: &str,
  ) -> Result<(), String> {
    let path = format!(
      "/v1/harness/sessions/{}:cancel",
      encode_uri_component(session_id)
    );
    let resp = send_request("POST", &path, token, tenant, None, None, None).await?;
    if !resp.ok() {
      let text = read_response_text(&resp).await?;
      return Err(format!("HTTP {}: {text}", resp.status()));
    }
    Ok(())
  }

  pub async fn resume_harness_session(
    session_id: &str,
    mode: &str,
    user_input: Option<&str>,
    tenant: &str,
    token: &str,
  ) -> Result<(), String> {
    let path = format!(
      "/v1/harness/sessions/{}:resume",
      encode_uri_component(session_id)
    );
    let mut body = serde_json::json!({ "mode": mode });
    if let Some(prompt) = user_input.map(str::trim).filter(|s| !s.is_empty())
      && let Some(obj) = body.as_object_mut()
    {
      obj.insert("user_input".to_string(), serde_json::json!(prompt));
    }
    let body_str = serde_json::to_string(&body).map_err(|e| e.to_string())?;
    let resp = send_request("POST", &path, token, tenant, Some(&body_str), None, None).await?;
    if !resp.ok() {
      let text = read_response_text(&resp).await?;
      return Err(format!("HTTP {}: {text}", resp.status()));
    }
    Ok(())
  }

  pub async fn pick_desktop_directory(
    token: &str,
    initial_dir: &str,
  ) -> Result<Option<String>, String> {
    let body = serde_json::json!({
      "initial_dir": if initial_dir.trim().is_empty() { serde_json::Value::Null } else { serde_json::json!(initial_dir.trim()) }
    });
    let raw: serde_json::Value =
      request_json("POST", "/v1/desktop/pick-directory", token, "", Some(&body)).await?;
    Ok(
      raw
        .get("path")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToString::to_string),
    )
  }

  pub struct SseAbortHandle {
    controller: AbortController,
  }

  impl SseAbortHandle {
    pub fn new() -> Result<Self, String> {
      AbortController::new()
        .map(|controller| Self { controller })
        .map_err(js_err)
    }

    pub fn abort(&self) {
      self.controller.abort();
    }

    fn signal(&self) -> web_sys::AbortSignal {
      self.controller.signal()
    }
  }

  impl Drop for SseAbortHandle {
    fn drop(&mut self) {
      self.abort();
    }
  }

  pub async fn stream_sse<F>(
    path: &str,
    token: &str,
    tenant: &str,
    abort_handle: &SseAbortHandle,
    mut on_frame: F,
  ) -> Result<usize, String>
  where
    F: FnMut(SseFrame),
  {
    let signal = abort_handle.signal();
    let resp = send_request(
      "GET",
      path,
      token,
      tenant,
      None,
      Some("text/event-stream"),
      Some(&signal),
    )
    .await?;
    if !resp.ok() {
      return Err(format!("SSE HTTP {}", resp.status()));
    }
    let body = resp
      .body()
      .ok_or_else(|| "SSE response body stream is empty".to_string())?;
    let reader: ReadableStreamDefaultReader = body
      .get_reader()
      .dyn_into()
      .map_err(|_| "failed to obtain ReadableStreamDefaultReader".to_string())?;
    let decoder = TextDecoder::new().map_err(js_err)?;
    let mut pending = String::new();
    let mut frame_count = 0usize;

    loop {
      let read_result = JsFuture::from(reader.read()).await.map_err(js_err)?;
      let done = Reflect::get(&read_result, &JsValue::from_str("done"))
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
      if done {
        break;
      }
      if let Ok(value) = Reflect::get(&read_result, &JsValue::from_str("value")) {
        let bytes = Uint8Array::new(&value).to_vec();
        let text = decoder.decode_with_u8_array(&bytes).map_err(js_err)?;
        for frame in parse_sse_chunk(&mut pending, &text) {
          frame_count += 1;
          on_frame(frame);
        }
      }
    }

    Ok(frame_count)
  }

  pub fn create_blob_previews() -> Result<BlobPreviewBundle, String> {
    let svg_markup = r##"<svg xmlns="http://www.w3.org/2000/svg" width="420" height="140" viewBox="0 0 420 140">
  <rect width="420" height="140" rx="12" fill="#0f172a"/>
  <rect x="16" y="16" width="388" height="108" rx="8" fill="#1e293b" stroke="#38bdf8" stroke-width="1.5"/>
  <text x="32" y="56" fill="#f8fafc" font-family="system-ui, sans-serif" font-size="16" font-weight="600">Yanshi Dioxus · Blob SVG Preview</text>
  <text x="32" y="84" fill="#94a3b8" font-family="ui-monospace, monospace" font-size="12">img-src blob: verified under strict CSP (zero eval)</text>
  <circle cx="376" cy="48" r="10" fill="#10b981"/>
</svg>"##;
    let html_markup = r##"<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><style>
body{margin:0;padding:14px 16px;font-family:system-ui,sans-serif;background:#f8fafc;color:#0f172a;font-size:13px}
code{background:#e2e8f0;padding:2px 6px;border-radius:4px}
</style></head><body><strong>Blob iframe Preview OK</strong><p><code>frame-src blob:</code> rendered cleanly without inline script or eval.</p></body></html>"##;

    let svg_url = make_blob_url(svg_markup, "image/svg+xml")?;
    let html_url = match make_blob_url(html_markup, "text/html;charset=utf-8") {
      Ok(url) => url,
      Err(err) => {
        revoke_blob_url(&svg_url);
        return Err(err);
      }
    };
    Ok(BlobPreviewBundle { svg_url, html_url })
  }

  fn make_blob_url(content: &str, mime: &str) -> Result<String, String> {
    let parts = Array::new();
    parts.push(&JsValue::from_str(content));
    let opts = BlobPropertyBag::new();
    opts.set_type(mime);
    let blob = Blob::new_with_str_sequence_and_options(&parts, &opts).map_err(js_err)?;
    Url::create_object_url_with_blob(&blob).map_err(js_err)
  }

  pub fn revoke_blob_url(url: &str) {
    if !url.is_empty() {
      let _ = Url::revoke_object_url(url);
    }
  }
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
mod wasm_impl {
  use super::*;

  pub fn resolve_api_base() -> String {
    String::new()
  }

  pub fn resolve_embedded_token() -> String {
    String::new()
  }

  pub fn is_local_desktop_engine() -> bool {
    false
  }

  pub fn read_storage(_key: &str, fallback: &str) -> String {
    fallback.to_string()
  }

  pub fn write_storage(_key: &str, _value: &str) {}

  pub fn current_location_path_and_search() -> (String, String) {
    ("/ui-dx/diagnostics".to_string(), String::new())
  }

  pub fn push_browser_route(_path_with_search: &str) {}

  pub async fn fetch_models_config(
    _token: &str,
    _tenant: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn put_provider(
    _name: &str,
    _payload: &serde_json::Value,
    _token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn delete_provider(
    _name: &str,
    _clear_api_key: bool,
    _token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn test_provider(
    _name: &str,
    _payload: &serde_json::Value,
    _token: &str,
  ) -> Result<TestProviderOutcome, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn put_model_item(
    _name: &str,
    _payload: &serde_json::Value,
    _token: &str,
  ) -> Result<ModelConfigSnapshot, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn delete_model_item(_name: &str, _token: &str) -> Result<ModelConfigSnapshot, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn fetch_diagnostics(_token: &str) -> Result<DiagnosticsReport, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn submit_run(
    _tenant: &str,
    _profile: &str,
    _workflow_yaml: &str,
    _inputs: Option<serde_json::Value>,
    _token: &str,
  ) -> Result<String, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn list_harness_sessions(
    _tenant: &str,
    _token: &str,
  ) -> Result<Vec<HarnessSession>, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn create_harness_session(
    _body: &serde_json::Value,
    _tenant: &str,
    _token: &str,
  ) -> Result<String, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn get_harness_session(
    _session_id: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<HarnessSession, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn get_harness_events_history(
    _session_id: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<Vec<HarnessEvent>, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn list_harness_approvals(
    _session_id: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<Vec<PendingApproval>, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn decide_harness_approval(
    _session_id: &str,
    _request_id: &str,
    _decision: &str,
    _scope: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<(), String> {
    Err("wasm32 only".to_string())
  }

  pub async fn get_harness_interrupt(
    _session_id: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<Option<PendingInterrupt>, String> {
    Err("wasm32 only".to_string())
  }

  pub async fn answer_harness_interrupt(
    _session_id: &str,
    _answer: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<(), String> {
    Err("wasm32 only".to_string())
  }

  pub async fn cancel_harness_session(
    _session_id: &str,
    _tenant: &str,
    _token: &str,
  ) -> Result<(), String> {
    Err("wasm32 only".to_string())
  }

  pub async fn resume_harness_session(
    _session_id: &str,
    _mode: &str,
    _user_input: Option<&str>,
    _tenant: &str,
    _token: &str,
  ) -> Result<(), String> {
    Err("wasm32 only".to_string())
  }

  pub async fn pick_desktop_directory(
    _token: &str,
    _initial_dir: &str,
  ) -> Result<Option<String>, String> {
    Err("wasm32 only".to_string())
  }

  pub struct SseAbortHandle;

  impl SseAbortHandle {
    pub fn new() -> Result<Self, String> {
      Ok(Self)
    }

    pub fn abort(&self) {}
  }

  impl Drop for SseAbortHandle {
    fn drop(&mut self) {
      self.abort();
    }
  }

  pub async fn stream_sse<F>(
    _path: &str,
    _token: &str,
    _tenant: &str,
    _abort_handle: &SseAbortHandle,
    _on_frame: F,
  ) -> Result<usize, String>
  where
    F: FnMut(SseFrame),
  {
    Err("wasm32 only".to_string())
  }

  pub fn create_blob_previews() -> Result<BlobPreviewBundle, String> {
    Err("wasm32 only".to_string())
  }

  pub fn revoke_blob_url(_url: &str) {}
}

pub use wasm_impl::{
  SseAbortHandle, answer_harness_interrupt, cancel_harness_session, create_blob_previews,
  create_harness_session, current_location_path_and_search, decide_harness_approval,
  delete_model_item, delete_provider, fetch_diagnostics, fetch_models_config,
  get_harness_events_history, get_harness_interrupt, get_harness_session, is_local_desktop_engine,
  list_harness_approvals, list_harness_sessions, pick_desktop_directory, push_browser_route,
  put_model_item, put_provider, read_storage, resolve_api_base, resolve_embedded_token,
  resume_harness_session, revoke_blob_url, stream_sse, submit_run, test_provider, write_storage,
};

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_sse_chunk_handles_split_frames_and_crlf() {
    let mut pending = String::new();
    let first = parse_sse_chunk(
      &mut pending,
      "event: run_started\r\ndata: {\"id\":\"r1\"}\r\n\r\nevent: node",
    );
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].event, "run_started");
    assert_eq!(first[0].data, "{\"id\":\"r1\"}");

    let second = parse_sse_chunk(&mut pending, "_done\ndata: line1\ndata: line2\n\n");
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].event, "node_done");
    assert_eq!(second[0].data, "line1\nline2");
    assert!(pending.is_empty());
  }

  #[test]
  fn join_api_url_normalizes_slashes() {
    assert_eq!(join_api_url("", "/v1/models/config"), "/v1/models/config");
    assert_eq!(
      join_api_url("http://127.0.0.1:8080/", "v1/runs"),
      "http://127.0.0.1:8080/v1/runs"
    );
    assert_eq!(
      join_api_url("http://127.0.0.1:8080", "https://example.com/x"),
      "https://example.com/x"
    );
  }

  #[test]
  fn mask_token_and_uri_encode_work() {
    assert_eq!(mask_token(""), "(none)");
    assert_eq!(mask_token("abc"), "••abc");
    assert_eq!(mask_token("sk-secret-9876"), "••••9876");
    assert_eq!(
      encode_uri_component("my/provider name"),
      "my%2Fprovider%20name"
    );
  }
}
