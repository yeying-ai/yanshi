//! Y5.2 acceptance: notebook sources are ingested in the background,
//! searchable, and still searchable after a restart (a fresh `AppState`
//! on the same database: the index is rebuilt from `source_chunks`).
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
use yanshi_db::Database;
use yanshi_server::{AppState, create_router, tenant::TENANT_HEADER};

async fn state() -> Option<AppState> {
  let url = std::env::var("YANSHI_DATABASE_TEST_URL").ok()?;
  let db = Database::connect_and_migrate(&url, 4).await.ok()?;
  Some(AppState::new(db))
}

fn minimal_pdf_bytes(line: &str) -> Vec<u8> {
  let stream = format!("BT /F1 12 Tf 72 720 Td ({line}) Tj ET");
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

async fn call(
  state: &AppState,
  method: &str,
  uri: &str,
  tenant: &str,
  body: Option<Value>,
) -> (StatusCode, Value) {
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
  let bytes = axum::body::to_bytes(response.into_body(), 1 << 20)
    .await
    .expect("body");
  (
    status,
    serde_json::from_slice(&bytes).unwrap_or(Value::Null),
  )
}

/// Poll until every source of the notebook has left pending / ingesting.
async fn settled_sources(state: &AppState, tenant: &str, notebook: &str) -> Vec<Value> {
  for _ in 0..200 {
    let (_, body) = call(
      state,
      "GET",
      &format!("/v1/notebooks/{notebook}"),
      tenant,
      None,
    )
    .await;
    let sources = body["sources"].as_array().cloned().unwrap_or_default();
    if !sources.is_empty()
      && sources
        .iter()
        .all(|s| matches!(s["status"].as_str(), Some("ready" | "failed")))
    {
      return sources;
    }
    tokio::time::sleep(Duration::from_millis(25)).await;
  }
  panic!("sources did not settle");
}

#[tokio::test]
async fn sources_are_ingested_searchable_and_survive_a_restart() {
  let Some(state) = state().await else {
    eprintln!("skipping notebook routes — set YANSHI_DATABASE_TEST_URL");
    return;
  };
  let tenant = format!("tenant-nb-routes-{}", Uuid::new_v4());
  let other = format!("tenant-nb-other-{}", Uuid::new_v4());

  let (code, notebook) = call(
    &state,
    "POST",
    "/v1/notebooks",
    &tenant,
    Some(json!({"title": "Tides"})),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{notebook}");
  let id = notebook["id"].as_str().expect("id").to_string();

  // Text, an HTML file, and a URL the SSRF filter must refuse.
  let text = "The tides are caused by the gravitational pull of the moon.\n\nSpring tides happen when the sun and moon align.";
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({"kind": "text", "text": text})),
  )
  .await;
  assert_eq!(code, StatusCode::OK);
  use base64::Engine as _;
  let html = "<html><body><h1>Neap tides</h1><p>Neap tides occur at quarter moon.</p><script>noise()</script></body></html>";
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file", "filename": "neap.html",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(html)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK);
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({"kind": "url", "url": "http://127.0.0.1:9/private"})),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "accepted, then refused at fetch");
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({"kind": "recording"})),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST, "empty recording payload");

  // PDF upload and audio recording (transcribed via mock-asr).
  let pdf_bytes = minimal_pdf_bytes("Perigean king tides peak when the moon is closest.");
  let (code, pdf_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "perigean.pdf",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(&pdf_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{pdf_src}");

  let dir = std::env::temp_dir().join(format!("yanshi-nb-routes-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("dir");
  let models_yml = dir.join("models.yml");
  std::fs::write(
    &models_yml,
    "models:\n  mock-asr:\n    vendor: mock\n    type: asr\n    model_id: mock-asr\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
  )
  .expect("models.yml");
  yanshi_llm::Yanshi::init_with_config(models_yml.to_str().expect("utf8"))
    .await
    .expect("init mock-asr");
  let wav_bytes = yanshi_llm::providers::mock_media::silent_wav(200);
  let (code, rec_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "recording",
      "filename": "briefing.wav",
      "model": "mock-asr",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(&wav_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{rec_src}");

  let sources = settled_sources(&state, &tenant, &id).await;
  assert_eq!(sources.len(), 5);
  let by_kind = |kind: &str| {
    sources
      .iter()
      .find(|s| s["kind"] == kind)
      .expect(kind)
      .clone()
  };
  assert_eq!(by_kind("text")["status"], "ready");
  assert_eq!(by_kind("text")["sha256"].as_str().map(str::len), Some(64));
  assert!(
    sources
      .iter()
      .filter(|s| s["kind"] == "file")
      .all(|s| s["status"] == "ready"),
    "both HTML and PDF files must ingest cleanly: {sources:?}"
  );
  assert_eq!(
    by_kind("recording")["status"],
    "ready",
    "recording source must transcribe to ready: {sources:?}"
  );
  let url = by_kind("url");
  assert_eq!(url["status"], "failed", "loopback must be refused: {url}");
  assert!(url["error"].as_str().is_some_and(|e| !e.is_empty()));

  // Search finds the right chunk, with its source and chunk id.
  let search = |state: AppState, q: &'static str| {
    let tenant = tenant.clone();
    let id = id.clone();
    async move {
      call(
        &state,
        "GET",
        &format!("/v1/notebooks/{id}/search?q={q}"),
        &tenant,
        None,
      )
      .await
    }
  };
  let (code, hits) = search(state.clone(), "moon").await;
  assert_eq!(code, StatusCode::OK, "{hits}");
  let first = &hits["chunks"][0];
  assert!(
    first["content"]
      .as_str()
      .is_some_and(|c| c.contains("moon")),
    "{hits}"
  );
  assert_eq!(first["source_id"], by_kind("text")["id"]);
  assert!(
    first["chunk_id"]
      .as_str()
      .is_some_and(|c| c.starts_with(by_kind("text")["id"].as_str().unwrap_or("?")))
  );
  let (_, hits) = search(state.clone(), "quarter").await;
  assert_eq!(
    hits["chunks"][0]["source_id"],
    sources
      .iter()
      .find(|s| s["title"] == "neap.html")
      .expect("neap.html")["id"],
    "HTML text is searchable: {hits}"
  );
  assert!(!hits.to_string().contains("noise()"), "scripts are dropped");
  let (_, pdf_hits) = search(state.clone(), "Perigean").await;
  assert_eq!(
    pdf_hits["chunks"][0]["source_id"], pdf_src["id"],
    "PDF text is searchable: {pdf_hits}"
  );
  let (_, rec_hits) = search(state.clone(), "transcript").await;
  assert_eq!(
    rec_hits["chunks"][0]["source_id"], rec_src["id"],
    "Recording transcript is searchable: {rec_hits}"
  );

  // "Restart": a new AppState (empty index cache) on the same database.
  let restarted = AppState::new(state.db.clone());
  let (code, hits) = search(restarted, "moon").await;
  assert_eq!(code, StatusCode::OK);
  assert!(
    hits["chunks"].as_array().is_some_and(|c| !c.is_empty()),
    "the index is rebuilt from source_chunks: {hits}"
  );

  // Another tenant sees nothing.
  for uri in [
    format!("/v1/notebooks/{id}"),
    format!("/v1/notebooks/{id}/search?q=moon"),
  ] {
    let (code, _) = call(&state, "GET", &uri, &other, None).await;
    assert_eq!(code, StatusCode::NOT_FOUND, "{uri}");
  }
  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &other,
    Some(json!({"kind": "text", "text": "x"})),
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);
}
