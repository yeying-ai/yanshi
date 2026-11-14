//! Run artifacts: registration at run end and the two download routes
//! (Y1.3, `docs/RFC_CONTENT_STUDIO.md` §4.2).
//!
//! Nodes write media under `<run_dir>/artifacts/<node_id>/` and put a
//! `FlowValue::File` in the state pool (Y1.1). When `flow_execute`
//! finishes, [`register_run_artifacts`] walks the final state pool and
//! records every such file in the `artifacts` table with its size and
//! SHA-256, so a client can list what a run produced and fetch it:
//!
//! ```text
//! GET /v1/runs/{id}/artifacts                → { run_id, artifacts: [ArtifactSummary] }
//! GET /v1/runs/{id}/artifacts/{artifact_id}  → the bytes, Content-Type from the stored MIME
//! ```
//!
//! Only files under the run base directory are registered. A `File`
//! value can also point at a sandbox-allowed path outside it (`file`
//! node `read_bytes`); serving those would turn the artifact route into
//! a read primitive over whatever the workflow's policy allowed, so they
//! are logged and skipped. Map items' outputs are nested JSON inside
//! their parent node's `results` and are not walked here — a later node
//! that surfaces such a file as its own `File` output gets registered.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

use axum::{
  Extension, Json,
  body::Body,
  extract::{Path as RoutePath, Query, State},
  http::{HeaderValue, header},
  response::{IntoResponse, Response},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt as _;
use uuid::Uuid;
use yanshi_core::async_node::AsyncNodeResult;
use yanshi_db::{ArtifactRepo, NewArtifact, Repositories, RunRepo};

use crate::{AppState, error::ApiError, tenant::TenantId};

pub use yanshi_core::run_context::{FileOutput, collect_file_outputs};

/// Size and hex SHA-256 of a file, streamed in 64 KiB reads so a large
/// video is never fully buffered.
async fn size_and_sha256(path: &Path) -> std::io::Result<(u64, String)> {
  let mut file = tokio::fs::File::open(path).await?;
  let mut hasher = Sha256::new();
  let mut buf = vec![0u8; 64 * 1024];
  let mut total: u64 = 0;
  loop {
    let n = file.read(&mut buf).await?;
    if n == 0 {
      break;
    }
    hasher.update(&buf[..n]);
    total += n as u64;
  }
  Ok((total, format!("{:x}", hasher.finalize())))
}

/// Register the run's `File` outputs in the `artifacts` table. A file
/// that cannot be read (deleted between the node and this pass) is
/// logged and skipped; a database failure is returned so the run is
/// marked failed rather than silently missing its deliverables.
pub async fn register_run_artifacts(
  repos: &Repositories,
  run_id: Uuid,
  tenant_id: &str,
  state: &HashMap<String, AsyncNodeResult>,
  base_dir: &Path,
) -> Result<usize, yanshi_db::DbError> {
  let mut registered = 0;
  for output in collect_file_outputs(state, base_dir) {
    let (size_bytes, sha256) = match size_and_sha256(&output.path).await {
      Ok(v) => v,
      Err(err) => {
        tracing::warn!(
          run_id = %run_id,
          node_id = %output.node_id,
          path = %output.path.display(),
          error = %err,
          "artifact file unreadable at run end; not registered"
        );
        continue;
      }
    };
    let name = output
      .path
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_else(|| output.output_key.clone());
    repos
      .artifacts
      .create(NewArtifact {
        id: Uuid::new_v4(),
        run_id,
        node_id: output.node_id,
        name,
        path_or_url: output.path.to_string_lossy().to_string(),
        mime_type: output.mime_type,
        tenant_id: Some(tenant_id.to_string()),
        size_bytes: Some(size_bytes as i64),
        sha256: Some(sha256),
      })
      .await?;
    registered += 1;
  }
  if registered > 0 {
    tracing::info!(run_id = %run_id, artifacts = registered, "run artifacts registered");
  }
  Ok(registered)
}

// ── Routes ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct ArtifactSummary {
  pub id: Uuid,
  pub node_id: String,
  pub name: String,
  pub mime_type: Option<String>,
  pub size_bytes: Option<i64>,
  pub sha256: Option<String>,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ListArtifactsResponse {
  pub run_id: Uuid,
  pub artifacts: Vec<ArtifactSummary>,
}

/// The run must exist under the caller's tenant; anything else is a 404
/// (never a 403, so tenants cannot probe each other's run ids).
async fn run_for_tenant(state: &AppState, id: Uuid, tenant: &TenantId) -> Result<(), ApiError> {
  let run = state
    .repos
    .runs
    .get(id)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("run {id} not found")))?;
  if run.tenant_id != tenant.as_str() {
    return Err(ApiError::NotFound(format!("run {id} not found")));
  }
  Ok(())
}

