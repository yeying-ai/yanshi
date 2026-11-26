//! Web UI mount for Yanshi Studio.
//!
//! Serves the Dioxus 0.7 WASM frontend (`yanshi-ui-dx/dist/public`, or the
//! directory specified by `YANSHI_UI_DIR` / `YANSHI_DX_UI_DIR`, or
//! `/usr/share/yanshi/ui`) at `/ui/*` under a strict zero-eval Content Security
//! Policy, and redirects legacy `/ui-dx/*` bookmarks to `/ui/*`.

use std::path::{Component, Path, PathBuf};

use axum::{
  Router,
  body::Body,
  extract::Path as RoutePath,
  http::{
    HeaderName, HeaderValue, StatusCode, Uri,
    header::{CACHE_CONTROL, CONTENT_TYPE},
  },
  response::{IntoResponse, Redirect, Response},
  routing::get,
};

use crate::AppState;

/// Preferred runtime override pointing at a built `yanshi-ui-dx/dist/public` bundle.
pub const UI_DIR_ENV: &str = "YANSHI_UI_DIR";

/// Backwards-compatible alias for [`UI_DIR_ENV`].
pub const DX_UI_DIR_ENV: &str = "YANSHI_DX_UI_DIR";

/// Strict CSP applied to Dioxus UI responses (`dioxus-shadcn` RFC 0080:
/// zero JS `eval` / zero inline script execution; only `'wasm-unsafe-eval'`).
pub const DX_UI_STRICT_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; connect-src 'self' http://127.0.0.1:* http://localhost:*; frame-src 'self' blob:; object-src 'none'; base-uri 'self'";

const UI_BUNDLE_MISSING_HTML: &str = r#"<!doctype html>
<html lang="en">
<head><meta charset="utf-8"><title>Yanshi Studio UI Not Built</title></head>
<body style="font-family:system-ui,sans-serif;max-width:44rem;margin:3rem auto;padding:0 1.25rem;line-height:1.6">
  <h1>Yanshi Studio UI bundle not found</h1>
  <p>The Dioxus 0.7 WASM Web UI has not been built on this host yet.</p>
  <p>Build it once with <code>./yanshi-ui-dx/scripts/build.sh</code> or set <code>YANSHI_UI_DIR</code> to a built <code>yanshi-ui-dx/dist/public</code> directory.</p>
</body>
</html>"#;

/// Static UI routes mounted by [`crate::create_router`].
///
/// Serves every `/ui/*` SPA path and static asset from the built Dioxus WASM
/// bundle and redirects legacy `/ui-dx/*` bookmarks to `/ui/*`.
pub fn ui_router() -> Router<AppState> {
  Router::new()
    .route("/", get(root_redirect))
    .route("/ui", get(index_html))
    .route("/ui/", get(index_html))
    .route("/ui/*asset_path", get(ui_asset))
    .route("/ui-dx", get(ui_dx_redirect_root))
    .route("/ui-dx/", get(ui_dx_redirect_root))
    .route("/ui-dx/*rest", get(ui_dx_redirect_subpath))
}

/// Redirect bare domain root (`/`) to the Studio workspace.
pub async fn root_redirect() -> Redirect {
  Redirect::temporary("/ui/studio")
}

/// Serve the Dioxus SPA shell (`index.html`), or `503 Service Unavailable` with
/// build instructions when no UI bundle is present on disk.
pub async fn index_html() -> Response {
  match resolve_dx_ui_dir() {
    Some(root) => serve_dx_ui_file(&root, Some("index.html")).await,
    None => ui_bundle_missing_response(),
  }
}

/// Build a cacheable static asset response.
pub fn asset_response(body: &'static str, content_type: &'static str) -> Response {
  (
    StatusCode::OK,
    [
      (CONTENT_TYPE, content_type),
      (CACHE_CONTROL, "public, max-age=3600"),
    ],
    Body::from(body),
  )
    .into_response()
}

fn ui_bundle_missing_response() -> Response {
  (
    StatusCode::SERVICE_UNAVAILABLE,
    [
      (CONTENT_TYPE, "text/html; charset=utf-8"),
      (CACHE_CONTROL, "no-store"),
    ],
    Body::from(UI_BUNDLE_MISSING_HTML),
  )
    .into_response()
}

