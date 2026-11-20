//! DeepSeek native `/v1/tools/*` client (`fetch` & `ocr`, plus shared endpoint helper).
//!
//! Uses the same `DEEPSEEK_API_KEY` (and optional `DEEPSEEK_BASE_URL`) as
//! DeepSeek chat completions. Target URLs passed to `POST /v1/tools/fetch` are
//! validated against [`HttpTool`]'s [`SandboxPolicy`] before dispatch so private,
//! loopback, CGNAT, and cloud-metadata targets are rejected uniformly.

use std::sync::Arc;
use std::time::Duration;

use reqwest::{
  Client, Url,
  multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{ToolError, builtin::HttpTool, sandbox::SandboxPolicy};

pub const DEFAULT_DEEPSEEK_TOOLS_BASE_URL: &str = "https://api.deepseek.com";
const MAX_FETCH_URL_CHARS: usize = 8192;
const MAX_FETCH_RESPONSE_BYTES: usize = 8 * 1024 * 1024;
const MAX_OCR_INPUT_BYTES: usize = 50 * 1024 * 1024;
const MAX_OCR_RESPONSE_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_FETCH_TIMEOUT_SECS: u64 = 60;
const DEFAULT_OCR_TIMEOUT_SECS: u64 = 300;

/// Normalize an optional DeepSeek base URL (`https://api.deepseek.com` or
/// `https://api.deepseek.com/v1`) into `{root}/v1/tools/{op}`.
pub fn deepseek_tools_endpoint(base_url: Option<&str>, op: &str) -> String {
  let raw = base_url
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or(DEFAULT_DEEPSEEK_TOOLS_BASE_URL);
  let trimmed = raw.trim_end_matches('/');
  let root = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
  let op = op.trim_start_matches('/');
  format!("{root}/v1/tools/{op}")
}

/// OCR recognition mode accepted by `POST /v1/tools/ocr`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeepSeekOcrPurpose {
  /// Extracts native text from image-free Word/PPTX directly and OCRs other pages.
  TextRecognition,
  /// Runs page-by-page layout-aware OCR across every page/image.
  RichLayoutRecognition,
}

impl DeepSeekOcrPurpose {
  pub fn as_wire_str(self) -> &'static str {
    match self {
      Self::TextRecognition => "text_recognition",
      Self::RichLayoutRecognition => "rich_layout_recognition",
    }
  }
}

/// Markdown page fetched by `POST /v1/tools/fetch`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekFetchResponse {
  #[serde(default)]
  pub title: String,
  pub content: String,
}

/// Single page recognized by `POST /v1/tools/ocr`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekOcrPage {
  pub index: usize,
  pub text: String,
  #[serde(default)]
  pub finish_reason: String,
}

/// Full document recognition response from `POST /v1/tools/ocr`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeepSeekOcrResponse {
  #[serde(default)]
  pub id: String,
  pub page_count: usize,
  pub pages: Vec<DeepSeekOcrPage>,
}

impl DeepSeekOcrResponse {
  /// Return trimmed per-page text strings, optionally capped to `max_pages`.
  pub fn page_texts(&self, max_pages: Option<usize>) -> Vec<String> {
    let limit = max_pages.unwrap_or(usize::MAX);
    self
      .pages
      .iter()
      .take(limit)
      .map(|p| p.text.trim().to_string())
      .collect()
  }

  /// Format non-empty pages as numbered `[Page N]` sections (or plain text for a single page).
  pub fn formatted_text(&self, max_pages: Option<usize>) -> String {
    let pages = self.page_texts(max_pages);
    if pages.len() == 1 {
      return pages[0].clone();
    }
    pages
      .iter()
      .enumerate()
      .filter(|(_, text)| !text.is_empty())
      .map(|(idx, text)| format!("[Page {}]\n{text}", idx + 1))
      .collect::<Vec<_>>()
      .join("\n\n")
  }
}

/// Client for DeepSeek's `/v1/tools/fetch` and `/v1/tools/ocr` endpoints.
#[derive(Clone)]
pub struct DeepSeekToolsClient {
  api_key: String,
  base_url: Option<String>,
  policy: Arc<SandboxPolicy>,
  no_proxy: bool,
}

impl std::fmt::Debug for DeepSeekToolsClient {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DeepSeekToolsClient")
      .field("base_url", &self.base_url)
      .field("api_key", &"<redacted>")
      .field("no_proxy", &self.no_proxy)
      .finish()
  }
}

