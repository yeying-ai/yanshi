//! Zero-Postgres Content Studio notebooks engine for the Tauri Desktop app.
//!
//! Mirrors `yanshi-server/src/notebooks.rs` 1:1 on top of local JSON files
//! under `<base_dir>/_notebooks/<notebook_id>/`:
//! - `POST   /v1/notebooks`
//! - `POST   /v1/notebooks/import`
//! - `GET    /v1/notebooks`
//! - `GET    /v1/notebooks/:id`
//! - `GET    /v1/notebooks/:id/export`
//! - `PATCH  /v1/notebooks/:id`
//! - `DELETE /v1/notebooks/:id`
//! - `POST   /v1/notebooks/:id/sources` (`url` | `text` | `file` | `recording`)
//! - `DELETE /v1/notebooks/:id/sources/:source_id`
//! - `GET    /v1/notebooks/:id/chunks/:chunk_id`
//! - `GET    /v1/notebooks/:id/search?q=&top_k=&source_ids=`
//! - `POST   /v1/notebooks/:id/ask` (including multi-turn `history`)
//! - `POST   /v1/notebooks/:id/generate` (all 12 compiled-in Studio generators + `cost_limit_usd`)
//! - `GET    /v1/notebooks/:id/artifacts` (including `deliverable` JSON)
//! - `GET    /v1/notebooks/:id/artifacts.zip` (`<kind>/<file>` + `BRIEFING.md` + manifest)
//! - `DELETE /v1/notebooks/:id/artifacts/:artifact_id`

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::{
  Json, Router,
  body::Body,
  extract::{DefaultBodyLimit, Path as RoutePath, Query, State},
  http::{HeaderMap, HeaderValue, StatusCode, header},
  response::{IntoResponse, Response},
  routing::{delete, get, post},
};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio_util::io::ReaderStream;
use uuid::Uuid;
use yanshi_rag::{
  Bm25KnowledgeBackend, KnowledgeBackend,
  chunking::{ChunkingStrategy, ParagraphChunker},
  sources::{DocumentLoader, html::HtmlLoader, pdf::PdfLoader},
};

use crate::local_engine::{
  LocalArtifactSummary, LocalEngineState, SpawnLocalRunParams, ensure_llm_initialized,
  extract_tenant, load_user_dotenv,
};

/// Uploaded / pasted sources larger than 10 MiB are refused at submit.
const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Uncompressed XML read ceiling when unpacking `.docx` / `.xlsx` / `.pptx` archives (zip-bomb guard).
const MAX_OOXML_XML_BYTES: usize = 16 * 1024 * 1024;
/// Maximum prior Q&A turns retained in multi-turn grounded `/ask`.
const MAX_ASK_HISTORY_TURNS: usize = 8;
const MAX_ASK_TURN_CHARS: usize = 2_000;
/// Paragraph chunking target size and overlap (characters), matching `yanshi-server`.
const CHUNK_SIZE: usize = 1200;
const CHUNK_OVERLAP: usize = 150;
/// Character budget of ready chunks passed into generator workflows and `/ask`.
const GENERATE_CHUNK_BUDGET: usize = 24_000;

