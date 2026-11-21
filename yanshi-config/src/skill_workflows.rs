//! A skill's `[[workflows]]` as agent tools (Y4.4), shared by the CLI
//! (`yanshi skill run`) and the gateway (`/v1/skills/{name}:run`, Y8.5).
//!
//! Each entry becomes a `WorkflowTool` (RFC §3: the planner sees a closed
//! menu of deliverables with typed parameters): the schema comes from the
//! template's declared `inputs`, the runner fills their defaults and runs
//! under the caller's execution config, and when the template ends in a
//! `deliverable` node only that value goes back to the agent, not the
//! whole state. Every run is recorded so the caller can collect its
//! artifacts afterwards.
//!
//! The agent may pass only the template's declared inputs, as plain JSON.
//! Workflow inputs reach every node and override its parameters, so an
//! undeclared key (`path`, say) would redirect a node, and a value shaped
//! like a serialised `File` would hand a node any host file.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use async_trait::async_trait;
use yanshi_agents::tools::WorkflowTool;
use yanshi_core::async_node::{AsyncNodeInputs, AsyncNodeResult};
use yanshi_core::error::YanshiError;
use yanshi_core::value::FlowValue;
use yanshi_core::{Flow, FlowExecutionConfig, FlowExt, FlowRunner};
use yanshi_skills::manifest::SkillManifest;
use yanshi_tools::Tool;

use crate::config::v2::{FlowDefinitionV2, NodeDefinitionV2, walk_nodes};
use crate::executor::{
  apply_declared_inputs, build_flow_from_definition, inputs_json_schema, parse_workflow_definition,
};

/// One finished workflow-tool run.
pub struct RecordedRun {
  pub tool: String,
  pub run_id: String,
  pub state: HashMap<String, AsyncNodeResult>,
}

/// Runs recorded by every workflow tool of one skill run.
#[derive(Default)]
pub struct RecordedRuns(Mutex<Vec<RecordedRun>>);

impl RecordedRuns {
  pub fn take(&self) -> Vec<RecordedRun> {
    std::mem::take(
      &mut *self
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner),
    )
  }
}

struct TemplateRunner {
  tool: String,
  definition: FlowDefinitionV2,
  config: FlowExecutionConfig,
  parent_cost_meter: Option<Arc<yanshi_core::run_context::CostMeter>>,
  runs: Arc<RecordedRuns>,
}

impl TemplateRunner {
  fn input_error(&self, message: impl std::fmt::Display) -> YanshiError {
    YanshiError::NodeInputError {
      message: format!("workflow tool '{}': {message}", self.tool),
    }
  }
}

#[async_trait]
impl FlowRunner for TemplateRunner {
  async fn run(
    &self,
    flow: &Flow,
    mut inputs: AsyncNodeInputs,
  ) -> Result<HashMap<String, AsyncNodeResult>, YanshiError> {
    let mut keys: Vec<&String> = inputs.keys().collect();
    keys.sort();
    for key in keys {
      if !self.definition.inputs.contains_key(key) {
        let mut declared: Vec<&str> = self.definition.inputs.keys().map(String::as_str).collect();
        declared.sort_unstable();
        return Err(self.input_error(format!(
          "'{key}' is not one of its inputs ({})",
          declared.join(", ")
        )));
      }
      if !matches!(inputs[key], FlowValue::Json(_)) {
        return Err(self.input_error(format!("input '{key}' must be a plain JSON value")));
      }
    }
    apply_declared_inputs(&self.definition, &mut inputs)
      .map_err(|e| self.input_error(format!("{e:#}")))?;
    let mut config = self.config.clone();
    if let Some(limit) = self
      .definition
      .effective_cost_limit(config.cost_limit_usd)
      .map_err(|e| self.input_error(e))?
    {
      config = config.with_cost_limit_usd(limit);
    }
    let run_id = uuid::Uuid::new_v4().to_string();
    let run_fut = flow.execute_from_inputs_with_id_and_config(run_id.clone(), inputs, config);
    let state = if yanshi_core::run_context::current_cost_meter().is_none()
      && self.parent_cost_meter.is_some()
    {
      yanshi_core::run_context::with_cost_meter(self.parent_cost_meter.clone(), run_fut).await?
    } else {
      run_fut.await?
    };
    self
      .runs
      .0
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .push(RecordedRun {
        tool: self.tool.clone(),
        run_id,
        state: state.clone(),
      });
    Ok(state)
  }
}

