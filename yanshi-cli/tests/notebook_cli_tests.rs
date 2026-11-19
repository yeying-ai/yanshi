//! End-to-end CLI integration tests for `yanshi notebook`:
//! 1. Always-on offline loopback Axum wire-contract test (verifies headers,
//!    base64 `--file` upload, `--wait` source & run polling, `ask`,
//!    `generate --output-dir` streaming artifact download + `artifacts.json`).
//! 2. Real `yanshi-server` + PostgreSQL acceptance test (runs automatically
//!    whenever `YANSHI_DATABASE_TEST_URL` is set).

use std::sync::{
  Arc,
  atomic::{AtomicUsize, Ordering},
};

use assert_cmd::Command;
use axum::{
  Json, Router,
  extract::{Path, Query},
  http::{HeaderMap, StatusCode},
  response::IntoResponse,
  routing::{delete, get, post},
};
use serde_json::{Value, json};
use uuid::Uuid;

#[tokio::test]
async fn notebook_cli_drives_sources_ask_and_generate_output_dir_over_loopback() {
  let nb_id = Uuid::new_v4().to_string();
  let src_id = Uuid::new_v4().to_string();
  let run_id = Uuid::new_v4().to_string();
  let art_id = Uuid::new_v4().to_string();
  let detail_polls = Arc::new(AtomicUsize::new(0));
  let run_polls = Arc::new(AtomicUsize::new(0));

  let nb_create = nb_id.clone();
  let nb_detail = nb_id.clone();
  let src_create = src_id.clone();
  let src_detail = src_id.clone();
  let detail_counter = detail_polls.clone();
  let run_gen = run_id.clone();
  let run_get = run_id.clone();
  let run_counter = run_polls.clone();
  let art_list_id = art_id.clone();

  let router = Router::new()
    .route(
      "/v1/notebooks",
      post(move |headers: HeaderMap, Json(body): Json<Value>| {
        let nb_create = nb_create.clone();
        async move {
          assert_eq!(
            headers.get("x-yanshi-tenant").and_then(|v| v.to_str().ok()),
            Some("cli-team")
          );
          assert_eq!(
            headers.get("authorization").and_then(|v| v.to_str().ok()),
            Some("Bearer secret-tok")
          );
          Json(json!({
            "id": nb_create,
            "tenant_id": "cli-team",
            "title": body["title"],
            "created_at": "2026-11-19T00:00:00Z",
            "updated_at": "2026-11-19T00:00:00Z"
          }))
        }
      }),
    )
    .route(
      "/v1/notebooks/:id/sources",
      post(move |Path(_id): Path<String>, Json(body): Json<Value>| {
        let src_create = src_create.clone();
        async move {
          assert_eq!(body["kind"], "file");
          assert_eq!(body["filename"], "basins.csv");
          Json(json!({
            "id": src_create,
            "status": "pending",
            "kind": "file",
            "title": body["title"].as_str().unwrap_or("basins.csv")
          }))
        }
      }),
    )
    .route(
      "/v1/notebooks/:id",
      get(move |Path(id): Path<String>| {
        let nb_detail = nb_detail.clone();
        let src_detail = src_detail.clone();
        let polls = detail_counter.fetch_add(1, Ordering::SeqCst);
        async move {
          let status = if polls == 0 { "ingesting" } else { "ready" };
          Json(json!({
            "notebook": { "id": nb_detail, "title": "Tidal Research" },
            "sources": [{
              "id": src_detail,
              "notebook_id": id,
              "kind": "file",
              "title": "Basins CSV",
              "status": status
            }]
          }))
        }
      })
      .patch(|Path(id): Path<String>, Json(body): Json<Value>| async move {
        Json(json!({
          "id": id,
          "tenant_id": "cli-team",
          "title": body["title"],
          "created_at": "2026-11-19T00:00:00Z",
          "updated_at": "2026-11-19T00:01:00Z"
        }))
      })
      .delete(|| async { StatusCode::NO_CONTENT }),
    )
    .route(
      "/v1/notebooks/:id/search",
      get(
        |Path(_id): Path<String>, Query(q): Query<std::collections::HashMap<String, String>>| async move {
          assert_eq!(q.get("q").map(String::as_str), Some("Fundy"));
          assert!(q.contains_key("source_ids"));
          Json(json!({
            "chunks": [{
              "source_id": q["source_ids"],
              "chunk_id": format!("{}:0", q["source_ids"]),
              "content": "Row 1: basin: BayOfFundy | peak_m: 16.3",
              "score": 4.25
            }]
          }))
        },
      ),
    )
    .route(
      "/v1/notebooks/:id/ask",
      post(|Path(id): Path<String>, Json(body): Json<Value>| async move {
        let sid = body["source_ids"][0].as_str().unwrap_or("src");
        Json(json!({
          "notebook_id": id,
          "question": body["question"],
          "answer": "Bay of Fundy reaches 16.3 m.",
          "model": body["model"].as_str().unwrap_or("mock"),
          "citations": [{
            "source_id": sid,
            "chunk_id": format!("{sid}:0"),
            "quote": "basin: BayOfFundy | peak_m: 16.3"
          }],
          "used_chunk_count": 1
        }))
      }),
    )
    .route(
      "/v1/notebooks/:id/generate",
      post(move |Path(_id): Path<String>, Json(body): Json<Value>| {
        let run_gen = run_gen.clone();
        async move {
          assert_eq!(body["kind"], "data_table");
          assert_eq!(body["params"]["max_rows"], 10);
          Json(json!({
            "run_id": run_gen,
            "kind": "data_table",
            "status": "queued"
          }))
        }
      }),
    )
    .route(
      "/v1/runs/:run_id",
      get(move |Path(_rid): Path<String>| {
        let run_get = run_get.clone();
        let polls = run_counter.fetch_add(1, Ordering::SeqCst);
        async move {
          let status = if polls == 0 { "running" } else { "succeeded" };
          Json(json!({ "id": run_get, "status": status }))
        }
      }),
    )
    .route(
      "/v1/runs/:run_id/artifacts",
      get(move |Path(_rid): Path<String>| {
        let art_list_id = art_list_id.clone();
        async move {
          Json(json!({
            "artifacts": [{
              "id": art_list_id,
              "node_id": "table_csv",
              "name": "1.csv",
              "mime_type": "text/csv"
            }]
          }))
        }
      }),
    )
    .route(
      "/v1/runs/:run_id/artifacts/:artifact_id",
      get(|| async {
        (
          StatusCode::OK,
          [("content-type", "text/csv")],
          "Basin,Peak\nBay of Fundy,16.3\n",
        )
          .into_response()
      }),
    )
    .route(
      "/v1/notebooks/:id/sources/:source_id",
      delete(|| async { StatusCode::NO_CONTENT }),
    )
    .route(
      "/v1/notebooks/:id/artifacts/:artifact_id",
      delete(|| async { StatusCode::NO_CONTENT }),
    );

  let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
    .await
    .expect("bind loopback");
  let addr = listener.local_addr().expect("local addr");
  let server_url = format!("http://{addr}");
  let server_handle = tokio::spawn(async move {
    let _ = axum::serve(listener, router).await;
  });

  let temp = tempfile::tempdir().expect("tempdir");
  let csv_path = temp.path().join("basins.csv");
  std::fs::write(&csv_path, b"basin,peak_m\nBayOfFundy,16.3\n").expect("write csv");
  let out_dir = temp.path().join("downloaded");

  let bin_run = |cli_args: Vec<String>| {
    let server_url = server_url.clone();
    tokio::task::spawn_blocking(move || {
      let mut cmd = Command::cargo_bin("yanshi").expect("yanshi binary");
      cmd
        .arg("notebook")
        .arg("--server")
        .arg(&server_url)
        .arg("--token")
        .arg("secret-tok")
        .arg("--tenant")
        .arg("cli-team")
        .arg("--output")
        .arg("json")
        .args(cli_args);
      let output = cmd.assert().success().get_output().stdout.clone();
      serde_json::from_slice::<Value>(&output).expect("valid stdout JSON")
    })
  };

  // 1. Create & rename notebook
  let created = bin_run(vec!["create".into(), "Tidal Research".into()])
    .await
    .expect("join");
  assert_eq!(created["id"], nb_id);
  let renamed = bin_run(vec![
    "rename".into(),
    nb_id.clone(),
    "Macrotidal Estuaries".into(),
  ])
  .await
  .expect("join");
  assert_eq!(renamed["title"], "Macrotidal Estuaries");

  // 2. Upload CSV with `--wait`
  let added = bin_run(vec![
    "add-source".into(),
    nb_id.clone(),
    "--file".into(),
    csv_path.display().to_string(),
    "--title".into(),
    "Basins CSV".into(),
    "--wait".into(),
  ])
  .await
  .expect("join");
  assert_eq!(added["status"], "ready");

  // 3. Search with `--source-id`
  let hits = bin_run(vec![
    "search".into(),
    nb_id.clone(),
    "Fundy".into(),
    "--source-id".into(),
    src_id.clone(),
  ])
  .await
  .expect("join");
  assert_eq!(hits["chunks"].as_array().map(Vec::len), Some(1));

  // 4. Grounded `ask` with `--source-id` and `--history-file` multi-turn persistence
  let history_path = temp.path().join("chat-history.json");
  let ask = bin_run(vec![
    "ask".into(),
    nb_id.clone(),
    "What is the peak range?".into(),
    "--model".into(),
    "mock-ask".into(),
    "--source-id".into(),
    src_id.clone(),
    "--history-file".into(),
    history_path.display().to_string(),
  ])
  .await
  .expect("join");
  assert!(
    ask["answer"]
      .as_str()
      .unwrap_or_default()
      .contains("16.3 m")
  );
  assert_eq!(ask["citations"].as_array().map(Vec::len), Some(1));
  let saved_turns: Vec<Value> =
    serde_json::from_str(&std::fs::read_to_string(&history_path).expect("history json"))
      .expect("parse turns");
  assert_eq!(saved_turns.len(), 1);
  assert_eq!(saved_turns[0]["question"], "What is the peak range?");

  let _ask2 = bin_run(vec![
    "ask".into(),
    nb_id.clone(),
    "Which basin has that peak?".into(),
    "--model".into(),
    "mock-ask".into(),
    "--source-id".into(),
    src_id.clone(),
    "--history-file".into(),
    history_path.display().to_string(),
  ])
  .await
  .expect("join");
  let saved_turns2: Vec<Value> =
    serde_json::from_str(&std::fs::read_to_string(&history_path).expect("history json 2"))
      .expect("parse turns 2");
  assert_eq!(saved_turns2.len(), 2);

  // 5. `generate` with `--input max_rows=10` and `--output-dir`
  let generated = bin_run(vec![
    "generate".into(),
    nb_id.clone(),
    "data_table".into(),
    "--model".into(),
    "mock-ask".into(),
    "--input".into(),
    "max_rows=10".into(),
    "--source-id".into(),
    src_id.clone(),
    "--output-dir".into(),
    out_dir.display().to_string(),
  ])
  .await
  .expect("join");
  assert_eq!(generated["status"], "succeeded");
  let downloaded_csv = std::fs::read_to_string(out_dir.join("table_csv").join("1.csv"))
    .expect("downloaded artifact file");
  assert!(downloaded_csv.contains("Bay of Fundy,16.3"));
  assert!(out_dir.join("artifacts.json").is_file());

  // 6. Remove artifact, remove source & delete notebook
  let removed_art = bin_run(vec!["remove-artifact".into(), nb_id.clone(), art_id])
    .await
    .expect("join");
  assert_eq!(removed_art["deleted"], true);
  let removed = bin_run(vec!["remove-source".into(), nb_id.clone(), src_id])
    .await
    .expect("join");
  assert_eq!(removed["deleted"], true);
  let deleted = bin_run(vec!["delete".into(), nb_id]).await.expect("join");
  assert_eq!(deleted["deleted"], true);

  server_handle.abort();
}

