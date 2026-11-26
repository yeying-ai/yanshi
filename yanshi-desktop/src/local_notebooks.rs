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
//! - `PATCH  /v1/notebooks/:id/sources/:source_id` (`{title?, text?}`)
//! - `POST   /v1/notebooks/:id/sources/:source_id:retry`
//! - `GET    /v1/notebooks/:id/sources/:source_id/chunks`
//! - `DELETE /v1/notebooks/:id/sources/:source_id`
//! - `GET    /v1/notebooks/:id/chunks/:chunk_id`
//! - `GET    /v1/notebooks/:id/search?q=&top_k=&source_ids=`
//! - `POST   /v1/notebooks/:id/ask` (including multi-turn `history` & auto-persisted messages)
//! - `GET    /v1/notebooks/:id/messages`
//! - `DELETE /v1/notebooks/:id/messages`
//! - `POST   /v1/notebooks/:id/plan`
//! - `POST   /v1/notebooks/:id/generate` (all 12 compiled-in Studio generators + `cost_limit_usd`)
//! - `POST   /v1/notebooks/:id/generate-batch`
//! - `GET    /v1/notebooks/:id/artifacts` (including `deliverable` JSON)
//! - `POST   /v1/notebooks/:id/artifacts` (attach a completed Studio / workflow run)
//! - `GET    /v1/notebooks/:id/artifacts.zip` (`<kind>/<file>` + `BRIEFING.md` + manifest)
//! - `DELETE /v1/notebooks/:id/artifacts/:artifact_id`

use std::collections::{HashMap, HashSet};
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
use yanshi_notebook::bundle::{
  BriefArtifact, BriefChunk, BriefMessage, BriefSource, ExportedNotebookDeliverable,
  ExportedNotebookMessage, ExportedNotebookMeta, ExportedNotebookSource, ExportedSourceChunk,
  ImportNotebookRequest, MAX_IMPORT_CHUNKS_PER_SOURCE, MAX_IMPORT_SOURCES, NOTEBOOK_BUNDLE_VERSION,
  NotebookExportBundle, ZipArtifactItem, is_synthesizable_text_deliverable,
  pack_notebook_artifacts_zip_blocking as pack_shared_notebook_artifacts_zip_blocking,
  remap_imported_json_refs,
  render_notebook_markdown_brief as render_shared_notebook_markdown_brief, sanitize_zip_segment,
  synthesize_imported_deliverable_bytes,
};
use yanshi_notebook::ingest::{
  AddSourceRequest, BytesFormat, MAX_SOURCE_BYTES, chunk_paragraphs, extract_bytes_text,
  ingest_url as ingest_source_url, parse_source_ids_csv, sniff_image_canonical_name,
  transcribe_recording,
};
use yanshi_notebook::qa::{
  AskNotebookRequest, ListMessagesQuery, SearchHit, SearchQuery, build_ask_retrieval_hint,
  format_ask_history_block, grounded_answer_schema, normalize_ask_history,
  parse_grounded_answer_reply, resolve_notebook_model as resolve_notebook_model_raw,
  truncate_chars,
};
#[cfg(test)]
use yanshi_notebook::studio::GENERATORS;
use yanshi_notebook::studio::{
  DEFAULT_PLAN_MAX_ITEMS, GENERATE_CHUNK_BUDGET, GenerateBatchRequest, GenerateRequest,
  MAX_BATCH_GENERATE_ITEMS, PlanNotebookRequest, build_generator_catalogue_prompt,
  find_generator_template as find_local_generator_template,
  sanitize_studio_plan_reply as sanitize_studio_plan_reply_raw, select_chunks_by_budget,
  split_batch_cost_limit, studio_plan_schema,
};
use yanshi_rag::{Bm25KnowledgeBackend, KnowledgeBackend};

use crate::local_engine::{
  LocalArtifactSummary, LocalEngineState, SpawnLocalRunParams, ensure_llm_initialized,
  extract_tenant, load_user_dotenv,
};

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
pub struct LocalNotebookMessage {
  pub id: Uuid,
  pub notebook_id: Uuid,
  pub tenant_id: String,
  pub question: String,
  pub answer: String,
  pub model: String,
  pub citations: Value,
  pub used_chunk_count: i32,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PersistedNotebookBundle {
  notebook: LocalNotebook,
  #[serde(default)]
  sources: Vec<LocalSource>,
  /// Legacy inline `raw_texts` kept solely for reading older `notebook.json`
  /// files on startup and migrating them out to `<nb_dir>/raw/<source_id>.txt`.
  #[serde(default, skip_serializing_if = "HashMap::is_empty")]
  raw_texts: HashMap<Uuid, String>,
  #[serde(default)]
  artifacts: Vec<LocalNotebookArtifact>,
  #[serde(default)]
  messages: Vec<LocalNotebookMessage>,
}

impl PersistedNotebookBundle {
  fn from_entry(entry: &NotebookEntry) -> Self {
    Self {
      notebook: entry.notebook.clone(),
      sources: entry.sources.clone(),
      raw_texts: HashMap::new(),
      artifacts: entry.artifacts.clone(),
      messages: entry.messages.clone(),
    }
  }
}

struct NotebookEntry {
  notebook: LocalNotebook,
  sources: Vec<LocalSource>,
  chunks_by_source: HashMap<Uuid, Vec<LocalSourceChunk>>,
  artifacts: Vec<LocalNotebookArtifact>,
  messages: Vec<LocalNotebookMessage>,
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

