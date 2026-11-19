//! Notebooks and their sources (Y5.2, `docs/RFC_CONTENT_STUDIO.md` §6).
//!
//! ```text
//! POST   /v1/notebooks                              {title}              → notebook
//! POST   /v1/notebooks/import                       bundle | {bundle}    → {notebook, imported_sources, imported_chunks}
//! GET    /v1/notebooks                                                   → {notebooks}
//! GET    /v1/notebooks/{id}                                              → {notebook, sources}
//! GET    /v1/notebooks/{id}/export                                       → NotebookExportBundle
//! PATCH  /v1/notebooks/{id}                         {title}              → notebook
//! DELETE /v1/notebooks/{id}                                              → 204 No Content
//! POST   /v1/notebooks/{id}/sources                 {kind, ...}          → source (pending; ingests in the background)
//! PATCH  /v1/notebooks/{id}/sources/{source_id}     {title?, text?}      → source (renames or re-chunks text note)
//! POST   /v1/notebooks/{id}/sources/{source_id}:retry                    → source (re-ingests failed URL source)
//! GET    /v1/notebooks/{id}/sources/{source_id}/chunks                   → {notebook_id, source, chunks}
//! DELETE /v1/notebooks/{id}/sources/{source_id}                          → 204 No Content
//! GET    /v1/notebooks/{id}/chunks/{chunk_id}                            → chunk
//! GET    /v1/notebooks/{id}/search?q=&top_k=&source_ids=                 → {chunks}
//! POST   /v1/notebooks/{id}/ask                     {question, ...}      → {answer, citations, ...}
//! GET    /v1/notebooks/{id}/messages                                     → {notebook_id, messages}
//! DELETE /v1/notebooks/{id}/messages                                     → {notebook_id, deleted}
//! POST   /v1/notebooks/{id}/plan                    {goal, ...}          → {rationale, items, ...}
//! POST   /v1/notebooks/{id}/generate                {kind, params, ...}  → {run_id, kind}
//! POST   /v1/notebooks/{id}/generate-batch          {items, ...}         → {notebook_id, runs, ...}
//! GET    /v1/notebooks/{id}/artifacts                                    → {artifacts}
//! GET    /v1/notebooks/{id}/artifacts.zip                                → zip bundle (+ BRIEFING.md)
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
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};

use axum::{
  Extension, Json,
  body::Body,
  extract::{Path, Query, State},
  http::{HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use yanshi_db::{
  ArtifactRepo, EventRepo, NewNotebookMessage, NewSource, Notebook, NotebookArtifact,
  NotebookMessage, NotebookMessageRepo, NotebookRepo, RunRepo, Source, SourceChunk,
  SourceChunkRepo, SourceKind, SourceRepo, SourceStatus,
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
  crate::error::JsonReq(req): crate::error::JsonReq<CreateNotebookRequest>,
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
  crate::error::JsonReq(req): crate::error::JsonReq<UpdateNotebookRequest>,
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

#[derive(Debug, Deserialize)]
pub struct UpdateSourceRequest {
  #[serde(default)]
  pub title: Option<String>,
  #[serde(default)]
  pub text: Option<String>,
}

pub async fn update_source(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, source_id)): Path<(Uuid, Uuid)>,
  crate::error::JsonReq(req): crate::error::JsonReq<UpdateSourceRequest>,
) -> Result<Json<Source>, ApiError> {
  own_notebook(&state, &tenant, notebook_id).await?;
  if req.title.is_none() && req.text.is_none() {
    return Err(ApiError::BadRequest(
      "update requires at least one of `title` or `text`".into(),
    ));
  }
  let mut source = state
    .repos
    .sources
    .get(tenant.as_str(), source_id)
    .await?
    .filter(|s| s.notebook_id == notebook_id)
    .ok_or_else(|| {
      ApiError::NotFound(format!(
        "source {source_id} in notebook {notebook_id} not found"
      ))
    })?;

  if source.status == "pending" || source.status == "ingesting" {
    return Err(ApiError::BadRequest(format!(
      "source {source_id} is currently '{}'; wait for ingestion to finish before editing",
      source.status
    )));
  }

  if let Some(raw_text) = req.text {
    if source.kind != "text" {
      return Err(ApiError::BadRequest(format!(
        "only `text` note sources support inline `text` updates (source is `{}`)",
        source.kind
      )));
    }
    if raw_text.trim().is_empty() {
      return Err(ApiError::BadRequest("text must not be empty".into()));
    }
    if raw_text.len() > MAX_SOURCE_BYTES {
      return Err(ApiError::PayloadTooLarge(format!(
        "text exceeds {MAX_SOURCE_BYTES} bytes"
      )));
    }
    let chunks = chunk(&raw_text).map_err(ApiError::BadRequest)?;
    if chunks.is_empty() {
      return Err(ApiError::BadRequest("text produced no chunks".into()));
    }
    let digest = format!("{:x}", Sha256::digest(raw_text.as_bytes()));
    state
      .repos
      .chunks
      .replace_for_source(&source, chunks)
      .await?;
    state
      .repos
      .sources
      .update_status(source.id, SourceStatus::Ready, None, Some(&digest))
      .await?;
    state
      .repos
      .sources
      .set_raw_content(tenant.as_str(), notebook_id, source_id, &raw_text)
      .await?;
    state.notebook_indexes.invalidate(notebook_id);
    if let Some(refreshed) = state.repos.sources.get(tenant.as_str(), source_id).await? {
      source = refreshed;
    }
  }

  if let Some(raw_title) = req.title {
    let trimmed = raw_title.trim();
    if trimmed.is_empty() {
      return Err(ApiError::BadRequest("title must not be empty".into()));
    }
    source = state
      .repos
      .sources
      .rename(tenant.as_str(), notebook_id, source_id, trimmed)
      .await?
      .ok_or_else(|| {
        ApiError::NotFound(format!(
          "source {source_id} in notebook {notebook_id} not found"
        ))
      })?;
  }

  Ok(Json(source))
}

pub async fn post_source_action(
  state: State<AppState>,
  tenant: Extension<TenantId>,
  Path((notebook_id, raw_segment)): Path<(Uuid, String)>,
) -> Result<Json<Source>, ApiError> {
  let Some(raw_id) = raw_segment.strip_suffix(":retry") else {
    return Err(ApiError::BadRequest(
      "source action route must end with :retry".into(),
    ));
  };
  let source_id = Uuid::parse_str(raw_id)
    .map_err(|_| ApiError::BadRequest(format!("invalid source id '{raw_id}'")))?;
  retry_source(state, tenant, Path((notebook_id, source_id))).await
}

pub async fn retry_source(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, source_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Source>, ApiError> {
  own_notebook(&state, &tenant, notebook_id).await?;
  let existing = state
    .repos
    .sources
    .get(tenant.as_str(), source_id)
    .await?
    .filter(|s| s.notebook_id == notebook_id)
    .ok_or_else(|| {
      ApiError::NotFound(format!(
        "source {source_id} in notebook {notebook_id} not found"
      ))
    })?;
  if existing.status != "failed" {
    return Err(ApiError::BadRequest(format!(
      "source {source_id} is '{}'; only failed sources can be retried",
      existing.status
    )));
  }
  let Some(uri) = existing
    .uri
    .as_deref()
    .map(str::trim)
    .filter(|u| !u.is_empty())
    .map(str::to_string)
  else {
    return Err(ApiError::BadRequest(
      "only failed URL sources with a stored URI can be retried; re-upload file or recording sources"
        .into(),
    ));
  };
  if existing.kind != "url" {
    return Err(ApiError::BadRequest(format!(
      "only `url` sources can be retried without re-uploading bytes (source is `{}`)",
      existing.kind
    )));
  }

  let reopened = state
    .repos
    .sources
    .reset_for_retry(tenant.as_str(), notebook_id, source_id)
    .await?
    .ok_or_else(|| {
      ApiError::BadRequest(format!("source {source_id} is no longer in failed status"))
    })?;
  tokio::spawn(ingest(state.clone(), reopened.clone(), Payload::Url(uri)));
  Ok(Json(reopened))
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

pub async fn list_source_chunks(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((notebook_id, source_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, notebook_id).await?;
  let source = state
    .repos
    .sources
    .get(tenant.as_str(), source_id)
    .await?
    .filter(|s| s.notebook_id == notebook_id)
    .ok_or_else(|| {
      ApiError::NotFound(format!(
        "source {source_id} in notebook {notebook_id} not found"
      ))
    })?;
  let chunks = state
    .repos
    .chunks
    .list_for_source(tenant.as_str(), notebook_id, source_id)
    .await?;
  let raw_text = if source.kind == "text" {
    Some(
      source
        .raw_content
        .clone()
        .unwrap_or_else(|| reconstruct_non_overlapping_chunk_text(&chunks)),
    )
  } else {
    None
  };
  Ok(Json(json!({
    "notebook_id": notebook_id,
    "source": source,
    "chunks": chunks,
    "raw_text": raw_text,
  })))
}

/// Reconstruct original note text without duplicating `ParagraphChunker` overlap
/// lines when `sources.raw_content` is absent (pre-`0017` rows).
pub(crate) fn reconstruct_non_overlapping_chunk_text(chunks: &[SourceChunk]) -> String {
  if chunks.is_empty() {
    return String::new();
  }
  if chunks.len() == 1 {
    return chunks[0].content.clone();
  }
  let mut out_lines: Vec<&str> = Vec::new();
  let mut max_end_line: Option<usize> = None;
  for chunk in chunks {
    let start_line = chunk
      .metadata
      .get("start_line")
      .and_then(Value::as_u64)
      .map(|n| n as usize);
    let end_line = chunk
      .metadata
      .get("end_line")
      .and_then(Value::as_u64)
      .map(|n| n as usize);
    let lines: Vec<&str> = chunk.content.lines().collect();
    if let (Some(start), Some(end), Some(prev_end)) = (start_line, end_line, max_end_line)
      && start < prev_end
    {
      let overlap = prev_end.saturating_sub(start).min(lines.len());
      out_lines.extend_from_slice(&lines[overlap..]);
      max_end_line = Some(prev_end.max(end));
      continue;
    }
    if !out_lines.is_empty() {
      out_lines.push("");
    }
    out_lines.extend_from_slice(&lines);
    if let Some(end) = end_line {
      max_end_line = Some(max_end_line.map_or(end, |prev| prev.max(end)));
    }
  }
  out_lines.join("\n")
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
  Subtitle,
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
  crate::error::JsonReq(req): crate::error::JsonReq<AddSourceRequest>,
) -> Result<Json<Source>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let mut raw_text_to_save: Option<String> = None;
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
      let derived_title = title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| text.chars().take(60).collect());
      raw_text_to_save = Some(text.clone());
      (
        SourceKind::Text,
        derived_title,
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
        "srt" | "vtt" => BytesFormat::Subtitle,
        other => {
          return Err(ApiError::BadRequest(format!(
            "unsupported file type '.{other}': upload .txt, .md, .html, .pdf, .docx, .xlsx, .pptx, .csv, .tsv, .json, .srt or .vtt"
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
  if let Some(raw_text) = &raw_text_to_save {
    state
      .repos
      .sources
      .set_raw_content(tenant.as_str(), id, source.id, raw_text)
      .await?;
  }
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
      Payload::Url(url) => ingest_url(&url).await?,
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
    BytesFormat::Subtitle => subtitle_text(&bytes),
    BytesFormat::PlainText => {
      String::from_utf8(bytes).map_err(|_| "the source is not UTF-8 text".to_string())
    }
  }
}

pub(crate) fn subtitle_text(bytes: &[u8]) -> Result<String, String> {
  let raw = std::str::from_utf8(bytes)
    .map_err(|_| "subtitle file is not UTF-8 text".to_string())?
    .trim_start_matches('\u{feff}');
  let mut cues: Vec<String> = Vec::new();
  let mut cur_ts: Option<String> = None;
  let mut cur_lines: Vec<String> = Vec::new();

  let flush_cue = |ts: &mut Option<String>, lines: &mut Vec<String>, out: &mut Vec<String>| {
    if !lines.is_empty() {
      let body = lines.join(" ");
      if let Some(stamp) = ts.take() {
        out.push(format!("[{stamp}] {body}"));
      } else {
        out.push(body);
      }
      lines.clear();
    } else {
      *ts = None;
    }
  };

  for line in raw.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);
      continue;
    }
    if trimmed.eq_ignore_ascii_case("WEBVTT")
      || trimmed.starts_with("NOTE")
      || trimmed.starts_with("STYLE")
    {
      continue;
    }
    if cur_ts.is_none() && trimmed.chars().all(|c| c.is_ascii_digit()) {
      continue;
    }
    if let Some((start, rest)) = trimmed.split_once("-->") {
      flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);
      let end = rest.split_whitespace().next().unwrap_or(rest);
      cur_ts = Some(format!(
        "{} -> {}",
        start.trim().replace(',', "."),
        end.trim().replace(',', ".")
      ));
      continue;
    }
    let cleaned = trimmed
      .replace("<i>", "")
      .replace("</i>", "")
      .replace("<b>", "")
      .replace("</b>", "");
    if !cleaned.trim().is_empty() {
      cur_lines.push(cleaned.trim().to_string());
    }
  }
  flush_cue(&mut cur_ts, &mut cur_lines, &mut cues);

  if cues.is_empty() {
    return Ok(raw.trim().to_string());
  }
  Ok(cues.join("\n\n"))
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SmartUrlTarget {
  Direct,
  RewriteUrl(String),
  GitHubRepo {
    owner: String,
    repo: String,
    git_ref: Option<String>,
    subpath: Option<String>,
  },
}

fn percent_encode_component(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len());
  for &b in raw.as_bytes() {
    if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
      out.push(b as char);
    } else {
      out.push_str(&format!("%{b:02X}"));
    }
  }
  out
}

fn encode_github_path(raw: &str) -> String {
  raw
    .split('/')
    .filter(|seg| !seg.is_empty())
    .map(percent_encode_component)
    .collect::<Vec<_>>()
    .join("/")
}

pub(crate) fn classify_smart_source_url(raw_url: &str) -> SmartUrlTarget {
  let Ok(parsed) = url::Url::parse(raw_url.trim()) else {
    return SmartUrlTarget::Direct;
  };
  let Some(host) = parsed.host_str().map(str::to_ascii_lowercase) else {
    return SmartUrlTarget::Direct;
  };
  let segments: Vec<&str> = parsed
    .path_segments()
    .map(|it| it.filter(|s| !s.is_empty()).collect())
    .unwrap_or_default();

  if matches!(
    host.as_str(),
    "arxiv.org" | "www.arxiv.org" | "export.arxiv.org"
  ) && segments.len() >= 2
    && matches!(segments[0], "abs" | "e-print" | "html")
  {
    let paper_id = segments[1..].join("/");
    let paper_id = paper_id.strip_suffix(".pdf").unwrap_or(&paper_id);
    if !paper_id.is_empty() {
      return SmartUrlTarget::RewriteUrl(format!("https://arxiv.org/pdf/{paper_id}.pdf"));
    }
  }

  if matches!(host.as_str(), "openreview.net" | "www.openreview.net")
    && segments.first().copied() == Some("forum")
    && let Some((_, forum_id)) = parsed
      .query_pairs()
      .find(|(k, v)| k == "id" && !v.is_empty())
  {
    return SmartUrlTarget::RewriteUrl(format!(
      "https://openreview.net/pdf?id={}",
      percent_encode_component(&forum_id)
    ));
  }

  if matches!(host.as_str(), "github.com" | "www.github.com") && segments.len() >= 2 {
    let owner = segments[0];
    let repo = segments[1].strip_suffix(".git").unwrap_or(segments[1]);
    if !owner.is_empty() && !repo.is_empty() {
      if segments.len() == 2 {
        return SmartUrlTarget::GitHubRepo {
          owner: owner.to_string(),
          repo: repo.to_string(),
          git_ref: None,
          subpath: None,
        };
      }
      if segments[2] == "tree" && segments.len() >= 4 {
        let git_ref = segments[3].to_string();
        let subpath = (segments.len() > 4).then(|| segments[4..].join("/"));
        return SmartUrlTarget::GitHubRepo {
          owner: owner.to_string(),
          repo: repo.to_string(),
          git_ref: Some(git_ref),
          subpath,
        };
      }
      if segments[2] == "blob" && segments.len() >= 5 {
        let enc_owner = percent_encode_component(owner);
        let enc_repo = percent_encode_component(repo);
        let enc_tail = encode_github_path(&segments[3..].join("/"));
        return SmartUrlTarget::RewriteUrl(format!(
          "https://raw.githubusercontent.com/{enc_owner}/{enc_repo}/{enc_tail}"
        ));
      }
    }
  }

  SmartUrlTarget::Direct
}

pub(crate) fn select_github_snapshot_paths(
  tree_paths: &[String],
  subpath: Option<&str>,
  max_files: usize,
) -> Vec<String> {
  const SKIP_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "vendor",
    "__pycache__",
    ".next",
    "coverage",
  ];
  const SKIP_EXTS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "webp", "ico", "pdf", "zip", "tar", "gz", "xz", "woff", "woff2",
    "ttf", "otf", "mp3", "mp4", "mov", "wav", "wasm", "pyc", "so", "dylib", "dll", "exe", "bin",
    "lock",
  ];
  const SKIP_NAMES: &[&str] = &[
    "Cargo.lock",
    "package-lock.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "poetry.lock",
    "uv.lock",
    "go.sum",
  ];

  let prefix = subpath
    .map(|s| s.trim_matches('/'))
    .filter(|s| !s.is_empty());

  let mut scored: Vec<(u8, usize, String)> = Vec::new();
  for path in tree_paths {
    if let Some(pfx) = prefix
      && path != pfx
      && !path.starts_with(&format!("{pfx}/"))
    {
      continue;
    }
    if path
      .split('/')
      .any(|seg| SKIP_DIRS.contains(&seg) || seg.starts_with('.'))
    {
      continue;
    }
    let file_name = path.rsplit('/').next().unwrap_or(path.as_str());
    if SKIP_NAMES.contains(&file_name) {
      continue;
    }
    let ext = FsPath::new(file_name)
      .extension()
      .and_then(|e| e.to_str())
      .map(str::to_ascii_lowercase);
    if let Some(ref e) = ext
      && SKIP_EXTS.contains(&e.as_str())
    {
      continue;
    }
    let lower_name = file_name.to_ascii_lowercase();
    let priority = if lower_name.starts_with("readme") {
      0
    } else if matches!(
      file_name,
      "Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod" | "pom.xml"
    ) {
      1
    } else if matches!(
      lower_name.as_str(),
      "lib.rs"
        | "main.rs"
        | "mod.rs"
        | "index.ts"
        | "index.tsx"
        | "app.tsx"
        | "main.py"
        | "main.go"
    ) {
      2
    } else if matches!(ext.as_deref(), Some("md" | "toml" | "yaml" | "yml")) {
      3
    } else {
      4
    };
    let depth = path.matches('/').count();
    scored.push((priority, depth, path.clone()));
  }
  scored.sort_by(|a, b| {
    a.0
      .cmp(&b.0)
      .then_with(|| a.1.cmp(&b.1))
      .then_with(|| a.2.cmp(&b.2))
  });
  scored
    .into_iter()
    .take(max_files)
    .map(|(_, _, p)| p)
    .collect()
}

