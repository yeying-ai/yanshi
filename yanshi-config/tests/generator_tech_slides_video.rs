//! End-to-end test for `examples/generators/tech_slides_video.yml`:
//! LLM structured `SlideDeck` (with decoupled `spoken` sightline cues + `caption`)
//! → `render_slides` (section opener cards + progressive 1080p PNGs + PiP safe zone +
//! Pandoc Markdown + HTML player) → per-step `tts` → `render_video` (step/slide
//! breathing pauses + smooth slide fades + rounded PiP presenter overlay + SRT/VTT)
//! + `render_document` (`.pptx`) → `Deliverable`.

use std::collections::HashMap;
use std::io::Read as _;

use serde_json::json;
use yanshi_config::content::{Deliverable, DeliverableKind};
use yanshi_config::executor::{
  apply_declared_inputs, build_flow_from_definition, parse_workflow_definition,
};
use yanshi_core::{FlowExecutionConfig, FlowExt, value::FlowValue};

#[tokio::test]
async fn tech_slides_video_generator_produces_1080p_mp4_pip_pptx_and_html_player() {
  for bin in ["ffmpeg", "ffprobe"] {
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
    "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\n  mock-tts:\n    vendor: mock\n    type: tts\n    model_id: mock-tts\n  mock-image:\n    vendor: mock\n    type: text_to_image\n    model_id: mock-image\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
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
          "code": "pub trait Summary {\n  fn summarize(&self) -> String;\n}\n\npub fn notify<T: Summary>(item: &T) {\n  println!(\"{}\", item.summarize());\n}",
          "callouts": [{"line": 1, "text": "正交能力契约", "tone": "rust"}]
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
        "visual_mode": "structured_panel",
        "takeaway": "数据布局与行为契约正交解耦。",
        "notes": "首先定义 Summary 契约。随后任意类型可独立实现该能力。",
        "steps": [
          {
            "caption": "声明 Summary 共享行为契约",
            "spoken": "请大家先看屏幕左侧第一到第三行，这里声明了不含任何数据字段的 Summary 契约。",
            "code_lines": [1, 3],
            "focus_ids": ["t"]
          },
          {
            "caption": "编译期泛型静态单态化调用",
            "spoken": "再往下看第五行，泛型函数通过编译期单态化实现零虚表开销调用。",
            "code_lines": [5, 7],
            "focus_ids": ["s"],
            "show_takeaway": true
          }
        ]
      },
      {
        "section": "02 判据",
        "title": "静态分发 vs 动态分发选型",
        "left": {
          "kind": "bullets",
          "items": [
            {"badge": "Static", "heading": "热路径泛型", "body": "支持跨函数 LLVM 内联优化"}
          ]
        },
        "visual_mode": "structured_panel",
        "takeaway": "热路径优先泛型单态化，异构容器按需使用胖指针。",
        "notes": "最后总结两者的核心工程选型边界。",
        "steps": [
          {
            "caption": "热路径优先静态单态化",
            "spoken": "翻到第二章，我们在热路径循环中优先采用静态单态化以解锁内联优化。",
            "show_takeaway": true
          }
        ]
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
    ("chat_model", json!("mock-model")),
    ("tts_model", json!("mock-tts")),
    ("image_model", json!("mock-image")),
    ("video_model", json!("")),
    ("presenter_mode", json!("pip")),
    ("enable_web_search", json!(false)),
    ("enable_d2_diagram", json!(false)),
    ("enable_illustrations", json!(false)),
    ("section_cards", json!(true)),
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
  assert!(
    matches!(film.get("film_vtt"), Some(FlowValue::File { .. })),
    "expected WebVTT subtitle artifact alongside SRT"
  );
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
  } = &state["slides"].as_ref().expect("slides")["slides_pptx"]
  else {
    panic!("expected native pptx file");
  };
  let mut zip = zip::ZipArchive::new(std::fs::File::open(pptx_path).expect("pptx")).expect("zip");
  let mut slides_xml = String::new();
  let mut notes_xml = String::new();
  for i in 0..zip.len() {
    let mut entry = zip.by_index(i).expect("entry");
    let name = entry.name().expect("utf-8 zip entry").into_owned();
    if name.starts_with("ppt/slides/slide") && name.ends_with(".xml") {
      entry.read_to_string(&mut slides_xml).expect("utf-8");
    } else if name.starts_with("ppt/notesSlides/") && name.ends_with(".xml") {
      entry.read_to_string(&mut notes_xml).expect("utf-8");
    }
  }
  assert!(
    slides_xml.contains("正交能力契约") && slides_xml.contains("Summary"),
    "native editable code and callout shapes present in slide XML"
  );
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
  assert_eq!(val["content"]["presenter_mode"], "pip");
  assert!(
    deliverable.files.len() >= 5,
    "expected mp4 + srt + vtt + html + pptx in deliverable: {:?}",
    deliverable.files
  );
}
