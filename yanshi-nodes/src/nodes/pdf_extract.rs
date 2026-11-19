use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  run_context,
  value::FlowValue,
};
use yanshi_tools::SandboxPolicy;
use yanshi_tools::builtin::HttpTool;

use crate::common::{artifacts, sandboxed};

const DEFAULT_MAX_CHARS: usize = 60_000;
const MIN_MAX_CHARS: usize = 1_000;
const MAX_MAX_CHARS: usize = 500_000;
const MAX_PDF_BYTES: usize = 50 * 1024 * 1024;
const HTTP_TIMEOUT_SECS: u64 = 60;
const PDFTOTEXT_TIMEOUT_SECS: u64 = 120;

/// Extract page-structured UTF-8 text from a PDF (`FlowValue::File`, `https?://`
/// URL, `data:application/pdf;base64,...`, or sandbox-allowed local path) using
/// `pdftotext` inside the platform OS sandbox.
#[derive(Clone)]
pub struct PdfExtractNode {
  id: String,
  policy: Arc<SandboxPolicy>,
  no_proxy: bool,
}

impl std::fmt::Debug for PdfExtractNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("PdfExtractNode")
      .field("id", &self.id)
      .finish_non_exhaustive()
  }
}

impl PdfExtractNode {
  pub fn new(id: impl Into<String>, policy: Arc<SandboxPolicy>) -> Self {
    Self {
      id: id.into(),
      policy,
      no_proxy: false,
    }
  }

  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }

  async fn load_pdf_bytes(&self, inputs: &AsyncNodeInputs) -> Result<Vec<u8>, YanshiError> {
    let source = inputs
      .get("source")
      .ok_or_else(|| input_err(&self.id, "missing required input 'source'"))?;

    let bytes = match source {
      FlowValue::File { path, .. } => self.read_allowed_file(path).await?,
      FlowValue::Url { url, .. } => self.download_pdf(url).await?,
      FlowValue::Json(Value::Object(obj)) => {
        let path = obj
          .get("path")
          .and_then(Value::as_str)
          .ok_or_else(|| input_err(&self.id, "'source' object must contain a string 'path'"))?;
        self.read_allowed_file(Path::new(path)).await?
      }
      FlowValue::Json(Value::String(raw)) => {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
          return Err(input_err(&self.id, "'source' must not be empty"));
        }
        if trimmed.starts_with("data:") {
          decode_data_uri(&self.id, trimmed)?
        } else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
          self.download_pdf(trimmed).await?
        } else {
          self.read_allowed_file(Path::new(trimmed)).await?
        }
      }
      other => {
        return Err(input_err(
          &self.id,
          format!("unsupported 'source' value: {other:?}"),
        ));
      }
    };

    validate_pdf_magic(&self.id, &bytes)?;
    Ok(bytes)
  }

  async fn read_allowed_file(&self, path: &Path) -> Result<Vec<u8>, YanshiError> {
    let canonical = resolve_readable_path(&self.id, path, &self.policy)?;
    let meta = tokio::fs::metadata(&canonical).await.map_err(|err| {
      input_err(
        &self.id,
        format!("cannot stat '{}': {err}", canonical.display()),
      )
    })?;
    if meta.len() > MAX_PDF_BYTES as u64 {
      return Err(input_err(
        &self.id,
        format!(
          "PDF file '{}' ({} bytes) exceeds {} MB limit",
          canonical.display(),
          meta.len(),
          MAX_PDF_BYTES / (1024 * 1024)
        ),
      ));
    }
    tokio::fs::read(&canonical).await.map_err(|err| {
      input_err(
        &self.id,
        format!("cannot read '{}': {err}", canonical.display()),
      )
    })
  }

  async fn download_pdf(&self, raw_url: &str) -> Result<Vec<u8>, YanshiError> {
    let url = normalize_pdf_url(raw_url);
    let mut http = HttpTool::new(Arc::clone(&self.policy))
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("{}: failed to build HTTP client: {err}", self.id),
      })?
      .with_timeout(Duration::from_secs(HTTP_TIMEOUT_SECS));
    if self.no_proxy {
      http = http.with_no_proxy();
    }

    let resp = http
      .send_bytes(
        &json!({
          "url": url,
          "method": "GET",
          "headers": {
            "Accept": "application/pdf,application/octet-stream;q=0.9,*/*;q=0.8"
          }
        }),
        Some(MAX_PDF_BYTES),
      )
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("{}: PDF download failed for '{url}': {err}", self.id),
      })?;

    if !resp.status.is_success() {
      let preview: String = String::from_utf8_lossy(&resp.body)
        .chars()
        .take(240)
        .collect();
      return Err(YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: HTTP {} when fetching PDF '{url}': {preview}",
          self.id, resp.status
        ),
      });
    }

    Ok(resp.body)
  }
}