impl DeepSeekToolsClient {
  pub fn new(api_key: impl Into<String>, base_url: Option<String>) -> Result<Self, ToolError> {
    let trimmed = api_key.into().trim().to_string();
    if trimmed.is_empty() {
      return Err(ToolError::InvalidParams {
        message: "DEEPSEEK_API_KEY must not be empty".to_string(),
      });
    }
    let normalized_base = base_url
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty());
    let is_loopback_base = normalized_base
      .as_deref()
      .and_then(|u| Url::parse(u).ok())
      .and_then(|u| u.host_str().map(str::to_ascii_lowercase))
      .is_some_and(|h| matches!(h.as_str(), "127.0.0.1" | "localhost" | "::1" | "[::1]"));
    Ok(Self {
      api_key: trimmed,
      base_url: normalized_base,
      policy: Arc::new(SandboxPolicy::default()),
      no_proxy: is_loopback_base,
    })
  }

  /// Build from `DEEPSEEK_API_KEY` (and optional `DEEPSEEK_BASE_URL`) when set.
  pub fn from_env() -> Option<Self> {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
      .ok()
      .map(|v| v.trim().to_string())
      .filter(|v| !v.is_empty())?;
    let base_url = std::env::var("DEEPSEEK_BASE_URL")
      .ok()
      .map(|v| v.trim().to_string())
      .filter(|v| !v.is_empty());
    Self::new(api_key, base_url).ok()
  }

  pub fn with_policy(mut self, policy: Arc<SandboxPolicy>) -> Self {
    self.policy = policy;
    self
  }

  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }

  fn build_http_client(&self, timeout: Duration) -> Result<Client, ToolError> {
    let mut builder = Client::builder().timeout(timeout);
    if self.no_proxy {
      builder = builder.no_proxy();
    }
    builder.build().map_err(|err| ToolError::ExecutionFailed {
      message: format!("Failed to construct DeepSeek tools HTTP client: {err}"),
    })
  }

  /// Fetch clean Markdown and page title via `POST /v1/tools/fetch`.
  pub async fn fetch_markdown(&self, target_url: &str) -> Result<DeepSeekFetchResponse, ToolError> {
    let trimmed = target_url.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_FETCH_URL_CHARS {
      return Err(ToolError::InvalidParams {
        message: format!("Fetch URL must be 1..={MAX_FETCH_URL_CHARS} characters"),
      });
    }
    let parsed = Url::parse(trimmed).map_err(|err| ToolError::InvalidParams {
      message: format!("Invalid fetch URL '{trimmed}': {err}"),
    })?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
      return Err(ToolError::InvalidParams {
        message: "Fetch URL must not embed username or password credentials".to_string(),
      });
    }

    // Enforce SSRF / domain-allowlist rules on the target URL before delegating.
    let mut validator = HttpTool::new(Arc::clone(&self.policy))?;
    if self.no_proxy {
      validator = validator.with_no_proxy();
    }
    validator.validate_url_str(trimmed).await?;

    let endpoint = deepseek_tools_endpoint(self.base_url.as_deref(), "fetch");
    let client = self.build_http_client(Duration::from_secs(DEFAULT_FETCH_TIMEOUT_SECS))?;
    let response = client
      .post(&endpoint)
      .bearer_auth(&self.api_key)
      .json(&json!({ "url": trimmed }))
      .send()
      .await
      .map_err(|err| ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/fetch request failed: {err}"),
      })?;

    let status = response.status();
    let body_bytes = read_response_capped(response, MAX_FETCH_RESPONSE_BYTES, "fetch").await?;
    if !status.is_success() {
      let excerpt: String = String::from_utf8_lossy(&body_bytes)
        .chars()
        .take(400)
        .collect();
      return Err(ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/fetch returned HTTP {status}: {excerpt}"),
      });
    }

    let raw_json: Value =
      serde_json::from_slice(&body_bytes).map_err(|err| ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/fetch returned invalid JSON: {err}"),
      })?;
    if let Some(err_msg) = extract_embedded_error(&raw_json) {
      return Err(ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/fetch error: {err_msg}"),
      });
    }

    serde_json::from_value(raw_json).map_err(|err| ToolError::HttpError {
      message: format!("DeepSeek /v1/tools/fetch payload mismatch: {err}"),
    })
  }

  /// Run OCR over a PDF, Word, PowerPoint, or image document via `POST /v1/tools/ocr`.
  pub async fn ocr_bytes(
    &self,
    bytes: &[u8],
    filename: &str,
    purpose: DeepSeekOcrPurpose,
  ) -> Result<DeepSeekOcrResponse, ToolError> {
    if bytes.is_empty() {
      return Err(ToolError::InvalidParams {
        message: "OCR input file must not be empty".to_string(),
      });
    }
    if bytes.len() > MAX_OCR_INPUT_BYTES {
      return Err(ToolError::InvalidParams {
        message: format!(
          "OCR input file ({} bytes) exceeds {} MiB limit",
          bytes.len(),
          MAX_OCR_INPUT_BYTES / (1024 * 1024)
        ),
      });
    }

    let safe_name = if filename.trim().is_empty() {
      "document.bin".to_string()
    } else {
      filename.trim().to_string()
    };
    let mime = infer_ocr_mime(&safe_name, bytes);
    let part = Part::bytes(bytes.to_vec())
      .file_name(safe_name.clone())
      .mime_str(mime)
      .unwrap_or_else(|_| Part::bytes(bytes.to_vec()).file_name(safe_name));
    let form = Form::new()
      .text("purpose", purpose.as_wire_str().to_string())
      .part("file", part);

    let endpoint = deepseek_tools_endpoint(self.base_url.as_deref(), "ocr");
    let client = self.build_http_client(Duration::from_secs(DEFAULT_OCR_TIMEOUT_SECS))?;
    let response = client
      .post(&endpoint)
      .bearer_auth(&self.api_key)
      .multipart(form)
      .send()
      .await
      .map_err(|err| ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/ocr request failed: {err}"),
      })?;

    let status = response.status();
    let body_bytes = read_response_capped(response, MAX_OCR_RESPONSE_BYTES, "ocr").await?;
    if !status.is_success() {
      let excerpt: String = String::from_utf8_lossy(&body_bytes)
        .chars()
        .take(400)
        .collect();
      return Err(ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/ocr returned HTTP {status}: {excerpt}"),
      });
    }

    let raw_json: Value =
      serde_json::from_slice(&body_bytes).map_err(|err| ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/ocr returned invalid JSON: {err}"),
      })?;
    if let Some(err_msg) = extract_embedded_error(&raw_json) {
      return Err(ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/ocr error: {err_msg}"),
      });
    }

    let mut parsed: DeepSeekOcrResponse =
      serde_json::from_value(raw_json).map_err(|err| ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/ocr payload mismatch: {err}"),
      })?;
    parsed.pages.sort_by_key(|p| p.index);
    Ok(parsed)
  }
}

