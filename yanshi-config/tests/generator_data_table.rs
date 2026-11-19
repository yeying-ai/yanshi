//! AA2: `examples/generators/data_table.yml` runs end to end under `vendor: mock`
//! for both standalone text and notebook `chunks`, producing valid `.xlsx`, `.csv`,
//! and `.html` artifacts plus a contract-compliant `DeliverableKind::DataTable`.

use std::collections::HashMap;
use std::io::Read as _;

use serde_json::json;
use yanshi_config::content::{Deliverable, DeliverableKind, FileRole};
use yanshi_config::executor::{
  apply_declared_inputs, build_flow_from_definition, parse_workflow_definition,
};
use yanshi_core::{FlowExecutionConfig, FlowExt, FlowStateExt, value::FlowValue};

const MOCK_REGISTRY: &str = "models:\n  mock-model:\n    vendor: mock\n    type: text\n    model_id: mock-model\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n";

#[tokio::test]
async fn data_table_generator_renders_xlsx_csv_html_and_notebook_citations() {
  let dir = std::env::temp_dir().join(format!("yanshi-gen-datatable-{}", std::process::id()));
  std::fs::create_dir_all(&dir).expect("dir");
  let registry = dir.join("models.yml");
  std::fs::write(&registry, MOCK_REGISTRY).expect("registry");

  let replies = json!([
    json!({
      "title": "Tidal Regimes Comparison",
      "summary": "Comparison of spring and neap tidal amplitudes and lunar alignment.",
      "columns": ["Tide Regime", "Moon Phase", "Typical Range (m)"],
      "rows": [
        {
          "cells": ["Spring | Perigean", "New or Full", "6.4"],
          "chunk_id": "",
          "quote": "Spring tides peak at 6.4 m during new or full moon."
        },
        {
          "cells": ["Neap", "First or Third Quarter", "2.1"],
          "chunk_id": "",
          "quote": "Neap tides drop to 2.1 m at quarter moon."
        }
      ]
    })
    .to_string(),
    json!({
      "title": "Database Lock Modes",
      "summary": "PostgreSQL advisory locking latency and scope.",
      "columns": ["Primitive", "Scope", "Latency"],
      "rows": [
        {
          "cells": ["pg_advisory_xact_lock", "Transaction", "0.12 ms"],
          "chunk_id": "src-pg:0",
          "quote": "pg_advisory_xact_lock releases automatically at commit in 0.12 ms."
        },
        {
          "cells": ["pg_advisory_lock", "Session", "0.15 ms"],
          "chunk_id": "src-net:1",
          "quote": "Session-level advisory locks persist across transactions."
        }
      ]
    })
    .to_string()
  ]);

  // SAFETY: single test in its own integration test binary before any model call.
  unsafe {
    std::env::set_var("YANSHI_MODELS_CONFIG", &registry);
    std::env::set_var("MOCK_API_KEY", "x");
    std::env::set_var("YANSHI_MOCK_RESPONSES", replies.to_string());
  }
  yanshi_llm::Yanshi::init_with_config(registry.to_str().expect("utf-8"))
    .await
    .expect("mock registry");

  let template_path =
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../examples/generators/data_table.yml");
  let yaml = std::fs::read_to_string(&template_path).expect("read data_table.yml");
  let definition = parse_workflow_definition(&yaml).expect("template parses");

  // 1. Standalone pasted-text mode
  let mut inputs: HashMap<String, FlowValue> = HashMap::from([
    (
      "text".to_string(),
      FlowValue::Json(json!(
        "Spring tides peak at 6.4 m during new or full moon. Neap tides drop to 2.1 m at quarter moon."
      )),
    ),
    ("max_rows".to_string(), FlowValue::Json(json!(10))),
  ]);
  apply_declared_inputs(&definition, &mut inputs).expect("defaults");
  let run_dir = tempfile::TempDir::new().expect("tempdir");
  let state = build_flow_from_definition(&definition, Some("mock-model"))
    .expect("build flow")
    .execute_from_inputs_with_config(
      inputs,
      FlowExecutionConfig::serial().with_run_base_dir(run_dir.path()),
    )
    .await
    .expect("execute");
  state.ensure_succeeded().expect("all nodes succeeded");

  // Verify CSV round-trip (including pipe inside cell preserved cleanly)
  let FlowValue::File { path: csv_path, .. } =
    &state["table_csv"].as_ref().expect("csv")["table_csv_output"]
  else {
    panic!("expected csv File");
  };
  let mut csv_reader = csv::Reader::from_path(csv_path).expect("open csv");
  assert_eq!(
    csv_reader
      .headers()
      .expect("headers")
      .iter()
      .collect::<Vec<_>>(),
    vec!["Tide Regime", "Moon Phase", "Typical Range (m)"]
  );
  let records: Vec<csv::StringRecord> = csv_reader
    .records()
    .collect::<Result<_, _>>()
    .expect("records");
  assert_eq!(records.len(), 2);
  assert_eq!(&records[0][0], "Spring | Perigean");
  assert_eq!(&records[1][2], "2.1");

  // Verify XLSX ZIP archive contains worksheet XML
  let FlowValue::File {
    path: xlsx_path, ..
  } = &state["table_xlsx"].as_ref().expect("xlsx")["table_xlsx_output"]
  else {
    panic!("expected xlsx File");
  };
  let xlsx_file = std::fs::File::open(xlsx_path).expect("open xlsx");
  let mut archive = zip::ZipArchive::new(xlsx_file).expect("valid zip xlsx");
  let mut sheet_xml = String::new();
  archive
    .by_name("xl/worksheets/sheet1.xml")
    .expect("sheet1.xml")
    .read_to_string(&mut sheet_xml)
    .expect("read xml");
  assert!(sheet_xml.contains("<worksheet"), "{sheet_xml}");

  // Verify HTML preview renders escaped Markdown table without broken pipe column splits
  let FlowValue::File {
    path: html_path, ..
  } = &state["table_html"].as_ref().expect("html")["table_html_output"]
  else {
    panic!("expected html File");
  };
  let html = std::fs::read_to_string(html_path).expect("read html");
  assert!(
    html.contains("<table>") && html.contains("Tidal Regimes Comparison"),
    "{html}"
  );

  // Verify Deliverable contract
  let FlowValue::Json(deliv_val) =
    &state["deliverable"].as_ref().expect("deliverable")["deliverable"]
  else {
    panic!("deliverable JSON");
  };
  let deliverable = Deliverable::from_value(deliv_val.clone()).expect("valid Deliverable");
  assert_eq!(deliverable.kind, DeliverableKind::DataTable);
  assert_eq!(deliverable.title, "Tidal Regimes Comparison");
  assert_eq!(deliverable.citations.len(), 2);
  assert!(deliverable.citations.iter().all(|c| c.source_id == "text"));
  assert_eq!(deliverable.files.len(), 3);
  assert_eq!(deliverable.files[0].role, FileRole::Primary);
  assert_eq!(deliverable.files[2].role, FileRole::Preview);

  // 2. Notebook chunks mode: citations resolve chunk_id -> source_id
  let mut chunk_inputs: HashMap<String, FlowValue> = HashMap::from([(
    "chunks".to_string(),
    FlowValue::Json(json!([
      {"chunk_id": "src-pg:0", "source_id": "src-pg", "content": "pg_advisory_xact_lock releases automatically at commit in 0.12 ms."},
      {"chunk_id": "src-net:1", "source_id": "src-net", "content": "Session-level advisory locks persist across transactions."}
    ])),
  )]);
  apply_declared_inputs(&definition, &mut chunk_inputs).expect("defaults");
  let chunk_run_dir = tempfile::TempDir::new().expect("tempdir");
  let chunk_state = build_flow_from_definition(&definition, Some("mock-model"))
    .expect("build flow")
    .execute_from_inputs_with_config(
      chunk_inputs,
      FlowExecutionConfig::serial().with_run_base_dir(chunk_run_dir.path()),
    )
    .await
    .expect("execute");
  chunk_state.ensure_succeeded().expect("chunk run succeeded");

  let FlowValue::Json(chunk_deliv_val) =
    &chunk_state["deliverable"].as_ref().expect("deliverable")["deliverable"]
  else {
    panic!("deliverable JSON");
  };
  let chunk_deliverable =
    Deliverable::from_value(chunk_deliv_val.clone()).expect("valid Deliverable");
  assert_eq!(chunk_deliverable.kind, DeliverableKind::DataTable);
  assert_eq!(chunk_deliverable.citations.len(), 2);
  assert_eq!(chunk_deliverable.citations[0].source_id, "src-pg");
  assert_eq!(chunk_deliverable.citations[0].chunk_id, "src-pg:0");
  assert_eq!(chunk_deliverable.citations[1].source_id, "src-net");
  assert_eq!(chunk_deliverable.citations[1].chunk_id, "src-net:1");
}
