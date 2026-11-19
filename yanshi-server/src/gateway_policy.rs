//! Server-side ceiling on what a submitted workflow or harness session may
//! touch on the gateway host (eval 2026-10-07 S1/S2/S7).
//!
//! A workflow YAML carries its own sandbox settings (`allowed_commands`,
//! `allowed_paths`), and several node types run host processes or load
//! code from a path the YAML names (`mcp`'s `server_command`, `plugin`'s
//! `manifest`, `skill_agent`/`agent`/`multi_agent`'s skill dir). On the
//! CLI the author is the operator, so that's fine; on the gateway the
//! author is any token holder, so those settings must be capped by the
//! operator. Secure by default: with nothing configured, every
//! host-reaching node type is rejected and no path is allowed.
//!
//! Operator knobs (comma-separated):
//! - [`HOST_NODE_TYPES_ENV`] — host-reaching node types tenants may use.
//! - [`ALLOWED_COMMANDS_ENV`] — the most a `shell` node may allow.
//! - [`PATH_ROOTS_ENV`] — absolute roots every `allowed_paths` entry,
//!   harness `workspace_root`, `checkpoint_dir` and any node's
//!   path-naming parameter (Y7.3) must sit under.
//! - [`URL_HOSTS_ENV`] — hosts a node's endpoint parameter (`qdrant_url`,
//!   any `*_url`) may name (Y7.3). The `http` node's `url` is not capped
//!   here: its own SSRF filter runs on every request.

use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};

use yanshi_config::config::{
  schema::node_accepts_parameter,
  v2::{FlowDefinitionV2, HOST_NODE_TYPES, NodeDefinitionV2, walk_nodes},
};

pub const HOST_NODE_TYPES_ENV: &str = "YANSHI_GATEWAY_HOST_NODE_TYPES";
pub const ALLOWED_COMMANDS_ENV: &str = "YANSHI_GATEWAY_ALLOWED_COMMANDS";
pub const PATH_ROOTS_ENV: &str = "YANSHI_GATEWAY_PATH_ROOTS";
pub const URL_HOSTS_ENV: &str = "YANSHI_GATEWAY_URL_HOSTS";

#[derive(Debug, Clone, Default)]
pub struct GatewayPolicy {
  pub host_node_types: HashSet<String>,
  pub allowed_commands: HashSet<String>,
  pub path_roots: Vec<PathBuf>,
  /// Y7.3: hosts endpoint parameters may name (exact match).
  pub url_hosts: HashSet<String>,
}

impl GatewayPolicy {
  /// Read the operator ceiling from the environment. Fails on a relative
  /// root or an unknown node type so a typo can't silently widen or
  /// narrow the policy.
  pub fn from_env() -> Result<Self, String> {
    let list = |key: &str| -> Vec<String> {
      std::env::var(key)
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
    };
    let host_node_types: HashSet<String> = list(HOST_NODE_TYPES_ENV).into_iter().collect();
    if let Some(unknown) = host_node_types
      .iter()
      .find(|kind| !HOST_NODE_TYPES.contains(&kind.as_str()))
    {
      return Err(format!(
        "{HOST_NODE_TYPES_ENV}: '{unknown}' is not a host-reaching node type (expected any of {HOST_NODE_TYPES:?})"
      ));
    }
    let path_roots: Vec<PathBuf> = list(PATH_ROOTS_ENV)
      .into_iter()
      .map(PathBuf::from)
      .collect();
    if let Some(bad) = path_roots.iter().find(|root| !is_plain_absolute(root)) {
      return Err(format!(
        "{PATH_ROOTS_ENV}: '{}' must be an absolute path without '..'",
        bad.display()
      ));
    }
    Ok(Self {
      host_node_types,
      allowed_commands: list(ALLOWED_COMMANDS_ENV).into_iter().collect(),
      path_roots,
      url_hosts: list(URL_HOSTS_ENV)
        .into_iter()
        .map(|host| host.to_ascii_lowercase())
        .collect(),
    })
  }

  /// Reject a workflow that reaches past the operator ceiling.
  pub fn check_workflow(&self, def: &FlowDefinitionV2) -> Result<(), String> {
    walk_nodes(&def.nodes, &mut |node| self.check_node(def, node))?;
    let mut defaults = serde_json::Map::new();
    for (key, spec) in &def.inputs {
      if let Some(default_val) = &spec.default
        && let Ok(json_val) = serde_json::to_value(default_val)
      {
        defaults.insert(key.clone(), json_val);
      }
    }
    if !defaults.is_empty() {
      self.check_run_inputs(def, &defaults)?;
    }
    Ok(())
  }

