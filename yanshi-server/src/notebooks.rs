//! Notebooks and their sources (Y5.2, `docs/RFC_CONTENT_STUDIO.md` §6).
//!
//! ```text
//! POST   /v1/notebooks                              {title}              → notebook
//! GET    /v1/notebooks                                                   → {notebooks}
//! GET    /v1/notebooks/{id}                                              → {notebook, sources}
//! DELETE /v1/notebooks/{id}                                              → 204 No Content
//! POST   /v1/notebooks/{id}/sources                 {kind, ...}          → source (pending; ingests in the background)
//! DELETE /v1/notebooks/{id}/sources/{source_id}                          → 204 No Content
//! GET    /v1/notebooks/{id}/chunks/{chunk_id}                            → chunk
//! GET    /v1/notebooks/{id}/search?q=&top_k=                             → {chunks}
//! POST   /v1/notebooks/{id}/generate                {kind, params, model} → {run_id, kind}
//! GET    /v1/notebooks/{id}/artifacts                                    → {artifacts}
//! ```
//!
//! Ingestion: fetch (`url`, through the http tool's SSRF filter: the URL
//! is tenant-supplied), or take the pasted `text`, or decode an uploaded
//! `file` (`.txt`, `.md`, `.html`, `.pdf`, `.docx`, `.csv`, `.tsv`, `.json`),
//! or transcribe an uploaded audio `recording` via a configured ASR model;
//! extract text (HTML and PDF via the RAG loaders with `pdftotext` fallback,
//! DOCX via pure-Rust WordprocessingML extraction, CSV/TSV/JSON with header
//! labels preserved per row); chunk by paragraph; store the chunks in
//! `source_chunks`; mark the source `ready` (or `failed`, with the reason).
//!
//! The index is derived, not stored: a notebook's BM25 index is built
//! from its ready chunks on first search, cached in this process, and
//! dropped whenever one of its sources finishes ingesting. A restart
//! therefore costs one rebuild per notebook, nothing more.

use std::collections::HashMap;
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex};

use axum::{
  Extension, Json,
  extract::{Path, Query, State},
  http::StatusCode,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use yanshi_db::{
  NewSource, Notebook, NotebookRepo, Source, SourceChunk, SourceChunkRepo, SourceKind, SourceRepo,
  SourceStatus,
};
use yanshi_rag::{Bm25KnowledgeBackend, KnowledgeBackend};

use crate::{AppState, error::ApiError, tenant::TenantId};

/// Uploaded / pasted sources larger than this are refused at submit.
const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Uncompressed XML read ceiling when unpacking `.docx` archives (zip-bomb guard).
const MAX_DOCX_XML_BYTES: usize = 16 * 1024 * 1024;
/// Paragraph chunking: target size and overlap, in characters.
const CHUNK_SIZE: usize = 1200;
const CHUNK_OVERLAP: usize = 150;

/// Per-notebook BM25 indexes, built on first search.
#[derive(Clone, Default)]
pub struct NotebookIndexCache(Arc<Mutex<HashMap<Uuid, Arc<NotebookIndex>>>>);

/// A notebook's index and the chunks it was built from (search results
/// carry the source id, which BM25's metadata does not round-trip).
pub struct NotebookIndex {
  backend: Bm25KnowledgeBackend,
  chunks: HashMap<String, SourceChunk>,
}

impl NotebookIndexCache {
  fn get(&self, notebook_id: Uuid) -> Option<Arc<NotebookIndex>> {
    self
      .0
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .get(&notebook_id)
      .cloned()
  }

  fn put(&self, notebook_id: Uuid, index: Arc<NotebookIndex>) {
    self
      .0
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .insert(notebook_id, index);
  }

  /// Drop a notebook's index (a source changed).
  pub fn invalidate(&self, notebook_id: Uuid) {
    self
      .0
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .remove(&notebook_id);
  }
}

impl std::fmt::Debug for NotebookIndexCache {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str("NotebookIndexCache")
  }
}

#[derive(Debug, Deserialize)]
pub struct CreateNotebookRequest {
  pub title: String,
}

pub async fn create_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Json(req): Json<CreateNotebookRequest>,
) -> Result<Json<Notebook>, ApiError> {
  let title = req.title.trim();
  if title.is_empty() {
    return Err(ApiError::BadRequest("title must not be empty".into()));
  }
  Ok(Json(
    state.repos.notebooks.create(tenant.as_str(), title).await?,
  ))
}

pub async fn list_notebooks(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
) -> Result<Json<Value>, ApiError> {
  let notebooks = state.repos.notebooks.list(tenant.as_str(), 100).await?;
  Ok(Json(json!({ "notebooks": notebooks })))
}

async fn own_notebook(state: &AppState, tenant: &TenantId, id: Uuid) -> Result<Notebook, ApiError> {
  state
    .repos
    .notebooks
    .get(tenant.as_str(), id)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("notebook {id} not found")))
}

pub async fn get_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
  let notebook = own_notebook(&state, &tenant, id).await?;
  let sources = state
    .repos
    .sources
    .list_for_notebook(tenant.as_str(), id)
    .await?;
  Ok(Json(json!({ "notebook": notebook, "sources": sources })))
}

pub async fn delete_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
  let deleted = state.repos.notebooks.delete(tenant.as_str(), id).await?;
  if !deleted {
    return Err(ApiError::NotFound(format!("notebook {id} not found")));
  }
  state.notebook_indexes.invalidate(id);
  Ok(StatusCode::NO_CONTENT)
}

pub async fn delete_source(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, source_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
  let deleted = state
    .repos
    .sources
    .delete(tenant.as_str(), notebook_id, source_id)
    .await?;
  if !deleted {
    return Err(ApiError::NotFound(format!(
      "source {source_id} in notebook {notebook_id} not found"
    )));
  }
  state.notebook_indexes.invalidate(notebook_id);
  Ok(StatusCode::NO_CONTENT)
}

pub async fn get_notebook_chunk(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, chunk_id)): Path<(Uuid, String)>,
) -> Result<Json<SourceChunk>, ApiError> {
  own_notebook(&state, &tenant, notebook_id).await?;
  let chunk = state
    .repos
    .chunks
    .get(tenant.as_str(), chunk_id.trim())
    .await?
    .filter(|c| c.notebook_id == notebook_id)
    .ok_or_else(|| {
      ApiError::NotFound(format!(
        "chunk '{chunk_id}' in notebook {notebook_id} not found"
      ))
    })?;
  Ok(Json(chunk))
}

