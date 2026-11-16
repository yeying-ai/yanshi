//! End-to-end test for `examples/generators/illustrated_slides.yml`:
//! Outline planning → D2 architecture figure (`figure`) + content expansion
//! → art-directed `illustration` node (LLM prompt refinement + concurrent
//! image generation + `arch_image` & `fallback_right` assembly) → `render_slides`
//! (native editable OOXML `.pptx` + `16:9` widescreen `.pdf` + interactive `.html`)
//! → valid `Deliverable`.

use std::collections::HashMap;
use std::io::Read as _;

use serde_json::json;
use yanshi_config::content::{Deliverable, DeliverableKind};
use yanshi_config::executor::{
  apply_declared_inputs, build_flow_from_definition, parse_workflow_definition,
};
use yanshi_core::{FlowExecutionConfig, FlowExt, value::FlowValue};

#[tokio::test]
async fn illustrated_slides_generator_wires_illustrations_d2_svg_native_pptx_and_pdf() {
  if yanshi_nodes::common::sandboxed::find_tool("d2").is_none() {
    eprintln!("skipping: d2 is not installed");
    return;
  }
  let has_typst = yanshi_nodes::common::sandboxed::find_tool("typst").is_some();

  let dir = std::env::temp_dir().join(format!("yanshi-illus-slides-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("dir");
  let registry = dir.join("models.yml");
  std::fs::write(
    &registry,
    "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\n  mock-image:\n    vendor: mock\n    type: text_to_image\n    model_id: mock-image\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
  )
  .expect("registry");

  let outline = json!({
    "title": "Tokio Work-Stealing 调度器",
    "subtitle": "无锁本地队列与跨核任务迁移",
    "arch_diagram_spec": "Workers -> Global Queue -> IO Driver",
    "chapters": [
      {
        "section": "01 架构总览",
        "title": "多线程分层调度架构",
        "key_points": ["Local Queue 256 槽", "Global Injection Queue"],
        "narration_angle": "架构全景拆解",
        "visual_mode": "arch_diagram"
      },
      {
        "section": "02 工作窃取",
        "title": "空闲核半数任务迁移",
        "key_points": ["LIFO 热槽位", "无锁 CAS 窃取一半"],
        "narration_angle": "动态负载平衡",
        "visual_mode": "illustration",
        "illustration_prompt": "Geometric crystalline queues balancing luminous spheres"
      }
    ]
  });

  let d2_source =
    "direction: right\nworker_a: Worker A\nworker_b: Worker B\nworker_a -> worker_b: steal half";

  let expanded = json!({
    "title": "Tokio Work-Stealing 调度器",
    "subtitle": "无锁本地队列与跨核任务迁移",
    "author": "Yanshi Studio",
    "slides": [
      {
        "section": "01 架构总览",
        "title": "多线程分层调度架构",
        "subtitle": "无锁本地环形缓冲与全局队列协同",
        "left": {
          "kind": "bullets",
          "items": [
            {"badge": "Local", "heading": "每核独立环形队列", "body": "256 容量无锁单生产者访问", "tone": "cyan"}
          ]
        },
        "fallback_right": {
          "kind": "table",
          "columns": ["组件", "职责"],
          "rows": [["Local", "零锁本地入队"]]
        },
        "visual_mode": "arch_diagram",
        "takeaway": "绝大多数任务流转在核内无锁完成。",
        "notes": "首先看整体调度拓扑与 D2 矢量架构图。"
      },
      {
        "section": "02 工作窃取",
        "title": "空闲核半数任务迁移",
        "subtitle": "LIFO 热缓存槽与批量窃取均摊开销",
        "left": {
          "kind": "code",
          "language": "rust",
          "code": "pub fn steal_half(&self, dst: &Local) {\n  self.ring.transfer_half(dst);\n}",
          "callouts": [{"line": 2, "text": "批量转移均摊 CAS", "tone": "rust"}]
        },
        "fallback_right": {
          "kind": "bullets",
          "items": [{"heading": "兜底卡片", "body": "当禁用插图时显示"}]
        },
        "visual_mode": "illustration",
        "illustration_prompt": "Geometric crystalline queues balancing luminous spheres",
        "illustration_caption": "跨核无锁半数窃取视觉隐喻",
        "takeaway": "批量窃取兼顾负载均衡与缓存行局部性。",
        "notes": "第二页结合代码与概念插图直观理解窃取语义。"
      }
    ]
  });

  let refined_art_prompt = "Isometric crystalline processor cores linked by glowing copper conduits balancing luminous spheres in equilibrium";

  // Serial execution order: plan_outline -> arch_figure -> expand_slides -> illustrate (refine_prompt)
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &registry);
    std::env::set_var(
      "YANSHI_MOCK_RESPONSES",
      json!([
        outline.to_string(),
        d2_source,
        expanded.to_string(),
        refined_art_prompt,
      ])
      .to_string(),
    );
  }
  yanshi_llm::Yanshi::init_with_config(registry.to_str().expect("utf-8"))
    .await
    .expect("mock registry");

  let template = std::fs::read_to_string(
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
      .join("../examples/generators/illustrated_slides.yml"),
  )
  .expect("template");
  let definition = parse_workflow_definition(&template).expect("validates");
  let mut inputs: HashMap<String, FlowValue> = [
    ("enable_web_search", json!(false)),
    ("enable_illustrations", json!(true)),
    ("refine_prompt", json!(true)),
    ("style_preset", json!("isometric_3d")),
    ("image_model", json!("mock-image")),
    ("export_pptx", json!(true)),
    ("export_pdf", json!(has_typst)),
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

  let rendered = state["slides_render"].as_ref().expect("slides_render");
  let FlowValue::File {
    path: pptx_path, ..
  } = &rendered["slides_render_pptx"]
  else {
    panic!("expected native pptx");
  };

  let mut zip =
    zip::ZipArchive::new(std::fs::File::open(pptx_path).expect("open pptx")).expect("zip");
  let names: Vec<String> = zip
    .file_names()
    .map(|n| n.expect("utf-8").to_string())
    .collect();
  let slide_count = names
    .iter()
    .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
    .count();
  assert_eq!(
    slide_count, 3,
    "cover + 2 content slides (no duplicate companion pages)"
  );
  assert!(
    names
      .iter()
      .any(|n| n.starts_with("ppt/media/") && n.ends_with(".svg")),
    "D2 architecture SVG embedded as vector"
  );
  assert!(
    names
      .iter()
      .filter(|n| n.starts_with("ppt/media/") && n.ends_with(".png"))
      .count()
      >= 2,
    "both D2 raster fallback and generated AI illustration PNG embedded"
  );

  let mut slide3_xml = String::new();
  zip
    .by_name("ppt/slides/slide3.xml")
    .expect("slide3")
    .read_to_string(&mut slide3_xml)
    .expect("utf-8");
  assert!(
    slide3_xml.contains("批量转移均摊 CAS") && slide3_xml.contains("跨核无锁半数窃取视觉隐喻"),
    "native editable code callout and illustration caption present in slide3.xml"
  );

  let FlowValue::Json(val) = &state["deliverable"].as_ref().expect("deliverable")["deliverable"]
  else {
    panic!("deliverable json");
  };
  let deliverable = Deliverable::from_value(val.clone()).expect("valid Deliverable");
  assert_eq!(deliverable.kind, DeliverableKind::Slides);
  assert!(deliverable.files.len() >= 4);
}