async fn ingest_github_repo_snapshot(
  owner: &str,
  repo: &str,
  git_ref: Option<&str>,
  subpath: Option<&str>,
) -> Result<(String, String), String> {
  const MAX_SNAPSHOT_FILES: usize = 16;
  const MAX_SNAPSHOT_TOTAL_CHARS: usize = 96_000;
  const MAX_SINGLE_FILE_CHARS: usize = 10_000;

  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|e| format!("http client: {e}"))?;
  let gh_headers = json!({
    "Accept": "application/vnd.github+json",
    "User-Agent": "yanshi-notebook/0.2",
  });

  let enc_owner = percent_encode_component(owner);
  let enc_repo = percent_encode_component(repo);
  let repo_api = format!("https://api.github.com/repos/{enc_owner}/{enc_repo}");
  let meta_res = tool
    .send_bytes(
      &json!({ "url": repo_api, "method": "GET", "headers": gh_headers }),
      Some(512 * 1024),
    )
    .await
    .map_err(|e| format!("fetching GitHub repo metadata failed: {e}"))?;
  if !meta_res.status.is_success() {
    return Err(format!(
      "GitHub repo metadata returned HTTP {}",
      meta_res.status
    ));
  }
  let meta_json: Value =
    serde_json::from_slice(&meta_res.body).map_err(|e| format!("invalid GitHub repo JSON: {e}"))?;
  let mut resolved_ref = git_ref
    .filter(|r| !r.is_empty())
    .or_else(|| meta_json.get("default_branch").and_then(Value::as_str))
    .unwrap_or("main")
    .to_string();
  let mut resolved_subpath = subpath.map(str::to_string);
  let description = meta_json
    .get("description")
    .and_then(Value::as_str)
    .unwrap_or_default()
    .trim();

  let fetch_tree = |ref_name: &str| {
    let enc_ref = percent_encode_component(ref_name);
    let url = format!(
      "https://api.github.com/repos/{enc_owner}/{enc_repo}/git/trees/{enc_ref}?recursive=1"
    );
    let tool = &tool;
    let gh_headers = &gh_headers;
    async move {
      tool
        .send_bytes(
          &json!({ "url": url, "method": "GET", "headers": gh_headers }),
          Some(4 * 1024 * 1024),
        )
        .await
    }
  };

  let mut tree_res = fetch_tree(&resolved_ref)
    .await
    .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;

  // Resolve slash-bearing branch/tag names (e.g. `/tree/feat/studio/crates`) when
  // the single-segment ref lookup returns 404.
  if tree_res.status.as_u16() == 404
    && let (Some(first_seg), Some(rest_sub)) = (git_ref, subpath)
  {
    let full_tail = format!("{first_seg}/{rest_sub}");
    let enc_prefix = percent_encode_component(first_seg);
    let mut candidate_refs: Vec<String> = Vec::new();
    for ns in ["heads", "tags"] {
      let refs_url = format!(
        "https://api.github.com/repos/{enc_owner}/{enc_repo}/git/matching-refs/{ns}/{enc_prefix}"
      );
      if let Ok(res) = tool
        .send_bytes(
          &json!({ "url": refs_url, "method": "GET", "headers": gh_headers }),
          Some(512 * 1024),
        )
        .await
        && res.status.is_success()
        && let Ok(Value::Array(items)) = serde_json::from_slice::<Value>(&res.body)
      {
        let strip_pfx = format!("refs/{ns}/");
        for item in items {
          if let Some(r) = item
            .get("ref")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix(&strip_pfx))
          {
            candidate_refs.push(r.to_string());
          }
        }
      }
    }
    candidate_refs.sort_by_key(|r| std::cmp::Reverse(r.len()));
    for cand in candidate_refs {
      if full_tail == cand {
        resolved_ref = cand;
        resolved_subpath = None;
        tree_res = fetch_tree(&resolved_ref)
          .await
          .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;
        break;
      } else if let Some(rem) = full_tail.strip_prefix(&format!("{cand}/")) {
        resolved_ref = cand;
        resolved_subpath = (!rem.is_empty()).then(|| rem.to_string());
        tree_res = fetch_tree(&resolved_ref)
          .await
          .map_err(|e| format!("fetching GitHub tree failed: {e}"))?;
        break;
      }
    }
  }

  if !tree_res.status.is_success() {
    return Err(format!("GitHub tree returned HTTP {}", tree_res.status));
  }
  let tree_json: Value =
    serde_json::from_slice(&tree_res.body).map_err(|e| format!("invalid GitHub tree JSON: {e}"))?;
  let blob_paths: Vec<String> = tree_json
    .get("tree")
    .and_then(Value::as_array)
    .map(|items| {
      items
        .iter()
        .filter(|entry| entry.get("type").and_then(Value::as_str) == Some("blob"))
        .filter_map(|entry| {
          entry
            .get("path")
            .and_then(Value::as_str)
            .map(str::to_string)
        })
        .collect()
    })
    .unwrap_or_default();

  let chosen =
    select_github_snapshot_paths(&blob_paths, resolved_subpath.as_deref(), MAX_SNAPSHOT_FILES);
  let mut md = format!("# GitHub Repository: {owner}/{repo} (`{resolved_ref}`)\n\n");
  if !description.is_empty() {
    md.push_str(&format!("{description}\n\n"));
  }
  if let Some(sub) = resolved_subpath.as_deref().filter(|s| !s.is_empty()) {
    md.push_str(&format!("Subpath filter: `{sub}`\n\n"));
  }
  md.push_str("## Repository File Tree\n\n```text\n");
  for path in blob_paths.iter().take(250) {
    md.push_str(path);
    md.push('\n');
  }
  if blob_paths.len() > 250 {
    md.push_str(&format!("... ({} more files)\n", blob_paths.len() - 250));
  }
  md.push_str("```\n\n");

  let enc_resolved_ref = encode_github_path(&resolved_ref);
  let mut used_chars = 0usize;
  for rel_path in chosen {
    if used_chars >= MAX_SNAPSHOT_TOTAL_CHARS {
      break;
    }
    let enc_rel = encode_github_path(&rel_path);
    let raw_url = format!(
      "https://raw.githubusercontent.com/{enc_owner}/{enc_repo}/{enc_resolved_ref}/{enc_rel}"
    );
    let Ok(raw_res) = tool
      .send_bytes(
        &json!({ "url": raw_url, "method": "GET" }),
        Some(256 * 1024),
      )
      .await
    else {
      continue;
    };
    if !raw_res.status.is_success() {
      continue;
    }
    let Ok(content) = String::from_utf8(raw_res.body) else {
      continue;
    };
    let trimmed: String = content.chars().take(MAX_SINGLE_FILE_CHARS).collect();
    used_chars += trimmed.len();
    md.push_str(&format!("## File: `{rel_path}`\n\n```\n{trimmed}\n```\n\n"));
  }

  let digest = format!("{:x}", Sha256::digest(md.as_bytes()));
  Ok((digest, md))
}

