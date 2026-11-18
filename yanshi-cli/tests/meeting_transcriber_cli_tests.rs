//! A4 acceptance: `yanshi workflow run examples/applications/meeting-transcriber/workflow.yml`
//! under scripted `mock-asr` + `mock-model`, verifying deterministic multi-file
//! extraction (`transcript_md`, `summary_md`, `summary_html`, `action_items_md`,
//! `action_items_xlsx`, `action_items_csv`, and `.srt`).

use assert_cmd::Command;
use serde_json::json;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

const REGISTRY: &str = "\
models:
  mock-model:
    vendor: mock
    type: text
    model_id: mock-model
  mock-asr:
    vendor: mock
    type: asr
    model_id: mock-asr
providers:
  mock:
    api_key_env: MOCK_API_KEY
";

#[test]
fn meeting_transcriber_validates_strict_and_exports_minutes_and_action_trackers() {
  let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
    .parent()
    .expect("root");
  let workflow = repo_root.join("examples/applications/meeting-transcriber/workflow.yml");
  let fixture =
    repo_root.join("examples/applications/meeting-transcriber/fixtures/standup_sample.mp3");

  Command::cargo_bin("yanshi")
    .expect("binary")
    .args(["workflow", "validate"])
    .arg(&workflow)
    .arg("--strict")
    .assert()
    .success();

  let work = TempDir::new().expect("tempdir");
  fs::create_dir_all(work.path().join("home/.yanshi")).expect("config dir");
  fs::write(work.path().join("home/.yanshi/models.yml"), REGISTRY).expect("models.yml");

  let summary_reply = json!({
    "meeting_title": "Weekly Platform Standup",
    "executive_summary": "Reviewed PDF parser rollout and staging Postgres pool tuning ahead of Monday RC cut.",
    "decisions": ["Lock release candidate cut for Monday afternoon"],
    "topics": [
      {
        "topic": "Staging Load Test",
        "summary": "Hit connection pool ceiling at 200 concurrent runs.",
        "open_questions": "Verify max_connections headroom on replica."
      }
    ],
    "speaker_turns": [
      {
        "speaker": "Alice",
        "timestamp_hint": "00:00:00",
        "utterance": "We shipped the PDF notebook parser and will add the retry dashboard by Friday."
      },
      {
        "speaker": "Bob",
        "timestamp_hint": "00:00:07",
        "utterance": "I will tune the Postgres pool config by Wednesday."
      }
    ]
  })
  .to_string();

  let actions_reply = json!({
    "items": [
      {
        "id": "AI-01",
        "owner": "Alice",
        "action": "add the retry metrics dashboard",
        "deadline": "Friday",
        "priority": "P1",
        "evidence": "Alice [00:00:00]"
      },
      {
        "id": "AI-02",
        "owner": "Bob",
        "action": "tune the Postgres pool config",
        "deadline": "Wednesday",
        "priority": "P0",
        "evidence": "Bob [00:00:07]"
      }
    ]
  })
  .to_string();

  let out_dir = work.path().join("out");
  Command::cargo_bin("yanshi")
    .expect("binary")
    .env("HOME", work.path().join("home"))
    .env(
      "YANSHI_MODELS_CONFIG",
      work.path().join("home/.yanshi/models.yml"),
    )
    .env("YANSHI_TRACE_DIR", work.path().join("traces"))
    .env(
      "YANSHI_MOCK_RESPONSES",
      json!([summary_reply, actions_reply]).to_string(),
    )
    .env_remove("YANSHI_SERVER_URL")
    .args(["workflow", "run"])
    .arg(&workflow)
    .args(["--model", "mock-model"])
    .arg("--input")
    .arg("audio_source")
    .arg(fixture.as_os_str())
    .args(["--input", "asr_model", "mock-asr"])
    .args(["--input", "export_pdf", "false"])
    .arg("--run-dir")
    .arg(work.path().join("runs"))
    .arg("--output-dir")
    .arg(&out_dir)
    .assert()
    .success();

  for rel in [
    "transcribe/1.srt",
    "transcript_md/1.md",
    "summary_md/1.md",
    "summary_html/1.html",
    "action_items_md/1.md",
    "action_items_xlsx/1.xlsx",
    "action_items_csv/1.csv",
  ] {
    let p = out_dir.join(rel);
    let meta = fs::metadata(&p).unwrap_or_else(|e| panic!("missing {rel}: {e}"));
    assert!(meta.len() > 0, "{rel} should be non-empty");
  }

  let actions_md =
    fs::read_to_string(out_dir.join("action_items_md/1.md")).expect("read action_items_md");
  assert!(
    actions_md.contains("**Alice** will add the retry metrics dashboard by **Friday**"),
    "{actions_md}"
  );
  let xlsx_bytes = fs::read(out_dir.join("action_items_xlsx/1.xlsx")).expect("read xlsx");
  assert!(xlsx_bytes.starts_with(b"PK\x03\x04"), "valid OOXML zip");
}