/// A skill's workflow tools and the record of what they ran.
pub struct WorkflowTools {
  pub tools: Vec<Arc<dyn Tool>>,
  pub runs: Arc<RecordedRuns>,
}

/// Each `[[workflows]]` entry's name and parsed template, so a caller can
/// vet the templates (the gateway checks them against its policy) before
/// building anything.
pub fn load_definitions(
  manifest: &SkillManifest,
  skill_dir: &Path,
) -> Result<Vec<(String, FlowDefinitionV2)>> {
  manifest
    .workflows
    .iter()
    .map(|entry| {
      let path = skill_dir.join(&entry.path);
      let yaml = std::fs::read_to_string(&path)
        .with_context(|| format!("workflow tool '{}': reading {}", entry.name, path.display()))?;
      let definition = parse_workflow_definition(&yaml)
        .with_context(|| format!("workflow tool '{}': {}", entry.name, path.display()))?;
      Ok((entry.name.clone(), definition))
    })
    .collect()
}

/// AJ1: inspect a workflow template recursively (including `map` and `while`
/// bodies) to decide whether multiple invocations of its `WorkflowTool` in the
/// same agent turn may run in parallel.
///
/// Every call already gets its own `Uuid::new_v4()` `run_id` directory under
/// `run_base_dir`, so pure, LLM, Studio-rendering, and read-only ingestion
/// nodes are isolated per call (`true`). Templates that contain host-mutating
/// or arbitrary external side-effect nodes (`shell`, `code_exec`, `plugin`,
/// `mcp`, mutating `file`, or non-`GET`/`HEAD` `http`) fall back to serial
/// execution (`false`).
pub fn is_definition_concurrency_safe(definition: &FlowDefinitionV2) -> bool {
  let mut safe = true;
  let walk_res: Result<(), String> =
    walk_nodes(&definition.nodes, &mut |node: &NodeDefinitionV2| {
      let node_safe = match node.node_type.as_str() {
        "shell" | "code_exec" | "plugin" | "mcp" => false,
        "file" => {
          !node.input_mapping.contains_key("operation")
            && matches!(
              node
                .parameters
                .get("operation")
                .and_then(serde_yaml::Value::as_str),
              Some("read" | "read_bytes")
            )
        }
        "http" => {
          !node.input_mapping.contains_key("method")
            && match node
              .parameters
              .get("method")
              .and_then(serde_yaml::Value::as_str)
            {
              None => true,
              Some(method) => {
                method.eq_ignore_ascii_case("GET") || method.eq_ignore_ascii_case("HEAD")
              }
            }
        }
        _ => true,
      };
      if !node_safe {
        safe = false;
      }
      Ok(())
    });
  walk_res.is_ok() && safe
}