/// `POST /v1/notebooks/{id}/sources` body.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AddSourceRequest {
  Url {
    url: String,
    #[serde(default)]
    title: Option<String>,
  },
  Text {
    text: String,
    #[serde(default)]
    title: Option<String>,
  },
  /// An uploaded document (`.txt`, `.md`, `.html`, `.pdf`, `.docx`, `.csv`,
  /// `.tsv`, `.json`), base64-encoded.
  File {
    filename: String,
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
  },
  /// An uploaded audio recording (`.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac`,
  /// `.webm`, `.aac`), base64-encoded and transcribed via ASR.
  Recording {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
    /// Registry name of the ASR model; falls back to `YANSHI_ASR_MODEL`.
    #[serde(default)]
    model: Option<String>,
    /// Optional BCP-47 / ISO-639 language hint passed to the ASR provider.
    #[serde(default)]
    language: Option<String>,
  },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BytesFormat {
  PlainText,
  Html,
  Pdf,
  Docx,
  Csv,
  Tsv,
  Json,
}

/// What ingestion starts from, resolved at submit time.
enum Payload {
  Url(String),
  Bytes {
    bytes: Vec<u8>,
    format: BytesFormat,
  },
  Recording {
    bytes: Vec<u8>,
    filename: String,
    model: String,
    language: Option<String>,
  },
}

pub async fn add_source(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Json(req): Json<AddSourceRequest>,
) -> Result<Json<Source>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let (kind, title, uri, payload) = match req {
    AddSourceRequest::Url { url, title } => {
      // The http tool parses and SSRF-checks the URL at fetch time;
      // here only the scheme is checked so a typo fails at submit.
      let lowered = url.trim().to_ascii_lowercase();
      if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return Err(ApiError::BadRequest(
          "url must start with http:// or https://".into(),
        ));
      }
      (
        SourceKind::Url,
        title
          .filter(|t| !t.trim().is_empty())
          .unwrap_or_else(|| url.clone()),
        Some(url.clone()),
        Payload::Url(url),
      )
    }
    AddSourceRequest::Text { text, title } => {
      if text.trim().is_empty() {
        return Err(ApiError::BadRequest("text must not be empty".into()));
      }
      if text.len() > MAX_SOURCE_BYTES {
        return Err(ApiError::PayloadTooLarge(format!(
          "text is larger than {MAX_SOURCE_BYTES} bytes"
        )));
      }
      (
        SourceKind::Text,
        title
          .filter(|t| !t.trim().is_empty())
          .unwrap_or_else(|| text.chars().take(60).collect()),
        None,
        Payload::Bytes {
          bytes: text.into_bytes(),
          format: BytesFormat::PlainText,
        },
      )
    }
    AddSourceRequest::File {
      filename,
      content_base64,
      title,
    } => {
      use base64::Engine as _;
      let extension = FsPath::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
      let format = match extension.as_str() {
        "html" | "htm" => BytesFormat::Html,
        "txt" | "md" | "markdown" => BytesFormat::PlainText,
        "pdf" => BytesFormat::Pdf,
        "docx" => BytesFormat::Docx,
        "csv" => BytesFormat::Csv,
        "tsv" => BytesFormat::Tsv,
        "json" => BytesFormat::Json,
        other => {
          return Err(ApiError::BadRequest(format!(
            "unsupported file type '.{other}': upload .txt, .md, .html, .pdf, .docx, .csv, .tsv or .json"
          )));
        }
      };
      let bytes = base64::engine::general_purpose::STANDARD
        .decode(content_base64.trim().as_bytes())
        .map_err(|e| ApiError::BadRequest(format!("content_base64: {e}")))?;
      if bytes.is_empty() {
        return Err(ApiError::BadRequest("file must not be empty".into()));
      }
      if bytes.len() > MAX_SOURCE_BYTES {
        return Err(ApiError::PayloadTooLarge(format!(
          "file is larger than {MAX_SOURCE_BYTES} bytes"
        )));
      }
      if format == BytesFormat::Pdf && !bytes.starts_with(b"%PDF-") {
        return Err(ApiError::BadRequest(
          "file is not a valid PDF (missing %PDF- header)".into(),
        ));
      }
      if format == BytesFormat::Docx && !bytes.starts_with(b"PK\x03\x04") {
        return Err(ApiError::BadRequest(
          "file is not a valid DOCX (missing PK zip header)".into(),
        ));
      }
      let resolved_title = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| filename.clone());
      (
        SourceKind::File,
        resolved_title,
        Some(filename),
        Payload::Bytes { bytes, format },
      )
    }
    AddSourceRequest::Recording {
      filename,
      content_base64,
      title,
      model,
      language,
    } => {
      use base64::Engine as _;
      let filename = filename.trim().to_string();
      if filename.is_empty() {
        return Err(ApiError::BadRequest(
          "recording filename must not be empty".into(),
        ));
      }
      let extension = FsPath::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
      if !matches!(
        extension.as_str(),
        "mp3" | "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "mp4"
      ) {
        return Err(ApiError::BadRequest(format!(
          "unsupported recording type '.{extension}': upload .mp3, .wav, .m4a, .ogg, .flac, .webm or .aac"
        )));
      }
      let bytes = base64::engine::general_purpose::STANDARD
        .decode(content_base64.trim().as_bytes())
        .map_err(|e| ApiError::BadRequest(format!("content_base64: {e}")))?;
      if bytes.is_empty() {
        return Err(ApiError::BadRequest(
          "recording audio must not be empty".into(),
        ));
      }
      if bytes.len() > MAX_SOURCE_BYTES {
        return Err(ApiError::PayloadTooLarge(format!(
          "recording is larger than {MAX_SOURCE_BYTES} bytes"
        )));
      }
      let model = model
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .or_else(|| {
          std::env::var("YANSHI_ASR_MODEL")
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
        })
        .ok_or_else(|| {
          ApiError::BadRequest("set `model`, or YANSHI_ASR_MODEL on the gateway".into())
        })?;
      let resolved_title = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| filename.clone());
      (
        SourceKind::Recording,
        resolved_title,
        Some(filename.clone()),
        Payload::Recording {
          bytes,
          filename,
          model,
          language,
        },
      )
    }
  };
  let source = state
    .repos
    .sources
    .create(NewSource {
      notebook_id: id,
      tenant_id: tenant.as_str().to_string(),
      kind,
      title,
      uri,
    })
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("notebook {id} not found")))?;
  tokio::spawn(ingest(state.clone(), source.clone(), payload));
  Ok(Json(source))
}

