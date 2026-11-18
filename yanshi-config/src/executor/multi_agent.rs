//! Workflow node for `type: multi_agent`.
//!
//! Dispatches to one of three multi-agent supervisors — `HandoffSupervisor`,
//! `BlackboardSupervisor`, or `DebateSupervisor` (in `yanshi-agents`) —
//! based on the YAML `mode` field. Each participant may either reference a
//! skill directory (`skill: ...`) or declare a self-contained inline persona
//! (`system: ...` with optional `description`, `tools: [web_search, http]`,
//! and `max_steps`) so portable Studio workflows can run without host paths.

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{Value, json};

use yanshi_agents::react::{ReActAgent, ReActConfig};
use yanshi_agents::runtime::AgentEventSink;
use yanshi_agents::supervisor::{
  Blackboard, BlackboardReadTool, BlackboardSchedule, BlackboardStop, BlackboardSupervisorBuilder,
  BlackboardWriteTool, DebateSupervisorBuilder, HandoffSignal, HandoffSupervisorBuilder,
  HandoffTool,
};
use yanshi_agents::yanshi_memory::SessionMemory;
use yanshi_agents::{AgentContext, AgentEvent, AgentRunResult, AgentRuntime};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_llm::{Yanshi, pricing::PricingTable};
use yanshi_skills::{SkillBuilder, SkillLoader, SkillManifest};
use yanshi_tools::builtin::{HttpTool, WebSearchTool};
use yanshi_tools::{SandboxPolicy, ToolRegistry};

// ── YAML config ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct MultiAgentSpec {
  /// Logical name used for handoff targets / blackboard write attribution /
  /// debate proposal labels. Must be unique within the node.
  name: String,
  /// Filesystem path to the skill directory whose manifest builds this agent.
  /// Mutually exclusive with inline `system`.
  #[serde(default)]
  skill: Option<String>,
  /// Optional one-line role summary passed to Handoff / Blackboard supervisors
  /// when building an inline participant.
  #[serde(default)]
  description: Option<String>,
  /// Inline persona / system prompt (self-contained, gateway-safe alternative
  /// to `skill`).
  #[serde(default)]
  system: Option<String>,
  /// Optional SSRF-filtered built-in tools for inline participants
  /// (`web_search`, `http`).
  #[serde(default)]
  tools: Option<Vec<String>>,
  /// Optional max ReAct iterations per turn for inline participants
  /// (defaults to 10, clamped to 1..=32).
  #[serde(default)]
  max_steps: Option<usize>,
}

impl MultiAgentSpec {
  fn validate(&self) -> anyhow::Result<()> {
    let trimmed_name = self.name.trim();
    if trimmed_name.is_empty() {
      anyhow::bail!("multi_agent participant 'name' must not be empty");
    }
    let has_skill = self.skill.as_deref().is_some_and(|s| !s.trim().is_empty());
    let has_system = self.system.as_deref().is_some_and(|s| !s.trim().is_empty());
    if has_skill && has_system {
      anyhow::bail!(
        "multi_agent participant '{trimmed_name}': specify either 'skill' or inline 'system', not both"
      );
    }
    if !has_skill && !has_system {
      anyhow::bail!(
        "multi_agent participant '{trimmed_name}': requires either 'skill' (skill directory) or inline 'system' prompt"
      );
    }
    if let Some(tools) = &self.tools {
      if has_skill {
        anyhow::bail!(
          "multi_agent participant '{trimmed_name}': inline 'tools' cannot be combined with 'skill' (declare tools in SKILL.md / skill.toml)"
        );
      }
      for tool in tools {
        if !matches!(tool.as_str(), "web_search" | "http") {
          anyhow::bail!(
            "multi_agent participant '{trimmed_name}': unsupported inline tool '{tool}' (supported: web_search, http)"
          );
        }
      }
    }
    Ok(())
  }
}