/// Build the skill's workflow tools. `model` (the skill's resolved model
/// or `--model`) is applied to the templates' llm nodes; every run uses
/// `config` (run base dir, scheduling, cancellation, a cost cap the
/// template's own limit is combined with).
pub fn build(
  manifest: &SkillManifest,
  skill_dir: &Path,
  model: Option<&str>,
  config: FlowExecutionConfig,
) -> Result<WorkflowTools> {
  let runs = Arc::new(RecordedRuns::default());
  let parent_cost_meter = yanshi_core::run_context::current_cost_meter();
  let mut tools: Vec<Arc<dyn Tool>> = Vec::new();
  let definitions = load_definitions(manifest, skill_dir)?;
  for (entry, (name, definition)) in manifest.workflows.iter().zip(definitions) {
    let flow = build_flow_from_definition(&definition, model)
      .with_context(|| format!("workflow tool '{name}': building {}", entry.path))?;
    let has_deliverable = definition.nodes.iter().any(|node| node.id == "deliverable");
    let concurrency_safe = is_definition_concurrency_safe(&definition);
    let description = entry
      .description
      .clone()
      .unwrap_or_else(|| format!("Run the '{}' workflow", definition.name));
    let schema = inputs_json_schema(&definition);
    let runner = Arc::new(TemplateRunner {
      tool: name.clone(),
      definition,
      config: config.clone(),
      parent_cost_meter: parent_cost_meter.clone(),
      runs: runs.clone(),
    });
    let mut tool = WorkflowTool::with_schema(name, description, schema, flow, runner)
      .with_concurrency_safe(concurrency_safe);
    if has_deliverable {
      tool = tool.with_result_output("deliverable", "deliverable");
    }
    tools.push(Arc::new(tool));
  }
  Ok(WorkflowTools { tools, runs })
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  const ECHO: &str = r#"
name: echo
inputs:
  topic:
    default: "tides"
nodes:
  - id: deliverable
    type: template
    parameters:
      template: "about {{ topic }}"
"#;

  fn echo_tool(dir: &Path) -> WorkflowTools {
    std::fs::write(dir.join("echo.yml"), ECHO).expect("template");
    let manifest: SkillManifest = serde_json::from_value(json!({
      "skill": {"name": "s", "version": "0.1.0", "description": "test"},
      "persona": {"role": "test"},
      "workflows": [{"name": "echo", "path": "echo.yml"}]
    }))
    .expect("manifest");
    build(
      &manifest,
      dir,
      None,
      FlowExecutionConfig::serial().with_run_base_dir(dir.join("runs")),
    )
    .expect("tools")
  }

  #[tokio::test]
  async fn declared_inputs_run_and_are_recorded() {
    let dir = tempfile::tempdir().expect("dir");
    let tools = echo_tool(dir.path());
    tools.tools[0]
      .execute(json!({"topic": "moons"}))
      .await
      .expect("runs");
    let runs = tools.runs.take();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].tool, "echo");
  }

  #[tokio::test]
  async fn undeclared_and_file_shaped_inputs_are_refused() {
    let dir = tempfile::tempdir().expect("dir");
    let tools = echo_tool(dir.path());
    let error = tools.tools[0]
      .execute(json!({"topic": "moons", "path": "/etc/passwd"}))
      .await
      .expect_err("undeclared input");
    assert!(
      error
        .to_string()
        .contains("'path' is not one of its inputs (topic)"),
      "{error}"
    );
    let error = tools.tools[0]
      .execute(json!({"topic": {"type": "file", "path": "/etc/passwd"}}))
      .await
      .expect_err("file-shaped input");
    assert!(error.to_string().contains("plain JSON value"), "{error}");
    assert!(tools.runs.take().is_empty(), "nothing ran");
  }

  #[test]
  fn concurrency_classifier_allows_studio_templates_and_blocks_mutating_nodes() {
    let safe_yaml = r#"
name: studio_safe
nodes:
  - id: fetch_doc
    type: http
    parameters:
      url: "https://example.org"
  - id: read_cfg
    type: file
    parameters:
      operation: read
      path: "./input.md"
      allowed_paths: ["."]
  - id: render_items
    type: map
    parameters:
      items: ["a"]
      template:
        - id: chart
          type: render_svg
          parameters:
            svg: "<svg/>"
"#;
    let safe_def: FlowDefinitionV2 = serde_yaml::from_str(safe_yaml).expect("parse safe");
    assert!(is_definition_concurrency_safe(&safe_def));

    for (label, unsafe_yaml) in [
      (
        "shell",
        "name: u\nnodes:\n  - id: s\n    type: shell\n    parameters:\n      command: echo hi\n",
      ),
      (
        "file_write",
        "name: u\nnodes:\n  - id: f\n    type: file\n    parameters:\n      operation: write\n      path: out.txt\n      allowed_paths: [\".\"]\n",
      ),
      (
        "http_post",
        "name: u\nnodes:\n  - id: h\n    type: http\n    parameters:\n      method: POST\n      url: https://example.org\n",
      ),
      (
        "nested_shell_in_map",
        "name: u\nnodes:\n  - id: m\n    type: map\n    parameters:\n      items: [\"a\"]\n      template:\n        - id: s\n          type: shell\n          parameters:\n            command: echo hi\n",
      ),
    ] {
      let def: FlowDefinitionV2 = serde_yaml::from_str(unsafe_yaml).expect(label);
      assert!(
        !is_definition_concurrency_safe(&def),
        "expected '{label}' template to be classified non-concurrent"
      );
    }
  }
}