/// Fetch / decode / transcribe → text → chunks → `source_chunks`, then
/// mark the source ready (or failed with the reason) and drop the
/// notebook's cached index.
async fn ingest(state: AppState, source: Source, payload: Payload) {
  let sources = &state.repos.sources;
  let _ = sources
    .update_status(source.id, SourceStatus::Ingesting, None, None)
    .await;
  let outcome = async {
    let (digest, text) = match payload {
      Payload::Url(url) => {
        let (bytes, format) = fetch(&url).await?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let text = extract_bytes_text(bytes, format).await?;
        (digest, text)
      }
      Payload::Bytes { bytes, format } => {
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let text = extract_bytes_text(bytes, format).await?;
        (digest, text)
      }
      Payload::Recording {
        bytes,
        filename,
        model,
        language,
      } => {
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let text = transcribe_recording(bytes, filename, model, language).await?;
        (digest, text)
      }
    };
    let chunks = chunk(&text)?;
    if chunks.is_empty() {
      return Err("the source has no text".to_string());
    }
    state
      .repos
      .chunks
      .replace_for_source(&source, chunks)
      .await
      .map_err(|e| format!("storing chunks failed: {e}"))?;
    Ok::<_, String>(digest)
  }
  .await;
  let _ = match outcome {
    Ok(digest) => {
      sources
        .update_status(source.id, SourceStatus::Ready, None, Some(&digest))
        .await
    }
    Err(reason) => {
      tracing::warn!(source = %source.id, %reason, "source ingestion failed");
      sources
        .update_status(source.id, SourceStatus::Failed, Some(&reason), None)
        .await
    }
  };
  state.notebook_indexes.invalidate(source.notebook_id);
}

async fn extract_bytes_text(bytes: Vec<u8>, format: BytesFormat) -> Result<String, String> {
  match format {
    BytesFormat::Html => html_text(&bytes).await,
    BytesFormat::Pdf => pdf_text(&bytes).await,
    BytesFormat::Docx => docx_text(&bytes),
    BytesFormat::Csv => delimited_text(&bytes, b','),
    BytesFormat::Tsv => delimited_text(&bytes, b'\t'),
    BytesFormat::Json => json_text(&bytes),
    BytesFormat::PlainText => {
      String::from_utf8(bytes).map_err(|_| "the source is not UTF-8 text".to_string())
    }
  }
}

fn url_path_extension(raw_url: &str) -> Option<String> {
  let path = url::Url::parse(raw_url)
    .ok()
    .map(|u| u.path().to_string())
    .unwrap_or_else(|| {
      raw_url
        .split(['?', '#'])
        .next()
        .unwrap_or_default()
        .to_string()
    });
  FsPath::new(&path)
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_ascii_lowercase)
}

/// GET `url` through the http tool's default policy, which refuses
/// loopback, private and cloud-metadata addresses.
async fn fetch(url: &str) -> Result<(Vec<u8>, BytesFormat), String> {
  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|e| format!("http client: {e}"))?;
  let response = tool
    .send_bytes(
      &json!({ "url": url, "method": "GET" }),
      Some(MAX_SOURCE_BYTES),
    )
    .await
    .map_err(|e| format!("fetching {url} failed: {e}"))?;
  if !response.status.is_success() {
    return Err(format!("fetching {url} returned HTTP {}", response.status));
  }
  let bytes = response.body;
  let ext = url_path_extension(url);
  let format = if bytes.starts_with(b"%PDF-") {
    BytesFormat::Pdf
  } else if bytes.starts_with(b"PK\x03\x04") && ext.as_deref() == Some("docx") {
    BytesFormat::Docx
  } else {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    let lowered = sample.trim_start().to_ascii_lowercase();
    if lowered.starts_with("<!doctype html") || lowered.contains("<html") {
      BytesFormat::Html
    } else if ext.as_deref() == Some("csv") {
      BytesFormat::Csv
    } else if ext.as_deref() == Some("tsv") {
      BytesFormat::Tsv
    } else if ext.as_deref() == Some("json") {
      BytesFormat::Json
    } else {
      BytesFormat::PlainText
    }
  };
  Ok((bytes, format))
}

/// Visible text of an HTML document (scripts and styles dropped).
async fn html_text(bytes: &[u8]) -> Result<String, String> {
  use yanshi_rag::sources::{DocumentLoader, html::HtmlLoader};
  let path = std::env::temp_dir().join(format!("yanshi-ingest-{}.html", Uuid::new_v4()));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|e| format!("staging the page: {e}"))?;
  let loaded = HtmlLoader::new().load(&path).await;
  let _ = tokio::fs::remove_file(&path).await;
  loaded
    .map(|document| document.content)
    .map_err(|e| format!("extracting text: {e}"))
}

/// Text of a PDF document: pure-Rust `PdfLoader` first, falling back to
/// `pdftotext` when installed if the pure-Rust extractor yields no text.
async fn pdf_text(bytes: &[u8]) -> Result<String, String> {
  use yanshi_rag::sources::{DocumentLoader, pdf::PdfLoader};
  let id = Uuid::new_v4();
  let path = std::env::temp_dir().join(format!("yanshi-ingest-{id}.pdf"));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|e| format!("staging the PDF: {e}"))?;
  let pure_rust = PdfLoader::new()
    .with_max_bytes(Some(MAX_SOURCE_BYTES as u64))
    .load(&path)
    .await;
  if let Ok(doc) = &pure_rust
    && !doc.content.trim().is_empty()
  {
    let _ = tokio::fs::remove_file(&path).await;
    return Ok(doc.content.clone());
  }
  let fallback = pdftotext_fallback(&path, id).await;
  let _ = tokio::fs::remove_file(&path).await;
  if let Ok(text) = fallback
    && !text.trim().is_empty()
  {
    return Ok(text);
  }
  pure_rust
    .map(|doc| doc.content)
    .map_err(|e| format!("extracting PDF text: {e}"))
}

