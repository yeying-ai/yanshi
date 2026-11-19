//! Y5.4 acceptance, end to end on Postgres: create a notebook → ingest a
//! source → `POST /v1/notebooks/{id}/generate {kind: report}` under a
//! mock model → the run succeeds → its files are linked to the notebook
//! and downloadable (the PDF when pandoc + typst are installed) → every
//! citation in the report's Deliverable names a chunk that exists.
//!
//! Skipped without `YANSHI_DATABASE_TEST_URL`.

use std::time::Duration;

use axum::{
  body::Body,
  http::{Request, StatusCode, header::CONTENT_TYPE},
};
use serde_json::{Value, json};
use tower::ServiceExt;
use uuid::Uuid;
use yanshi_db::{Database, EventRepo, RunRepo, SourceChunkRepo};
use yanshi_server::{AppState, create_router, tenant::TENANT_HEADER};

const MOCK_REGISTRY: &str = "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n";

async fn call(
  state: &AppState,
  method: &str,
  uri: &str,
  tenant: &str,
  body: Option<Value>,
) -> (StatusCode, Vec<u8>) {
  let request = Request::builder()
    .method(method)
    .uri(uri)
    .header(TENANT_HEADER, tenant)
    .header(CONTENT_TYPE, "application/json")
    .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
    .expect("request");
  let response = create_router(state.clone())
    .oneshot(request)
    .await
    .expect("response");
  let status = response.status();
  let bytes = axum::body::to_bytes(response.into_body(), 64 << 20)
    .await
    .expect("body");
  (status, bytes.to_vec())
}

fn json_of(bytes: &[u8]) -> Value {
  serde_json::from_slice(bytes).unwrap_or(Value::Null)
}

fn on_path(tool: &str) -> bool {
  std::env::var_os("PATH")
    .is_some_and(|path| std::env::split_paths(&path).any(|d| d.join(tool).is_file()))
}

