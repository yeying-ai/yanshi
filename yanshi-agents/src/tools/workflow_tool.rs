//! Wrap a DAG [`Flow`] as an agent-callable tool.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use yanshi_graph::{Flow, FlowRunner, FlowValue, async_node::AsyncNodeInputs};
use yanshi_tool::{Tool, ToolError, ToolMetadata, ToolOutput, ToolOutputPart};

/// Tool adapter that lets an agent call a workflow as a normal tool.
///
/// The wrapped `Flow` is executed via an injected [`FlowRunner`] (the surface
/// passes `yanshi_core::CoreFlowRunner`), so this crate depends on the IR +
/// the runner contract, not the executor runtime.
pub struct WorkflowTool {
  name: String,
  description: String,
  parameters_schema: Value,
  flow: Arc<Flow>,
  runner: Arc<dyn FlowRunner>,
  timeout: Option<Duration>,
  /// Y4.4: `(node_id, output_key)` to return instead of the whole final
  /// state (see [`Self::with_result_output`]).
  result_output: Option<(String, String)>,
  /// AJ1: whether multiple invocations in a single agent turn can execute in
  /// parallel (defaults to `true` because each run allocates an isolated
  /// `run_id` directory; callers wrapping host-mutating templates can disable
  /// via [`Self::with_concurrency_safe`]).
  concurrency_safe: bool,
}

impl WorkflowTool {
  pub fn new(
    name: impl Into<String>,
    description: impl Into<String>,
    flow: Flow,
    runner: Arc<dyn FlowRunner>,
  ) -> Self {
    Self::with_schema(
      name,
      description,
      default_workflow_parameters_schema(),
      flow,
      runner,
    )
  }

  pub fn with_schema(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters_schema: Value,
    flow: Flow,
    runner: Arc<dyn FlowRunner>,
  ) -> Self {
    Self {
      name: name.into(),
      description: description.into(),
      parameters_schema,
      flow: Arc::new(flow),
      runner,
      timeout: None,
      result_output: None,
      concurrency_safe: true,
    }
  }

  pub fn from_shared(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters_schema: Value,
    flow: Arc<Flow>,
    runner: Arc<dyn FlowRunner>,
  ) -> Self {
    Self {
      name: name.into(),
      description: description.into(),
      parameters_schema,
      flow,
      runner,
      timeout: None,
      result_output: None,
      concurrency_safe: true,
    }
  }

  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = Some(timeout);
    self
  }

  pub fn with_timeout_ms(self, timeout_ms: u64) -> Self {
    self.with_timeout(Duration::from_millis(timeout_ms))
  }

  /// Configure whether multiple calls to this workflow tool in one agent turn
  /// may run in parallel.
  pub fn with_concurrency_safe(mut self, concurrency_safe: bool) -> Self {
    self.concurrency_safe = concurrency_safe;
    self
  }

  pub fn is_concurrency_safe(&self) -> bool {
    self.concurrency_safe
  }

  /// Y4.4: return only `node_id`'s `output_key` (e.g. a generator's
  /// `deliverable`) instead of the whole final state, which for a content
  /// generator includes the full source text and would flood the calling
  /// agent's context. A failed run still reports its first error.
  pub fn with_result_output(
    mut self,
    node_id: impl Into<String>,
    output_key: impl Into<String>,
  ) -> Self {
    self.result_output = Some((node_id.into(), output_key.into()));
    self
  }

  pub fn timeout(&self) -> Option<Duration> {
    self.timeout
  }

  pub fn flow_handle(&self) -> Arc<Flow> {
    self.flow.clone()
  }
}

#[async_trait]
impl Tool for WorkflowTool {
  fn name(&self) -> &str {
    &self.name
  }

  fn description(&self) -> &str {
    &self.description
  }

  fn parameters_schema(&self) -> Value {
    self.parameters_schema.clone()
  }

  fn metadata(&self) -> ToolMetadata {
    ToolMetadata::workflow()
  }

  fn concurrency_safe(&self, _params: &Value) -> bool {
    self.concurrency_safe
  }