  pub(crate) fn attach_run_artifacts(
    &self,
    tenant_id: &str,
    notebook_id: Uuid,
    kind: &str,
    run_id: Uuid,
    summaries: &[LocalArtifactSummary],
    deliverable: Option<&Value>,
  ) -> usize {
    let curated = select_local_deliverable_summaries(summaries, deliverable);
    if curated.is_empty() {
      return 0;
    }
    let prepared: Vec<LocalNotebookArtifact> = curated
      .into_iter()
      .map(|summary| {
        let sha256 = summary.sha256.clone().or_else(|| {
          std::fs::read(&summary.file_path)
            .ok()
            .map(|bytes| format!("{:x}", Sha256::digest(&bytes)))
        });
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
    let (attached_count, bundle_to_persist) = {
      let mut guard = self.lock_entries();
      let Some(entry) = guard.get_mut(&notebook_id) else {
        return 0;
      };
      if entry.notebook.tenant_id != tenant_id {
        return 0;
      }
      let mut changed = false;
      let mut attached_count = 0usize;
      for candidate in prepared {
        if let Some(existing) = entry.artifacts.iter_mut().find(|a| a.id == candidate.id) {
          if existing.deliverable.is_none() && candidate.deliverable.is_some() {
            existing.deliverable = candidate.deliverable;
            changed = true;
          }
        } else {
          entry.artifacts.push(candidate);
          attached_count += 1;
          changed = true;
        }
      }
      let snapshot = if changed {
        entry.notebook.updated_at = Utc::now();
        Some(PersistedNotebookBundle::from_entry(entry))
      } else {
        None
      };
      (attached_count, snapshot)
    };
    if let Some(bundle) = bundle_to_persist {
      if let Ok(handle) = tokio::runtime::Handle::try_current() {
        handle.spawn(async move {
          if let Err(err) = write_notebook_bundle_async(nb_dir, bundle).await {
            tracing::warn!(error = %err, "failed to persist notebook artifacts asynchronously");
          }
        });
      } else if let Err(err) = write_persisted_notebook_bundle_sync(&nb_dir, &bundle) {
        tracing::warn!(error = %err, "failed to persist notebook artifacts");
      }
    }
    attached_count
  }
}

fn select_local_deliverable_summaries<'a>(
  summaries: &'a [LocalArtifactSummary],
  deliverable: Option<&Value>,
) -> Vec<&'a LocalArtifactSummary> {
  if let Some(files) = deliverable
    .and_then(|d| d.get("files"))
    .and_then(Value::as_array)
  {
    let curated: HashSet<(&str, &str)> = files
      .iter()
      .filter(|entry| {
        matches!(
          entry.get("role").and_then(Value::as_str),
          Some("primary" | "preview")
        )
      })
      .filter_map(|entry| {
        let node_id = entry.get("node_id")?.as_str()?;
        let name = entry.get("name")?.as_str()?;
        Some((node_id, name))
      })
      .collect();
    if !curated.is_empty() {
      let matched: Vec<&'a LocalArtifactSummary> = summaries
        .iter()
        .filter(|s| curated.contains(&(s.node_id.as_str(), s.name.as_str())))
        .collect();
      if !matched.is_empty() {
        return matched;
      }
    }
  }
  summaries.iter().collect()
}

fn notebooks_root_dir(base_dir: &Path) -> PathBuf {
  base_dir.join("_notebooks")
}

fn raw_text_path(nb_dir: &Path, source_id: Uuid) -> PathBuf {
  nb_dir.join("raw").join(format!("{source_id}.txt"))
}

fn write_source_raw_text_sync(nb_dir: &Path, source_id: Uuid, text: &str) -> std::io::Result<()> {
  let raw_dir = nb_dir.join("raw");
  std::fs::create_dir_all(&raw_dir)?;
  let tmp = raw_dir.join(format!("{source_id}.txt.tmp.{}", Uuid::new_v4()));
  let dst = raw_dir.join(format!("{source_id}.txt"));
  std::fs::write(&tmp, text.as_bytes())?;
  std::fs::rename(tmp, dst)
}

async fn write_source_raw_text_async(
  nb_dir: PathBuf,
  source_id: Uuid,
  text: String,
) -> std::io::Result<()> {
  tokio::task::spawn_blocking(move || write_source_raw_text_sync(&nb_dir, source_id, &text))
    .await
    .map_err(std::io::Error::other)?
}

async fn read_source_raw_text_async(nb_dir: PathBuf, source_id: Uuid) -> Option<String> {
  tokio::task::spawn_blocking(move || {
    std::fs::read_to_string(raw_text_path(&nb_dir, source_id)).ok()
  })
  .await
  .ok()
  .flatten()
}

async fn remove_source_raw_text_async(nb_dir: PathBuf, source_id: Uuid) {
  let _ = tokio::task::spawn_blocking(move || {
    let _ = std::fs::remove_file(raw_text_path(&nb_dir, source_id));
  })
  .await;
}

fn write_persisted_notebook_bundle_sync(
  nb_dir: &Path,
  bundle: &PersistedNotebookBundle,
) -> std::io::Result<()> {
  std::fs::create_dir_all(nb_dir)?;
  let bytes = serde_json::to_vec_pretty(bundle).map_err(std::io::Error::other)?;
  let tmp = nb_dir.join(format!("notebook.json.tmp.{}", Uuid::new_v4()));
  let dst = nb_dir.join("notebook.json");
  std::fs::write(&tmp, bytes)?;
  std::fs::rename(tmp, dst)
}

async fn write_notebook_bundle_async(
  nb_dir: PathBuf,
  bundle: PersistedNotebookBundle,
) -> std::io::Result<()> {
  tokio::task::spawn_blocking(move || write_persisted_notebook_bundle_sync(&nb_dir, &bundle))
    .await
    .map_err(std::io::Error::other)?
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

async fn write_source_chunks_async(
  nb_dir: PathBuf,
  source_id: Uuid,
  chunks: Vec<LocalSourceChunk>,
) -> std::io::Result<()> {
  tokio::task::spawn_blocking(move || write_source_chunks_sync(&nb_dir, source_id, &chunks))
    .await
    .map_err(std::io::Error::other)?
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

    // Backward-compatible migration: split legacy inline `raw_texts` out of
    // `notebook.json` into per-source `<nb_dir>/raw/<source_id>.txt` files.
    if !bundle.raw_texts.is_empty() {
      let mut all_migrated = true;
      for (source_id, raw_text) in &bundle.raw_texts {
        if let Err(err) = write_source_raw_text_sync(&nb_dir, *source_id, raw_text) {
          tracing::warn!(
            notebook = %notebook_id,
            source = %source_id,
            error = %err,
            "failed to migrate legacy inline raw_text to sidecar file"
          );
          all_migrated = false;
        }
      }
      if all_migrated {
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
      messages: bundle.messages,
    };
    if needs_rewrite {
      let clean_bundle = PersistedNotebookBundle::from_entry(&entry);
      let _ = write_persisted_notebook_bundle_sync(&nb_dir, &clean_bundle);
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
      axum::routing::patch(update_source)
        .delete(delete_source)
        .post(post_source_action),
    )
    .route(
      "/v1/notebooks/:id/sources/:source_id/retry",
      post(retry_source),
    )
    .route(
      "/v1/notebooks/:id/sources/:source_id/chunks",
      get(list_source_chunks),
    )
    .route(
      "/v1/notebooks/:id/chunks/:chunk_id",
      get(get_notebook_chunk),
    )
    .route("/v1/notebooks/:id/search", get(search_notebook))
    .route("/v1/notebooks/:id/ask", post(ask_notebook))
    .route(
      "/v1/notebooks/:id/messages",
      get(list_notebook_messages).delete(clear_notebook_messages),
    )
    .route("/v1/notebooks/:id/plan", post(plan_notebook))
    .route("/v1/notebooks/:id/generate", post(generate_from_notebook))
    .route(
      "/v1/notebooks/:id/generate-batch",
      post(generate_batch_from_notebook),
    )
    .route(
      "/v1/notebooks/:id/artifacts",
      get(list_notebook_artifacts).post(attach_notebook_run_artifacts),
    )
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
  crate::local_engine::error_json(status, message)
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
    messages: Vec::new(),
  };
  let bundle = PersistedNotebookBundle::from_entry(&entry);
  if let Err(err) = write_notebook_bundle_async(nb_dir, bundle).await {
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
  let (updated_notebook, bundle) = {
    let mut guard = notebooks.lock_entries();
    let Some(entry) = guard
      .get_mut(&id)
      .filter(|e| e.notebook.tenant_id == tenant)
    else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    entry.notebook.title = title.to_string();
    entry.notebook.updated_at = Utc::now();
    (
      entry.notebook.clone(),
      PersistedNotebookBundle::from_entry(entry),
    )
  };
  if let Err(err) = write_notebook_bundle_async(nb_dir, bundle).await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist notebook rename: {err}"),
    );
  }
  (StatusCode::OK, Json(updated_notebook)).into_response()
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

#[derive(Debug, Deserialize)]
struct UpdateSourceRequest {
  #[serde(default)]
  title: Option<String>,
  #[serde(default)]
  text: Option<String>,
}

async fn update_source(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, source_id)): RoutePath<(Uuid, Uuid)>,
  Json(req): Json<UpdateSourceRequest>,
) -> Response {
  if req.title.is_none() && req.text.is_none() {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      "update requires at least one of `title` or `text`",
    );
  }
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);

  let prepared_text = if let Some(raw_text) = &req.text {
    if raw_text.trim().is_empty() {
      return notebook_error_json(StatusCode::BAD_REQUEST, "text must not be empty");
    }
    if raw_text.len() > MAX_SOURCE_BYTES {
      return notebook_error_json(
        StatusCode::PAYLOAD_TOO_LARGE,
        &format!("text exceeds {MAX_SOURCE_BYTES} bytes"),
      );
    }
    let raw_chunks = match chunk_paragraphs(raw_text) {
      Ok(c) if !c.is_empty() => c,
      Ok(_) => return notebook_error_json(StatusCode::BAD_REQUEST, "text produced no chunks"),
      Err(err) => return notebook_error_json(StatusCode::BAD_REQUEST, &err),
    };
    let mut built = Vec::with_capacity(raw_chunks.len());
    for (idx, (content, metadata)) in raw_chunks.into_iter().enumerate() {
      let Ok(ordinal) = i32::try_from(idx) else {
        return notebook_error_json(StatusCode::BAD_REQUEST, "too many chunks in source");
      };
      built.push(LocalSourceChunk {
        chunk_id: format!("{source_id}:{ordinal}"),
        source_id,
        notebook_id,
        tenant_id: tenant.clone(),
        ordinal,
        content,
        metadata,
      });
    }
    let digest = format!("{:x}", Sha256::digest(raw_text.as_bytes()));
    Some((digest, built))
  } else {
    None
  };

  let trimmed_title = match &req.title {
    Some(raw) => {
      let t = raw.trim();
      if t.is_empty() {
        return notebook_error_json(StatusCode::BAD_REQUEST, "title must not be empty");
      }
      Some(t.to_string())
    }
    None => None,
  };

  let updated_source = {
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
    let Some(src_idx) = entry.sources.iter().position(|s| s.id == source_id) else {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("source {source_id} in notebook {notebook_id} not found"),
      );
    };
    if matches!(
      entry.sources[src_idx].status.as_str(),
      "pending" | "ingesting"
    ) {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "source {source_id} is currently '{}'; wait for ingestion to finish before editing",
          entry.sources[src_idx].status
        ),
      );
    }
    if prepared_text.is_some() && entry.sources[src_idx].kind != "text" {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "only `text` note sources support inline `text` updates (source is `{}`)",
          entry.sources[src_idx].kind
        ),
      );
    }

    let now = Utc::now();
    let chunks_to_persist = if let Some((digest, built_chunks)) = prepared_text {
      let chunks_copy = built_chunks.clone();
      entry.chunks_by_source.insert(source_id, built_chunks);
      entry.sources[src_idx].status = "ready".to_string();
      entry.sources[src_idx].error = None;
      entry.sources[src_idx].sha256 = Some(digest);
      entry.sources[src_idx].updated_at = now;
      Some(chunks_copy)
    } else {
      None
    };
    if let Some(new_title) = trimmed_title {
      entry.sources[src_idx].title = new_title;
      entry.sources[src_idx].updated_at = now;
    }
    entry.notebook.updated_at = now;
    (
      entry.sources[src_idx].clone(),
      PersistedNotebookBundle::from_entry(entry),
      chunks_to_persist,
    )
  };
  let (updated_source, bundle, chunks_to_persist) = updated_source;
  if let Some(built_chunks) = chunks_to_persist
    && let Err(err) = write_source_chunks_async(nb_dir.clone(), source_id, built_chunks).await
  {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("storing chunks failed: {err}"),
    );
  }
  if let Some(raw_text) = req.text
    && let Err(err) = write_source_raw_text_async(nb_dir.clone(), source_id, raw_text).await
  {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("storing source text failed: {err}"),
    );
  }
  if let Err(err) = write_notebook_bundle_async(nb_dir, bundle).await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist source update: {err}"),
    );
  }

  notebooks.indexes.invalidate(notebook_id);
  (StatusCode::OK, Json(updated_source)).into_response()
}