fn has_dx_index(dir: &Path) -> bool {
  dir.is_dir() && dir.join("index.html").is_file()
}

fn resolve_dx_ui_dir() -> Option<PathBuf> {
  for env_key in [UI_DIR_ENV, DX_UI_DIR_ENV] {
    if let Ok(raw) = std::env::var(env_key) {
      let trimmed = raw.trim();
      if !trimmed.is_empty() {
        let candidate = PathBuf::from(trimmed);
        if has_dx_index(&candidate) {
          return Some(candidate);
        }
      }
    }
  }
  [
    PathBuf::from("yanshi-ui-dx/dist/public"),
    PathBuf::from("/usr/share/yanshi/ui"),
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../yanshi-ui-dx/dist/public"),
  ]
  .into_iter()
  .find(|candidate| has_dx_index(candidate))
}

fn sanitize_rel_asset_path(raw: &str) -> Option<PathBuf> {
  let mut clean = PathBuf::new();
  for component in Path::new(raw).components() {
    match component {
      Component::Normal(part) => clean.push(part),
      Component::CurDir => {}
      Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
    }
  }
  Some(clean)
}

fn dx_content_type_for_path(path: &Path) -> &'static str {
  match path.extension().and_then(|e| e.to_str()) {
    Some("html") => "text/html; charset=utf-8",
    Some("js") | Some("mjs") => "application/javascript; charset=utf-8",
    Some("css") => "text/css; charset=utf-8",
    Some("wasm") => "application/wasm",
    Some("svg") => "image/svg+xml",
    Some("png") => "image/png",
    Some("ico") => "image/x-icon",
    Some("json") => "application/json; charset=utf-8",
    _ => "application/octet-stream",
  }
}

async fn serve_dx_ui_file(root: &Path, rel_path: Option<&str>) -> Response {
  let Some(clean_rel) = sanitize_rel_asset_path(rel_path.unwrap_or("index.html")) else {
    return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
  };
  let target = if clean_rel.as_os_str().is_empty() {
    root.join("index.html")
  } else {
    let direct = root.join(&clean_rel);
    if direct.is_file() {
      direct
    } else if clean_rel.extension().is_none() {
      // SPA history-mode fallback for client routes under `/ui/*`.
      root.join("index.html")
    } else {
      return (StatusCode::NOT_FOUND, "Dioxus UI asset not found").into_response();
    }
  };

  let Ok(bytes) = tokio::fs::read(&target).await else {
    return (StatusCode::NOT_FOUND, "Dioxus UI file not readable").into_response();
  };
  let content_type = dx_content_type_for_path(&target);
  let cache_control = if target.extension().and_then(|e| e.to_str()) == Some("html") {
    "no-cache"
  } else {
    "public, max-age=3600"
  };

  let mut resp = (
    StatusCode::OK,
    [(CONTENT_TYPE, content_type), (CACHE_CONTROL, cache_control)],
    Body::from(bytes),
  )
    .into_response();
  resp.headers_mut().insert(
    HeaderName::from_static("content-security-policy"),
    HeaderValue::from_static(DX_UI_STRICT_CSP),
  );
  resp
}

async fn ui_asset(RoutePath(asset_path): RoutePath<String>) -> Response {
  match resolve_dx_ui_dir() {
    Some(root) => serve_dx_ui_file(&root, Some(&asset_path)).await,
    None => ui_bundle_missing_response(),
  }
}

fn redirect_with_query(path: &str, query: Option<&str>) -> Redirect {
  match query.filter(|q| !q.is_empty()) {
    Some(q) => Redirect::temporary(&format!("{path}?{q}")),
    None => Redirect::temporary(path),
  }
}

async fn ui_dx_redirect_root(uri: Uri) -> Redirect {
  redirect_with_query("/ui/studio", uri.query())
}

async fn ui_dx_redirect_subpath(RoutePath(rest): RoutePath<String>, uri: Uri) -> Response {
  let Some(clean) = sanitize_rel_asset_path(rest.trim_start_matches('/')) else {
    return (StatusCode::BAD_REQUEST, "invalid redirect path").into_response();
  };
  let suffix = clean.to_string_lossy().replace('\\', "/");
  let target = if suffix.is_empty() {
    "/ui/studio".to_string()
  } else {
    format!("/ui/{suffix}")
  };
  redirect_with_query(&target, uri.query()).into_response()
}