async fn ingest_url(url: &str) -> Result<(String, String), String> {
  match classify_smart_source_url(url) {
    SmartUrlTarget::GitHubRepo {
      owner,
      repo,
      git_ref,
      subpath,
    } => {
      match ingest_github_repo_snapshot(&owner, &repo, git_ref.as_deref(), subpath.as_deref()).await
      {
        Ok(snapshot) => return Ok(snapshot),
        // Explicit `/tree/<ref>` URLs must surface the 404/API error rather than
        // silently scraping GitHub's HTML navigation chrome.
        Err(err) if git_ref.is_some() => return Err(err),
        Err(_) => {}
      }
    }
    SmartUrlTarget::RewriteUrl(rewritten) => {
      let is_github_raw = rewritten.starts_with("https://raw.githubusercontent.com/");
      match fetch(&rewritten).await {
        Ok((bytes, format)) => {
          let text = extract_bytes_text(bytes.clone(), format).await?;
          if !text.trim().is_empty() {
            let digest = format!("{:x}", Sha256::digest(&bytes));
            return Ok((digest, text));
          }
        }
        Err(err) if is_github_raw => return Err(err),
        Err(_) => {}
      }
    }
    SmartUrlTarget::Direct => {}
  }

  let (bytes, format) = fetch(url).await?;
  let digest = format!("{:x}", Sha256::digest(&bytes));
  let text = extract_bytes_text(bytes, format).await?;
  Ok((digest, text))
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
  let content_type = response
    .content_type
    .as_deref()
    .and_then(|ct| ct.split(';').next())
    .map(|ct| ct.trim().to_ascii_lowercase());
  if let Some(ref ct) = content_type
    && (ct.starts_with("audio/") || ct.starts_with("video/") || ct.starts_with("image/"))
  {
    return Err(format!(
      "URL returned binary media Content-Type '{ct}'; upload audio via `kind: recording` or run `video_summarizer` in Studio"
    ));
  }
  let bytes = response.body;
  let ext = url_path_extension(url);
  let format = if bytes.starts_with(b"%PDF-") || content_type.as_deref() == Some("application/pdf")
  {
    BytesFormat::Pdf
  } else if let Some(ooxml) = sniff_ooxml_zip_format(&bytes, ext.as_deref()) {
    ooxml
  } else {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]);
    let lowered = sample.trim_start().to_ascii_lowercase();
    if lowered.starts_with("<!doctype html")
      || lowered.contains("<html")
      || matches!(
        content_type.as_deref(),
        Some("text/html" | "application/xhtml+xml")
      )
    {
      BytesFormat::Html
    } else if ext.as_deref() == Some("csv") || content_type.as_deref() == Some("text/csv") {
      BytesFormat::Csv
    } else if ext.as_deref() == Some("tsv")
      || content_type.as_deref() == Some("text/tab-separated-values")
    {
      BytesFormat::Tsv
    } else if ext.as_deref() == Some("json")
      || content_type
        .as_deref()
        .is_some_and(|ct| ct == "application/json" || ct.ends_with("+json"))
    {
      BytesFormat::Json
    } else if matches!(ext.as_deref(), Some("srt" | "vtt"))
      || matches!(
        content_type.as_deref(),
        Some("text/vtt" | "application/x-subrip")
      )
    {
      BytesFormat::Subtitle
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
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub message_id: Option<Uuid>,
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
  crate::error::JsonReq(req): crate::error::JsonReq<AskNotebookRequest>,
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

  let citations_json = serde_json::to_value(&citations).unwrap_or_else(|_| json!([]));
  let persisted = state
    .repos
    .notebook_messages
    .append(NewNotebookMessage {
      notebook_id: id,
      tenant_id: tenant.as_str().to_string(),
      question: question.clone(),
      answer: answer.clone(),
      model: model.clone(),
      citations: citations_json,
      used_chunk_count: i32::try_from(used_chunk_count).unwrap_or(i32::MAX),
    })
    .await?;

  Ok(Json(AskNotebookResponse {
    message_id: persisted.map(|m| m.id),
    notebook_id: id,
    question,
    answer,
    model,
    citations,
    used_chunk_count,
  }))
}

#[derive(Debug, Default, Deserialize)]
pub struct ListMessagesQuery {
  #[serde(default)]
  pub limit: Option<usize>,
}

pub async fn list_notebook_messages(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  Query(query): Query<ListMessagesQuery>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let limit = query.limit.unwrap_or(200).clamp(1, 500) as i64;
  let messages = state
    .repos
    .notebook_messages
    .list_for_notebook(tenant.as_str(), id, limit)
    .await?;
  Ok(Json(json!({
    "notebook_id": id,
    "messages": messages,
  })))
}

pub async fn clear_notebook_messages(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let deleted = state
    .repos
    .notebook_messages
    .clear_for_notebook(tenant.as_str(), id)
    .await?;
  Ok(Json(json!({
    "notebook_id": id,
    "deleted": deleted,
  })))
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
/// Maximum number of generator runs a single `generate-batch` or `plan` call may contain.
pub const MAX_BATCH_GENERATE_ITEMS: usize = 6;
/// Default maximum deliverables suggested by `POST /v1/notebooks/{id}/plan`.
const DEFAULT_PLAN_MAX_ITEMS: usize = 4;

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
  /// Optional USD spend cap for this generation run (`RFC_CONTENT_STUDIO` §6 / §7).
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchGenerateItem {
  pub kind: String,
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
}

#[derive(Debug, Deserialize)]
pub struct GenerateBatchRequest {
  pub items: Vec<BatchGenerateItem>,
  /// Optional subset of notebook source IDs to include across all items in the batch.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  /// Model for the generators' llm steps; falls back to `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional aggregate USD spend cap across the batch (`> 0` and finite),
  /// split evenly across each dispatched run (`total / items.len()`).
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchedNotebookRun {
  pub run_id: Uuid,
  pub kind: String,
  pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateBatchResponse {
  pub notebook_id: Uuid,
  pub runs: Vec<DispatchedNotebookRun>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub per_run_cost_limit_usd: Option<f64>,
}

struct PreparedGeneratorItem {
  kind: String,
  template: &'static str,
  inputs: serde_json::Map<String, Value>,
}

fn find_generator_template(kind: &str) -> Result<&'static str, ApiError> {
  GENERATORS
    .iter()
    .find(|(k, _)| *k == kind)
    .map(|(_, tpl)| *tpl)
    .ok_or_else(|| {
      ApiError::BadRequest(format!(
        "unknown kind '{kind}': expected one of {}",
        GENERATORS
          .iter()
          .map(|(k, _)| *k)
          .collect::<Vec<_>>()
          .join(", ")
      ))
    })
}

async fn load_scoped_ready_chunks(
  state: &AppState,
  tenant: &TenantId,
  notebook_id: Uuid,
  source_ids: Option<&[Uuid]>,
  verb: &str,
) -> Result<Vec<SourceChunk>, ApiError> {
  let scope = validate_source_scope(state, tenant, notebook_id, source_ids).await?;
  let mut ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), notebook_id)
    .await?;
  if let Some(allowed) = &scope {
    ready_chunks.retain(|c| allowed.contains(&c.source_id));
  }
  if ready_chunks.is_empty() {
    let msg = if scope.is_some() {
      format!("selected sources have no ready chunks to {verb}")
    } else {
      format!("notebook {notebook_id} has no ready sources to {verb}")
    };
    return Err(ApiError::BadRequest(msg));
  }
  Ok(ready_chunks)
}

async fn prepare_generator_item(
  kind: &str,
  params: serde_json::Map<String, Value>,
  ready_chunks: &[SourceChunk],
  model: &str,
  gateway_policy: &crate::gateway_policy::GatewayPolicy,
) -> Result<PreparedGeneratorItem, ApiError> {
  let trimmed_kind = kind.trim();
  let template = find_generator_template(trimmed_kind)?;
  let definition = yanshi_config::executor::parse_workflow_definition(template)
    .map_err(|e| ApiError::Internal(format!("generator template: {e:#}")))?;
  for key in params.keys() {
    if matches!(key.as_str(), "chunks" | "url" | "text") {
      return Err(ApiError::BadRequest(format!(
        "'{key}' comes from the notebook and cannot be set"
      )));
    }
    if !definition.inputs.contains_key(key) {
      return Err(ApiError::BadRequest(format!(
        "'{key}' is not a parameter of '{trimmed_kind}'"
      )));
    }
  }

  let query_hint = params
    .get("focus")
    .and_then(Value::as_str)
    .or_else(|| params.get("topic").and_then(Value::as_str))
    .map(str::trim)
    .filter(|s| !s.is_empty());
  let selected =
    select_chunks_for_generate(ready_chunks.to_vec(), query_hint, GENERATE_CHUNK_BUDGET).await;
  let chunks: Vec<Value> = selected
    .into_iter()
    .map(|chunk| {
      json!({ "chunk_id": chunk.chunk_id, "source_id": chunk.source_id, "content": chunk.content })
    })
    .collect();
  let mut inputs = params;
  inputs.insert("chunks".to_string(), Value::Array(chunks));
  if definition.inputs.contains_key("chat_model") && !inputs.contains_key("chat_model") {
    inputs.insert("chat_model".to_string(), Value::String(model.to_string()));
  }
  gateway_policy
    .check_run_inputs(&definition, &inputs)
    .map_err(ApiError::BadRequest)?;

  Ok(PreparedGeneratorItem {
    kind: trimmed_kind.to_string(),
    template,
    inputs,
  })
}

fn split_batch_cost_limit(
  total_limit_usd: Option<f64>,
  item_count: usize,
) -> Result<Option<f64>, ApiError> {
  let Some(limit) = total_limit_usd else {
    return Ok(None);
  };
  let validated =
    yanshi_config::config::v2::validate_cost_limit(limit).map_err(ApiError::BadRequest)?;
  let count = item_count.max(1) as f64;
  let per_run = validated / count;
  yanshi_config::config::v2::validate_cost_limit(per_run).map_err(ApiError::BadRequest)?;
  Ok(Some(per_run))
}

pub async fn generate(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  crate::error::JsonReq(req): crate::error::JsonReq<GenerateRequest>,
) -> Result<Json<Value>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let _ = find_generator_template(&req.kind)?;
  let model = resolve_notebook_model(req.model.as_deref())?;
  if let Some(limit) = req.cost_limit_usd {
    yanshi_config::config::v2::validate_cost_limit(limit).map_err(ApiError::BadRequest)?;
  }
  let ready_chunks = load_scoped_ready_chunks(
    &state,
    &tenant,
    id,
    req.source_ids.as_deref(),
    "generate from",
  )
  .await?;
  let prepared = prepare_generator_item(
    &req.kind,
    req.params,
    &ready_chunks,
    &model,
    &state.gateway_policy,
  )
  .await?;

  let run = crate::runs::admit_and_dispatch(
    &state,
    state.executor.clone(),
    tenant.as_str().to_string(),
    prepared.template.to_string(),
    crate::runs::RunOptions {
      inputs: prepared.inputs,
      model: Some(model),
      cost_limit_usd: req.cost_limit_usd,
      notebook: Some(crate::runs::NotebookLink {
        notebook_id: id,
        kind: prepared.kind.clone(),
      }),
      ..Default::default()
    },
  )
  .await?;
  Ok(Json(
    json!({ "run_id": run.id, "kind": prepared.kind, "status": run.status }),
  ))
}

pub async fn generate_batch(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  crate::error::JsonReq(req): crate::error::JsonReq<GenerateBatchRequest>,
) -> Result<Json<GenerateBatchResponse>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  if req.items.is_empty() {
    return Err(ApiError::BadRequest(
      "items must contain at least one generator".into(),
    ));
  }
  if req.items.len() > MAX_BATCH_GENERATE_ITEMS {
    return Err(ApiError::BadRequest(format!(
      "batch contains {} items, exceeding limit of {MAX_BATCH_GENERATE_ITEMS}",
      req.items.len()
    )));
  }
  let max_concurrent = state.run_admission_registry.max_concurrent_per_tenant() as usize;
  if req.items.len() > max_concurrent {
    return Err(ApiError::TooManyRequests(format!(
      "batch of {} items exceeds tenant concurrency limit ({max_concurrent})",
      req.items.len()
    )));
  }
  let model = resolve_notebook_model(req.model.as_deref())?;
  let per_run_cost_limit_usd = split_batch_cost_limit(req.cost_limit_usd, req.items.len())?;
  let ready_chunks = load_scoped_ready_chunks(
    &state,
    &tenant,
    id,
    req.source_ids.as_deref(),
    "generate from",
  )
  .await?;

  // All-or-nothing preflight: validate every item's kind, params, gateway policy,
  // and active tenant concurrency/rate budget BEFORE creating or dispatching run #1.
  let mut prepared_items = Vec::with_capacity(req.items.len());
  for item in req.items {
    prepared_items.push(
      prepare_generator_item(
        &item.kind,
        item.params,
        &ready_chunks,
        &model,
        &state.gateway_policy,
      )
      .await?,
    );
  }

  use yanshi_db::RunRepo as _;
  let queued_db = state
    .repos
    .runs
    .list_filtered(
      tenant.as_str(),
      Some("queued"),
      (max_concurrent + 1) as i64,
      0,
    )
    .await?
    .len();
  let running_db = state
    .repos
    .runs
    .list_filtered(
      tenant.as_str(),
      Some("running"),
      (max_concurrent + 1) as i64,
      0,
    )
    .await?
    .len();
  let active_db = queued_db + running_db;
  if active_db + prepared_items.len() > max_concurrent {
    return Err(ApiError::TooManyRequests(format!(
      "batch of {} items plus {active_db} active run(s) exceeds tenant concurrency limit ({max_concurrent})",
      prepared_items.len()
    )));
  }

  let admission_guards = state
    .run_admission_registry
    .try_admit_batch(tenant.as_str(), prepared_items.len())
    .map_err(|err| ApiError::TooManyRequests(err.to_string()))?;

  let mut runs = Vec::with_capacity(prepared_items.len());
  for (prepared, guard) in prepared_items.into_iter().zip(admission_guards) {
    let run = crate::runs::admit_and_dispatch_with_guard(
      &state,
      state.executor.clone(),
      tenant.as_str().to_string(),
      prepared.template.to_string(),
      crate::runs::RunOptions {
        inputs: prepared.inputs,
        model: Some(model.clone()),
        cost_limit_usd: per_run_cost_limit_usd,
        notebook: Some(crate::runs::NotebookLink {
          notebook_id: id,
          kind: prepared.kind.clone(),
        }),
        ..Default::default()
      },
      guard,
    )
    .await?;
    runs.push(DispatchedNotebookRun {
      run_id: run.id,
      kind: prepared.kind,
      status: run.status,
    });
  }

  Ok(Json(GenerateBatchResponse {
    notebook_id: id,
    runs,
    per_run_cost_limit_usd,
  }))
}