#[derive(Debug, Deserialize)]
struct BlackboardScheduleYaml {
  #[serde(default)]
  mode: Option<String>, // "sequential" | "parallel"
  #[serde(default)]
  agents: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct BlackboardStopYaml {
  #[serde(rename = "type")]
  kind: String, // "all_completed" | "key_set"
  #[serde(default)]
  key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
enum MultiAgentConfig {
  Handoff {
    agents: Vec<MultiAgentSpec>,
    #[serde(default)]
    initial_agent: Option<String>,
    #[serde(default = "default_max_handoffs")]
    max_handoffs: usize,
  },
  Blackboard {
    agents: Vec<MultiAgentSpec>,
    #[serde(default)]
    schedule: Option<BlackboardScheduleYaml>,
    #[serde(default)]
    stop_when: Option<BlackboardStopYaml>,
    #[serde(default)]
    answer_from: Option<String>,
  },
  Debate {
    participants: Vec<MultiAgentSpec>,
    judge: MultiAgentSpec,
    #[serde(default = "default_rounds")]
    rounds: usize,
    #[serde(default)]
    judge_prompt: Option<String>,
  },
}

fn default_max_handoffs() -> usize {
  5
}
fn default_rounds() -> usize {
  1
}

/// Validate `multi_agent` node parameters structurally without constructing
/// runtime tools or touching the filesystem.
pub(crate) fn validate_params(params: &HashMap<String, serde_yaml::Value>) -> anyhow::Result<()> {
  MultiAgentConfig::from_params(params).map(|_| ())
}

impl MultiAgentConfig {
  /// Build a `MultiAgentConfig` from a node's flat YAML parameter map.
  pub fn from_params(params: &HashMap<String, serde_yaml::Value>) -> anyhow::Result<Self> {
    let mut mapping = serde_yaml::Mapping::new();
    for (k, v) in params {
      mapping.insert(serde_yaml::Value::String(k.clone()), v.clone());
    }
    let value = serde_yaml::Value::Mapping(mapping);
    let parsed: Self = serde_yaml::from_value(value)
      .map_err(|e| anyhow::anyhow!("multi_agent config could not be parsed: {e}"))?;
    match &parsed {
      Self::Handoff { agents, .. } | Self::Blackboard { agents, .. } => {
        if agents.is_empty() {
          anyhow::bail!("multi_agent 'agents' list must not be empty");
        }
        for spec in agents {
          spec.validate()?;
        }
      }
      Self::Debate {
        participants,
        judge,
        ..
      } => {
        if participants.is_empty() {
          anyhow::bail!("multi_agent debate 'participants' list must not be empty");
        }
        for spec in participants {
          spec.validate()?;
        }
        judge.validate()?;
      }
    }
    Ok(parsed)
  }
}

struct MultiAgentEventSink;

#[async_trait]
impl AgentEventSink for MultiAgentEventSink {
  async fn emit(&self, event: &AgentEvent) {
    match event {
      AgentEvent::LlmCallCompleted {
        model,
        prompt_tokens,
        completion_tokens,
        ..
      } => {
        if let Ok(table) = PricingTable::shared() {
          let usd = table
            .lookup(model)
            .cost_for_call(*prompt_tokens, *completion_tokens);
          yanshi_core::run_context::record_cost_usd(usd);
        }
      }
      AgentEvent::ToolCallStarted {
        step_index, tool, ..
      } => {
        yanshi_core::run_context::report_progress(
          "multi_agent.tool_call",
          json!({ "step_index": step_index, "tool": tool }),
        );
      }
      _ => {}
    }
  }
}

// ── Node ─────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct MultiAgentNode {
  name: String,
  config: MultiAgentConfig,
}

impl MultiAgentNode {
  pub fn from_params(
    name: impl Into<String>,
    params: &HashMap<String, serde_yaml::Value>,
  ) -> anyhow::Result<Self> {
    Ok(Self {
      name: name.into(),
      config: MultiAgentConfig::from_params(params)?,
    })
  }

  fn input_error(&self, message: impl Into<String>) -> YanshiError {
    YanshiError::NodeInputError {
      message: format!("multi_agent '{}': {}", self.name, message.into()),
    }
  }

  fn execution_error(&self, message: impl Into<String>) -> YanshiError {
    YanshiError::NodeExecutionFailed {
      message: format!("multi_agent '{}': {}", self.name, message.into()),
    }
  }

  async fn load_manifest(
    &self,
    skill_path: &str,
  ) -> Result<(std::path::PathBuf, SkillManifest), YanshiError> {
    let dir = std::path::PathBuf::from(skill_path);
    let manifest = SkillLoader::load(&dir)
      .map_err(|err| self.input_error(format!("failed to load skill '{}': {}", skill_path, err)))?;
    SkillLoader::validate(&manifest, &dir).map_err(|err| {
      self.input_error(format!(
        "skill validation failed for '{}': {}",
        skill_path, err
      ))
    })?;
    Ok((dir, manifest))
  }
}

#[async_trait]
impl AsyncNode for MultiAgentNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let message = get_required_string(inputs, "message")
      .map_err(|m| self.input_error(m))?
      .to_string();
    let model_override = get_optional_string(inputs, "model").map_err(|m| self.input_error(m))?;