pub async fn list_run_artifacts(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  RoutePath(id): RoutePath<Uuid>,
) -> Result<Json<ListArtifactsResponse>, ApiError> {
  run_for_tenant(&state, id, &tenant).await?;
  let artifacts = state
    .repos
    .artifacts
    .list_for_run(id)
    .await?
    .into_iter()
    .map(|a| ArtifactSummary {
      id: a.id,
      node_id: a.node_id,
      name: a.name,
      mime_type: a.mime_type,
      size_bytes: a.size_bytes,
      sha256: a.sha256,
      created_at: a.created_at,
    })
    .collect();
  Ok(Json(ListArtifactsResponse {
    run_id: id,
    artifacts,
  }))
}

/// Stream the artifact's bytes. `Content-Type` is the MIME recorded at
/// registration (`application/octet-stream` when unknown); the file name
/// goes out as an attachment with any quote / control characters
/// stripped so it cannot break the header.
pub async fn download_run_artifact(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  RoutePath((id, artifact_id)): RoutePath<(Uuid, Uuid)>,
) -> Result<Response, ApiError> {
  run_for_tenant(&state, id, &tenant).await?;
  let artifact = state
    .repos
    .artifacts
    .get(artifact_id)
    .await?
    .filter(|a| a.run_id == id && a.tenant_id == tenant.as_str())
    .ok_or_else(|| ApiError::NotFound(format!("artifact {artifact_id} not found")))?;

  let path = PathBuf::from(&artifact.path_or_url);
  let file = tokio::fs::File::open(&path).await.map_err(|err| {
    tracing::warn!(artifact_id = %artifact_id, path = %path.display(), error = %err, "artifact file missing");
    ApiError::NotFound(format!(
      "artifact {artifact_id} is registered but its file is no longer on disk"
    ))
  })?;
  let len = file.metadata().await.map(|m| m.len()).ok();

  let mime = artifact
    .mime_type
    .as_deref()
    .filter(|m| !m.is_empty())
    .unwrap_or("application/octet-stream");
  let safe_name: String = artifact
    .name
    .chars()
    .filter(|c| !c.is_control() && *c != '"' && *c != '\\' && *c != '/')
    .collect();
  let disposition = format!("attachment; filename=\"{safe_name}\"");

  let mut response = Body::from_stream(tokio_util::io::ReaderStream::new(file)).into_response();
  let headers = response.headers_mut();
  headers.insert(
    header::CONTENT_TYPE,
    HeaderValue::from_str(mime)
      .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
  );
  if let Some(len) = len {
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
  }
  if let Ok(value) = HeaderValue::from_str(&disposition) {
    headers.insert(header::CONTENT_DISPOSITION, value);
  }
  Ok(response)
}

#[derive(Debug, Deserialize)]
pub struct PreviewFileQuery {
  /// Relative path inside `<run_dir>` or an absolute path already rooted under `<run_dir>`.
  pub path: String,
}