/// Compiled-in Content Studio generator catalogue (`examples/generators/*.yml`),
/// kept in 1:1 sync with `yanshi-server/src/notebooks.rs::GENERATORS`.
const GENERATORS: &[(&str, &str)] = &[
  (
    "report",
    include_str!("../../../examples/generators/report.yml"),
  ),
  (
    "mindmap",
    include_str!("../../../examples/generators/mindmap.yml"),
  ),
  (
    "audio_overview",
    include_str!("../../../examples/generators/audio_overview.yml"),
  ),
  (
    "slides",
    include_str!("../../../examples/generators/slides.yml"),
  ),
  (
    "flashcards",
    include_str!("../../../examples/generators/flashcards.yml"),
  ),
  (
    "quiz",
    include_str!("../../../examples/generators/quiz.yml"),
  ),
  (
    "infographic",
    include_str!("../../../examples/generators/infographic.yml"),
  ),
  (
    "data_table",
    include_str!("../../../examples/generators/data_table.yml"),
  ),
  (
    "video_overview",
    include_str!("../../../examples/generators/video_overview.yml"),
  ),
  (
    "illustrated_slides",
    include_str!("../../../examples/generators/illustrated_slides.yml"),
  ),
  (
    "tech_slides_video",
    include_str!("../../../examples/generators/tech_slides_video.yml"),
  ),
  (
    "debate_podcast_show",
    include_str!("../../../examples/generators/debate_podcast_show.yml"),
  ),
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalNotebook {
  pub id: Uuid,
  pub tenant_id: String,
  pub title: String,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSource {
  pub id: Uuid,
  pub notebook_id: Uuid,
  pub tenant_id: String,
  pub kind: String,
  pub title: String,
  pub uri: Option<String>,
  pub status: String,
  pub error: Option<String>,
  pub sha256: Option<String>,
  pub created_at: DateTime<Utc>,
  pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LocalSourceChunk {
  pub chunk_id: String,
  pub source_id: Uuid,
  pub notebook_id: Uuid,
  pub tenant_id: String,
  pub ordinal: i32,
  pub content: String,
  pub metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalNotebookArtifact {
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub deliverable: Option<Value>,
  pub id: Uuid,
  pub run_id: Uuid,
  pub node_id: String,
  pub name: String,
  pub path_or_url: String,
  pub mime_type: Option<String>,
  pub created_at: DateTime<Utc>,
  pub tenant_id: String,
  pub size_bytes: Option<i64>,
  pub sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedNotebookBundle {
  notebook: LocalNotebook,
  #[serde(default)]
  sources: Vec<LocalSource>,
  #[serde(default)]
  artifacts: Vec<LocalNotebookArtifact>,
}

struct NotebookEntry {
  notebook: LocalNotebook,
  sources: Vec<LocalSource>,
  chunks_by_source: HashMap<Uuid, Vec<LocalSourceChunk>>,
  artifacts: Vec<LocalNotebookArtifact>,
}

struct NotebookIndex {
  backend: Bm25KnowledgeBackend,
  chunks: HashMap<String, LocalSourceChunk>,
}

#[derive(Clone, Default)]
struct NotebookIndexCache(Arc<Mutex<HashMap<Uuid, Arc<NotebookIndex>>>>);

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

  fn invalidate(&self, notebook_id: Uuid) {
    self
      .0
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .remove(&notebook_id);
  }
}

#[derive(Clone)]
pub struct LocalNotebooksState {
  entries: Arc<Mutex<HashMap<Uuid, NotebookEntry>>>,
  indexes: NotebookIndexCache,
  base_dir: PathBuf,
}

impl LocalNotebooksState {
  pub fn new(base_dir: PathBuf) -> Self {
    let hydrated = hydrate_notebooks_from_disk(&base_dir);
    Self {
      entries: Arc::new(Mutex::new(hydrated)),
      indexes: NotebookIndexCache::default(),
      base_dir,
    }
  }

  fn lock_entries(&self) -> std::sync::MutexGuard<'_, HashMap<Uuid, NotebookEntry>> {
    self
      .entries
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
  }

  fn notebook_dir(&self, notebook_id: Uuid) -> PathBuf {
    notebooks_root_dir(&self.base_dir).join(notebook_id.to_string())
  }

  fn attach_run_artifacts(
    &self,
    tenant_id: &str,
    notebook_id: Uuid,
    kind: &str,
    run_id: Uuid,
    summaries: &[LocalArtifactSummary],
    deliverable: Option<&Value>,
  ) {
    if summaries.is_empty() {
      return;
    }
    let prepared: Vec<LocalNotebookArtifact> = summaries
      .iter()
      .map(|summary| {
        let sha256 = std::fs::read(&summary.file_path)
          .ok()
          .map(|bytes| format!("{:x}", Sha256::digest(&bytes)));
        LocalNotebookArtifact {
          kind: kind.to_string(),
          deliverable: deliverable.cloned(),
          id: summary.id,
          run_id,
          node_id: summary.node_id.clone(),
          name: summary.name.clone(),
          path_or_url: summary.file_path.to_string_lossy().to_string(),
          mime_type: Some(summary.mime_type.clone()),
          created_at: summary.created_at,
          tenant_id: tenant_id.to_string(),
          size_bytes: i64::try_from(summary.size_bytes).ok(),
          sha256,
        }
      })
      .collect();

    let nb_dir = self.notebook_dir(notebook_id);
    let mut guard = self.lock_entries();
    let Some(entry) = guard.get_mut(&notebook_id) else {
      return;
    };
    if entry.notebook.tenant_id != tenant_id {
      return;
    }
    let mut changed = false;
    for candidate in prepared {
      if let Some(existing) = entry.artifacts.iter_mut().find(|a| a.id == candidate.id) {
        if existing.deliverable.is_none() && candidate.deliverable.is_some() {
          existing.deliverable = candidate.deliverable;
          changed = true;
        }
      } else {
        entry.artifacts.push(candidate);
        changed = true;
      }
    }
    if changed {
      entry.notebook.updated_at = Utc::now();
      let _ = write_notebook_bundle_sync(&nb_dir, entry);
    }
  }
}

fn notebooks_root_dir(base_dir: &Path) -> PathBuf {
  base_dir.join("_notebooks")
}

fn write_notebook_bundle_sync(nb_dir: &Path, entry: &NotebookEntry) -> std::io::Result<()> {
  std::fs::create_dir_all(nb_dir)?;
  let bundle = PersistedNotebookBundle {
    notebook: entry.notebook.clone(),
    sources: entry.sources.clone(),
    artifacts: entry.artifacts.clone(),
  };
  let bytes = serde_json::to_vec_pretty(&bundle).map_err(std::io::Error::other)?;
  let tmp = nb_dir.join(format!("notebook.json.tmp.{}", Uuid::new_v4()));
  let dst = nb_dir.join("notebook.json");
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

fn write_source_chunks_sync(
  nb_dir: &Path,
  source_id: Uuid,
  chunks: &[LocalSourceChunk],
) -> std::io::Result<()> {
  let chunks_dir = nb_dir.join("chunks");
  std::fs::create_dir_all(&chunks_dir)?;
  let bytes = serde_json::to_vec_pretty(chunks).map_err(std::io::Error::other)?;
  let tmp = chunks_dir.join(format!("{source_id}.json.tmp.{}", Uuid::new_v4()));
  let dst = chunks_dir.join(format!("{source_id}.json"));
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

fn hydrate_notebooks_from_disk(base_dir: &Path) -> HashMap<Uuid, NotebookEntry> {
  let mut out = HashMap::new();
  let root = notebooks_root_dir(base_dir);
  let Ok(read_dir) = std::fs::read_dir(&root) else {
    return out;
  };
  for item in read_dir.flatten() {
    let nb_dir = item.path();
    if !nb_dir.is_dir() {
      continue;
    }
    let Some(dir_name) = nb_dir.file_name().and_then(|n| n.to_str()) else {
      continue;
    };
    let Ok(notebook_id) = Uuid::parse_str(dir_name) else {
      continue;
    };
    let Ok(bytes) = std::fs::read(nb_dir.join("notebook.json")) else {
      continue;
    };
    let Ok(mut bundle) = serde_json::from_slice::<PersistedNotebookBundle>(&bytes) else {
      continue;
    };

    let mut needs_rewrite = false;
    for source in &mut bundle.sources {
      if matches!(source.status.as_str(), "pending" | "ingesting") {
        source.status = "failed".to_string();
        source.error = Some("desktop_restarted_while_ingesting".to_string());
        source.updated_at = Utc::now();
        needs_rewrite = true;
      }
    }

    let mut chunks_by_source = HashMap::new();
    if let Ok(chunks_dir) = std::fs::read_dir(nb_dir.join("chunks")) {
      for chunk_file in chunks_dir.flatten() {
        let cpath = chunk_file.path();
        if cpath.extension().and_then(|e| e.to_str()) != Some("json") {
          continue;
        }
        let Some(stem) = cpath.file_stem().and_then(|s| s.to_str()) else {
          continue;
        };
        let Ok(source_id) = Uuid::parse_str(stem) else {
          continue;
        };
        let Ok(cbytes) = std::fs::read(&cpath) else {
          continue;
        };
        if let Ok(mut chunks) = serde_json::from_slice::<Vec<LocalSourceChunk>>(&cbytes) {
          chunks.sort_by_key(|c| c.ordinal);
          chunks_by_source.insert(source_id, chunks);
        }
      }
    }

    let entry = NotebookEntry {
      notebook: bundle.notebook,
      sources: bundle.sources,
      chunks_by_source,
      artifacts: bundle.artifacts,
    };
    if needs_rewrite {
      let _ = write_notebook_bundle_sync(&nb_dir, &entry);
    }
    out.insert(notebook_id, entry);
  }
  out
}

pub fn build_notebooks_router(engine: LocalEngineState) -> Router {
  Router::new()
    .route("/v1/notebooks", get(list_notebooks).post(create_notebook))
    .route(
      "/v1/notebooks/import",
      post(import_notebook).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
    )
    .route(
      "/v1/notebooks/:id",
      get(get_notebook)
        .patch(update_notebook)
        .delete(delete_notebook),
    )
    .route("/v1/notebooks/:id/export", get(export_notebook))
    .route(
      "/v1/notebooks/:id/sources",
      post(add_source).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
    )
    .route(
      "/v1/notebooks/:id/sources/:source_id",
      delete(delete_source),
    )
    .route(
      "/v1/notebooks/:id/chunks/:chunk_id",
      get(get_notebook_chunk),
    )
    .route("/v1/notebooks/:id/search", get(search_notebook))
    .route("/v1/notebooks/:id/ask", post(ask_notebook))
    .route("/v1/notebooks/:id/generate", post(generate_from_notebook))
    .route("/v1/notebooks/:id/artifacts", get(list_notebook_artifacts))
    .route(
      "/v1/notebooks/:id/artifacts.zip",
      get(download_notebook_artifacts_zip),
    )
    .route(
      "/v1/notebooks/:id/artifacts/:artifact_id",
      delete(delete_notebook_artifact),
    )
    .with_state(engine)
}

fn notebook_error_json(status: StatusCode, message: &str) -> Response {
  (status, Json(json!({ "error": message }))).into_response()
}

#[derive(Debug, Deserialize)]
struct CreateNotebookRequest {
  title: String,
}

async fn create_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  Json(req): Json<CreateNotebookRequest>,
) -> Response {
  let title = req.title.trim();
  if title.is_empty() {
    return notebook_error_json(StatusCode::BAD_REQUEST, "title must not be empty");
  }
  let tenant = extract_tenant(&headers);
  let now = Utc::now();
  let notebook = LocalNotebook {
    id: Uuid::new_v4(),
    tenant_id: tenant,
    title: title.to_string(),
    created_at: now,
    updated_at: now,
  };
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook.id);
  let entry = NotebookEntry {
    notebook: notebook.clone(),
    sources: Vec::new(),
    chunks_by_source: HashMap::new(),
    artifacts: Vec::new(),
  };
  if let Err(err) = write_notebook_bundle_sync(&nb_dir, &entry) {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist notebook: {err}"),
    );
  }
  notebooks.lock_entries().insert(notebook.id, entry);
  (StatusCode::OK, Json(notebook)).into_response()
}

async fn list_notebooks(State(engine): State<LocalEngineState>, headers: HeaderMap) -> Response {
  let tenant = extract_tenant(&headers);
  let mut list: Vec<LocalNotebook> = engine
    .notebooks()
    .lock_entries()
    .values()
    .filter(|e| e.notebook.tenant_id == tenant)
    .map(|e| e.notebook.clone())
    .collect();
  list.sort_by_key(|a| std::cmp::Reverse(a.created_at));
  list.truncate(100);
  (StatusCode::OK, Json(json!({ "notebooks": list }))).into_response()
}

#[derive(Debug, Deserialize)]
struct UpdateNotebookRequest {
  title: String,
}

async fn get_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = engine.notebooks().lock_entries();
  let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  };
  let mut sources = entry.sources.clone();
  sources.sort_by_key(|a| a.created_at);
  (
    StatusCode::OK,
    Json(json!({
      "notebook": entry.notebook,
      "sources": sources,
    })),
  )
    .into_response()
}

async fn update_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<UpdateNotebookRequest>,
) -> Response {
  let title = req.title.trim();
  if title.is_empty() {
    return notebook_error_json(StatusCode::BAD_REQUEST, "title must not be empty");
  }
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(id);
  let mut guard = notebooks.lock_entries();
  let Some(entry) = guard
    .get_mut(&id)
    .filter(|e| e.notebook.tenant_id == tenant)
  else {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  };
  entry.notebook.title = title.to_string();
  entry.notebook.updated_at = Utc::now();
  if let Err(err) = write_notebook_bundle_sync(&nb_dir, entry) {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist notebook rename: {err}"),
    );
  }
  (StatusCode::OK, Json(entry.notebook.clone())).into_response()
}

async fn delete_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(id);
  let removed = {
    let mut guard = notebooks.lock_entries();
    if guard
      .get(&id)
      .is_some_and(|e| e.notebook.tenant_id == tenant)
    {
      guard.remove(&id);
      true
    } else {
      false
    }
  };
  if !removed {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  }
  notebooks.indexes.invalidate(id);
  let _ = std::fs::remove_dir_all(&nb_dir);
  StatusCode::NO_CONTENT.into_response()
}

async fn delete_source(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, source_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let deleted = {
    let mut guard = notebooks.lock_entries();
    let Some(entry) = guard
      .get_mut(&notebook_id)
      .filter(|e| e.notebook.tenant_id == tenant)
    else {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("source {source_id} in notebook {notebook_id} not found"),
      );
    };
    let before = entry.sources.len();
    entry.sources.retain(|s| s.id != source_id);
    if entry.sources.len() == before {
      false
    } else {
      entry.chunks_by_source.remove(&source_id);
      entry.notebook.updated_at = Utc::now();
      let _ = write_notebook_bundle_sync(&nb_dir, entry);
      true
    }
  };
  if !deleted {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("source {source_id} in notebook {notebook_id} not found"),
    );
  }
  let _ = std::fs::remove_file(nb_dir.join("chunks").join(format!("{source_id}.json")));
  notebooks.indexes.invalidate(notebook_id);
  StatusCode::NO_CONTENT.into_response()
}