  async fn execute(&self, params: Value) -> Result<ToolOutput, ToolError> {
    let inputs = params_to_inputs(params)?;
    let execution = self.runner.run(&self.flow, inputs);
    let results = match self.timeout {
      Some(timeout) => tokio::time::timeout(timeout, execution)
        .await
        .map_err(|_| ToolError::ExecutionFailed {
          message: format!(
            "Workflow tool '{}' timed out after {}ms",
            self.name,
            timeout.as_millis()
          ),
        })?,
      None => execution.await,
    }
    .map_err(|e| ToolError::ExecutionFailed {
      message: format!("Workflow tool '{}' failed: {}", self.name, e),
    })?;
    let has_node_error = results
      .values()
      .any(|r| matches!(r, Err(e) if !matches!(e, yanshi_graph::error::YanshiError::NodeSkipped)));
    let value = match &self.result_output {
      Some((node_id, key)) => match results.get(node_id) {
        Some(Ok(outputs)) => match outputs.get(key) {
          Some(FlowValue::Json(value)) => value.clone(),
          Some(other) => serde_json::to_value(other).map_err(ToolError::SerdeError)?,
          None => serde_json::json!({ "error": format!("node '{node_id}' has no output '{key}'") }),
        },
        _ => {
          let first_error = results.iter().find_map(|(id, result)| match result {
            Err(e) if !matches!(e, yanshi_graph::error::YanshiError::NodeSkipped) => {
              Some(format!("{id}: {e}"))
            }
            _ => None,
          });
          serde_json::json!({ "error": first_error.unwrap_or_else(|| format!("node '{node_id}' did not run")) })
        }
      },
      None => serde_json::to_value(&results).map_err(ToolError::SerdeError)?,
    };
    let content = serde_json::to_string_pretty(&value).map_err(ToolError::SerdeError)?;
    let parts = vec![ToolOutputPart::Resource {
      uri: format!("workflow://{}", self.name),
      mime_type: Some("application/json".to_string()),
      text: Some(content.clone()),
    }];

    if has_node_error {
      Ok(ToolOutput::error_parts(content, parts))
    } else {
      Ok(ToolOutput::success_parts(content, parts))
    }
  }
}

fn default_workflow_parameters_schema() -> Value {
  serde_json::json!({
    "type": "object",
    "description": "Initial workflow inputs keyed by input name.",
    "additionalProperties": true
  })
}

fn params_to_inputs(params: Value) -> Result<AsyncNodeInputs, ToolError> {
  let Value::Object(map) = params else {
    return Err(ToolError::InvalidParams {
      message: "Workflow tool parameters must be a JSON object".to_string(),
    });
  };

  let mut inputs = AsyncNodeInputs::new();
  for (key, value) in map {
    let flow_value =
      serde_json::from_value::<FlowValue>(value.clone()).unwrap_or(FlowValue::Json(value));
    inputs.insert(key, flow_value);
  }
  Ok(inputs)
}

#[cfg(test)]
mod tests {
  use super::*;
  use async_trait::async_trait;
  use serde_json::json;
  use yanshi_core::{
    async_node::{AsyncNodeInputs, AsyncNodeResult},
    error::YanshiError,
    flow::{GraphNode, NodeType},
  };

  struct EchoNode;