/// Infer a browser-renderable MIME type from a file extension for inline Studio previews.
pub fn infer_preview_mime(path: &Path) -> &'static str {
  let ext = path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("")
    .to_ascii_lowercase();
  match ext.as_str() {
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "webp" => "image/webp",
    "gif" => "image/gif",
    "svg" => "image/svg+xml",
    "wav" => "audio/wav",
    "mp3" => "audio/mpeg",
    "ogg" => "audio/ogg",
    "m4a" => "audio/mp4",
    "mp4" => "video/mp4",
    "webm" => "video/webm",
    "vtt" => "text/vtt; charset=utf-8",
    "srt" | "md" | "txt" | "log" | "yml" | "yaml" => "text/plain; charset=utf-8",
    "json" => "application/json; charset=utf-8",
    "html" | "htm" => "text/html; charset=utf-8",
    "pdf" => "application/pdf",
    "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    _ => "application/octet-stream",
  }
}

/// Resolve and confine `raw_path` strictly inside `run_dir`.
///
/// Rejects `..` components and out-of-root absolute paths lexically *before*
/// touching the target on disk so callers cannot probe host file existence,
/// then canonicalizes both `run_dir` and the target to defeat symlinks.
pub async fn resolve_preview_path(run_dir: &Path, raw_path: &str) -> Result<PathBuf, ApiError> {
  let trimmed = raw_path.trim();
  if trimmed.is_empty() {
    return Err(ApiError::BadRequest(
      "query parameter `path` must not be empty".into(),
    ));
  }

  let requested = PathBuf::from(trimmed);
  if requested
    .components()
    .any(|comp| matches!(comp, Component::ParentDir))
  {
    return Err(ApiError::BadRequest(
      "preview `path` must not contain parent directory ('..') segments".into(),
    ));
  }

  let canonical_run_dir = tokio::fs::canonicalize(run_dir)
    .await
    .map_err(|_| ApiError::NotFound("run directory does not exist on disk yet".into()))?;

  let candidate = if requested.is_absolute() {
    if !requested.starts_with(run_dir) && !requested.starts_with(&canonical_run_dir) {
      return Err(ApiError::BadRequest(
        "preview `path` must stay inside the run directory".into(),
      ));
    }
    requested
  } else {
    canonical_run_dir.join(&requested)
  };

  let canonical_target = tokio::fs::canonicalize(&candidate)
    .await
    .map_err(|_| ApiError::NotFound("preview file not found in run directory".into()))?;

  if !canonical_target.starts_with(&canonical_run_dir) {
    return Err(ApiError::BadRequest(
      "preview `path` escapes the run directory".into(),
    ));
  }

  let meta = tokio::fs::metadata(&canonical_target)
    .await
    .map_err(|_| ApiError::NotFound("preview file not found".into()))?;
  if !meta.is_file() {
    return Err(ApiError::BadRequest(
      "preview `path` must point to a regular file".into(),
    ));
  }

  Ok(canonical_target)
}

