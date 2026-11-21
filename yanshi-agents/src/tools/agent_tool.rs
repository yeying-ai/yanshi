//! `AgentTool` — wraps a [`ReActAgent`] as a [`Tool`] so that one agent can
//! delegate sub-tasks to another agent via the normal tool-calling mechanism.
//!
//! # Parameters schema (what the LLM passes)
//! ```json
//! { "message": "<task description for the sub-agent>" }
//! ```
//!
//! # Usage
//! Register `AgentTool` in a parent agent's
//! [`ToolRegistry`](yanshi_tool::ToolRegistry) just like any other tool.
//! The parent LLM will call it by name with a `message` argument, and the
//! sub-agent's final answer is returned as the tool output.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::Mutex;
use yanshi_tool::{Tool, ToolError, ToolOutput};

use crate::react::agent::ReActAgent;

// ── Public struct ─────────────────────────────────────────────────────────────

/// A [`Tool`] that delegates execution to a [`ReActAgent`].
///
/// ```rust,no_run
/// use yanshi_agents::tools::AgentTool;
/// use yanshi_agents::react::{ReActAgent, ReActConfig};
/// use yanshi_memory::SessionMemory;
/// use yanshi_tool::ToolRegistry;
/// use yanshi_tools::sandbox::SandboxPolicy;
/// use std::sync::Arc;
///
/// let sub = ReActAgent::new(
///     ReActConfig::new("gpt-4o"),
///     Box::new(SessionMemory::default_window()),
///     Arc::new(ToolRegistry::new()),
/// );
///
/// let mut registry = ToolRegistry::new();
/// registry.register(Arc::new(AgentTool::new("researcher", "Search for facts", sub)));
/// ```
pub struct AgentTool {
  tool_name: String,
  tool_description: String,
  agent: Arc<Mutex<ReActAgent>>,
  /// AJ2: when `true` (default for [`Self::new`]), each invocation forks an
  /// isolated session and executes lock-free in parallel; `false` (default for
  /// [`Self::from_shared`]) serializes access to the shared session.
  isolated_per_call: bool,
}

impl AgentTool {
  /// Create a new `AgentTool`.
  ///
  /// * `name` — the tool name the LLM uses to invoke this agent.
  /// * `description` — human-readable description shown in the system prompt.
  /// * `agent` — the [`ReActAgent`] template that handles delegated tasks
  ///   (forked into an isolated session per invocation by default).
  pub fn new(name: impl Into<String>, description: impl Into<String>, agent: ReActAgent) -> Self {
    Self {
      tool_name: name.into(),
      tool_description: description.into(),
      agent: Arc::new(Mutex::new(agent)),
      isolated_per_call: true,
    }
  }

  /// Construct from a shared `Arc<Mutex<ReActAgent>>` — useful when the same
  /// agent instance is also used as an [`AgentNode`](crate::nodes::AgentNode)
  /// with shared conversation state.
  pub fn from_shared(
    name: impl Into<String>,
    description: impl Into<String>,
    agent: Arc<Mutex<ReActAgent>>,
  ) -> Self {
    Self {
      tool_name: name.into(),
      tool_description: description.into(),
      agent,
      isolated_per_call: false,
    }
  }

  /// Opt into sharing a single stateful `session_id` across calls (disables
  /// parallel batch execution for this tool).
  pub fn with_shared_session(mut self) -> Self {
    self.isolated_per_call = false;
    self
  }

  /// Return a cloned handle to the inner agent lock.
  pub fn agent_handle(&self) -> Arc<Mutex<ReActAgent>> {
    self.agent.clone()
  }
}

// ── Tool implementation ───────────────────────────────────────────────────────

#[async_trait]
impl Tool for AgentTool {
  fn name(&self) -> &str {
    &self.tool_name
  }

  fn description(&self) -> &str {
    &self.tool_description
  }

  fn parameters_schema(&self) -> Value {
    json!({
        "type": "object",
        "properties": {
            "message": {
                "type": "string",
                "description": "The task or question to send to the sub-agent."
            }
        },
        "required": ["message"]
    })
  }

  fn concurrency_safe(&self, _params: &Value) -> bool {
    self.isolated_per_call
  }

