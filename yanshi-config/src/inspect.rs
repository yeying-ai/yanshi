//! Static workflow topology, input schema, and validation inspection.
//!
//! Powers `POST /v1/workflows/inspect` in both `yanshi-server` and the
//! local desktop runtime so the Studio DAG canvas and Release App Runner
//! share one exact topological model without executing any node.

use std::collections::{BTreeMap, BTreeSet, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::{
  schema::validate_flow_definition,
  v2::{FlowDefinitionV2, NodeDefinitionV2},
};

/// Inferred UI control kind for a declared workflow input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectedInputKind {
  String,
  Number,
  Boolean,
  Json,
}

/// One declared workflow input (`inputs:` block), ready for auto-generating
/// a Release Mode parameter form or Studio debug input drawer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectedInput {
  pub name: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub required: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub default: Option<Value>,
  pub kind: InspectedInputKind,
}

/// Relationship type for a directed edge in the workflow DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectedEdgeKind {
  Dependency,
  InputMapping,
  Condition,
}

/// Directed edge between two sibling nodes (`from` -> `to`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectedEdge {
  pub from: String,
  pub to: String,
  pub kind: InspectedEdgeKind,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
}

/// Static metadata for one node in a workflow (or nested inside `map`/`while`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectedNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  pub dependencies: Vec<String>,
  pub input_mapping: BTreeMap<String, String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub run_if: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub timeout_ms: Option<u64>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub max_retries: Option<u32>,
  pub parameters: Value,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub sub_nodes: Vec<InspectedNode>,
  pub level: usize,
}

/// Validation report surfaced alongside the static topology.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InspectDiagnostics {
  pub valid: bool,
  pub issues: Vec<String>,
  pub warnings: Vec<String>,
}

/// Complete response envelope for workflow inspection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InspectWorkflowResponse {
  pub valid: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub name: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub cost_limit_usd: Option<f64>,
  pub inputs: Vec<InspectedInput>,
  pub nodes: Vec<InspectedNode>,
  pub edges: Vec<InspectedEdge>,
  /// Deterministic topological execution order (`Flow::topological_sort`).
  pub execution_order: Vec<String>,
  /// Nodes grouped by topological depth level (`0..=max_level`).
  pub levels: Vec<Vec<String>>,
  pub diagnostics: InspectDiagnostics,
}

/// Parse, validate, and extract the static DAG topology of a workflow YAML/JSON string.
///
/// Syntax errors in `yaml_content` do not fail the call — they return an
/// [`InspectWorkflowResponse`] with `valid: false` and the parser message in
/// `diagnostics.issues`, allowing live editors to display diagnostics inline.
pub fn inspect_workflow(yaml_content: &str) -> InspectWorkflowResponse {
  let flow_def: FlowDefinitionV2 = match serde_yaml::from_str(yaml_content) {
    Ok(def) => def,
    Err(err) => {
      let msg = format!("YAML parse error: {err}");
      return InspectWorkflowResponse {
        valid: false,
        name: None,
        cost_limit_usd: None,
        inputs: Vec::new(),
        nodes: Vec::new(),
        edges: Vec::new(),
        execution_order: Vec::new(),
        levels: Vec::new(),
        diagnostics: InspectDiagnostics {
          valid: false,
          issues: vec![msg],
          warnings: Vec::new(),
        },
      };
    }
  };

  inspect_flow_definition(&flow_def)
}