async fn post_source_action(
  state: State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, raw_segment)): RoutePath<(Uuid, String)>,
) -> Response {
  let Some(raw_id) = raw_segment.strip_suffix(":retry") else {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      "source action route must end with :retry",
    );
  };
  let Ok(source_id) = Uuid::parse_str(raw_id) else {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!("invalid source id '{raw_id}'"),
    );
  };
  retry_source(state, headers, RoutePath((notebook_id, source_id))).await
}

async fn retry_source(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, source_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let (reopened, uri, bundle) = {
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
    let Some(src) = entry.sources.iter_mut().find(|s| s.id == source_id) else {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("source {source_id} in notebook {notebook_id} not found"),
      );
    };
    if src.status != "failed" {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "source {source_id} is '{}'; only failed sources can be retried",
          src.status
        ),
      );
    }
    let Some(uri) = src
      .uri
      .as_deref()
      .map(str::trim)
      .filter(|u| !u.is_empty())
      .map(str::to_string)
    else {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        "only failed URL sources with a stored URI can be retried",
      );
    };
    if src.kind != "url" {
      return notebook_error_json(
        StatusCode::BAD_REQUEST,
        &format!(
          "only `url` sources can be retried without re-uploading bytes (source is `{}`)",
          src.kind
        ),
      );
    }
    src.status = "pending".to_string();
    src.error = None;
    src.updated_at = Utc::now();
    let cloned = src.clone();
    let bundle = PersistedNotebookBundle::from_entry(entry);
    (cloned, uri, bundle)
  };
  let _ = write_notebook_bundle_async(nb_dir, bundle).await;

  tokio::spawn(ingest_local_source(
    notebooks.clone(),
    reopened.clone(),
    SourcePayload::Url(uri),
  ));
  (StatusCode::OK, Json(reopened)).into_response()
}

async fn delete_source(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, source_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let bundle_to_persist = {
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
      None
    } else {
      entry.chunks_by_source.remove(&source_id);
      entry.notebook.updated_at = Utc::now();
      Some(PersistedNotebookBundle::from_entry(entry))
    }
  };
  let Some(bundle) = bundle_to_persist else {
    return notebook_error_json(
      StatusCode::NOT_FOUND,
      &format!("source {source_id} in notebook {notebook_id} not found"),
    );
  };
  let _ = write_notebook_bundle_async(nb_dir.clone(), bundle).await;
  remove_source_raw_text_async(nb_dir.clone(), source_id).await;
  let chunk_file = nb_dir.join("chunks").join(format!("{source_id}.json"));
  let _ = tokio::fs::remove_file(chunk_file).await;
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

async fn list_source_chunks(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, source_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let nb_dir = engine.notebooks().notebook_dir(notebook_id);
  let (source, mut chunks) = {
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
    let Some(source) = entry.sources.iter().find(|s| s.id == source_id).cloned() else {
      return notebook_error_json(
        StatusCode::NOT_FOUND,
        &format!("source {source_id} in notebook {notebook_id} not found"),
      );
    };
    let chunks = entry
      .chunks_by_source
      .get(&source_id)
      .cloned()
      .unwrap_or_default();
    (source, chunks)
  };
  chunks.sort_by_key(|c| c.ordinal);
  let raw_text = read_source_raw_text_async(nb_dir, source_id).await;
  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": notebook_id,
      "source": source,
      "raw_text": raw_text,
      "chunks": chunks,
    })),
  )
    .into_response()
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

  let mut initial_raw_text: Option<String> = None;
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
      initial_raw_text = Some(text.clone());
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
        "srt" | "vtt" => BytesFormat::Subtitle,
        "pdf" => BytesFormat::Pdf,
        "png" | "jpg" | "jpeg" | "webp" | "gif" => BytesFormat::Image,
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
              "unsupported file type '.{other}': upload .txt, .md, .srt, .vtt, .html, .pdf, .png, .jpg, .jpeg, .webp, .gif, .docx, .xlsx, .pptx, .csv, .tsv or .json"
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
      if format == BytesFormat::Image && sniff_image_canonical_name(&bytes).is_none() {
        return notebook_error_json(
          StatusCode::BAD_REQUEST,
          &format!("uploaded .{extension} file is not a valid PNG, JPEG, GIF or WebP image"),
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
  let bundle = {
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
    PersistedNotebookBundle::from_entry(entry)
  };
  if let Some(raw_text) = initial_raw_text
    && let Err(err) = write_source_raw_text_async(nb_dir.clone(), source.id, raw_text).await
  {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to save source text: {err}"),
    );
  }
  if let Err(err) = write_notebook_bundle_async(nb_dir, bundle).await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to save source: {err}"),
    );
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
  let start_bundle = {
    let mut guard = notebooks.lock_entries();
    if let Some(entry) = guard.get_mut(&source.notebook_id)
      && let Some(target) = entry.sources.iter_mut().find(|s| s.id == source.id)
    {
      target.status = "ingesting".to_string();
      target.updated_at = Utc::now();
      Some(PersistedNotebookBundle::from_entry(entry))
    } else {
      None
    }
  };
  if let Some(bundle) = start_bundle {
    let _ = write_notebook_bundle_async(nb_dir.clone(), bundle).await;
  }

  load_user_dotenv();
  let outcome: Result<(String, Vec<LocalSourceChunk>), String> = async {
    let (digest, text) = match payload {
      SourcePayload::Url(url) => ingest_source_url(&url).await?,
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
    write_source_chunks_async(nb_dir.clone(), source.id, built.clone())
      .await
      .map_err(|err| format!("storing chunks failed: {err}"))?;
    Ok((digest, built))
  }
  .await;

  let final_bundle = {
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
      Some(PersistedNotebookBundle::from_entry(entry))
    } else {
      None
    }
  };
  if let Some(bundle) = final_bundle {
    let _ = write_notebook_bundle_async(nb_dir, bundle).await;
  }
  notebooks.indexes.invalidate(source.notebook_id);
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
  resolve_notebook_model_raw(explicit)
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

  let citations_json = serde_json::to_value(&citations).unwrap_or_else(|_| json!([]));
  let message_id = Uuid::new_v4();
  let nb_dir = notebooks.notebook_dir(id);
  let bundle_to_persist = {
    let mut guard = notebooks.lock_entries();
    if let Some(entry) = guard
      .get_mut(&id)
      .filter(|e| e.notebook.tenant_id == tenant)
    {
      entry.messages.push(LocalNotebookMessage {
        id: message_id,
        notebook_id: id,
        tenant_id: tenant.clone(),
        question: question.clone(),
        answer: answer.clone(),
        model: model.clone(),
        citations: citations_json,
        used_chunk_count: i32::try_from(used_chunk_count).unwrap_or(i32::MAX),
        created_at: Utc::now(),
      });
      Some(PersistedNotebookBundle::from_entry(entry))
    } else {
      None
    }
  };
  if let Some(bundle) = bundle_to_persist {
    let _ = write_notebook_bundle_async(nb_dir, bundle).await;
  }

  (
    StatusCode::OK,
    Json(json!({
      "message_id": message_id,
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

async fn list_notebook_messages(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Query(query): Query<ListMessagesQuery>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let limit = query.limit.unwrap_or(200).clamp(1, 500);
  let guard = engine.notebooks().lock_entries();
  let Some(entry) = guard.get(&id).filter(|e| e.notebook.tenant_id == tenant) else {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  };
  let mut messages = entry.messages.clone();
  messages.sort_by_key(|m| m.created_at);
  if messages.len() > limit {
    let skip = messages.len() - limit;
    messages.drain(0..skip);
  }
  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "messages": messages,
    })),
  )
    .into_response()
}

