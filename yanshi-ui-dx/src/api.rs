//! Thin browser API bridge for the Phase 0 Dioxus spike:
//! - Desktop/Web host token & base URL detection (`__YANSHI_API_BASE__` / `__YANSHI_API_TOKEN__`)
//! - Authenticated JSON `fetch` (`GET /v1/models/config`, `POST /v1/runs`)
//! - Authenticated SSE streaming via `fetch` + `ReadableStreamDefaultReader` + `AbortController`
//! - CSP-safe `Blob` ObjectURL creation & explicit revocation for media previews

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelConfigSummary {
  pub config_dir: String,
  pub providers: Vec<ProviderSummary>,
  pub models: Vec<ModelItemSummary>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderSummary {
  pub name: String,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub has_api_key: bool,
  #[serde(default)]
  pub api_key_hint: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelItemSummary {
  pub name: String,
  #[serde(default)]
  pub provider: String,
  #[serde(default)]
  pub model_id: Option<String>,
  #[serde(default)]
  pub base_url: Option<String>,
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

/// Pure SSE chunk parser shared across WASM and host unit tests.
/// Appends `chunk` to `pending`, emits complete `\n\n`-delimited frames,
/// and leaves any trailing partial line in `pending`.
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
    window_string_prop("__YANSHI_API_BASE__").unwrap_or_default()
  }

  pub fn resolve_embedded_token() -> String {
    window_string_prop("__YANSHI_API_TOKEN__").unwrap_or_default()
  }

  fn build_headers(
    token: &str,
    tenant: &str,
    content_type: Option<&str>,
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
    if !trimmed_tenant.is_empty() && trimmed_tenant != "default" {
      headers.set("X-Tenant-Id", trimmed_tenant).map_err(js_err)?;
    }
    if let Some(ct) = content_type {
      headers.set("Content-Type", ct).map_err(js_err)?;
    }
    Ok(headers)
  }

  async fn send_request(
    method: &str,
    path: &str,
    token: &str,
    tenant: &str,
    json_body: Option<&str>,
    signal: Option<&web_sys::AbortSignal>,
  ) -> Result<Response, String> {
    let win = web_sys::window().ok_or_else(|| "window is unavailable".to_string())?;
    let url = join_api_url(&resolve_api_base(), path);
    let init = RequestInit::new();
    init.set_method(method);
    init.set_mode(RequestMode::Cors);
    let headers = build_headers(token, tenant, json_body.map(|_| "application/json"))?;
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

  pub async fn fetch_models_config(
    token: &str,
    tenant: &str,
  ) -> Result<ModelConfigSummary, String> {
    let resp = send_request("GET", "/v1/models/config", token, tenant, None, None).await?;
    let status = resp.status();
    let text_promise = resp.text().map_err(js_err)?;
    let body_text = JsFuture::from(text_promise)
      .await
      .map_err(js_err)?
      .as_string()
      .unwrap_or_default();
    if !resp.ok() {
      return Err(format!("HTTP {status}: {body_text}"));
    }
    serde_json::from_str::<ModelConfigSummary>(&body_text)
      .map_err(|err| format!("invalid /v1/models/config JSON: {err}"))
  }

  pub async fn submit_smoke_run(token: &str, tenant: &str) -> Result<String, String> {
    let payload = serde_json::json!({
      "workflow_yaml": "name: dx-phase0-sse-smoke\nnodes:\n  - id: step_a\n    type: template\n    parameters:\n      template: \"hello from dioxus spike\"\n  - id: step_b\n    type: template\n    parameters:\n      template: \"chained: {{ step_a }}\"\n",
      "inputs": {}
    });
    let body = serde_json::to_string(&payload).map_err(|err| err.to_string())?;
    let resp = send_request("POST", "/v1/runs", token, tenant, Some(&body), None).await?;
    let status = resp.status();
    let text_promise = resp.text().map_err(js_err)?;
    let body_text = JsFuture::from(text_promise)
      .await
      .map_err(js_err)?
      .as_string()
      .unwrap_or_default();
    if !resp.ok() {
      return Err(format!("HTTP {status}: {body_text}"));
    }
    let parsed: serde_json::Value =
      serde_json::from_str(&body_text).map_err(|err| format!("invalid run JSON: {err}"))?;
    parsed
      .pointer("/run/id")
      .or_else(|| parsed.get("id"))
      .and_then(|v| v.as_str())
      .map(ToString::to_string)
      .ok_or_else(|| format!("missing run id in response: {body_text}"))
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

  pub async fn stream_run_sse<F>(
    run_id: &str,
    token: &str,
    tenant: &str,
    abort_handle: &SseAbortHandle,
    mut on_frame: F,
  ) -> Result<usize, String>
  where
    F: FnMut(SseFrame),
  {
    let signal = abort_handle.signal();
    let path = format!("/v1/runs/{run_id}/events");
    let resp = send_request("GET", &path, token, tenant, None, Some(&signal)).await?;
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
  <text x="32" y="56" fill="#f8fafc" font-family="system-ui, sans-serif" font-size="16" font-weight="600">Yanshi Dioxus Phase 0 · Blob SVG Preview</text>
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
mod wasm_impl {
  use super::*;

  pub fn resolve_api_base() -> String {
    String::new()
  }

  pub fn resolve_embedded_token() -> String {
    String::new()
  }

  pub async fn fetch_models_config(
    _token: &str,
    _tenant: &str,
  ) -> Result<ModelConfigSummary, String> {
    Err("fetch_models_config requires wasm32 browser target".to_string())
  }

  pub async fn submit_smoke_run(_token: &str, _tenant: &str) -> Result<String, String> {
    Err("submit_smoke_run requires wasm32 browser target".to_string())
  }

  pub struct SseAbortHandle;

  impl SseAbortHandle {
    pub fn new() -> Result<Self, String> {
      Ok(Self)
    }

    pub fn abort(&self) {}
  }

  pub async fn stream_run_sse<F>(
    _run_id: &str,
    _token: &str,
    _tenant: &str,
    _abort_handle: &SseAbortHandle,
    _on_frame: F,
  ) -> Result<usize, String>
  where
    F: FnMut(SseFrame),
  {
    Err("stream_run_sse requires wasm32 browser target".to_string())
  }

  pub fn create_blob_previews() -> Result<BlobPreviewBundle, String> {
    Err("create_blob_previews requires wasm32 browser target".to_string())
  }

  pub fn revoke_blob_url(_url: &str) {}
}

pub use wasm_impl::{
  SseAbortHandle, create_blob_previews, fetch_models_config, resolve_api_base,
  resolve_embedded_token, revoke_blob_url, stream_run_sse, submit_smoke_run,
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
  }
}