  /// Enforce the same host-path and endpoint ceilings on caller-supplied
  /// workflow `inputs` (`POST /v1/runs`, `POST /v1/notebooks/{id}/generate`).
  ///
  /// Workflow inputs broadcast to every node unless shadowed by `input_mapping`,
  /// so a path- or endpoint-reaching key (`save_to_file`, `qdrant_url`,
  /// `audio_source`, `image_source`, ...) accepted by any node in `def` must
  /// satisfy that node's reach ceiling.
  pub fn check_run_inputs(
    &self,
    def: &FlowDefinitionV2,
    inputs: &serde_json::Map<String, serde_json::Value>,
  ) -> Result<(), String> {
    if inputs.is_empty() {
      return Ok(());
    }
    walk_nodes(&def.nodes, &mut |node| {
      for (key, value) in inputs {
        if value.is_null() || node.input_mapping.contains_key(key) {
          continue;
        }
        if !node_accepts_parameter(&node.node_type, key) {
          continue;
        }
        let Some(reach) = reach_of(&node.node_type, key) else {
          continue;
        };
        let what = format!("input '{key}' (node '{}')", node.id);
        self.check_json_reach(&what, reach, value)?;
      }
      // Also enforce reach ceilings on indirect `inputs` keys referenced by a
      // node parameter (`parameters: { audio_source: target }` with `inputs.target`).
      for (param_key, param_val) in &node.parameters {
        let Some(reach) = reach_of(&node.node_type, param_key) else {
          continue;
        };
        if let Some(ref_key) = param_val.as_str().map(str::trim)
          && is_identifier_key(ref_key)
          && !node.input_mapping.contains_key(ref_key)
          && let Some(input_val) = inputs.get(ref_key)
          && !input_val.is_null()
        {
          let what = format!(
            "input '{ref_key}' (referenced by node '{}' '{param_key}')",
            node.id
          );
          self.check_json_reach(&what, reach, input_val)?;
        } else if reach == Reach::MediaSourceList
          && let Some(seq) = param_val.as_sequence()
        {
          for item in seq {
            if let Some(ref_key) = item.as_str().map(str::trim)
              && is_identifier_key(ref_key)
              && !node.input_mapping.contains_key(ref_key)
              && let Some(input_val) = inputs.get(ref_key)
              && !input_val.is_null()
            {
              let what = format!(
                "input '{ref_key}' (referenced by node '{}' '{param_key}')",
                node.id
              );
              self.check_json_reach(&what, Reach::MediaSource, input_val)?;
            }
          }
        }
      }
      Ok(())
    })
  }

  /// `what` names the field in the error message (e.g. `workspace_root`).
  pub fn check_path(&self, what: &str, path: &str) -> Result<(), String> {
    let path = Path::new(path);
    if !is_plain_absolute(path) {
      return Err(format!(
        "{what} '{}' must be an absolute path without '..'",
        path.display()
      ));
    }
    if self.path_roots.iter().any(|root| path.starts_with(root)) {
      return Ok(());
    }
    if self.path_roots.is_empty() {
      return Err(format!(
        "{what} '{}' is not allowed: this gateway permits no host paths (operator sets {PATH_ROOTS_ENV})",
        path.display()
      ));
    }
    Err(format!(
      "{what} '{}' is outside the gateway's allowed roots {:?}",
      path.display(),
      self.path_roots
    ))
  }

