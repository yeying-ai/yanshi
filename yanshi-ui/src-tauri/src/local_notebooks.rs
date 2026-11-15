//! Zero-Postgres Content Studio notebooks engine for the Tauri Desktop app.
//!
//! Mirrors `yanshi-server/src/notebooks.rs` 1:1 on top of local JSON files
//! under `<base_dir>/_notebooks/<notebook_id>/`:
//! - `POST /v1/notebooks`
//! - `GET  /v1/notebooks`
//! - `GET  /v1/notebooks/:id`
//! - `POST /v1/notebooks/:id/sources` (`url` | `text` | `file` base64)
//! - `GET  /v1/notebooks/:id/search?q=&top_k=`
//! - `POST /v1/notebooks/:id/generate` (8 compiled-in generators)
//! - `GET  /v1/notebooks/:id/artifacts`

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use axum::{
  Json, Router,
  extract::{DefaultBodyLimit, Path as RoutePath, Query, State},
  http::{HeaderMap, StatusCode},
  response::{IntoResponse, Response},
  routing::{get, post},
};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use yanshi_rag::{
  Bm25KnowledgeBackend, KnowledgeBackend,
  chunking::{ChunkingStrategy, ParagraphChunker},
  sources::{DocumentLoader, html::HtmlLoader},
};

use crate::local_engine::{
  LocalArtifactSummary, LocalEngineState, SpawnLocalRunParams, extract_tenant, load_user_dotenv,
};

/// Uploaded / pasted sources larger than 10 MiB are refused at submit.
const MAX_SOURCE_BYTES: usize = 10 * 1024 * 1024;
/// Paragraph chunking target size and overlap (characters), matching `yanshi-server`.
const CHUNK_SIZE: usize = 1200;
const CHUNK_OVERLAP: usize = 150;
/// Character budget of ready chunks passed into generator workflows.
const GENERATE_CHUNK_BUDGET: usize = 24_000;

/// Compiled-in Content Studio generator catalogue (`examples/generators/*.yml`).
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
    "video_overview",
    include_str!("../../../examples/generators/video_overview.yml"),
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
      if !entry
        .artifacts
        .iter()
        .any(|existing| existing.id == candidate.id)
      {
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
    .route("/v1/notebooks/:id", get(get_notebook))
    .route(
      "/v1/notebooks/:id/sources",
      post(add_source).layer(DefaultBodyLimit::max(16 * 1024 * 1024)),
    )
    .route("/v1/notebooks/:id/search", get(search_notebook))
    .route("/v1/notebooks/:id/generate", post(generate_from_notebook))
    .route("/v1/notebooks/:id/artifacts", get(list_notebook_artifacts))
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
  },
  Recording {},
}