#[tokio::test]
async fn a_notebook_generates_a_cited_report_with_downloadable_files() {
  let Ok(url) = std::env::var("YANSHI_DATABASE_TEST_URL") else {
    eprintln!("skipping notebook generate — set YANSHI_DATABASE_TEST_URL");
    return;
  };
  let dir = std::env::temp_dir().join(format!("yanshi-generate-e2e-{}", std::process::id()));
  std::fs::create_dir_all(dir.join("runs")).expect("dir");
  std::fs::write(dir.join("models.yml"), MOCK_REGISTRY).expect("registry");
  // SAFETY: env set before any model call or run in this binary.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", dir.join("models.yml"));
    std::env::set_var("YANSHI_RUN_DIR", dir.join("runs"));
  }
  let state = AppState::new(Database::connect_and_migrate(&url, 4).await.expect("db"));
  let tenant = format!("tenant-generate-{}", Uuid::new_v4());

  let (_, body) = call(
    &state,
    "POST",
    "/v1/notebooks",
    &tenant,
    Some(json!({"title": "Tides"})),
  )
  .await;
  let notebook = json_of(&body)["id"].as_str().expect("id").to_string();
  let (code, body) = call(&state, "POST", &format!("/v1/notebooks/{notebook}/sources"), &tenant, Some(json!({
    "kind": "text",
    "text": "Tides are caused by the gravitational pull of the moon.\n\nHigh tide comes twice daily."
  }))).await;
  assert_eq!(code, StatusCode::OK);
  let source = json_of(&body)["id"]
    .as_str()
    .expect("source id")
    .to_string();
  for _ in 0..200 {
    let (_, body) = call(
      &state,
      "GET",
      &format!("/v1/notebooks/{notebook}"),
      &tenant,
      None,
    )
    .await;
    if json_of(&body)["sources"][0]["status"] == "ready" {
      break;
    }
    tokio::time::sleep(Duration::from_millis(25)).await;
  }
  let chunk = format!("{source}:0");

  // Nothing ingested yet → 400; unknown kind → 400; empty or unknown source_ids → 400.
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({"kind": "essay", "model": "mock-model"})),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST);
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({"kind": "report", "model": "mock-model", "source_ids": []})),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST);
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({"kind": "report", "model": "mock-model", "source_ids": [Uuid::new_v4()]})),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST);
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({"kind": "report", "model": "mock-model", "cost_limit_usd": 0.0})),
  )
  .await;
  assert_eq!(
    code,
    StatusCode::BAD_REQUEST,
    "zero cost_limit_usd must 400"
  );
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({"kind": "report", "model": "mock-model", "cost_limit_usd": -2.0})),
  )
  .await;
  assert_eq!(
    code,
    StatusCode::BAD_REQUEST,
    "negative cost_limit_usd must 400"
  );

  // The model's replies (outline, two sections) cite the real chunk id.
  let section = |text: &str, quote: &str| {
    json!({"markdown": text, "quotes": [{"chunk_id": chunk, "quote": quote}]}).to_string()
  };
  let replies = json!([
    json!({"title": "Tides", "sections": [{"heading": "Cause", "brief": "why"}, {"heading": "Rhythm", "brief": "how often"}]}).to_string(),
    section("The moon pulls the sea.", "Tides are caused by the gravitational pull of the moon."),
    section("Two highs a day.", "High tide comes twice daily."),
  ]);
  // SAFETY: still before the first model call in this process.
  unsafe { std::env::set_var("YANSHI_MOCK_RESPONSES", replies.to_string()) };

  let want_pdf = on_path("pandoc") && on_path("typst");
  let (code, body) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &tenant,
    Some(json!({
      "kind": "report",
      "model": "mock-model",
      "cost_limit_usd": 5.0,
      "source_ids": [source],
      "params": {"sections": 2, "pdf": want_pdf}
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
  let run_id: Uuid = json_of(&body)["run_id"]
    .as_str()
    .expect("run")
    .parse()
    .expect("uuid");

  let mut status = String::new();
  for _ in 0..400 {
    status = state
      .repos
      .runs
      .get(run_id)
      .await
      .expect("get")
      .expect("row")
      .status;
    if status == "succeeded" || status == "failed" {
      break;
    }
    tokio::time::sleep(Duration::from_millis(50)).await;
  }
  let row = state
    .repos
    .runs
    .get(run_id)
    .await
    .expect("get")
    .expect("row");
  assert_eq!(status, "succeeded", "run error: {:?}", row.error);
  let ckpt_mgr = yanshi_core::checkpoint::CheckpointManager::new(
    yanshi_core::checkpoint::CheckpointConfig::default().with_checkpoint_dir(
      std::path::Path::new(row.run_dir.as_deref().expect("run_dir")).join("checkpoint"),
    ),
  )
  .expect("checkpoint manager");
  let ckpt = ckpt_mgr
    .load_latest_checkpoint(&run_id.to_string())
    .await
    .expect("load checkpoint")
    .expect("latest checkpoint");
  assert_eq!(
    ckpt.recorded_execution_config().cost_limit_usd,
    Some(5.0),
    "notebook generate must thread cost_limit_usd into FlowExecutionConfig"
  );

  // Files: linked to the notebook, each downloadable.
  let (code, body) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{notebook}/artifacts"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK);
  let artifacts = json_of(&body)["artifacts"]
    .as_array()
    .cloned()
    .expect("artifacts");
  let by_node = |node: &str| artifacts.iter().find(|a| a["node_id"] == node).cloned();
  for node in ["report_md", "report_html"] {
    assert!(
      by_node(node).is_some_and(|a| a["kind"] == "report"),
      "{node} in {artifacts:?}"
    );
  }
  let md = by_node("report_md").expect("md");
  assert_eq!(md["deliverable"]["title"], "Tides", "{md}");
  assert_eq!(
    md["deliverable"]["citations"].as_array().map(Vec::len),
    Some(2),
    "GET /v1/notebooks/:id/artifacts must expose persisted Deliverable citations: {md}"
  );
  assert_eq!(md["deliverable"]["citations"][0]["chunk_id"], chunk, "{md}");
  let (code, bytes) = call(
    &state,
    "GET",
    &format!(
      "/v1/runs/{run_id}/artifacts/{}",
      md["id"].as_str().expect("id")
    ),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK);
  let markdown = String::from_utf8(bytes).expect("utf-8");
  assert!(
    markdown.starts_with("# Tides") && markdown.contains(&format!("[{chunk}]")),
    "{markdown}"
  );
  if want_pdf {
    let pdf = by_node("report_pdf").expect("pdf linked");
    let (code, bytes) = call(
      &state,
      "GET",
      &format!(
        "/v1/runs/{run_id}/artifacts/{}",
        pdf["id"].as_str().expect("id")
      ),
      &tenant,
      None,
    )
    .await;
    assert_eq!(code, StatusCode::OK);
    assert!(bytes.starts_with(b"%PDF"), "a real PDF");
  } else {
    eprintln!("pandoc / typst not installed: PDF not checked");
  }

  // The Deliverable (from the run's events) cites chunks that exist.
  let events = state
    .repos
    .events
    .list_after(&tenant, run_id, -1, 10_000)
    .await
    .expect("events");
  let deliverable = events
    .iter()
    .find(|e| e.kind == "node.output.captured" && e.payload["node_id"] == "deliverable")
    .map(|e| e.payload["output"]["deliverable"]["value"].clone())
    .expect("deliverable output event");
  let citations = deliverable["citations"]
    .as_array()
    .cloned()
    .expect("citations");
  assert_eq!(citations.len(), 2, "{deliverable}");
  for citation in &citations {
    let id = citation["chunk_id"].as_str().expect("chunk id");
    let stored = state.repos.chunks.get(&tenant, id).await.expect("lookup");
    assert!(stored.is_some(), "cited chunk {id} exists");
    assert_eq!(citation["source_id"], source.as_str());
  }

  // Another tenant sees neither the notebook's files nor can generate.
  let other = format!("tenant-generate-other-{}", Uuid::new_v4());
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{notebook}/artifacts"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{notebook}/generate"),
    &other,
    Some(json!({"kind": "report", "model": "mock-model"})),
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);

  // AD1: GET /v1/notebooks/{id}/artifacts.zip streams `<kind>/<file>` + `BRIEFING.md` + manifest,
  // and enforces tenant isolation.
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{notebook}/artifacts.zip"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant zip must 404");

  let (code, zip_bytes) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{notebook}/artifacts.zip"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK);
  {
    use std::io::Read as _;
    let mut archive =
      zip::ZipArchive::new(std::io::Cursor::new(zip_bytes)).expect("valid notebook artifacts zip");
    let mut brief = String::new();
    archive
      .by_name("BRIEFING.md")
      .expect("BRIEFING.md in zip")
      .read_to_string(&mut brief)
      .expect("read brief");
    assert!(
      brief.contains("# Tides")
        && brief.contains("### Tides (`report`)")
        && brief.contains("Tides are caused by the gravitational pull of the moon."),
      "unexpected BRIEFING.md: {brief}"
    );
    assert!(archive.by_name("report/1.md").is_ok());
    assert!(archive.by_name("report/1.html").is_ok());
    let mut manifest_str = String::new();
    archive
      .by_name("notebook_artifacts.json")
      .expect("notebook_artifacts.json")
      .read_to_string(&mut manifest_str)
      .expect("read manifest");
    let manifest: Value = serde_json::from_str(&manifest_str).expect("manifest json");
    assert_eq!(manifest["notebook_id"], notebook);
  }

  // DELETE /v1/notebooks/{id}/artifacts/{artifact_id} enforces tenant scope and detaches the artifact.
  let md_artifact_id = md["id"].as_str().expect("md artifact id").to_string();
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{notebook}/artifacts/{md_artifact_id}"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant detach must 404");
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{notebook}/artifacts/{}", Uuid::new_v4()),
    &tenant,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::NOT_FOUND,
    "unknown artifact detach must 404"
  );
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{notebook}/artifacts/{md_artifact_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NO_CONTENT);
  let (_, remaining_body) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{notebook}/artifacts"),
    &tenant,
    None,
  )
  .await;
  let remaining = json_of(&remaining_body)["artifacts"]
    .as_array()
    .cloned()
    .expect("remaining artifacts");
  assert!(
    remaining.iter().all(|a| a["id"] != md_artifact_id),
    "detached artifact must no longer appear in notebook artifacts: {remaining:?}"
  );
}
