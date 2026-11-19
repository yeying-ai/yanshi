//! Notebooks and their sources (Y5.2, `docs/RFC_CONTENT_STUDIO.md` §6).
//!
//! ```text
//! POST   /v1/notebooks                              {title}              → notebook
//! GET    /v1/notebooks                                                   → {notebooks}
//! GET    /v1/notebooks/{id}                                              → {notebook, sources}
//! PATCH  /v1/notebooks/{id}                         {title}              → notebook
//! DELETE /v1/notebooks/{id}                                              → 204 No Content
//! POST   /v1/notebooks/{id}/sources                 {kind, ...}          → source (pending; ingests in the background)
//! DELETE /v1/notebooks/{id}/sources/{source_id}                          → 204 No Content
//! GET    /v1/notebooks/{id}/chunks/{chunk_id}                            → chunk
//! GET    /v1/notebooks/{id}/search?q=&top_k=&source_ids=                 → {chunks}
//! POST   /v1/notebooks/{id}/ask                     {question, ...}      → {answer, citations, ...}
//! POST   /v1/notebooks/{id}/generate                {kind, params, model} → {run_id, kind}
//! GET    /v1/notebooks/{id}/artifacts                                    → {artifacts}
//! DELETE /v1/notebooks/{id}/artifacts/{artifact_id}                      → 204 No Content
//! ```
//!
//! Ingestion: fetch (`url`, through the http tool's SSRF filter: the URL
//! is tenant-supplied), or take the pasted `text`, or decode an uploaded
//! `file` (`.txt`, `.md`, `.html`, `.pdf`, `.docx`, `.xlsx`, `.pptx`, `.csv`,
//! `.tsv`, `.json`), or transcribe an uploaded audio `recording` via a
//! configured ASR model; extract text (HTML and PDF via the RAG loaders with
//! `pdftotext` fallback, DOCX/XLSX/PPTX via pure-Rust OOXML extraction,
//! CSV/TSV/JSON with header labels preserved per row); chunk by paragraph;
//! store the chunks in `source_chunks`; mark the source `ready` (or `failed`,
//! with the reason).
//!
//! The index is derived, not stored: a notebook's BM25 index is built
//! from its ready chunks on first search, cached in this process, and
//! dropped whenever one of its sources finishes ingesting. A restart
//! therefore costs one rebuild per notebook, nothing more.

use std::collections::{HashMap, HashSet};
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
/// Uncompressed XML read ceiling when unpacking `.docx` / `.xlsx` / `.pptx` archives (zip-bomb guard).
const MAX_OOXML_XML_BYTES: usize = 16 * 1024 * 1024;
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