async fn get_notebook_chunk(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, chunk_id)): RoutePath<(Uuid, String)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let trimmed = chunk_id.trim();
  let guard = engine.notebooks().lock_entries();
  let Some(entry) = guard
    .get(&notebook_id)
    .filter(|e| e.notebook.tenant_id == tenant)
  else {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("notebook {notebook_id} not found"),
    );
  };
  let found = entry
    .chunks_by_source
    .values()
    .flat_map(|list| list.iter())
    .find(|c| c.chunk_id == trimmed)
    .cloned();
  match found {
    Some(chunk) => (StatusCode::OK, Json(chunk)).into_response(),
    None => notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("chunk '{chunk_id}' in notebook {notebook_id} not found"),
    ),
  }
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum AddSourceRequest {
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
  File {
    filename: String,
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
  },
  Recording {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    content_base64: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    model: Option<String>,
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

enum SourcePayload {
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

async fn add_source(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(notebook_id): RoutePath<Uuid>,
  Json(req): Json<AddSourceRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  {
    let guard = engine.notebooks().lock_entries();
    if !guard
      .get(&notebook_id)
      .is_some_and(|e| e.notebook.tenant_id == tenant)
    {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("notebook {notebook_id} not found"),
      );
    }
  }

  let (kind, title, uri, payload) = match req {
    AddSourceRequest::Url { url, title } => {
      let trimmed = url.trim().to_string();
      let lowered = trimmed.to_ascii_lowercase();
      if !(lowered.starts_with("http://") || lowered.starts_with("https://")) {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          "url must start with http:// or https://",
        );
      }
      let resolved_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| trimmed.clone());
      (
        "url",
        resolved_title,
        Some(trimmed.clone()),
        SourcePayload::Url(trimmed),
      )
    }
    AddSourceRequest::Text { text, title } => {
      if text.trim().is_empty() {
        return notebook_error_json(StatusCode::BAD_REQUEST, "text must not be empty");
      }
      if text.len() > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("text is larger than {MAX_SOURCE_BYTES} bytes"),
        );
      }
      let resolved_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| text.chars().take(60).collect());
      (
        "text",
        resolved_title,
        None,
        SourcePayload::Bytes {
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
      let extension = Path::new(&filename)
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
          return notebook_error_json(
            StatusCode::BAD_REQUEST,
            &format!(
              "unsupported file type '.{other}': upload .txt, .md, .html, .pdf, .docx, .xlsx, .pptx, .csv, .tsv or .json"
            ),
          );
        }
      };
      let bytes = match base64::engine::general_purpose::STANDARD.decode(content_base64.trim()) {
        Ok(b) => b,
        Err(err) => {
          return notebook_error_json(StatusCode::BAD_REQUEST, &format!("content_base64: {err}"));
        }
      };
      if bytes.is_empty() {
        return notebook_error_json(StatusCode::BAD_REQUEST, "file must not be empty");
      }
      if bytes.len() > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("file is larger than {MAX_SOURCE_BYTES} bytes"),
        );
      }
      if format == BytesFormat::Pdf && !bytes.starts_with(b"%PDF-") {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          "uploaded .pdf file does not start with a %PDF- header",
        );
      }
      if matches!(
        format,
        BytesFormat::Docx | BytesFormat::Xlsx | BytesFormat::Pptx
      ) && !bytes.starts_with(b"PK\x03\x04")
      {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          &format!("uploaded .{extension} file is not a valid ZIP archive"),
        );
      }
      let resolved_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| filename.clone());
      (
        "file",
        resolved_title,
        Some(filename),
        SourcePayload::Bytes { bytes, format },
      )
    }
    AddSourceRequest::Recording {
      filename,
      content_base64,
      title,
      model,
      language,
    } => {
      if filename.trim().is_empty() || content_base64.trim().is_empty() {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          "recording requires non-empty `filename` and `content_base64`",
        );
      }
      let extension = Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
      if !matches!(
        extension.as_str(),
        "mp3" | "wav" | "m4a" | "ogg" | "flac" | "webm" | "aac" | "mp4"
      ) {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          &format!(
            "unsupported audio file type '.{extension}': upload .mp3, .wav, .m4a, .ogg, .flac, .webm or .aac"
          ),
        );
      }
      let bytes = match base64::engine::general_purpose::STANDARD.decode(content_base64.trim()) {
        Ok(b) => b,
        Err(err) => {
          return notebook_error_json(StatusCode::BAD_REQUEST, &format!("content_base64: {err}"));
        }
      };
      if bytes.is_empty() {
        return notebook_error_json(StatusCode::BAD_REQUEST, "recording must not be empty");
      }
      if bytes.len() > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("recording is larger than {MAX_SOURCE_BYTES} bytes"),
        );
      }
      load_user_dotenv();
      let Some(asr_model) = model
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
        .or_else(|| {
          std::env::var("YANSHI_ASR_MODEL")
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
        })
      else {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          "set `model` on the recording request or YANSHI_ASR_MODEL on the studio environment",
        );
      };
      let resolved_title = title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| filename.clone());
      (
        "recording",
        resolved_title,
        Some(filename.clone()),
        SourcePayload::Recording {
          bytes,
          filename,
          model: asr_model,
          language,
        },
      )
    }
  };

  let now = Utc::now();
  let source = LocalSource {
    id: Uuid::new_v4(),
    notebook_id,
    tenant_id: tenant.clone(),
    kind: kind.to_string(),
    title,
    uri,
    status: "pending".to_string(),
    error: None,
    sha256: None,
    created_at: now,
    updated_at: now,
  };

  let notebooks = engine.notebooks().clone();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  {
    let mut guard = notebooks.lock_entries();
    let Some(entry) = guard
      .get_mut(&notebook_id)
      .filter(|e| e.notebook.tenant_id == tenant)
    else {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("notebook {notebook_id} not found"),
      );
    };
    entry.sources.push(source.clone());
    entry.notebook.updated_at = now;
    if let Err(err) = write_notebook_bundle_sync(&nb_dir, entry) {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("failed to save source: {err}"),
      );
    }
  }

  let source_bg = source.clone();
  tokio::spawn(async move {
    ingest_local_source(notebooks, source_bg, payload).await;
  });

  (StatusCode::OK, Json(source)).into_response()
}