enum SourcePayload {
  Url(String),
  Bytes { bytes: Vec<u8>, html: bool },
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
          html: false,
        },
      )
    }
    AddSourceRequest::File {
      filename,
      content_base64,
    } => {
      let extension = Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
      let html = match extension.as_str() {
        "html" | "htm" => true,
        "txt" | "md" | "markdown" => false,
        other => {
          return notebook_error_json(
            StatusCode::BAD_REQUEST,
            &format!("unsupported file type '.{other}': upload .txt, .md or .html"),
          );
        }
      };
      let bytes = match base64::engine::general_purpose::STANDARD.decode(content_base64.trim()) {
        Ok(b) => b,
        Err(err) => {
          return notebook_error_json(StatusCode::BAD_REQUEST, &format!("content_base64: {err}"));
        }
      };
      if bytes.len() > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("file is larger than {MAX_SOURCE_BYTES} bytes"),
        );
      }
      (
        "file",
        filename.clone(),
        Some(filename),
        SourcePayload::Bytes { bytes, html },
      )
    }
    AddSourceRequest::Recording {} => {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        "recording sources are not supported yet; transcribe first and add the text",
      );
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
    let (bytes, html) = match payload {
      SourcePayload::Url(url) => fetch_source_url(&url).await?,
      SourcePayload::Bytes { bytes, html } => (bytes, html),
    };
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let text = if html {
      extract_html_text(&bytes).await?
    } else {
      String::from_utf8(bytes).map_err(|_| "the source is not UTF-8 text".to_string())?
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
    if let Some(entry) = guard.get_mut(&source.notebook_id) {
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

async fn fetch_source_url(url: &str) -> Result<(Vec<u8>, bool), String> {
  let tool = yanshi_tools::builtin::HttpTool::new(Arc::new(yanshi_tools::SandboxPolicy::default()))
    .map_err(|err| format!("http client: {err}"))?;
  let response = tool
    .send(&json!({ "url": url, "method": "GET" }))
    .await
    .map_err(|err| format!("fetching {url} failed: {err}"))?;
  if !response.status.is_success() {
    return Err(format!("fetching {url} returned HTTP {}", response.status));
  }
  let body = response.body;
  let prefix: String = body.trim_start().chars().take(512).collect();
  let lowered = prefix.to_ascii_lowercase();
  let html = lowered.starts_with("<!doctype html") || lowered.contains("<html");
  Ok((body.into_bytes(), html))
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

fn ready_chunks_for_entry(entry: &NotebookEntry) -> Vec<LocalSourceChunk> {
  let mut ready_sources: Vec<&LocalSource> = entry
    .sources
    .iter()
    .filter(|s| s.status == "ready")
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

#[derive(Debug, Deserialize)]
struct SearchQuery {
  q: String,
  #[serde(default)]
  top_k: Option<usize>,
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
  if query.q.trim().is_empty() {
    let exists = notebooks
      .lock_entries()
      .get(&id)
      .is_some_and(|e| e.notebook.tenant_id == tenant);
    if !exists {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    }
    return notebook_error_json(StatusCode::BAD_REQUEST, "q must not be empty");
  }

  let ready_chunks_opt = {
    let guard = notebooks.lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    if notebooks.indexes.get(id).is_none() {
      Some(ready_chunks_for_entry(entry))
    } else {
      None
    }
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
  let hits = match index.backend.search(&query.q, top_k).await {
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

#[derive(Debug, Deserialize)]
struct GenerateRequest {
  kind: String,
  #[serde(default)]
  params: serde_json::Map<String, Value>,
  #[serde(default)]
  model: Option<String>,
}

async fn generate_from_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<GenerateRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks().clone();
  let ready_chunks = {
    let guard = notebooks.lock_entries();
    let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    ready_chunks_for_entry(entry)
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

  load_user_dotenv();
  let Some(model) = req
    .model
    .as_deref()
    .map(str::trim)
    .filter(|m| !m.is_empty())
    .map(str::to_string)
    .or_else(|| {
      std::env::var("YANSHI_GENERATE_MODEL")
        .ok()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
    })
  else {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      "set `model`, or YANSHI_GENERATE_MODEL on the gateway",
    );
  };

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

  let mut budget = GENERATE_CHUNK_BUDGET;
  let packed_chunks: Vec<Value> = ready_chunks
    .into_iter()
    .take_while(|chunk| {
      let fits = chunk.content.len() <= budget;
      budget = budget.saturating_sub(chunk.content.len());
      fits
    })
    .map(|chunk| {
      json!({
        "chunk_id": chunk.chunk_id,
        "source_id": chunk.source_id,
        "content": chunk.content,
      })
    })
    .collect();
  if packed_chunks.is_empty() {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!("notebook {id} has no ready sources to generate from"),
    );
  }

  let mut inputs = req.params;
  inputs.insert("chunks".to_string(), Value::Array(packed_chunks));

  let kind_for_hook = req.kind.clone();
  let tenant_for_hook = tenant.clone();
  let notebooks_for_hook = notebooks;
  let hook = Arc::new(
    move |run_id: Uuid, _status: &str, summaries: &[LocalArtifactSummary]| {
      notebooks_for_hook.attach_run_artifacts(
        &tenant_for_hook,
        id,
        &kind_for_hook,
        run_id,
        summaries,
      );
    },
  );

  let run_id = match engine
    .spawn_workflow_run(SpawnLocalRunParams {
      tenant_id: tenant,
      workflow: (*template).to_string(),
      inputs,
      model: Some(model),
      cost_limit_usd: None,
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::local_engine::start_loopback_engine;
  use crate::local_harness::tests::{mock_env_lock, register_mock_model};
  use std::time::Duration;

  #[test]
  fn every_compiled_in_desktop_generator_parses() {
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

  #[tokio::test]
  async fn local_notebooks_ingests_searches_generates_flashcards_and_persists_across_restart() {
    let _env_guard = mock_env_lock().lock().await;
    let model = format!("mock-desktop-notebook-{}", Uuid::new_v4());
    // SAFETY: serialized by `mock_env_lock()`.
    unsafe {
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![
          r#"{"title":"Ocean Tides Study Cards","items":[{"front":"What drives lunar tides?","back":"Gravitational pull of the Moon on Earth's oceans.","chunk_id":"c0"}]}"#,
        ])
        .expect("serialize mock responses"),
      );
      std::env::remove_var("YANSHI_MOCK_DELAY_MS");
    }
    register_mock_model(&model).await;

    let temp = tempfile::tempdir().expect("tempdir");
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

    // Tenant isolation check (`studio-b` cannot read `studio-a`'s notebook)
    let cross_tenant = client
      .get(format!("{base_url}/v1/notebooks/{notebook_id}"))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross tenant get");
    assert_eq!(cross_tenant.status(), StatusCode::NOT_FOUND);

    // 2. Add a text source and a base64 HTML file source
    let text_src_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "text",
        "title": "Lunar Gravity",
        "text": "Lunar gravitational forces pull ocean water into two tidal bulges on opposite sides of Earth."
      }))
      .send()
      .await
      .expect("add text source");
    assert_eq!(text_src_res.status(), StatusCode::OK);

    let html_b64 = base64::engine::general_purpose::STANDARD.encode(
      b"<html><head><script>var secret=42;</script></head><body><h1>Spring Tides</h1><p>When Sun and Moon align, spring tides reach maximum amplitude.</p></body></html>",
    );
    let html_src_res = client
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
    assert_eq!(html_src_res.status(), StatusCode::OK);

    // Poll until both sources reach `ready`
    let mut all_ready = false;
    for _ in 0..80 {
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
      if sources.len() == 2 && sources.iter().all(|s| s["status"] == "ready") {
        all_ready = true;
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(all_ready, "both ingested sources must reach ready");

    // 3. Search BM25 index
    let search_res: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=spring+amplitude&top_k=3"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search notebook")
      .json()
      .await
      .expect("search json");
    let hits = search_res["chunks"].as_array().expect("chunks array");
    assert!(!hits.is_empty(), "BM25 search should return matching chunk");
    assert!(
      hits[0]["content"]
        .as_str()
        .unwrap_or_default()
        .contains("spring tides"),
      "top hit should contain spring tides text, got {:?}",
      hits[0]
    );
    assert!(
      !hits[0]["content"]
        .as_str()
        .unwrap_or_default()
        .contains("secret=42"),
      "HTML script content must be stripped"
    );

    // 4. Generate `flashcards` deliverable from notebook chunks
    let gen_res = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/generate"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "flashcards",
        "model": model,
        "params": {
          "count": 5,
          "focus": "lunar and spring tides"
        }
      }))
      .send()
      .await
      .expect("generate flashcards");
    assert_eq!(gen_res.status(), StatusCode::OK);
    let gen_body: Value = gen_res.json().await.expect("generate body");
    let run_id = gen_body["run_id"].as_str().expect("run_id").to_string();

    // Poll `/v1/runs/:run_id` until terminal
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
          panic!("flashcards generator failed: {run_detail}");
        }
        break;
      }
      tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert_eq!(final_run_status, "succeeded");

    // Immediately verify notebook artifacts are attached and downloadable
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
    assert_eq!(art_list.len(), 1);
    assert_eq!(art_list[0]["kind"], "flashcards");
    assert_eq!(art_list[0]["run_id"], run_id);
    let artifact_id = art_list[0]["id"].as_str().expect("artifact id");

    let dl_html = client
      .get(format!(
        "{base_url}/v1/runs/{run_id}/artifacts/{artifact_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("download artifact")
      .text()
      .await
      .expect("html text");
    assert!(dl_html.contains("Ocean Tides Study Cards"));

    // 5. Simulate desktop restart on the same base_dir and verify hydration
    let restarted_url = start_loopback_engine(Some(temp.path().to_path_buf()))
      .await
      .expect("restart loopback engine");
    let listed_after_restart: Value = client
      .get(format!("{restarted_url}/v1/notebooks"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("list after restart")
      .json()
      .await
      .expect("list json");
    assert_eq!(
      listed_after_restart["notebooks"]
        .as_array()
        .expect("notebooks")
        .len(),
      1
    );

    let search_after_restart: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/search?q=bulges&top_k=2"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search after restart")
      .json()
      .await
      .expect("search after restart json");
    assert!(
      !search_after_restart["chunks"]
        .as_array()
        .expect("chunks")
        .is_empty(),
      "hydrated chunks must rebuild BM25 index after restart"
    );

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
      arts_after_restart["artifacts"]
        .as_array()
        .expect("artifacts")
        .len(),
      1
    );
  }
}