/// Inspect an already-parsed [`FlowDefinitionV2`].
pub fn inspect_flow_definition(flow_def: &FlowDefinitionV2) -> InspectWorkflowResponse {
  let report = validate_flow_definition(flow_def);
  let mut issues = report.issues;
  let warnings = report.warnings;

  let inputs = inspect_inputs(flow_def);
  let edges = extract_edges(&flow_def.nodes);
  let (execution_order, level_map, cycle_issue) = compute_topology(&flow_def.nodes, &edges);
  if let Some(cycle) = cycle_issue {
    issues.push(cycle);
  }

  let nodes: Vec<InspectedNode> = flow_def
    .nodes
    .iter()
    .map(|node| inspect_node(node, level_map.get(&node.id).copied().unwrap_or(0)))
    .collect();

  let max_level = nodes.iter().map(|n| n.level).max();
  let levels = match max_level {
    Some(max_l) if !execution_order.is_empty() => {
      let mut grouped = vec![Vec::new(); max_l + 1];
      for id in &execution_order {
        let lvl = level_map.get(id).copied().unwrap_or(0);
        if let Some(bucket) = grouped.get_mut(lvl) {
          bucket.push(id.clone());
        }
      }
      grouped
    }
    _ => Vec::new(),
  };

  let valid = issues.is_empty();
  InspectWorkflowResponse {
    valid,
    name: Some(flow_def.name.clone()),
    cost_limit_usd: flow_def.cost_limit_usd,
    inputs,
    nodes,
    edges,
    execution_order,
    levels,
    diagnostics: InspectDiagnostics {
      valid,
      issues,
      warnings,
    },
  }
}

fn inspect_inputs(flow_def: &FlowDefinitionV2) -> Vec<InspectedInput> {
  let mut entries: Vec<InspectedInput> = flow_def
    .inputs
    .iter()
    .map(|(name, def)| {
      let default: Option<Value> = def
        .default
        .as_ref()
        .and_then(|val| serde_yaml::from_value(val.clone()).ok());
      let kind = match &default {
        Some(Value::Bool(_)) => InspectedInputKind::Boolean,
        Some(Value::Number(_)) => InspectedInputKind::Number,
        Some(Value::Array(_)) | Some(Value::Object(_)) => InspectedInputKind::Json,
        _ => InspectedInputKind::String,
      };
      InspectedInput {
        name: name.clone(),
        description: def.description.clone(),
        required: def.required,
        default,
        kind,
      }
    })
    .collect();

  // Required inputs first, then alphabetical by name for stable UI forms.
  entries.sort_by(|a, b| {
    b.required
      .cmp(&a.required)
      .then_with(|| a.name.cmp(&b.name))
  });
  entries
}

fn inspect_node(node: &NodeDefinitionV2, level: usize) -> InspectedNode {
  let mut input_mapping = BTreeMap::new();
  for (k, v) in &node.input_mapping {
    input_mapping.insert(k.clone(), v.clone());
  }

  let mut params_map = serde_json::Map::new();
  for (k, v) in &node.parameters {
    if (node.node_type == "map" && k == "template") || (node.node_type == "while" && k == "do") {
      continue;
    }
    if let Ok(json_val) = serde_yaml::from_value::<Value>(v.clone()) {
      params_map.insert(k.clone(), json_val);
    }
  }

  let sub_nodes = extract_sub_nodes(node);

  InspectedNode {
    id: node.id.clone(),
    node_type: node.node_type.clone(),
    dependencies: node.dependencies.clone(),
    input_mapping,
    run_if: node.run_if.clone(),
    timeout_ms: node.timeout_ms,
    max_retries: node.max_retries,
    parameters: Value::Object(params_map),
    sub_nodes,
    level,
  }
}

fn extract_sub_nodes(node: &NodeDefinitionV2) -> Vec<InspectedNode> {
  let block_key = match node.node_type.as_str() {
    "map" => "template",
    "while" => "do",
    _ => return Vec::new(),
  };
  let Some(raw) = node.parameters.get(block_key) else {
    return Vec::new();
  };
  let Ok(children) = serde_yaml::from_value::<Vec<NodeDefinitionV2>>(raw.clone()) else {
    return Vec::new();
  };
  let child_edges = extract_edges(&children);
  let (_, child_levels, _) = compute_topology(&children, &child_edges);
  children
    .iter()
    .map(|child| inspect_node(child, child_levels.get(&child.id).copied().unwrap_or(0)))
    .collect()
}

fn parse_mapping_source(mapping: &str) -> Option<(&str, &str)> {
  let path = mapping
    .trim()
    .trim_start_matches("{{")
    .trim_end_matches("}}")
    .trim();
  let parts: Vec<_> = path.split('.').collect();
  if parts.len() == 4 && parts[0] == "nodes" && parts[2] == "outputs" {
    Some((parts[1], parts[3]))
  } else {
    None
  }
}

