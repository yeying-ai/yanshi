//! Integration tests for Workflow Studio inspection, debug stepping, and inline preview routes.

use std::time::Duration;

use axum::{
  body::Body,
  http::{Request, StatusCode, header::CONTENT_TYPE},
};
use serde_json::{Value, json};
use sqlx::postgres::PgPoolOptions;
use tokio::time::sleep;
use tower::ServiceExt;
use uuid::Uuid;
use yanshi_db::Database;
use yanshi_server::{AppState, create_router, tenant::TENANT_HEADER};

fn lazy_state() -> AppState {
  let pool = PgPoolOptions::new()
    .connect_lazy("postgres://postgres:postgres@localhost:5432/yanshi_test")
    .expect("lazy pg");
  AppState::new(Database {
    pool,
    read_pool: None,
  })
}

#[tokio::test]
async fn inspect_workflow_route_returns_topology_inputs_and_levels() {
  let app = create_router(lazy_state());
  let workflow = r#"
name: Studio Inspect Demo
inputs:
  topic:
    description: Presentation subject
    required: true
  slides:
    default: 5
nodes:
  - id: draft
    type: template
    parameters:
      template: "Topic: {{ topic }}"
  - id: summary
    type: template
    dependencies: [draft]
    input_mapping:
      draft_out: "{{ nodes.draft.outputs.output }}"
    parameters:
      template: "Summary: {{ draft_out }}"
"#;

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/workflows/inspect")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "workflow": workflow }).to_string()))
        .expect("request"),
    )
    .await
    .expect("response");

  assert_eq!(response.status(), StatusCode::OK);
  let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
    .await
    .expect("body");
  let body: Value = serde_json::from_slice(&bytes).expect("json");

  assert_eq!(body["valid"], json!(true));
  assert_eq!(body["name"], json!("Studio Inspect Demo"));
  assert_eq!(body["inputs"].as_array().expect("inputs").len(), 2);
  assert_eq!(body["inputs"][0]["name"], json!("topic"));
  assert_eq!(body["inputs"][0]["required"], json!(true));
  assert_eq!(body["execution_order"], json!(["draft", "summary"]));
  assert_eq!(body["levels"], json!([["draft"], ["summary"]]));
  assert_eq!(body["edges"][0]["from"], json!("draft"));
  assert_eq!(body["edges"][0]["to"], json!("summary"));
  assert_eq!(body["edges"][0]["kind"], json!("input_mapping"));
}

#[tokio::test]
async fn inspect_workflow_route_reports_invalid_yaml_without_500() {
  let app = create_router(lazy_state());
  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/workflows/inspect")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(
          json!({ "workflow": "name: [broken" }).to_string(),
        ))
        .expect("request"),
    )
    .await
    .expect("response");

  assert_eq!(response.status(), StatusCode::OK);
  let bytes = axum::body::to_bytes(response.into_body(), 256 * 1024)
    .await
    .expect("body");
  let body: Value = serde_json::from_slice(&bytes).expect("json");
  assert_eq!(body["valid"], json!(false));
  assert!(
    !body["diagnostics"]["issues"]
      .as_array()
      .expect("issues")
      .is_empty()
  );
}

#[tokio::test]
async fn debug_run_steps_node_by_node_when_live_db_is_configured() {
  let Ok(url) = std::env::var("YANSHI_DATABASE_TEST_URL") else {
    eprintln!("skipping debug_run_steps_node_by_node — set YANSHI_DATABASE_TEST_URL");
    return;
  };
  let Ok(db) = Database::connect_and_migrate(&url, 4).await else {
    return;
  };
  let state = AppState::new(db);
  let tenant = format!("tenant-debug-{}", Uuid::new_v4());
  let workflow = r#"
name: Two Step Debug
nodes:
  - id: first
    type: template
    parameters:
      template: "alpha"
  - id: second
    type: template
    dependencies: [first]
    input_mapping:
      prev: "{{ nodes.first.outputs.output }}"
    parameters:
      template: "{{ prev }}-beta"
"#;

  let submit_res = create_router(state.clone())
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/runs")
        .header(CONTENT_TYPE, "application/json")
        .header(TENANT_HEADER, &tenant)
        .body(Body::from(
          json!({
            "workflow": workflow,
            "run_mode": "debug",
          })
          .to_string(),
        ))
        .expect("submit req"),
    )
    .await
    .expect("submit res");
  assert_eq!(submit_res.status(), StatusCode::OK);
  let submit_bytes = axum::body::to_bytes(submit_res.into_body(), 64 * 1024)
    .await
    .expect("submit body");
  let submit_json: Value = serde_json::from_slice(&submit_bytes).expect("json");
  let run_id = submit_json["run_id"].as_str().expect("run_id");

  // Wait for `first` to pause at AfterNode.
  let mut paused_first = false;
  for _ in 0..40 {
    let dbg_res = create_router(state.clone())
      .oneshot(
        Request::builder()
          .uri(format!("/v1/runs/{run_id}/debug"))
          .header(TENANT_HEADER, &tenant)
          .body(Body::empty())
          .expect("dbg req"),
      )
      .await
      .expect("dbg res");
    let dbg_bytes = axum::body::to_bytes(dbg_res.into_body(), 64 * 1024)
      .await
      .expect("dbg body");
    let dbg_json: Value = serde_json::from_slice(&dbg_bytes).expect("json");
    if dbg_json["snapshot"]["paused_at"]["node_id"] == json!("first") {
      paused_first = true;
      break;
    }
    sleep(Duration::from_millis(25)).await;
  }
  assert!(paused_first, "expected run to pause after `first`");

  // Continue remaining nodes to completion.
  let cont_res = create_router(state.clone())
    .oneshot(
      Request::builder()
        .method("POST")
        .uri(format!("/v1/runs/{run_id}:debug"))
        .header(CONTENT_TYPE, "application/json")
        .header(TENANT_HEADER, &tenant)
        .body(Body::from(json!({ "action": "continue" }).to_string()))
        .expect("continue req"),
    )
    .await
    .expect("continue res");
  assert_eq!(cont_res.status(), StatusCode::OK);

  let mut succeeded = false;
  for _ in 0..40 {
    let get_res = create_router(state.clone())
      .oneshot(
        Request::builder()
          .uri(format!("/v1/runs/{run_id}"))
          .header(TENANT_HEADER, &tenant)
          .body(Body::empty())
          .expect("get req"),
      )
      .await
      .expect("get res");
    let get_bytes = axum::body::to_bytes(get_res.into_body(), 64 * 1024)
      .await
      .expect("get body");
    let get_json: Value = serde_json::from_slice(&get_bytes).expect("json");
    if get_json["status"] == json!("succeeded") {
      succeeded = true;
      break;
    }
    sleep(Duration::from_millis(25)).await;
  }
  assert!(
    succeeded,
    "expected debug run to reach succeeded after continue"
  );
}