// ── Studio content planner (`POST /v1/notebooks/{id}/plan`, RFC §2/§3) ──

#[derive(Debug, Deserialize)]
pub struct PlanNotebookRequest {
  pub goal: String,
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub max_items: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedGeneratorItem {
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNotebookResponse {
  pub notebook_id: Uuid,
  pub goal: String,
  pub model: String,
  pub rationale: String,
  pub items: Vec<PlannedGeneratorItem>,
  pub used_chunk_count: usize,
}

fn studio_plan_schema() -> Value {
  let kinds: Vec<Value> = GENERATORS
    .iter()
    .map(|(k, _)| Value::String((*k).to_string()))
    .collect();
  json!({
    "type": "object",
    "additionalProperties": false,
    "required": ["rationale", "items"],
    "properties": {
      "rationale": { "type": "string" },
      "items": {
        "type": "array",
        "items": {
          "type": "object",
          "additionalProperties": false,
          "required": ["kind", "reason", "parameters"],
          "properties": {
            "kind": {
              "type": "string",
              "enum": kinds
            },
            "reason": { "type": "string" },
            "parameters": {
              "type": "array",
              "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["key", "value"],
                "properties": {
                  "key": { "type": "string" },
                  "value": { "type": "string" }
                }
              }
            }
          }
        }
      }
    }
  })
}

fn build_generator_catalogue_prompt() -> String {
  let mut lines = Vec::with_capacity(GENERATORS.len());
  for &(kind, template) in GENERATORS {
    let Ok(def) = yanshi_config::executor::parse_workflow_definition(template) else {
      continue;
    };
    let mut param_names: Vec<&String> = def
      .inputs
      .keys()
      .filter(|k| !matches!(k.as_str(), "chunks" | "url" | "text" | "chat_model"))
      .collect();
    param_names.sort();
    let param_specs = param_names
      .into_iter()
      .map(|name| {
        let spec = &def.inputs[name];
        let default_hint = spec
          .default
          .as_ref()
          .and_then(|v| serde_json::to_value(v).ok())
          .map(|v| format!("={v}"))
          .unwrap_or_default();
        let desc = spec
          .description
          .as_deref()
          .map(str::trim)
          .filter(|d| !d.is_empty())
          .map(|d| format!(" ({d})"))
          .unwrap_or_default();
        format!("{name}{default_hint}{desc}")
      })
      .collect::<Vec<_>>()
      .join("; ");
    lines.push(format!("- `{kind}`: params [{param_specs}]"));
  }
  lines.join("\n")
}

fn coerce_planned_param_value(
  raw: Value,
  spec: &yanshi_config::config::v2::InputDefinitionV2,
) -> Option<Value> {
  if raw.is_null() {
    return None;
  }
  match &spec.default {
    Some(serde_yaml::Value::Bool(_)) => match raw {
      Value::Bool(b) => Some(Value::Bool(b)),
      Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(Value::Bool(true)),
        "false" | "0" | "no" => Some(Value::Bool(false)),
        _ => None,
      },
      _ => None,
    },
    Some(serde_yaml::Value::Number(default_num)) => {
      let prefer_int = default_num.is_i64() || default_num.is_u64();
      match raw {
        Value::Number(n) => {
          if prefer_int {
            n.as_i64()
              .map(Value::from)
              .or_else(|| n.as_f64().map(|f| Value::from(f.round() as i64)))
          } else {
            n.as_f64().filter(|f| f.is_finite()).map(Value::from)
          }
        }
        Value::String(s) => {
          let trimmed = s.trim();
          if prefer_int {
            trimmed.parse::<i64>().ok().map(Value::from).or_else(|| {
              trimmed
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(|f| Value::from(f.round() as i64))
            })
          } else {
            trimmed
              .parse::<f64>()
              .ok()
              .filter(|f| f.is_finite())
              .map(Value::from)
          }
        }
        _ => None,
      }
    }
    Some(serde_yaml::Value::String(_)) | None => match raw {
      Value::String(s) => {
        let trimmed = s.trim();
        if trimmed.is_empty() {
          None
        } else {
          Some(Value::String(trimmed.to_string()))
        }
      }
      Value::Bool(b) => Some(Value::String(b.to_string())),
      Value::Number(n) => Some(Value::String(n.to_string())),
      _ => None,
    },
    Some(_) => None,
  }
}

fn sanitize_studio_plan_reply(
  raw_reply: &str,
  max_items: usize,
  gateway_policy: &crate::gateway_policy::GatewayPolicy,
) -> Result<(String, Vec<PlannedGeneratorItem>), String> {
  #[derive(Deserialize)]
  struct RawKvParam {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: Value,
  }
  #[derive(Deserialize)]
  struct RawPlanItem {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    parameters: Vec<RawKvParam>,
    #[serde(default)]
    params: serde_json::Map<String, Value>,
  }
  #[derive(Deserialize)]
  struct RawPlan {
    #[serde(default)]
    rationale: String,
    #[serde(default)]
    items: Vec<RawPlanItem>,
  }

  let value = extract_json_object(raw_reply)?;
  let parsed: RawPlan =
    serde_json::from_value(value).map_err(|e| format!("invalid studio plan JSON: {e}"))?;
  let capped = max_items.clamp(1, MAX_BATCH_GENERATE_ITEMS);

  let mut seen_kinds: HashSet<String> = HashSet::new();
  let mut items = Vec::new();

  for raw_item in parsed.items {
    if items.len() >= capped {
      break;
    }
    let kind = raw_item.kind.trim().to_string();
    let Some(&(_, template)) = GENERATORS.iter().find(|(k, _)| *k == kind) else {
      continue;
    };
    if !seen_kinds.insert(kind.clone()) {
      continue;
    }
    let Ok(definition) = yanshi_config::executor::parse_workflow_definition(template) else {
      continue;
    };

    let mut merged_candidates: Vec<(String, Value)> = Vec::new();
    for kv in raw_item.parameters {
      merged_candidates.push((kv.key, kv.value));
    }
    for (k, v) in raw_item.params {
      merged_candidates.push((k, v));
    }

    let mut clean_params = serde_json::Map::new();
    for (raw_key, raw_val) in merged_candidates {
      let key = raw_key.trim();
      if key.is_empty() || matches!(key, "chunks" | "url" | "text" | "chat_model") {
        continue;
      }
      let Some(spec) = definition.inputs.get(key) else {
        continue;
      };
      let Some(coerced) = coerce_planned_param_value(raw_val, spec) else {
        continue;
      };
      let mut probe = clean_params.clone();
      probe.insert(key.to_string(), coerced.clone());
      if gateway_policy.check_run_inputs(&definition, &probe).is_ok() {
        clean_params.insert(key.to_string(), coerced);
      }
    }

    let reason = raw_item
      .reason
      .as_deref()
      .map(str::trim)
      .filter(|r| !r.is_empty())
      .map(|r| truncate_chars(r, 400));

    items.push(PlannedGeneratorItem {
      kind,
      reason,
      params: clean_params,
    });
  }

  if items.is_empty() {
    return Err("studio planner returned no valid generator items".to_string());
  }

  let rationale = {
    let trimmed = truncate_chars(&parsed.rationale, 1_200);
    if trimmed.is_empty() {
      format!(
        "Selected {} studio deliverable(s) for this notebook.",
        items.len()
      )
    } else {
      trimmed
    }
  };
  Ok((rationale, items))
}

pub async fn plan_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  crate::error::JsonReq(req): crate::error::JsonReq<PlanNotebookRequest>,
) -> Result<Json<PlanNotebookResponse>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let goal = req.goal.trim().to_string();
  if goal.is_empty() {
    return Err(ApiError::BadRequest("goal must not be empty".into()));
  }
  let max_items = req
    .max_items
    .unwrap_or(DEFAULT_PLAN_MAX_ITEMS)
    .clamp(1, MAX_BATCH_GENERATE_ITEMS);
  let model = resolve_notebook_model(req.model.as_deref())?;
  let ready_chunks =
    load_scoped_ready_chunks(&state, &tenant, id, req.source_ids.as_deref(), "plan from").await?;
  let selected = select_chunks_for_generate(ready_chunks, Some(&goal), 12_000).await;
  let used_chunk_count = selected.len();
  let passage_digest = selected
    .iter()
    .take(10)
    .map(|c| format!("[{}] {}", c.chunk_id, truncate_chars(&c.content, 600)))
    .collect::<Vec<_>>()
    .join("\n\n");
  let catalogue = build_generator_catalogue_prompt();
  let schema = studio_plan_schema();
  let schema_str = serde_json::to_string(&schema).unwrap_or_default();

  let prompt = format!(
    "You are the Content Studio producer for a research notebook (`RFC_CONTENT_STUDIO` §3).\n\
     Choose between 1 and {max_items} distinct deliverable generators from the catalogue below \
     that best satisfy the user's goal, and tailor each generator's declared parameters (`focus`, counts, etc.) \
     to the notebook's actual passages.\n\
     Never invent generator kinds or parameter names outside the catalogue, and never set `url`, `text`, or `chunks`.\n\n\
     User goal: {goal}\n\n\
     Available Studio generators:\n{catalogue}\n\n\
     Representative notebook passages:\n{passage_digest}\n\n\
     Output ONLY valid JSON strictly conforming to this JSON Schema (no markdown fences, no extra prose):\n{schema_str}"
  );

  yanshi_llm::Yanshi::ensure_init()
    .await
    .map_err(|e| ApiError::Internal(format!("initializing model registry: {e}")))?;
  let response = yanshi_llm::Yanshi::model(&model)
    .system(
      "You are a Content Studio planner. Select appropriate deliverable kinds from the closed catalogue and output strict JSON.",
    )
    .prompt(&prompt)
    .json_schema("notebook_studio_plan", schema)
    .execute_full()
    .await
    .map_err(|e| ApiError::Internal(format!("LLM execution failed: {e}")))?;

  let (rationale, items) =
    sanitize_studio_plan_reply(&response.content, max_items, &state.gateway_policy)
      .map_err(|e| ApiError::Internal(format!("parsing studio plan: {e}")))?;

  Ok(Json(PlanNotebookResponse {
    notebook_id: id,
    goal,
    model,
    rationale,
    items,
    used_chunk_count,
  }))
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