#[async_trait]
impl AsyncNode for PdfExtractNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let max_chars = optional_usize(inputs, "max_chars")
      .unwrap_or(DEFAULT_MAX_CHARS)
      .clamp(MIN_MAX_CHARS, MAX_MAX_CHARS);
    let max_pages = optional_usize(inputs, "max_pages").filter(|p| *p > 0);

    let pdf_bytes = self.load_pdf_bytes(inputs).await?;
    let pdftotext =
      sandboxed::find_tool("pdftotext").ok_or_else(|| YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: pdftotext is not installed (`poppler-utils` / `brew install poppler`)",
          self.id
        ),
      })?;

    let base_dir = artifacts::artifact_dir(&self.id);
    let workdir = base_dir.join(format!(".pdf-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&workdir)
      .await
      .map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!(
          "{}: cannot create scratch dir '{}': {err}",
          self.id,
          workdir.display()
        ),
      })?;

    let extract_result = run_pdftotext(&self.id, &pdftotext, &workdir, &pdf_bytes, max_pages).await;
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    let raw_text = extract_result?;

    let pages = split_pdf_pages(&raw_text);
    let page_count = pages.len();
    let full_text = format_numbered_pages(&pages);
    let (truncated_text, truncated) = truncate_chars(&full_text, max_chars, page_count);

    let mut outputs = HashMap::new();
    outputs.insert("text".to_string(), FlowValue::Json(json!(full_text)));
    outputs.insert(
      "truncated_text".to_string(),
      FlowValue::Json(json!(truncated_text.clone())),
    );
    outputs.insert("output".to_string(), FlowValue::Json(json!(truncated_text)));
    outputs.insert("pages".to_string(), FlowValue::Json(json!(pages)));
    outputs.insert("page_count".to_string(), FlowValue::Json(json!(page_count)));
    outputs.insert("truncated".to_string(), FlowValue::Json(json!(truncated)));
    Ok(outputs)
  }
}

async fn run_pdftotext(
  node_id: &str,
  pdftotext: &Path,
  workdir: &Path,
  pdf_bytes: &[u8],
  max_pages: Option<usize>,
) -> Result<String, YanshiError> {
  let input_pdf = workdir.join("input.pdf");
  let output_txt = workdir.join("output.txt");
  tokio::fs::write(&input_pdf, pdf_bytes)
    .await
    .map_err(|err| YanshiError::NodeExecutionFailed {
      message: format!("{node_id}: cannot stage input.pdf: {err}"),
    })?;

  let mut args: Vec<OsString> = vec!["-layout".into(), "-enc".into(), "UTF-8".into()];
  if let Some(last_page) = max_pages {
    args.push("-l".into());
    args.push(last_page.to_string().into());
  }
  args.push("input.pdf".into());
  args.push("output.txt".into());

  let output = sandboxed::run(sandboxed::SandboxedRun {
    node: node_id,
    program: pdftotext.to_path_buf(),
    args,
    workdir,
    read_paths: sandboxed::font_dirs(),
    write_paths: Vec::new(),
    env: Vec::new(),
    timeout: Duration::from_secs(PDFTOTEXT_TIMEOUT_SECS),
  })
  .await?;

  if !output.status.success() {
    return Err(YanshiError::NodeExecutionFailed {
      message: format!(
        "{node_id}: pdftotext exited with {}: {}",
        output.status,
        sandboxed::output_tail(&output, 12)
      ),
    });
  }

  let text_bytes =
    tokio::fs::read(&output_txt)
      .await
      .map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("{node_id}: cannot read pdftotext output: {err}"),
      })?;
  Ok(String::from_utf8_lossy(&text_bytes).into_owned())
}