async fn clear_notebook_messages(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(id);
  let (deleted, bundle) = {
    let mut guard = notebooks.lock_entries();
    let Some(entry) = guard
      .get_mut(&id)
      .filter(|e| e.notebook.tenant_id == tenant)
    else {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    };
    let count = entry.messages.len();
    entry.messages.clear();
    (count, PersistedNotebookBundle::from_entry(entry))
  };
  let _ = write_notebook_bundle_async(nb_dir, bundle).await;
  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "deleted": deleted,
    })),
  )
    .into_response()
}

struct PreparedLocalGeneratorItem {
  kind: String,
  template: &'static str,
  inputs: serde_json::Map<String, Value>,
}

fn load_local_ready_chunks(
  notebooks: &LocalNotebooksState,
  tenant: &str,
  notebook_id: Uuid,
  source_ids: Option<&[Uuid]>,
  verb: &str,
) -> Result<Vec<LocalSourceChunk>, (StatusCode, String)> {
  let guard = notebooks.lock_entries();
  let Some(entry) = guard
    .get(&notebook_id)
    .filter(|e| e.notebook.tenant_id == tenant)
  else {
    return Err((
      StatusCode::NOT_FOUND,
      format!("notebook {notebook_id} not found"),
    ));
  };
  let scope = validate_entry_source_scope(entry, notebook_id, source_ids)
    .map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;
  let chunks = ready_chunks_for_entry(entry, scope.as_ref());
  if chunks.is_empty() {
    let msg = if scope.is_some() {
      format!("selected sources have no ready chunks to {verb}")
    } else {
      format!("notebook {notebook_id} has no ready sources to {verb}")
    };
    return Err((StatusCode::BAD_REQUEST, msg));
  }
  Ok(chunks)
}

async fn prepare_local_generator_item(
  kind: &str,
  params: serde_json::Map<String, Value>,
  ready_chunks: &[LocalSourceChunk],
  model: &str,
) -> Result<PreparedLocalGeneratorItem, (StatusCode, String)> {
  let trimmed_kind = kind.trim();
  let template =
    find_local_generator_template(trimmed_kind).map_err(|msg| (StatusCode::BAD_REQUEST, msg))?;
  let definition = yanshi_config::executor::parse_workflow_definition(template).map_err(|err| {
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      format!("generator template: {err:#}"),
    )
  })?;
  for key in params.keys() {
    if matches!(key.as_str(), "chunks" | "url" | "text") {
      return Err((
        StatusCode::BAD_REQUEST,
        format!("'{key}' comes from the notebook and cannot be set"),
      ));
    }
    if !definition.inputs.contains_key(key) {
      return Err((
        StatusCode::BAD_REQUEST,
        format!("'{key}' is not a parameter of '{trimmed_kind}'"),
      ));
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

  let mut inputs = params;
  inputs.insert("chunks".to_string(), Value::Array(packed_chunks));
  if definition.inputs.contains_key("chat_model") && !inputs.contains_key("chat_model") {
    inputs.insert("chat_model".to_string(), Value::String(model.to_string()));
  }

  Ok(PreparedLocalGeneratorItem {
    kind: trimmed_kind.to_string(),
    template,
    inputs,
  })
}

async fn dispatch_prepared_local_generator(
  engine: &LocalEngineState,
  notebooks: &LocalNotebooksState,
  tenant: &str,
  notebook_id: Uuid,
  prepared: PreparedLocalGeneratorItem,
  model: &str,
  cost_limit_usd: Option<f64>,
) -> Result<(Uuid, String), String> {
  let kind_for_hook = prepared.kind.clone();
  let tenant_for_hook = tenant.to_string();
  let notebooks_for_hook = notebooks.clone();
  let hook = Arc::new(
    move |run_id: Uuid,
          status: &str,
          summaries: &[LocalArtifactSummary],
          deliverable: Option<&Value>| {
      if status == "succeeded" {
        notebooks_for_hook.attach_run_artifacts(
          &tenant_for_hook,
          notebook_id,
          &kind_for_hook,
          run_id,
          summaries,
          deliverable,
        );
      }
    },
  );

  let run_id = engine
    .spawn_workflow_run(SpawnLocalRunParams {
      tenant_id: tenant.to_string(),
      workflow: prepared.template.to_string(),
      inputs: prepared.inputs,
      model: Some(model.to_string()),
      cost_limit_usd,
      max_concurrency: None,
      debug_controller: None,
      notebook_id: Some(notebook_id),
      notebook_kind: Some(prepared.kind.clone()),
      on_complete: Some(hook),
    })
    .await?;
  Ok((run_id, prepared.kind))
}

async fn generate_from_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<GenerateRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks().clone();
  if let Err(msg) = find_local_generator_template(req.kind.trim()) {
    let has_nb = notebooks
      .lock_entries()
      .get(&id)
      .is_some_and(|e| e.notebook.tenant_id == tenant);
    if !has_nb {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    }
    return notebook_error_json(StatusCode::BAD_REQUEST, &msg);
  }
  let model = match resolve_notebook_model(req.model.as_deref()) {
    Ok(m) => m,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  if let Some(limit) = req.cost_limit_usd
    && let Err(msg) = yanshi_config::config::v2::validate_cost_limit(limit)
  {
    return notebook_error_json(StatusCode::BAD_REQUEST, &msg);
  }
  let ready_chunks = match load_local_ready_chunks(
    &notebooks,
    &tenant,
    id,
    req.source_ids.as_deref(),
    "generate from",
  ) {
    Ok(c) => c,
    Err((code, msg)) => return notebook_error_json(code, &msg),
  };
  let prepared =
    match prepare_local_generator_item(&req.kind, req.params, &ready_chunks, &model).await {
      Ok(p) => p,
      Err((code, msg)) => return notebook_error_json(code, &msg),
    };
  let (run_id, kind) = match dispatch_prepared_local_generator(
    &engine,
    &notebooks,
    &tenant,
    id,
    prepared,
    &model,
    req.cost_limit_usd,
  )
  .await
  {
    Ok(pair) => pair,
    Err(msg) => return notebook_error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
  };

  (
    StatusCode::OK,
    Json(json!({
      "run_id": run_id,
      "kind": kind,
      "status": "queued",
    })),
  )
    .into_response()
}

async fn generate_batch_from_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<GenerateBatchRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks().clone();
  let has_nb = notebooks
    .lock_entries()
    .get(&id)
    .is_some_and(|e| e.notebook.tenant_id == tenant);
  if !has_nb {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  }
  if req.items.is_empty() {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      "items must contain at least one generator",
    );
  }
  if req.items.len() > MAX_BATCH_GENERATE_ITEMS {
    return notebook_error_json(
      StatusCode::BAD_REQUEST,
      &format!(
        "batch contains {} items, exceeding limit of {MAX_BATCH_GENERATE_ITEMS}",
        req.items.len()
      ),
    );
  }
  let model = match resolve_notebook_model(req.model.as_deref()) {
    Ok(m) => m,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let per_run_cost_limit_usd = match split_batch_cost_limit(req.cost_limit_usd, req.items.len()) {
    Ok(v) => v,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let ready_chunks = match load_local_ready_chunks(
    &notebooks,
    &tenant,
    id,
    req.source_ids.as_deref(),
    "generate from",
  ) {
    Ok(c) => c,
    Err((code, msg)) => return notebook_error_json(code, &msg),
  };

  let mut prepared_items = Vec::with_capacity(req.items.len());
  for item in req.items {
    match prepare_local_generator_item(&item.kind, item.params, &ready_chunks, &model).await {
      Ok(p) => prepared_items.push(p),
      Err((code, msg)) => return notebook_error_json(code, &msg),
    }
  }

  let mut runs = Vec::with_capacity(prepared_items.len());
  for prepared in prepared_items {
    let (run_id, kind) = match dispatch_prepared_local_generator(
      &engine,
      &notebooks,
      &tenant,
      id,
      prepared,
      &model,
      per_run_cost_limit_usd,
    )
    .await
    {
      Ok(pair) => pair,
      Err(msg) => return notebook_error_json(StatusCode::INTERNAL_SERVER_ERROR, &msg),
    };
    runs.push(json!({
      "run_id": run_id,
      "kind": kind,
      "status": "queued",
    }));
  }

  let mut out = serde_json::Map::new();
  out.insert("notebook_id".to_string(), json!(id));
  out.insert("runs".to_string(), Value::Array(runs));
  if let Some(per_run) = per_run_cost_limit_usd {
    out.insert("per_run_cost_limit_usd".to_string(), json!(per_run));
  }
  (StatusCode::OK, Json(Value::Object(out))).into_response()
}

fn sanitize_local_studio_plan_reply(
  raw_reply: &str,
  max_items: usize,
) -> Result<(String, Vec<Value>), String> {
  let (rationale, items) = sanitize_studio_plan_reply_raw(raw_reply, max_items, |_, _| true)?;
  let json_items = items
    .into_iter()
    .map(|item| serde_json::to_value(item).unwrap_or_else(|_| json!({})))
    .collect();
  Ok((rationale, json_items))
}

async fn plan_notebook(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<PlanNotebookRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks().clone();
  let goal = req.goal.trim().to_string();
  if goal.is_empty() {
    let has_nb = notebooks
      .lock_entries()
      .get(&id)
      .is_some_and(|e| e.notebook.tenant_id == tenant);
    if !has_nb {
      return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
    }
    return notebook_error_json(StatusCode::BAD_REQUEST, "goal must not be empty");
  }
  let max_items = req
    .max_items
    .unwrap_or(DEFAULT_PLAN_MAX_ITEMS)
    .clamp(1, MAX_BATCH_GENERATE_ITEMS);
  let model = match resolve_notebook_model(req.model.as_deref()) {
    Ok(m) => m,
    Err(msg) => return notebook_error_json(StatusCode::BAD_REQUEST, &msg),
  };
  let ready_chunks = match load_local_ready_chunks(
    &notebooks,
    &tenant,
    id,
    req.source_ids.as_deref(),
    "plan from",
  ) {
    Ok(c) => c,
    Err((code, msg)) => return notebook_error_json(code, &msg),
  };
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

  if let Err(err) = yanshi_llm::Yanshi::ensure_init().await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("initializing model registry: {err}"),
    );
  }
  let response = match yanshi_llm::Yanshi::model(&model)
    .system(
      "You are a Content Studio planner. Select appropriate deliverable kinds from the closed catalogue and output strict JSON.",
    )
    .prompt(&prompt)
    .json_schema("notebook_studio_plan", schema)
    .execute_full()
    .await
  {
    Ok(r) => r,
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("LLM execution failed: {err}"),
      );
    }
  };

  let (rationale, items) = match sanitize_local_studio_plan_reply(&response.content, max_items) {
    Ok(pair) => pair,
    Err(err) => {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("parsing studio plan: {err}"),
      );
    }
  };

  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "goal": goal,
      "model": model,
      "rationale": rationale,
      "items": items,
      "used_chunk_count": used_chunk_count,
    })),
  )
    .into_response()
}