async fn ingest_local_source(
  notebooks: LocalNotebooksState,
  source: LocalSource,
  payload: SourcePayload,
) {
  let nb_dir = notebooks.notebook_dir(source.notebook_id);
  {
    let mut guard = notebooks.lock_entries();
    if let Some(entry) = guard.get_mut(&source.notebook_id)
      && let Some(target) = entry.sources.iter_mut().find(|s| s.id == source.id)
    {
      target.status = "ingesting".to_string();
      target.updated_at = Utc::now();
      let _ = write_notebook_bundle_sync(&nb_dir, entry);
    }
  }

  let outcome: Result<(String, Vec<LocalSourceChunk>), String> = async {
    let (digest, text) = match payload {
      SourcePayload::Url(url) => {
        let (bytes, format) = fetch_source_url(&url).await?;
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let text = extract_bytes_text(bytes, format).await?;
        (digest, text)
      }
      SourcePayload::Bytes { bytes, format } => {
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let text = extract_bytes_text(bytes, format).await?;
        (digest, text)
      }
      SourcePayload::Recording {
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
    let raw_chunks = chunk_paragraphs(&text)?;
    if raw_chunks.is_empty() {
      return Err("the source has no text".to_string());
    }
    let mut built = Vec::with_capacity(raw_chunks.len());
    for (idx, (content, metadata)) in raw_chunks.into_iter().enumerate() {
      let ordinal = i32::try_from(idx).map_err(|_| "too many chunks in source".to_string())?;
      built.push(LocalSourceChunk {
        chunk_id: format!("{}:{ordinal}", source.id),
        source_id: source.id,
        notebook_id: source.notebook_id,
        tenant_id: source.tenant_id.clone(),
        ordinal,
        content,
        metadata,
      });
    }
    write_source_chunks_sync(&nb_dir, source.id, &built)
      .map_err(|err| format!("storing chunks failed: {err}"))?;
    Ok((digest, built))
  }
  .await;

  {
    let mut guard = notebooks.lock_entries();
    if let Some(entry) = guard.get_mut(&source.notebook_id)
      && entry.sources.iter().any(|s| s.id == source.id)
    {
      let now = Utc::now();
      match outcome {
        Ok((digest, built_chunks)) => {
          entry.chunks_by_source.insert(source.id, built_chunks);
          if let Some(target) = entry.sources.iter_mut().find(|s| s.id == source.id) {
            target.status = "ready".to_string();
            target.error = None;
            target.sha256 = Some(digest);
            target.updated_at = now;
          }
        }
        Err(reason) => {
          tracing::warn!(source = %source.id, %reason, "local source ingestion failed");
          if let Some(target) = entry.sources.iter_mut().find(|s| s.id == source.id) {
            target.status = "failed".to_string();
            target.error = Some(reason);
            target.updated_at = now;
          }
        }
      }
      entry.notebook.updated_at = now;
      let _ = write_notebook_bundle_sync(&nb_dir, entry);
    }
  }
  notebooks.indexes.invalidate(source.notebook_id);
}

async fn extract_bytes_text(bytes: Vec<u8>, format: BytesFormat) -> Result<String, String> {
  match format {
    BytesFormat::Html => extract_html_text(&bytes).await,
    BytesFormat::Pdf => extract_pdf_text(&bytes).await,
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
  Path::new(&path)
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_ascii_lowercase)
}

async fn fetch_source_url(url: &str) -> Result<(Vec<u8>, BytesFormat), String> {
  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|err| format!("http client: {err}"))?;
  let response = tool
    .send_bytes(
      &json!({ "url": url, "method": "GET" }),
      Some(MAX_SOURCE_BYTES),
    )
    .await
    .map_err(|err| format!("fetching {url} failed: {err}"))?;
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

async fn extract_html_text(bytes: &[u8]) -> Result<String, String> {
  let path = std::env::temp_dir().join(format!("yanshi-local-ingest-{}.html", Uuid::new_v4()));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|err| format!("staging the page: {err}"))?;
  let loaded = HtmlLoader::new().load(&path).await;
  let _ = tokio::fs::remove_file(&path).await;
  loaded
    .map(|doc| doc.content)
    .map_err(|err| format!("extracting text: {err}"))
}

async fn extract_pdf_text(bytes: &[u8]) -> Result<String, String> {
  let id = Uuid::new_v4();
  let path = std::env::temp_dir().join(format!("yanshi-local-ingest-{id}.pdf"));
  tokio::fs::write(&path, bytes)
    .await
    .map_err(|err| format!("staging the PDF: {err}"))?;
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
  let out_path = std::env::temp_dir().join(format!("yanshi-local-ingest-{id}.txt"));
  let fallback = tokio::time::timeout(
    std::time::Duration::from_secs(60),
    tokio::process::Command::new("pdftotext")
      .args(["-layout", "-enc", "UTF-8"])
      .arg(&path)
      .arg(&out_path)
      .output(),
  )
  .await;
  let fallback_text = match fallback {
    Ok(Ok(output)) if output.status.success() => tokio::fs::read_to_string(&out_path).await.ok(),
    _ => None,
  };
  let _ = tokio::fs::remove_file(&path).await;
  let _ = tokio::fs::remove_file(&out_path).await;
  if let Some(text) = fallback_text
    && !text.trim().is_empty()
  {
    return Ok(text);
  }
  pure_rust
    .map(|doc| doc.content)
    .map_err(|err| format!("extracting PDF text: {err}"))
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
    let tag_name = tag_body.split_whitespace().next().unwrap_or_default();
    let local = tag_name.rsplit_once(':').map_or(tag_name, |(_, l)| l);
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
      ("rPh", false, false) => phonetic_depth += 1,
      ("rPh", true, _) => phonetic_depth = phonetic_depth.saturating_sub(1),
      ("t", false, false) if in_si => in_t = true,
      ("t", true, _) => in_t = false,
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
      ("v", false, false) if in_cell => in_v = true,
      ("v", true, _) => in_v = false,
      ("t", false, false) if in_cell => in_is_t = true,
      ("t", true, _) => in_is_t = false,
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
      ("w:t" | "a:t" | "t", false, false) => in_text_run = true,
      ("w:t" | "a:t" | "t", true, _) => in_text_run = false,
      ("w:tab" | "a:tab", false, _) => current_para.push('\t'),
      ("w:br" | "w:cr" | "a:br", false, _) => current_para.push('\n'),
      ("w:tbl" | "a:tbl", false, false) => {
        let trimmed = current_para.trim();
        if !trimmed.is_empty() {
          blocks.push(trimmed.to_string());
        }
        current_para.clear();
        table_depth += 1;
      }
      ("w:tbl" | "a:tbl", true, _) => table_depth = table_depth.saturating_sub(1),
      ("w:tr" | "a:tr", false, false) if table_depth > 0 => current_row.clear(),
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

  let mut lines: Vec<String> = vec![format!("Columns: {}", headers.join(" | "))];
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

async fn transcribe_recording(
  bytes: Vec<u8>,
  filename: String,
  model: String,
  language: Option<String>,
) -> Result<String, String> {
  let _ = ensure_llm_initialized().await;
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

fn chunk_paragraphs(text: &str) -> Result<Vec<(String, Value)>, String> {
  let chunks = ParagraphChunker::new(CHUNK_SIZE, CHUNK_OVERLAP)
    .chunk(text)
    .map_err(|err| format!("chunking failed: {err}"))?;
  Ok(
    chunks
      .into_iter()
      .filter(|c| !c.content.trim().is_empty())
      .map(|c| {
        (
          c.content,
          json!({ "start_line": c.start_idx, "end_line": c.end_idx }),
        )
      })
      .collect(),
  )
}

fn ready_chunks_for_entry(
  entry: &NotebookEntry,
  scope: Option<&HashSet<Uuid>>,
) -> Vec<LocalSourceChunk> {
  let mut ready_sources: Vec<&LocalSource> = entry
    .sources
    .iter()
    .filter(|s| s.status == "ready" && scope.is_none_or(|allowed| allowed.contains(&s.id)))
    .collect();
  ready_sources.sort_by_key(|a| a.created_at);

  let mut out = Vec::new();
  for src in ready_sources {
    if let Some(chunks) = entry.chunks_by_source.get(&src.id) {
      out.extend(chunks.iter().cloned());
    }
  }
  out
}

fn parse_source_ids_csv(raw: Option<&str>) -> Result<Option<Vec<Uuid>>, String> {
  let Some(raw) = raw else {
    return Ok(None);
  };
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return Err("source_ids must select at least one source".to_string());
  }
  let mut ids = Vec::new();
  for token in trimmed.split(',') {
    let token = token.trim();
    if token.is_empty() {
      continue;
    }
    let id = Uuid::parse_str(token).map_err(|_| format!("invalid source_id UUID '{token}'"))?;
    if !ids.contains(&id) {
      ids.push(id);
    }
  }
  if ids.is_empty() {
    return Err("source_ids must select at least one source".to_string());
  }
  Ok(Some(ids))
}

fn validate_entry_source_scope(
  entry: &NotebookEntry,
  notebook_id: Uuid,
  source_ids: Option<&[Uuid]>,
) -> Result<Option<HashSet<Uuid>>, String> {
  let Some(ids) = source_ids else {
    return Ok(None);
  };
  if ids.is_empty() {
    return Err("source_ids must select at least one source".to_string());
  }
  let known: HashSet<Uuid> = entry.sources.iter().map(|s| s.id).collect();
  let mut allowed = HashSet::with_capacity(ids.len());
  for &sid in ids {
    if !known.contains(&sid) {
      return Err(format!("source {sid} is not in notebook {notebook_id}"));
    }
    allowed.insert(sid);
  }
  Ok(Some(allowed))
}

fn resolve_notebook_model(explicit: Option<&str>) -> Result<String, String> {
  load_user_dotenv();
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
    .ok_or_else(|| "set `model`, or YANSHI_GENERATE_MODEL on the gateway".to_string())
}

#[derive(Debug, Deserialize)]
struct SearchQuery {
  q: String,
  #[serde(default)]
  top_k: Option<usize>,
  #[serde(default)]
  source_ids: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchHit {
  source_id: Uuid,
  chunk_id: String,
  content: String,
  score: f32,
}

async fn search_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Query(query): Query<SearchQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let parsed_ids = match parse_source_ids_csv(query.source_ids.as_deref()) {
    Ok(ids) => ids,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };

  let (scope, ready_chunks_opt) = {
    let guard = notebooks.lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    if query.q.trim().is_empty() {
      return notebook_error_json(StatusCode::BAD_REQUEST, "q must not be empty");
    }
    let scope = match validate_entry_source_scope(entry, id, parsed_ids.as_deref()) {
      Ok(scope) => scope,
      Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
    };
    let uncached = if notebooks.indexes.get(id).is_none() {
      Some(ready_chunks_for_entry(entry, None))
    } else {
      None
    };
    (scope, uncached)
  };

  let index = match notebooks.indexes.get(id) {
    Some(cached) => cached,
    None => {
      let ready_chunks = ready_chunks_opt.unwrap_or_default();
      let backend = Bm25KnowledgeBackend::from_documents(
        ready_chunks
          .iter()
          .map(|c| (c.chunk_id.clone(), c.content.clone())),
      )
      .with_name(format!("notebook:{id}"));
      let built = Arc::new(NotebookIndex {
        backend,
        chunks: ready_chunks
          .into_iter()
          .map(|c| (c.chunk_id.clone(), c))
          .collect(),
      });
      notebooks.indexes.put(id, built.clone());
      built
    }
  };

  if index.chunks.is_empty() {
    return (StatusCode::OK, Json(json!({ "chunks": [] }))).into_response();
  }

  let top_k = query.top_k.unwrap_or(5).clamp(1, 50);
  let raw_results = match &scope {
    None => index.backend.search(&query.q, top_k).await,
    Some(allowed) => {
      let scoped_docs: Vec<(String, String)> = index
        .chunks
        .values()
        .filter(|c| allowed.contains(&c.source_id))
        .map(|c| (c.chunk_id.clone(), c.content.clone()))
        .collect();
      if scoped_docs.is_empty() {
        return (StatusCode::OK, Json(json!({ "chunks": [] }))).into_response();
      }
      Bm25KnowledgeBackend::from_documents(scoped_docs)
        .search(&query.q, top_k)
        .await
    }
  };

  let hits = match raw_results {
    Ok(results) => results
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
      .collect::<Vec<_>>(),
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("search failed: {err}"),
      );
    }
  };

  (StatusCode::OK, Json(json!({ "chunks": hits }))).into_response()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct NotebookConversationTurn {
  #[serde(default)]
  question: String,
  #[serde(default)]
  answer: String,
}

#[derive(Debug, Deserialize)]
struct AskNotebookRequest {
  question: String,
  #[serde(default)]
  history: Vec<NotebookConversationTurn>,
  #[serde(default)]
  source_ids: Option<Vec<Uuid>>,
  #[serde(default)]
  model: Option<String>,
  #[serde(default)]
  top_k: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GroundedAnswerCitation {
  source_id: Uuid,
  chunk_id: String,
  quote: String,
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

async fn ask_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<AskNotebookRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let question = req.question.trim().to_string();
  let notebooks = engine.notebooks();
  let (has_scope, ready_chunks) = {
    let guard = notebooks.lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    if question.is_empty() {
      return notebook_error_json(StatusCode::BAD_REQUEST, "question must not be empty");
    }
    let scope = match validate_entry_source_scope(entry, id, req.source_ids.as_deref()) {
      Ok(scope) => scope,
      Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
    };
    let chunks = ready_chunks_for_entry(entry, scope.as_ref());
    (scope.is_some(), chunks)
  };

  let model = match resolve_notebook_model(req.model.as_deref()) {
    Ok(m) => m,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  if ready_chunks.is_empty() {
    let msg = if has_scope {
      "selected sources have no ready chunks to answer from".to_string()
    } else {
      format!("notebook {id} has no ready sources to answer from")
    };
    return notebook_error_json(StatusCode::BAD_REQUEST, &msg);
  }

  let history = normalize_ask_history(&req.history);
  let retrieval_hint = build_ask_retrieval_hint(&question, &history);
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

  let _ = ensure_llm_initialized().await;
  if let Err(err) = yanshi_llm::Yanshi::ensure_init().await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("initializing model registry: {err}"),
    );
  }
  let response = match yanshi_llm::Yanshi::model(&model)
    .system(
      "You are a source-grounded research assistant. Answer concisely in the language of the user's question and cite exact source chunk_ids.",
    )
    .prompt(&prompt)
    .json_schema("grounded_answer", schema)
    .execute_full()
    .await
  {
    Ok(res) => res,
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("LLM execution failed: {err}"),
      );
    }
  };

  let (answer, citations) = match parse_grounded_answer_reply(&response.content, &chunk_sources) {
    Ok(pair) => pair,
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("parsing grounded answer: {err}"),
      );
    }
  };

  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "question": question,
      "answer": answer,
      "model": model,
      "citations": citations,
      "used_chunk_count": used_chunk_count,
    })),
  )
    .into_response()
}