async fn pdftotext_fallback(pdf_path: &FsPath, id: Uuid) -> Result<String, String> {
  let out_path = std::env::temp_dir().join(format!("yanshi-ingest-{id}.txt"));
  let run = tokio::time::timeout(
    std::time::Duration::from_secs(60),
    tokio::process::Command::new("pdftotext")
      .args(["-layout", "-enc", "UTF-8"])
      .arg(pdf_path)
      .arg(&out_path)
      .output(),
  )
  .await;
  let result = match run {
    Ok(Ok(output)) if output.status.success() => tokio::fs::read_to_string(&out_path)
      .await
      .map_err(|e| format!("reading pdftotext output: {e}")),
    Ok(Ok(output)) => Err(format!(
      "pdftotext exited with {}: {}",
      output.status,
      String::from_utf8_lossy(&output.stderr)
    )),
    Ok(Err(e)) => Err(format!("spawning pdftotext: {e}")),
    Err(_) => Err("pdftotext timed out".to_string()),
  };
  let _ = tokio::fs::remove_file(&out_path).await;
  result
}

/// Pure-Rust OOXML `.docx` text extraction (`word/document.xml`, plus optional
/// `word/footnotes.xml` and `word/endnotes.xml`), preserving paragraph breaks
/// and table rows (`| cell1 | cell2 |`) while bounding uncompressed XML bytes
/// against zip-bomb payloads.
fn docx_text(bytes: &[u8]) -> Result<String, String> {
  use std::io::Read as _;
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid DOCX archive: {e}"))?;

  let mut budget_remaining = MAX_DOCX_XML_BYTES;
  let mut sections: Vec<String> = Vec::new();
  for entry_name in [
    "word/document.xml",
    "word/footnotes.xml",
    "word/endnotes.xml",
  ] {
    let mut entry = match archive.by_name(entry_name) {
      Ok(file) => file,
      Err(zip::result::ZipError::FileNotFound) => {
        if entry_name == "word/document.xml" {
          return Err("invalid DOCX archive: missing word/document.xml".to_string());
        }
        continue;
      }
      Err(e) => return Err(format!("reading {entry_name} failed: {e}")),
    };
    let mut xml_bytes = Vec::new();
    entry
      .by_ref()
      .take((budget_remaining as u64).saturating_add(1))
      .read_to_end(&mut xml_bytes)
      .map_err(|e| format!("decompressing {entry_name} failed: {e}"))?;
    if xml_bytes.len() > budget_remaining {
      return Err(format!(
        "DOCX uncompressed XML exceeds {MAX_DOCX_XML_BYTES} bytes safety limit"
      ));
    }
    budget_remaining -= xml_bytes.len();
    let xml =
      String::from_utf8(xml_bytes).map_err(|_| format!("{entry_name} is not valid UTF-8 XML"))?;
    let extracted = extract_wordprocessingml_text(&xml);
    if !extracted.trim().is_empty() {
      sections.push(extracted);
    }
  }
  Ok(sections.join("\n\n"))
}

fn extract_wordprocessingml_text(xml: &str) -> String {
  let mut blocks: Vec<String> = Vec::new();
  let mut current_para = String::new();
  let mut in_text_run = false;
  let mut table_depth: usize = 0;
  let mut current_row: Vec<String> = Vec::new();
  let mut current_cell_paras: Vec<String> = Vec::new();

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_text_run && lt > 0 {
      current_para.push_str(&decode_xml_entities(&rest[..lt]));
    }
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let is_closing = raw_tag.starts_with('/');
    let is_self_closing = raw_tag.ends_with('/');
    let tag_body = raw_tag.trim_start_matches('/').trim_end_matches('/').trim();
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();

    match (tag_name, is_closing, is_self_closing) {
      ("w:t" | "t", false, false) => {
        in_text_run = true;
      }
      ("w:t" | "t", true, _) => {
        in_text_run = false;
      }
      ("w:tab", false, _) => {
        current_para.push('\t');
      }
      ("w:br" | "w:cr", false, _) => {
        current_para.push('\n');
      }
      ("w:tbl", false, false) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          blocks.push(trimmed.to_string());
        }
        current_para.clear();
        table_depth += 1;
      }
      ("w:tbl", true, _) => {
        table_depth = table_depth.saturating_sub(1);
      }
      ("w:tr", false, false) if table_depth > 0 => {
        current_row.clear();
      }
      ("w:tc", false, false) if table_depth > 0 => {
        current_cell_paras.clear();
        current_para.clear();
      }
      ("w:p", true, _) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          if table_depth > 0 {
            current_cell_paras.push(trimmed.to_string());
          } else {
            blocks.push(trimmed.to_string());
          }
        }
        current_para.clear();
      }
      ("w:tc", true, _) if table_depth > 0 => {
        current_row.push(current_cell_paras.join(" "));
        current_cell_paras.clear();
      }
      ("w:tr", true, _) if table_depth > 0 => {
        if current_row.iter().any(|c| !c.trim().is_empty()) {
          blocks.push(format!("| {} |", current_row.join(" | ")));
        }
        current_row.clear();
      }
      _ => {}
    }
  }
  blocks.join("\n\n")
}

fn decode_xml_entities(input: &str) -> String {
  if !input.contains('&') {
    return input.to_string();
  }
  let mut out = String::with_capacity(input.len());
  let mut rest = input;
  while let Some(amp) = rest.find('&') {
    out.push_str(&rest[..amp]);
    let after = &rest[amp + 1..];
    if let Some(semi) = after.find(';') {
      let entity = &after[..semi];
      let decoded = match entity {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ if entity.starts_with("#x") || entity.starts_with("#X") => {
          u32::from_str_radix(&entity[2..], 16)
            .ok()
            .and_then(char::from_u32)
        }
        _ if entity.starts_with('#') => entity[1..].parse::<u32>().ok().and_then(char::from_u32),
        _ => None,
      };
      if let Some(ch) = decoded {
        out.push(ch);
        rest = &after[semi + 1..];
        continue;
      }
    }
    out.push('&');
    rest = after;
  }
  out.push_str(rest);
  out
}