  fn check_node(&self, def: &FlowDefinitionV2, node: &NodeDefinitionV2) -> Result<(), String> {
    let kind = node.node_type.as_str();
    let id = &node.id;
    let is_safe_inline_agent = (kind == "agent" && is_gateway_safe_inline_agent(node))
      || (kind == "multi_agent" && is_gateway_safe_inline_multi_agent(node));
    if HOST_NODE_TYPES.contains(&kind)
      && !is_safe_inline_agent
      && !self.host_node_types.contains(kind)
    {
      return Err(format!(
        "node '{id}': type '{kind}' is disabled on this gateway (operator enables it via {HOST_NODE_TYPES_ENV})"
      ));
    }
    self.check_reach_parameters(def, node)?;
    match kind {
      "shell" => {
        for command in string_list(node, "allowed_commands") {
          if !self.allowed_commands.contains(&command) {
            return Err(format!(
              "node '{id}': command '{command}' exceeds the gateway ceiling (operator sets {ALLOWED_COMMANDS_ENV})"
            ));
          }
        }
        // Without allowed_paths the shell sandbox falls back to the
        // gateway's own cwd, which is not the tenant's to choose.
        let paths = string_list(node, "allowed_paths");
        if paths.is_empty() {
          return Err(format!(
            "node '{id}': shell nodes on the gateway must declare 'allowed_paths'"
          ));
        }
        for path in paths {
          self.check_path(&format!("node '{id}' allowed_paths"), &path)?;
        }
      }
      "file" | "repo_snapshot" | "pdf_extract" | "video_ingest" => {
        for path in string_list(node, "allowed_paths") {
          self.check_path(&format!("node '{id}' allowed_paths"), &path)?;
        }
      }
      _ => {}
    }
    Ok(())
  }
}

/// What a parameter reaches, judged by its name (Y7.3 / Z1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
  Path,
  Url,
  MediaSource,
  MediaSourceList,
}

fn reach_of(node_type: &str, key: &str) -> Option<Reach> {
  match key {
    // Lists checked per node type above.
    "allowed_paths" | "allowed_commands" => None,
    // `http` filters its target at run time; `arxiv` builds arxiv.org
    // URLs from the paper id it parses out of `url`.
    "url"
      if matches!(
        node_type,
        "http" | "arxiv" | "repo_snapshot" | "pdf_extract" | "video_ingest"
      ) =>
    {
      None
    }
    "audio_source" | "image_source" | "source_image" | "reference_image" => {
      Some(Reach::MediaSource)
    }
    "source" if matches!(node_type, "pdf_extract" | "video_ingest" | "repo_snapshot") => {
      Some(Reach::MediaSource)
    }
    "image" if node_type == "text_to_video" => Some(Reach::MediaSource),
    "reference_images" => Some(Reach::MediaSourceList),
    "url" | "endpoint" | "base_url" => Some(Reach::Url),
    "path" | "save_to_file" => Some(Reach::Path),
    _ if key.ends_with("_url") || key.ends_with("_endpoint") => Some(Reach::Url),
    _ if key.ends_with("_path") || key.ends_with("_dir") || key.ends_with("_file") => {
      Some(Reach::Path)
    }
    _ => None,
  }
}