#[derive(Debug, Deserialize)]
struct GenerateRequest {
  kind: String,
  #[serde(default)]
  params: serde_json::Map<String, Value>,
  #[serde(default)]
  source_ids: Option<Vec<Uuid>>,
  #[serde(default)]
  model: Option<String>,
  #[serde(default)]
  cost_limit_usd: Option<f64>,
}

async fn generate_from_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<GenerateRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks().clone();
  let (has_scope, ready_chunks) = {
    let guard = notebooks.lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    let scope = match validate_entry_source_scope(entry, id, req.source_ids.as_deref()) {
      Ok(scope) => scope,
      Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
    };
    let chunks = ready_chunks_for_entry(entry, scope.as_ref());
    (scope.is_some(), chunks)
  };

  let Some((_, template)) = GENERATORS.iter().find(|(k, _)| *k == req.kind) else {
    let known = GENERATORS
      .iter()
      .map(|(k, _)| *k)
      .collect::<Vec<_>>()
      .join(", ");
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!("unknown kind '{}': expected one of {known}", req.kind),
    );
  };

  let model = match resolve_notebook_model(req.model.as_deref()) {
    Ok(m) => m,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  if let Some(limit) = req.cost_limit_usd
    && let Err(msg) = yanshi_config::config::v2::validate_cost_limit(limit)
  {
    return notebook_error_json(StatusCode::BAD_REQUEST, &msg);
  }

  let definition = match yanshi_config::executor::parse_workflow_definition(template) {
    Ok(def) => def,
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("generator template: {err:#}"),
      );
    }
  };
  for key in req.params.keys() {
    if matches!(key.as_str(), "chunks" | "url" | "text") {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!("'{key}' comes from the notebook and cannot be set"),
      );
    }
    if !definition.inputs.contains_key(key) {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!("'{key}' is not a parameter of '{}'", req.kind),
      );
    }
  }

  if ready_chunks.is_empty() {
    let msg = if has_scope {
      "selected sources have no ready chunks to generate from".to_string()
    } else {
      format!("notebook {id} has no ready sources to generate from")
    };
    return notebook_error_json(StatusCode::BAD_REQUEST, &msg);
  }

  let query_hint = req
    .params
    .get("focus")
    .and_then(Value::as_str)
    .or_else(|| req.params.get("topic").and_then(Value::as_str))
    .map(str::trim)
    .filter(|s| !s.is_empty());
  let selected = select_chunks_for_generate(ready_chunks, query_hint, GENERATE_CHUNK_BUDGET).await;
  let packed_chunks: Vec<Value> = selected
    .into_iter()
    .map(|chunk| {
      json!({
        "chunk_id": chunk.chunk_id,
        "source_id": chunk.source_id,
        "content": chunk.content,
      })
    })
    .collect();

  let mut inputs = req.params;
  inputs.insert("chunks".to_string(), Value::Array(packed_chunks));
  if definition.inputs.contains_key("chat_model") && !inputs.contains_key("chat_model") {
    inputs.insert("chat_model".to_string(), Value::String(model.clone()));
  }

  let kind_for_hook = req.kind.clone();
  let tenant_for_hook = tenant.clone();
  let notebooks_for_hook = notebooks;
  let hook = Arc::new(
    move |run_id: Uuid,
          _status: &str,
          summaries: &[LocalArtifactSummary],
          deliverable: Option<&Value>| {
      notebooks_for_hook.attach_run_artifacts(
        &tenant_for_hook,
        id,
        &kind_for_hook,
        run_id,
        summaries,
        deliverable,
      );
    },
  );

  let run_id = match engine
    .spawn_workflow_run(SpawnLocalRunParams {
      tenant_id: tenant,
      workflow: (*template).to_string(),
      inputs,
      model: Some(model),
      cost_limit_usd: req.cost_limit_usd,
      debug_controller: None,
      on_complete: Some(hook),
    })
    .await
  {
    Ok(rid) => rid,
    Err(msg) => return notebook_error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };

  (
    StatusCode::OK,
    Json(json!({
      "run_id": run_id,
      "kind": req.kind,
      "status": "queued",
    })),
  )
    .into_response()
}

async fn select_chunks_for_generate(
  ready: Vec<LocalSourceChunk>,
  query_hint: Option<&str>,
  budget: usize,
) -> Vec<LocalSourceChunk> {
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

  let mut source_order: Vec<Uuid> = Vec::new();
  let mut buckets: HashMap<Uuid, VecDeque<usize>> = HashMap::new();
  for (idx, chunk) in ready.iter().enumerate() {
    if picked[idx] {
      continue;
    }
    let entry = buckets.entry(chunk.source_id).or_insert_with(|| {
      source_order.push(chunk.source_id);
      VecDeque::new()
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

  let mut selected: Vec<LocalSourceChunk> = ready
    .iter()
    .enumerate()
    .filter(|(idx, _)| picked[*idx])
    .map(|(_, chunk)| chunk.clone())
    .collect();

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

async fn list_notebook_artifacts(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let guard = engine.notebooks().lock_entries();
  let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  };
  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "artifacts": entry.artifacts,
    })),
  )
    .into_response()
}

async fn delete_notebook_artifact(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, artifact_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let mut guard = notebooks.lock_entries();
  let Some(entry) = guard
    .get_mut(&notebook_id)
    .filter(|e| e.notebook.tenant_id == tenant)
  else {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("artifact {artifact_id} in notebook {notebook_id} not found"),
    );
  };
  let before = entry.artifacts.len();
  entry.artifacts.retain(|a| a.id != artifact_id);
  if entry.artifacts.len() == before {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("artifact {artifact_id} in notebook {notebook_id} not found"),
    );
  }
  entry.notebook.updated_at = Utc::now();
  let _ = write_notebook_bundle_sync(&nb_dir, entry);
  StatusCode::NO_CONTENT.into_response()
}

// ── Portable bundle export / import & deliverables `.zip` (AD2) ─────────

const NOTEBOOK_BUNDLE_VERSION: u32 = 1;
const MAX_IMPORT_SOURCES: usize = 200;
const MAX_IMPORT_CHUNKS_PER_SOURCE: usize = 2_000;

fn default_json_object() -> Value {
  json!({})
}