fn parse_condition_sources(condition: &str) -> Vec<String> {
  let is_ident = |c: char| c.is_alphanumeric() || c == '_' || c == '-';
  let mut refs = Vec::new();
  for (start, _) in condition.match_indices("nodes.") {
    if condition[..start].chars().next_back().is_some_and(is_ident) {
      continue;
    }
    let rest = &condition[start + "nodes.".len()..];
    let end = rest.find(|c: char| !is_ident(c)).unwrap_or(rest.len());
    let candidate = &rest[..end];
    if !candidate.is_empty() && !refs.iter().any(|seen| seen == candidate) {
      refs.push(candidate.to_string());
    }
  }
  refs
}

fn extract_edges(nodes: &[NodeDefinitionV2]) -> Vec<InspectedEdge> {
  let known_ids: HashSet<&str> = nodes.iter().map(|n| n.id.as_str()).collect();
  let mut edges = Vec::new();

  for node in nodes {
    let mut connected_from: BTreeSet<String> = BTreeSet::new();

    // 1. Explicit `input_mapping` edges carry rich field-binding labels.
    let mut mapping_keys: Vec<(&String, &String)> = node.input_mapping.iter().collect();
    mapping_keys.sort_by_key(|(ka, _)| *ka);

    let mut labels_by_source: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (target_input, expr) in mapping_keys {
      if let Some((source_node, source_output)) = parse_mapping_source(expr)
        && source_node != node.id
        && known_ids.contains(source_node)
      {
        labels_by_source
          .entry(source_node.to_string())
          .or_default()
          .push(format!("{source_output} → {target_input}"));
      }
    }
    for (source_node, labels) in labels_by_source {
      connected_from.insert(source_node.clone());
      edges.push(InspectedEdge {
        from: source_node,
        to: node.id.clone(),
        kind: InspectedEdgeKind::InputMapping,
        label: Some(labels.join(", ")),
      });
    }

    // 2. Conditional `run_if` references (dashed edges in Studio).
    if let Some(condition) = &node.run_if {
      for source_node in parse_condition_sources(condition) {
        if source_node != node.id && known_ids.contains(source_node.as_str()) {
          connected_from.insert(source_node.clone());
          edges.push(InspectedEdge {
            from: source_node,
            to: node.id.clone(),
            kind: InspectedEdgeKind::Condition,
            label: Some("run_if".to_string()),
          });
        }
      }
    }

    // 3. Explicit `dependencies` not already covered by `input_mapping` or `run_if`.
    for dep in &node.dependencies {
      if dep != &node.id && known_ids.contains(dep.as_str()) && !connected_from.contains(dep) {
        connected_from.insert(dep.clone());
        edges.push(InspectedEdge {
          from: dep.clone(),
          to: node.id.clone(),
          kind: InspectedEdgeKind::Dependency,
          label: None,
        });
      }
    }
  }

  edges
}

