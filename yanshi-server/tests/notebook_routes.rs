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
use yanshi_db::{ArtifactRepo, Database, RunRepo};
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

  // PATCH /v1/notebooks/{id} validates title, enforces tenant ownership, and persists rename.
  let (code, _) = call(
    &state,
    "PATCH",
    &format!("/v1/notebooks/{id}"),
    &tenant,
    Some(json!({"title": "   "})),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST, "blank title rejected");
  let (code, _) = call(
    &state,
    "PATCH",
    &format!("/v1/notebooks/{id}"),
    &other,
    Some(json!({"title": "Intruder"})),
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant rename 404");
  let (code, renamed) = call(
    &state,
    "PATCH",
    &format!("/v1/notebooks/{id}"),
    &tenant,
    Some(json!({"title": "Oceanic Tides & Estuaries"})),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{renamed}");
  assert_eq!(renamed["title"], "Oceanic Tides & Estuaries");

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

  // PDF, DOCX, CSV uploads and audio recording (transcribed via mock-asr).
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

  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "corrupt.docx",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(b"not-a-zip")
    })),
  )
  .await;
  assert_eq!(
    code,
    StatusCode::BAD_REQUEST,
    "non-zip docx rejected at submit"
  );

  let docx_bytes = {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
      let mut zip = zip::ZipWriter::new(&mut cursor);
      let opts = zip::write::SimpleFileOptions::default();
      zip.start_file("word/document.xml", opts).expect("start");
      zip
        .write_all(
          br#"<?xml version="1.0" encoding="UTF-8"?><w:document><w:body><w:p><w:r><w:t>Amphidromic nodes anchor zero-amplitude rotary tidal systems.</w:t></w:r></w:p></w:body></w:document>"#,
        )
        .expect("write");
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  };
  let (code, docx_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "amphidromic.docx",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(&docx_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{docx_src}");

  let csv_bytes = b"estuary,macrotidal_range_m\nSevernBore,15.0\n";
  let (code, csv_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "estuaries.csv",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(csv_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{csv_src}");

  let xlsx_bytes = {
    use std::io::Write as _;
    let mut cursor = std::io::Cursor::new(Vec::new());
    {
      let mut zip = zip::ZipWriter::new(&mut cursor);
      let opts = zip::write::SimpleFileOptions::default();
      zip.start_file("xl/workbook.xml", opts).expect("start");
      zip
        .write_all(
          br#"<workbook><sheets><sheet name="Macrotidal" sheetId="1"/></sheets></workbook>"#,
        )
        .expect("write");
      zip.start_file("xl/sharedStrings.xml", opts).expect("start");
      zip
        .write_all(
          br#"<sst><si><t>Basin</t></si><si><t>RangeM</t></si><si><t>UngavaBay</t></si></sst>"#,
        )
        .expect("write");
      zip
        .start_file("xl/worksheets/sheet1.xml", opts)
        .expect("start");
      zip
        .write_all(
          br#"<worksheet><sheetData><row r="1"><c r="A1" t="s"><v>0</v></c><c r="B1" t="s"><v>1</v></c></row><row r="2"><c r="A2" t="s"><v>2</v></c><c r="B2"><v>16.8</v></c></row></sheetData></worksheet>"#,
        )
        .expect("write");
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  };
  let (code, xlsx_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "ungava.xlsx",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(&xlsx_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{xlsx_src}");

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
          br#"<p:sld><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>K1 Diurnal Constituent Overview</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:sld>"#,
        )
        .expect("write");
      zip
        .start_file("ppt/notesSlides/notesSlide1.xml", opts)
        .expect("start");
      zip
        .write_all(
          br#"<p:notes><p:cSld><p:spTree><p:sp><p:txBody><a:p><a:r><a:t>Lunisolar diurnal K1 period is 23.93 hours.</a:t></a:r></a:p></p:txBody></p:sp></p:spTree></p:cSld></p:notes>"#,
        )
        .expect("write");
      zip.finish().expect("finish");
    }
    cursor.into_inner()
  };
  let (code, pptx_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "harmonics.pptx",
      "content_base64": base64::engine::general_purpose::STANDARD.encode(&pptx_bytes)
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{pptx_src}");

  let dir = std::env::temp_dir().join(format!("yanshi-nb-routes-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("dir");
  let models_yml = dir.join("models.yml");
  std::fs::write(
    &models_yml,
    "models:\n  mock-asr:\n    vendor: mock\n    type: asr\n    model_id: mock-asr\n  mock-ask:\n    vendor: mock\n    type: text\n    model_id: mock-ask\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
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
  assert_eq!(sources.len(), 9);
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
  let (code, hits) = search(state.clone(), "gravitational").await;
  assert_eq!(code, StatusCode::OK, "{hits}");
  let first = &hits["chunks"][0];
  assert!(
    first["content"]
      .as_str()
      .is_some_and(|c| c.contains("gravitational") && c.contains("moon")),
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
  let (_, docx_hits) = search(state.clone(), "Amphidromic").await;
  assert_eq!(
    docx_hits["chunks"][0]["source_id"], docx_src["id"],
    "DOCX text is searchable: {docx_hits}"
  );
  let (_, csv_hits) = search(state.clone(), "SevernBore").await;
  assert_eq!(
    csv_hits["chunks"][0]["source_id"], csv_src["id"],
    "CSV rows are searchable with column headers: {csv_hits}"
  );
  let (_, xlsx_hits) = search(state.clone(), "UngavaBay").await;
  assert_eq!(
    xlsx_hits["chunks"][0]["source_id"], xlsx_src["id"],
    "XLSX workbook rows are searchable with sheet and column headers: {xlsx_hits}"
  );
  let (_, pptx_hits) = search(state.clone(), "Lunisolar").await;
  assert_eq!(
    pptx_hits["chunks"][0]["source_id"], pptx_src["id"],
    "PPTX slide body and speaker notes are searchable: {pptx_hits}"
  );
  let (_, rec_hits) = search(state.clone(), "transcript").await;
  assert_eq!(
    rec_hits["chunks"][0]["source_id"], rec_src["id"],
    "Recording transcript is searchable: {rec_hits}"
  );

  // `source_ids` scoping restricts search and `/ask` strictly to the chosen sources.
  let csv_source_id = csv_src["id"].as_str().expect("csv id").to_string();
  let docx_source_id = docx_src["id"].as_str().expect("docx id").to_string();
  let (code, scoped_miss) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/search?q=gravitational&source_ids={csv_source_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{scoped_miss}");
  assert_eq!(
    scoped_miss["chunks"].as_array().map(Vec::len),
    Some(0),
    "text-source match must be excluded when search is scoped to csv_src: {scoped_miss}"
  );
  let (code, scoped_hit) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/search?q=SevernBore&source_ids={csv_source_id},{docx_source_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{scoped_hit}");
  assert_eq!(scoped_hit["chunks"][0]["source_id"], csv_src["id"]);

  let (code, _) = call(
    &state,
    "GET",
    &format!(
      "/v1/notebooks/{id}/search?q=SevernBore&source_ids={}",
      Uuid::new_v4()
    ),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST, "unknown source_id rejected");

  // Grounded Q&A (`POST /v1/notebooks/{id}/ask`) verifies real chunk_ids and drops hallucinated ones.
  let csv_chunk_id = csv_hits["chunks"][0]["chunk_id"]
    .as_str()
    .expect("csv chunk_id")
    .to_string();
  let docx_chunk_id = docx_hits["chunks"][0]["chunk_id"]
    .as_str()
    .expect("docx chunk_id")
    .to_string();
  let mock_ask_reply = json!([
    json!({
      "answer": "Severn Bore reaches 15.0 m while amphidromic nodes exhibit zero amplitude.",
      "citations": [
        { "chunk_id": csv_chunk_id, "quote": "SevernBore | macrotidal_range_m: 15.0" },
        { "chunk_id": docx_chunk_id, "quote": "zero-amplitude rotary tidal systems" },
        { "chunk_id": "nonexistent:999", "quote": "Hallucinated evidence" }
      ]
    })
    .to_string(),
    json!({
      "answer": "Those rotary systems have zero amplitude at their amphidromic anchor nodes.",
      "citations": [
        { "chunk_id": docx_chunk_id, "quote": "Amphidromic nodes anchor zero-amplitude rotary tidal systems." }
      ]
    })
    .to_string()
  ]);
  // SAFETY: single test function in this integration test binary.
  unsafe {
    std::env::set_var("YANSHI_MOCK_RESPONSES", mock_ask_reply.to_string());
  }
  yanshi_llm::Yanshi::init_with_config(models_yml.to_str().expect("utf8"))
    .await
    .expect("reinit mock-ask");
  let (code, ask_res) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/ask"),
    &tenant,
    Some(json!({
      "question": "Compare Severn Bore macrotidal range with amphidromic nodes.",
      "model": "mock-ask",
      "source_ids": [csv_source_id]
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{ask_res}");
  assert!(
    ask_res["answer"]
      .as_str()
      .is_some_and(|a| a.contains("15.0 m")),
    "{ask_res}"
  );
  // Because `source_ids` only included `csv_source_id`, both `nonexistent:999` AND
  // `docx_chunk_id` (outside the selected scope!) must be stripped by server verification!
  let ask_cites = ask_res["citations"].as_array().expect("ask citations");
  assert_eq!(
    ask_cites.len(),
    1,
    "only citations belonging to the scoped ready chunks may survive: {ask_res}"
  );
  assert_eq!(ask_cites[0]["chunk_id"], csv_chunk_id);
  assert_eq!(ask_cites[0]["source_id"], csv_src["id"]);

  // Multi-turn follow-up `/ask` accepts bounded `history` and verifies citations.
  let (code, followup_res) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/ask"),
    &tenant,
    Some(json!({
      "question": "Where are those zero-amplitude systems anchored?",
      "history": [
        {
          "question": "Tell me about amphidromic systems.",
          "answer": "They are rotary tidal systems."
        }
      ],
      "model": "mock-ask"
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{followup_res}");
  assert_eq!(followup_res["citations"][0]["chunk_id"], docx_chunk_id);
  assert_eq!(followup_res["citations"][0]["source_id"], docx_src["id"]);

  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/ask"),
    &other,
    Some(json!({ "question": "probe", "model": "mock-ask" })),
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant /ask must 404");

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

  // Citation chunk lookup resolves chunk_id scoped to (tenant, notebook).
  let pdf_chunk_id = pdf_hits["chunks"][0]["chunk_id"]
    .as_str()
    .expect("pdf chunk_id")
    .to_string();
  let (code, chunk_doc) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/chunks/{pdf_chunk_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{chunk_doc}");
  assert_eq!(chunk_doc["chunk_id"], pdf_chunk_id);
  assert!(
    chunk_doc["content"]
      .as_str()
      .is_some_and(|c| c.contains("Perigean")),
    "{chunk_doc}"
  );
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/chunks/{pdf_chunk_id}"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{}/chunks/{pdf_chunk_id}", Uuid::new_v4()),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);

  // Deleting a source invalidates the in-memory BM25 cache immediately and cascades chunks.
  let pdf_source_id = pdf_src["id"].as_str().expect("pdf source id").to_string();

  // AE1: `GET /v1/notebooks/{id}/sources/{source_id}/chunks` lists ordered passages for an owned source
  // and returns 404 across tenants or wrong notebook IDs.
  let (code, source_chunks_doc) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/sources/{pdf_source_id}/chunks"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{source_chunks_doc}");
  assert_eq!(source_chunks_doc["notebook_id"], id);
  assert_eq!(source_chunks_doc["source"]["id"], pdf_source_id);
  let source_chunks_arr = source_chunks_doc["chunks"]
    .as_array()
    .expect("source chunks array");
  assert!(
    !source_chunks_arr.is_empty(),
    "ready PDF source must list non-empty ordered chunks: {source_chunks_doc}"
  );
  assert_eq!(source_chunks_arr[0]["ordinal"], 0);
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/sources/{pdf_source_id}/chunks"),
    &other,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::NOT_FOUND,
    "cross-tenant source chunks must 404"
  );
  let (code, _) = call(
    &state,
    "GET",
    &format!(
      "/v1/notebooks/{}/sources/{pdf_source_id}/chunks",
      Uuid::new_v4()
    ),
    &tenant,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::NOT_FOUND,
    "wrong-notebook source chunks must 404"
  );
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/sources/{}/chunks", Uuid::new_v4()),
    &tenant,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::NOT_FOUND,
    "unknown source_id chunks must 404"
  );
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{id}/sources/{pdf_source_id}"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{id}/sources/{pdf_source_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NO_CONTENT);
  let (code, after_delete_hits) = search(state.clone(), "Perigean").await;
  assert_eq!(code, StatusCode::OK);
  assert_eq!(
    after_delete_hits["chunks"].as_array().map(Vec::len),
    Some(0),
    "deleted source must disappear immediately from cached BM25 search: {after_delete_hits}"
  );
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/chunks/{pdf_chunk_id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "source_chunks cascaded");

  // AF2: `/ask` turns are persisted in `GET /v1/notebooks/{id}/messages`,
  // sources support `PATCH` rename + inline text re-chunking and `:retry`,
  // and `export`/`import` preserves Q&A turns with remapped `chunk_id`s.
  let (code, listed_msgs) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/messages"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{listed_msgs}");
  assert_eq!(
    listed_msgs["messages"].as_array().map(Vec::len),
    Some(2),
    "both `/ask` turns must be persisted chronologically: {listed_msgs}"
  );

  let text_source_id = by_kind("text")["id"].as_str().expect("text id").to_string();
  let (code, patched_src) = call(
    &state,
    "PATCH",
    &format!("/v1/notebooks/{id}/sources/{text_source_id}"),
    &tenant,
    Some(json!({
      "title": "Updated Syzygy Field Note",
      "text": "KingTideSyzygy alignment amplifies gravitational tidal range along continental shelves."
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{patched_src}");
  assert_eq!(patched_src["title"], "Updated Syzygy Field Note");
  let (code, syzygy_hits) = search(state.clone(), "KingTideSyzygy").await;
  assert_eq!(code, StatusCode::OK, "{syzygy_hits}");
  assert_eq!(
    syzygy_hits["chunks"][0]["source_id"],
    json!(text_source_id),
    "edited text source must be re-chunked and immediately searchable: {syzygy_hits}"
  );

  let failed_url_id = url["id"].as_str().expect("failed url id").to_string();
  let (code, retried_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources/{failed_url_id}:retry"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{retried_src}");
  let _ = settled_sources(&state, &tenant, &id).await;

  // AG2: Verify `raw_text` lossless note retrieval + `.vtt` subtitle upload + Studio run attachment (`POST /v1/notebooks/{id}/artifacts`).
  let (code, edited_chunks_env) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/sources/{text_source_id}/chunks"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{edited_chunks_env}");
  assert_eq!(
    edited_chunks_env["raw_text"],
    "KingTideSyzygy alignment amplifies gravitational tidal range along continental shelves.",
    "GET /sources/:id/chunks must return lossless raw_text for text notes"
  );

  let vtt_b64 = base64::Engine::encode(
    &base64::engine::general_purpose::STANDARD,
    b"WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nSevernEstuary funnel resonance reaches fourteen meters.\n",
  );
  let (code, vtt_src) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/sources"),
    &tenant,
    Some(json!({
      "kind": "file",
      "filename": "lecture.vtt",
      "content_base64": vtt_b64,
      "title": "Severn Lecture Subtitles"
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{vtt_src}");
  let _ = settled_sources(&state, &tenant, &id).await;

  let studio_run_id = Uuid::new_v4();
  let studio_run_dir = tempfile::tempdir().expect("studio run dir");
  let studio_html_path = studio_run_dir.path().join("cards.html");
  std::fs::write(&studio_html_path, b"<html><body>Flashcards</body></html>").expect("write html");
  let cited_text_cid = format!("{text_source_id}:0");
  let studio_deliv_state: yanshi_core::async_node::AsyncNodeResult =
    Ok(std::collections::HashMap::from([(
      "deliverable".to_string(),
      yanshi_core::value::FlowValue::Json(json!({
        "deliverable": {
          "kind": "flashcards",
          "title": "Syzygy Flashcards",
          "content": {
            "items": [{
              "front": "What maximizes amplitude?",
              "back": "Syzygy alignment",
              "chunk_id": cited_text_cid
            }]
          },
          "citations": [{
            "source_id": text_source_id,
            "chunk_id": cited_text_cid,
            "quote": "Syzygy gravitational alignment maximizes tidal amplitude"
          }],
          "files": [{
            "node_id": "page",
            "name": "cards.html",
            "mime_type": "text/html",
            "role": "primary"
          }]
        }
      })),
    )]));
  std::fs::write(
    studio_run_dir.path().join("deliverable_outputs.json"),
    serde_json::to_vec(&studio_deliv_state).expect("json"),
  )
  .expect("write deliverable_outputs.json");
  state
    .repos
    .runs
    .create(yanshi_db::NewRun {
      id: studio_run_id,
      workflow: "name: flashcards\nnodes: []".to_string(),
      status: yanshi_db::RunStatus::Succeeded,
      run_dir: Some(studio_run_dir.path().display().to_string()),
      tenant_id: tenant.clone(),
      events_retention_days: None,
      artifacts_retention_days: None,
    })
    .await
    .expect("create studio run");
  state
    .repos
    .artifacts
    .create(yanshi_db::NewArtifact {
      id: Uuid::new_v4(),
      run_id: studio_run_id,
      node_id: "page".to_string(),
      name: "cards.html".to_string(),
      path_or_url: studio_html_path.display().to_string(),
      mime_type: Some("text/html".to_string()),
      tenant_id: Some(tenant.clone()),
      size_bytes: Some(36),
      sha256: Some("00".repeat(32)),
    })
    .await
    .expect("create studio artifact");
  state
    .repos
    .artifacts
    .create(yanshi_db::NewArtifact {
      id: Uuid::new_v4(),
      run_id: studio_run_id,
      node_id: "illustrate".to_string(),
      name: "raw_asset.png".to_string(),
      path_or_url: studio_html_path.display().to_string(),
      mime_type: Some("image/png".to_string()),
      tenant_id: Some(tenant.clone()),
      size_bytes: Some(36),
      sha256: Some("11".repeat(32)),
    })
    .await
    .expect("create intermediate studio asset");

  let (code, _) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/artifacts"),
    &other,
    Some(json!({ "run_id": studio_run_id })),
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant attach must 404");

  let (code, attached_res) = call(
    &state,
    "POST",
    &format!("/v1/notebooks/{id}/artifacts"),
    &tenant,
    Some(json!({ "run_id": studio_run_id })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{attached_res}");
  assert_eq!(
    attached_res["attached_count"], 1,
    "must attach primary cards.html and skip intermediate asset: {attached_res}"
  );

  // AD1: Export portable NotebookExportBundle and re-import twice in the same tenant
  // + once in another tenant without `source_chunks` PK collision, immediately searchable.
  let (code, _) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/export"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND, "cross-tenant export must 404");

  let (code, exported_bundle) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{id}/export"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{exported_bundle}");
  assert_eq!(exported_bundle["version"], 1);
  assert_eq!(exported_bundle["sources"].as_array().map(Vec::len), Some(9));
  assert_eq!(
    exported_bundle["deliverables"].as_array().map(Vec::len),
    Some(1),
    "exported bundle must include attached flashcards deliverable"
  );
  assert_eq!(
    exported_bundle["messages"].as_array().map(Vec::len),
    Some(2),
    "exported bundle must include persisted Q&A turns: {exported_bundle}"
  );
  assert!(
    exported_bundle["markdown_brief"]
      .as_str()
      .is_some_and(|md| md.contains("## Sources (9)") && md.contains("## Grounded Q&A (2)")),
    "export must include markdown_brief with Q&A history: {exported_bundle}"
  );

  // Invalid bundle version is rejected before creating a notebook.
  let (code, _) = call(
    &state,
    "POST",
    "/v1/notebooks/import",
    &tenant,
    Some(json!({ "version": 99, "notebook": { "title": "Bad" }, "sources": [] })),
  )
  .await;
  assert_eq!(code, StatusCode::BAD_REQUEST);

  // Direct raw bundle import (#1 in `tenant`).
  let (code, imported_1) = call(
    &state,
    "POST",
    "/v1/notebooks/import",
    &tenant,
    Some(exported_bundle.clone()),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_1}");
  assert_eq!(imported_1["imported_sources"], 9);
  assert_eq!(
    imported_1["imported_deliverables"], 1,
    "import must restore exported deliverables: {imported_1}"
  );
  assert!(
    imported_1["imported_chunks"]
      .as_u64()
      .is_some_and(|n| n >= 7)
  );
  let imported_id_1 = imported_1["notebook"]["id"]
    .as_str()
    .expect("imported id 1")
    .to_string();
  assert_eq!(imported_1["imported_messages"], 2);
  let (code, imported_msgs) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_id_1}/messages"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_msgs}");
  let remapped_cid = imported_msgs["messages"][0]["citations"][0]["chunk_id"]
    .as_str()
    .expect("remapped chunk_id");
  let (code, resolved_chunk) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_id_1}/chunks/{remapped_cid}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::OK,
    "imported message citation chunk_id must resolve in imported notebook: {resolved_chunk}"
  );
  let (code, imported_arts) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_id_1}/artifacts"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_arts}");
  let imp_art_arr = imported_arts["artifacts"]
    .as_array()
    .expect("imported artifacts");
  assert_eq!(imp_art_arr.len(), 1);
  let imp_item_cid = imp_art_arr[0]["deliverable"]["content"]["items"][0]["chunk_id"]
    .as_str()
    .expect("remapped deliverable item chunk_id");
  let (code, resolved_deliv_chunk) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_id_1}/chunks/{imp_item_cid}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(
    code,
    StatusCode::OK,
    "imported deliverable nested chunk_id must resolve in imported notebook: {resolved_deliv_chunk}"
  );
  let (code, cleared_msgs) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{imported_id_1}/messages"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{cleared_msgs}");
  assert_eq!(cleared_msgs["deleted"], 2);

  // Wrapped bundle import (#2 in same `tenant` with title override) — proves zero PK collision.
  let (code, imported_2) = call(
    &state,
    "POST",
    "/v1/notebooks/import",
    &tenant,
    Some(json!({
      "bundle": exported_bundle.clone(),
      "title": "Cloned Tidal Archive"
    })),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_2}");
  assert_eq!(imported_2["notebook"]["title"], "Cloned Tidal Archive");

  // Cross-tenant import into `other` also succeeds and is immediately searchable via BM25.
  let (code, imported_other) = call(
    &state,
    "POST",
    "/v1/notebooks/import",
    &other,
    Some(exported_bundle),
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_other}");
  let imported_other_id = imported_other["notebook"]["id"]
    .as_str()
    .expect("imported other id")
    .to_string();

  let (code, imported_hits) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_id_1}/search?q=UngavaBay"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{imported_hits}");
  assert!(
    imported_hits["chunks"]
      .as_array()
      .is_some_and(|arr| !arr.is_empty()),
    "imported notebook must be immediately searchable via BM25: {imported_hits}"
  );
  let (code, other_hits) = call(
    &state,
    "GET",
    &format!("/v1/notebooks/{imported_other_id}/search?q=Lunisolar"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::OK, "{other_hits}");
  assert!(
    other_hits["chunks"]
      .as_array()
      .is_some_and(|arr| !arr.is_empty()),
    "cross-tenant imported notebook must be immediately searchable: {other_hits}"
  );

  // Deleting the notebook cascades remaining sources and returns 404 thereafter.
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{id}"),
    &other,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NOT_FOUND);
  let (code, _) = call(
    &state,
    "DELETE",
    &format!("/v1/notebooks/{id}"),
    &tenant,
    None,
  )
  .await;
  assert_eq!(code, StatusCode::NO_CONTENT);
  let (code, _) = call(&state, "GET", &format!("/v1/notebooks/{id}"), &tenant, None).await;
  assert_eq!(code, StatusCode::NOT_FOUND);
}
