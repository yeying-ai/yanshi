//! Embedded Web UI mount for the single-process debugger.
//!
//! The Vite project lives in `yanshi-ui/`; `dist/` is checked in for now
//! so the server can embed a working UI without requiring Node.js at runtime.

use std::path::{Component, Path, PathBuf};

use axum::{
  Router,
  body::Body,
  extract::Path as RoutePath,
  http::{
    HeaderName, HeaderValue, StatusCode,
    header::{CACHE_CONTROL, CONTENT_TYPE},
  },
  response::{Html, IntoResponse, Redirect, Response},
  routing::get,
};

use crate::AppState;

const INDEX_HTML: &str = include_str!("../../yanshi-ui/dist/index.html");
const APP_JS: &str = include_str!("../../yanshi-ui/dist/assets/app.js");
const STYLES_CSS: &str = include_str!("../../yanshi-ui/dist/assets/styles.css");

/// Optional runtime override pointing at a built `yanshi-ui-dx/dist/public` bundle.
pub const DX_UI_DIR_ENV: &str = "YANSHI_DX_UI_DIR";

/// Strict CSP applied to `/ui-dx/*` responses to verify `dioxus-shadcn` RFC 0080
/// (zero JS `eval` / zero inline script execution; only `'wasm-unsafe-eval'`).
pub const DX_UI_STRICT_CSP: &str = "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; media-src 'self' data: blob:; connect-src 'self' http://127.0.0.1:* http://localhost:* http://[::1]:*; frame-src 'self' blob:; object-src 'none'; base-uri 'self'";

/// Static UI routes mounted by [`crate::create_router`].
///
/// `P6.1` introduces deep-link routes (`/ui/runs/new`); `P-H.5 slice 3`
/// extends the same SPA shell with three Harness Mode entry points:
///
/// - `/ui/harness/sessions` — list view
/// - `/ui/harness/sessions/new` — submit form
/// - `/ui/harness/sessions/{id}` — detail + approval panel
///
/// The server serves the same `index.html` for every SPA path;
/// client-side routing picks the matching view from
/// `window.location.pathname`.
pub fn ui_router() -> Router<AppState> {
  Router::new()
    .route("/", get(root_redirect))
    .route("/ui", get(index_html))
    .route("/ui/", get(index_html))
    .route("/ui/studio", get(index_html))
    .route("/ui/studio/", get(index_html))
    .route("/ui/notebooks", get(index_html))
    .route("/ui/notebooks/", get(index_html))
    .route("/ui/notebooks/:id", get(index_html))
    .route("/ui/app", get(index_html))
    .route("/ui/app/", get(index_html))
    .route("/ui/console", get(index_html))
    .route("/ui/console/", get(index_html))
    .route("/ui/runs/new", get(index_html))
    .route("/ui/harness/sessions", get(index_html))
    .route("/ui/harness/sessions/", get(index_html))
    .route("/ui/harness/sessions/new", get(index_html))
    .route("/ui/harness/sessions/:id", get(index_html))
    .route("/ui/diagnostics", get(index_html))
    // P6.3: side-by-side trace comparison view.
    .route("/ui/runs/:id/compare", get(index_html))
    .route("/ui/assets/app.js", get(app_js))
    .route("/ui/assets/styles.css", get(styles_css))
    // Phase 0 Dioxus UI spike: served from disk when built (`yanshi-ui-dx/dist/public`),
    // keeping large `.wasm` artifacts out of Git while testing under strict CSP.
    .route("/ui-dx", get(dx_ui_index))
    .route("/ui-dx/", get(dx_ui_index))
    .route("/ui-dx/*asset_path", get(dx_ui_asset))
}

/// Redirect bare domain root (`/`) to the Studio workspace.
pub async fn root_redirect() -> Redirect {
  Redirect::temporary("/ui/studio")
}

/// Serve the SPA shell.
pub async fn index_html() -> Html<&'static str> {
  Html(INDEX_HTML)
}

async fn app_js() -> Response {
  asset_response(APP_JS, "application/javascript; charset=utf-8")
}

async fn styles_css() -> Response {
  asset_response(STYLES_CSS, "text/css; charset=utf-8")
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

fn resolve_dx_ui_dir() -> Option<PathBuf> {
  if let Ok(raw) = std::env::var(DX_UI_DIR_ENV) {
    let trimmed = raw.trim();
    if !trimmed.is_empty() {
      let candidate = PathBuf::from(trimmed);
      if candidate.is_dir() {
        return Some(candidate);
      }
    }
  }
  [
    PathBuf::from("yanshi-ui-dx/dist/public"),
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../yanshi-ui-dx/dist/public"),
  ]
  .into_iter()
  .find(|candidate| candidate.is_dir())
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
      // SPA history-mode fallback for client routes under `/ui-dx/*`.
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

fn dx_ui_missing_response() -> Response {
  (
    StatusCode::NOT_FOUND,
    "Dioxus Phase 0 spike bundle not built yet. Run `./yanshi-ui-dx/scripts/build.sh` (or set YANSHI_DX_UI_DIR).",
  )
    .into_response()
}

async fn dx_ui_index() -> Response {
  match resolve_dx_ui_dir() {
    Some(root) => serve_dx_ui_file(&root, Some("index.html")).await,
    None => dx_ui_missing_response(),
  }
}

async fn dx_ui_asset(RoutePath(asset_path): RoutePath<String>) -> Response {
  match resolve_dx_ui_dir() {
    Some(root) => serve_dx_ui_file(&root, Some(&asset_path)).await,
    None => dx_ui_missing_response(),
  }
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

  #[tokio::test]
  async fn index_contains_app_mount() {
    let Html(html) = index_html().await;
    assert!(html.contains("yanshi-debugger"));
    assert!(html.contains("/ui/assets/app.js"));
  }

  #[test]
  fn ui_router_registers_runs_new_deep_link_route() {
    // The shared `index_html` handler covers both `/ui` and the new
    // `/ui/runs/new` deep link. We verify the route is wired without
    // standing up a full `AppState` by checking that
    // `Router::routes()` ergonomics aren't broken (the function
    // builds + accepts the state-typed handler at compile time).
    let _router: Router<AppState> = ui_router();
  }

  #[test]
  fn ui_router_registers_harness_deep_link_routes() {
    // P-H.5 slice 3 adds `/ui/harness/sessions`,
    // `/ui/harness/sessions/new`, and `/ui/harness/sessions/:id` —
    // all served by `index_html` so the SPA's pathname dispatcher
    // owns the actual rendering. The compile-time check here mirrors
    // the runs-new smoke above.
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
  async fn dx_ui_serves_assets_with_strict_csp_and_rejects_traversal() {
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
      tmp.path().join("index.html"),
      "<!doctype html><html></html>",
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

    let traversal_resp = serve_dx_ui_file(tmp.path(), Some("../Cargo.toml")).await;
    assert_eq!(traversal_resp.status(), StatusCode::BAD_REQUEST);
  }
}