fn compute_topology(
  nodes: &[NodeDefinitionV2],
  edges: &[InspectedEdge],
) -> (Vec<String>, BTreeMap<String, usize>, Option<String>) {
  let mut in_degree: BTreeMap<String, usize> = BTreeMap::new();
  let mut adj: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
  let mut level_map: BTreeMap<String, usize> = BTreeMap::new();

  for node in nodes {
    if !node.id.trim().is_empty() {
      in_degree.entry(node.id.clone()).or_insert(0);
      adj.entry(node.id.clone()).or_default();
      level_map.entry(node.id.clone()).or_insert(0);
    }
  }

  for edge in edges {
    if let Some(neighbors) = adj.get_mut(&edge.from)
      && neighbors.insert(edge.to.clone())
      && let Some(deg) = in_degree.get_mut(&edge.to)
    {
      *deg += 1;
    }
  }

  let mut queue: VecDeque<String> = in_degree
    .iter()
    .filter(|&(_, &deg)| deg == 0)
    .map(|(id, _)| id.clone())
    .collect();

  let mut order = Vec::new();
  while let Some(u) = queue.pop_front() {
    let u_level = level_map.get(&u).copied().unwrap_or(0);
    order.push(u.clone());
    if let Some(neighbors) = adj.get(&u) {
      for v in neighbors {
        let next_level = level_map.get(v).copied().unwrap_or(0).max(u_level + 1);
        level_map.insert(v.clone(), next_level);
        if let Some(deg) = in_degree.get_mut(v) {
          *deg = deg.saturating_sub(1);
          if *deg == 0 {
            queue.push_back(v.clone());
          }
        }
      }
    }
  }

  let cycle_issue = if order.len() != in_degree.len() {
    Some("workflow contains a circular dependency cycle".to_string())
  } else {
    None
  };

  (order, level_map, cycle_issue)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn inspects_inputs_edges_levels_and_map_subnodes() {
    let yaml = r#"
name: Tech Deck Studio Test
cost_limit_usd: 1.5
inputs:
  topic:
    description: Presentation topic
    required: true
  slide_count:
    description: Number of slides
    default: 6
  include_audio:
    default: true
nodes:
  - id: plan
    type: llm
    parameters:
      model: mock-llm
      prompt: "Plan {{ topic }}"
  - id: render_deck
    type: render_slides
    dependencies: [plan]
    input_mapping:
      deck: "{{ nodes.plan.outputs.response }}"
  - id: speak_each
    type: map
    dependencies: [render_deck]
    run_if: "include_audio == true"
    input_mapping:
      items: "{{ nodes.render_deck.outputs.slides }}"
    parameters:
      parallel: true
      template:
        - id: tts_slide
          type: tts
          parameters:
            model: mock-tts
            voice: alloy
            input_template: "{{ item.narration }}"
  - id: mux_video
    type: render_video
    run_if: "len(nodes.speak_each.outputs.results) > 0"
    dependencies: [render_deck]
    input_mapping:
      scenes: "{{ nodes.speak_each.outputs.results }}"
"#;

    let res = inspect_workflow(yaml);
    assert!(
      res.valid,
      "expected valid workflow, got {:?}",
      res.diagnostics
    );
    assert_eq!(res.name.as_deref(), Some("Tech Deck Studio Test"));
    assert_eq!(res.cost_limit_usd, Some(1.5));

    // Required input `topic` comes first, followed by alphabetical optional inputs.
    assert_eq!(res.inputs.len(), 3);
    assert_eq!(res.inputs[0].name, "topic");
    assert!(res.inputs[0].required);
    assert_eq!(res.inputs[0].kind, InspectedInputKind::String);
    assert_eq!(res.inputs[1].name, "include_audio");
    assert_eq!(res.inputs[1].kind, InspectedInputKind::Boolean);
    assert_eq!(res.inputs[2].name, "slide_count");
    assert_eq!(res.inputs[2].kind, InspectedInputKind::Number);

    assert_eq!(
      res.execution_order,
      vec!["plan", "render_deck", "speak_each", "mux_video"]
    );
    assert_eq!(
      res.levels,
      vec![
        vec!["plan"],
        vec!["render_deck"],
        vec!["speak_each"],
        vec!["mux_video"],
      ]
    );

    let map_node = res.nodes.iter().find(|n| n.id == "speak_each").unwrap();
    assert_eq!(map_node.sub_nodes.len(), 1);
    assert_eq!(map_node.sub_nodes[0].id, "tts_slide");
    assert!(map_node.parameters.get("template").is_none());

    assert!(res.edges.iter().any(|e| {
      e.from == "speak_each" && e.to == "mux_video" && e.kind == InspectedEdgeKind::InputMapping
    }));
  }

  #[test]
  fn returns_structured_diagnostics_on_syntax_or_cycle_errors() {
    let broken = inspect_workflow("name: [unterminated");
    assert!(!broken.valid);
    assert!(!broken.diagnostics.issues.is_empty());

    let cyclic = inspect_workflow(
      r#"
name: Cycle
nodes:
  - id: a
    type: template
    dependencies: [b]
    parameters:
      template: "x"
  - id: b
    type: template
    dependencies: [a]
    parameters:
      template: "y"
"#,
    );
    assert!(!cyclic.valid);
    assert!(
      cyclic
        .diagnostics
        .issues
        .iter()
        .any(|msg| msg.contains("circular dependency"))
    );
  }
}
