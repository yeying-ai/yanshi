//! End-to-end checks for `GET /v1/skills` + `POST /v1/skills/{name}:run`.
//!
//! Skill resolution requires only the in-memory catalog, so these tests
//! work without a live DB *for the listing path*. The submit-run path
//! reuses the runs table and is gated by `YANSHI_DATABASE_TEST_URL`
//! the same way as the other server e2e tests.

use axum::{
  body::Body,
  http::{Request, StatusCode, header::CONTENT_TYPE},
};
use serde_json::json;
use tower::ServiceExt;
use yanshi_db::Database;
use yanshi_server::{AppState, SkillCatalog, create_router};

fn live_url() -> Option<String> {
  std::env::var("YANSHI_DATABASE_TEST_URL").ok()
}

async fn fresh_state() -> Option<AppState> {
  let url = live_url()?;
  let db = Database::connect_and_migrate(&url, 4).await.ok()?;
  // No TRUNCATE: tests run in parallel and each uses a fresh run id, so a
  // reset here deleted a sibling's run mid-poll (CI saw its GET return 404).
  Some(AppState::new(db))
}

#[tokio::test]
async fn list_skills_returns_empty_when_catalog_unset() {
  let Some(state) = fresh_state().await else {
    eprintln!("skipping list_skills_returns_empty_when_catalog_unset");
    return;
  };
  let app = create_router(state);

  let response = app
    .oneshot(
      Request::builder()
        .uri("/v1/skills")
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let bytes = axum::body::to_bytes(response.into_body(), 4096)
    .await
    .unwrap();
  let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(body["skills"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn run_skill_returns_404_when_skill_unknown() {
  let Some(state) = fresh_state().await else {
    eprintln!("skipping run_skill_returns_404_when_skill_unknown");
    return;
  };
  let app = create_router(state.with_skills(SkillCatalog::empty()));

  let response = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/skills/missing:run")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"input": "hi"}).to_string()))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::NOT_FOUND);
  let bytes = axum::body::to_bytes(response.into_body(), 4096)
    .await
    .unwrap();
  let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
  assert_eq!(body["error"]["code"], "not_found");
}

/// W0.2 regression: `POST /v1/skills/{name}:run` used to persist a
/// `runs` row with `workflow = "@skill:<name>"` and hand it to
/// `FlowRunExecutor`, which tried to parse that marker as a Flow
/// YAML/JSON definition and always failed. Drives the route against a
/// real (mock-provider) skill end to end and asserts the run reaches
/// `succeeded` with the mock's answer recorded.
///
/// SAFETY: mutates process-wide env vars (`YANSHI_MODELS_CONFIG`,
/// `MOCK_API_KEY`, `YANSHI_MOCK_RESPONSES`) that
/// `yanshi_llm::Yanshi::init()` reads exactly once per process
/// (behind an internal `OnceCell`). This is the only test in this binary
/// that touches them, so there is no cross-test race within this file —
/// each `tests/*.rs` file is its own process.
#[tokio::test]
async fn run_skill_executes_the_agent_and_reaches_succeeded() {
  let Some(state) = fresh_state().await else {
    eprintln!("skipping run_skill_executes_the_agent_and_reaches_succeeded");
    return;
  };

  let tmp = tempfile::tempdir().unwrap();
  let skill_dir = tmp.path().join("greeter");
  std::fs::create_dir_all(&skill_dir).unwrap();
  std::fs::write(
    skill_dir.join("skill.toml"),
    r#"
[skill]
name = "greeter"
version = "0.1.0"
description = "test skill for the W0.2 run_skill regression test"

[persona]
role = "Answer very briefly."

[model]
name = "mock-chat"
"#,
  )
  .unwrap();

  let index_path = tmp.path().join("skills.index.toml");
  std::fs::write(
    &index_path,
    r#"
schema_version = 1
name = "test-registry"

[[skills]]
name = "greeter"
version = "0.1.0"
path = "greeter"
"#,
  )
  .unwrap();
  let index = yanshi_skills::SkillRegistryIndex::load(&index_path).unwrap();
  let catalog = SkillCatalog::from_index(index, index_path);

  let models_config = tmp.path().join("models.yml");
  std::fs::write(
    &models_config,
    "models:\n  mock-chat: { vendor: mock, type: text, model_id: mock-chat }\n\
     providers:\n  mock: { api_key_env: MOCK_API_KEY }\n",
  )
  .unwrap();
  // SAFETY: see the fn-level doc comment above.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &models_config);
    std::env::set_var("MOCK_API_KEY", "x");
    std::env::set_var(
      "YANSHI_MOCK_RESPONSES",
      r#"["{\"thought\":\"done\",\"answer\":\"hello from the skill\"}"]"#,
    );
  }

  let app = create_router(state.with_skills(catalog));
  let response = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/skills/greeter:run")
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json!({"input": "hi"}).to_string()))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(response.status(), StatusCode::OK);
  let bytes = axum::body::to_bytes(response.into_body(), 4096)
    .await
    .unwrap();
  let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
  let run_id = body["run_id"].as_str().expect("run_id").to_string();

  let mut terminal: Option<serde_json::Value> = None;
  for _ in 0..60 {
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
    let get = app
      .clone()
      .oneshot(
        Request::builder()
          .uri(format!("/v1/runs/{run_id}"))
          .body(Body::empty())
          .unwrap(),
      )
      .await
      .unwrap();
    assert_eq!(get.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(get.into_body(), 4096).await.unwrap();
    let payload: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    if payload["status"].as_str() != Some("running") && payload["status"].as_str() != Some("queued")
    {
      terminal = Some(payload);
      break;
    }
  }
  // SAFETY: see the fn-level doc comment above; cleanup regardless of outcome.
  unsafe {
    std::env::remove_var("YANSHI_MODELS_CONFIG");
    std::env::remove_var("MOCK_API_KEY");
    std::env::remove_var("YANSHI_MOCK_RESPONSES");
  }
  let final_row = terminal.expect("run reached a terminal status within timeout");
  assert_eq!(
    final_row["status"].as_str(),
    Some("succeeded"),
    "unexpected terminal row: {final_row}"
  );

  let history = app
    .oneshot(
      Request::builder()
        .uri(format!("/v1/runs/{run_id}/events/history"))
        .body(Body::empty())
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(history.status(), StatusCode::OK);
  let bytes = axum::body::to_bytes(history.into_body(), 4096)
    .await
    .unwrap();
  let events: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
  let events = events.as_array().expect("history is array");
  let completed = events
    .iter()
    .find(|e| e["kind"].as_str() == Some("skill_run_completed"))
    .expect("skill_run_completed event present");
  assert_eq!(
    completed["payload"]["answer"].as_str(),
    Some("hello from the skill")
  );
}

struct ParkedExecutor;

#[async_trait::async_trait]
impl yanshi_server::RunExecutor for ParkedExecutor {
  async fn execute(&self, _ctx: yanshi_server::RunContext) {}
}

#[tokio::test]
async fn run_skill_enforces_tenant_admission_and_persists_run_dir() {
  let Some(state) = fresh_state().await else {
    eprintln!("skipping run_skill_enforces_tenant_admission_and_persists_run_dir");
    return;
  };
  let tmp = tempfile::tempdir().unwrap();
  let skill_dir = tmp.path().join("greeter");
  std::fs::create_dir_all(&skill_dir).unwrap();
  std::fs::write(
    skill_dir.join("skill.toml"),
    r#"
[skill]
name = "greeter"
version = "0.1.0"
description = "admission test skill"

[persona]
role = "Answer briefly."

[model]
name = "mock-chat"
"#,
  )
  .unwrap();
  let index_path = tmp.path().join("skills.index.toml");
  std::fs::write(
    &index_path,
    "schema_version = 1\nname = \"test-registry\"\n\n[[skills]]\nname = \"greeter\"\nversion = \"0.1.0\"\npath = \"greeter\"\n",
  )
  .unwrap();
  let index = yanshi_skills::SkillRegistryIndex::load(&index_path).unwrap();
  let catalog = SkillCatalog::from_index(index, index_path);

  let mut defaults = yanshi_tools::SecurityProfile::Local.defaults();
  defaults.run_admission.max_concurrent_runs_per_tenant = 1;
  defaults
    .run_admission
    .max_run_submissions_per_minute_per_tenant = 10;

  let repos = state.repos.clone();
  let app = create_router(
    state
      .with_security_defaults(defaults)
      .with_skills(catalog)
      .with_executor(std::sync::Arc::new(ParkedExecutor)),
  );
  let tenant_a = format!("tenant-skill-{}", uuid::Uuid::new_v4());
  let tenant_b = format!("tenant-skill-{}", uuid::Uuid::new_v4());

  // First submission for tenant_a is admitted and records run_dir + structured inputs.
  let first = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/skills/greeter:run")
        .header(CONTENT_TYPE, "application/json")
        .header("X-Yanshi-Tenant", &tenant_a)
        .body(Body::from(
          json!({"inputs": {"want": "report", "sections": 3}}).to_string(),
        ))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(first.status(), StatusCode::OK);
  let first_body: serde_json::Value =
    serde_json::from_slice(&axum::body::to_bytes(first.into_body(), 4096).await.unwrap()).unwrap();
  let run_id: uuid::Uuid = first_body["run_id"]
    .as_str()
    .expect("run_id")
    .parse()
    .expect("uuid");

  use yanshi_db::RunRepo as _;
  let stored = repos
    .runs
    .get(run_id)
    .await
    .expect("db get")
    .expect("run exists");
  assert!(
    stored
      .run_dir
      .as_deref()
      .is_some_and(|dir| dir.contains(&run_id.to_string())),
    "skill run must persist run_dir for retention cleanup, got {:?}",
    stored.run_dir
  );
  assert!(
    stored.workflow.contains("- sections: 3") && stored.workflow.contains("- want: report"),
    "structured inputs must reach skill workflow prompt: {}",
    stored.workflow
  );

  // Second concurrent submission for tenant_a exceeds max_concurrent_runs_per_tenant = 1 → 429.
  let second = app
    .clone()
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/skills/greeter:run")
        .header(CONTENT_TYPE, "application/json")
        .header("X-Yanshi-Tenant", &tenant_a)
        .body(Body::from(json!({"input": "second"}).to_string()))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(second.status(), StatusCode::TOO_MANY_REQUESTS);

  // Another tenant is isolated and still admitted.
  let other_tenant = app
    .oneshot(
      Request::builder()
        .method("POST")
        .uri("/v1/skills/greeter:run")
        .header(CONTENT_TYPE, "application/json")
        .header("X-Yanshi-Tenant", &tenant_b)
        .body(Body::from(json!({"input": "hello"}).to_string()))
        .unwrap(),
    )
    .await
    .unwrap();
  assert_eq!(other_tenant.status(), StatusCode::OK);
}