fn default_ready_status() -> String {
  "ready".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportedNotebookMeta {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  id: Option<Uuid>,
  title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  created_at: Option<DateTime<Utc>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportedSourceChunk {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  chunk_id: Option<String>,
  #[serde(default)]
  ordinal: i32,
  content: String,
  #[serde(default = "default_json_object")]
  metadata: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportedNotebookSource {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  id: Option<Uuid>,
  kind: String,
  title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  uri: Option<String>,
  #[serde(default = "default_ready_status")]
  status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  error: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  sha256: Option<String>,
  #[serde(default)]
  chunks: Vec<ExportedSourceChunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExportedNotebookDeliverable {
  artifact_id: Uuid,
  run_id: Uuid,
  kind: String,
  name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  mime_type: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  size_bytes: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  sha256: Option<String>,
  created_at: DateTime<Utc>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  deliverable: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct NotebookExportBundle {
  version: u32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  exported_at: Option<DateTime<Utc>>,
  notebook: ExportedNotebookMeta,
  #[serde(default)]
  sources: Vec<ExportedNotebookSource>,
  #[serde(default)]
  deliverables: Vec<ExportedNotebookDeliverable>,
  #[serde(default)]
  markdown_brief: String,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum ImportNotebookRequest {
  Wrapped {
    bundle: NotebookExportBundle,
    #[serde(default)]
    title: Option<String>,
  },
  Direct(NotebookExportBundle),
}

fn render_local_notebook_markdown_brief(
  title: &str,
  sources: &[LocalSource],
  chunks: &[LocalSourceChunk],
  artifacts: &[LocalNotebookArtifact],
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

  let mut group_order: Vec<(Uuid, String)> = Vec::new();
  let mut groups: HashMap<(Uuid, String), Vec<&LocalNotebookArtifact>> = HashMap::new();
  for item in artifacts {
    let key = (item.run_id, item.kind.clone());
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
      let files_list: Vec<String> = items.iter().map(|a| format!("`{}`", a.name)).collect();
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
  lines.push(String::new());
  lines.join("\n")
}

async fn export_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let bundle = {
    let guard = engine.notebooks().lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    let ready_chunks = ready_chunks_for_entry(entry, None);
    let mut chunks_by_source: HashMap<Uuid, Vec<ExportedSourceChunk>> = HashMap::new();
    for chunk in &ready_chunks {
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

    let exported_sources: Vec<ExportedNotebookSource> = entry
      .sources
      .iter()
      .map(|src| ExportedNotebookSource {
        id: Some(src.id),
        kind: src.kind.clone(),
        title: src.title.clone(),
        uri: src.uri.clone(),
        status: src.status.clone(),
        error: src.error.clone(),
        sha256: src.sha256.clone(),
        chunks: chunks_by_source.remove(&src.id).unwrap_or_default(),
      })
      .collect();

    let exported_deliverables: Vec<ExportedNotebookDeliverable> = entry
      .artifacts
      .iter()
      .map(|item| ExportedNotebookDeliverable {
        artifact_id: item.id,
        run_id: item.run_id,
        kind: item.kind.clone(),
        name: item.name.clone(),
        mime_type: item.mime_type.clone(),
        size_bytes: item.size_bytes,
        sha256: item.sha256.clone(),
        created_at: item.created_at,
        deliverable: item.deliverable.clone(),
      })
      .collect();

    let markdown_brief = render_local_notebook_markdown_brief(
      &entry.notebook.title,
      &entry.sources,
      &ready_chunks,
      &entry.artifacts,
    );

    NotebookExportBundle {
      version: NOTEBOOK_BUNDLE_VERSION,
      exported_at: Some(Utc::now()),
      notebook: ExportedNotebookMeta {
        id: Some(entry.notebook.id),
        title: entry.notebook.title.clone(),
        created_at: Some(entry.notebook.created_at),
        updated_at: Some(entry.notebook.updated_at),
      },
      sources: exported_sources,
      deliverables: exported_deliverables,
      markdown_brief,
    }
  };
  (StatusCode::OK, Json(bundle)).into_response()
}

async fn import_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  Json(req): Json<ImportNotebookRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let (bundle, title_override) = match req {
    ImportNotebookRequest::Wrapped { bundle, title } => (bundle, title),
    ImportNotebookRequest::Direct(bundle) => (bundle, None),
  };
  if bundle.version != NOTEBOOK_BUNDLE_VERSION {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!(
        "unsupported notebook bundle version {} (expected {})",
        bundle.version, NOTEBOOK_BUNDLE_VERSION
      ),
    );
  }
  let resolved_title = title_override
    .as_deref()
    .map(str::trim)
    .filter(|t| !t.is_empty())
    .unwrap_or_else(|| bundle.notebook.title.trim())
    .to_string();
  if resolved_title.is_empty() {
    return notebook_error_json(StatusCode::BAD_REQUEST, "notebook title must not be empty");
  }
  if bundle.sources.len() > MAX_IMPORT_SOURCES {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!(
        "bundle contains {} sources, exceeding limit of {MAX_IMPORT_SOURCES}",
        bundle.sources.len()
      ),
    );
  }

  let now = Utc::now();
  let notebook_id = Uuid::new_v4();
  let notebook = LocalNotebook {
    id: notebook_id,
    tenant_id: tenant.clone(),
    title: resolved_title,
    created_at: now,
    updated_at: now,
  };

  let mut total_bytes: usize = 0;
  let mut sources: Vec<LocalSource> = Vec::with_capacity(bundle.sources.len());
  let mut chunks_by_source: HashMap<Uuid, Vec<LocalSourceChunk>> = HashMap::new();
  let mut imported_chunks: usize = 0;

  for (idx, src) in bundle.sources.iter().enumerate() {
    let kind = src.kind.trim();
    if !matches!(kind, "url" | "file" | "text" | "recording") {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "source #{idx} has unsupported kind '{}': expected url, file, text, or recording",
          src.kind
        ),
      );
    }
    if src.chunks.len() > MAX_IMPORT_CHUNKS_PER_SOURCE {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "source #{idx} contains {} chunks, exceeding limit of {MAX_IMPORT_CHUNKS_PER_SOURCE}",
          src.chunks.len()
        ),
      );
    }
    let source_id = Uuid::new_v4();
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

    let mut ordered_raw: Vec<(i32, String, Value)> = Vec::with_capacity(src.chunks.len());
    for chunk in &src.chunks {
      let content = chunk.content.trim().to_string();
      if content.is_empty() {
        continue;
      }
      total_bytes = total_bytes.saturating_add(content.len());
      if total_bytes > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("imported chunk text exceeds {MAX_SOURCE_BYTES} bytes"),
        );
      }
      let meta = if chunk.metadata.is_object() {
        chunk.metadata.clone()
      } else {
        json!({})
      };
      ordered_raw.push((chunk.ordinal, content, meta));
    }
    ordered_raw.sort_by_key(|(ord, _, _)| *ord);

    let local_chunks: Vec<LocalSourceChunk> = ordered_raw
      .into_iter()
      .enumerate()
      .map(|(pos, (_, content, metadata))| {
        let ordinal = i32::try_from(pos).unwrap_or(0);
        LocalSourceChunk {
          chunk_id: format!("{source_id}:{ordinal}"),
          source_id,
          notebook_id,
          tenant_id: tenant.clone(),
          ordinal,
          content,
          metadata,
        }
      })
      .collect();

    let (status, error, sha256) = if !local_chunks.is_empty() {
      let digest = src
        .sha256
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
          let mut hasher = Sha256::new();
          for c in &local_chunks {
            hasher.update(c.content.as_bytes());
            hasher.update(b"\n");
          }
          format!("{:x}", hasher.finalize())
        });
      imported_chunks += local_chunks.len();
      chunks_by_source.insert(source_id, local_chunks);
      ("ready".to_string(), None, Some(digest))
    } else {
      let reason = src
        .error
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| "imported source had no text chunks".to_string());
      ("failed".to_string(), Some(reason), None)
    };

    sources.push(LocalSource {
      id: source_id,
      notebook_id,
      tenant_id: tenant.clone(),
      kind: kind.to_string(),
      title: source_title,
      uri: src
        .uri
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string),
      status,
      error,
      sha256,
      created_at: now,
      updated_at: now,
    });
  }

  let imported_sources = sources.len();
  let entry = NotebookEntry {
    notebook: notebook.clone(),
    sources,
    chunks_by_source,
    artifacts: Vec::new(),
  };

  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  if let Err(err) = write_notebook_bundle_sync(&nb_dir, &entry) {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist imported notebook: {err}"),
    );
  }
  for (sid, clist) in &entry.chunks_by_source {
    if let Err(err) = write_source_chunks_sync(&nb_dir, *sid, clist) {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("failed to persist imported source chunks: {err}"),
      );
    }
  }
  notebooks.lock_entries().insert(notebook_id, entry);

  (
    StatusCode::OK,
    Json(json!({
      "notebook": notebook,
      "imported_sources": imported_sources,
      "imported_chunks": imported_chunks,
    })),
  )
    .into_response()
}

fn sanitize_local_zip_segment(raw: &str, fallback: &str) -> String {
  let cleaned: String = raw
    .chars()
    .map(|c| {
      if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
        '_'
      } else {
        c
      }
    })
    .collect::<String>()
    .trim_matches(|c: char| c == '.' || c.is_whitespace())
    .to_string();
  if cleaned.is_empty() {
    fallback.to_string()
  } else {
    cleaned
  }
}

fn deduplicate_local_entry_path(seen: &mut HashSet<String>, dir: &str, filename: &str) -> String {
  let primary = format!("{dir}/{filename}");
  if seen.insert(primary.clone()) {
    return primary;
  }
  let (stem, ext) = match filename.rsplit_once('.') {
    Some((s, e)) if !s.is_empty() => (s, Some(e)),
    _ => (filename, None),
  };
  let mut idx = 2usize;
  loop {
    let candidate = match ext {
      Some(e) => format!("{dir}/{stem}_{idx}.{e}"),
      None => format!("{dir}/{stem}_{idx}"),
    };
    if seen.insert(candidate.clone()) {
      return candidate;
    }
    idx += 1;
  }
}