/// Format CSV / TSV tabular bytes into self-describing paragraph blocks
/// (`Header: Value | ...`) so every chunk retains column labels during BM25
/// search and downstream Studio generation.
fn delimited_text(bytes: &[u8], delimiter: u8) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "tabular file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let mut reader = csv::ReaderBuilder::new()
    .delimiter(delimiter)
    .flexible(true)
    .from_reader(raw.as_bytes());
  let headers: Vec<String> = reader
    .headers()
    .map_err(|e| format!("parsing table header failed: {e}"))?
    .iter()
    .map(|h| h.trim().to_string())
    .collect();
  if headers.is_empty() {
    return Err("tabular file has no columns".to_string());
  }

  let mut lines: Vec<String> = Vec::new();
  lines.push(format!("Columns: {}", headers.join(" | ")));

  let mut current_group: Vec<String> = Vec::new();
  for (row_idx, record) in reader.records().enumerate() {
    let record = record.map_err(|e| format!("parsing row {} failed: {e}", row_idx + 1))?;
    let pairs: Vec<String> = record
      .iter()
      .enumerate()
      .filter_map(|(col_idx, cell)| {
        let val = cell.trim();
        if val.is_empty() {
          return None;
        }
        let col_label = headers
          .get(col_idx)
          .filter(|h| !h.is_empty())
          .cloned()
          .unwrap_or_else(|| format!("col_{}", col_idx + 1));
        Some(format!("{col_label}: {val}"))
      })
      .collect();
    if !pairs.is_empty() {
      current_group.push(format!("Row {}: {}", row_idx + 1, pairs.join(" | ")));
      if current_group.len() >= 10 {
        lines.push(current_group.join("\n"));
        current_group.clear();
      }
    }
  }
  if !current_group.is_empty() {
    lines.push(current_group.join("\n"));
  }
  Ok(lines.join("\n\n"))
}

/// Format structured JSON into paragraph-separated records for chunking & BM25.
fn json_text(bytes: &[u8]) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "JSON file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let parsed: Value = serde_json::from_str(raw).map_err(|e| format!("invalid JSON: {e}"))?;
  match parsed {
    Value::Array(items) => {
      let mut blocks = Vec::with_capacity(items.len());
      for (idx, item) in items.into_iter().enumerate() {
        match item {
          Value::Object(map) => {
            let pairs: Vec<String> = map
              .into_iter()
              .filter(|(_, v)| !v.is_null())
              .map(|(k, v)| match v {
                Value::String(s) => format!("{k}: {s}"),
                other => format!("{k}: {other}"),
              })
              .collect();
            if !pairs.is_empty() {
              blocks.push(format!("Record {}: {}", idx + 1, pairs.join(" | ")));
            }
          }
          other => {
            blocks.push(format!(
              "Record {}: {}",
              idx + 1,
              serde_json::to_string_pretty(&other).unwrap_or_default()
            ));
          }
        }
      }
      Ok(blocks.join("\n\n"))
    }
    other => serde_json::to_string_pretty(&other).map_err(|e| format!("formatting JSON: {e}")),
  }
}

/// Transcribe an uploaded audio recording via a registered ASR model.
async fn transcribe_recording(
  bytes: Vec<u8>,
  filename: String,
  model: String,
  language: Option<String>,
) -> Result<String, String> {
  yanshi_llm::Yanshi::ensure_init()
    .await
    .map_err(|e| format!("initializing model registry: {e}"))?;
  let provider = yanshi_llm::Yanshi::asr(&model)
    .await
    .map_err(|e| format!("resolving ASR model '{model}': {e}"))?;
  let response = provider
    .transcribe(yanshi_llm::AsrRequest {
      model,
      response_format: "text".to_string(),
      audio_data: bytes,
      filename,
      language: language
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty()),
      temperature: None,
      prompt: None,
    })
    .await
    .map_err(|e| format!("transcribing recording failed: {e}"))?;
  Ok(response.text)
}

fn chunk(text: &str) -> Result<Vec<(String, Value)>, String> {
  use yanshi_rag::chunking::{ChunkingStrategy, ParagraphChunker};
  let chunks = ParagraphChunker::new(CHUNK_SIZE, CHUNK_OVERLAP)
    .chunk(text)
    .map_err(|e| format!("chunking failed: {e}"))?;
  Ok(
    chunks
      .into_iter()
      .filter(|c| !c.content.trim().is_empty())
      // ParagraphChunker's indices are source line numbers.
      .map(|c| {
        (
          c.content,
          json!({ "start_line": c.start_idx, "end_line": c.end_idx }),
        )
      })
      .collect(),
  )
}

#[derive(Debug, Deserialize)]
pub struct SearchQuery {
  pub q: String,
  #[serde(default)]
  pub top_k: Option<usize>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
  pub source_id: Uuid,
  pub chunk_id: String,
  pub content: String,
  pub score: f32,
}

pub async fn search_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Query(query): Query<SearchQuery>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  if query.q.trim().is_empty() {
    return Err(ApiError::BadRequest("q must not be empty".into()));
  }
  let index = match state.notebook_indexes.get(id) {
    Some(index) => index,
    None => {
      let chunks = state
        .repos
        .chunks
        .list_ready_for_notebook(tenant.as_str(), id)
        .await?;
      let backend = Bm25KnowledgeBackend::from_documents(
        chunks
          .iter()
          .map(|c| (c.chunk_id.clone(), c.content.clone())),
      )
      .with_name(format!("notebook:{id}"));
      let index = Arc::new(NotebookIndex {
        backend,
        chunks: chunks
          .into_iter()
          .map(|c| (c.chunk_id.clone(), c))
          .collect(),
      });
      state.notebook_indexes.put(id, index.clone());
      index
    }
  };
  if index.chunks.is_empty() {
    return Ok(Json(json!({ "chunks": [] })));
  }
  let top_k = query.top_k.unwrap_or(5).clamp(1, 50);
  let hits: Vec<SearchHit> = index
    .backend
    .search(&query.q, top_k)
    .await
    .map_err(|e| ApiError::Internal(format!("search failed: {e}")))?
    .into_iter()
    .filter_map(|hit| {
      let chunk = index.chunks.get(&hit.id)?;
      Some(SearchHit {
        source_id: chunk.source_id,
        chunk_id: chunk.chunk_id.clone(),
        content: chunk.content.clone(),
        score: hit.score,
      })
    })
    .collect();
  Ok(Json(json!({ "chunks": hits })))
}