#[derive(Debug, Deserialize)]
pub struct UpdateNotebookRequest {
  pub title: String,
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

pub async fn update_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Json(req): Json<UpdateNotebookRequest>,
) -> Result<Json<Notebook>, ApiError> {
  let title = req.title.trim();
  if title.is_empty() {
    return Err(ApiError::BadRequest("title must not be empty".into()));
  }
  let updated = state
    .repos
    .notebooks
    .rename(tenant.as_str(), id, title)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("notebook {id} not found")))?;
  Ok(Json(updated))
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
  /// An uploaded document (`.txt`, `.md`, `.html`, `.pdf`, `.docx`, `.xlsx`,
  /// `.pptx`, `.csv`, `.tsv`, `.json`), base64-encoded.
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
  Xlsx,
  Pptx,
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
        "xlsx" => BytesFormat::Xlsx,
        "pptx" => BytesFormat::Pptx,
        "csv" => BytesFormat::Csv,
        "tsv" => BytesFormat::Tsv,
        "json" => BytesFormat::Json,
        other => {
          return Err(ApiError::BadRequest(format!(
            "unsupported file type '.{other}': upload .txt, .md, .html, .pdf, .docx, .xlsx, .pptx, .csv, .tsv or .json"
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
      if matches!(
        format,
        BytesFormat::Docx | BytesFormat::Xlsx | BytesFormat::Pptx
      ) && !bytes.starts_with(b"PK\x03\x04")
      {
        return Err(ApiError::BadRequest(format!(
          "file is not a valid {} archive (missing PK zip header)",
          extension.to_ascii_uppercase()
        )));
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
    BytesFormat::Xlsx => xlsx_text(&bytes),
    BytesFormat::Pptx => pptx_text(&bytes),
    BytesFormat::Csv => delimited_text(&bytes, b','),
    BytesFormat::Tsv => delimited_text(&bytes, b'\t'),
    BytesFormat::Json => json_text(&bytes),
    BytesFormat::PlainText => {
      String::from_utf8(bytes).map_err(|_| "the source is not UTF-8 text".to_string())
    }
  }
}

fn sniff_ooxml_zip_format(bytes: &[u8], ext: Option<&str>) -> Option<BytesFormat> {
  if !bytes.starts_with(b"PK\x03\x04") {
    return None;
  }
  match ext {
    Some("docx") => return Some(BytesFormat::Docx),
    Some("xlsx") => return Some(BytesFormat::Xlsx),
    Some("pptx") => return Some(BytesFormat::Pptx),
    _ => {}
  }
  let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).ok()?;
  for name in archive.file_names() {
    let Ok(name) = name else {
      continue;
    };
    let name: &str = name.as_ref();
    if name == "word/document.xml" {
      return Some(BytesFormat::Docx);
    }
    if name == "xl/workbook.xml" || name.starts_with("xl/worksheets/sheet") {
      return Some(BytesFormat::Xlsx);
    }
    if name == "ppt/presentation.xml" || name.starts_with("ppt/slides/slide") {
      return Some(BytesFormat::Pptx);
    }
  }
  None
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
  } else if let Some(ooxml) = sniff_ooxml_zip_format(&bytes, ext.as_deref()) {
    ooxml
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

fn read_zip_xml_bounded(
  archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>,
  entry_name: &str,
  budget_remaining: &mut usize,
  label: &str,
) -> Result<Option<String>, String> {
  use std::io::Read as _;
  let mut entry = match archive.by_name(entry_name) {
    Ok(file) => file,
    Err(zip::result::ZipError::FileNotFound) => return Ok(None),
    Err(e) => return Err(format!("reading {entry_name} failed: {e}")),
  };
  let mut xml_bytes = Vec::new();
  entry
    .by_ref()
    .take((*budget_remaining as u64).saturating_add(1))
    .read_to_end(&mut xml_bytes)
    .map_err(|e| format!("decompressing {entry_name} failed: {e}"))?;
  if xml_bytes.len() > *budget_remaining {
    return Err(format!(
      "{label} uncompressed XML exceeds {MAX_OOXML_XML_BYTES} bytes safety limit"
    ));
  }
  *budget_remaining -= xml_bytes.len();
  String::from_utf8(xml_bytes)
    .map(Some)
    .map_err(|_| format!("{entry_name} is not valid UTF-8 XML"))
}

/// Pure-Rust OOXML `.docx` text extraction (`word/document.xml`, plus optional
/// `word/footnotes.xml` and `word/endnotes.xml`), preserving paragraph breaks
/// and table rows (`| cell1 | cell2 |`) while bounding uncompressed XML bytes
/// against zip-bomb payloads.
fn docx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid DOCX archive: {e}"))?;

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let mut sections: Vec<String> = Vec::new();
  for entry_name in [
    "word/document.xml",
    "word/footnotes.xml",
    "word/endnotes.xml",
  ] {
    let Some(xml) = read_zip_xml_bounded(&mut archive, entry_name, &mut budget_remaining, "DOCX")?
    else {
      if entry_name == "word/document.xml" {
        return Err("invalid DOCX archive: missing word/document.xml".to_string());
      }
      continue;
    };
    let extracted = extract_ooxml_rich_text(&xml);
    if !extracted.trim().is_empty() {
      sections.push(extracted);
    }
  }
  Ok(sections.join("\n\n"))
}

/// Pure-Rust OOXML `.pptx` slide & speaker-notes extraction (`ppt/slides/slide*.xml`
/// and optional `ppt/notesSlides/notesSlide*.xml`), ordered numerically by slide
/// index and preserving paragraph breaks and `<a:tbl>` rows.
fn pptx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid PPTX archive: {e}"))?;

  let mut slide_entries: Vec<(usize, String)> = archive
    .file_names()
    .filter_map(|name| {
      let name = name.ok()?;
      let stem = name
        .as_ref()
        .strip_prefix("ppt/slides/slide")?
        .strip_suffix(".xml")?;
      let num = stem.parse::<usize>().ok()?;
      Some((num, name.into_owned()))
    })
    .collect();
  if slide_entries.is_empty() {
    return Err("invalid PPTX archive: missing ppt/slides/slide*.xml".to_string());
  }
  slide_entries.sort_by_key(|(num, _)| *num);

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let mut slides_out: Vec<String> = Vec::new();
  for (slide_num, entry_name) in slide_entries {
    let Some(slide_xml) =
      read_zip_xml_bounded(&mut archive, &entry_name, &mut budget_remaining, "PPTX")?
    else {
      continue;
    };
    let body = extract_ooxml_rich_text(&slide_xml);
    let notes_entry = format!("ppt/notesSlides/notesSlide{slide_num}.xml");
    let notes = read_zip_xml_bounded(&mut archive, &notes_entry, &mut budget_remaining, "PPTX")?
      .map(|xml| extract_ooxml_rich_text(&xml))
      .unwrap_or_default();
    let cleaned_notes: Vec<&str> = notes
      .lines()
      .map(str::trim)
      .filter(|line| !line.is_empty() && line.parse::<usize>().ok() != Some(slide_num))
      .collect();

    if body.trim().is_empty() && cleaned_notes.is_empty() {
      continue;
    }
    let mut section = format!("Slide {slide_num}:");
    if !body.trim().is_empty() {
      section.push('\n');
      section.push_str(body.trim());
    }
    if !cleaned_notes.is_empty() {
      section.push_str("\nSpeaker notes: ");
      section.push_str(&cleaned_notes.join(" "));
    }
    slides_out.push(section);
  }
  Ok(slides_out.join("\n\n"))
}

/// Pure-Rust OOXML `.xlsx` workbook extraction (`xl/sharedStrings.xml`,
/// `xl/workbook.xml`, and `xl/worksheets/sheet*.xml`), formatting rows with
/// sheet and column header labels (`Header: Value | ...`) for BM25 recall.
fn xlsx_text(bytes: &[u8]) -> Result<String, String> {
  let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
    .map_err(|e| format!("invalid XLSX archive: {e}"))?;

  let mut budget_remaining = MAX_OOXML_XML_BYTES;
  let shared_strings = match read_zip_xml_bounded(
    &mut archive,
    "xl/sharedStrings.xml",
    &mut budget_remaining,
    "XLSX",
  )? {
    Some(xml) => parse_xlsx_shared_strings(&xml),
    None => Vec::new(),
  };

  let sheet_titles = match read_zip_xml_bounded(
    &mut archive,
    "xl/workbook.xml",
    &mut budget_remaining,
    "XLSX",
  )? {
    Some(xml) => parse_xlsx_sheet_titles(&xml),
    None => Vec::new(),
  };

  let mut sheet_entries: Vec<(usize, String)> = archive
    .file_names()
    .filter_map(|name| {
      let name = name.ok()?;
      let stem = name
        .as_ref()
        .strip_prefix("xl/worksheets/sheet")?
        .strip_suffix(".xml")?;
      let num = stem.parse::<usize>().ok()?;
      Some((num, name.into_owned()))
    })
    .collect();
  if sheet_entries.is_empty() {
    return Err("invalid XLSX archive: missing xl/worksheets/sheet*.xml".to_string());
  }
  sheet_entries.sort_by_key(|(num, _)| *num);

  let mut sections: Vec<String> = Vec::new();
  for (pos, (sheet_num, entry_name)) in sheet_entries.into_iter().enumerate() {
    let Some(sheet_xml) =
      read_zip_xml_bounded(&mut archive, &entry_name, &mut budget_remaining, "XLSX")?
    else {
      continue;
    };
    let rows = parse_xlsx_sheet_rows(&sheet_xml, &shared_strings);
    if rows.is_empty() {
      continue;
    }
    let sheet_title = sheet_titles
      .get(pos)
      .cloned()
      .unwrap_or_else(|| format!("Sheet{sheet_num}"));
    sections.push(format_xlsx_sheet_rows(&sheet_title, &rows));
  }
  Ok(sections.join("\n\n"))
}

fn xml_attr_value(tag_body: &str, attr_name: &str) -> Option<String> {
  let mut rest = tag_body;
  while let Some(eq_idx) = rest.find('=') {
    let before = rest[..eq_idx].trim_end();
    let key = before
      .rsplit_once(char::is_whitespace)
      .map_or(before, |(_, k)| k);
    let after = rest[eq_idx + 1..].trim_start();
    let Some(quote) = after.chars().next().filter(|&c| c == '"' || c == '\'') else {
      rest = after;
      continue;
    };
    let val_rest = &after[quote.len_utf8()..];
    let Some(end_idx) = val_rest.find(quote) else {
      break;
    };
    if key == attr_name {
      return Some(decode_xml_entities(&val_rest[..end_idx]));
    }
    rest = &val_rest[end_idx + quote.len_utf8()..];
  }
  None
}

fn parse_xlsx_shared_strings(xml: &str) -> Vec<String> {
  let mut items = Vec::new();
  let mut current = String::new();
  let mut in_si = false;
  let mut in_t = false;
  let mut phonetic_depth: usize = 0;

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_si && in_t && phonetic_depth == 0 && lt > 0 {
      current.push_str(&decode_xml_entities(&rest[..lt]));
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
    let local = tag_body
      .split_whitespace()
      .next()
      .unwrap_or_default()
      .rsplit_once(':')
      .map_or_else(
        || tag_body.split_whitespace().next().unwrap_or_default(),
        |(_, l)| l,
      );
    match (local, is_closing, is_self_closing) {
      ("si", false, false) => {
        in_si = true;
        current.clear();
      }
      ("si", false, true) => {
        items.push(String::new());
      }
      ("si", true, _) => {
        if in_si {
          items.push(current.clone());
        }
        in_si = false;
        in_t = false;
        current.clear();
      }
      ("rPh", false, false) => {
        phonetic_depth += 1;
      }
      ("rPh", true, _) => {
        phonetic_depth = phonetic_depth.saturating_sub(1);
      }
      ("t", false, false) if in_si => {
        in_t = true;
      }
      ("t", true, _) => {
        in_t = false;
      }
      _ => {}
    }
  }
  items
}

fn parse_xlsx_sheet_titles(xml: &str) -> Vec<String> {
  let mut titles = Vec::new();
  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    rest = &rest[lt + 1..];
    let Some(gt) = rest.find('>') else {
      break;
    };
    let raw_tag = rest[..gt].trim();
    rest = &rest[gt + 1..];
    if raw_tag.starts_with('/') || raw_tag.starts_with('?') || raw_tag.starts_with('!') {
      continue;
    }
    let tag_body = raw_tag.trim_end_matches('/').trim();
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();
    let local = tag_name.rsplit_once(':').map_or(tag_name, |(_, l)| l);
    if local == "sheet"
      && let Some(name) = xml_attr_value(tag_body, "name")
      && !name.trim().is_empty()
    {
      titles.push(name.trim().to_string());
    }
  }
  titles
}