  async fn execute(&self, params: Value) -> Result<ToolOutput, ToolError> {
    let message = params["message"]
      .as_str()
      .ok_or_else(|| ToolError::InvalidParams {
        message: format!(
          "AgentTool '{}': 'message' parameter must be a string",
          self.tool_name
        ),
      })?
      .to_string();

    let run_result = if self.isolated_per_call {
      let mut forked = {
        let guard = self.agent.lock().await;
        guard.fork_isolated()
      };
      forked.run(&message).await
    } else {
      let mut agent = self.agent.lock().await;
      agent.run(&message).await
    };
    match run_result {
      Ok(answer) => Ok(ToolOutput::success(answer)),
      Err(e) => Ok(ToolOutput::error(e.to_string())),
    }
  }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;
  use yanshi_memory::SessionMemory;
  use yanshi_tool::ToolRegistry;

  use crate::react::{ReActAgent, ReActConfig};

  fn make_agent_tool(name: &str, desc: &str) -> AgentTool {
    let agent = ReActAgent::new(
      ReActConfig::new("gpt-4o"),
      Box::new(SessionMemory::default_window()),
      Arc::new(ToolRegistry::new()),
    );
    AgentTool::new(name, desc, agent)
  }

  // ── Trait accessors ───────────────────────────────────────────────────────

  #[test]
  fn name_matches_constructor() {
    let t = make_agent_tool("researcher", "Finds information");
    assert_eq!(t.name(), "researcher");
  }

  #[test]
  fn description_matches_constructor() {
    let t = make_agent_tool("researcher", "Finds information");
    assert_eq!(t.description(), "Finds information");
  }

  #[test]
  fn parameters_schema_has_message_field() {
    let t = make_agent_tool("any", "any");
    let schema = t.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert!(
      schema["properties"]["message"].is_object(),
      "schema must have a 'message' property"
    );
    let required = schema["required"]
      .as_array()
      .expect("required must be array");
    assert!(
      required.iter().any(|v| v.as_str() == Some("message")),
      "'message' must be in required array"
    );
  }

  #[test]
  fn from_shared_returns_same_arc() {
    let agent = ReActAgent::new(
      ReActConfig::new("gpt-4o"),
      Box::new(SessionMemory::default_window()),
      Arc::new(ToolRegistry::new()),
    );
    let shared = Arc::new(Mutex::new(agent));
    let t = AgentTool::from_shared("a", "b", shared.clone());
    assert!(Arc::ptr_eq(&t.agent_handle(), &shared));
  }

  // ── execute() — missing message parameter ────────────────────────────────

  #[tokio::test]
  async fn execute_without_message_returns_invalid_input_error() {
    let t = make_agent_tool("test", "test");
    let result = t.execute(json!({})).await;
    assert!(
      matches!(result, Err(ToolError::InvalidParams { .. })),
      "missing 'message' should return InvalidParams"
    );
  }

  #[tokio::test]
  async fn execute_with_null_message_returns_invalid_input_error() {
    let t = make_agent_tool("test", "test");
    let result = t.execute(json!({"message": null})).await;
    assert!(matches!(result, Err(ToolError::InvalidParams { .. })));
  }

  // ── execute() — LLM failures are soft errors ──────────────────────────────
  //
  // When the sub-agent returns ReActError (e.g. LLM unreachable), we convert
  // it to ToolOutput::error (not Err(ToolError)) so the orchestrator can
  // continue reasoning.

  #[tokio::test]
  async fn execute_agent_failure_returns_tool_output_error_not_tool_error() {
    let agent = ReActAgent::new(
      ReActConfig::new(""), // empty model → LLM call will fail
      Box::new(SessionMemory::default_window()),
      Arc::new(ToolRegistry::new()),
    );
    let t = AgentTool::new("failing", "will fail", agent);
    let result = t.execute(json!({"message": "hello"})).await;
    // Must be Ok(ToolOutput { is_error: true, … }), NOT Err(ToolError)
    let output = result.expect("should be Ok(ToolOutput::error)");
    assert!(
      output.is_error,
      "sub-agent failure should produce ToolOutput::error"
    );
  }

  // ── prompt_description contains name and description ─────────────────────

  #[test]
  fn prompt_description_includes_name_and_description() {
    let t = make_agent_tool("code-reviewer", "Reviews Rust code for correctness");
    let desc = t.prompt_description();
    assert!(desc.contains("code-reviewer"));
    assert!(desc.contains("Reviews Rust code for correctness"));
  }

  struct HighWaterVerifier {
    in_flight: Arc<std::sync::atomic::AtomicUsize>,
    high_water: Arc<std::sync::atomic::AtomicUsize>,
  }