fn input_err(node: &str, message: impl std::fmt::Display) -> YanshiError {
  YanshiError::NodeInputError {
    message: format!("{node}: {message}"),
  }
}

fn optional_usize(inputs: &AsyncNodeInputs, key: &str) -> Option<usize> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::Number(num))) => num.as_u64().and_then(|v| usize::try_from(v).ok()),
    Some(FlowValue::Json(Value::String(text))) => text.trim().parse::<usize>().ok(),
    _ => None,
  }
}

fn resolve_readable_path(
  node: &str,
  path: &Path,
  policy: &SandboxPolicy,
) -> Result<PathBuf, YanshiError> {
  if path.components().any(|c| matches!(c, Component::ParentDir)) {
    return Err(input_err(
      node,
      format!(
        "refusing path '{}': parent-directory components ('..') are not allowed",
        path.display()
      ),
    ));
  }

  let canonical = run_context::resolve_confined_path(path)
    .map_err(|msg| input_err(node, msg))?
    .canonicalize()
    .map_err(|err| input_err(node, format!("cannot resolve '{}': {err}", path.display())))?;

  if let Some(root) = run_context::current_artifact_root() {
    let root_canon = root.canonicalize().unwrap_or(root);
    if canonical.starts_with(&root_canon) {
      return Ok(canonical);
    }
  }

  if let Some(reason) = policy.path_denial_reason(&canonical) {
    return Err(YanshiError::NodeInputError {
      message: format!(
        "{node}: local path '{}' denied by sandbox policy ({reason})",
        path.display()
      ),
    });
  }

  Ok(canonical)
}

fn decode_data_uri(node: &str, uri: &str) -> Result<Vec<u8>, YanshiError> {
  let (_, payload) = uri
    .split_once(',')
    .ok_or_else(|| input_err(node, "malformed data URI (missing ',')"))?;
  let clean: String = payload.chars().filter(|c| !c.is_whitespace()).collect();
  let bytes = base64::engine::general_purpose::STANDARD
    .decode(clean)
    .map_err(|err| input_err(node, format!("invalid base64 in data URI: {err}")))?;
  if bytes.len() > MAX_PDF_BYTES {
    return Err(input_err(
      node,
      format!(
        "decoded PDF ({} bytes) exceeds {} MB limit",
        bytes.len(),
        MAX_PDF_BYTES / (1024 * 1024)
      ),
    ));
  }
  Ok(bytes)
}

/// Normalize common academic paper landing URLs (e.g. `https://arxiv.org/abs/1706.03762`
/// or `https://openreview.net/forum?id=...`) into direct PDF download URLs.
pub fn normalize_pdf_url(raw_url: &str) -> String {
  let trimmed = raw_url.trim();
  let Ok(mut parsed) = reqwest::Url::parse(trimmed) else {
    return trimmed.to_string();
  };
  let host = parsed.host_str().unwrap_or_default().to_ascii_lowercase();
  if matches!(
    host.as_str(),
    "arxiv.org" | "www.arxiv.org" | "export.arxiv.org"
  ) {
    let path = parsed.path().trim_end_matches('/').to_string();
    if let Some(paper_id) = path
      .strip_prefix("/abs/")
      .or_else(|| path.strip_prefix("/html/"))
      && !paper_id.is_empty()
    {
      let pdf_id = paper_id.strip_suffix(".pdf").unwrap_or(paper_id);
      parsed.set_path(&format!("/pdf/{pdf_id}.pdf"));
      parsed.set_query(None);
      parsed.set_fragment(None);
      return parsed.to_string();
    }
  } else if host == "openreview.net" && parsed.path() == "/forum" {
    parsed.set_path("/pdf");
    return parsed.to_string();
  }
  trimmed.to_string()
}

fn validate_pdf_magic(node: &str, bytes: &[u8]) -> Result<(), YanshiError> {
  let window = &bytes[..bytes.len().min(1024)];
  if window.windows(5).any(|w| w == b"%PDF-") {
    Ok(())
  } else {
    let preview: String = String::from_utf8_lossy(&bytes[..bytes.len().min(120)])
      .chars()
      .take(80)
      .collect();
    Err(input_err(
      node,
      format!("input is not a valid PDF (missing '%PDF-' header; starts with {preview:?})"),
    ))
  }
}