    Yanshi::ensure_init()
      .await
      .map_err(|err| YanshiError::ConfigurationError {
        message: format!(
          "multi_agent '{}': failed to initialize LLM: {}",
          self.name, err
        ),
      })?;

    let result = match &self.config {
      MultiAgentConfig::Handoff {
        agents,
        initial_agent,
        max_handoffs,
      } => {
        self
          .run_handoff(
            agents,
            initial_agent.as_deref(),
            *max_handoffs,
            &message,
            model_override,
          )
          .await
      }
      MultiAgentConfig::Blackboard {
        agents,
        schedule,
        stop_when,
        answer_from,
      } => {
        self
          .run_blackboard(
            agents,
            schedule.as_ref(),
            stop_when.as_ref(),
            answer_from.as_deref(),
            &message,
            model_override,
          )
          .await
      }
      MultiAgentConfig::Debate {
        participants,
        judge,
        rounds,
        judge_prompt,
      } => {
        self
          .run_debate(
            participants,
            judge,
            *rounds,
            judge_prompt.as_deref(),
            &message,
            model_override,
          )
          .await
      }
    }?;

    if !result.stop_reason.is_success() {
      let partial_outputs = build_outputs(&self.name, &result)?;
      return Err(YanshiError::NodePartialExecutionFailed {
        message: format!(
          "multi_agent '{}': supervisor stopped before final answer: {:?}",
          self.name, result.stop_reason
        ),
        partial_outputs,
      });
    }
    build_outputs(&self.name, &result)
  }
}

impl MultiAgentNode {
  async fn run_handoff(
    &self,
    specs: &[MultiAgentSpec],
    initial: Option<&str>,
    max_handoffs: usize,
    message: &str,
    model_override: Option<&str>,
  ) -> Result<AgentRunResult, YanshiError> {
    let target_names: Vec<String> = specs.iter().map(|s| s.name.clone()).collect();
    let signal = HandoffSignal::new();
    let handoff_tool: Arc<dyn yanshi_tools::Tool> =
      Arc::new(HandoffTool::new(target_names, signal.clone()));

    let mut builder = HandoffSupervisorBuilder::new().use_signal(signal);
    if let Some(name) = initial {
      builder = builder.initial_agent(name);
    }
    builder = builder.max_handoffs(max_handoffs);

    for spec in specs {
      let extra_tool = handoff_tool.clone();
      let (description, agent) = self
        .build_participant_agent(spec, model_override, vec![extra_tool])
        .await?;
      let name = spec.name.clone();
      builder = builder.add_agent(name, description, move |_handoff| agent);
    }

    let mut supervisor = builder
      .build()
      .map_err(|err| self.input_error(format!("handoff: {err}")))?;
    let event_sink: Arc<dyn AgentEventSink> = Arc::new(MultiAgentEventSink);
    let context = AgentContext::new(supervisor.session_id().to_string(), message, "")
      .with_event_sink(event_sink);
    AgentRuntime::run(&mut supervisor, context)
      .await
      .map_err(|err| self.execution_error(err.to_string()))
  }

