//! End-to-end test for `examples/generators/tech_slides_video.yml`:
//! LLM structured `SlideDeck` → `render_slides` (progressive 1080p PNGs +
//! Pandoc Markdown + HTML player) → `tts` → `render_video` (multi-frame
//! progressive scenes) + `render_document` (`.pptx`) → `Deliverable`.

use std::collections::HashMap;
use std::io::Read as _;

use serde_json::json;
use yanshi_config::content::{Deliverable, DeliverableKind};
use yanshi_config::executor::{
  apply_declared_inputs, build_flow_from_definition, parse_workflow_definition,
};
use yanshi_core::{FlowExecutionConfig, FlowExt, value::FlowValue};

#[tokio::test]
async fn tech_slides_video_generator_produces_1080p_mp4_pptx_and_html_player() {
  for bin in ["ffmpeg", "ffprobe", "pandoc"] {
    if yanshi_nodes::common::sandboxed::find_tool(bin).is_none() {
      eprintln!("skipping: {bin} is not installed");
      return;
    }
  }

  let dir = std::env::temp_dir().join(format!("yanshi-tech-slides-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("dir");
  let registry = dir.join("models.yml");
  std::fs::write(
    &registry,
    "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\n  mock-tts:\n    vendor: mock\n    type: tts\n    model_id: mock-tts\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
  )
  .expect("registry");

  let deck = json!({
    "title": "Rust Traits 精讲",
    "subtitle": "零成本契约与虚表分发",
    "slides": [
      {
        "section": "01 介绍",
        "title": "Traits：共享行为契约",
        "left": {
          "kind": "code",
          "language": "rust",
          "code": "pub trait Summary {\n  fn summarize(&self) -> String;\n}"
        },
        "right": {
          "kind": "flowchart",
          "direction": "LR",
          "nodes": [
            {"id": "t", "label": "Tweet"},
            {"id": "s", "label": "trait Summary"}
          ],
          "edges": [{"from": "t", "to": "s", "label": "impl"}]
        },
        "takeaway": "数据布局与行为契约正交解耦。",
        "notes": "首先定义 Summary 契约。随后任意类型可独立实现该能力。"
      }
    ]
  });

  // SAFETY: single-test binary setting env before first model initialization.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &registry);
    std::env::set_var(
      "YANSHI_MOCK_RESPONSES",
      json!([deck.to_string()]).to_string(),
    );
  }
  yanshi_llm::Yanshi::init_with_config(registry.to_str().expect("utf-8"))
    .await
    .expect("mock registry");

  let template = std::fs::read_to_string(
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../examples/generators/tech_slides_video.yml"),
  )
  .expect("template");
  let definition = parse_workflow_definition(&template).expect("validates");
  let mut inputs: HashMap<String, FlowValue> = [
    ("model", json!("mock-model")),
    ("tts_model", json!("mock-tts")),
    ("export_pptx", json!(true)),
    ("export_pdf", json!(false)),
  ]
  .into_iter()
  .map(|(k, v)| (k.to_string(), FlowValue::Json(v)))
  .collect();
  apply_declared_inputs(&definition, &mut inputs).expect("defaults");

  let runs = tempfile::TempDir::new().expect("tempdir");
  let state = build_flow_from_definition(&definition, Some("mock-model"))
    .expect("builds")
    .execute_from_inputs_with_config(
      inputs,
      FlowExecutionConfig::serial().with_run_base_dir(runs.path()),
    )
    .await
    .expect("runs");

  let film = state
    .get("film")
    .unwrap_or_else(|| panic!("missing film in state: {state:?}"))
    .as_ref()
    .unwrap_or_else(|e| panic!("film failed: {e} (state: {state:?})"));
  let FlowValue::File { path: mp4_path, .. } = &film["film_output"] else {
    panic!("expected mp4 file");
  };
  let probe = std::process::Command::new("ffprobe")
    .args([
      "-v",
      "error",
      "-select_streams",
      "v:0",
      "-show_entries",
      "stream=width,height",
      "-of",
      "csv=p=0",
    ])
    .arg(mp4_path)
    .output()
    .expect("ffprobe");
  assert_eq!(String::from_utf8_lossy(&probe.stdout).trim(), "1920,1080");

  let FlowValue::File {
    path: pptx_path, ..
  } = &state["deck_pptx"].as_ref().expect("pptx")["deck_pptx_output"]
  else {
    panic!("expected pptx file");
  };
  let mut zip = zip::ZipArchive::new(std::fs::File::open(pptx_path).expect("pptx")).expect("zip");
  let mut notes_xml = String::new();
  for i in 0..zip.len() {
    let mut entry = zip.by_index(i).expect("entry");
    let name = entry.name().expect("utf-8 zip entry").into_owned();
    if name.starts_with("ppt/notesSlides/") && name.ends_with(".xml") {
      entry.read_to_string(&mut notes_xml).expect("utf-8");
    }
  }
  assert!(
    notes_xml.contains("Summary"),
    "speaker notes preserved in pptx"
  );

  let FlowValue::Json(val) = &state["deliverable"].as_ref().expect("deliverable")["deliverable"]
  else {
    panic!("deliverable json");
  };
  let deliverable = Deliverable::from_value(val.clone()).expect("valid Deliverable");
  assert_eq!(deliverable.kind, DeliverableKind::VideoOverview);
  assert_eq!(deliverable.files.len(), 3);
}