async fn select_chunks_for_generate(
  ready: Vec<LocalSourceChunk>,
  query_hint: Option<&str>,
  budget: usize,
) -> Vec<LocalSourceChunk> {
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
struct AttachNotebookRunRequest {
  run_id: Uuid,
  #[serde(default)]
  kind: Option<String>,
}

async fn attach_notebook_run_artifacts(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath(id): RoutePath<Uuid>,
  Json(req): Json<AttachNotebookRunRequest>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let has_nb = engine
    .notebooks()
    .lock_entries()
    .get(&id)
    .is_some_and(|e| e.notebook.tenant_id == tenant);
  if !has_nb {
    return notebook_error_json(StatusCode::NOT_FOUND, &format!("notebook {id} not found"));
  }

  let completed = match engine.get_completed_run_for_attach(&tenant, req.run_id) {
    Ok(found) => found,
    Err((code, msg)) => return notebook_error_json(code, &msg),
  };

  let resolved_kind = req
    .kind
    .as_deref()
    .map(str::trim)
    .filter(|k| !k.is_empty())
    .map(str::to_string)
    .or_else(|| {
      completed
        .deliverable
        .as_ref()
        .and_then(|d| d.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
    })
    .or(completed.notebook_kind)
    .or_else(|| {
      yanshi_config::executor::parse_workflow_definition(&completed.record.workflow)
        .ok()
        .map(|d| d.name)
        .filter(|n| !n.trim().is_empty())
    })
    .unwrap_or_else(|| "report".to_string());

  let attached_count = engine.notebooks().attach_run_artifacts(
    &tenant,
    id,
    &resolved_kind,
    req.run_id,
    &completed.artifacts,
    completed.deliverable.as_ref(),
  );
  let attached: Vec<LocalNotebookArtifact> = engine
    .notebooks()
    .lock_entries()
    .get(&id)
    .map(|entry| {
      entry
        .artifacts
        .iter()
        .filter(|a| a.run_id == req.run_id)
        .cloned()
        .collect()
    })
    .unwrap_or_default();

  (
    StatusCode::OK,
    Json(json!({
      "notebook_id": id,
      "run_id": req.run_id,
      "kind": resolved_kind,
      "attached_count": attached_count,
      "artifacts": attached,
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

async fn delete_notebook_artifact(
  State(engine): State<LocalEngineState>,
  headers: HeaderMap,
  RoutePath((notebook_id, artifact_id)): RoutePath<(Uuid, Uuid)>,
) -> Response {
  let tenant = extract_tenant(&headers);
  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let bundle = {
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
    PersistedNotebookBundle::from_entry(entry)
  };
  let _ = write_notebook_bundle_async(nb_dir, bundle).await;
  StatusCode::NO_CONTENT.into_response()
}

// ── Portable bundle export / import & deliverables `.zip` (AD2) ─────────

fn render_local_notebook_markdown_brief(
  title: &str,
  sources: &[LocalSource],
  chunks: &[LocalSourceChunk],
  artifacts: &[LocalNotebookArtifact],
  messages: &[LocalNotebookMessage],
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
      run_id: a.run_id,
      kind: a.kind.as_str(),
      name: a.name.as_str(),
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
        raw_text: None,
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

    let exported_messages: Vec<ExportedNotebookMessage> = entry
      .messages
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

    let markdown_brief = render_local_notebook_markdown_brief(
      &entry.notebook.title,
      &entry.sources,
      &ready_chunks,
      &entry.artifacts,
      &entry.messages,
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
      messages: exported_messages,
      markdown_brief,
    }
  };
  let mut bundle = bundle;
  let nb_dir = engine.notebooks().notebook_dir(id);
  for src in &mut bundle.sources {
    if let Some(source_id) = src.id {
      src.raw_text = read_source_raw_text_async(nb_dir.clone(), source_id).await;
    }
  }
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
  let mut raw_texts_by_source: HashMap<Uuid, String> = HashMap::new();
  let mut chunks_by_source: HashMap<Uuid, Vec<LocalSourceChunk>> = HashMap::new();
  let mut source_id_map: HashMap<Uuid, Uuid> = HashMap::new();
  let mut chunk_id_map: HashMap<String, String> = HashMap::new();
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
    if let Some(old_id) = src.id {
      source_id_map.insert(old_id, source_id);
    }
    if kind == "text"
      && let Some(raw_t) = src
        .raw_text
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string)
    {
      if raw_t.len() > MAX_SOURCE_BYTES {
        return notebook_error_json(
          StatusCode::PAYLOAD_TOO_LARGE,
          &format!("imported raw_text exceeds {MAX_SOURCE_BYTES} bytes"),
        );
      }
      raw_texts_by_source.insert(source_id, raw_t);
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

    let mut ordered_raw: Vec<(i32, Option<String>, String, Value)> =
      Vec::with_capacity(src.chunks.len());
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
      let old_cid = chunk
        .chunk_id
        .as_deref()
        .map(str::trim)
        .filter(|c| !c.is_empty())
        .map(str::to_string)
        .or_else(|| src.id.map(|oid| format!("{oid}:{}", chunk.ordinal)));
      ordered_raw.push((chunk.ordinal, old_cid, content, meta));
    }
    ordered_raw.sort_by_key(|(ord, _, _, _)| *ord);

    let local_chunks: Vec<LocalSourceChunk> = ordered_raw
      .into_iter()
      .enumerate()
      .map(|(pos, (_, old_cid, content, metadata))| {
        let ordinal = i32::try_from(pos).unwrap_or(0);
        let new_cid = format!("{source_id}:{ordinal}");
        if let Some(ocid) = old_cid {
          chunk_id_map.insert(ocid, new_cid.clone());
        }
        LocalSourceChunk {
          chunk_id: new_cid,
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

  let mut messages: Vec<LocalNotebookMessage> = Vec::new();
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
    let raw_cites = if msg.citations.is_array() {
      &msg.citations
    } else {
      &Value::Array(Vec::new())
    };
    let citations = remap_imported_json_refs(raw_cites, &source_id_map, &chunk_id_map);
    messages.push(LocalNotebookMessage {
      id: Uuid::new_v4(),
      notebook_id,
      tenant_id: tenant.clone(),
      question: q.to_string(),
      answer: a.to_string(),
      model,
      citations,
      used_chunk_count: msg.used_chunk_count.max(0),
      created_at: msg.created_at.unwrap_or(now),
    });
  }
  let imported_messages = messages.len();

  let runs_with_text_sibling: HashSet<Uuid> = bundle
    .deliverables
    .iter()
    .take(200)
    .filter(|d| is_synthesizable_text_deliverable(&d.name, d.mime_type.as_deref()))
    .map(|d| d.run_id)
    .collect();
  let mut imported_run_map: HashMap<Uuid, (Uuid, PathBuf, String, Vec<LocalArtifactSummary>)> =
    HashMap::new();
  let mut fallback_emitted_runs: HashSet<Uuid> = HashSet::new();
  let mut artifacts: Vec<LocalNotebookArtifact> = Vec::new();

  for (idx, deliv) in bundle.deliverables.iter().take(200).enumerate() {
    let raw_kind = deliv.kind.trim();
    if raw_kind.is_empty() {
      continue;
    }
    let safe_kind = sanitize_zip_segment(raw_kind, "deliverable");
    let kind_clean = safe_kind.as_str();
    let is_text = is_synthesizable_text_deliverable(&deliv.name, deliv.mime_type.as_deref());
    if !is_text
      && (runs_with_text_sibling.contains(&deliv.run_id)
        || !fallback_emitted_runs.insert(deliv.run_id))
    {
      continue;
    }
    let (new_run_id, run_dir) =
      if let Some((existing_id, existing_dir, _, _)) = imported_run_map.get(&deliv.run_id) {
        (*existing_id, existing_dir.clone())
      } else {
        let fresh_id = Uuid::new_v4();
        let dir = engine.base_dir().join(fresh_id.to_string());
        imported_run_map.insert(
          deliv.run_id,
          (fresh_id, dir.clone(), kind_clean.to_string(), Vec::new()),
        );
        (fresh_id, dir)
      };

    let remapped_deliverable = deliv
      .deliverable
      .as_ref()
      .map(|val| remap_imported_json_refs(val, &source_id_map, &chunk_id_map));
    let (file_name, mime_type, file_bytes) = synthesize_imported_deliverable_bytes(
      kind_clean,
      &deliv.name,
      deliv.mime_type.as_deref(),
      remapped_deliverable.as_ref(),
    );
    let node_dir = run_dir.join("artifacts").join("imported");
    if let Err(err) = tokio::fs::create_dir_all(&node_dir).await {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("creating imported deliverable dir: {err}"),
      );
    }
    let disk_name = format!("{}_{}", idx + 1, file_name);
    let file_path = node_dir.join(&disk_name);
    if let Err(err) = tokio::fs::write(&file_path, &file_bytes).await {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("writing imported deliverable file: {err}"),
      );
    }
    let sha = format!("{:x}", Sha256::digest(&file_bytes));
    let art_id = Uuid::new_v4();
    let created_at = deliv.created_at;
    let summary = LocalArtifactSummary {
      id: art_id,
      node_id: "imported".to_string(),
      name: file_name.clone(),
      mime_type: mime_type.clone(),
      size_bytes: file_bytes.len() as u64,
      sha256: Some(sha.clone()),
      created_at,
      file_path: file_path.clone(),
    };
    if let Some((_, _, _, list)) = imported_run_map.get_mut(&deliv.run_id) {
      list.push(summary);
    }
    artifacts.push(LocalNotebookArtifact {
      kind: kind_clean.to_string(),
      deliverable: remapped_deliverable,
      id: art_id,
      run_id: new_run_id,
      node_id: "imported".to_string(),
      name: file_name,
      path_or_url: file_path.display().to_string(),
      mime_type: Some(mime_type),
      created_at,
      tenant_id: tenant.clone(),
      size_bytes: i64::try_from(file_bytes.len()).ok(),
      sha256: Some(sha),
    });
  }
  for (fresh_id, dir, kind_str, summaries) in imported_run_map.into_values() {
    engine.register_imported_run(crate::local_engine::ImportedRunRegistration {
      run_id: fresh_id,
      tenant_id: tenant.clone(),
      workflow: format!("@imported:{kind_str}"),
      run_dir: dir,
      notebook_id,
      notebook_kind: kind_str,
      artifacts: summaries,
    });
  }
  let imported_deliverables = artifacts.len();

  let imported_sources = sources.len();
  let entry = NotebookEntry {
    notebook: notebook.clone(),
    sources,
    chunks_by_source,
    artifacts,
    messages,
  };

  let notebooks = engine.notebooks();
  let nb_dir = notebooks.notebook_dir(notebook_id);
  let persisted = PersistedNotebookBundle::from_entry(&entry);
  if let Err(err) = write_notebook_bundle_async(nb_dir.clone(), persisted).await {
    return notebook_error_json(
      StatusCode::INTERNAL_SERVER_ERROR,
      &format!("failed to persist imported notebook: {err}"),
    );
  }
  for (sid, raw_text) in raw_texts_by_source {
    if let Err(err) = write_source_raw_text_async(nb_dir.clone(), sid, raw_text).await {
      return notebook_error_json(
        StatusCode::INTERNAL_SERVER_ERROR,
        &format!("failed to persist imported source text: {err}"),
      );
    }
  }
  for (sid, clist) in &entry.chunks_by_source {
    if let Err(err) = write_source_chunks_async(nb_dir.clone(), *sid, clist.clone()).await {
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
      "imported_deliverables": imported_deliverables,
      "imported_messages": imported_messages,
    })),
  )
    .into_response()
}

fn pack_local_notebook_artifacts_zip_blocking(
  notebook_id: Uuid,
  notebook_title: String,
  brief_markdown: String,
  artifacts: Vec<LocalNotebookArtifact>,
) -> std::io::Result<Option<(std::fs::File, u64)>> {
  let items: Vec<ZipArtifactItem> = artifacts
    .into_iter()
    .map(|item| ZipArtifactItem {
      artifact_id: item.id,
      run_id: item.run_id,
      kind: item.kind,
      name: item.name,
      path_or_url: item.path_or_url,
      mime_type: item.mime_type,
      deliverable: item.deliverable,
    })
    .collect();
  pack_shared_notebook_artifacts_zip_blocking(notebook_id, notebook_title, brief_markdown, items)
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
      &entry.messages,
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
    assert!(
      imported["imported_deliverables"]
        .as_u64()
        .is_some_and(|n| n >= 1),
      "desktop import must restore deliverables: {imported}"
    );
    let imported_nb_id = imported["notebook"]["id"]
      .as_str()
      .expect("imported desktop id");
    let imported_arts: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{imported_nb_id}/artifacts"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("list imported desktop artifacts")
      .json()
      .await
      .expect("imported desktop artifacts json");
    let imported_art0 = &imported_arts["artifacts"][0];
    let remapped_cid = imported_art0["deliverable"]["content"]["rows"][0]["chunk_id"]
      .as_str()
      .expect("remapped desktop chunk_id");
    assert_ne!(
      remapped_cid, csv_chunk_id,
      "imported deliverable chunk_id must remap onto cloned source UUID"
    );
    let synth_run_id = imported_art0["run_id"]
      .as_str()
      .expect("synthesized run_id");
    let synth_file_id = imported_art0["id"]
      .as_str()
      .expect("synthesized artifact id");
    let synth_dl = client
      .get(format!(
        "{restarted_url}/v1/runs/{synth_run_id}/artifacts/{synth_file_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("download synthesized desktop artifact");
    assert_eq!(synth_dl.status(), StatusCode::OK);
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

    // AE2: `GET /v1/notebooks/:id/sources/:source_id/chunks` + `plan` + `generate-batch` parity.
    let cross_src_chunks = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources/{csv_id}/chunks"
      ))
      .header("X-Yanshi-Tenant", "studio-b")
      .send()
      .await
      .expect("cross-tenant source chunks");
    assert_eq!(cross_src_chunks.status(), StatusCode::NOT_FOUND);

    let src_chunks_doc: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources/{csv_id}/chunks"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("owner source chunks")
      .json()
      .await
      .expect("source chunks json");
    assert_eq!(src_chunks_doc["source"]["id"], csv_id);
    assert!(
      src_chunks_doc["chunks"]
        .as_array()
        .is_some_and(|arr| !arr.is_empty() && arr[0]["ordinal"] == 0),
      "desktop source chunks must list ordered passages: {src_chunks_doc}"
    );

    let desktop_plan_reply = json!({
      "rationale": "Flashcards and quiz on tidal ranges.",
      "items": [
        {
          "kind": "flashcards",
          "reason": "Key basin facts",
          "parameters": [{"key": "count", "value": "4"}, {"key": "chunks", "value": "drop"}]
        },
        {
          "kind": "unknown_kind",
          "reason": "Dropped",
          "parameters": []
        },
        {
          "kind": "quiz",
          "reason": "Self-check",
          "parameters": [{"key": "count", "value": "3"}]
        }
      ]
    })
    .to_string();
    let desktop_study_reply = json!({
      "title": "Desktop Tidal Study",
      "items": [{
        "front": "Peak range in Bay of Fundy?",
        "back": "16.3 m.",
        "question": "Peak range in Bay of Fundy?",
        "options": ["4.0 m", "16.3 m", "1.2 m"],
        "answer": 1,
        "explanation": "CSV records 16.3 m.",
        "chunk_id": csv_chunk_id
      }]
    })
    .to_string();
    write_models(vec![
      desktop_plan_reply,
      desktop_study_reply.clone(),
      desktop_study_reply,
    ]);
    yanshi_llm::Yanshi::init_with_config(models_cfg.to_str().expect("utf8"))
      .await
      .expect("reinit desktop plan + batch mock");

    let plan_doc: Value = client
      .post(format!("{restarted_url}/v1/notebooks/{notebook_id}/plan"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "goal": "Create flashcards and a quiz on Bay of Fundy",
        "model": model,
        "source_ids": [csv_id]
      }))
      .send()
      .await
      .expect("plan desktop notebook")
      .json()
      .await
      .expect("plan desktop json");
    let desktop_plan_items = plan_doc["items"].as_array().expect("desktop plan items");
    assert_eq!(desktop_plan_items.len(), 2, "{plan_doc}");
    assert_eq!(desktop_plan_items[0]["kind"], "flashcards");
    assert_eq!(desktop_plan_items[0]["params"]["count"], 4);

    let batch_doc: Value = client
      .post(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/generate-batch"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "model": model,
        "cost_limit_usd": 3.0,
        "source_ids": [csv_id],
        "items": desktop_plan_items
      }))
      .send()
      .await
      .expect("desktop generate-batch")
      .json()
      .await
      .expect("desktop generate-batch json");
    assert_eq!(batch_doc["per_run_cost_limit_usd"], 1.5);
    let batch_runs = batch_doc["runs"].as_array().expect("desktop batch runs");
    assert_eq!(batch_runs.len(), 2);
    for br in batch_runs {
      let rid = br["run_id"].as_str().expect("batch run_id");
      for _ in 0..120 {
        let rd: Value = client
          .get(format!("{restarted_url}/v1/runs/{rid}"))
          .header("X-Yanshi-Tenant", "studio-a")
          .send()
          .await
          .expect("poll desktop batch run")
          .json()
          .await
          .expect("poll json");
        let st = rd["status"].as_str().unwrap_or_default();
        if matches!(st, "succeeded" | "failed" | "cancelled") {
          assert_eq!(st, "succeeded", "desktop batch run failed: {rd}");
          break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
      }
    }
    let after_batch_arts: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/artifacts"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("list desktop artifacts after batch")
      .json()
      .await
      .expect("artifacts json");
    let kinds_attached: Vec<&str> = after_batch_arts["artifacts"]
      .as_array()
      .expect("artifacts")
      .iter()
      .filter_map(|a| a["kind"].as_str())
      .collect();
    assert!(kinds_attached.contains(&"flashcards"), "{kinds_attached:?}");
    assert!(kinds_attached.contains(&"quiz"), "{kinds_attached:?}");

    // AF3: persisted `/ask` messages survive desktop restart, can be cleared,
    // and inline text sources support `PATCH` rename + re-chunking.
    let reloaded_msgs: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/messages"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("list desktop messages")
      .json()
      .await
      .expect("messages json");
    assert!(
      reloaded_msgs["messages"]
        .as_array()
        .is_some_and(|arr| !arr.is_empty()),
      "desktop messages must survive restart: {reloaded_msgs}"
    );
    let cleared_msgs: Value = client
      .delete(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/messages"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("clear desktop messages")
      .json()
      .await
      .expect("clear messages json");
    assert!(cleared_msgs["deleted"].as_u64().is_some_and(|n| n >= 1));

    let note_src: Value = client
      .post(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "text",
        "title": "Draft Note",
        "text": "Initial draft note text."
      }))
      .send()
      .await
      .expect("create text note")
      .json()
      .await
      .expect("note json");
    let note_id = note_src["id"].as_str().expect("note id").to_string();
    for _ in 0..40 {
      let detail: Value = client
        .get(format!("{restarted_url}/v1/notebooks/{notebook_id}"))
        .header("X-Yanshi-Tenant", "studio-a")
        .send()
        .await
        .expect("poll note")
        .json()
        .await
        .expect("poll note json");
      if detail["sources"]
        .as_array()
        .and_then(|arr| arr.iter().find(|s| s["id"] == note_id))
        .and_then(|s| s["status"].as_str())
        == Some("ready")
      {
        break;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let patched_note: Value = client
      .patch(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources/{note_id}"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "title": "Final Field Memo",
        "text": "BorealSurgeResonance amplifies winter spring tides."
      }))
      .send()
      .await
      .expect("patch note")
      .json()
      .await
      .expect("patched note json");
    assert_eq!(patched_note["title"], "Final Field Memo");
    let surge_search: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/search?q=BorealSurgeResonance"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search patched note")
      .json()
      .await
      .expect("search patched json");
    assert_eq!(surge_search["chunks"][0]["source_id"], json!(note_id));
    let note_chunks_doc: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources/{note_id}/chunks"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("note chunks")
      .json()
      .await
      .expect("note chunks json");
    assert_eq!(
      note_chunks_doc["raw_text"],
      "BorealSurgeResonance amplifies winter spring tides."
    );

    // AG3: `.srt` subtitle ingestion + standalone Studio run attach (`POST /v1/notebooks/:id/artifacts`)
    // + `local_engine` parity (`?status=bogus` -> 400, `:cancel` on terminal -> `cancelled: false`,
    // UTF-8 `?filter=中文=1` -> 400 without panic).
    let srt_payload =
      b"1\n00:00:01,000 --> 00:00:04,500\n<i>EquinoctialSpringSurge</i> peaks at 16.8 meters.\n";
    let srt_src: Value = client
      .post(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/sources"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "kind": "file",
        "filename": "lecture.srt",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(srt_payload),
      }))
      .send()
      .await
      .expect("upload srt source")
      .json()
      .await
      .expect("srt source json");
    let srt_id = srt_src["id"].as_str().expect("srt id").to_string();
    for _ in 0..40 {
      let detail: Value = client
        .get(format!("{restarted_url}/v1/notebooks/{notebook_id}"))
        .header("X-Yanshi-Tenant", "studio-a")
        .send()
        .await
        .expect("poll srt")
        .json()
        .await
        .expect("poll srt json");
      if detail["sources"]
        .as_array()
        .and_then(|arr| arr.iter().find(|s| s["id"] == srt_id))
        .and_then(|s| s["status"].as_str())
        == Some("ready")
      {
        break;
      }
      tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let srt_search: Value = client
      .get(format!(
        "{restarted_url}/v1/notebooks/{notebook_id}/search?q=EquinoctialSpringSurge"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("search srt")
      .json()
      .await
      .expect("search srt json");
    assert_eq!(srt_search["chunks"].as_array().map(Vec::len), Some(1));

    let standalone_run_resp: Value = client
      .post(format!("{restarted_url}/v1/runs"))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "workflow": "name: standalone-brief\nnodes:\n  - id: tpl\n    type: template\n    parameters:\n      template: \"# Standalone Studio Brief\\n\\nSynthesized outside notebook.\"\n  - id: brief_md\n    type: render_markdown\n    dependencies: [\"tpl\"]\n    input_mapping:\n      markdown: \"{{ nodes.tpl.outputs.output }}\"\n    parameters:\n      format: md\n"
      }))
      .send()
      .await
      .expect("create standalone studio run")
      .json()
      .await
      .expect("standalone studio run json");
    let standalone_run_id = standalone_run_resp["run_id"]
      .as_str()
      .or_else(|| standalone_run_resp["id"].as_str())
      .expect("standalone run id")
      .to_string();
    for _ in 0..80 {
      let rd: Value = client
        .get(format!("{restarted_url}/v1/runs/{standalone_run_id}"))
        .header("X-Yanshi-Tenant", "studio-a")
        .send()
        .await
        .expect("poll standalone run")
        .json()
        .await
        .expect("poll standalone run json");
      let st = rd["status"].as_str().unwrap_or_default();
      if matches!(st, "succeeded" | "failed" | "cancelled") {
        assert_eq!(st, "succeeded", "standalone run failed: {rd}");
        break;
      }
      tokio::time::sleep(Duration::from_millis(25)).await;
    }
    let attached_resp: Value = client
      .post(format!(
        "{restarted_url}/v1/notebooks/{imported_nb_id}/artifacts"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .json(&json!({
        "run_id": standalone_run_id,
        "kind": "briefing"
      }))
      .send()
      .await
      .expect("attach standalone run")
      .json()
      .await
      .expect("attach standalone run json");
    assert!(
      attached_resp["attached_count"]
        .as_u64()
        .is_some_and(|n| n >= 1),
      "attaching standalone studio run must register artifacts: {attached_resp}"
    );

    let bogus_runs = client
      .get(format!("{restarted_url}/v1/runs?status=bogus"))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("bogus status query");
    assert_eq!(bogus_runs.status(), StatusCode::BAD_REQUEST);

    let terminal_cancel: Value = client
      .post(format!(
        "{restarted_url}/v1/runs/{standalone_run_id}:cancel"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("cancel terminal run")
      .json()
      .await
      .expect("cancel terminal run json");
    assert_eq!(terminal_cancel["cancelled"], false);
    assert_eq!(terminal_cancel["id"], standalone_run_id);
    assert_eq!(terminal_cancel["run"]["id"], standalone_run_id);

    let utf8_filter = client
      .get(format!(
        "{restarted_url}/v1/runs/{standalone_run_id}/events/history?filter=%E4%B8%AD%E6%96%87=1"
      ))
      .header("X-Yanshi-Tenant", "studio-a")
      .send()
      .await
      .expect("utf8 filter query");
    assert_eq!(utf8_filter.status(), StatusCode::BAD_REQUEST);

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

  #[tokio::test]
  async fn local_notebook_ingests_png_image_via_deepseek_ocr_and_rejects_bad_header() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let tmp = tempfile::tempdir().expect("tempdir");
    let base_url = crate::local_engine::start_loopback_engine(Some(tmp.path().to_path_buf()))
      .await
      .expect("start loopback engine");
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no-proxy client");

    let created: Value = client
      .post(format!("{base_url}/v1/notebooks"))
      .json(&json!({ "title": "Image OCR Notebook" }))
      .send()
      .await
      .expect("create notebook")
      .json()
      .await
      .expect("notebook json");
    let notebook_id = created["id"].as_str().expect("id").to_string();

    // Corrupted `.png` without PNG magic bytes fails immediately with 400.
    let bad_resp = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .json(&json!({
        "kind": "file",
        "filename": "broken.png",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(b"not-a-png")
      }))
      .send()
      .await
      .expect("bad png req");
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let ocr_addr = listener.local_addr().expect("addr");
    let ocr_server = tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 8192];
      let _ = socket.read(&mut buf).await.expect("read");
      let body = r#"{"id":"ocr-local-1","object":"ocr.result","created":1762790000,"purpose":"rich_layout_recognition","page_count":1,"pages":[{"index":0,"text":"Kalman Filter State Estimation Equations","finish_reason":"stop"}],"usage":{"input_tokens":180,"output_tokens":9,"total_tokens":189}}"#;
      let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      );
      socket.write_all(resp.as_bytes()).await.expect("write");
    });

    let prev_key = std::env::var("DEEPSEEK_API_KEY").ok();
    let prev_base = std::env::var("DEEPSEEK_BASE_URL").ok();
    // SAFETY: local unit test restores env vars once ingestion finishes.
    unsafe {
      std::env::set_var("DEEPSEEK_API_KEY", "sk-tauri-ocr");
      std::env::set_var("DEEPSEEK_BASE_URL", format!("http://{ocr_addr}/v1"));
    }

    let ok_src: Value = client
      .post(format!("{base_url}/v1/notebooks/{notebook_id}/sources"))
      .json(&json!({
        "kind": "file",
        "filename": "slide.png",
        "content_base64": base64::engine::general_purpose::STANDARD.encode(b"\x89PNG\r\n\x1a\nslide")
      }))
      .send()
      .await
      .expect("upload png")
      .json()
      .await
      .expect("src json");
    let src_id = ok_src["id"].as_str().expect("src id").to_string();

    for _ in 0..40 {
      let nb: Value = client
        .get(format!("{base_url}/v1/notebooks/{notebook_id}"))
        .send()
        .await
        .expect("get nb")
        .json()
        .await
        .expect("nb json");
      let status = nb["sources"]
        .as_array()
        .and_then(|arr| arr.iter().find(|s| s["id"].as_str() == Some(&src_id)))
        .and_then(|s| s["status"].as_str())
        .unwrap_or("");
      if status == "ready" {
        break;
      }
      assert_ne!(status, "failed", "png OCR ingestion failed: {nb}");
      tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }

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

    let search_res: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{notebook_id}/search?q=Kalman"
      ))
      .send()
      .await
      .expect("search")
      .json()
      .await
      .expect("search json");
    assert_eq!(search_res["chunks"].as_array().map(Vec::len), Some(1));
    ocr_server.await.expect("ocr server");
  }

  #[tokio::test]
  async fn hydrate_migrates_legacy_inline_raw_texts_to_sidecar_files() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let nb_id = Uuid::new_v4();
    let src_id = Uuid::new_v4();
    let nb_dir = notebooks_root_dir(tmp.path()).join(nb_id.to_string());
    let now = Utc::now();
    let mut legacy_raw = HashMap::new();
    legacy_raw.insert(src_id, "legacy inline note body\nline 2".to_string());
    let legacy_bundle = PersistedNotebookBundle {
      notebook: LocalNotebook {
        id: nb_id,
        tenant_id: "default".to_string(),
        title: "Legacy Migration NB".to_string(),
        created_at: now,
        updated_at: now,
      },
      sources: vec![LocalSource {
        id: src_id,
        notebook_id: nb_id,
        tenant_id: "default".to_string(),
        kind: "text".to_string(),
        title: "Old Note".to_string(),
        uri: None,
        status: "ready".to_string(),
        error: None,
        sha256: None,
        created_at: now,
        updated_at: now,
      }],
      raw_texts: legacy_raw,
      artifacts: Vec::new(),
      messages: Vec::new(),
    };
    write_persisted_notebook_bundle_sync(&nb_dir, &legacy_bundle).expect("write legacy bundle");

    let base_url = crate::local_engine::start_loopback_engine(Some(tmp.path().to_path_buf()))
      .await
      .expect("start engine");

    // 1. Sidecar raw text file must exist and notebook.json must no longer contain inline raw_texts.
    let sidecar = std::fs::read_to_string(raw_text_path(&nb_dir, src_id)).expect("sidecar raw");
    assert_eq!(sidecar, "legacy inline note body\nline 2");
    let rewritten = std::fs::read_to_string(nb_dir.join("notebook.json")).expect("notebook.json");
    assert!(
      !rewritten.contains("legacy inline note body"),
      "notebook.json should no longer inline raw_texts: {rewritten}"
    );

    // 2. HTTP /sources/:sid/chunks must transparently return the migrated raw_text.
    let client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("no-proxy client");
    let payload: Value = client
      .get(format!(
        "{base_url}/v1/notebooks/{nb_id}/sources/{src_id}/chunks"
      ))
      .send()
      .await
      .expect("chunks")
      .json()
      .await
      .expect("chunks json");
    assert_eq!(payload["raw_text"], "legacy inline note body\nline 2");
  }
}