fn is_identifier_key(text: &str) -> bool {
  !text.is_empty()
    && text
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl GatewayPolicy {
  /// Y7.3 / Z1: every node parameter that names a host path, a media source,
  /// or an endpoint is held to the same ceiling as `allowed_paths`.
  fn check_reach_parameters(
    &self,
    def: &FlowDefinitionV2,
    node: &NodeDefinitionV2,
  ) -> Result<(), String> {
    let id = &node.id;
    for (key, value) in &node.parameters {
      let Some(reach) = reach_of(&node.node_type, key) else {
        continue;
      };
      let what = format!("node '{id}' {key}");
      match reach {
        Reach::Path => {
          let Some(text) = value.as_str() else {
            return Err(format!("{what}: must be a plain string on the gateway"));
          };
          self.check_path(&what, text)?;
        }
        Reach::Url => {
          let Some(text) = value.as_str() else {
            return Err(format!("{what}: must be a plain string on the gateway"));
          };
          self.check_url(&what, text)?;
        }
        Reach::MediaSource => {
          let Some(text) = value.as_str() else {
            return Err(format!("{what}: must be a plain string on the gateway"));
          };
          self.check_media_source_param(&what, text, Some((def, node)))?;
        }
        Reach::MediaSourceList => {
          let Some(seq) = value.as_sequence() else {
            return Err(format!("{what}: must be a list of strings on the gateway"));
          };
          for (idx, item) in seq.iter().enumerate() {
            let item_what = format!("{what}[{idx}]");
            let Some(text) = item.as_str() else {
              return Err(format!(
                "{item_what}: must be a plain string on the gateway"
              ));
            };
            self.check_media_source_param(&item_what, text, Some((def, node)))?;
          }
        }
      }
    }
    Ok(())
  }

  fn check_json_reach(
    &self,
    what: &str,
    reach: Reach,
    value: &serde_json::Value,
  ) -> Result<(), String> {
    match reach {
      Reach::Path => {
        let Some(text) = value.as_str() else {
          return Err(format!("{what}: must be a plain string on the gateway"));
        };
        if text.trim().is_empty() {
          return Ok(());
        }
        self.check_path(what, text)
      }
      Reach::Url => {
        let Some(text) = value.as_str() else {
          return Err(format!("{what}: must be a plain string on the gateway"));
        };
        if text.trim().is_empty() {
          return Ok(());
        }
        self.check_url(what, text)
      }
      Reach::MediaSource => match value {
        serde_json::Value::String(text) => self.check_media_source_param(what, text, None),
        serde_json::Value::Object(obj) => {
          if let Some(path_val) = obj.get("path") {
            let Some(path_str) = path_val.as_str() else {
              return Err(format!(
                "{what}.path: must be a plain string on the gateway"
              ));
            };
            if !path_str.trim().is_empty() {
              self.check_path(&format!("{what}.path"), path_str.trim())?;
            }
            return Ok(());
          }
          if let Some(url_val) = obj.get("url") {
            let Some(url_str) = url_val.as_str() else {
              return Err(format!("{what}.url: must be a plain string on the gateway"));
            };
            return self.check_media_source_param(&format!("{what}.url"), url_str, None);
          }
          Err(format!(
            "{what}: media source object must specify 'path' or 'url'"
          ))
        }
        _ => Err(format!("{what}: must be a string or media source object")),
      },
      Reach::MediaSourceList => {
        let Some(seq) = value.as_array() else {
          return Err(format!("{what}: must be a list of media sources"));
        };
        for (idx, item) in seq.iter().enumerate() {
          self.check_json_reach(&format!("{what}[{idx}]"), Reach::MediaSource, item)?;
        }
        Ok(())
      }
    }
  }

  /// Validate a single media-source string (`data:` URI, SSRF-filtered
  /// `http(s)://` URL, `{{ ... }}` node-output placeholder, indirect input
  /// key, or a host path subject to `path_roots`).
  fn check_media_source_param(
    &self,
    what: &str,
    raw: &str,
    yaml_ctx: Option<(&FlowDefinitionV2, &NodeDefinitionV2)>,
  ) -> Result<(), String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.starts_with("data:") {
      return Ok(());
    }
    if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
      url::Url::parse(trimmed).map_err(|e| format!("{what} '{trimmed}' is not a URL: {e}"))?;
      return Ok(());
    }
    if trimmed.starts_with("github.com/") {
      url::Url::parse(&format!("https://{trimmed}"))
        .map_err(|e| format!("{what} '{trimmed}' is not a valid GitHub URL: {e}"))?;
      return Ok(());
    }
    if let Some((def, node)) = yaml_ctx {
      if trimmed.contains("{{") && trimmed.contains("}}") {
        return Ok(());
      }
      if is_identifier_key(trimmed)
        && (node.input_mapping.contains_key(trimmed) || def.inputs.contains_key(trimmed))
      {
        return Ok(());
      }
    }
    self.check_path(what, trimmed)
  }

  /// `what` names the field in the error message.
  pub fn check_url(&self, what: &str, text: &str) -> Result<(), String> {
    let parsed = url::Url::parse(text).map_err(|e| format!("{what} '{text}' is not a URL: {e}"))?;
    let host = parsed
      .host_str()
      .map(str::to_ascii_lowercase)
      .ok_or_else(|| format!("{what} '{text}' names no host"))?;
    if self.url_hosts.contains(&host) {
      return Ok(());
    }
    Err(format!(
      "{what} '{text}': host '{host}' is not allowed on this gateway (operator lists endpoints in {URL_HOSTS_ENV})"
    ))
  }
}

fn string_list(node: &NodeDefinitionV2, key: &str) -> Vec<String> {
  node
    .parameters
    .get(key)
    .and_then(|value| value.as_sequence())
    .map(|seq| {
      seq
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect()
    })
    .unwrap_or_default()
}

/// Containment is checked lexically, so `..` must be refused outright —
/// `/srv/work/../../etc` "starts with" `/srv/work`.
fn is_plain_absolute(path: &Path) -> bool {
  path.is_absolute() && !path.components().any(|c| c == Component::ParentDir)
}