// ── generate (Y5.4) ─────────────────────────────────────────────────────

/// The source-grounded generator catalogue (RFC §3 + Studio flagship
/// workflows), compiled in so the gateway serves the same templates the CLI
/// runs from `examples/generators/`.
const GENERATORS: &[(&str, &str)] = &[
  (
    "report",
    include_str!("../../examples/generators/report.yml"),
  ),
  (
    "mindmap",
    include_str!("../../examples/generators/mindmap.yml"),
  ),
  (
    "audio_overview",
    include_str!("../../examples/generators/audio_overview.yml"),
  ),
  (
    "slides",
    include_str!("../../examples/generators/slides.yml"),
  ),
  (
    "flashcards",
    include_str!("../../examples/generators/flashcards.yml"),
  ),
  ("quiz", include_str!("../../examples/generators/quiz.yml")),
  (
    "infographic",
    include_str!("../../examples/generators/infographic.yml"),
  ),
  (
    "data_table",
    include_str!("../../examples/generators/data_table.yml"),
  ),
  (
    "video_overview",
    include_str!("../../examples/generators/video_overview.yml"),
  ),
  (
    "illustrated_slides",
    include_str!("../../examples/generators/illustrated_slides.yml"),
  ),
  (
    "tech_slides_video",
    include_str!("../../examples/generators/tech_slides_video.yml"),
  ),
  (
    "debate_podcast_show",
    include_str!("../../examples/generators/debate_podcast_show.yml"),
  ),
];

/// How much notebook text a generator gets, in characters (the
/// templates' own source budget).
const GENERATE_CHUNK_BUDGET: usize = 24_000;

#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
  pub kind: String,
  /// Values for the generator's declared inputs (`focus`, `sections`,
  /// `pdf`, ...). `chunks`, `url` and `text` are set from the notebook.
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
  /// Model for the generator's llm steps; falls back to
  /// `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
}

pub async fn generate(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Json(req): Json<GenerateRequest>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let (_, template) = GENERATORS
    .iter()
    .find(|(kind, _)| *kind == req.kind)
    .ok_or_else(|| {
      ApiError::BadRequest(format!(
        "unknown kind '{}': expected one of {}",
        req.kind,
        GENERATORS
          .iter()
          .map(|(k, _)| *k)
          .collect::<Vec<_>>()
          .join(", ")
      ))
    })?;
  let model = req
    .model
    .clone()
    .or_else(|| {
      std::env::var("YANSHI_GENERATE_MODEL")
        .ok()
        .filter(|m| !m.trim().is_empty())
    })
    .ok_or_else(|| {
      ApiError::BadRequest("set `model`, or YANSHI_GENERATE_MODEL on the gateway".into())
    })?;
  let definition = yanshi_config::executor::parse_workflow_definition(template)
    .map_err(|e| ApiError::Internal(format!("generator template: {e:#}")))?;
  for key in req.params.keys() {
    if matches!(key.as_str(), "chunks" | "url" | "text") {
      return Err(ApiError::BadRequest(format!(
        "'{key}' comes from the notebook and cannot be set"
      )));
    }
    if !definition.inputs.contains_key(key) {
      return Err(ApiError::BadRequest(format!(
        "'{key}' is not a parameter of '{}'",
        req.kind
      )));
    }
  }

  let ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), id)
    .await?;
  if ready_chunks.is_empty() {
    return Err(ApiError::BadRequest(format!(
      "notebook {id} has no ready sources to generate from"
    )));
  }
  let query_hint = req
    .params
    .get("focus")
    .and_then(Value::as_str)
    .or_else(|| req.params.get("topic").and_then(Value::as_str))
    .map(str::trim)
    .filter(|s| !s.is_empty());
  let selected = select_chunks_for_generate(ready_chunks, query_hint, GENERATE_CHUNK_BUDGET).await;
  let chunks: Vec<Value> = selected
    .into_iter()
    .map(|chunk| {
      json!({ "chunk_id": chunk.chunk_id, "source_id": chunk.source_id, "content": chunk.content })
    })
    .collect();
  let mut inputs = req.params;
  inputs.insert("chunks".to_string(), Value::Array(chunks));
  if definition.inputs.contains_key("chat_model") && !inputs.contains_key("chat_model") {
    inputs.insert("chat_model".to_string(), Value::String(model.clone()));
  }
  state
    .gateway_policy
    .check_run_inputs(&definition, &inputs)
    .map_err(ApiError::BadRequest)?;

  let run = crate::runs::admit_and_dispatch(
    &state,
    state.executor.clone(),
    tenant.as_str().to_string(),
    template.to_string(),
    crate::runs::RunOptions {
      inputs,
      model: Some(model),
      notebook: Some(crate::runs::NotebookLink {
        notebook_id: id,
        kind: req.kind.clone(),
      }),
      ..Default::default()
    },
  )
  .await?;
  Ok(Json(
    json!({ "run_id": run.id, "kind": req.kind, "status": run.status }),
  ))
}