  #[async_trait]
  impl yanshi_core::AsyncNode for EchoNode {
    async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
      let mut outputs = AsyncNodeInputs::new();
      outputs.insert(
        "echo".to_string(),
        inputs
          .get("text")
          .cloned()
          .unwrap_or_else(|| FlowValue::Json(json!(""))),
      );
      Ok(outputs)
    }
  }

  struct FailingNode;

  struct SlowNode;

  #[async_trait]
  impl yanshi_core::AsyncNode for FailingNode {
    async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
      Err(YanshiError::NodeExecutionFailed {
        message: "boom".to_string(),
      })
    }
  }

  #[async_trait]
  impl yanshi_core::AsyncNode for SlowNode {
    async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
      tokio::time::sleep(Duration::from_millis(100)).await;
      let mut outputs = AsyncNodeInputs::new();
      outputs.insert("done".to_string(), FlowValue::Json(json!(true)));
      Ok(outputs)
    }
  }

  fn single_node_flow(id: &str, node: Arc<dyn yanshi_core::AsyncNode>) -> Flow {
    Flow::new(vec![GraphNode {
      id: id.to_string(),
      node_type: NodeType::Standard(node),
      dependencies: Vec::new(),
      input_mapping: None,
      run_if: None,
      initial_inputs: AsyncNodeInputs::new(),
    }])
  }

  fn use_writable_home() {
    let home = std::env::temp_dir().join(format!(
      "yanshi-workflow-tool-test-{}",
      uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&home).unwrap();
    // SAFETY: this test helper sets HOME before the workflow code reads it and
    // does not race with another environment mutation in the same test.
    unsafe {
      std::env::set_var("HOME", home);
    }
  }

  #[tokio::test]
  async fn workflow_tool_executes_flow_and_returns_json_resource() {
    use_writable_home();
    let tool = WorkflowTool::new(
      "echo_workflow",
      "Run echo workflow",
      single_node_flow("echo", Arc::new(EchoNode)),
      Arc::new(yanshi_core::CoreFlowRunner::serial()),
    );

    let output = tool.execute(json!({"text": "hello"})).await.unwrap();

    assert!(!output.is_error);
    assert!(output.content.contains("\"echo\""));
    assert!(output.content.contains("hello"));
    assert!(matches!(
      output.parts.as_slice(),
      [ToolOutputPart::Resource {
        uri,
        mime_type: Some(mime_type),
        ..
      }] if uri == "workflow://echo_workflow" && mime_type == "application/json"
    ));
  }

  #[tokio::test]
  async fn workflow_tool_marks_node_errors_as_tool_error_output() {
    use_writable_home();
    let tool = WorkflowTool::new(
      "failing_workflow",
      "Run failing workflow",
      single_node_flow("fail", Arc::new(FailingNode)),
      Arc::new(yanshi_core::CoreFlowRunner::serial()),
    );

    let output = tool.execute(json!({})).await.unwrap();

    assert!(output.is_error);
    assert!(output.content.contains("boom"));
  }

  #[tokio::test]
  async fn workflow_tool_rejects_non_object_params() {
    let tool = WorkflowTool::new(
      "echo_workflow",
      "Run echo workflow",
      single_node_flow("echo", Arc::new(EchoNode)),
      Arc::new(yanshi_core::CoreFlowRunner::serial()),
    );

    let err = tool.execute(json!("bad")).await.unwrap_err();

    assert!(matches!(err, ToolError::InvalidParams { .. }));
  }

  #[tokio::test]
  async fn workflow_tool_times_out_slow_flows() {
    use_writable_home();
    let tool = WorkflowTool::new(
      "slow_workflow",
      "Run slow workflow",
      single_node_flow("slow", Arc::new(SlowNode)),
      Arc::new(yanshi_core::CoreFlowRunner::serial()),
    )
    .with_timeout_ms(10);

    assert_eq!(tool.timeout(), Some(Duration::from_millis(10)));

    let err = tool.execute(json!({})).await.unwrap_err();

    match err {
      ToolError::ExecutionFailed { message } => {
        assert!(message.contains("slow_workflow"));
        assert!(message.contains("timed out after 10ms"));
      }
      other => panic!("expected timeout execution failure, got {other:?}"),
    }
  }

  struct HighWaterNode {
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
    high_water: Arc<std::sync::atomic::AtomicUsize>,
  }

  #[async_trait]
  impl yanshi_core::AsyncNode for HighWaterNode {
    async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
      let current = self
        .in_flight
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
      self
        .high_water
        .fetch_max(current, std::sync::atomic::Ordering::SeqCst);
      tokio::time::sleep(Duration::from_millis(80)).await;
      self
        .in_flight
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
      let mut outputs = AsyncNodeInputs::new();
      outputs.insert(
        "deliverable".to_string(),
        inputs
          .get("topic")
          .cloned()
          .unwrap_or_else(|| FlowValue::Json(json!("ok"))),
      );
      Ok(outputs)
    }
  }

  async fn run_three_workflow_tools_batch(
    concurrency_safe: bool,
  ) -> (usize, crate::AgentRunResult) {
    let _guard = crate::LLM_TEST_LOCK.lock().await;
    use_writable_home();
    let run_dir =
      std::env::temp_dir().join(format!("yanshi-wf-batch-runs-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&run_dir).unwrap();

    let model = format!("mock-wf-batch-{}", uuid::Uuid::new_v4());
    // SAFETY: LLM_TEST_LOCK serializes process-wide mock env mutation.
    unsafe {
      std::env::set_var(
        "YANSHI_MOCK_TOOL_CALLS",
        serde_json::to_string(&vec![
          vec![
            json!({"id": "c1", "name": "wf_report", "arguments": {"topic": "alpha"}}),
            json!({"id": "c2", "name": "wf_mindmap", "arguments": {"topic": "beta"}}),
            json!({"id": "c3", "name": "wf_slides", "arguments": {"topic": "gamma"}}),
          ],
          Vec::<serde_json::Value>::new(),
        ])
        .unwrap(),
      );
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![
          "(unused)",
          r#"{"thought":"all deliverables ready","answer":"done"}"#,
        ])
        .unwrap(),
      );
    }
    let config_yaml = format!(
      "models:\n  {model}:\n    vendor: mock\n    type: text\n    model_id: {model}\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n"
    );
    let cfg_path = run_dir.join("models.yml");
    std::fs::write(&cfg_path, config_yaml).unwrap();
    yanshi_llm::Yanshi::init_with_config(cfg_path.to_str().unwrap())
      .await
      .unwrap();

    let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let high_water = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let runner = Arc::new(yanshi_core::CoreFlowRunner::new(
      yanshi_core::FlowExecutionConfig::concurrent(4).with_run_base_dir(&run_dir),
    ));

    let mut registry = yanshi_tool::ToolRegistry::new();
    for name in ["wf_report", "wf_mindmap", "wf_slides"] {
      let node = Arc::new(HighWaterNode {
        in_flight: in_flight.clone(),
        high_water: high_water.clone(),
      });
      let tool = WorkflowTool::new(
        name,
        "Studio generator",
        single_node_flow("deliverable", node),
        runner.clone(),
      )
      .with_result_output("deliverable", "deliverable")
      .with_concurrency_safe(concurrency_safe);
      assert_eq!(
        tool.idempotency(&json!({})),
        yanshi_tool::ToolIdempotency::Unknown,
        "replay idempotency must stay Unknown for production approval escalation"
      );
      assert_eq!(tool.concurrency_safe(&json!({})), concurrency_safe);
      registry.register(Arc::new(tool));
    }

    let mut agent = crate::react::ReActAgent::new(
      crate::react::ReActConfig::new(&model).with_max_iterations(4),
      Box::new(yanshi_memory::SessionMemory::default_window()),
      Arc::new(registry),
    );
    let result = agent
      .run_with_context(crate::AgentContext::new(
        "s-wf-batch",
        "generate all",
        &model,
      ))
      .await
      .unwrap();
    (high_water.load(std::sync::atomic::Ordering::SeqCst), result)
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  async fn react_agent_runs_workflow_tools_in_parallel_while_keeping_unknown_idempotency() {
    let (peak, result) = run_three_workflow_tools_batch(true).await;
    assert_eq!(result.answer.as_deref(), Some("done"));
    assert!(
      peak >= 2,
      "expected >=2 WorkflowTools executing in parallel across Tokio worker tasks, got peak={peak}"
    );
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  async fn react_agent_serializes_workflow_tools_when_concurrency_safe_is_false() {
    let (peak, result) = run_three_workflow_tools_batch(false).await;
    assert_eq!(result.answer.as_deref(), Some("done"));
    assert_eq!(
      peak, 1,
      "WorkflowTools with concurrency_safe=false must run strictly one at a time"
    );
  }
}