/// `GET /v1/runs/{id}/preview?path=...` — inline media/file preview for
/// mid-run Debug Mode inspection and nested Map outputs.
pub async fn preview_run_file(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  RoutePath(id): RoutePath<Uuid>,
  Query(query): Query<PreviewFileQuery>,
) -> Result<Response, ApiError> {
  let run = state
    .repos
    .runs
    .get(id)
    .await?
    .filter(|r| r.tenant_id == tenant.as_str())
    .ok_or_else(|| ApiError::NotFound(format!("run {id} not found")))?;

  let run_dir = run
    .run_dir
    .as_deref()
    .map(PathBuf::from)
    .unwrap_or_else(|| crate::runs::default_run_dir_for_id(id));

  let target = resolve_preview_path(&run_dir, &query.path).await?;
  let file = tokio::fs::File::open(&target)
    .await
    .map_err(|_| ApiError::NotFound("preview file could not be opened".into()))?;
  let len = file.metadata().await.map(|m| m.len()).ok();

  let mime = infer_preview_mime(&target);
  let safe_name: String = target
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "preview".to_string())
    .chars()
    .filter(|c| !c.is_control() && *c != '"' && *c != '\\' && *c != '/')
    .collect();
  let disposition = format!("inline; filename=\"{safe_name}\"");

  let mut response = Body::from_stream(tokio_util::io::ReaderStream::new(file)).into_response();
  let headers = response.headers_mut();
  headers.insert(
    header::CONTENT_TYPE,
    HeaderValue::from_str(mime)
      .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
  );
  if let Some(len) = len {
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(len));
  }
  if let Ok(value) = HeaderValue::from_str(&disposition) {
    headers.insert(header::CONTENT_DISPOSITION, value);
  }
  if mime.starts_with("text/html") {
    headers.insert(
      header::CONTENT_SECURITY_POLICY,
      HeaderValue::from_static("sandbox allow-scripts"),
    );
  }
  Ok(response)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn size_and_digest_match_the_file() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let path = dir.path().join("a.bin");
    std::fs::write(&path, b"hello artifact").expect("write");
    let (size, digest) = size_and_sha256(&path).await.expect("hash");
    assert_eq!(size, 14);
    assert_eq!(digest, format!("{:x}", Sha256::digest(b"hello artifact")),);
  }

  #[tokio::test]
  async fn resolve_preview_path_confines_to_run_dir_and_blocks_traversal() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let run_dir = dir.path().join("run-1");
    let nested = run_dir.join("artifacts").join("slides");
    std::fs::create_dir_all(&nested).expect("mkdir");
    let slide = nested.join("slide_01.png");
    std::fs::write(&slide, b"png").expect("write");

    let outside = dir.path().join("secret.txt");
    std::fs::write(&outside, b"secret").expect("write secret");

    // Relative path inside run_dir works.
    let resolved_rel = resolve_preview_path(&run_dir, "artifacts/slides/slide_01.png")
      .await
      .expect("relative preview");
    assert_eq!(
      resolved_rel,
      std::fs::canonicalize(&slide).expect("canonical slide")
    );

    // Absolute path inside run_dir works.
    let resolved_abs = resolve_preview_path(&run_dir, &slide.to_string_lossy())
      .await
      .expect("absolute preview");
    assert_eq!(resolved_abs, resolved_rel);

    // Parent traversal (`..`) is rejected with BadRequest.
    let err_parent = resolve_preview_path(&run_dir, "../secret.txt")
      .await
      .expect_err("parent traversal rejected");
    assert!(matches!(err_parent, ApiError::BadRequest(_)));

    // Out-of-root absolute path is rejected with BadRequest before touching target.
    let err_abs = resolve_preview_path(&run_dir, &outside.to_string_lossy())
      .await
      .expect_err("outside absolute path rejected");
    assert!(matches!(err_abs, ApiError::BadRequest(_)));

    #[cfg(unix)]
    {
      let link = nested.join("escape_link.txt");
      std::os::unix::fs::symlink(&outside, &link).expect("symlink");
      let err_link = resolve_preview_path(&run_dir, "artifacts/slides/escape_link.txt")
        .await
        .expect_err("symlink escape rejected");
      assert!(matches!(err_link, ApiError::BadRequest(_)));
    }
  }

  #[test]
  fn infer_preview_mime_covers_studio_media_types() {
    assert_eq!(infer_preview_mime(Path::new("slide.PNG")), "image/png");
    assert_eq!(infer_preview_mime(Path::new("voice.wav")), "audio/wav");
    assert_eq!(infer_preview_mime(Path::new("deck.mp4")), "video/mp4");
    assert_eq!(
      infer_preview_mime(Path::new("deck.html")),
      "text/html; charset=utf-8"
    );
    assert_eq!(
      infer_preview_mime(Path::new("deck.pptx")),
      "application/vnd.openxmlformats-officedocument.presentationml.presentation"
    );
  }
}