fn pack_local_notebook_artifacts_zip_blocking(
  notebook_id: Uuid,
  notebook_title: String,
  brief_markdown: String,
  artifacts: Vec<LocalNotebookArtifact>,
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
    let disk_path = PathBuf::from(&item.path_or_url);
    let mut file = match std::fs::File::open(&disk_path) {
      Ok(f) => f,
      Err(_) => continue,
    };
    let safe_kind = sanitize_local_zip_segment(&item.kind, "deliverable");
    let safe_name = sanitize_local_zip_segment(&item.name, "output.bin");
    let entry_path = deduplicate_local_entry_path(&mut seen_paths, &safe_kind, &safe_name);

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
      "artifact_id": item.id,
      "run_id": item.run_id,
      "kind": item.kind,
      "name": safe_name,
      "entry_path": entry_path,
      "mime_type": item.mime_type,
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

async fn download_notebook_artifacts_zip(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let (title, brief_md, artifacts) = {
    let guard = engine.notebooks().lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    if entry.artifacts.is_empty() {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("notebook {id} has no downloadable artifacts"),
      );
    }
    let ready_chunks = ready_chunks_for_entry(entry, None);
    let brief = render_local_notebook_markdown_brief(
      &entry.notebook.title,
      &entry.sources,
      &ready_chunks,
      &entry.artifacts,
    );
    (entry.notebook.title.clone(), brief, entry.artifacts.clone())
  };

  let packed = match tokio::task::spawn_blocking(move || {
    pack_local_notebook_artifacts_zip_blocking(id, title, brief_md, artifacts)
  })
  .await
  {
    Ok(Ok(packed)) => packed,
    Ok(Err(err)) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("failed to build notebook zip: {err}"),
      );
    }
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("notebook zip task failed: {err}"),
      );
    }
  };

  let Some((std_file, total_len)) = packed else {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("notebook {id} has no readable artifact files on disk"),
    );
  };

  let async_file = tokio::fs::File::from_std(std_file);
  let short_id = &id.to_string()[..8];
  let disposition = format!("attachment; filename=\"yanshi-notebook-{short_id}-artifacts.zip\"");

  let mut res = Response::new(Body::from_stream(ReaderStream::new(async_file)));
  res.headers_mut().insert(
    header::CONTENT_TYPE,
    HeaderValue::from_static("application/zip"),
  );
  res
    .headers_mut()
    .insert(header::CONTENT_LENGTH, HeaderValue::from(total_len));
  if let Ok(val) = HeaderValue::from_str(&disposition) {
    res.headers_mut().insert(header::CONTENT_DISPOSITION, val);
  }
  res
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::local_engine::start_loopback_engine;
  use crate::local_harness::tests::{mock_env_lock, register_mock_model};
  use std::time::Duration;

  #[test]
  fn every_compiled_in_desktop_generator_parses() {
    assert_eq!(
      GENERATORS.len(),
      12,
      "desktop engine must compile all 12 Studio notebook generators"
    );
    for (kind, template) in GENERATORS {
      let definition = yanshi_config::executor::parse_workflow_definition(template)
        .unwrap_or_else(|err| panic!("{kind}: {err:#}"));
      assert!(
        definition.inputs.contains_key("chunks"),
        "{kind} takes notebook chunks"
      );
      assert!(
        definition.nodes.iter().any(|n| n.id == "deliverable"),
        "{kind} defines deliverable node"
      );
    }
  }

  fn build_docx_bytes(document_xml: &str) -> Vec<u8> {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
      let mut zip = zip::ZipWriter::new(&mut cursor);
      let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
      zip.start_file("word/document.xml", opts).expect("start");
      zip.write_all(document_xml.as_bytes()).expect("write");
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  }

  #[tokio::test]
  async fn local_notebooks_supports_rich_sources_scoping_ask_deliverable_citations_and_deletion() {
    let _env_guard = mock_env_lock().lock().await;
    let model = format!("mock-desktop-notebook-{}", Uuid::new_v4());
    let asr_model = format!("mock-desktop-asr-{}", Uuid::new_v4());

    let temp = tempfile::tempdir().expect("tempdir");
    let models_cfg = temp.path().join("models.yml");
    let write_models = |responses: Vec<String>| {
      // SAFETY: serialized by `mock_env_lock()`.
      unsafe {
        std::env::set_var(
          "YANSHI_MOCK_RESPONSES",
          serde_json::to_string(&responses).expect("serialize mock responses"),
        );
        std::env::remove_var("YANSHI_MOCK_DELAY_MS");
      }
      std::fs::write(
        &models_cfg,
        format!(
          "models:\n  {model}:\n    vendor: mock\n    type: text\n    model_id: {model}\n  {asr_model}:\n    vendor: mock\n    type: asr\n    model_id: {asr_model}\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n"
        ),
      )
      .expect("write models.yml");
    };
    write_models(vec![]);
    yanshi_llm::Yanshi::init_with_config(models_cfg.to_str().expect("utf8"))
      .await
      .expect("init mock models");

    let base_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("start loopback engine");
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no_proxy client");

    // 1. Create notebook under tenant `studio-a`
    let created: Value = client
      .post(format!("{base_url}/v1/notebooks"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({ "title": "Oceanography Notes" }))
      .send()
      .await
      .expect("create notebook")
      .json()
      .await
      .expect("notebook json");
    let notebook_id = created["id"].as_str().expect("notebook id").to_string();
    assert_eq!(created["title"], "Oceanography Notes");

    // Tenant isolation check (`studio-b` cannot read, rename, or delete `studio-a`'s notebook)
    let cross_tenant = client
      .get(format!("{base_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross tenant get");
    assert_eq!(cross_tenant.status(), StatusCode::NOT_FOUND);
    let cross_rename = client
      .patch(format!("{base_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-b")
      .json(&json!({ "title": "Intruder" }))
      .send()
      .await
      .expect("cross tenant rename");
    assert_eq!(cross_rename.status(), StatusCode::NOT_FOUND);
    let renamed: Value = client
      .patch(format!("{base_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({ "title": "Oceanography & Estuarine Notes" }))
      .send()
      .await
      .expect("owner rename")
      .json()
      .await
      .expect("renamed json");
    assert_eq!(renamed["title"], "Oceanography & Estuarine Notes");
    let cross_delete = client
      .delete(format!("{base_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross tenant delete");
    assert_eq!(cross_delete.status(), StatusCode::NOT_FOUND);

    // 2. Add text, HTML, DOCX, CSV, and audio recording sources
    let text_src: Value = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "text",
        "title": "Lunar Gravity",
        "text": "Lunar gravitational forces pull ocean water into two tidal bulges on opposite sides of Earth."
      }))
      .send()
      .await
      .expect("add text source")
      .json()
      .await
      .expect("text src json");

    let html_b64 = base64::engine::general_purpose::STANDARD.encode(
      b"<html><head><script>var secret=42;</script></head><body><h1>Spring Tides</h1><p>When Sun and Moon align, spring tides reach maximum amplitude.</p></body></html>",
    );
    let html_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "spring-tides.html",
        "content_base64": html_b64,
      }))
      .send()
      .await
      .expect("add html source");
    assert_eq!(html_res.status(), StatusCode::OK);

    let docx_bytes = build_docx_bytes(
      r#"<?xml version="1.0" encoding="UTF-8"?><w:document><w:body><w:p><w:r><w:t>Amphidromic rotary waves have zero tidal range at the node.</w:t></w:r></w:p></w:body></w:document>"#,
    );
    let docx_src: Value = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "amphidromic.docx",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(&docx_bytes),
      }))
      .send()
      .await
      .expect("add docx source")
      .json()
      .await
      .expect("docx src json");

    let csv_src: Value = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "basins.csv",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(b"basin,peak_m\nBayOfFundy,16.3\n"),
      }))
      .send()
      .await
      .expect("add csv source")
      .json()
      .await
      .expect("csv src json");

    let xlsx_bytes = {
      use std::io::Write as _;
      let mut cursor = std::io::Cursor::new(Vec::new());
      {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("xl/workbook.xml", opts).expect("start");
        zip
          .write_all(
            br#"<workbook><sheets><sheet name="Harmonics" sheetId="1"/></sheets></workbook>"#,
          )
          .expect("write");
        zip.start_file("xl/sharedStrings.xml", opts).expect("start");
        zip
          .write_all(br#"<sst><si><t>Constituent</t></si><si><t>PeriodH</t></si><si><t>SemidiurnalM2</t></si></sst>"#)
          .expect("write");
        zip
          .start_file("xl/worksheets/sheet1.xml", opts)
          .expect("start");
        zip
          .write_all(
            br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>12.42</v></c></row></sheetData></worksheet>"#,
          )
          .expect("write");
        zip.finish().expect("finish");
      }
      cursor.into_inner()
    };
    let xlsx_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "harmonics.xlsx",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes),
      }))
      .send()
      .await
      .expect("add xlsx source");
    assert_eq!(xlsx_res.status(), StatusCode::OK);

    let pptx_bytes = {
      use std::io::Write as _;
      let mut cursor = std::io::Cursor::new(Vec::new());
      {
        let mut zip = zip::ZipWriter::new(&mut cursor);
        let opts = zip::write::SimpleFileOptions::default();
        zip
          .start_file("ppt/slides/slide1.xml", opts)
          .expect("start");
        zip
          .write_all(
            br#"<p:sld><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Estuarine KelvinWave Dynamics</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
          )
          .expect("write");
        zip.finish().expect("finish");
      }
      cursor.into_inner()
    };
    let pptx_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "kelvin.pptx",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(&pptx_bytes),
      }))
      .send()
      .await
      .expect("add pptx source");
    assert_eq!(pptx_res.status(), StatusCode::OK);

    let wav_bytes = yanshi_llm::providers::mock_media::silent_wav(150);
    let rec_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "recording",
        "filename": "field-note.wav",
        "model": asr_model,
        "content_base64": base64::engine::general_purpose::STANDARD.encode(&wav_bytes),
      }))
      .send()
      .await
      .expect("add recording source");
    assert_eq!(rec_res.status(), StatusCode::OK);

    // Poll until all 5 sources reach `ready`
    let mut all_ready = false;
    for _ in 0..100 {
      let detail: Value = client
        .get(format!("{base_url}/v1/notebooks/{notebook_id}"))
        .header("X-Yanshi-Tenant", "studio-a")
        .send()
        .await
        .expect("get notebook")
        .json()
        .await
        .expect("detail json");
      let sources = detail["sources"].as_array().expect("sources array");
      if sources.len() == 7 && sources.iter().all(|s| s["status"] == "ready") {
        all_ready = true;
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(all_ready, "all 7 desktop sources must reach ready");

    let xlsx_search: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=SemidiurnalM2"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("xlsx search")
      .json()
      .await
      .expect("xlsx search json");
    assert!(
      xlsx_search["chunks"]
        .as_array()
        .is_some_and(|c| !c.is_empty()),
      "xlsx must be searchable: {xlsx_search}"
    );
    let pptx_search: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=KelvinWave"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("pptx search")
      .json()
      .await
      .expect("pptx search json");
    assert!(
      pptx_search["chunks"]
        .as_array()
        .is_some_and(|c| !c.is_empty()),
      "pptx must be searchable: {pptx_search}"
    );

    // 3. Scoped BM25 search + chunk lookup
    let csv_id = csv_src["id"].as_str().expect("csv id");
    let docx_id = docx_src["id"].as_str().expect("docx id");
    let text_id = text_src["id"].as_str().expect("text id");
    let scoped_miss: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=bulges&source_ids={csv_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("scoped miss")
      .json()
      .await
      .expect("scoped miss json");
    assert_eq!(scoped_miss["chunks"].as_array().map(Vec::len), Some(0));

    let scoped_hit: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=BayOfFundy&source_ids={csv_id},{docx_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("scoped hit")
      .json()
      .await
      .expect("scoped hit json");
    let csv_chunk_id = scoped_hit["chunks"][0]["chunk_id"]
      .as_str()
      .expect("csv chunk_id")
      .to_string();

    let chunk_lookup: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/chunks/{csv_chunk_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("chunk lookup")
      .json()
      .await
      .expect("chunk json");
    assert!(
      chunk_lookup["content"]
        .as_str()
        .is_some_and(|c| c.contains("BayOfFundy")),
      "{chunk_lookup}"
    );

    // 4. Grounded Q&A (`POST /v1/notebooks/:id/ask`) + `data_table` generation with `deliverable`
    let ask_reply = json!({
      "answer": "Bay of Fundy reaches a peak tidal range of 16.3 m.",
      "citations": [
        { "chunk_id": csv_chunk_id, "quote": "BayOfFundy | peak_m: 16.3" },
        { "chunk_id": format!("{text_id}:0"), "quote": "unselected source chunk" },
        { "chunk_id": "hallucinated:99", "quote": "fake quote" }
      ]
    })
    .to_string();
    let table_reply = json!({
      "title": "Macrotidal Basin Metrics",
      "summary": "Extracted tidal basin measurements.",
      "columns": ["Basin", "Peak Range (m)"],
      "rows": [
        {
          "cells": ["Bay of Fundy", "16.3"],
          "chunk_id": csv_chunk_id,
          "quote": "basin: BayOfFundy | peak_m: 16.3"
        }
      ]
    })
    .to_string();
    write_models(vec![ask_reply, table_reply]);
    yanshi_llm::Yanshi::init_with_config(models_cfg.to_str().expect("utf8"))
      .await
      .expect("reinit mock responses");

    let ask_res: Value = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/ask"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "question": "What is the peak range of Bay of Fundy?",
        "model": model,
        "source_ids": [csv_id]
      }))
      .send()
      .await
      .expect("ask notebook")
      .json()
      .await
      .expect("ask json");
    assert!(
      ask_res["answer"]
        .as_str()
        .is_some_and(|a| a.contains("16.3 m")),
      "{ask_res}"
    );
    let ask_cites = ask_res["citations"].as_array().expect("ask citations");
    assert_eq!(
      ask_cites.len(),
      1,
      "unselected and hallucinated chunk_ids must be stripped: {ask_res}"
    );
    assert_eq!(ask_cites[0]["chunk_id"], csv_chunk_id);

    // 5. Generate `data_table` deliverable scoped to `csv_id`
    let gen_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/generate"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "data_table",
        "model": model,
        "source_ids": [csv_id],
        "params": {
          "max_rows": 10,
          "focus": "basin peak range"
        }
      }))
      .send()
      .await
      .expect("generate data_table");
    assert_eq!(gen_res.status(), StatusCode::OK);
    let gen_body: Value = gen_res.json().await.expect("generate body");
    let run_id = gen_body["run_id"].as_str().expect("run_id").to_string();

    let mut final_run_status = String::new();
    for _ in 0..120 {
      let run_detail: Value = client
        .get(format!("{base_url}/v1/runs/{run_id}"))
        .header("X-Yanshi-Tenant", "studio-a")
        .send()
        .await
        .expect("get run")
        .json()
        .await
        .expect("run json");
      final_run_status = run_detail["status"]
        .as_str()
        .unwrap_or_default()
        .to_string();
      if matches!(
        final_run_status.as_str(),
        "succeeded" | "failed" | "cancelled"
      ) {
        if final_run_status != "succeeded" {
          panic!("data_table generator failed: {run_detail}");
        }
        break;
      }
      tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert_eq!(final_run_status, "succeeded");

    let nb_artifacts: Value = client
      .get(format!("{base_url}/v1/notebooks/{notebook_id}/artifacts"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("list notebook artifacts")
      .json()
      .await
      .expect("artifacts json");
    let art_list = nb_artifacts["artifacts"]
      .as_array()
      .expect("artifacts list");
    assert_eq!(art_list.len(), 3, "xlsx + csv + html preview expected");
    assert_eq!(
      art_list[0]["deliverable"]["title"],
      "Macrotidal Basin Metrics"
    );
    assert_eq!(
      art_list[0]["deliverable"]["citations"][0]["chunk_id"],
      csv_chunk_id
    );

    // 6. Restart loopback engine and verify deliverable + chunks survive, then test source/notebook DELETE
    let restarted_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("restart loopback engine");
    let arts_after_restart: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("artifacts after restart")
      .json()
      .await
      .expect("artifacts after restart json");
    assert_eq!(
      arts_after_restart["artifacts"][0]["deliverable"]["title"],
      "Macrotidal Basin Metrics"
    );
    let first_art_id = arts_after_restart["artifacts"][0]["id"]
      .as_str()
      .expect("first art id")
      .to_string();
    let cross_detach = client
      .delete(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts/{first_art_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross-tenant detach");
    assert_eq!(cross_detach.status(), StatusCode::NOT_FOUND);
    let owner_detach = client
      .delete(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts/{first_art_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("owner detach");
    assert_eq!(owner_detach.status(), StatusCode::NO_CONTENT);
    let arts_after_detach: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("arts after detach")
      .json()
      .await
      .expect("arts after detach json");
    assert_eq!(
      arts_after_detach["artifacts"].as_array().map(Vec::len),
      Some(2)
    );

    // AD2: `cost_limit_usd <= 0` rejected, `.zip` archive download + portable export/import parity.
    let bad_cost = client
      .post(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/generate"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "data_table",
        "model": "mock-desktop-nb",
        "cost_limit_usd": 0.0
      }))
      .send()
      .await
      .expect("bad cost_limit_usd");
    assert_eq!(bad_cost.status(), StatusCode::BAD_REQUEST);

    let cross_zip = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts.zip"
      ))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross-tenant zip");
    assert_eq!(cross_zip.status(), StatusCode::NOT_FOUND);

    let nb_zip_resp = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts.zip"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("owner notebook zip");
    assert_eq!(nb_zip_resp.status(), StatusCode::OK);
    let nb_zip_bytes = nb_zip_resp.bytes().await.expect("nb zip bytes");
    {
      use std::io::Read as _;
      let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(nb_zip_bytes)).expect("open desktop nb zip");
      let mut brief = String::new();
      archive
        .by_name("BRIEFING.md")
        .expect("BRIEFING.md in desktop zip")
        .read_to_string(&mut brief)
        .expect("read desktop BRIEFING.md");
      assert!(
        brief.contains("Macrotidal Basin Metrics"),
        "desktop BRIEFING.md must include deliverable title: {brief}"
      );
      assert!(archive.by_name("notebook_artifacts.json").is_ok());
    }

    let exported: Value = client
      .get(format!("{restarted_url}/v1/notebooks/{notebook_id}/export"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("export desktop notebook")
      .json()
      .await
      .expect("export json");
    assert_eq!(exported["version"], 1);

    let imported: Value = client
      .post(format!("{restarted_url}/v1/notebooks/import"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "bundle": exported,
        "title": "Desktop Cloned Notebook"
      }))
      .send()
      .await
      .expect("import desktop notebook")
      .json()
      .await
      .expect("import json");
    assert_eq!(imported["notebook"]["title"], "Desktop Cloned Notebook");
    let imported_nb_id = imported["notebook"]["id"]
      .as_str()
      .expect("imported desktop id");
    let imported_hits: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{imported_nb_id}/search?q=BayOfFundy"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search imported desktop nb")
      .json()
      .await
      .expect("search imported json");
    assert!(
      imported_hits["chunks"]
        .as_array()
        .is_some_and(|arr| !arr.is_empty()),
      "imported desktop notebook must be immediately searchable: {imported_hits}"
    );

    let del_src = client
      .delete(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources/{docx_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("delete source");
    assert_eq!(del_src.status(), StatusCode::NO_CONTENT);
    let after_del_search: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/search?q=Amphidromic"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search deleted source")
      .json()
      .await
      .expect("search deleted json");
    assert_eq!(after_del_search["chunks"].as_array().map(Vec::len), Some(0));

    let del_nb = client
      .delete(format!("{restarted_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("delete notebook");
    assert_eq!(del_nb.status(), StatusCode::NO_CONTENT);
    let get_deleted = client
      .get(format!("{restarted_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("get deleted notebook");
    assert_eq!(get_deleted.status(), StatusCode::NOT_FOUND);

    // Ensure unused test helper import stays exercised cleanly.
    let _ = register_mock_model;
  }
}