#[cfg(test)]
mod tests {
  use super::*;
  use axum::http::header;

  #[tokio::test]
  async fn root_redirects_to_studio_ui() {
    let response = root_redirect().await.into_response();
    assert_eq!(response.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
      response.headers().get(header::LOCATION),
      Some(&"/ui/studio".parse().expect("valid location header"))
    );
  }

  #[test]
  fn ui_router_registers_canonical_and_redirect_routes() {
    let _router: Router<AppState> = ui_router();
  }

  #[test]
  fn asset_response_sets_content_type() {
    let response = asset_response("body", "text/css; charset=utf-8");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
      response.headers().get(header::CONTENT_TYPE),
      Some(
        &"text/css; charset=utf-8"
          .parse()
          .expect("valid header value")
      )
    );
  }

  #[tokio::test]
  async fn missing_ui_bundle_returns_503_with_build_instructions() {
    let resp = ui_bundle_missing_response();
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let body = axum::body::to_bytes(resp.into_body(), 1024 * 16)
      .await
      .expect("read body");
    let html = String::from_utf8_lossy(&body);
    assert!(html.contains("./yanshi-ui-dx/scripts/build.sh"));
    assert!(html.contains("YANSHI_UI_DIR"));
  }

  #[tokio::test]
  async fn dx_ui_serves_assets_with_strict_csp_and_rejects_traversal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
      tmp.path().join("index.html"),
      "<!doctype html><html><head><title>Yanshi Studio</title></head></html>",
    )
    .expect("write index");
    std::fs::create_dir_all(tmp.path().join("assets")).expect("mkdir assets");
    std::fs::write(tmp.path().join("assets/app.wasm"), b"\0asm").expect("write wasm");

    let index_resp = serve_dx_ui_file(tmp.path(), Some("index.html")).await;
    assert_eq!(index_resp.status(), StatusCode::OK);
    assert_eq!(
      index_resp.headers().get("content-security-policy"),
      Some(&HeaderValue::from_static(DX_UI_STRICT_CSP))
    );

    let wasm_resp = serve_dx_ui_file(tmp.path(), Some("assets/app.wasm")).await;
    assert_eq!(wasm_resp.status(), StatusCode::OK);
    assert_eq!(
      wasm_resp.headers().get(header::CONTENT_TYPE),
      Some(&HeaderValue::from_static("application/wasm"))
    );

    let spa_resp = serve_dx_ui_file(tmp.path(), Some("studio/probe")).await;
    assert_eq!(spa_resp.status(), StatusCode::OK);

    let missing_resp = serve_dx_ui_file(tmp.path(), Some("assets/does-not-exist.js")).await;
    assert_eq!(missing_resp.status(), StatusCode::NOT_FOUND);

    let traversal_resp = serve_dx_ui_file(tmp.path(), Some("../Cargo.toml")).await;
    assert_eq!(traversal_resp.status(), StatusCode::BAD_REQUEST);
  }

  #[tokio::test]
  async fn ui_dx_redirects_to_canonical_ui_preserving_query() {
    let root_uri: Uri = "/ui-dx?preset=demo".parse().expect("valid uri");
    let root_resp = ui_dx_redirect_root(root_uri).await.into_response();
    assert_eq!(root_resp.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
      root_resp.headers().get(header::LOCATION),
      Some(&"/ui/studio?preset=demo".parse().expect("valid location"))
    );

    let sub_uri: Uri = "/ui-dx/notebooks/nb-7?tab=sources"
      .parse()
      .expect("valid uri");
    let sub_resp = ui_dx_redirect_subpath(RoutePath("notebooks/nb-7".to_string()), sub_uri).await;
    assert_eq!(sub_resp.status(), StatusCode::TEMPORARY_REDIRECT);
    assert_eq!(
      sub_resp.headers().get(header::LOCATION),
      Some(
        &"/ui/notebooks/nb-7?tab=sources"
          .parse()
          .expect("valid location")
      )
    );
  }
}