fn xlsx_col_index_from_ref(cell_ref: &str) -> Option<usize> {
  let letters: String = cell_ref
    .chars()
    .take_while(char::is_ascii_alphabetic)
    .collect();
  if letters.is_empty() {
    return None;
  }
  let mut idx: usize = 0;
  for ch in letters.chars() {
    let digit = (ch.to_ascii_uppercase() as u8).saturating_sub(b'A') as usize + 1;
    idx = idx.saturating_mul(26).saturating_add(digit);
  }
  idx.checked_sub(1)
}

fn xlsx_col_letters(mut col_idx: usize) -> String {
  let mut chars = Vec::new();
  loop {
    chars.push((b'A' + (col_idx % 26) as u8) as char);
    if col_idx < 26 {
      break;
    }
    col_idx = col_idx / 26 - 1;
  }
  chars.into_iter().rev().collect()
}

fn parse_xlsx_sheet_rows(xml: &str, shared_strings: &[String]) -> Vec<Vec<(usize, String)>> {
  let mut rows: Vec<Vec<(usize, String)>> = Vec::new();
  let mut current_row: Vec<(usize, String)> = Vec::new();
  let mut in_row = false;
  let mut in_cell = false;
  let mut next_col_idx: usize = 0;
  let mut cell_col_idx: usize = 0;
  let mut cell_type = String::new();
  let mut in_v = false;
  let mut in_is_t = false;
  let mut cell_v = String::new();
  let mut cell_inline = String::new();

  let mut rest = xml;
  while let Some(lt) = rest.find('<') {
    if in_cell && lt > 0 {
      if in_v {
        cell_v.push_str(&decode_xml_entities(&rest[..lt]));
      } else if in_is_t {
        cell_inline.push_str(&decode_xml_entities(&rest[..lt]));
      }
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
    let local = tag_name.rsplit_once(':').map_or(tag_name, |(_, l)| l);

    match (local, is_closing, is_self_closing) {
      ("row", false, false) => {
        in_row = true;
        current_row.clear();
        next_col_idx = 0;
      }
      ("row", true, _) => {
        if in_row && !current_row.is_empty() {
          rows.push(std::mem::take(&mut current_row));
        }
        in_row = false;
      }
      ("c", false, false) if in_row => {
        in_cell = true;
        cell_col_idx = xml_attr_value(tag_body, "r")
          .and_then(|r| xlsx_col_index_from_ref(&r))
          .unwrap_or(next_col_idx);
        next_col_idx = cell_col_idx + 1;
        cell_type = xml_attr_value(tag_body, "t").unwrap_or_default();
        cell_v.clear();
        cell_inline.clear();
      }
      ("c", false, true) if in_row => {
        let col = xml_attr_value(tag_body, "r")
          .and_then(|r| xlsx_col_index_from_ref(&r))
          .unwrap_or(next_col_idx);
        next_col_idx = col + 1;
      }
      ("c", true, _) if in_cell => {
        let resolved = match cell_type.as_str() {
          "s" => cell_v
            .trim()
            .parse::<usize>()
            .ok()
            .and_then(|idx| shared_strings.get(idx).cloned())
            .unwrap_or_default(),
          "inlineStr" => cell_inline.clone(),
          "b" => match cell_v.trim() {
            "1" => "true".to_string(),
            "0" => "false".to_string(),
            other => other.to_string(),
          },
          _ => cell_v.clone(),
        };
        let trimmed = resolved.trim();
        if !trimmed.is_empty() {
          current_row.push((cell_col_idx, trimmed.to_string()));
        }
        in_cell = false;
        in_v = false;
        in_is_t = false;
      }
      ("v", false, false) if in_cell => {
        in_v = true;
      }
      ("v", true, _) => {
        in_v = false;
      }
      ("t", false, false) if in_cell => {
        in_is_t = true;
      }
      ("t", true, _) => {
        in_is_t = false;
      }
      _ => {}
    }
  }
  rows
}

fn format_xlsx_sheet_rows(sheet_title: &str, rows: &[Vec<(usize, String)>]) -> String {
  let max_col = rows
    .iter()
    .flat_map(|r| r.iter().map(|(col, _)| *col))
    .max()
    .unwrap_or(0);
  if rows.len() == 1 {
    let pairs: Vec<String> = rows[0]
      .iter()
      .map(|(col, val)| format!("{}: {val}", xlsx_col_letters(*col)))
      .collect();
    return format!("Sheet: {sheet_title}\nRow 1: {}", pairs.join(" | "));
  }

  let mut headers = vec![String::new(); max_col + 1];
  for (col, val) in &rows[0] {
    if let Some(slot) = headers.get_mut(*col) {
      *slot = val.clone();
    }
  }
  for (col_idx, slot) in headers.iter_mut().enumerate() {
    if slot.trim().is_empty() {
      *slot = format!("col_{}", xlsx_col_letters(col_idx));
    }
  }

  let mut blocks = vec![format!(
    "Sheet: {sheet_title}\nColumns: {}",
    headers.join(" | ")
  )];
  let mut current_group: Vec<String> = Vec::new();
  for (row_idx, row) in rows[1..].iter().enumerate() {
    let pairs: Vec<String> = row
      .iter()
      .map(|(col, val)| {
        let label = headers
          .get(*col)
          .cloned()
          .unwrap_or_else(|| format!("col_{}", xlsx_col_letters(*col)));
        format!("{label}: {val}")
      })
      .collect();
    if !pairs.is_empty() {
      current_group.push(format!("Row {}: {}", row_idx + 1, pairs.join(" | ")));
      if current_group.len() >= 10 {
        blocks.push(current_group.join("\n"));
        current_group.clear();
      }
    }
  }
  if !current_group.is_empty() {
    blocks.push(current_group.join("\n"));
  }
  blocks.join("\n\n")
}

/// Shared paragraph + table extractor for WordprocessingML (`w:*`, `.docx`) and
/// DrawingML / PresentationML (`a:*`, `.pptx`).
fn extract_ooxml_rich_text(xml: &str) -> String {
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
      ("w:t" | "a:t" | "t", false, false) => {
        in_text_run = true;
      }
      ("w:t" | "a:t" | "t", true, _) => {
        in_text_run = false;
      }
      ("w:tab" | "a:tab", false, _) => {
        current_para.push('\t');
      }
      ("w:br" | "w:cr" | "a:br", false, _) => {
        current_para.push('\n');
      }
      ("w:tbl" | "a:tbl", false, false) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          blocks.push(trimmed.to_string());
        }
        current_para.clear();
        table_depth += 1;
      }
      ("w:tbl" | "a:tbl", true, _) => {
        table_depth = table_depth.saturating_sub(1);
      }
      ("w:tr" | "a:tr", false, false) if table_depth > 0 => {
        current_row.clear();
      }
      ("w:tc" | "a:tc", false, false) if table_depth > 0 => {
        current_cell_paras.clear();
        current_para.clear();
      }
      ("w:p" | "a:p", true, _) => {
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
      ("w:tc" | "a:tc", true, _) if table_depth > 0 => {
        current_row.push(current_cell_paras.join(" "));
        current_cell_paras.clear();
      }
      ("w:tr" | "a:tr", true, _) if table_depth > 0 => {
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
  /// Optional comma-separated source UUIDs scoping search to a subset of the
  /// notebook's sources (`?source_ids=uuid1,uuid2`).
  #[serde(default)]
  pub source_ids: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchHit {
  pub source_id: Uuid,
  pub chunk_id: String,
  pub content: String,
  pub score: f32,
}

fn parse_source_ids_csv(raw: Option<&str>) -> Result<Option<Vec<Uuid>>, ApiError> {
  let Some(raw) = raw else {
    return Ok(None);
  };
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return Err(ApiError::BadRequest(
      "source_ids must select at least one source".into(),
    ));
  }
  let mut ids = Vec::new();
  for token in trimmed.split(',') {
    let token = token.trim();
    if token.is_empty() {
      continue;
    }
    let id = Uuid::parse_str(token)
      .map_err(|_| ApiError::BadRequest(format!("invalid source_id UUID '{token}'")))?;
    if !ids.contains(&id) {
      ids.push(id);
    }
  }
  if ids.is_empty() {
    return Err(ApiError::BadRequest(
      "source_ids must select at least one source".into(),
    ));
  }
  Ok(Some(ids))
}

async fn validate_source_scope(
  state: &AppState,
  tenant: &TenantId,
  notebook_id: Uuid,
  source_ids: Option<&[Uuid]>,
) -> Result<Option<HashSet<Uuid>>, ApiError> {
  let Some(ids) = source_ids else {
    return Ok(None);
  };
  if ids.is_empty() {
    return Err(ApiError::BadRequest(
      "source_ids must select at least one source".into(),
    ));
  }
  let sources = state
    .repos
    .sources
    .list_for_notebook(tenant.as_str(), notebook_id)
    .await?;
  let known: HashSet<Uuid> = sources.into_iter().map(|s| s.id).collect();
  let mut allowed = HashSet::with_capacity(ids.len());
  for &sid in ids {
    if !known.contains(&sid) {
      return Err(ApiError::BadRequest(format!(
        "source {sid} is not in notebook {notebook_id}"
      )));
    }
    allowed.insert(sid);
  }
  Ok(Some(allowed))
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
  let parsed_ids = parse_source_ids_csv(query.source_ids.as_deref())?;
  let scope = validate_source_scope(&state, &tenant, id, parsed_ids.as_deref()).await?;

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
  let raw_hits = match &scope {
    None => index
      .backend
      .search(&query.q, top_k)
      .await
      .map_err(|e| ApiError::Internal(format!("search failed: {e}")))?,
    Some(allowed) => {
      let scoped_docs: Vec<(String, String)> = index
        .chunks
        .values()
        .filter(|c| allowed.contains(&c.source_id))
        .map(|c| (c.chunk_id.clone(), c.content.clone()))
        .collect();
      if scoped_docs.is_empty() {
        return Ok(Json(json!({ "chunks": [] })));
      }
      Bm25KnowledgeBackend::from_documents(scoped_docs)
        .search(&query.q, top_k)
        .await
        .map_err(|e| ApiError::Internal(format!("search failed: {e}")))?
    }
  };
  let hits: Vec<SearchHit> = raw_hits
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

// ── grounded Q&A (`/ask`) ───────────────────────────────────────────────

/// Maximum prior Q&A turns retained in multi-turn grounded `/ask`.
pub const MAX_ASK_HISTORY_TURNS: usize = 8;
/// Per-field character cap when normalizing caller-supplied conversation turns.
const MAX_ASK_TURN_CHARS: usize = 2_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookConversationTurn {
  #[serde(default)]
  pub question: String,
  #[serde(default)]
  pub answer: String,
}

#[derive(Debug, Deserialize)]
pub struct AskNotebookRequest {
  pub question: String,
  /// Optional prior Q&A turns for multi-turn follow-up grounding.
  #[serde(default)]
  pub history: Vec<NotebookConversationTurn>,
  /// Optional subset of notebook source IDs to ground the answer against.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  /// Chat model for grounded synthesis; falls back to `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional cap on how many relevant passages are passed to the model.
  #[serde(default)]
  pub top_k: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedAnswerCitation {
  pub source_id: Uuid,
  pub chunk_id: String,
  pub quote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskNotebookResponse {
  pub notebook_id: Uuid,
  pub question: String,
  pub answer: String,
  pub model: String,
  pub citations: Vec<GroundedAnswerCitation>,
  pub used_chunk_count: usize,
}

fn grounded_answer_schema() -> Value {
  json!({
    "type": "object",
    "additionalProperties": false,
    "required": ["answer", "citations"],
    "properties": {
      "answer": { "type": "string" },
      "citations": {
        "type": "array",
        "items": {
          "type": "object",
          "additionalProperties": false,
          "required": ["chunk_id", "quote"],
          "properties": {
            "chunk_id": { "type": "string" },
            "quote": { "type": "string" }
          }
        }
      }
    }
  })
}

fn resolve_notebook_model(explicit: Option<&str>) -> Result<String, ApiError> {
  explicit
    .map(str::trim)
    .filter(|m| !m.is_empty())
    .map(str::to_string)
    .or_else(|| {
      std::env::var("YANSHI_GENERATE_MODEL")
        .ok()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
    })
    .ok_or_else(|| {
      ApiError::BadRequest("set `model`, or YANSHI_GENERATE_MODEL on the gateway".into())
    })
}

fn parse_grounded_answer_reply(
  raw_reply: &str,
  chunk_sources: &HashMap<String, Uuid>,
) -> Result<(String, Vec<GroundedAnswerCitation>), String> {
  #[derive(Deserialize)]
  struct RawCitation {
    #[serde(default)]
    chunk_id: String,
    #[serde(default)]
    quote: String,
  }
  #[derive(Deserialize)]
  struct RawAnswer {
    answer: String,
    #[serde(default)]
    citations: Vec<RawCitation>,
  }

  let value = extract_json_object(raw_reply)?;
  let parsed: RawAnswer =
    serde_json::from_value(value).map_err(|e| format!("invalid grounded answer JSON: {e}"))?;
  let answer = parsed.answer.trim().to_string();
  if answer.is_empty() {
    return Err("grounded answer was empty".to_string());
  }

  let mut seen: HashSet<(String, String)> = HashSet::new();
  let mut citations = Vec::new();
  for raw in parsed.citations {
    let cid = raw.chunk_id.trim();
    let quote = raw.quote.trim();
    if cid.is_empty() || quote.is_empty() {
      continue;
    }
    let Some(&source_id) = chunk_sources.get(cid) else {
      continue;
    };
    let key = (cid.to_string(), quote.to_string());
    if seen.insert(key) {
      citations.push(GroundedAnswerCitation {
        source_id,
        chunk_id: cid.to_string(),
        quote: quote.to_string(),
      });
    }
  }
  Ok((answer, citations))
}

fn extract_json_object(reply: &str) -> Result<Value, String> {
  let trimmed = reply.trim();
  if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
    return Ok(v);
  }
  let without_think = if let Some(start) = trimmed.find("<think>")
    && let Some(end) = trimmed[start..].find("</think>")
  {
    format!(
      "{}{}",
      &trimmed[..start],
      &trimmed[start + end + "</think>".len()..]
    )
  } else {
    trimmed.to_string()
  };
  let s = without_think.trim();
  if let Ok(v) = serde_json::from_str::<Value>(s) {
    return Ok(v);
  }
  if let Some(fence_start) = s.find("```") {
    let after_ticks = &s[fence_start + 3..];
    let body_start = after_ticks.find('\n').map_or(0, |nl| nl + 1);
    let body = &after_ticks[body_start..];
    if let Some(fence_end) = body.find("```")
      && let Ok(v) = serde_json::from_str::<Value>(body[..fence_end].trim())
    {
      return Ok(v);
    }
  }
  if let Some(open) = s.find('{')
    && let Some(close) = s.rfind('}')
    && close > open
    && let Ok(v) = serde_json::from_str::<Value>(&s[open..=close])
  {
    return Ok(v);
  }
  Err("model reply did not contain valid JSON".to_string())
}

fn truncate_chars(s: &str, max_chars: usize) -> String {
  s.trim().chars().take(max_chars).collect()
}

fn normalize_ask_history(raw: &[NotebookConversationTurn]) -> Vec<NotebookConversationTurn> {
  let cleaned: Vec<NotebookConversationTurn> = raw
    .iter()
    .filter_map(|turn| {
      let question = truncate_chars(&turn.question, MAX_ASK_TURN_CHARS);
      let answer = truncate_chars(&turn.answer, MAX_ASK_TURN_CHARS);
      if question.is_empty() && answer.is_empty() {
        None
      } else {
        Some(NotebookConversationTurn { question, answer })
      }
    })
    .collect();
  let skip = cleaned.len().saturating_sub(MAX_ASK_HISTORY_TURNS);
  cleaned.into_iter().skip(skip).collect()
}

/// Build a BM25 retrieval hint combining recent conversation turns with the
/// active follow-up question so pronoun/coreference follow-ups ("What is its
/// peak range?") still match the subject introduced in prior turns.
fn build_ask_retrieval_hint(question: &str, history: &[NotebookConversationTurn]) -> String {
  let recent_skip = history.len().saturating_sub(2);
  let mut parts: Vec<String> = Vec::with_capacity(5);
  for turn in history.iter().skip(recent_skip) {
    if !turn.question.is_empty() {
      parts.push(truncate_chars(&turn.question, 240));
    }
    if !turn.answer.is_empty() {
      parts.push(truncate_chars(&turn.answer, 240));
    }
  }
  parts.push(question.trim().to_string());
  parts.join(" ")
}

fn format_ask_history_block(history: &[NotebookConversationTurn]) -> String {
  if history.is_empty() {
    return String::new();
  }
  let turns = history
    .iter()
    .enumerate()
    .map(|(idx, turn)| {
      format!(
        "Turn {}:\nUser: {}\nAssistant: {}",
        idx + 1,
        turn.question,
        turn.answer
      )
    })
    .collect::<Vec<_>>()
    .join("\n\n");
  format!(
    "Prior conversation context (use ONLY to resolve follow-up references; every factual claim MUST still be grounded in Source passages below):\n{turns}\n\n"
  )
}

pub async fn ask_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Json(req): Json<AskNotebookRequest>,
) -> Result<Json<AskNotebookResponse>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let question = req.question.trim().to_string();
  if question.is_empty() {
    return Err(ApiError::BadRequest("question must not be empty".into()));
  }
  let model = resolve_notebook_model(req.model.as_deref())?;
  let scope = validate_source_scope(&state, &tenant, id, req.source_ids.as_deref()).await?;
  let history = normalize_ask_history(&req.history);
  let retrieval_hint = build_ask_retrieval_hint(&question, &history);

  let mut ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), id)
    .await?;
  if let Some(allowed) = &scope {
    ready_chunks.retain(|c| allowed.contains(&c.source_id));
  }
  if ready_chunks.is_empty() {
    let msg = if scope.is_some() {
      "selected sources have no ready chunks to answer from".to_string()
    } else {
      format!("notebook {id} has no ready sources to answer from")
    };
    return Err(ApiError::BadRequest(msg));
  }

  let mut selected =
    select_chunks_for_generate(ready_chunks, Some(&retrieval_hint), GENERATE_CHUNK_BUDGET).await;
  if let Some(limit) = req.top_k.map(|k| k.clamp(1, 50))
    && selected.len() > limit
  {
    selected.truncate(limit);
  }
  let used_chunk_count = selected.len();
  let chunk_sources: HashMap<String, Uuid> = selected
    .iter()
    .map(|c| (c.chunk_id.clone(), c.source_id))
    .collect();
  let formatted_passages = selected
    .iter()
    .map(|c| format!("[chunk_id: {}]\n{}", c.chunk_id, c.content))
    .collect::<Vec<_>>()
    .join("\n\n---\n\n");

  let schema = grounded_answer_schema();
  let schema_str = serde_json::to_string(&schema).unwrap_or_default();
  let history_block = format_ask_history_block(&history);
  let prompt = format!(
    "Answer the user's question strictly using ONLY the notebook source passages below. \
     If the passages do not contain enough evidence, state clearly what is missing rather than guessing. \
     Cite every supporting passage by its exact `chunk_id` and a verbatim `quote` copied from that passage.\n\n\
     {history_block}\
     Question: {question}\n\n\
     Source passages:\n{formatted_passages}\n\n\
     Output ONLY valid JSON strictly conforming to this JSON Schema (no markdown fences, no extra prose):\n{schema_str}"
  );

  yanshi_llm::Yanshi::ensure_init()
    .await
    .map_err(|e| ApiError::Internal(format!("initializing model registry: {e}")))?;
  let response = yanshi_llm::Yanshi::model(&model)
    .system(
      "You are a source-grounded research assistant. Answer concisely in the language of the user's question and cite exact source chunk_ids.",
    )
    .prompt(&prompt)
    .json_schema("grounded_answer", schema)
    .execute_full()
    .await
    .map_err(|e| ApiError::Internal(format!("LLM execution failed: {e}")))?;

  let (answer, citations) = parse_grounded_answer_reply(&response.content, &chunk_sources)
    .map_err(|e| ApiError::Internal(format!("parsing grounded answer: {e}")))?;

  Ok(Json(AskNotebookResponse {
    notebook_id: id,
    question,
    answer,
    model,
    citations,
    used_chunk_count,
  }))
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
  /// Optional subset of notebook source IDs to include in this generation run.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
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
  let model = resolve_notebook_model(req.model.as_deref())?;
  let scope = validate_source_scope(&state, &tenant, id, req.source_ids.as_deref()).await?;
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

  let mut ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), id)
    .await?;
  if let Some(allowed) = &scope {
    ready_chunks.retain(|c| allowed.contains(&c.source_id));
  }
  if ready_chunks.is_empty() {
    let msg = if scope.is_some() {
      "selected sources have no ready chunks to generate from".to_string()
    } else {
      format!("notebook {id} has no ready sources to generate from")
    };
    return Err(ApiError::BadRequest(msg));
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

pub async fn delete_notebook_artifact(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, artifact_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, ApiError> {
  let deleted = state
    .repos
    .notebooks
    .detach_artifact(tenant.as_str(), notebook_id, artifact_id)
    .await?;
  if !deleted {
    return Err(ApiError::NotFound(format!(
      "artifact {artifact_id} in notebook {notebook_id} not found"
    )));
  }
  Ok(StatusCode::NO_CONTENT)
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

    // Oversized deflated XML (> MAX_OOXML_XML_BYTES) is rejected cleanly.
    let huge_xml = format!(
      "<w:document><w:body><w:p><w:r><w:t>{}</w:t></w:r></w:p></w:body></w:document>",
      "A".repeat(MAX_OOXML_XML_BYTES + 1024)
    );
    let bomb = build_docx_bytes(&huge_xml);
    assert!(
      bomb.len() < MAX_SOURCE_BYTES,
      "deflated payload fits upload size"
    );
    let err = docx_text(&bomb).expect_err("must reject oversized uncompressed XML");
    assert!(err.contains("safety limit"), "{err}");
  }

  fn build_zip_archive(entries: &[(&str, &str)]) -> Vec<u8> {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
      let mut zip = zip::ZipWriter::new(&mut cursor);
      let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
      for (name, body) in entries {
        zip.start_file(*name, opts).expect("start");
        zip.write_all(body.as_bytes()).expect("write");
      }
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  }

  #[test]
  fn xlsx_extracts_sheets_shared_strings_inline_cells_and_sniffs_zip() {
    let xlsx = build_zip_archive(&[
      (
        "xl/workbook.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <workbook><sheets><sheet name="TideBench" sheetId="1" r:id="rId1"/></sheets></workbook>"#,
      ),
      (
        "xl/sharedStrings.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <sst count="3" uniqueCount="3">
          <si><t>Estuary</t></si>
          <si><t>PeakRangeM</t></si>
          <si><r><t>Ungava</t></r><r><t> Bay</t></r></si>
        </sst>"#,
      ),
      (
        "xl/worksheets/sheet1.xml",
        r#"<?xml version="1.0" encoding="UTF-8"?>
        <worksheet><sheetData>
          <row r="1">
            <c r="A1" t="s"><v>0</v></c>
            <c r="B1" t="s"><v>1</v></c>
            <c r="C1" t="inlineStr"><is><t>Macrotidal</t></is></c>
          </row>
          <row r="2">
            <c r="A2" t="s"><v>2</v></c>
            <c r="B2"><v>16.8</v></c>
            <c r="C2" t="b"><v>1</v></c>
          </row>
        </sheetData></worksheet>"#,
      ),
    ]);
    assert_eq!(sniff_ooxml_zip_format(&xlsx, None), Some(BytesFormat::Xlsx));
    let out = xlsx_text(&xlsx).expect("xlsx text");
    assert!(out.contains("Sheet: TideBench"), "{out}");
    assert!(
      out.contains("Columns: Estuary | PeakRangeM | Macrotidal"),
      "{out}"
    );
    assert!(
      out.contains("Row 1: Estuary: Ungava Bay | PeakRangeM: 16.8 | Macrotidal: true"),
      "{out}"
    );
  }

  #[test]
  fn pptx_orders_slides_numerically_and_extracts_tables_and_speaker_notes() {
    let pptx = build_zip_archive(&[
      ("ppt/presentation.xml", "<p:presentation/>"),
      (
        "ppt/slides/slide10.xml",
        r#"<p:sld><p:cSld><p:spTree>
          <p:sp><p:txBody><a:p><a:r><a:t>Tenth Slide Closing Takeaway</a:t></a:r></a:p></p:txBody></p:sp>
        </p:spTree></p:cSld></p:sld>"#,
      ),
      (
        "ppt/slides/slide2.xml",
        r#"<p:sld><p:cSld><p:spTree>
          <p:sp><p:txBody><a:p><a:r><a:t>Second Slide Harmonic Constituents</a:t></a:r></a:p></p:txBody></p:sp>
          <p:graphicFrame><a:graphic><a:graphicData><a:tbl>
            <a:tr>
              <a:tc><a:txBody><a:p><a:r><a:t>Constituent</a:t></a:r></a:p></a:txBody></a:tc>
              <a:tc><a:txBody><a:p><a:r><a:t>PeriodHours</a:t></a:r></a:p></a:txBody></a:tc>
            </a:tr>
            <a:tr>
              <a:tc><a:txBody><a:p><a:r><a:t>M2</a:t></a:r></a:p></a:txBody></a:tc>
              <a:tc><a:txBody><a:p><a:r><a:t>12.42</a:t></a:r></a:p></a:txBody></a:tc>
            </a:tr>
          </a:tbl></a:graphicData></a:graphic></p:graphicFrame>
        </p:spTree></p:cSld></p:sld>"#,
      ),
      (
        "ppt/notesSlides/notesSlide2.xml",
        r#"<p:notes><p:cSld><p:spTree>
          <p:sp><p:txBody><a:p><a:r><a:t>Emphasize principal lunar semidiurnal M2 dominance.</a:t></a:r></a:p></p:txBody></p:sp>
          <p:sp><p:txBody><a:p><a:r><a:t>2</a:t></a:r></a:p></p:txBody></p:sp>
        </p:spTree></p:cSld></p:notes>"#,
      ),
    ]);
    assert_eq!(sniff_ooxml_zip_format(&pptx, None), Some(BytesFormat::Pptx));
    let out = pptx_text(&pptx).expect("pptx text");
    let pos2 = out.find("Slide 2:").expect("slide 2");
    let pos10 = out.find("Slide 10:").expect("slide 10");
    assert!(
      pos2 < pos10,
      "Slide 2 must appear before Slide 10 (numeric ordering): {out}"
    );
    assert!(out.contains("| M2 | 12.42 |"), "{out}");
    assert!(
      out.contains("Speaker notes: Emphasize principal lunar semidiurnal M2 dominance."),
      "{out}"
    );
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

  #[test]
  fn source_ids_csv_parses_dedupes_and_rejects_empty_or_invalid() {
    assert!(parse_source_ids_csv(None).expect("none").is_none());
    assert!(parse_source_ids_csv(Some("  ")).is_err());
    assert!(parse_source_ids_csv(Some(", ,")).is_err());
    assert!(parse_source_ids_csv(Some("not-a-uuid")).is_err());

    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let parsed = parse_source_ids_csv(Some(&format!(" {a}, {b} , {a} ")))
      .expect("valid")
      .expect("some");
    assert_eq!(parsed, vec![a, b]);
  }

  #[test]
  fn grounded_answer_reply_verifies_real_chunks_and_strips_hallucinated_ids() {
    let src_a = Uuid::new_v4();
    let src_b = Uuid::new_v4();
    let chunk_a = format!("{src_a}:0");
    let chunk_b = format!("{src_b}:1");
    let lookup = HashMap::from([(chunk_a.clone(), src_a), (chunk_b.clone(), src_b)]);

    let fenced_reply = format!(
      "<think>checking citations</think>\n```json\n{}\n```",
      json!({
        "answer": "Severn Bore reaches 15.0 m while amphidromic nodes have zero amplitude.",
        "citations": [
          { "chunk_id": chunk_a, "quote": "SevernBore, 15.0" },
          { "chunk_id": chunk_a, "quote": "SevernBore, 15.0" },
          { "chunk_id": "hallucinated-chunk:99", "quote": "Made up quote" },
          { "chunk_id": chunk_b, "quote": "   " },
          { "chunk_id": chunk_b, "quote": "zero-amplitude rotary tidal systems" }
        ]
      })
    );

    let (answer, citations) =
      parse_grounded_answer_reply(&fenced_reply, &lookup).expect("parsed grounded answer");
    assert!(answer.contains("15.0 m"));
    assert_eq!(
      citations,
      vec![
        GroundedAnswerCitation {
          source_id: src_a,
          chunk_id: chunk_a,
          quote: "SevernBore, 15.0".to_string(),
        },
        GroundedAnswerCitation {
          source_id: src_b,
          chunk_id: chunk_b,
          quote: "zero-amplitude rotary tidal systems".to_string(),
        },
      ]
    );
  }

  #[test]
  fn normalize_ask_history_trims_drops_blank_and_caps_recent_turns() {
    let mut raw = vec![NotebookConversationTurn {
      question: "   ".to_string(),
      answer: "\n\t".to_string(),
    }];
    for idx in 1..=10 {
      raw.push(NotebookConversationTurn {
        question: format!("  Question {idx}  "),
        answer: format!("  Answer {idx}  "),
      });
    }
    let normalized = normalize_ask_history(&raw);
    assert_eq!(normalized.len(), MAX_ASK_HISTORY_TURNS);
    assert_eq!(normalized[0].question, "Question 3");
    assert_eq!(
      normalized[MAX_ASK_HISTORY_TURNS - 1].question,
      "Question 10"
    );
    let block = format_ask_history_block(&normalized);
    assert!(block.contains("Turn 1:\nUser: Question 3\nAssistant: Answer 3"));
    assert!(block.contains("Turn 8:\nUser: Question 10\nAssistant: Answer 10"));
  }

  #[tokio::test]
  async fn multi_turn_retrieval_hint_recovers_coreference_follow_up_chunks() {
    let notebook_id = Uuid::new_v4();
    let src_doc = Uuid::new_v4();

    let mut ready = Vec::new();
    for ord in 0..20 {
      ready.push(SourceChunk {
        chunk_id: format!("{src_doc}:{ord}"),
        source_id: src_doc,
        notebook_id,
        tenant_id: "t1".to_string(),
        ordinal: ord,
        content: format!(
          "Chapter {ord} oceanic tidal dynamics: lunar gravitational pull governs semidiurnal amplitude. {}",
          "Coastal tide gauge measurements and spring-neap cycles. ".repeat(4)
        ),
        metadata: json!({"start_line": 1, "end_line": 8}),
      });
    }
    for ord in 20..23 {
      ready.push(SourceChunk {
        chunk_id: format!("{src_doc}:{ord}"),
        source_id: src_doc,
        notebook_id,
        tenant_id: "t1".to_string(),
        ordinal: ord,
        content: format!(
          "Appendix {ord} PostgreSQL concurrency: pg_advisory_xact_lock serialises tenant run admission. {}",
          "Transaction-level advisory lock semantics under multi-tenant connection pooling. ".repeat(3)
        ),
        metadata: json!({"start_line": 1, "end_line": 4}),
      });
    }

    // Reverse verification: a pronoun follow-up ("What exact primitive does it invoke?")
    // WITHOUT history has zero lexical overlap with the late Appendix chunks (`ord 20..23`),
    // so sequential budget fill starves all 3 PostgreSQL chunks (`0` recalled).
    let follow_up = "What exact primitive does it invoke?";
    let without_history = select_chunks_for_generate(ready.clone(), Some(follow_up), 2_000).await;
    let pg_without_history = without_history
      .iter()
      .filter(|c| c.content.contains("pg_advisory_xact_lock"))
      .count();
    assert_eq!(
      pg_without_history, 0,
      "reverse check: pronoun-only query without history must miss late appendix chunks"
    );

    // WITH prior conversation turn naming PostgreSQL concurrency, BM25 recalls all 3 appendix chunks.
    let history = vec![NotebookConversationTurn {
      question: "How does PostgreSQL concurrency serialise tenant admission?".to_string(),
      answer: "It relies on a transaction-scoped lock in PostgreSQL.".to_string(),
    }];
    let hint = build_ask_retrieval_hint(follow_up, &history);
    let with_history = select_chunks_for_generate(ready, Some(&hint), 2_000).await;
    let pg_with_history = with_history
      .iter()
      .filter(|c| c.content.contains("pg_advisory_xact_lock"))
      .count();
    assert_eq!(
      pg_with_history, 3,
      "multi-turn retrieval hint must recall all 3 PostgreSQL appendix chunks on pronoun follow-up"
    );
  }
}