  async fn run_blackboard(
    &self,
    specs: &[MultiAgentSpec],
    schedule_yaml: Option<&BlackboardScheduleYaml>,
    stop_yaml: Option<&BlackboardStopYaml>,
    answer_from: Option<&str>,
    message: &str,
    model_override: Option<&str>,
  ) -> Result<AgentRunResult, YanshiError> {
    let blackboard = Blackboard::new();
    let mut builder = BlackboardSupervisorBuilder::new().use_blackboard(blackboard.clone());

    for spec in specs {
      let agent_name = spec.name.clone();
      let extras: Vec<Arc<dyn yanshi_tools::Tool>> = vec![
        Arc::new(BlackboardReadTool::new(blackboard.clone(), &agent_name)),
        Arc::new(BlackboardWriteTool::new(blackboard.clone(), &agent_name)),
      ];
      let (description, agent) = self
        .build_participant_agent(spec, model_override, extras)
        .await?;
      builder = builder.add_agent(agent_name, description, move |_bb| agent);
    }

    if let Some(schedule_yaml) = schedule_yaml {
      let mode = schedule_yaml.mode.as_deref().unwrap_or("sequential");
      let agents = schedule_yaml
        .agents
        .clone()
        .unwrap_or_else(|| specs.iter().map(|s| s.name.clone()).collect());
      let schedule = match mode {
        "sequential" => BlackboardSchedule::Sequential(agents),
        "parallel" => BlackboardSchedule::Parallel(agents),
        other => {
          return Err(self.input_error(format!(
            "unknown blackboard schedule '{}': expected sequential|parallel",
            other
          )));
        }
      };
      builder = builder.schedule(schedule);
    }

    if let Some(stop) = stop_yaml {
      let stop = match stop.kind.as_str() {
        "all_completed" => BlackboardStop::AllAgentsCompleted,
        "key_set" => {
          let key = stop
            .key
            .as_ref()
            .ok_or_else(|| self.input_error("blackboard stop_when type=key_set requires 'key'"))?;
          BlackboardStop::KeySet(key.clone())
        }
        other => {
          return Err(self.input_error(format!(
            "unknown blackboard stop type '{}': expected all_completed|key_set",
            other
          )));
        }
      };
      builder = builder.stop_when(stop);
    }

    if let Some(key) = answer_from {
      builder = builder.answer_from(key);
    }

    let mut supervisor = builder
      .build()
      .map_err(|err| self.input_error(format!("blackboard: {err}")))?;
    let event_sink: Arc<dyn AgentEventSink> = Arc::new(MultiAgentEventSink);
    let context = AgentContext::new(supervisor.session_id().to_string(), message, "")
      .with_event_sink(event_sink);
    AgentRuntime::run(&mut supervisor, context)
      .await
      .map_err(|err| self.execution_error(err.to_string()))
  }

  async fn run_debate(
    &self,
    participants: &[MultiAgentSpec],
    judge: &MultiAgentSpec,
    rounds: usize,
    judge_prompt: Option<&str>,
    message: &str,
    model_override: Option<&str>,
  ) -> Result<AgentRunResult, YanshiError> {
    let mut builder = DebateSupervisorBuilder::new().rounds(rounds);
    if let Some(prompt) = judge_prompt {
      builder = builder.judge_prompt(prompt);
    }

    for spec in participants {
      let (_desc, agent) = self
        .build_participant_agent(spec, model_override, Vec::new())
        .await?;
      builder = builder.add_participant(spec.name.clone(), agent);
    }
    let (_judge_desc, judge_agent) = self
      .build_participant_agent(judge, model_override, Vec::new())
      .await?;
    builder = builder.judge(judge_agent);

    let mut supervisor = builder
      .build()
      .map_err(|err| self.input_error(format!("debate: {err}")))?;
    let event_sink: Arc<dyn AgentEventSink> = Arc::new(MultiAgentEventSink);
    let context = AgentContext::new(supervisor.session_id().to_string(), message, "")
      .with_event_sink(event_sink);
    AgentRuntime::run(&mut supervisor, context)
      .await
      .map_err(|err| self.execution_error(err.to_string()))
  }