  #[async_trait]
  impl crate::verification::VerificationStrategy for HighWaterVerifier {
    fn name(&self) -> &'static str {
      "high-water"
    }

    async fn verify(
      &self,
      _ctx: &crate::verification::VerificationContext,
    ) -> Result<crate::verification::VerificationOutcome, crate::verification::VerificationError>
    {
      let cur = self
        .in_flight
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
        + 1;
      self
        .high_water
        .fetch_max(cur, std::sync::atomic::Ordering::SeqCst);
      tokio::time::sleep(std::time::Duration::from_millis(80)).await;
      self
        .in_flight
        .fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
      Ok(crate::verification::VerificationOutcome::Approved)
    }
  }

  async fn run_supervisor_three_delegations(shared_session: bool) -> usize {
    let _guard = crate::LLM_TEST_LOCK.lock().await;
    let sub_model = format!("mock-sub-{}", uuid::Uuid::new_v4());
    let sup_model = format!("mock-sup-{}", uuid::Uuid::new_v4());
    let cfg_path =
      std::env::temp_dir().join(format!("yanshi-agent-tool-{}.yml", uuid::Uuid::new_v4()));

    unsafe {
      std::env::remove_var("YANSHI_MOCK_RESPONSE");
      std::env::set_var(
        "YANSHI_MOCK_TOOL_CALLS",
        serde_json::to_string(&vec![vec![
          json!({"id": "d1", "name": "researcher", "arguments": {"message": "topic A"}}),
          json!({"id": "d2", "name": "researcher", "arguments": {"message": "topic B"}}),
          json!({"id": "d3", "name": "researcher", "arguments": {"message": "topic C"}}),
        ]])
        .unwrap(),
      );
      std::env::set_var(
        "YANSHI_MOCK_RESPONSES",
        serde_json::to_string(&vec![
          "(unused)",
          r#"{"thought":"sub-agent done","answer":"sub answer"}"#,
          r#"{"thought":"sub-agent done","answer":"sub answer"}"#,
          r#"{"thought":"sub-agent done","answer":"sub answer"}"#,
          r#"{"thought":"synthesized","answer":"all three topics synthesized"}"#,
        ])
        .unwrap(),
      );
    }
    std::fs::write(
      &cfg_path,
      format!(
        "models:\n  {sup_model}:\n    vendor: mock\n    type: text\n    model_id: {sup_model}\n  {sub_model}:\n    vendor: mock\n    type: text\n    model_id: {sub_model}\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n"
      ),
    )
    .unwrap();
    yanshi_llm::Yanshi::init_with_config(cfg_path.to_str().unwrap())
      .await
      .unwrap();
    unsafe {
      std::env::remove_var("YANSHI_MOCK_TOOL_CALLS");
      std::env::remove_var("YANSHI_MOCK_RESPONSES");
    }

    let in_flight = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let high_water = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let sub = ReActAgent::new(
      ReActConfig::new(&sub_model).with_max_iterations(2),
      Box::new(SessionMemory::default_window()),
      Arc::new(ToolRegistry::new()),
    )
    .with_verification_strategy(Arc::new(HighWaterVerifier {
      in_flight,
      high_water: high_water.clone(),
    }));

    let mut tool = AgentTool::new("researcher", "Delegate research", sub);
    if shared_session {
      tool = tool.with_shared_session();
    }
    assert_eq!(tool.concurrency_safe(&json!({})), !shared_session);
    assert_eq!(
      tool.idempotency(&json!({})),
      yanshi_tool::ToolIdempotency::Unknown
    );

    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(tool));
    let mut orchestrator = ReActAgent::new(
      ReActConfig::new(&sup_model).with_max_iterations(4),
      Box::new(SessionMemory::default_window()),
      Arc::new(registry),
    );
    let answer = orchestrator.run("research A, B, C").await.unwrap();
    assert_eq!(answer, "all three topics synthesized");
    high_water.load(std::sync::atomic::Ordering::SeqCst)
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  async fn agent_tool_delegations_run_in_parallel_by_default() {
    let peak = run_supervisor_three_delegations(false).await;
    assert!(
      peak >= 2,
      "expected >=2 concurrent sub-agent delegations in one supervisor turn, got peak={peak}"
    );
  }

  #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
  async fn agent_tool_with_shared_session_serializes_delegations() {
    let peak = run_supervisor_three_delegations(true).await;
    assert_eq!(
      peak, 1,
      "shared-session AgentTool must serialize sub-agent turns"
    );
  }
}