async fn read_response_capped(
  mut response: reqwest::Response,
  max_bytes: usize,
  op: &str,
) -> Result<Vec<u8>, ToolError> {
  if let Some(declared) = response.content_length()
    && declared > max_bytes as u64
  {
    return Err(ToolError::HttpError {
      message: format!(
        "DeepSeek /v1/tools/{op} response declares {declared} bytes, over {max_bytes}-byte cap"
      ),
    });
  }
  let mut buf = Vec::new();
  while let Some(chunk) = response.chunk().await.map_err(|err| ToolError::HttpError {
    message: format!("DeepSeek /v1/tools/{op} stream error: {err}"),
  })? {
    if buf.len().saturating_add(chunk.len()) > max_bytes {
      return Err(ToolError::HttpError {
        message: format!("DeepSeek /v1/tools/{op} response exceeded {max_bytes}-byte cap"),
      });
    }
    buf.extend_from_slice(&chunk);
  }
  Ok(buf)
}

fn extract_embedded_error(value: &Value) -> Option<String> {
  let err = value.get("error")?;
  if err.is_null() {
    return None;
  }
  if let Some(msg) = err.get("message").and_then(Value::as_str) {
    return Some(msg.to_string());
  }
  if let Some(msg) = err.as_str() {
    return Some(msg.to_string());
  }
  Some(err.to_string())
}