/// An inline `type: agent` node (no host `skill` directory, restricted to
/// SSRF-filtered built-in `web_search` / `http` tools) does not access host
/// paths or spawn host processes, so tenants may run it under the default
/// gateway policy without enabling skill-directory `agent` nodes.
fn is_gateway_safe_inline_agent(node: &NodeDefinitionV2) -> bool {
  if node.parameters.contains_key("skill") || node.input_mapping.contains_key("skill") {
    return false;
  }
  let has_prompt = node.parameters.contains_key("prompt")
    || node.input_mapping.contains_key("prompt")
    || node.parameters.contains_key("message")
    || node.input_mapping.contains_key("message");
  if !has_prompt || node.input_mapping.contains_key("tools") {
    return false;
  }
  match node.parameters.get("tools") {
    None => true,
    Some(value) => value.as_sequence().is_some_and(|seq| {
      seq
        .iter()
        .all(|entry| matches!(entry.as_str(), Some("web_search" | "http")))
    }),
  }
}

/// An inline `type: multi_agent` node whose participants all declare inline
/// `system` prompts (zero host `skill` paths) and restrict optional `tools` to
/// SSRF-filtered `web_search` / `http` is pure in-memory coordination and safe
/// under the default gateway policy.
fn is_gateway_safe_inline_multi_agent(node: &NodeDefinitionV2) -> bool {
  for blocked in ["mode", "agents", "participants", "judge"] {
    if node.input_mapping.contains_key(blocked) {
      return false;
    }
  }
  let has_message =
    node.parameters.contains_key("message") || node.input_mapping.contains_key("message");
  if !has_message {
    return false;
  }

  let is_safe_spec = |val: &serde_yaml::Value| -> bool {
    let Some(map) = val.as_mapping() else {
      return false;
    };
    if map.contains_key("skill") {
      return false;
    }
    let has_name = map
      .get("name")
      .and_then(|v| v.as_str())
      .is_some_and(|s| !s.trim().is_empty());
    let has_system = map
      .get("system")
      .and_then(|v| v.as_str())
      .is_some_and(|s| !s.trim().is_empty());
    if !has_name || !has_system {
      return false;
    }
    match map.get("tools") {
      None => true,
      Some(tools) => tools.as_sequence().is_some_and(|seq| {
        seq
          .iter()
          .all(|entry| matches!(entry.as_str(), Some("web_search" | "http")))
      }),
    }
  };

  match node.parameters.get("mode").and_then(|v| v.as_str()) {
    Some("handoff" | "blackboard") => {
      if node.parameters.contains_key("participants") || node.parameters.contains_key("judge") {
        return false;
      }
      node
        .parameters
        .get("agents")
        .and_then(|v| v.as_sequence())
        .is_some_and(|seq| !seq.is_empty() && seq.iter().all(is_safe_spec))
    }
    Some("debate") => {
      if node.parameters.contains_key("agents") {
        return false;
      }
      let ok_participants = node
        .parameters
        .get("participants")
        .and_then(|v| v.as_sequence())
        .is_some_and(|seq| !seq.is_empty() && seq.iter().all(is_safe_spec));
      let ok_judge = node.parameters.get("judge").is_some_and(is_safe_spec);
      ok_participants && ok_judge
    }
    _ => false,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn workflow(yaml: &str) -> FlowDefinitionV2 {
    serde_yaml::from_str(yaml).unwrap()
  }

  fn operator_policy() -> GatewayPolicy {
    GatewayPolicy {
      host_node_types: ["shell", "file"].iter().map(|s| s.to_string()).collect(),
      allowed_commands: ["git", "ls"].iter().map(|s| s.to_string()).collect(),
      path_roots: vec![PathBuf::from("/srv/work")],
      url_hosts: HashSet::new(),
    }
  }

  #[test]
  fn default_policy_rejects_every_host_node_type() {
    for kind in HOST_NODE_TYPES {
      let def = workflow(&format!("name: w\nnodes:\n  - id: n\n    type: {kind}\n"));
      let err = GatewayPolicy::default().check_workflow(&def).unwrap_err();
      assert!(err.contains(HOST_NODE_TYPES_ENV), "{kind}: {err}");
    }
    let llm = workflow("name: w\nnodes:\n  - id: n\n    type: llm\n");
    assert!(GatewayPolicy::default().check_workflow(&llm).is_ok());
  }

  #[test]
  fn shell_commands_and_paths_are_capped() {
    let policy = operator_policy();
    let ok = workflow(
      "name: w\nnodes:\n  - id: n\n    type: shell\n    parameters:\n      allowed_commands: [git]\n      allowed_paths: [/srv/work/repo]\n",
    );
    assert!(policy.check_workflow(&ok).is_ok());
    for bad in [
      "allowed_commands: [python3]\n      allowed_paths: [/srv/work]",
      "allowed_commands: [git]\n      allowed_paths: [/]",
      "allowed_commands: [git]\n      allowed_paths: [/srv/work/../../proc/self]",
      "allowed_commands: [git]\n      allowed_paths: [srv/work]",
      "allowed_commands: [git]",
    ] {
      let def = workflow(&format!(
        "name: w\nnodes:\n  - id: n\n    type: shell\n    parameters:\n      {bad}\n"
      ));
      assert!(policy.check_workflow(&def).is_err(), "{bad}");
    }
  }

  #[test]
  fn nested_map_and_while_nodes_are_checked() {
    let def = workflow(
      "name: w\nnodes:\n  - id: m\n    type: map\n    parameters:\n      template:\n        - id: inner\n          type: while\n          parameters:\n            do:\n              - id: leaf\n                type: mcp\n",
    );
    let err = operator_policy().check_workflow(&def).unwrap_err();
    assert!(err.contains("'leaf'"), "{err}");
  }

  #[test]
  fn path_roots_bound_workspace_root() {
    let policy = operator_policy();
    assert!(policy.check_path("workspace_root", "/srv/work/t1").is_ok());
    assert!(policy.check_path("workspace_root", "/").is_err());
    assert!(
      policy
        .check_path("workspace_root", "/srv/workshop")
        .is_err()
    );
    assert!(
      GatewayPolicy::default()
        .check_path("workspace_root", "/srv/work")
        .is_err()
    );
  }

  /// Y7.3 (SEC-2/3): any node parameter that names a host path or a
  /// network endpoint is capped, not only on the host-reaching types.
  #[test]
  fn path_and_url_parameters_are_capped_on_every_node() {
    let mut policy = operator_policy();
    let markmap = |path: &str| {
      workflow(&format!(
        "name: w\nnodes:\n  - id: m\n    type: markmap\n    parameters:\n      save_to_file: {path}\n"
      ))
    };
    assert!(
      policy
        .check_workflow(&markmap("/srv/work/map.html"))
        .is_ok()
    );
    for bad in ["/etc/cron.d/x", "relative.html", "/srv/work/../../etc/x"] {
      let err = policy.check_workflow(&markmap(bad)).unwrap_err();
      assert!(err.contains("save_to_file"), "{bad}: {err}");
    }
    let rag = |url: &str| {
      workflow(&format!(
        "name: w\nnodes:\n  - id: r\n    type: rag\n    parameters:\n      operation: search\n      qdrant_url: {url}\n"
      ))
    };
    let err = policy
      .check_workflow(&rag("http://169.254.169.254:6334"))
      .unwrap_err();
    assert!(
      err.contains("qdrant_url") && err.contains(URL_HOSTS_ENV),
      "{err}"
    );
    policy.url_hosts = ["qdrant.internal".to_string()].into_iter().collect();
    assert!(
      policy
        .check_workflow(&rag("http://qdrant.internal:6334"))
        .is_ok()
    );
    assert!(
      policy
        .check_workflow(&rag("http://qdrant.internal.evil.com:6334"))
        .is_err()
    );
    assert!(policy.check_workflow(&rag("not a url")).is_err());
    // `http` applies its own SSRF filter at run time; its `url` stays free.
    let http = workflow(
      "name: w\nnodes:\n  - id: h\n    type: http\n    parameters:\n      url: https://example.org\n",
    );
    assert!(GatewayPolicy::default().check_workflow(&http).is_ok());

    // Inline `agent` (no `skill`, safe built-in tools only) is permitted by default,
    // whereas skill-backed `agent` or unsupported tools remain host-gated.
    let inline_agent = workflow(
      "name: w\nnodes:\n  - id: clarify\n    type: agent\n    parameters:\n      prompt: Plan backend\n      tools: [web_search, http]\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&inline_agent)
        .is_ok()
    );
    let skill_backed_agent = workflow(
      "name: w\nnodes:\n  - id: clarify\n    type: agent\n    parameters:\n      skill: /srv/skills/planner\n      message: Plan backend\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&skill_backed_agent)
        .is_err()
    );
    let shell_tool_agent = workflow(
      "name: w\nnodes:\n  - id: clarify\n    type: agent\n    parameters:\n      prompt: Plan backend\n      tools: [shell]\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&shell_tool_agent)
        .is_err()
    );

    // Inline `multi_agent` (no `skill`, safe built-in tools only) is permitted,
    // whereas skill-backed or unsafe-tool `multi_agent` remains host-gated.
    let inline_multi_agent = workflow(
      "name: w\nnodes:\n  - id: debate\n    type: multi_agent\n    parameters:\n      mode: debate\n      message: Motion\n      participants:\n        - name: pro\n          system: Argue pro\n          tools: [web_search]\n        - name: con\n          system: Argue con\n      judge:\n        name: mod_host\n        system: Judge fairly\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&inline_multi_agent)
        .is_ok()
    );
    let skill_multi_agent = workflow(
      "name: w\nnodes:\n  - id: relay\n    type: multi_agent\n    parameters:\n      mode: handoff\n      message: Build lore\n      agents:\n        - name: lore\n          skill: examples/skills/worldbuilders/lore-architect\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&skill_multi_agent)
        .is_err()
    );
    let shell_multi_agent = workflow(
      "name: w\nnodes:\n  - id: room\n    type: multi_agent\n    parameters:\n      mode: blackboard\n      message: Draft script\n      agents:\n        - name: writer\n          system: Write script\n          tools: [shell]\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&shell_multi_agent)
        .is_err()
    );
  }

  #[test]
  fn modality_media_sources_are_capped_while_data_uris_urls_and_mappings_pass() {
    let policy = operator_policy();
    for (kind, field) in [
      ("asr", "audio_source"),
      ("image_understand", "image_source"),
      ("image_understand", "reference_image"),
      ("image_edit", "image_source"),
      ("image_to_image", "source_image"),
      ("text_to_video", "image"),
    ] {
      let host_escape = workflow(&format!(
        "name: w\nnodes:\n  - id: m\n    type: {kind}\n    parameters:\n      {field}: /etc/hosts\n"
      ));
      let err = GatewayPolicy::default()
        .check_workflow(&host_escape)
        .expect_err("default gateway policy must reject host file media source");
      assert!(err.contains(field), "{kind}.{field}: {err}");

      let allowed_host = workflow(&format!(
        "name: w\nnodes:\n  - id: m\n    type: {kind}\n    parameters:\n      {field}: /srv/work/sample.bin\n"
      ));
      assert!(
        policy.check_workflow(&allowed_host).is_ok(),
        "{kind}.{field} under allowed path_roots should pass"
      );

      for safe in [
        "data:image/png;base64,iVBORw0KGgo=",
        "https://cdn.example.org/asset.png",
        "\"{{ nodes.upstream.outputs.file }}\"",
      ] {
        let ok_def = workflow(&format!(
          "name: w\nnodes:\n  - id: m\n    type: {kind}\n    parameters:\n      {field}: {safe}\n"
        ));
        assert!(
          GatewayPolicy::default().check_workflow(&ok_def).is_ok(),
          "{kind}.{field} = {safe} should pass default policy"
        );
      }
    }

    let indirect_mapping = workflow(
      "name: w\nnodes:\n  - id: m\n    type: image_understand\n    parameters:\n      image_source: picture\n    input_mapping:\n      picture: \"{{ nodes.draw.outputs.image }}\"\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&indirect_mapping)
        .is_ok()
    );

    let bad_refs = workflow(
      "name: w\nnodes:\n  - id: m\n    type: image_to_image\n    parameters:\n      reference_images: [\"https://cdn.example.org/ok.png\", \"/etc/hosts\"]\n",
    );
    assert!(GatewayPolicy::default().check_workflow(&bad_refs).is_err());

    // Declared workflow input defaults cannot smuggle host paths either.
    let sneaky_default = workflow(
      "name: w\ninputs:\n  audio_source:\n    default: /etc/passwd\nnodes:\n  - id: t\n    type: asr\n    parameters:\n      model: mock-asr\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&sneaky_default)
        .is_err()
    );

    // Runtime `CreateRunRequest.inputs` are held to the same ceiling, while
    // SSRF-filtered `http.url` broadcast across `llm`/`template` nodes passes.
    let asr_def = workflow(
      "name: w\ninputs:\n  audio_source:\n    required: true\nnodes:\n  - id: t\n    type: asr\n    parameters:\n      model: mock-asr\n",
    );
    assert!(GatewayPolicy::default().check_workflow(&asr_def).is_ok());
    let mut bad_inputs = serde_json::Map::new();
    bad_inputs.insert("audio_source".into(), serde_json::json!("/etc/passwd"));
    assert!(
      GatewayPolicy::default()
        .check_run_inputs(&asr_def, &bad_inputs)
        .is_err()
    );
    assert!(policy.check_run_inputs(&asr_def, &bad_inputs).is_err());

    let mut ok_inputs = serde_json::Map::new();
    ok_inputs.insert(
      "audio_source".into(),
      serde_json::json!("/srv/work/standup.mp3"),
    );
    assert!(policy.check_run_inputs(&asr_def, &ok_inputs).is_ok());
    ok_inputs.insert(
      "audio_source".into(),
      serde_json::json!("https://cdn.example.org/standup.mp3"),
    );
    assert!(
      GatewayPolicy::default()
        .check_run_inputs(&asr_def, &ok_inputs)
        .is_ok()
    );

    let report_def = workflow(include_str!("../../examples/generators/report.yml"));
    let mut report_inputs = serde_json::Map::new();
    report_inputs.insert("url".into(), serde_json::json!("https://example.org/post"));
    assert!(
      GatewayPolicy::default()
        .check_run_inputs(&report_def, &report_inputs)
        .is_ok()
    );

    // Indirect workflow inputs (`parameters: { audio_source: target }`) must also
    // obey `Reach::MediaSource` so host paths cannot bypass `check_run_inputs`.
    let indirect_asr = workflow(
      "name: w\ninputs:\n  target:\n    required: true\nnodes:\n  - id: t\n    type: asr\n    parameters:\n      model: mock-asr\n      audio_source: target\n",
    );
    assert!(
      GatewayPolicy::default()
        .check_workflow(&indirect_asr)
        .is_ok()
    );
    let mut sneaky_indirect = serde_json::Map::new();
    sneaky_indirect.insert("target".into(), serde_json::json!("/etc/hosts"));
    assert!(
      GatewayPolicy::default()
        .check_run_inputs(&indirect_asr, &sneaky_indirect)
        .is_err()
    );
    sneaky_indirect.insert("target".into(), serde_json::json!("/srv/work/clip.mp3"));
    assert!(
      policy
        .check_run_inputs(&indirect_asr, &sneaky_indirect)
        .is_ok()
    );
  }

  #[test]
  fn all_official_generator_workflows_validate_build_and_pass_default_gateway_policy() {
    let generators_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
      .parent()
      .expect("workspace root")
      .join("examples/generators");
    let strict_suite = [
      "illustrated_slides.yml",
      "repo_architecture.yml",
      "backend_project_planner.yml",
      "paper_reader.yml",
      "video_summarizer.yml",
      "episodic_drama_studio.yml",
      "debate_podcast_show.yml",
      "worldbuilding_bible.yml",
    ];
    let policy = GatewayPolicy::default();
    let mut count = 0usize;

    for entry in std::fs::read_dir(&generators_dir).expect("read examples/generators") {
      let path = entry.expect("dir entry").path();
      if path.extension().and_then(|s| s.to_str()) != Some("yml") {
        continue;
      }
      count += 1;
      let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or_default();
      let yaml = std::fs::read_to_string(&path).expect("read generator yaml");
      let def: FlowDefinitionV2 = serde_yaml::from_str(&yaml)
        .unwrap_or_else(|e| panic!("{file_name} failed YAML parse: {e}"));
      let report = yanshi_config::config::schema::validate_flow_definition(&def);
      assert!(
        report.issues.is_empty(),
        "{file_name} has schema issues: {:?}",
        report.issues
      );
      if strict_suite.contains(&file_name) {
        assert!(
          report.warnings.is_empty(),
          "{file_name} has schema warnings: {:?}",
          report.warnings
        );
      }
      yanshi_config::executor::build_flow_from_definition(&def, Some("gemini-2.5-flash"))
        .unwrap_or_else(|e| panic!("{file_name} failed to build runnable Flow: {e}"));
      policy
        .check_workflow(&def)
        .unwrap_or_else(|e| panic!("{file_name} rejected by GatewayPolicy::default(): {e}"));
    }

    assert!(count >= 18, "expected at least 18 generator workflows");
  }
}