#[derive(Debug, Deserialize)]
pub struct AttachNotebookRunRequest {
  pub run_id: Uuid,
  #[serde(default)]
  pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct AttachNotebookRunResponse {
  pub notebook_id: Uuid,
  pub run_id: Uuid,
  pub kind: String,
  pub attached_count: usize,
  pub artifacts: Vec<NotebookArtifact>,
}

async fn load_completed_run_deliverable(
  state: &AppState,
  tenant_id: &str,
  run: &yanshi_db::Run,
) -> Option<Value> {
  // 1. Check `<run_dir>/deliverable_outputs.json` persisted by FlowExecutor.
  let run_dir = run
    .run_dir
    .as_deref()
    .map(std::path::PathBuf::from)
    .unwrap_or_else(|| crate::runs::default_run_dir_for_id(run.id));
  let step_path = run_dir.join("deliverable_outputs.json");
  if let Ok(bytes) = tokio::fs::read(&step_path).await
    && let Ok(result) = serde_json::from_slice::<yanshi_core::async_node::AsyncNodeResult>(&bytes)
  {
    let mut state_map = HashMap::new();
    state_map.insert("deliverable".to_string(), result);
    if let Some(deliv) = yanshi_config::content::extract_deliverable_from_state(&state_map) {
      return Some(deliv);
    }
  }

  // 2. Fallback: scan persisted `node_output_captured` events for `deliverable`.
  if let Ok(events) = state
    .repos
    .events
    .list_after(tenant_id, run.id, -1, 500)
    .await
  {
    for ev in events.into_iter().rev() {
      if ev.kind == "node_output_captured"
        && ev.payload.get("node_id").and_then(Value::as_str) == Some("deliverable")
        && let Some(output) = ev.payload.get("output")
      {
        if let Some(inner) = output.get("deliverable").filter(|v| v.is_object()) {
          return Some(inner.clone());
        }
        if output.get("kind").is_some() && output.get("title").is_some() {
          return Some(output.clone());
        }
      }
    }
  }
  None
}

/// `POST /v1/notebooks/{id}/artifacts` — attach primary/preview deliverables of
/// any completed Studio / workflow run owned by the same tenant to `notebook_id`.
pub async fn attach_notebook_run_artifacts(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
  crate::error::JsonReq(req): crate::error::JsonReq<AttachNotebookRunRequest>,
) -> Result<Json<AttachNotebookRunResponse>, ApiError> {
  own_notebook(&state, &tenant, id).await?;
  let run = state
    .repos
    .runs
    .get(req.run_id)
    .await?
    .filter(|r| r.tenant_id == tenant.as_str())
    .ok_or_else(|| ApiError::NotFound(format!("run {} not found", req.run_id)))?;
  if run.status != "succeeded" {
    return Err(ApiError::BadRequest(format!(
      "run {} has status '{}'; only succeeded runs can be attached to a notebook",
      req.run_id, run.status
    )));
  }

  let run_artifacts = state.repos.artifacts.list_for_run(req.run_id).await?;
  if run_artifacts.is_empty() {
    return Err(ApiError::BadRequest(format!(
      "run {} produced no downloadable artifacts to attach",
      req.run_id
    )));
  }

  let deliverable = load_completed_run_deliverable(&state, tenant.as_str(), &run).await;
  let resolved_kind = req
    .kind
    .as_deref()
    .map(str::trim)
    .filter(|k| !k.is_empty())
    .map(str::to_string)
    .or_else(|| {
      deliverable
        .as_ref()
        .and_then(|d| d.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
    })
    .or_else(|| run.notebook_kind.clone())
    .or_else(|| {
      yanshi_config::executor::parse_workflow_definition(&run.workflow)
        .ok()
        .map(|d| d.name)
        .filter(|n| !n.trim().is_empty())
    })
    .unwrap_or_else(|| "report".to_string());

  let selected =
    crate::artifacts::select_notebook_deliverable_artifacts(&run_artifacts, deliverable.as_ref());
  let mut attached_ids: HashSet<Uuid> = HashSet::new();
  for artifact in selected {
    if state
      .repos
      .notebooks
      .attach_artifact_with_deliverable(
        tenant.as_str(),
        id,
        artifact.id,
        &resolved_kind,
        deliverable.as_ref(),
      )
      .await?
    {
      attached_ids.insert(artifact.id);
    }
  }

  let all_notebook_artifacts = state
    .repos
    .notebooks
    .list_artifacts(tenant.as_str(), id)
    .await?;
  let attached: Vec<NotebookArtifact> = all_notebook_artifacts
    .into_iter()
    .filter(|na| attached_ids.contains(&na.artifact.id) || na.artifact.run_id == req.run_id)
    .collect();

  Ok(Json(AttachNotebookRunResponse {
    notebook_id: id,
    run_id: req.run_id,
    kind: resolved_kind,
    attached_count: attached_ids.len(),
    artifacts: attached,
  }))
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

// ── Portable bundle export / import & deliverables `.zip` (AD1) ─────────

/// Schema version for portable `NotebookExportBundle` JSON documents.
pub const NOTEBOOK_BUNDLE_VERSION: u32 = 1;
const MAX_IMPORT_SOURCES: usize = 200;
const MAX_IMPORT_CHUNKS_PER_SOURCE: usize = 2_000;

fn default_json_object() -> Value {
  json!({})
}

fn default_ready_status() -> String {
  "ready".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookMeta {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<DateTime<Utc>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSourceChunk {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chunk_id: Option<String>,
  #[serde(default)]
  pub ordinal: i32,
  pub content: String,
  #[serde(default = "default_json_object")]
  pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookSource {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub kind: String,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub uri: Option<String>,
  #[serde(default = "default_ready_status")]
  pub status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub raw_text: Option<String>,
  #[serde(default)]
  pub chunks: Vec<ExportedSourceChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookDeliverable {
  pub artifact_id: Uuid,
  pub run_id: Uuid,
  pub kind: String,
  pub name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub mime_type: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub size_bytes: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  pub created_at: DateTime<Utc>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub deliverable: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookMessage {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub question: String,
  pub answer: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub citations: Value,
  #[serde(default)]
  pub used_chunk_count: i32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookExportBundle {
  pub version: u32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub exported_at: Option<DateTime<Utc>>,
  pub notebook: ExportedNotebookMeta,
  #[serde(default)]
  pub sources: Vec<ExportedNotebookSource>,
  #[serde(default)]
  pub deliverables: Vec<ExportedNotebookDeliverable>,
  #[serde(default)]
  pub messages: Vec<ExportedNotebookMessage>,
  #[serde(default)]
  pub markdown_brief: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ImportNotebookRequest {
  Wrapped {
    bundle: NotebookExportBundle,
    #[serde(default)]
    title: Option<String>,
  },
  Direct(NotebookExportBundle),
}

#[derive(Debug, Serialize)]
pub struct ImportNotebookResponse {
  pub notebook: Notebook,
  pub imported_sources: usize,
  pub imported_chunks: usize,
  pub imported_messages: usize,
  pub imported_deliverables: usize,
}

#[derive(Debug)]
enum ImportValidationError {
  BadRequest(String),
  PayloadTooLarge(String),
}

impl From<ImportValidationError> for ApiError {
  fn from(err: ImportValidationError) -> Self {
    match err {
      ImportValidationError::BadRequest(msg) => Self::BadRequest(msg),
      ImportValidationError::PayloadTooLarge(msg) => Self::PayloadTooLarge(msg),
    }
  }
}

#[derive(Debug)]
struct ValidatedImportChunk {
  old_chunk_id: Option<String>,
  old_ordinal: i32,
  content: String,
  metadata: Value,
}

#[derive(Debug)]
struct ValidatedImportSource {
  original_id: Option<Uuid>,
  kind: SourceKind,
  title: String,
  uri: Option<String>,
  status: String,
  error: Option<String>,
  sha256: Option<String>,
  raw_text: Option<String>,
  chunks: Vec<ValidatedImportChunk>,
}

#[derive(Debug)]
struct ValidatedImportMessage {
  question: String,
  answer: String,
  model: String,
  citations: Value,
  used_chunk_count: i32,
}

#[derive(Debug)]
struct ValidatedImportDeliverable {
  old_run_id: Uuid,
  kind: String,
  name: String,
  mime_type: Option<String>,
  deliverable: Option<Value>,
}

#[derive(Debug)]
struct ValidatedImportBundle {
  title: String,
  sources: Vec<ValidatedImportSource>,
  messages: Vec<ValidatedImportMessage>,
  deliverables: Vec<ValidatedImportDeliverable>,
}

fn parse_source_kind(raw: &str) -> Option<SourceKind> {
  match raw.trim() {
    "url" => Some(SourceKind::Url),
    "file" => Some(SourceKind::File),
    "text" => Some(SourceKind::Text),
    "recording" => Some(SourceKind::Recording),
    _ => None,
  }
}

fn validate_notebook_import_bundle(
  bundle: &NotebookExportBundle,
  title_override: Option<&str>,
) -> Result<ValidatedImportBundle, ImportValidationError> {
  if bundle.version != NOTEBOOK_BUNDLE_VERSION {
    return Err(ImportValidationError::BadRequest(format!(
      "unsupported notebook bundle version {} (expected {})",
      bundle.version, NOTEBOOK_BUNDLE_VERSION
    )));
  }
  let resolved_title = title_override
    .map(str::trim)
    .filter(|t| !t.is_empty())
    .unwrap_or_else(|| bundle.notebook.title.trim());
  if resolved_title.is_empty() {
    return Err(ImportValidationError::BadRequest(
      "notebook title must not be empty".to_string(),
    ));
  }
  if bundle.sources.len() > MAX_IMPORT_SOURCES {
    return Err(ImportValidationError::BadRequest(format!(
      "bundle contains {} sources, exceeding limit of {MAX_IMPORT_SOURCES}",
      bundle.sources.len()
    )));
  }

  let mut total_bytes: usize = 0;
  let mut validated_sources = Vec::with_capacity(bundle.sources.len());

  for (idx, src) in bundle.sources.iter().enumerate() {
    let Some(kind) = parse_source_kind(&src.kind) else {
      return Err(ImportValidationError::BadRequest(format!(
        "source #{idx} has unsupported kind '{}': expected url, file, text, or recording",
        src.kind
      )));
    };
    if src.chunks.len() > MAX_IMPORT_CHUNKS_PER_SOURCE {
      return Err(ImportValidationError::BadRequest(format!(
        "source #{idx} contains {} chunks, exceeding limit of {MAX_IMPORT_CHUNKS_PER_SOURCE}",
        src.chunks.len()
      )));
    }
    let source_title = {
      let t = src.title.trim();
      if !t.is_empty() {
        t.to_string()
      } else if let Some(uri) = src.uri.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        uri.to_string()
      } else {
        format!("source-{}", idx + 1)
      }
    };

    let mut ordered_chunks: Vec<ValidatedImportChunk> = Vec::with_capacity(src.chunks.len());
    for chunk in &src.chunks {
      let content = chunk.content.trim().to_string();
      if content.is_empty() {
        continue;
      }
      total_bytes = total_bytes.saturating_add(content.len());
      if total_bytes > MAX_SOURCE_BYTES {
        return Err(ImportValidationError::PayloadTooLarge(format!(
          "imported chunk text exceeds {MAX_SOURCE_BYTES} bytes"
        )));
      }
      let meta = if chunk.metadata.is_object() {
        chunk.metadata.clone()
      } else {
        json!({})
      };
      ordered_chunks.push(ValidatedImportChunk {
        old_chunk_id: chunk
          .chunk_id
          .as_deref()
          .map(str::trim)
          .filter(|c| !c.is_empty())
          .map(str::to_string),
        old_ordinal: chunk.ordinal,
        content,
        metadata: meta,
      });
    }
    ordered_chunks.sort_by_key(|c| c.old_ordinal);

    validated_sources.push(ValidatedImportSource {
      original_id: src.id,
      kind,
      title: source_title,
      uri: src
        .uri
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string),
      status: src.status.trim().to_ascii_lowercase(),
      error: src
        .error
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_string),
      sha256: src
        .sha256
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(str::to_string),
      raw_text: src
        .raw_text
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string),
      chunks: ordered_chunks,
    });
  }

  let mut validated_messages = Vec::new();
  for msg in bundle.messages.iter().take(200) {
    let q = msg.question.trim();
    let a = msg.answer.trim();
    if q.is_empty() || a.is_empty() {
      continue;
    }
    let model = if msg.model.trim().is_empty() {
      "imported".to_string()
    } else {
      msg.model.trim().to_string()
    };
    let citations = if msg.citations.is_array() {
      msg.citations.clone()
    } else {
      json!([])
    };
    validated_messages.push(ValidatedImportMessage {
      question: q.to_string(),
      answer: a.to_string(),
      model,
      citations,
      used_chunk_count: msg.used_chunk_count.max(0),
    });
  }

  let mut validated_deliverables = Vec::new();
  for deliv in bundle.deliverables.iter().take(200) {
    let kind = deliv.kind.trim();
    if kind.is_empty() {
      continue;
    }
    let name = {
      let n = deliv.name.trim();
      if n.is_empty() {
        format!("{kind}.json")
      } else {
        n.to_string()
      }
    };
    validated_deliverables.push(ValidatedImportDeliverable {
      old_run_id: deliv.run_id,
      kind: kind.to_string(),
      name,
      mime_type: deliv
        .mime_type
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string),
      deliverable: deliv.deliverable.clone().filter(Value::is_object),
    });
  }

  Ok(ValidatedImportBundle {
    title: resolved_title.to_string(),
    sources: validated_sources,
    messages: validated_messages,
    deliverables: validated_deliverables,
  })
}

#[cfg(test)]
pub(crate) fn remap_imported_citations(raw: &Value, source_id_map: &HashMap<Uuid, Uuid>) -> Value {
  remap_imported_json_refs(raw, source_id_map, &HashMap::new())
}

pub(crate) fn remap_imported_json_refs(
  raw: &Value,
  source_id_map: &HashMap<Uuid, Uuid>,
  chunk_id_map: &HashMap<String, String>,
) -> Value {
  match raw {
    Value::Array(arr) => Value::Array(
      arr
        .iter()
        .map(|item| remap_imported_json_refs(item, source_id_map, chunk_id_map))
        .collect(),
    ),
    Value::Object(obj) => {
      let mut next = serde_json::Map::with_capacity(obj.len());
      for (k, v) in obj {
        if k == "source_id"
          && let Some(old_src) = v.as_str().and_then(|s| Uuid::parse_str(s).ok())
          && let Some(new_src) = source_id_map.get(&old_src)
        {
          next.insert(k.clone(), json!(new_src.to_string()));
          continue;
        }
        if k == "chunk_id"
          && let Some(cid) = v.as_str()
        {
          if let Some(mapped) = chunk_id_map.get(cid) {
            next.insert(k.clone(), json!(mapped));
            continue;
          }
          if let Some((prefix, ordinal)) = cid.split_once(':')
            && let Ok(old_src) = Uuid::parse_str(prefix)
            && let Some(new_src) = source_id_map.get(&old_src)
          {
            next.insert(k.clone(), json!(format!("{new_src}:{ordinal}")));
            continue;
          }
        }
        next.insert(
          k.clone(),
          remap_imported_json_refs(v, source_id_map, chunk_id_map),
        );
      }
      Value::Object(next)
    }
    other => other.clone(),
  }
}

fn is_synthesizable_text_deliverable(name: &str, mime_type: Option<&str>) -> bool {
  let lower_name = name.to_ascii_lowercase();
  let lower_mime = mime_type.unwrap_or_default().to_ascii_lowercase();
  lower_name.ends_with(".html")
    || lower_name.ends_with(".htm")
    || lower_name.ends_with(".md")
    || lower_name.ends_with(".csv")
    || lower_name.ends_with(".tsv")
    || lower_name.ends_with(".json")
    || lower_mime.starts_with("text/")
    || lower_mime == "application/json"
}

fn html_escape_min(raw: &str) -> String {
  raw
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

fn synthesize_imported_deliverable_bytes(
  kind: &str,
  name: &str,
  mime_type: Option<&str>,
  deliverable: Option<&Value>,
) -> (String, String, Vec<u8>) {
  let safe_base: String = name
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
        c
      } else {
        '_'
      }
    })
    .collect();
  let lower_name = safe_base.to_ascii_lowercase();
  let lower_mime = mime_type.unwrap_or_default().to_ascii_lowercase();
  let title = deliverable
    .and_then(|d| d.get("title"))
    .and_then(Value::as_str)
    .filter(|t| !t.trim().is_empty())
    .unwrap_or(kind);
  let content = deliverable.and_then(|d| d.get("content"));

  if (lower_name.ends_with(".csv") || lower_mime == "text/csv")
    && let Some(cols) = content
      .and_then(|c| c.get("columns"))
      .and_then(Value::as_array)
  {
    let col_keys: Vec<(String, String)> = cols
      .iter()
      .filter_map(|col| {
        if let Some(s) = col.as_str() {
          Some((s.to_string(), s.to_string()))
        } else {
          let key = col.get("key").or_else(|| col.get("name"))?.as_str()?;
          let label = col
            .get("label")
            .or_else(|| col.get("title"))
            .and_then(Value::as_str)
            .unwrap_or(key);
          Some((key.to_string(), label.to_string()))
        }
      })
      .collect();
    let esc_csv = |cell: &str| {
      if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
      } else {
        cell.to_string()
      }
    };
    let mut lines = Vec::new();
    if !col_keys.is_empty() {
      lines.push(
        col_keys
          .iter()
          .map(|(_, label)| esc_csv(label))
          .collect::<Vec<_>>()
          .join(","),
      );
      if let Some(rows) = content
        .and_then(|c| c.get("rows"))
        .and_then(Value::as_array)
      {
        for row in rows {
          let cells = if let Some(obj) = row.as_object() {
            let values_map = obj.get("values").and_then(Value::as_object).unwrap_or(obj);
            col_keys
              .iter()
              .map(|(k, _)| match values_map.get(k) {
                Some(Value::String(s)) => esc_csv(s),
                Some(Value::Null) | None => String::new(),
                Some(other) => esc_csv(&other.to_string()),
              })
              .collect::<Vec<_>>()
          } else if let Some(arr) = row.as_array() {
            arr
              .iter()
              .map(|v| {
                v.as_str()
                  .map(esc_csv)
                  .unwrap_or_else(|| esc_csv(&v.to_string()))
              })
              .collect::<Vec<_>>()
          } else {
            Vec::new()
          };
          if !cells.is_empty() {
            lines.push(cells.join(","));
          }
        }
      }
    }
    let out_name = if lower_name.ends_with(".csv") {
      safe_base
    } else {
      format!("{kind}.csv")
    };
    return (
      out_name,
      "text/csv".to_string(),
      lines.join("\n").into_bytes(),
    );
  }

  if lower_name.ends_with(".md") || lower_mime == "text/markdown" {
    let mut md = format!("# {title}\n\n");
    if let Some(sections) = content
      .and_then(|c| c.get("sections"))
      .and_then(Value::as_array)
    {
      for sec in sections {
        if let Some(h) = sec
          .get("heading")
          .or_else(|| sec.get("title"))
          .and_then(Value::as_str)
        {
          md.push_str(&format!("## {h}\n\n"));
        }
        if let Some(body) = sec
          .get("body")
          .or_else(|| sec.get("content"))
          .and_then(Value::as_str)
        {
          md.push_str(body);
          md.push_str("\n\n");
        }
      }
    } else if let Some(raw_md) = content
      .and_then(|c| c.get("markdown"))
      .and_then(Value::as_str)
    {
      md.push_str(raw_md);
      md.push('\n');
    } else if let Some(deliv) = deliverable {
      md.push_str("```json\n");
      md.push_str(&serde_json::to_string_pretty(deliv).unwrap_or_default());
      md.push_str("\n```\n");
    }
    let out_name = if lower_name.ends_with(".md") {
      safe_base
    } else {
      format!("{kind}.md")
    };
    return (out_name, "text/markdown".to_string(), md.into_bytes());
  }

  if lower_name.ends_with(".html") || lower_name.ends_with(".htm") || lower_mime == "text/html" {
    let mut body_html = String::new();
    if let Some(sections) = content
      .and_then(|c| c.get("sections"))
      .and_then(Value::as_array)
    {
      for sec in sections {
        let h = sec
          .get("heading")
          .or_else(|| sec.get("title"))
          .and_then(Value::as_str)
          .unwrap_or("Section");
        let b = sec
          .get("body")
          .or_else(|| sec.get("content"))
          .and_then(Value::as_str)
          .unwrap_or_default();
        body_html.push_str(&format!(
          "<section><h2>{}</h2><p>{}</p></section>",
          html_escape_min(h),
          html_escape_min(b).replace('\n', "<br/>")
        ));
      }
    } else if let Some(deliv) = deliverable {
      body_html.push_str(&format!(
        "<pre>{}</pre>",
        html_escape_min(&serde_json::to_string_pretty(deliv).unwrap_or_default())
      ));
    }
    let page = format!(
      "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"/><title>{}</title>\
       <style>body{{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;margin:24px;line-height:1.6}}\
       h1{{font-size:1.35rem;margin-bottom:12px}}h2{{font-size:1.05rem;color:#93c5fd}}pre{{white-space:pre-wrap;background:#1e293b;padding:14px;border-radius:8px}}</style>\
       </head><body><h1>{}</h1>{body_html}</body></html>",
      html_escape_min(title),
      html_escape_min(title)
    );
    let out_name = if lower_name.ends_with(".html") || lower_name.ends_with(".htm") {
      safe_base
    } else {
      format!("{kind}.html")
    };
    return (out_name, "text/html".to_string(), page.into_bytes());
  }

  let json_payload = deliverable
    .cloned()
    .unwrap_or_else(|| json!({ "kind": kind, "title": title, "imported_from": name }));
  let stem = FsPath::new(&safe_base)
    .file_stem()
    .and_then(|s| s.to_str())
    .filter(|s| !s.is_empty())
    .unwrap_or(kind);
  (
    format!("{stem}.json"),
    "application/json".to_string(),
    serde_json::to_vec_pretty(&json_payload).unwrap_or_else(|_| b"{}".to_vec()),
  )
}

/// Render a human-readable Markdown research dossier summarizing a notebook's
/// ingested sources, generated deliverables, and verbatim cited quotes.
pub fn render_notebook_markdown_brief(
  title: &str,
  sources: &[Source],
  chunks: &[SourceChunk],
  artifacts: &[NotebookArtifact],
) -> String {
  render_notebook_markdown_brief_with_messages(title, sources, chunks, artifacts, &[])
}

pub fn render_notebook_markdown_brief_with_messages(
  title: &str,
  sources: &[Source],
  chunks: &[SourceChunk],
  artifacts: &[NotebookArtifact],
  messages: &[NotebookMessage],
) -> String {
  let mut chunk_counts: HashMap<Uuid, usize> = HashMap::new();
  let mut chunk_lookup: HashMap<&str, (Uuid, i32)> = HashMap::with_capacity(chunks.len());
  for c in chunks {
    *chunk_counts.entry(c.source_id).or_insert(0) += 1;
    chunk_lookup.insert(c.chunk_id.as_str(), (c.source_id, c.ordinal));
  }
  let source_titles: HashMap<Uuid, &str> =
    sources.iter().map(|s| (s.id, s.title.as_str())).collect();

  let mut lines = vec![
    format!("# {title}"),
    String::new(),
    format!("## Sources ({})", sources.len()),
  ];
  if sources.is_empty() {
    lines.push("- *(no sources ingested)*".to_string());
  } else {
    for src in sources {
      let count = chunk_counts.get(&src.id).copied().unwrap_or(0);
      let uri_suffix = src
        .uri
        .as_deref()
        .filter(|u| !u.is_empty() && *u != src.title)
        .map(|u| format!(" — `{u}`"))
        .unwrap_or_default();
      lines.push(format!(
        "- **{}** (`{}`, status: `{}`, {} chunks){}",
        src.title, src.kind, src.status, count, uri_suffix
      ));
    }
  }
  lines.push(String::new());

  // Group artifacts by `(run_id, kind)` so multi-file generators (`report.md`,
  // `report.html`, `report.pdf`) present their shared `Deliverable` citations once.
  let mut group_order: Vec<(Uuid, String)> = Vec::new();
  let mut groups: HashMap<(Uuid, String), Vec<&NotebookArtifact>> = HashMap::new();
  for item in artifacts {
    let key = (item.artifact.run_id, item.kind.clone());
    let entry = groups.entry(key.clone()).or_insert_with(|| {
      group_order.push(key);
      Vec::new()
    });
    entry.push(item);
  }

  lines.push(format!("## Deliverables ({})", group_order.len()));
  if group_order.is_empty() {
    lines.push("- *(no deliverables generated)*".to_string());
  } else {
    for key in group_order {
      let Some(items) = groups.get(&key) else {
        continue;
      };
      let first = items[0];
      let deliverable = items.iter().find_map(|a| a.deliverable.as_ref());
      let heading = deliverable
        .and_then(|d| d.get("title"))
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(first.kind.as_str());
      lines.push(String::new());
      lines.push(format!("### {heading} (`{}`)", first.kind));
      let files_list: Vec<String> = items
        .iter()
        .map(|a| format!("`{}`", a.artifact.name))
        .collect();
      lines.push(format!("- Files: {}", files_list.join(", ")));

      if let Some(citations) = deliverable
        .and_then(|d| d.get("citations"))
        .and_then(Value::as_array)
        .filter(|arr| !arr.is_empty())
      {
        lines.push("- Citations:".to_string());
        let mut seen_quotes: HashSet<(String, String)> = HashSet::new();
        for cit in citations {
          let cid = cit
            .get("chunk_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
          let quote = cit
            .get("quote")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
          if cid.is_empty() && quote.is_empty() {
            continue;
          }
          if !seen_quotes.insert((cid.to_string(), quote.to_string())) {
            continue;
          }
          let provenance = if let Some(&(src_id, ordinal)) = chunk_lookup.get(cid) {
            let src_title = source_titles.get(&src_id).copied().unwrap_or("source");
            format!("{src_title} #{ordinal} (`{cid}`)")
          } else {
            format!("`{cid}`")
          };
          if quote.is_empty() {
            lines.push(format!("  - {provenance}"));
          } else {
            lines.push(format!("  - \"{quote}\" — *{provenance}*"));
          }
        }
      }
    }
  }
  if !messages.is_empty() {
    lines.push(String::new());
    lines.push(format!("## Grounded Q&A ({})", messages.len()));
    for (idx, msg) in messages.iter().enumerate() {
      lines.push(String::new());
      lines.push(format!("### Q{}: {}", idx + 1, msg.question));
      lines.push(msg.answer.clone());
    }
  }
  lines.push(String::new());
  lines.join("\n")
}

fn build_notebook_export_bundle(
  notebook: &Notebook,
  sources: &[Source],
  ready_chunks: &[SourceChunk],
  artifacts: &[NotebookArtifact],
  messages: &[NotebookMessage],
) -> NotebookExportBundle {
  let mut chunks_by_source: HashMap<Uuid, Vec<ExportedSourceChunk>> = HashMap::new();
  for chunk in ready_chunks {
    chunks_by_source
      .entry(chunk.source_id)
      .or_default()
      .push(ExportedSourceChunk {
        chunk_id: Some(chunk.chunk_id.clone()),
        ordinal: chunk.ordinal,
        content: chunk.content.clone(),
        metadata: chunk.metadata.clone(),
      });
  }
  for list in chunks_by_source.values_mut() {
    list.sort_by_key(|c| c.ordinal);
  }

  let exported_sources: Vec<ExportedNotebookSource> = sources
    .iter()
    .map(|src| ExportedNotebookSource {
      id: Some(src.id),
      kind: src.kind.clone(),
      title: src.title.clone(),
      uri: src.uri.clone(),
      status: src.status.clone(),
      error: src.error.clone(),
      sha256: src.sha256.clone(),
      raw_text: src.raw_content.clone(),
      chunks: chunks_by_source.remove(&src.id).unwrap_or_default(),
    })
    .collect();

  let exported_deliverables: Vec<ExportedNotebookDeliverable> = artifacts
    .iter()
    .map(|item| ExportedNotebookDeliverable {
      artifact_id: item.artifact.id,
      run_id: item.artifact.run_id,
      kind: item.kind.clone(),
      name: item.artifact.name.clone(),
      mime_type: item.artifact.mime_type.clone(),
      size_bytes: item.artifact.size_bytes,
      sha256: item.artifact.sha256.clone(),
      created_at: item.artifact.created_at,
      deliverable: item.deliverable.clone(),
    })
    .collect();

  let exported_messages: Vec<ExportedNotebookMessage> = messages
    .iter()
    .map(|msg| ExportedNotebookMessage {
      id: Some(msg.id),
      question: msg.question.clone(),
      answer: msg.answer.clone(),
      model: msg.model.clone(),
      citations: msg.citations.clone(),
      used_chunk_count: msg.used_chunk_count,
      created_at: Some(msg.created_at),
    })
    .collect();

  let markdown_brief = render_notebook_markdown_brief_with_messages(
    &notebook.title,
    sources,
    ready_chunks,
    artifacts,
    messages,
  );

  NotebookExportBundle {
    version: NOTEBOOK_BUNDLE_VERSION,
    exported_at: Some(Utc::now()),
    notebook: ExportedNotebookMeta {
      id: Some(notebook.id),
      title: notebook.title.clone(),
      created_at: Some(notebook.created_at),
      updated_at: Some(notebook.updated_at),
    },
    sources: exported_sources,
    deliverables: exported_deliverables,
    messages: exported_messages,
    markdown_brief,
  }
}

/// `GET /v1/notebooks/{id}/export` — export a portable `NotebookExportBundle`
/// containing notebook metadata, sources + ready chunks, deliverable metadata
/// with citations, grounded Q&A turns, and a `markdown_brief` research dossier.
pub async fn export_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<Json<NotebookExportBundle>, ApiError> {
  let notebook = own_notebook(&state, &tenant, id).await?;
  let sources = state
    .repos
    .sources
    .list_for_notebook(tenant.as_str(), id)
    .await?;
  let ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), id)
    .await?;
  let artifacts = state
    .repos
    .notebooks
    .list_artifacts(tenant.as_str(), id)
    .await?;
  let messages = state
    .repos
    .notebook_messages
    .list_for_notebook(tenant.as_str(), id, 200)
    .await?;
  Ok(Json(build_notebook_export_bundle(
    &notebook,
    &sources,
    &ready_chunks,
    &artifacts,
    &messages,
  )))
}

/// `POST /v1/notebooks/import` — import a portable `NotebookExportBundle` into
/// a fresh notebook owned by the caller's tenant, remapping source UUIDs and
/// `<new_source_id>:<ordinal>` chunk IDs so repeated or cross-tenant imports
/// never collide on `source_chunks` primary keys.
pub async fn import_notebook(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  crate::error::JsonReq(req): crate::error::JsonReq<ImportNotebookRequest>,
) -> Result<Json<ImportNotebookResponse>, ApiError> {
  let (bundle, title_override) = match req {
    ImportNotebookRequest::Wrapped { bundle, title } => (bundle, title),
    ImportNotebookRequest::Direct(bundle) => (bundle, None),
  };
  let validated = validate_notebook_import_bundle(&bundle, title_override.as_deref())?;

  let notebook = state
    .repos
    .notebooks
    .create(tenant.as_str(), &validated.title)
    .await?;

  let mut imported_sources = 0usize;
  let mut imported_chunks = 0usize;
  let mut source_id_map: HashMap<Uuid, Uuid> = HashMap::new();
  let mut chunk_id_map: HashMap<String, String> = HashMap::new();

  for src in validated.sources {
    let original_id = src.original_id;
    let raw_text_opt = src.raw_text;
    let Some(created_source) = state
      .repos
      .sources
      .create(NewSource {
        notebook_id: notebook.id,
        tenant_id: tenant.as_str().to_string(),
        kind: src.kind,
        title: src.title,
        uri: src.uri,
      })
      .await?
    else {
      continue;
    };
    if let Some(old_id) = original_id {
      source_id_map.insert(old_id, created_source.id);
    }
    if let Some(raw_text) = &raw_text_opt {
      state
        .repos
        .sources
        .set_raw_content(tenant.as_str(), notebook.id, created_source.id, raw_text)
        .await?;
    }
    imported_sources += 1;

    if !src.chunks.is_empty() {
      let digest = src.sha256.unwrap_or_else(|| {
        let mut hasher = Sha256::new();
        for c in &src.chunks {
          hasher.update(c.content.as_bytes());
          hasher.update(b"\n");
        }
        format!("{:x}", hasher.finalize())
      });
      for (new_ordinal, c) in src.chunks.iter().enumerate() {
        let new_cid = format!("{}:{new_ordinal}", created_source.id);
        if let Some(old_cid) = &c.old_chunk_id {
          chunk_id_map.insert(old_cid.clone(), new_cid.clone());
        }
        if let Some(old_src) = original_id {
          chunk_id_map.insert(format!("{old_src}:{}", c.old_ordinal), new_cid);
        }
      }
      let chunk_pairs: Vec<(String, Value)> = src
        .chunks
        .into_iter()
        .map(|c| (c.content, c.metadata))
        .collect();
      let stored = state
        .repos
        .chunks
        .replace_for_source(&created_source, chunk_pairs)
        .await?;
      imported_chunks += stored;
      state
        .repos
        .sources
        .update_status(created_source.id, SourceStatus::Ready, None, Some(&digest))
        .await?;
    } else {
      let reason = src.error.unwrap_or_else(|| {
        if src.status == "failed" {
          "imported failed source".to_string()
        } else {
          "imported source had no text chunks".to_string()
        }
      });
      state
        .repos
        .sources
        .update_status(created_source.id, SourceStatus::Failed, Some(&reason), None)
        .await?;
    }
  }

  let mut imported_messages = 0usize;
  for msg in validated.messages {
    let citations = remap_imported_json_refs(&msg.citations, &source_id_map, &chunk_id_map);
    if state
      .repos
      .notebook_messages
      .append(NewNotebookMessage {
        notebook_id: notebook.id,
        tenant_id: tenant.as_str().to_string(),
        question: msg.question,
        answer: msg.answer,
        model: msg.model,
        citations,
        used_chunk_count: msg.used_chunk_count,
      })
      .await?
      .is_some()
    {
      imported_messages += 1;
    }
  }

  // Restore portable deliverables (C2): synthesize viewable local artifacts per
  // imported deliverable, skipping redundant binary sibling entries when the
  // same `old_run_id` already has a synthesizable text/HTML/CSV representation.
  let runs_with_text_sibling: HashSet<Uuid> = validated
    .deliverables
    .iter()
    .filter(|d| is_synthesizable_text_deliverable(&d.name, d.mime_type.as_deref()))
    .map(|d| d.old_run_id)
    .collect();
  let mut imported_run_map: HashMap<Uuid, (Uuid, std::path::PathBuf)> = HashMap::new();
  let mut fallback_emitted_runs: HashSet<Uuid> = HashSet::new();
  let mut imported_deliverables = 0usize;

  for (idx, deliv) in validated.deliverables.into_iter().enumerate() {
    let is_text = is_synthesizable_text_deliverable(&deliv.name, deliv.mime_type.as_deref());
    if !is_text
      && (runs_with_text_sibling.contains(&deliv.old_run_id)
        || !fallback_emitted_runs.insert(deliv.old_run_id))
    {
      continue;
    }

    let (new_run_id, run_dir) = if let Some(existing) = imported_run_map.get(&deliv.old_run_id) {
      existing.clone()
    } else {
      let fresh_run_id = Uuid::new_v4();
      let dir = crate::runs::default_run_dir_for_id(fresh_run_id);
      state
        .repos
        .runs
        .create(yanshi_db::NewRun {
          id: fresh_run_id,
          workflow: format!("@imported:{}", deliv.kind),
          status: yanshi_db::RunStatus::Succeeded,
          run_dir: Some(dir.display().to_string()),
          tenant_id: tenant.as_str().to_string(),
          events_retention_days: None,
          artifacts_retention_days: None,
        })
        .await?;
      state
        .repos
        .runs
        .set_resume_context(
          fresh_run_id,
          None,
          None,
          Some(notebook.id),
          Some(&deliv.kind),
        )
        .await?;
      imported_run_map.insert(deliv.old_run_id, (fresh_run_id, dir.clone()));
      (fresh_run_id, dir)
    };

    let remapped_deliverable = deliv
      .deliverable
      .as_ref()
      .map(|val| remap_imported_json_refs(val, &source_id_map, &chunk_id_map));
    let (file_name, mime_type, file_bytes) = synthesize_imported_deliverable_bytes(
      &deliv.kind,
      &deliv.name,
      deliv.mime_type.as_deref(),
      remapped_deliverable.as_ref(),
    );
    let node_dir = run_dir.join("artifacts").join("imported");
    tokio::fs::create_dir_all(&node_dir)
      .await
      .map_err(|e| ApiError::Internal(format!("creating imported artifact dir: {e}")))?;
    let disk_name = format!("{}_{}", idx + 1, file_name);
    let file_path = node_dir.join(&disk_name);
    tokio::fs::write(&file_path, &file_bytes)
      .await
      .map_err(|e| ApiError::Internal(format!("writing imported deliverable file: {e}")))?;
    let sha = format!("{:x}", Sha256::digest(&file_bytes));
    let created_artifact = state
      .repos
      .artifacts
      .create(yanshi_db::NewArtifact {
        id: Uuid::new_v4(),
        run_id: new_run_id,
        node_id: "imported".to_string(),
        name: file_name,
        path_or_url: file_path.display().to_string(),
        mime_type: Some(mime_type),
        tenant_id: Some(tenant.as_str().to_string()),
        size_bytes: Some(file_bytes.len() as i64),
        sha256: Some(sha),
      })
      .await?;
    if state
      .repos
      .notebooks
      .attach_artifact_with_deliverable(
        tenant.as_str(),
        notebook.id,
        created_artifact.id,
        &deliv.kind,
        remapped_deliverable.as_ref(),
      )
      .await?
    {
      imported_deliverables += 1;
    }
  }

  state.notebook_indexes.invalidate(notebook.id);
  Ok(Json(ImportNotebookResponse {
    notebook,
    imported_sources,
    imported_chunks,
    imported_messages,
    imported_deliverables,
  }))
}

fn pack_notebook_artifacts_zip_blocking(
  notebook_id: Uuid,
  notebook_title: String,
  brief_markdown: String,
  artifacts: Vec<NotebookArtifact>,
) -> std::io::Result<Option<(std::fs::File, u64)>> {
  use std::io::{Read, Seek, SeekFrom, Write};

  let tmp = tempfile::tempfile()?;
  let mut writer = zip::ZipWriter::new(tmp);
  let options =
    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

  let mut seen_paths = HashSet::new();
  seen_paths.insert("BRIEFING.md".to_string());
  seen_paths.insert("notebook_artifacts.json".to_string());

  let mut manifest_items = Vec::new();
  let mut buf = vec![0u8; 64 * 1024];

  for item in artifacts {
    let disk_path = PathBuf::from(&item.artifact.path_or_url);
    let mut file = match std::fs::File::open(&disk_path) {
      Ok(f) => f,
      Err(_) => continue,
    };
    let safe_kind = crate::artifacts::sanitize_zip_segment(&item.kind, "deliverable");
    let safe_name = crate::artifacts::sanitize_zip_segment(&item.artifact.name, "output.bin");
    let entry_path =
      crate::artifacts::deduplicate_entry_path(&mut seen_paths, &safe_kind, &safe_name);

    writer
      .start_file(&entry_path, options)
      .map_err(std::io::Error::other)?;
    let mut hasher = Sha256::new();
    let mut size_bytes: u64 = 0;
    loop {
      let n = file.read(&mut buf)?;
      if n == 0 {
        break;
      }
      writer.write_all(&buf[..n])?;
      hasher.update(&buf[..n]);
      size_bytes += n as u64;
    }
    let sha256 = format!("{:x}", hasher.finalize());
    manifest_items.push(json!({
      "artifact_id": item.artifact.id,
      "run_id": item.artifact.run_id,
      "kind": item.kind,
      "name": safe_name,
      "entry_path": entry_path,
      "mime_type": item.artifact.mime_type,
      "size_bytes": size_bytes,
      "sha256": sha256,
      "deliverable": item.deliverable,
    }));
  }

  if manifest_items.is_empty() {
    return Ok(None);
  }

  writer
    .start_file("BRIEFING.md", options)
    .map_err(std::io::Error::other)?;
  writer.write_all(brief_markdown.as_bytes())?;

  let manifest = json!({
    "notebook_id": notebook_id,
    "title": notebook_title,
    "artifacts": manifest_items,
  });
  let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(std::io::Error::other)?;
  writer
    .start_file("notebook_artifacts.json", options)
    .map_err(std::io::Error::other)?;
  writer.write_all(&manifest_bytes)?;

  let mut finished = writer.finish().map_err(std::io::Error::other)?;
  let total_len = finished.seek(SeekFrom::End(0))?;
  finished.seek(SeekFrom::Start(0))?;
  Ok(Some((finished, total_len)))
}

/// `GET /v1/notebooks/{id}/artifacts.zip` — stream all deliverables attached to
/// a notebook as a single `.zip` bundle (`<kind>/<file>` + `BRIEFING.md` +
/// `notebook_artifacts.json`).
pub async fn download_notebook_artifacts_zip(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
  let notebook = own_notebook(&state, &tenant, id).await?;
  let artifacts = state
    .repos
    .notebooks
    .list_artifacts(tenant.as_str(), id)
    .await?;
  if artifacts.is_empty() {
    return Err(ApiError::NotFound(format!(
      "notebook {id} has no downloadable artifacts"
    )));
  }
  let sources = state
    .repos
    .sources
    .list_for_notebook(tenant.as_str(), id)
    .await?;
  let ready_chunks = state
    .repos
    .chunks
    .list_ready_for_notebook(tenant.as_str(), id)
    .await?;
  let messages = state
    .repos
    .notebook_messages
    .list_for_notebook(tenant.as_str(), id, 200)
    .await?;
  let brief_md = render_notebook_markdown_brief_with_messages(
    &notebook.title,
    &sources,
    &ready_chunks,
    &artifacts,
    &messages,
  );
  let title = notebook.title.clone();

  let packed = tokio::task::spawn_blocking(move || {
    pack_notebook_artifacts_zip_blocking(id, title, brief_md, artifacts)
  })
  .await
  .map_err(|e| ApiError::Internal(format!("notebook zip task failed: {e}")))?
  .map_err(|e| ApiError::Internal(format!("failed to build notebook zip: {e}")))?;

  let Some((std_file, total_len)) = packed else {
    return Err(ApiError::NotFound(format!(
      "notebook {id} has no readable artifact files on disk"
    )));
  };

  let async_file = tokio::fs::File::from_std(std_file);
  let short_id = &id.to_string()[..8];
  let disposition = format!("attachment; filename=\"yanshi-notebook-{short_id}-artifacts.zip\"");

  let mut response =
    Body::from_stream(tokio_util::io::ReaderStream::new(async_file)).into_response();
  let headers = response.headers_mut();
  headers.insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/zip"),
  );
  headers.insert(header::CONTENT_LENGTH, HeaderValue::from(total_len));
  if let Ok(val) = HeaderValue::from_str(&disposition) {
    headers.insert(header::CONTENT_DISPOSITION, val);
  }
  Ok(response)
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

  #[test]
  fn notebook_bundle_brief_validation_and_zip_archive_round_trip() {
    use std::io::Read as _;

    let dir = tempfile::tempdir().expect("tempdir");
    let md_path = dir.path().join("1.md");
    let html_path = dir.path().join("1.html");
    std::fs::write(&md_path, b"# Tides Report\n").expect("write md");
    std::fs::write(&html_path, b"<h1>Tides Report</h1>\n").expect("write html");

    let now = Utc::now();
    let notebook_id = Uuid::new_v4();
    let source_id = Uuid::new_v4();
    let run_id = Uuid::new_v4();
    let chunk_id = format!("{source_id}:0");

    let notebook = Notebook {
      id: notebook_id,
      tenant_id: "t1".to_string(),
      title: "Estuarine Dynamics".to_string(),
      created_at: now,
      updated_at: now,
    };
    let sources = vec![Source {
      id: source_id,
      notebook_id,
      tenant_id: "t1".to_string(),
      kind: "file".to_string(),
      title: "severn_bore.docx".to_string(),
      uri: Some("severn_bore.docx".to_string()),
      status: "ready".to_string(),
      error: None,
      sha256: Some("abc123".to_string()),
      raw_content: None,
      created_at: now,
      updated_at: now,
    }];
    let chunks = vec![SourceChunk {
      chunk_id: chunk_id.clone(),
      source_id,
      notebook_id,
      tenant_id: "t1".to_string(),
      ordinal: 0,
      content: "Severn Bore reaches 15 meters during equinoctial spring tides.".to_string(),
      metadata: json!({"start_line": 1, "end_line": 2}),
    }];
    let deliverable = json!({
      "kind": "report",
      "title": "Severn Bore Brief",
      "citations": [
        {"source_id": source_id, "chunk_id": chunk_id, "quote": "Severn Bore reaches 15 meters"}
      ]
    });
    let artifacts = vec![
      NotebookArtifact {
        kind: "report".to_string(),
        deliverable: Some(deliverable.clone()),
        artifact: yanshi_db::Artifact {
          id: Uuid::new_v4(),
          run_id,
          node_id: "report_md".to_string(),
          name: "1.md".to_string(),
          path_or_url: md_path.to_string_lossy().to_string(),
          mime_type: Some("text/markdown".to_string()),
          tenant_id: "t1".to_string(),
          size_bytes: Some(15),
          sha256: None,
          created_at: now,
        },
      },
      NotebookArtifact {
        kind: "report".to_string(),
        deliverable: Some(deliverable),
        artifact: yanshi_db::Artifact {
          id: Uuid::new_v4(),
          run_id,
          node_id: "report_html".to_string(),
          name: "1.html".to_string(),
          path_or_url: html_path.to_string_lossy().to_string(),
          mime_type: Some("text/html".to_string()),
          tenant_id: "t1".to_string(),
          size_bytes: Some(22),
          sha256: None,
          created_at: now,
        },
      },
    ];

    let bundle = build_notebook_export_bundle(&notebook, &sources, &chunks, &artifacts, &[]);
    assert_eq!(bundle.version, NOTEBOOK_BUNDLE_VERSION);
    assert!(
      bundle.markdown_brief.contains("# Estuarine Dynamics")
        && bundle
          .markdown_brief
          .contains("### Severn Bore Brief (`report`)")
        && bundle
          .markdown_brief
          .contains("Severn Bore reaches 15 meters")
        && bundle.markdown_brief.contains("severn_bore.docx #0"),
      "unexpected markdown brief:\n{}",
      bundle.markdown_brief
    );
    assert_eq!(
      bundle
        .markdown_brief
        .matches("### Severn Bore Brief (`report`)")
        .count(),
      1,
      "multi-file deliverables from the same run must be grouped once"
    );

    // Validation accepts direct + overridden titles and rejects bad versions / kinds.
    let validated =
      validate_notebook_import_bundle(&bundle, Some("  Cloned Estuaries  ")).expect("valid bundle");
    assert_eq!(validated.title, "Cloned Estuaries");
    assert_eq!(validated.sources.len(), 1);
    assert_eq!(validated.sources[0].chunks.len(), 1);

    let mut bad_ver = bundle.clone();
    bad_ver.version = 99;
    assert!(matches!(
      validate_notebook_import_bundle(&bad_ver, None),
      Err(ImportValidationError::BadRequest(_))
    ));

    let mut bad_kind = bundle.clone();
    bad_kind.sources[0].kind = "s3_bucket".to_string();
    assert!(matches!(
      validate_notebook_import_bundle(&bad_kind, None),
      Err(ImportValidationError::BadRequest(_))
    ));

    // `.zip` packaging bundles `<kind>/<name>` + `BRIEFING.md` + `notebook_artifacts.json`.
    let (zip_file, total_len) = pack_notebook_artifacts_zip_blocking(
      notebook_id,
      notebook.title.clone(),
      bundle.markdown_brief.clone(),
      artifacts,
    )
    .expect("zip io")
    .expect("non-empty zip");
    assert!(total_len > 0);
    let mut archive = zip::ZipArchive::new(zip_file).expect("open zip");
    assert_eq!(archive.len(), 4);
    let mut brief_out = String::new();
    archive
      .by_name("BRIEFING.md")
      .expect("BRIEFING.md")
      .read_to_string(&mut brief_out)
      .expect("read brief");
    assert!(brief_out.contains("Severn Bore Brief"));
    assert!(archive.by_name("report/1.md").is_ok());
    assert!(archive.by_name("report/1.html").is_ok());
    assert!(archive.by_name("notebook_artifacts.json").is_ok());
  }

  #[test]
  fn sanitize_studio_plan_reply_coerces_valid_params_and_drops_hallucinations() {
    let policy = crate::gateway_policy::GatewayPolicy::default();
    let catalogue = build_generator_catalogue_prompt();
    assert!(catalogue.contains("`report`: params ["));
    assert!(catalogue.contains("`flashcards`: params ["));
    assert!(!catalogue.contains("chunks="));

    let raw = r#"<think>pick study pack</think>
```json
{
  "rationale": "Combine an executive summary report with flashcards and a quiz.",
  "items": [
    {
      "kind": "report",
      "reason": "Structured synthesis with citations",
      "parameters": [
        { "key": "focus", "value": "spring tide energetics" },
        { "key": "sections", "value": "3" },
        { "key": "pdf", "value": "false" },
        { "key": "chunks", "value": "forbidden" },
        { "key": "invented_param", "value": "42" }
      ]
    },
    {
      "kind": "hallucinated_podcast_v9",
      "reason": "Should be dropped",
      "parameters": []
    },
    {
      "kind": "report",
      "reason": "Duplicate kind should be deduplicated",
      "parameters": [{ "key": "sections", "value": "5" }]
    },
    {
      "kind": "flashcards",
      "reason": "Active recall",
      "params": {
        "count": 8,
        "focus": "key tidal definitions"
      }
    },
    {
      "kind": "quiz",
      "reason": "Self-check",
      "parameters": [{ "key": "count", "value": "5" }]
    }
  ]
}
```"#;

    let (rationale, items) = sanitize_studio_plan_reply(raw, 2, &policy).expect("valid plan");
    assert!(rationale.contains("executive summary report"));
    assert_eq!(
      items.len(),
      2,
      "must deduplicate, drop unknown kinds, and cap to max_items=2"
    );
    assert_eq!(items[0].kind, "report");
    assert_eq!(
      items[0].params.get("focus"),
      Some(&json!("spring tide energetics"))
    );
    assert_eq!(items[0].params.get("sections"), Some(&json!(3)));
    assert_eq!(items[0].params.get("pdf"), Some(&json!(false)));
    assert!(!items[0].params.contains_key("chunks"));
    assert!(!items[0].params.contains_key("invented_param"));

    assert_eq!(items[1].kind, "flashcards");
    assert_eq!(items[1].params.get("count"), Some(&json!(8)));
    assert_eq!(
      items[1].params.get("focus"),
      Some(&json!("key tidal definitions"))
    );

    // A plan whose items are all hallucinated must fail loudly rather than returning an empty plan.
    let empty_valid =
      r#"{"rationale":"bad","items":[{"kind":"nonexistent","reason":"x","parameters":[]}]}"#;
    assert!(sanitize_studio_plan_reply(empty_valid, 4, &policy).is_err());

    // Batch cost-limit splitting divides evenly and rejects <= 0 / NaN.
    assert_eq!(split_batch_cost_limit(None, 3).expect("none"), None);
    assert_eq!(
      split_batch_cost_limit(Some(6.0), 3).expect("6/3"),
      Some(2.0)
    );
    assert!(split_batch_cost_limit(Some(0.0), 2).is_err());
    assert!(split_batch_cost_limit(Some(-3.0), 2).is_err());
  }

  #[test]
  fn smart_url_classifier_and_citation_remap_handle_arxiv_github_and_imports() {
    assert_eq!(
      classify_smart_source_url("https://arxiv.org/abs/1706.03762v7"),
      SmartUrlTarget::RewriteUrl("https://arxiv.org/pdf/1706.03762v7.pdf".to_string())
    );
    assert_eq!(
      classify_smart_source_url("https://github.com/tokio-rs/axum.git"),
      SmartUrlTarget::GitHubRepo {
        owner: "tokio-rs".to_string(),
        repo: "axum".to_string(),
        git_ref: None,
        subpath: None,
      }
    );
    assert_eq!(
      classify_smart_source_url("https://github.com/tokio-rs/axum/tree/main/axum/src"),
      SmartUrlTarget::GitHubRepo {
        owner: "tokio-rs".to_string(),
        repo: "axum".to_string(),
        git_ref: Some("main".to_string()),
        subpath: Some("axum/src".to_string()),
      }
    );
    assert_eq!(
      classify_smart_source_url("https://github.com/tokio-rs/axum/blob/main/README.md"),
      SmartUrlTarget::RewriteUrl(
        "https://raw.githubusercontent.com/tokio-rs/axum/main/README.md".to_string()
      )
    );
    assert_eq!(
      classify_smart_source_url("https://example.com/docs/guide.html"),
      SmartUrlTarget::Direct
    );

    let chosen = select_github_snapshot_paths(
      &[
        "node_modules/pkg/index.js".to_string(),
        "Cargo.lock".to_string(),
        "src/lib.rs".to_string(),
        "Cargo.toml".to_string(),
        "README.md".to_string(),
        "assets/logo.png".to_string(),
      ],
      None,
      10,
    );
    assert_eq!(chosen, vec!["README.md", "Cargo.toml", "src/lib.rs"]);

    let old_src = Uuid::new_v4();
    let new_src = Uuid::new_v4();
    let remapped = remap_imported_citations(
      &json!([{
        "source_id": old_src.to_string(),
        "chunk_id": format!("{old_src}:2"),
        "quote": "verbatim passage"
      }]),
      &HashMap::from([(old_src, new_src)]),
    );
    assert_eq!(remapped[0]["source_id"], new_src.to_string());
    assert_eq!(remapped[0]["chunk_id"], format!("{new_src}:2"));

    assert_eq!(
      classify_smart_source_url("https://arxiv.org/html/2401.12345v2"),
      SmartUrlTarget::RewriteUrl("https://arxiv.org/pdf/2401.12345v2.pdf".to_string())
    );
    assert_eq!(
      classify_smart_source_url("https://openreview.net/forum?id=AbCdEf123"),
      SmartUrlTarget::RewriteUrl("https://openreview.net/pdf?id=AbCdEf123".to_string())
    );

    let sub_extracted = subtitle_text(
      b"WEBVTT\n\n1\n00:00:01.000 --> 00:00:04.500\n<i>Spring</i> tides peak at syzygy.\n\n2\n00:00:05.000 --> 00:00:08.000\nNeap tides follow quadrature.\n",
    )
    .expect("vtt parses");
    assert!(sub_extracted.contains("[00:00:01.000 -> 00:00:04.500] Spring tides peak at syzygy."));
    assert!(sub_extracted.contains("[00:00:05.000 -> 00:00:08.000] Neap tides follow quadrature."));

    let chunk_map = HashMap::from([(format!("{old_src}:9"), format!("{new_src}:0"))]);
    let nested = json!({
      "kind": "flashcards",
      "content": { "items": [{ "front": "Q", "back": "A", "chunk_id": format!("{old_src}:9") }] },
      "citations": [{ "source_id": old_src.to_string(), "chunk_id": format!("{old_src}:9"), "quote": "q" }]
    });
    let remapped_nested =
      remap_imported_json_refs(&nested, &HashMap::from([(old_src, new_src)]), &chunk_map);
    assert_eq!(
      remapped_nested["content"]["items"][0]["chunk_id"],
      format!("{new_src}:0")
    );
    assert_eq!(
      remapped_nested["citations"][0]["chunk_id"],
      format!("{new_src}:0")
    );
  }
}
