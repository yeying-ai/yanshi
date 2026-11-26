//! Y4.4 acceptance: `yanshi skill run examples/applications/content-studio`
//! with `--input` facts and `--output-dir`, under a scripted mock model.
//!
//! The mock serves planner and generator calls from one FIFO queue, in call
//! order. Because `WorkflowTool`s run concurrently within a single tool batch
//! (AJ1), each generator tool call is scheduled in its own planner turn so
//! concurrent generators never race on the shared `YANSHI_MOCK_RESPONSES`
//! queue. `YANSHI_MOCK_TOOL_CALLS` and `YANSHI_MOCK_RESPONSES` advance
//! together one entry per model call.

use assert_cmd::Command;
use predicates::prelude::*;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const SOURCE: &str = "Tides are caused by the moon. High tide comes twice daily.";

fn registry() -> &'static str {
  "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\n  mock-tts:\n    vendor: mock\n    type: tts\n    model_id: mock-tts\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n"
}

fn outline() -> String {
  json!({"title": "Tides", "sections": [
    {"heading": "Cause", "brief": "why"}, {"heading": "Rhythm", "brief": "how often"}
  ]})
  .to_string()
}

fn section(text: &str, quote: &str) -> String {
  json!({"markdown": text, "quotes": [{"chunk_id": "", "quote": quote}]}).to_string()
}

fn tree() -> String {
  json!({"label": "Tides", "children": [{"label": "Moon"}, {"label": "Rhythm"}]}).to_string()
}

/// Run content-studio with `steps` (each `(tool_call, generator_replies)`
/// scheduled on its own planner turn so generators consume the global mock
/// FIFO queue deterministically) followed by the planner's `final_reply`.
fn run_studio(
  want: &str,
  steps: Vec<(Value, Vec<String>)>,
  final_reply: &str,
) -> (TempDir, PathBuf) {
  let work = TempDir::new().expect("work");
  fs::create_dir_all(work.path().join("home/.yanshi")).expect("dir");
  fs::write(work.path().join("home/.yanshi/models.yml"), registry()).expect("registry");
  let mut all_replies = Vec::new();
  let mut tool_calls = Vec::new();
  for (call, replies) in steps {
    all_replies.push("(unused — native tool call)".to_string());
    tool_calls.push(json!([call]));
    for reply in replies {
      all_replies.push(reply);
      tool_calls.push(json!([]));
    }
  }
  all_replies.push(final_reply.to_string());
  tool_calls.push(json!([]));
  let out = work.path().join("out");
  Command::cargo_bin("yanshi")
    .expect("binary")
    .env("HOME", work.path().join("home"))
    .env(
      "YANSHI_MODELS_CONFIG",
      work.path().join("home/.yanshi/models.yml"),
    )
    .env("YANSHI_TRACE_DIR", work.path().join("traces"))
    .env("YANSHI_MOCK_RESPONSES", json!(all_replies).to_string())
    .env(
      "YANSHI_MOCK_TOOL_CALLS",
      Value::Array(tool_calls).to_string(),
    )
    .env_remove("YANSHI_SERVER_URL")
    .args(["skill", "run"])
    .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/applications/content-studio"))
    .args(["--model", "mock-model"])
    .arg(format!("--input=text={SOURCE}"))
    .args(["--input", "want", want])
    .arg("--run-dir")
    .arg(work.path().join("runs"))
    .arg("--output-dir")
    .arg(&out)
    .assert()
    .success()
    .stdout(predicate::str::contains(
      "🧰 Workflow tools: report, mindmap, audio_overview",
    ))
    .stdout(predicate::str::contains("🤖 Agent: done"));
  (work, out)
}

fn files_under(dir: &Path) -> Vec<String> {
  let mut found = Vec::new();
  if let Ok(entries) = fs::read_dir(dir) {
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() {
        found.extend(files_under(&path));
      } else {
        found.push(path.display().to_string());
      }
    }
  }
  found.sort();
  found
}

#[test]
fn content_studio_makes_a_report_and_a_mindmap() {
  let (_work, out) = run_studio(
    "report,mindmap",
    vec![
      (
        json!({"id": "c1", "name": "report", "arguments": {"text": SOURCE, "sections": 2, "pdf": false}}),
        vec![
          outline(),
          section("The moon pulls the sea.", "Tides are caused by the moon."),
          section("Two highs a day.", "High tide comes twice daily."),
        ],
      ),
      (
        json!({"id": "c2", "name": "mindmap", "arguments": {"text": SOURCE}}),
        vec![tree()],
      ),
    ],
    "done",
  );
  let files = files_under(&out);
  for expected in [
    "report/report_md/1.md",
    "report/report_html/1.html",
    "mindmap/map_svg/1.svg",
    "mindmap/map_png/1.png",
    "mindmap/map_html/1.html",
  ] {
    assert!(
      files.iter().any(|f| f.ends_with(expected)),
      "{expected} in {files:?}"
    );
  }
  let markdown = fs::read_to_string(out.join("report/report_md/1.md")).expect("md");
  assert!(markdown.starts_with("# Tides"), "{markdown}");
}

/// The full catalogue: needs ffmpeg (audio), pandoc and typst (PDF).
#[test]
#[ignore = "needs ffmpeg, pandoc and typst"]
fn content_studio_makes_all_three_deliverables() {
  let script = json!({"title": "Tides", "lines": [
    {"speaker": "host", "text": "Welcome."},
    {"speaker": "guest", "text": "Tides follow the moon."}
  ]})
  .to_string();
  let (_work, out) = run_studio(
    "report,mindmap,audio_overview",
    vec![
      (
        json!({"id": "c1", "name": "report", "arguments": {"text": SOURCE, "sections": 2}}),
        vec![
          outline(),
          section("The moon pulls the sea.", "Tides are caused by the moon."),
          section("Two highs a day.", "High tide comes twice daily."),
        ],
      ),
      (
        json!({"id": "c2", "name": "mindmap", "arguments": {"text": SOURCE}}),
        vec![tree()],
      ),
      (
        json!({"id": "c3", "name": "audio_overview", "arguments": {"text": SOURCE, "tts_model": "mock-tts"}}),
        vec![script],
      ),
    ],
    "done",
  );
  let files = files_under(&out);
  for expected in [
    "report/report_pdf/1.pdf",
    "mindmap/map_png/1.png",
    "audio_overview/episode/1.mp3",
  ] {
    assert!(
      files.iter().any(|f| f.ends_with(expected)),
      "{expected} in {files:?}"
    );
  }
}