/// Select chunks up to `budget` bytes while prioritising BM25 relevance to
/// `query_hint` (`focus` / `topic`) and round-robin coverage across sources,
/// then re-ordering in natural `(source, ordinal)` reading order.
async fn select_chunks_for_generate(
  ready: Vec<SourceChunk>,
  query_hint: Option<&str>,
  budget: usize,
) -> Vec<SourceChunk> {
  if ready.is_empty() || budget == 0 {
    return Vec::new();
  }
  let total_bytes: usize = ready.iter().map(|c| c.content.len()).sum();
  if total_bytes <= budget {
    return ready;
  }

  let mut remaining = budget;
  let mut picked = vec![false; ready.len()];
  let pos_by_id: HashMap<&str, usize> = ready
    .iter()
    .enumerate()
    .map(|(idx, c)| (c.chunk_id.as_str(), idx))
    .collect();

  // 1. Pull BM25 hits matching the caller's focus / topic first so relevant
  //    passages in later sources are never starved by earlier sources.
  if let Some(query) = query_hint.map(str::trim).filter(|q| !q.is_empty()) {
    let backend = Bm25KnowledgeBackend::from_documents(
      ready
        .iter()
        .map(|c| (c.chunk_id.clone(), c.content.clone())),
    );
    if let Ok(hits) = backend.search(query, ready.len()).await {
      for hit in hits {
        if hit.score <= 0.0 {
          continue;
        }
        if let Some(&idx) = pos_by_id.get(hit.id.as_str())
          && !picked[idx]
          && ready[idx].content.len() <= remaining
        {
          picked[idx] = true;
          remaining -= ready[idx].content.len();
        }
      }
    }
  }

  // 2. Fill remaining budget round-robin across sources so multi-document
  //    notebooks stay balanced even when `focus` is omitted or generic.
  let mut source_order: Vec<Uuid> = Vec::new();
  let mut buckets: HashMap<Uuid, std::collections::VecDeque<usize>> = HashMap::new();
  for (idx, chunk) in ready.iter().enumerate() {
    if picked[idx] {
      continue;
    }
    let entry = buckets.entry(chunk.source_id).or_insert_with(|| {
      source_order.push(chunk.source_id);
      std::collections::VecDeque::new()
    });
    entry.push_back(idx);
  }

  loop {
    let mut progressed = false;
    for source_id in &source_order {
      let Some(queue) = buckets.get_mut(source_id) else {
        continue;
      };
      while let Some(idx) = queue.pop_front() {
        let len = ready[idx].content.len();
        if len <= remaining {
          picked[idx] = true;
          remaining -= len;
          progressed = true;
          break;
        }
      }
    }
    if !progressed {
      break;
    }
  }

  let mut selected: Vec<SourceChunk> = ready
    .iter()
    .enumerate()
    .filter(|(idx, _)| picked[*idx])
    .map(|(_, chunk)| chunk.clone())
    .collect();

  // Safety fallback when every single chunk exceeds `budget`.
  if selected.is_empty()
    && let Some(mut first) = ready.into_iter().next()
  {
    let cut = first
      .content
      .char_indices()
      .take_while(|(byte_idx, ch)| byte_idx + ch.len_utf8() <= budget)
      .map(|(byte_idx, ch)| byte_idx + ch.len_utf8())
      .last()
      .unwrap_or(0);
    first.content.truncate(cut);
    if !first.content.is_empty() {
      selected.push(first);
    }
  }

  selected
}