fn infer_ocr_mime(filename: &str, bytes: &[u8]) -> &'static str {
  if bytes.starts_with(b"%PDF-") {
    return "application/pdf";
  }
  if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
    return "image/png";
  }
  if bytes.starts_with(b"\xff\xd8\xff") {
    return "image/jpeg";
  }
  if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
    return "image/gif";
  }
  if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
    return "image/webp";
  }
  let ext = std::path::Path::new(filename)
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_ascii_lowercase);
  match ext.as_deref() {
    Some("pdf") => "application/pdf",
    Some("png") => "image/png",
    Some("jpg" | "jpeg") => "image/jpeg",
    Some("webp") => "image/webp",
    Some("gif") => "image/gif",
    Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    Some("pptx") => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    _ => "application/octet-stream",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  #[test]
  fn endpoint_helper_strips_duplicate_v1_suffix() {
    assert_eq!(
      deepseek_tools_endpoint(None, "search"),
      "https://api.deepseek.com/v1/tools/search"
    );
    assert_eq!(
      deepseek_tools_endpoint(Some("https://api.deepseek.com/v1/"), "ocr"),
      "https://api.deepseek.com/v1/tools/ocr"
    );
    assert_eq!(
      deepseek_tools_endpoint(Some("http://127.0.0.1:9000"), "fetch"),
      "http://127.0.0.1:9000/v1/tools/fetch"
    );
  }

  #[test]
  fn debug_redacts_deepseek_api_key() {
    let client = DeepSeekToolsClient::new("sk-top-secret", None).expect("client");
    let dbg = format!("{client:?}");
    assert!(!dbg.contains("sk-top-secret"));
    assert!(dbg.contains("<redacted>"));
  }

  #[tokio::test]
  async fn fetch_markdown_rejects_userinfo_and_private_targets() {
    let client = DeepSeekToolsClient::new("sk-test", Some("http://127.0.0.1:1".to_string()))
      .expect("client")
      .with_no_proxy();

    let cred_err = client
      .fetch_markdown("https://user:pass@example.com/doc")
      .await
      .unwrap_err();
    assert!(cred_err.to_string().contains("username or password"));

    let ssrf_err = client
      .fetch_markdown("http://169.254.169.254/latest/meta-data/")
      .await
      .unwrap_err();
    assert!(matches!(ssrf_err, ToolError::SandboxViolation { .. }));

    let loopback_err = client
      .fetch_markdown("http://127.0.0.1:8080/internal")
      .await
      .unwrap_err();
    assert!(matches!(loopback_err, ToolError::SandboxViolation { .. }));
  }

  #[tokio::test]
  async fn fetch_markdown_posts_url_and_parses_markdown_response() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 4096];
      let n = socket.read(&mut buf).await.expect("read");
      let req = String::from_utf8_lossy(&buf[..n]).to_string();
      assert!(
        req.starts_with("POST /v1/tools/fetch HTTP/1.1"),
        "got: {req}"
      );
      assert!(req.contains("\"url\":\"http://93.184.216.34/guide\""));

      let body = r##"{"title":"DeepSeek Guide","content":"# Welcome\n\nClean Markdown body."}"##;
      let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      );
      socket.write_all(resp.as_bytes()).await.expect("write");
    });

    let client = DeepSeekToolsClient::new("sk-ds", Some(format!("http://{addr}/v1")))
      .expect("client")
      .with_no_proxy();
    let fetched = client
      .fetch_markdown("http://93.184.216.34/guide")
      .await
      .expect("fetch ok");
    assert_eq!(fetched.title, "DeepSeek Guide");
    assert!(fetched.content.contains("# Welcome"));
    server.await.expect("server");
  }

  #[tokio::test]
  async fn ocr_bytes_uploads_multipart_and_formats_pages_and_surfaces_embedded_error() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
      // Request 1: successful 2-page OCR
      let (mut s1, _) = listener.accept().await.expect("accept 1");
      let mut buf = vec![0u8; 8192];
      let n = s1.read(&mut buf).await.expect("read 1");
      let req1 = String::from_utf8_lossy(&buf[..n]).to_string();
      assert!(
        req1.starts_with("POST /v1/tools/ocr HTTP/1.1"),
        "got: {req1}"
      );
      assert!(req1.contains("rich_layout_recognition"));
      assert!(req1.contains("filename=\"slide.png\""));
      let body1 = r#"{"id":"ocr-1","object":"ocr.result","created":1762790000,"purpose":"rich_layout_recognition","page_count":2,"pages":[{"index":1,"text":"Second page content","finish_reason":"stop"},{"index":0,"text":"First page heading","finish_reason":"stop"}],"usage":{"input_tokens":100,"output_tokens":20,"total_tokens":120}}"#;
      let resp1 = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body1.len(),
        body1
      );
      s1.write_all(resp1.as_bytes()).await.expect("write 1");

      // Request 2: HTTP 200 with embedded `{"error": {"message": ...}}`
      let (mut s2, _) = listener.accept().await.expect("accept 2");
      let _ = s2.read(&mut buf).await.expect("read 2");
      let body2 = r#"{"error":{"message":"unsupported encrypted pdf"}}"#;
      let resp2 = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body2.len(),
        body2
      );
      s2.write_all(resp2.as_bytes()).await.expect("write 2");
    });

    let client = DeepSeekToolsClient::new("sk-ds", Some(format!("http://{addr}/v1")))
      .expect("client")
      .with_no_proxy();
    let png_bytes = b"\x89PNG\r\n\x1a\nfake-image";
    let ok_res = client
      .ocr_bytes(
        png_bytes,
        "slide.png",
        DeepSeekOcrPurpose::RichLayoutRecognition,
      )
      .await
      .expect("ocr ok");
    assert_eq!(ok_res.page_count, 2);
    assert_eq!(ok_res.pages[0].text, "First page heading");
    assert_eq!(ok_res.pages[1].text, "Second page content");
    assert_eq!(
      ok_res.formatted_text(Some(1)),
      "First page heading".to_string()
    );

    let err_res = client
      .ocr_bytes(png_bytes, "enc.pdf", DeepSeekOcrPurpose::TextRecognition)
      .await
      .unwrap_err();
    assert!(err_res.to_string().contains("unsupported encrypted pdf"));
    server.await.expect("server");
  }
}