  async fn build_participant_agent(
    &self,
    spec: &MultiAgentSpec,
    model_override: Option<&str>,
    extra_tools: Vec<Arc<dyn yanshi_tools::Tool>>,
  ) -> Result<(String, ReActAgent), YanshiError> {
    if let Some(skill_path) = spec.skill.as_deref().filter(|s| !s.trim().is_empty()) {
      let (dir, mut manifest) = self.load_manifest(skill_path).await?;
      if let Some(model) = model_override.filter(|m| !m.trim().is_empty()) {
        manifest.model.name = Some(model.to_string());
      }
      let description = manifest.skill.description.clone();
      let agent = SkillBuilder::build_with_extra_tools(&manifest, &dir, extra_tools)
        .await
        .map_err(|err| {
          self.execution_error(format!(
            "failed to build agent for '{}': {}",
            spec.name, err
          ))
        })?;
      return Ok((description, agent));
    }

    let system = spec
      .system
      .as_deref()
      .filter(|s| !s.trim().is_empty())
      .ok_or_else(|| {
        self.input_error(format!(
          "participant '{}' requires either 'skill' or inline 'system'",
          spec.name
        ))
      })?;
    let resolved_model = model_override
      .filter(|m| !m.trim().is_empty())
      .unwrap_or("gpt-4o");
    let max_steps = spec.max_steps.unwrap_or(10).clamp(1, 32);
    let config = ReActConfig::new(resolved_model)
      .with_persona(system.to_string())
      .with_max_iterations(max_steps)
      .with_budget_tokens(200_000);

    let mut registry = ToolRegistry::new();
    if let Some(tools) = &spec.tools {
      for tool_name in tools {
        match tool_name.as_str() {
          "web_search" => {
            let tool =
              WebSearchTool::default_policy().map_err(|err| YanshiError::ConfigurationError {
                message: format!(
                  "multi_agent '{}': failed to build web_search tool for '{}': {err}",
                  self.name, spec.name
                ),
              })?;
            registry.register(Arc::new(tool));
          }
          "http" => {
            let tool = HttpTool::new(Arc::new(SandboxPolicy::default())).map_err(|err| {
              YanshiError::ConfigurationError {
                message: format!(
                  "multi_agent '{}': failed to build http tool for '{}': {err}",
                  self.name, spec.name
                ),
              }
            })?;
            registry.register(Arc::new(tool));
          }
          other => {
            return Err(self.input_error(format!(
              "participant '{}' unsupported inline tool '{other}' (supported: web_search, http)",
              spec.name
            )));
          }
        }
      }
    }
    for tool in extra_tools {
      registry.register(tool);
    }

    let description = spec
      .description
      .clone()
      .filter(|d| !d.trim().is_empty())
      .unwrap_or_else(|| spec.name.clone());
    let agent = ReActAgent::new(
      config,
      Box::new(SessionMemory::default_window()),
      Arc::new(registry),
    );
    Ok((description, agent))
  }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn get_required_string<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Result<&'a str, String> {
  get_optional_string(inputs, key)?.ok_or_else(|| format!("required input '{}' is missing", key))
}

fn get_optional_string<'a>(
  inputs: &'a AsyncNodeInputs,
  key: &str,
) -> Result<Option<&'a str>, String> {
  match inputs.get(key) {
    None => Ok(None),
    Some(FlowValue::Json(Value::String(value))) => Ok(Some(value.as_str())),
    Some(_) => Err(format!("input '{}' must be a string", key)),
  }
}

fn build_outputs(node_name: &str, result: &AgentRunResult) -> AsyncNodeResult {
  let response = result.answer.clone().unwrap_or_default();
  let stop_reason =
    serde_json::to_value(&result.stop_reason).map_err(|err| YanshiError::NodeExecutionFailed {
      message: format!(
        "multi_agent '{}': failed to serialize stop reason: {}",
        node_name, err
      ),
    })?;
  let agent_result =
    serde_json::to_value(result).map_err(|err| YanshiError::NodeExecutionFailed {
      message: format!(
        "multi_agent '{}': failed to serialize runtime result: {}",
        node_name, err
      ),
    })?;

  let mut outputs: HashMap<String, FlowValue> = HashMap::new();
  outputs.insert("response".to_string(), FlowValue::Json(json!(response)));
  outputs.insert(
    "session_id".to_string(),
    FlowValue::Json(json!(result.session_id)),
  );
  outputs.insert("stop_reason".to_string(), FlowValue::Json(stop_reason));
  outputs.insert("agent_result".to_string(), FlowValue::Json(agent_result));
  Ok(outputs)
}

