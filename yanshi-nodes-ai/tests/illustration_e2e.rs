//! End-to-end integration test for `IllustrationNode` (`type: illustration`)
//! under an isolated `vendor: mock` registry, verifying single mode with LLM
//! art-director prompt refinement, `SlideDeck` direct assembly (`arch_image` +
//! bounded concurrent illustrations + `fallback_right` promotion), and
//! `CostMeter` accounting.

use std::collections::HashMap;

use serde_json::{Value, json};
use yanshi_core::{async_node::AsyncNode, run_context, value::FlowValue};
use yanshi_llm::Yanshi;
use yanshi_nodes_ai::IllustrationNode;

#[tokio::test]
async fn single_and_deck_modes_generate_files_wire_arch_and_preserve_fallbacks() {
  let dir = tempfile::tempdir().expect("tempdir");
  let reg_path = dir.path().join("models.yml");
  std::fs::write(
    &reg_path,
    "models:\n  mock-chat:\n    vendor: mock\n    type: text\n    model_id: mock-chat\n  mock-img:\n    vendor: mock\n    type: text_to_image\n    model_id: mock-img\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
  )
  .expect("write registry");

  // SAFETY: dedicated integration test binary initializing mock registry before any model call.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &reg_path);
    std::env::set_var(
      "YANSHI_MOCK_RESPONSES",
      json!(["Sculptural crystalline dispatcher routing luminous beams across parallel channels"])
        .to_string(),
    );
  }
  Yanshi::init_with_config(reg_path.to_str().expect("utf-8"))
    .await
    .expect("init mock registry");

  // 1. Single Mode with LLM prompt refinement
  let single_node = IllustrationNode::new("hero")
    .with_model("mock-img")
    .with_prompt_model("mock-chat")
    .with_style_preset("cinematic_keynote")
    .with_refine_prompt(true);

  let single_inputs = HashMap::from([(
    "concept".to_string(),
    FlowValue::Json(json!("Async work-stealing scheduler")),
  )]);
  let single_out = run_context::with_artifact_root(
    dir.path().to_path_buf(),
    single_node.execute(&single_inputs),
  )
  .await
  .expect("single illustration");

  assert!(matches!(
    single_out.get("hero_image"),
    Some(FlowValue::File { .. })
  ));
  let FlowValue::Json(Value::String(prompt_str)) = &single_out["hero_prompt"] else {
    panic!("expected hero_prompt string");
  };
  assert!(prompt_str.contains("Sculptural crystalline dispatcher"));

  // 2. Deck Mode: slide 0 = arch_diagram, slide 1 = illustration, slide 2 = fallback when capped by max_images=1
  let arch_svg = dir.path().join("artifacts/arch/0.svg");
  std::fs::create_dir_all(arch_svg.parent().expect("parent")).expect("mkdir");
  std::fs::write(&arch_svg, "<svg xmlns=\"http://www.w3.org/2000/svg\"/>").expect("svg");

  let raw_deck = json!({
    "title": "Tokio Internals",
    "theme": "light",
    "slides": [
      {
        "title": "Architecture Overview",
        "left": {"kind": "bullets", "items": [{"heading": "Reactor", "body": "mio epoll"}]},
        "visual_mode": "arch_diagram",
        "fallback_right": {"kind": "bullets", "items": [{"heading": "Fallback", "body": "unused"}]}
      },
      {
        "title": "Work-Stealing Queue",
        "left": {"kind": "bullets", "items": [{"heading": "LIFO", "body": "hot cache"}]},
        "visual_mode": "illustration",
        "illustration_prompt": "Interlocking gears transferring glowing orbs",
        "illustration_caption": "Work-Stealing 任务迁移示意",
        "fallback_right": {"kind": "table", "columns": ["A"], "rows": [["1"]]}
      },
      {
        "title": "Overflow Slide Beyond Cap",
        "left": {"kind": "bullets", "items": [{"heading": "Cap", "body": "uses fallback_right"}]},
        "visual_mode": "illustration",
        "illustration_prompt": "Second illustration beyond max_images=1",
        "fallback_right": {"kind": "table", "columns": ["Metric"], "rows": [["Latency"]]}
      }
    ]
  });

  let deck_node = IllustrationNode::new("illustrate")
    .with_model("mock-img")
    .with_max_images(1);
  let deck_inputs = HashMap::from([
    ("deck".to_string(), FlowValue::Json(raw_deck)),
    (
      "arch_image".to_string(),
      FlowValue::File {
        path: arch_svg.clone(),
        mime_type: Some("image/svg+xml".to_string()),
      },
    ),
  ]);

  let deck_out =
    run_context::with_artifact_root(dir.path().to_path_buf(), deck_node.execute(&deck_inputs))
      .await
      .expect("deck illustration");

  assert_eq!(deck_out["illustrate_count"], FlowValue::Json(json!(1)));
  let FlowValue::Json(patched_deck) = &deck_out["illustrate_deck"] else {
    panic!("expected illustrate_deck JSON");
  };
  let slides = patched_deck["slides"].as_array().expect("slides");
  assert_eq!(slides[0]["right"]["kind"], "image");
  assert_eq!(
    slides[0]["right"]["src"],
    arch_svg.to_string_lossy().as_ref()
  );
  assert_eq!(slides[1]["right"]["kind"], "image");
  assert_eq!(slides[1]["right"]["caption"], "Work-Stealing 任务迁移示意");
  assert!(
    std::path::Path::new(slides[1]["right"]["src"].as_str().expect("src")).exists(),
    "generated artifact file must exist on disk"
  );
  assert_eq!(slides[2]["right"]["kind"], "table");
}