pub async fn list_notebook_artifacts(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let artifacts = state
    .repos
    .notebooks
    .list_artifacts(tenant.as_str(), id)
    .await?;
  Ok(Json(json!({ "notebook_id": id, "artifacts": artifacts })))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn chunks_carry_their_line_range() {
    let text = format!("{}\n\n{}", "a".repeat(800), "b".repeat(800));
    let chunks = chunk(&text).expect("chunks");
    assert!(
      chunks.len() >= 2,
      "two paragraphs over the size limit split"
    );
    assert!(chunks.iter().all(|(content, meta)| {
      !content.is_empty() && meta["end_line"].as_u64() >= meta["start_line"].as_u64()
    }));
    assert!(
      chunks[1].1["start_line"].as_u64() > chunks[0].1["start_line"].as_u64(),
      "later chunks start on later lines"
    );
  }

  #[test]
  fn every_compiled_in_generator_parses() {
    for (kind, template) in GENERATORS {
      let definition = yanshi_config::executor::parse_workflow_definition(template)
        .unwrap_or_else(|e| panic!("{kind}: {e:#}"));
      assert!(
        definition.inputs.contains_key("chunks"),
        "{kind} takes notebook chunks"
      );
      assert!(
        definition.nodes.iter().any(|n| n.id == "deliverable"),
        "{kind}"
      );
    }
  }

  #[tokio::test]
  async fn html_is_reduced_to_its_visible_text() {
    let text = html_text(b"<html><head><script>var x=1;</script></head><body><h1>Tides</h1><p>The moon pulls.</p></body></html>")
      .await
      .expect("text");
    assert!(
      text.contains("Tides") && text.contains("The moon pulls."),
      "{text}"
    );
    assert!(!text.contains("var x"), "{text}");
  }

  /// Build a valid 1-page PDF with an uncompressed Helvetica content stream.
  fn minimal_pdf_bytes(line: &str) -> Vec<u8> {
    let escaped = line
      .replace('\\', "\\\\")
      .replace('(', "\\(")
      .replace(')', "\\)");
    let stream = format!("BT /F1 12 Tf 72 720 Td ({escaped}) Tj ET");
    let obj1 = "1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n";
    let obj2 = "2 0 obj\n<< /Type /Pages /Kids [3 0 R] /Count 1 >>\nendobj\n";
    let obj3 = "3 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>\nendobj\n";
    let obj4 = format!(
      "4 0 obj\n<< /Length {} >>\nstream\n{}\nendstream\nendobj\n",
      stream.len(),
      stream
    );
    let obj5 = "5 0 obj\n<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>\nendobj\n";

    let mut pdf = String::from("%PDF-1.4\n");
    let mut offsets = Vec::with_capacity(5);
    for obj in [obj1, obj2, obj3, obj4.as_str(), obj5] {
      offsets.push(pdf.len());
      pdf.push_str(obj);
    }
    let xref_offset = pdf.len();
    pdf.push_str("xref\n0 6\n0000000000 65535 f \n");
    for off in offsets {
      pdf.push_str(&format!("{off:010} 00000 n \n"));
    }
    pdf.push_str(&format!(
      "trailer\n<< /Size 6 /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n"
    ));
    pdf.into_bytes()
  }

  #[tokio::test]
  async fn pdf_is_extracted_to_searchable_text() {
    let pdf = minimal_pdf_bytes("Spring tides happen when the sun and moon align.");
    let text = pdf_text(&pdf).await.expect("pdf text");
    assert!(
      text.contains("Spring tides happen when the sun and moon align."),
      "extracted PDF text: {text}"
    );
  }

  #[tokio::test]
  async fn recording_is_transcribed_under_mock_asr() {
    let dir = tempfile::tempdir().expect("tempdir");
    let models_path = dir.path().join("models.yml");
    std::fs::write(
      &models_path,
      "models:\n  mock-asr:\n    vendor: mock\n    type: asr\n    model_id: mock-asr\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
    )
    .expect("write models.yml");
    yanshi_llm::Yanshi::init_with_config(models_path.to_str().expect("utf8"))
      .await
      .expect("init");

    let wav = yanshi_llm::providers::mock_media::silent_wav(250);
    let transcript = transcribe_recording(
      wav,
      "standup.wav".to_string(),
      "mock-asr".to_string(),
      Some("en".to_string()),
    )
    .await
    .expect("transcript");
    assert_eq!(transcript, "mock transcript");
  }

  #[tokio::test]
  async fn select_chunks_prioritises_focused_and_multi_source_chunks_under_budget() {
    let notebook_id = Uuid::new_v4();
    let src_a = Uuid::new_v4();
    let src_b = Uuid::new_v4();

    let mut ready = Vec::new();
    for ord in 0..30 {
      ready.push(SourceChunk {
        chunk_id: format!("{src_a}:{ord}"),
        source_id: src_a,
        notebook_id,
        tenant_id: "t1".to_string(),
        ordinal: ord,
        content: format!(
          "Gravitational ocean tides paragraph {ord}. {}",
          "Lunar orbital mechanics and coastal amplitude observations. ".repeat(14)
        ),
        metadata: json!({"start_line": ord * 10 + 1, "end_line": ord * 10 + 8}),
      });
    }
    for ord in 0..3 {
      ready.push(SourceChunk {
        chunk_id: format!("{src_b}:{ord}"),
        source_id: src_b,
        notebook_id,
        tenant_id: "t1".to_string(),
        ordinal: ord,
        content: format!(
          "PostgreSQL advisory lock pool tuning section {ord}: pg_advisory_xact_lock serialises per-tenant run admission cleanly."
        ),
        metadata: json!({"start_line": ord * 5 + 1, "end_line": ord * 5 + 4}),
      });
    }

    let focused = select_chunks_for_generate(
      ready.clone(),
      Some("PostgreSQL advisory lock pool tuning"),
      4_000,
    )
    .await;
    let total_bytes: usize = focused.iter().map(|c| c.content.len()).sum();
    assert!(
      total_bytes <= 4_000,
      "must stay within budget: {total_bytes}"
    );
    assert!(
      focused.iter().filter(|c| c.source_id == src_b).count() == 3,
      "all 3 focused chunks from later source B must be selected instead of starved by source A"
    );

    let balanced = select_chunks_for_generate(ready, None, 4_000).await;
    assert!(
      balanced.iter().any(|c| c.source_id == src_a)
        && balanced.iter().any(|c| c.source_id == src_b),
      "unfocused selection over budget must represent both sources"
    );
  }

  fn build_docx_bytes(document_xml: &str) -> Vec<u8> {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
      let mut zip = zip::ZipWriter::new(&mut cursor);
      let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
      zip.start_file("[Content_Types].xml", opts).expect("start");
      zip
        .write_all(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?><Types/>")
        .expect("write");
      zip.start_file("word/document.xml", opts).expect("start");
      zip.write_all(document_xml.as_bytes()).expect("write");
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  }

  #[test]
  fn docx_extracts_paragraphs_tables_entities_and_blocks_oversized_xml() {
    let xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
      <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
        <w:body>
          <w:p><w:r><w:t>Amphidromic Ocean Dynamics &amp; Resonance</w:t></w:r></w:p>
          <w:p><w:r><w:t>Coriolis deflection rotates tidal waves around amphidromic nodes.</w:t></w:r></w:p>
          <w:tbl>
            <w:tr>
              <w:tc><w:p><w:r><w:t>Basin</w:t></w:r></w:p></w:tc>
              <w:tc><w:p><w:r><w:t>Peak Range</w:t></w:r></w:p></w:tc>
            </w:tr>
            <w:tr>
              <w:tc><w:p><w:r><w:t>Bay of Fundy</w:t></w:r></w:p></w:tc>
              <w:tc><w:p><w:r><w:t>16.3 m</w:t></w:r></w:p></w:tc>
            </w:tr>
          </w:tbl>
        </w:body>
      </w:document>"#;
    let docx = build_docx_bytes(xml);
    let extracted = docx_text(&docx).expect("docx text");
    assert!(
      extracted.contains("Amphidromic Ocean Dynamics & Resonance"),
      "{extracted}"
    );
    assert!(
      extracted.contains("| Bay of Fundy | 16.3 m |"),
      "{extracted}"
    );

    // Oversized deflated XML (> MAX_DOCX_XML_BYTES) is rejected cleanly.
    let huge_xml = format!(
      "<w:document><w:body><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:body></w:document>",
      "A".repeat(MAX_DOCX_XML_BYTES + 1024)
    );
    let bomb = build_docx_bytes(&huge_xml);
    assert!(
      bomb.len() < MAX_SOURCE_BYTES,
      "deflated payload fits upload size"
    );
    let err = docx_text(&bomb).expect_err("must reject oversized uncompressed XML");
    assert!(err.contains("safety limit"), "{err}");
  }

  #[test]
  fn csv_tsv_and_json_sources_preserve_field_labels() {
    let csv = b"station,range_m,regime\nFundy,16.3,Semidiurnal\nSevern,15.0,Macrotidal\n";
    let csv_out = delimited_text(csv, b',').expect("csv");
    assert!(csv_out.contains("Columns: station | range_m | regime"));
    assert!(csv_out.contains("Row 1: station: Fundy | range_m: 16.3 | regime: Semidiurnal"));

    let tsv = b"metric\tvalue\np99_latency_ms\t4.2\n";
    let tsv_out = delimited_text(tsv, b'\t').expect("tsv");
    assert!(tsv_out.contains("Row 1: metric: p99_latency_ms | value: 4.2"));

    let json_bytes = br#"[{"service":"gateway","qps":12500},{"service":"worker","qps":4200}]"#;
    let json_out = json_text(json_bytes).expect("json");
    assert!(
      json_out.contains("Record 1:")
        && json_out.contains("service: gateway")
        && json_out.contains("qps: 12500"),
      "{json_out}"
    );
    assert!(
      json_out.contains("Record 2:")
        && json_out.contains("service: worker")
        && json_out.contains("qps: 4200"),
      "{json_out}"
    );
  }
}
