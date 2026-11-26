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
use std::path::Path as FsPath;
use std::sync::{Arc, Mutex};

use axum::{
  Extension, Json,
  body::Body,
  extract::{Path, Query, State},
  http::{HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use yanshi_db::{
  ArtifactRepo, EventRepo, NewNotebookMessage, NewSource, Notebook, NotebookArtifact,
  NotebookMessage, NotebookMessageRepo, NotebookRepo, RunRepo, Source, SourceChunk,
  SourceChunkRepo, SourceKind, SourceRepo, SourceStatus,
};
pub(crate) use yanshi_notebook::bundle::remap_imported_json_refs;
use yanshi_notebook::bundle::{
  BriefArtifact, BriefChunk, BriefMessage, BriefSource, ImportValidationError, NotebookSourceKind,
  ZipArtifactItem, is_synthesizable_text_deliverable,
  pack_notebook_artifacts_zip_blocking as pack_shared_notebook_artifacts_zip_blocking,
  render_notebook_markdown_brief as render_shared_notebook_markdown_brief,
  synthesize_imported_deliverable_bytes, validate_notebook_import_bundle,
};
pub use yanshi_notebook::bundle::{
  ExportedNotebookDeliverable, ExportedNotebookMessage, ExportedNotebookMeta,
  ExportedNotebookSource, ExportedSourceChunk, ImportNotebookRequest, NOTEBOOK_BUNDLE_VERSION,
  NotebookExportBundle,
};
pub use yanshi_notebook::ingest::AddSourceRequest;
use yanshi_notebook::ingest::{
  BytesFormat, MAX_SOURCE_BYTES, chunk_paragraphs as chunk, extract_bytes_text, ingest_url,
  parse_source_ids_csv as parse_source_ids_csv_raw, sniff_image_canonical_name,
  transcribe_recording,
};
#[cfg(test)]
use yanshi_notebook::ingest::{
  MAX_OOXML_XML_BYTES, SmartUrlTarget, classify_smart_source_url, delimited_text, docx_text,
  html_text, json_text, pdf_text, pptx_text, select_github_snapshot_paths, sniff_ooxml_zip_format,
  subtitle_text, xlsx_text,
};
pub use yanshi_notebook::qa::{
  AskNotebookRequest, AskNotebookResponse, GroundedAnswerCitation, ListMessagesQuery,
  MAX_ASK_HISTORY_TURNS, NotebookConversationTurn, SearchHit, SearchQuery,
};
use yanshi_notebook::qa::{
  build_ask_retrieval_hint, format_ask_history_block, grounded_answer_schema,
  normalize_ask_history, parse_grounded_answer_reply,
  resolve_notebook_model as resolve_notebook_model_raw, truncate_chars,
};
#[cfg(test)]
use yanshi_notebook::studio::GENERATORS;
pub use yanshi_notebook::studio::{
  BatchGenerateItem, DispatchedNotebookRun, GenerateBatchRequest, GenerateBatchResponse,
  GenerateRequest, MAX_BATCH_GENERATE_ITEMS, PlanNotebookRequest, PlanNotebookResponse,
  PlannedGeneratorItem,
};
use yanshi_notebook::studio::{
  DEFAULT_PLAN_MAX_ITEMS, GENERATE_CHUNK_BUDGET, build_generator_catalogue_prompt,
  find_generator_template as find_generator_template_raw,
  sanitize_studio_plan_reply as sanitize_studio_plan_reply_raw, select_chunks_by_budget,
  split_batch_cost_limit as split_batch_cost_limit_raw, studio_plan_schema,
};
use yanshi_rag::{Bm25KnowledgeBackend, KnowledgeBackend};

use crate::{AppState, error::ApiError, tenant::TenantId};

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
        "png" | "jpg" | "jpeg" | "webp" | "gif" => BytesFormat::Image,
        "docx" => BytesFormat::Docx,
        "xlsx" => BytesFormat::Xlsx,
        "pptx" => BytesFormat::Pptx,
        "csv" => BytesFormat::Csv,
        "tsv" => BytesFormat::Tsv,
        "json" => BytesFormat::Json,
        "srt" | "vtt" => BytesFormat::Subtitle,
        other => {
          return Err(ApiError::BadRequest(format!(
            "unsupported file type '.{other}': upload .txt, .md, .html, .pdf, .png, .jpg, .jpeg, .webp, .gif, .docx, .xlsx, .pptx, .csv, .tsv, .json, .srt or .vtt"
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
      if format == BytesFormat::Image && sniff_image_canonical_name(&bytes).is_none() {
        return Err(ApiError::BadRequest(format!(
          "file is not a valid .{} image (expected PNG, JPEG, GIF or WebP header)",
          extension
        )));
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

fn parse_source_ids_csv(raw: Option<&str>) -> Result<Option<Vec<Uuid>>, ApiError> {
  parse_source_ids_csv_raw(raw).map_err(ApiError::BadRequest)
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

fn resolve_notebook_model(explicit: Option<&str>) -> Result<String, ApiError> {
  resolve_notebook_model_raw(explicit).map_err(ApiError::BadRequest)
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

struct PreparedGeneratorItem {
  kind: String,
  template: &'static str,
  inputs: serde_json::Map<String, Value>,
}

fn find_generator_template(kind: &str) -> Result<&'static str, ApiError> {
  find_generator_template_raw(kind).map_err(ApiError::BadRequest)
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
  split_batch_cost_limit_raw(total_limit_usd, item_count).map_err(ApiError::BadRequest)
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

fn sanitize_studio_plan_reply(
  raw_reply: &str,
  max_items: usize,
  gateway_policy: &crate::gateway_policy::GatewayPolicy,
) -> Result<(String, Vec<PlannedGeneratorItem>), String> {
  sanitize_studio_plan_reply_raw(raw_reply, max_items, |definition, probe| {
    gateway_policy.check_run_inputs(definition, probe).is_ok()
  })
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
  select_chunks_by_budget(
    ready,
    query_hint,
    budget,
    |c| c.chunk_id.as_str(),
    |c| c.source_id,
    |c| c.content.as_str(),
    |c, cut| {
      c.content.truncate(cut);
      !c.content.is_empty()
    },
  )
  .await
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

  // 2. Fallback: scan persisted `node.output.captured` events for `deliverable`.
  if let Ok(events) = state
    .repos
    .events
    .list_after(tenant_id, run.id, -1, 5000)
    .await
  {
    for ev in events.into_iter().rev() {
      if (ev.kind == "node.output.captured" || ev.kind == "node_output_captured")
        && ev.payload.get("node_id").and_then(Value::as_str) == Some("deliverable")
        && let Some(output) = ev.payload.get("output")
        && let Some(deliv) = unwrap_captured_deliverable_output(output)
      {
        return Some(deliv);
      }
    }
  }
  None
}

fn unwrap_flow_json_envelope(val: &Value) -> &Value {
  if val.get("type").and_then(Value::as_str) == Some("json")
    && let Some(inner) = val.get("value")
  {
    return inner;
  }
  val
}

fn unwrap_captured_deliverable_output(output: &Value) -> Option<Value> {
  if let Some(raw_deliv) = output.get("deliverable") {
    let candidate = unwrap_flow_json_envelope(raw_deliv);
    if candidate.get("kind").is_some() && candidate.get("title").is_some() {
      return Some(candidate.clone());
    }
  }
  if let Some(raw_out) = output.get("output") {
    let inner_out = unwrap_flow_json_envelope(raw_out);
    if let Some(nested) = inner_out.get("deliverable") {
      let candidate = unwrap_flow_json_envelope(nested);
      if candidate.get("kind").is_some() && candidate.get("title").is_some() {
        return Some(candidate.clone());
      }
    }
    if inner_out.get("kind").is_some() && inner_out.get("title").is_some() {
      return Some(inner_out.clone());
    }
  }
  let direct = unwrap_flow_json_envelope(output);
  if direct.get("kind").is_some() && direct.get("title").is_some() {
    return Some(direct.clone());
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

#[derive(Debug, Serialize)]
pub struct ImportNotebookResponse {
  pub notebook: Notebook,
  pub imported_sources: usize,
  pub imported_chunks: usize,
  pub imported_messages: usize,
  pub imported_deliverables: usize,
}

fn import_validation_err_to_api(err: ImportValidationError) -> ApiError {
  match err {
    ImportValidationError::BadRequest(msg) => ApiError::BadRequest(msg),
    ImportValidationError::PayloadTooLarge(msg) => ApiError::PayloadTooLarge(msg),
  }
}

fn db_source_kind(kind: NotebookSourceKind) -> SourceKind {
  match kind {
    NotebookSourceKind::Url => SourceKind::Url,
    NotebookSourceKind::File => SourceKind::File,
    NotebookSourceKind::Text => SourceKind::Text,
    NotebookSourceKind::Recording => SourceKind::Recording,
  }
}

#[cfg(test)]
pub(crate) fn remap_imported_citations(raw: &Value, source_id_map: &HashMap<Uuid, Uuid>) -> Value {
  remap_imported_json_refs(raw, source_id_map, &HashMap::new())
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
  let brief_sources: Vec<BriefSource<'_>> = sources
    .iter()
    .map(|s| BriefSource {
      id: s.id,
      title: s.title.as_str(),
      kind: s.kind.as_str(),
      status: s.status.as_str(),
      uri: s.uri.as_deref(),
    })
    .collect();
  let brief_chunks: Vec<BriefChunk<'_>> = chunks
    .iter()
    .map(|c| BriefChunk {
      source_id: c.source_id,
      chunk_id: c.chunk_id.as_str(),
      ordinal: c.ordinal,
    })
    .collect();
  let brief_artifacts: Vec<BriefArtifact<'_>> = artifacts
    .iter()
    .map(|a| BriefArtifact {
      run_id: a.artifact.run_id,
      kind: a.kind.as_str(),
      name: a.artifact.name.as_str(),
      deliverable: a.deliverable.as_ref(),
    })
    .collect();
  let brief_messages: Vec<BriefMessage<'_>> = messages
    .iter()
    .map(|m| BriefMessage {
      question: m.question.as_str(),
      answer: m.answer.as_str(),
    })
    .collect();
  render_shared_notebook_markdown_brief(
    title,
    &brief_sources,
    &brief_chunks,
    &brief_artifacts,
    &brief_messages,
  )
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
  let validated = validate_notebook_import_bundle(&bundle, title_override.as_deref())
    .map_err(import_validation_err_to_api)?;

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
        kind: db_source_kind(src.kind),
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
  let items: Vec<ZipArtifactItem> = artifacts
    .into_iter()
    .map(|item| ZipArtifactItem {
      artifact_id: item.artifact.id,
      run_id: item.artifact.run_id,
      kind: item.kind,
      name: item.artifact.name,
      path_or_url: item.artifact.path_or_url,
      mime_type: item.artifact.mime_type,
      deliverable: item.deliverable,
    })
    .collect();
  pack_shared_notebook_artifacts_zip_blocking(notebook_id, notebook_title, brief_markdown, items)
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

    let flow_wrapped = json!({
      "deliverable": {
        "type": "json",
        "value": { "kind": "slides", "title": "Deck", "files": [] }
      }
    });
    let unwrapped = unwrap_captured_deliverable_output(&flow_wrapped).expect("unwrapped");
    assert_eq!(unwrapped["kind"], "slides");
    assert_eq!(unwrapped["title"], "Deck");

    let bundle_with_slash_kind: NotebookExportBundle = serde_json::from_value(json!({
      "version": 1,
      "notebook": { "title": "Slash Kind Import" },
      "sources": [],
      "deliverables": [{
        "artifact_id": Uuid::new_v4(),
        "run_id": Uuid::new_v4(),
        "kind": "custom/report",
        "name": "out.bin",
        "mime_type": "text/html",
        "created_at": chrono::Utc::now()
      }],
      "messages": []
    }))
    .expect("bundle json");
    let validated =
      validate_notebook_import_bundle(&bundle_with_slash_kind, None).expect("validates");
    assert!(!validated.deliverables[0].kind.contains('/'));
    let (synth_name, _, _) = synthesize_imported_deliverable_bytes(
      &validated.deliverables[0].kind,
      &validated.deliverables[0].name,
      validated.deliverables[0].mime_type.as_deref(),
      None,
    );
    assert!(!synth_name.contains('/'));
  }

  #[tokio::test]
  async fn sniffs_image_headers_and_extracts_image_text_via_deepseek_ocr() {
    assert_eq!(
      sniff_image_canonical_name(b"\x89PNG\r\n\x1a\npayload"),
      Some("image.png")
    );
    assert_eq!(
      sniff_image_canonical_name(b"\xff\xd8\xff\xe0jpeg"),
      Some("image.jpg")
    );
    assert_eq!(
      sniff_image_canonical_name(b"GIF89apayload"),
      Some("image.gif")
    );
    assert_eq!(
      sniff_image_canonical_name(b"RIFF\x20\x00\x00\x00WEBPVP8 "),
      Some("image.webp")
    );
    assert_eq!(sniff_image_canonical_name(b"not-an-image"), None);

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 8192];
      let n = socket.read(&mut buf).await.expect("read");
      let req = String::from_utf8_lossy(&buf[..n]).to_string();
      assert!(req.starts_with("POST /v1/tools/ocr HTTP/1.1"), "got: {req}");
      assert!(req.contains("rich_layout_recognition"));
      let body = r#"{"id":"ocr-nb-1","object":"ocr.result","created":1762790000,"purpose":"rich_layout_recognition","page_count":1,"pages":[{"index":0,"text":"Whiteboard Architecture: Zero-Copy Ring Buffer","finish_reason":"stop"}],"usage":{"input_tokens":200,"output_tokens":12,"total_tokens":212}}"#;
      let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      );
      socket.write_all(resp.as_bytes()).await.expect("write");
    });

    let prev_key = std::env::var("DEEPSEEK_API_KEY").ok();
    let prev_base = std::env::var("DEEPSEEK_BASE_URL").ok();
    // SAFETY: unit test restores env vars immediately after awaiting the local mock.
    unsafe {
      std::env::set_var("DEEPSEEK_API_KEY", "sk-nb-ocr-test");
      std::env::set_var("DEEPSEEK_BASE_URL", format!("http://{addr}/v1"));
    }

    let extracted = extract_bytes_text(b"\x89PNG\r\n\x1a\nfake".to_vec(), BytesFormat::Image).await;

    unsafe {
      match prev_key {
        Some(v) => std::env::set_var("DEEPSEEK_API_KEY", v),
        None => std::env::remove_var("DEEPSEEK_API_KEY"),
      }
      match prev_base {
        Some(v) => std::env::set_var("DEEPSEEK_BASE_URL", v),
        None => std::env::remove_var("DEEPSEEK_BASE_URL"),
      }
    }

    let text = extracted.expect("ocr text");
    assert!(text.contains("Zero-Copy Ring Buffer"));
    server.await.expect("server");
  }
}