fn split_pdf_pages(raw_text: &str) -> Vec<String> {
  let normalized = raw_text.replace("\r\n", "\n");
  let mut parts: Vec<String> = normalized
    .split('\x0c')
    .map(collapse_excess_blank_lines)
    .collect();
  if parts.len() > 1 && parts.last().is_some_and(|last| last.is_empty()) {
    parts.pop();
  }
  if parts.len() == 1 && parts[0].is_empty() {
    return Vec::new();
  }
  parts
}

fn collapse_excess_blank_lines(page: &str) -> String {
  let mut out = String::with_capacity(page.len());
  let mut blank_run = 0usize;
  for line in page.lines() {
    let trimmed_end = line.trim_end();
    if trimmed_end.trim().is_empty() {
      blank_run += 1;
      if blank_run <= 1 && !out.is_empty() {
        out.push('\n');
      }
    } else {
      blank_run = 0;
      if !out.is_empty() {
        out.push('\n');
      }
      out.push_str(trimmed_end);
    }
  }
  out.trim().to_string()
}

fn format_numbered_pages(pages: &[String]) -> String {
  pages
    .iter()
    .enumerate()
    .filter(|(_, p)| !p.is_empty())
    .map(|(idx, p)| format!("[Page {}]\n{p}", idx + 1))
    .collect::<Vec<_>>()
    .join("\n\n")
}