#[tokio::test]
async fn notebook_cli_runs_against_live_postgres_gateway_when_configured() {
  let Ok(db_url) = std::env::var("YANSHI_DATABASE_TEST_URL") else {
    eprintln!("skipping live gateway notebook CLI test — set YANSHI_DATABASE_TEST_URL");
    return;
  };

  let temp = tempfile::tempdir().expect("tempdir");
  std::fs::create_dir_all(temp.path().join("runs")).expect("runs dir");
  let models_yml = temp.path().join("models.yml");
  std::fs::write(
    &models_yml,
    "models:\n  mock-cli-nb:\n    vendor: mock\n    type: text\n    model_id: mock-cli-nb\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
  )
  .expect("models.yml");
  // SAFETY: test binary sets mock config before booting `AppState`.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &models_yml);
    std::env::set_var("YANSHI_RUN_DIR", temp.path().join("runs"));
  }

  let db = yanshi_db::Database::connect_and_migrate(&db_url, 4)
    .await
    .expect("connect db");
  let state = yanshi_server::AppState::new(db);
  let router = yanshi_server::create_router(state);
  let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
    .await
    .expect("bind gateway");
  let server_url = format!("http://{}", listener.local_addr().expect("addr"));
  let server_handle = tokio::spawn(async move {
    let _ = axum::serve(listener, router).await;
  });

  let tenant = format!("cli-pg-{}", Uuid::new_v4());
  let run_cli = |args: Vec<String>| {
    let server_url = server_url.clone();
    let tenant = tenant.clone();
    tokio::task::spawn_blocking(move || {
      let mut cmd = Command::cargo_bin("yanshi").expect("yanshi bin");
      cmd
        .arg("notebook")
        .arg("--server")
        .arg(&server_url)
        .arg("--tenant")
        .arg(&tenant)
        .arg("--output")
        .arg("json")
        .args(args);
      let out = cmd.assert().success().get_output().stdout.clone();
      serde_json::from_slice::<Value>(&out).expect("json")
    })
  };

  let nb = run_cli(vec!["create".into(), "Live PG CLI Notebook".into()])
    .await
    .expect("create");
  let nb_id = nb["id"].as_str().expect("nb id").to_string();

  let csv_file = temp.path().join("stations.csv");
  std::fs::write(&csv_file, b"station,range_m\nSevern,15.0\n").expect("csv");
  let src = run_cli(vec![
    "add-source".into(),
    nb_id.clone(),
    "--file".into(),
    csv_file.display().to_string(),
    "--wait".into(),
  ])
  .await
  .expect("add-source");
  let src_id = src["id"].as_str().expect("src id").to_string();
  let chunk_id = format!("{src_id}:0");

  let mock_replies = json!([
    json!({
      "answer": "Severn reaches 15.0 m.",
      "citations": [{ "chunk_id": chunk_id, "quote": "station: Severn | range_m: 15.0" }]
    })
    .to_string(),
    json!({
      "title": "Severn Tidal Table",
      "summary": "Station range summary.",
      "columns": ["Station", "Range (m)"],
      "rows": [{
        "cells": ["Severn", "15.0"],
        "chunk_id": chunk_id,
        "quote": "station: Severn | range_m: 15.0"
      }]
    })
    .to_string()
  ]);
  // SAFETY: sequential within this test.
  unsafe {
    std::env::set_var("YANSHI_MOCK_RESPONSES", mock_replies.to_string());
  }
  yanshi_llm::Yanshi::init_with_config(models_yml.to_str().expect("utf8"))
    .await
    .expect("init mock");

  let ask = run_cli(vec![
    "ask".into(),
    nb_id.clone(),
    "What is Severn's tidal range?".into(),
    "--model".into(),
    "mock-cli-nb".into(),
  ])
  .await
  .expect("ask");
  assert_eq!(ask["citations"][0]["chunk_id"], chunk_id);

  let out_dir = temp.path().join("pg-out");
  let gen_resp = run_cli(vec![
    "generate".into(),
    nb_id.clone(),
    "data_table".into(),
    "--model".into(),
    "mock-cli-nb".into(),
    "--output-dir".into(),
    out_dir.display().to_string(),
  ])
  .await
  .expect("generate");
  assert_eq!(gen_resp["status"], "succeeded");
  assert_eq!(gen_resp["artifacts"].as_array().map(Vec::len), Some(3));

  let nb_arts = run_cli(vec!["artifacts".into(), nb_id.clone()])
    .await
    .expect("artifacts");
  assert_eq!(
    nb_arts["artifacts"][0]["deliverable"]["title"],
    "Severn Tidal Table"
  );

  let deleted = run_cli(vec!["delete".into(), nb_id]).await.expect("delete");
  assert_eq!(deleted["deleted"], true);

  server_handle.abort();
}