// Avoid unused-import warning on the helper module.
#[allow(dead_code)]
fn _typecheck_path(_p: &Path) {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
  use super::*;

  fn yaml_map(pairs: &[(&str, &str)]) -> HashMap<String, serde_yaml::Value> {
    let mut m = HashMap::new();
    for (k, v) in pairs {
      m.insert(k.to_string(), serde_yaml::from_str(v).unwrap());
    }
    m
  }

  #[test]
  fn parses_handoff_config_with_defaults() {
    let params = yaml_map(&[
      ("mode", "handoff"),
      ("agents", "[{name: a, skill: ./a}, {name: b, skill: ./b}]"),
    ]);
    let cfg = MultiAgentConfig::from_params(&params).unwrap();
    match cfg {
      MultiAgentConfig::Handoff {
        agents,
        initial_agent,
        max_handoffs,
      } => {
        assert_eq!(agents.len(), 2);
        assert_eq!(agents[0].name, "a");
        assert!(initial_agent.is_none());
        assert_eq!(max_handoffs, 5);
      }
      other => panic!("expected handoff, got {other:?}"),
    }
  }

  #[test]
  fn parses_blackboard_config_with_full_options() {
    let params = yaml_map(&[
      ("mode", "blackboard"),
      ("agents", "[{name: r, skill: ./r}, {name: w, skill: ./w}]"),
      ("schedule", "{mode: parallel, agents: [r, w]}"),
      ("stop_when", "{type: key_set, key: report}"),
      ("answer_from", "report"),
    ]);
    let cfg = MultiAgentConfig::from_params(&params).unwrap();
    match cfg {
      MultiAgentConfig::Blackboard {
        agents,
        schedule,
        stop_when,
        answer_from,
      } => {
        assert_eq!(agents.len(), 2);
        let schedule = schedule.expect("schedule");
        assert_eq!(schedule.mode.as_deref(), Some("parallel"));
        let stop = stop_when.expect("stop_when");
        assert_eq!(stop.kind, "key_set");
        assert_eq!(stop.key.as_deref(), Some("report"));
        assert_eq!(answer_from.as_deref(), Some("report"));
      }
      other => panic!("expected blackboard, got {other:?}"),
    }
  }

  #[test]
  fn parses_debate_config_with_judge() {
    let params = yaml_map(&[
      ("mode", "debate"),
      (
        "participants",
        "[{name: a, skill: ./a}, {name: b, skill: ./b}]",
      ),
      ("judge", "{name: judge, skill: ./judge}"),
      ("rounds", "2"),
    ]);
    let cfg = MultiAgentConfig::from_params(&params).unwrap();
    match cfg {
      MultiAgentConfig::Debate {
        participants,
        judge,
        rounds,
        judge_prompt,
      } => {
        assert_eq!(participants.len(), 2);
        assert_eq!(judge.name, "judge");
        assert_eq!(rounds, 2);
        assert!(judge_prompt.is_none());
      }
      other => panic!("expected debate, got {other:?}"),
    }
  }

  #[test]
  fn rejects_unknown_mode() {
    let params = yaml_map(&[("mode", "ghost")]);
    let err = MultiAgentConfig::from_params(&params).unwrap_err();
    let s = err.to_string();
    assert!(s.contains("multi_agent"));
  }

  #[test]
  fn rejects_missing_judge_in_debate() {
    let params = yaml_map(&[
      ("mode", "debate"),
      ("participants", "[{name: a, skill: ./a}]"),
    ]);
    let err = MultiAgentConfig::from_params(&params).unwrap_err();
    let s = err.to_string();
    assert!(
      s.contains("judge") || s.contains("missing"),
      "expected error about missing 'judge', got: {s}"
    );
  }

  #[test]
  fn parses_inline_specs_and_rejects_ambiguous_or_unsafe_tools() {
    let ok = yaml_map(&[
      ("mode", "debate"),
      (
        "participants",
        "[{name: pro, description: Pro, system: 'Argue yes', tools: [web_search]}, {name: con, system: 'Argue no'}]",
      ),
      ("judge", "{name: mod_host, system: 'Synthesize verdict'}"),
      ("rounds", "2"),
    ]);
    assert!(MultiAgentConfig::from_params(&ok).is_ok());

    let missing_both = yaml_map(&[("mode", "handoff"), ("agents", "[{name: a}]")]);
    assert!(
      MultiAgentConfig::from_params(&missing_both)
        .unwrap_err()
        .to_string()
        .contains("requires either 'skill'")
    );

    let both_set = yaml_map(&[
      ("mode", "handoff"),
      ("agents", "[{name: a, skill: ./a, system: 'inline'}]"),
    ]);
    assert!(
      MultiAgentConfig::from_params(&both_set)
        .unwrap_err()
        .to_string()
        .contains("not both")
    );

    let bad_tool = yaml_map(&[
      ("mode", "blackboard"),
      ("agents", "[{name: a, system: 'inline', tools: [shell]}]"),
    ]);
    assert!(
      MultiAgentConfig::from_params(&bad_tool)
        .unwrap_err()
        .to_string()
        .contains("unsupported inline tool 'shell'")
    );
  }
}