fn truncate_chars(text: &str, max_chars: usize, page_count: usize) -> (String, bool) {
  let total_chars = text.chars().count();
  if total_chars <= max_chars {
    return (text.to_string(), false);
  }
  let cut_byte = text
    .char_indices()
    .nth(max_chars)
    .map(|(idx, _)| idx)
    .unwrap_or(text.len());
  let truncated = format!(
    "{}\n\n...[truncated at {max_chars} of {total_chars} chars across {page_count} pages]",
    &text[..cut_byte]
  );
  (truncated, true)
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
  };

  /// Build a minimal standards-compliant multi-page `%PDF-1.4` document
  /// containing the high-bit binary marker `%\xE2\xE3\xCF\xD3` so binary
  /// transport preservation and real `pdftotext` extraction are both tested.
  fn build_minimal_pdf(page_lines: &[&str]) -> Vec<u8> {
    let mut pdf = Vec::<u8>::new();
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let page_count = page_lines.len();
    // Object layout:
    // 1: Catalog
    // 2: Pages
    // 3: Font (Helvetica)
    // 4 + 2*i: Page i
    // 5 + 2*i: Content stream i
    let total_objs = 3 + page_count * 2;
    let mut offsets = vec![0usize; total_objs + 1];

    let push_obj = |buf: &mut Vec<u8>, offsets: &mut [usize], id: usize, body: &[u8]| {
      offsets[id] = buf.len();
      buf.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
      buf.extend_from_slice(body);
      if !body.ends_with(b"\n") {
        buf.push(b'\n');
      }
      buf.extend_from_slice(b"endobj\n");
    };

    push_obj(
      &mut pdf,
      &mut offsets,
      1,
      b"<< /Type /Catalog /Pages 2 0 R >>",
    );

    let kids: String = (0..page_count)
      .map(|i| format!("{} 0 R", 4 + i * 2))
      .collect::<Vec<_>>()
      .join(" ");
    push_obj(
      &mut pdf,
      &mut offsets,
      2,
      format!("<< /Type /Pages /Kids [{kids}] /Count {page_count} >>").as_bytes(),
    );

    push_obj(
      &mut pdf,
      &mut offsets,
      3,
      b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
    );

    for (i, text) in page_lines.iter().enumerate() {
      let page_id = 4 + i * 2;
      let stream_id = 5 + i * 2;
      let escaped = text
        .replace('\\', "\\\\")
        .replace('(', "\\(")
        .replace(')', "\\)");
      let stream_data = format!("BT /F1 14 Tf 72 720 Td ({escaped}) Tj ET");
      let page_body = format!(
        "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Resources << /Font << /F1 3 0 R >> >> /Contents {stream_id} 0 R >>"
      );
      push_obj(&mut pdf, &mut offsets, page_id, page_body.as_bytes());

      let stream_obj = format!(
        "<< /Length {} >>\nstream\n{}\nendstream",
        stream_data.len(),
        stream_data
      );
      push_obj(&mut pdf, &mut offsets, stream_id, stream_obj.as_bytes());
    }

    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", total_objs + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
      pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
      format!(
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
        total_objs + 1
      )
      .as_bytes(),
    );
    pdf
  }

  #[test]
  fn normalizes_arxiv_and_openreview_urls() {
    assert_eq!(
      normalize_pdf_url("https://arxiv.org/abs/1706.03762"),
      "https://arxiv.org/pdf/1706.03762.pdf"
    );
    assert_eq!(
      normalize_pdf_url("https://arxiv.org/html/2401.00001v2#S1"),
      "https://arxiv.org/pdf/2401.00001v2.pdf"
    );
    assert_eq!(
      normalize_pdf_url("https://openreview.net/forum?id=abc123"),
      "https://openreview.net/pdf?id=abc123"
    );
  }

  #[tokio::test]
  async fn extracts_pages_and_downloads_binary_pdf_over_http() {
    if sandboxed::find_tool("pdftotext").is_none() {
      return;
    }

    let pdf_bytes = build_minimal_pdf(&[
      "Attention Is All You Need - Transformer Architecture",
      "Table 2: WMT 2014 EN-DE BLEU 28.4 with 65M params",
    ]);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let served_bytes = pdf_bytes.clone();
    let server = tokio::spawn(async move {
      let (mut stream, _) = listener.accept().await.unwrap();
      let mut req_buf = [0_u8; 2048];
      let _ = stream.read(&mut req_buf).await.unwrap();
      let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/pdf\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        served_bytes.len()
      );
      stream.write_all(header.as_bytes()).await.unwrap();
      stream.write_all(&served_bytes).await.unwrap();
    });

    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let node = PdfExtractNode::new("paper_pdf", policy).with_no_proxy();
    let mut inputs = HashMap::new();
    inputs.insert(
      "source".to_string(),
      FlowValue::Json(json!(format!("http://{addr}/paper.pdf"))),
    );

    let outputs = node.execute(&inputs).await.unwrap();
    server.await.unwrap();

    assert_eq!(outputs["page_count"], FlowValue::Json(json!(2)));
    assert_eq!(outputs["truncated"], FlowValue::Json(json!(false)));
    let FlowValue::Json(Value::String(ref full_text)) = outputs["output"] else {
      panic!("expected string output");
    };
    assert!(full_text.contains("[Page 1]"));
    assert!(full_text.contains("Attention Is All You Need"));
    assert!(full_text.contains("[Page 2]"));
    assert!(full_text.contains("BLEU 28.4"));
  }

  #[tokio::test]
  async fn rejects_unallowed_local_path_and_supports_data_uri() {
    let dir = tempfile::tempdir().unwrap();
    let pdf_path = dir.path().join("sample.pdf");
    let pdf_bytes = build_minimal_pdf(&["Single page summary"]);
    std::fs::write(&pdf_path, &pdf_bytes).unwrap();

    // Default SandboxPolicy has empty allowed_paths -> local host path rejected.
    let denied_node = PdfExtractNode::new("denied_pdf", Arc::new(SandboxPolicy::default()));
    let mut denied_inputs = HashMap::new();
    denied_inputs.insert(
      "source".to_string(),
      FlowValue::Json(json!(pdf_path.to_string_lossy())),
    );
    let err = denied_node.execute(&denied_inputs).await.unwrap_err();
    assert!(err.to_string().contains("denied by sandbox policy"));

    if sandboxed::find_tool("pdftotext").is_none() {
      return;
    }

    let b64 = base64::engine::general_purpose::STANDARD.encode(&pdf_bytes);
    let data_uri = format!("data:application/pdf;base64,{b64}");
    let mut uri_inputs = HashMap::new();
    uri_inputs.insert("source".to_string(), FlowValue::Json(json!(data_uri)));
    let out = denied_node.execute(&uri_inputs).await.unwrap();
    assert_eq!(out["page_count"], FlowValue::Json(json!(1)));
  }
}
