//! Content Studio generator catalogue, planner schema/sanitization, batch cost
//! limit splitting, and BM25 + round-robin chunk selection.

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use yanshi_rag::{Bm25KnowledgeBackend, KnowledgeBackend};

use crate::qa::{extract_json_object, truncate_chars};

/// The source-grounded generator catalogue (`RFC_CONTENT_STUDIO` §3 + Studio
/// flagship workflows), compiled in so both server and desktop serve the exact
/// same templates as `examples/generators/`.
pub const GENERATORS: &[(&str, &str)] = &[
  (
    "report",
    include_str!("../../examples/generators/report.yml"),
  ),
  (
    "mindmap",
    include_str!("../../examples/generators/mindmap.yml"),
  ),
  (
    "audio_overview",
    include_str!("../../examples/generators/audio_overview.yml"),
  ),
  (
    "slides",
    include_str!("../../examples/generators/slides.yml"),
  ),
  (
    "flashcards",
    include_str!("../../examples/generators/flashcards.yml"),
  ),
  ("quiz", include_str!("../../examples/generators/quiz.yml")),
  (
    "infographic",
    include_str!("../../examples/generators/infographic.yml"),
  ),
  (
    "data_table",
    include_str!("../../examples/generators/data_table.yml"),
  ),
  (
    "video_overview",
    include_str!("../../examples/generators/video_overview.yml"),
  ),
  (
    "illustrated_slides",
    include_str!("../../examples/generators/illustrated_slides.yml"),
  ),
  (
    "tech_slides_video",
    include_str!("../../examples/generators/tech_slides_video.yml"),
  ),
  (
    "debate_podcast_show",
    include_str!("../../examples/generators/debate_podcast_show.yml"),
  ),
];

/// How much notebook text a generator receives, in characters.
pub const GENERATE_CHUNK_BUDGET: usize = 24_000;
/// Maximum number of generator runs a single `generate-batch` or `plan` call may contain.
pub const MAX_BATCH_GENERATE_ITEMS: usize = 6;
/// Default maximum deliverables suggested by `POST /v1/notebooks/{id}/plan`.
pub const DEFAULT_PLAN_MAX_ITEMS: usize = 4;

/// Request payload for `POST /v1/notebooks/{id}/generate`.
#[derive(Debug, Deserialize)]
pub struct GenerateRequest {
  pub kind: String,
  /// Values for the generator's declared inputs (`focus`, `sections`,
  /// `pdf`, ...). `chunks`, `url` and `text` are set from the notebook.
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
  /// Optional subset of notebook source IDs to include in this generation run.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  /// Model for the generator's llm steps; falls back to `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional USD spend cap for this generation run.
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
}

/// One generator item in `POST /v1/notebooks/{id}/generate-batch`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatchGenerateItem {
  pub kind: String,
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
}

/// Request payload for `POST /v1/notebooks/{id}/generate-batch`.
#[derive(Debug, Deserialize)]
pub struct GenerateBatchRequest {
  pub items: Vec<BatchGenerateItem>,
  /// Optional subset of notebook source IDs to include across all items in the batch.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  /// Model for the generators' llm steps; falls back to `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional aggregate USD spend cap across the batch, split evenly across runs.
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
}

/// Dispatched run summary entry returned by `generate-batch`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DispatchedNotebookRun {
  pub run_id: Uuid,
  pub kind: String,
  pub status: String,
}

/// Response payload for `POST /v1/notebooks/{id}/generate-batch`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateBatchResponse {
  pub notebook_id: Uuid,
  pub runs: Vec<DispatchedNotebookRun>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub per_run_cost_limit_usd: Option<f64>,
}

/// Request payload for `POST /v1/notebooks/{id}/plan`.
#[derive(Debug, Deserialize)]
pub struct PlanNotebookRequest {
  pub goal: String,
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  #[serde(default)]
  pub model: Option<String>,
  #[serde(default)]
  pub max_items: Option<usize>,
}

/// One planned Studio deliverable suggestion returned by `/plan`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedGeneratorItem {
  pub kind: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub reason: Option<String>,
  #[serde(default)]
  pub params: serde_json::Map<String, Value>,
}

/// Response payload for `POST /v1/notebooks/{id}/plan`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanNotebookResponse {
  pub notebook_id: Uuid,
  pub goal: String,
  pub model: String,
  pub rationale: String,
  pub items: Vec<PlannedGeneratorItem>,
  pub used_chunk_count: usize,
}

/// Look up a compiled-in generator workflow YAML template by `kind`.
pub fn find_generator_template(kind: &str) -> Result<&'static str, String> {
  GENERATORS
    .iter()
    .find(|(k, _)| *k == kind)
    .map(|(_, tpl)| *tpl)
    .ok_or_else(|| {
      format!(
        "unknown kind '{kind}': expected one of {}",
        GENERATORS
          .iter()
          .map(|(k, _)| *k)
          .collect::<Vec<_>>()
          .join(", ")
      )
    })
}

/// Validate an aggregate batch USD spend cap and divide it evenly across `item_count` runs.
pub fn split_batch_cost_limit(
  total_limit_usd: Option<f64>,
  item_count: usize,
) -> Result<Option<f64>, String> {
  let Some(limit) = total_limit_usd else {
    return Ok(None);
  };
  let validated = yanshi_config::config::v2::validate_cost_limit(limit)?;
  let count = item_count.max(1) as f64;
  let per_run = validated / count;
  yanshi_config::config::v2::validate_cost_limit(per_run)?;
  Ok(Some(per_run))
}

/// Strict JSON Schema for `POST /v1/notebooks/{id}/plan`.
pub fn studio_plan_schema() -> Value {
  let kinds: Vec<Value> = GENERATORS
    .iter()
    .map(|(k, _)| Value::String((*k).to_string()))
    .collect();
  json!({
    "type": "object",
    "additionalProperties": false,
    "required": ["rationale", "items"],
    "properties": {
      "rationale": { "type": "string" },
      "items": {
        "type": "array",
        "items": {
          "type": "object",
          "additionalProperties": false,
          "required": ["kind", "reason", "parameters"],
          "properties": {
            "kind": {
              "type": "string",
              "enum": kinds
            },
            "reason": { "type": "string" },
            "parameters": {
              "type": "array",
              "items": {
                "type": "object",
                "additionalProperties": false,
                "required": ["key", "value"],
                "properties": {
                  "key": { "type": "string" },
                  "value": { "type": "string" }
                }
              }
            }
          }
        }
      }
    }
  })
}

/// Render the closed generator catalogue and declared parameters for planner prompts.
pub fn build_generator_catalogue_prompt() -> String {
  let mut lines = Vec::with_capacity(GENERATORS.len());
  for &(kind, template) in GENERATORS {
    let Ok(def) = yanshi_config::executor::parse_workflow_definition(template) else {
      continue;
    };
    let mut param_names: Vec<&String> = def
      .inputs
      .keys()
      .filter(|k| !matches!(k.as_str(), "chunks" | "url" | "text" | "chat_model"))
      .collect();
    param_names.sort();
    let param_specs = param_names
      .into_iter()
      .map(|name| {
        let spec = &def.inputs[name];
        let default_hint = spec
          .default
          .as_ref()
          .and_then(|v| serde_json::to_value(v).ok())
          .map(|v| format!("={v}"))
          .unwrap_or_default();
        let desc = spec
          .description
          .as_deref()
          .map(str::trim)
          .filter(|d| !d.is_empty())
          .map(|d| format!(" ({d})"))
          .unwrap_or_default();
        format!("{name}{default_hint}{desc}")
      })
      .collect::<Vec<_>>()
      .join("; ");
    lines.push(format!("- `{kind}`: params [{param_specs}]"));
  }
  lines.join("\n")
}

/// Coerce an LLM-suggested parameter value to match a generator input's declared default type.
pub fn coerce_planned_param_value(
  raw: Value,
  spec: &yanshi_config::config::v2::InputDefinitionV2,
) -> Option<Value> {
  if raw.is_null() {
    return None;
  }
  match &spec.default {
    Some(serde_yaml::Value::Bool(_)) => match raw {
      Value::Bool(b) => Some(Value::Bool(b)),
      Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
        "true" | "1" | "yes" => Some(Value::Bool(true)),
        "false" | "0" | "no" => Some(Value::Bool(false)),
        _ => None,
      },
      _ => None,
    },
    Some(serde_yaml::Value::Number(default_num)) => {
      let prefer_int = default_num.is_i64() || default_num.is_u64();
      match raw {
        Value::Number(n) => {
          if prefer_int {
            n.as_i64()
              .map(Value::from)
              .or_else(|| n.as_f64().map(|f| Value::from(f.round() as i64)))
          } else {
            n.as_f64().filter(|f| f.is_finite()).map(Value::from)
          }
        }
        Value::String(s) => {
          let trimmed = s.trim();
          if prefer_int {
            trimmed.parse::<i64>().ok().map(Value::from).or_else(|| {
              trimmed
                .parse::<f64>()
                .ok()
                .filter(|f| f.is_finite())
                .map(|f| Value::from(f.round() as i64))
            })
          } else {
            trimmed
              .parse::<f64>()
              .ok()
              .filter(|f| f.is_finite())
              .map(Value::from)
          }
        }
        _ => None,
      }
    }
    Some(serde_yaml::Value::String(_)) | None => match raw {
      Value::String(s) => {
        let trimmed = s.trim();
        if trimmed.is_empty() {
          None
        } else {
          Some(Value::String(trimmed.to_string()))
        }
      }
      Value::Bool(b) => Some(Value::String(b.to_string())),
      Value::Number(n) => Some(Value::String(n.to_string())),
      _ => None,
    },
    Some(_) => None,
  }
}

/// Sanitize a planner LLM JSON reply against the closed generator catalogue and
/// an optional caller policy predicate (`allow_params`).
pub fn sanitize_studio_plan_reply(
  raw_reply: &str,
  max_items: usize,
  allow_params: impl Fn(
    &yanshi_config::config::v2::FlowDefinitionV2,
    &serde_json::Map<String, Value>,
  ) -> bool,
) -> Result<(String, Vec<PlannedGeneratorItem>), String> {
  #[derive(Deserialize)]
  struct RawKvParam {
    #[serde(default)]
    key: String,
    #[serde(default)]
    value: Value,
  }
  #[derive(Deserialize)]
  struct RawPlanItem {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    parameters: Vec<RawKvParam>,
    #[serde(default)]
    params: serde_json::Map<String, Value>,
  }
  #[derive(Deserialize)]
  struct RawPlan {
    #[serde(default)]
    rationale: String,
    #[serde(default)]
    items: Vec<RawPlanItem>,
  }

  let value = extract_json_object(raw_reply)?;
  let parsed: RawPlan =
    serde_json::from_value(value).map_err(|e| format!("invalid studio plan JSON: {e}"))?;
  let capped = max_items.clamp(1, MAX_BATCH_GENERATE_ITEMS);

  let mut seen_kinds: HashSet<String> = HashSet::new();
  let mut items = Vec::new();

  for raw_item in parsed.items {
    if items.len() >= capped {
      break;
    }
    let kind = raw_item.kind.trim().to_string();
    let Some(&(_, template)) = GENERATORS.iter().find(|(k, _)| *k == kind) else {
      continue;
    };
    if !seen_kinds.insert(kind.clone()) {
      continue;
    }
    let Ok(definition) = yanshi_config::executor::parse_workflow_definition(template) else {
      continue;
    };

    let mut merged_candidates: Vec<(String, Value)> = Vec::new();
    for kv in raw_item.parameters {
      merged_candidates.push((kv.key, kv.value));
    }
    for (k, v) in raw_item.params {
      merged_candidates.push((k, v));
    }

    let mut clean_params = serde_json::Map::new();
    for (raw_key, raw_val) in merged_candidates {
      let key = raw_key.trim();
      if key.is_empty() || matches!(key, "chunks" | "url" | "text" | "chat_model") {
        continue;
      }
      let Some(spec) = definition.inputs.get(key) else {
        continue;
      };
      let Some(coerced) = coerce_planned_param_value(raw_val, spec) else {
        continue;
      };
      let mut probe = clean_params.clone();
      probe.insert(key.to_string(), coerced.clone());
      if allow_params(&definition, &probe) {
        clean_params.insert(key.to_string(), coerced);
      }
    }

    let reason = raw_item
      .reason
      .as_deref()
      .map(str::trim)
      .filter(|r| !r.is_empty())
      .map(|r| truncate_chars(r, 400));

    items.push(PlannedGeneratorItem {
      kind,
      reason,
      params: clean_params,
    });
  }

  if items.is_empty() {
    return Err("studio planner returned no valid generator items".to_string());
  }

  let rationale = {
    let trimmed = truncate_chars(&parsed.rationale, 1_200);
    if trimmed.is_empty() {
      format!(
        "Selected {} studio deliverable(s) for this notebook.",
        items.len()
      )
    } else {
      trimmed
    }
  };
  Ok((rationale, items))
}

/// Select chunks up to `budget` bytes while prioritising BM25 relevance to
/// `query_hint` (`focus` / `topic`) and round-robin coverage across sources,
/// then preserving natural reading order.
pub async fn select_chunks_by_budget<T: Clone>(
  ready: Vec<T>,
  query_hint: Option<&str>,
  budget: usize,
  chunk_id: impl Fn(&T) -> &str,
  source_id: impl Fn(&T) -> Uuid,
  content: impl Fn(&T) -> &str,
  truncate_content: impl Fn(&mut T, usize) -> bool,
) -> Vec<T> {
  if ready.is_empty() || budget == 0 {
    return Vec::new();
  }
  let total_bytes: usize = ready.iter().map(|c| content(c).len()).sum();
  if total_bytes <= budget {
    return ready;
  }

  let mut remaining = budget;
  let mut picked = vec![false; ready.len()];
  let pos_by_id: HashMap<&str, usize> = ready
    .iter()
    .enumerate()
    .map(|(idx, c)| (chunk_id(c), idx))
    .collect();

  // 1. Pull BM25 hits matching the caller's focus / topic first so relevant
  //    passages in later sources are never starved by earlier sources.
  if let Some(query) = query_hint.map(str::trim).filter(|q| !q.is_empty()) {
    let backend = Bm25KnowledgeBackend::from_documents(
      ready
        .iter()
        .map(|c| (chunk_id(c).to_string(), content(c).to_string())),
    );
    if let Ok(hits) = backend.search(query, ready.len()).await {
      for hit in hits {
        if hit.score <= 0.0 {
          continue;
        }
        if let Some(&idx) = pos_by_id.get(hit.id.as_str())
          && !picked[idx]
          && content(&ready[idx]).len() <= remaining
        {
          picked[idx] = true;
          remaining -= content(&ready[idx]).len();
        }
      }
    }
  }

  // 2. Fill remaining budget round-robin across sources so multi-document
  //    notebooks stay balanced even when `focus` is omitted or generic.
  let mut source_order: Vec<Uuid> = Vec::new();
  let mut buckets: HashMap<Uuid, VecDeque<usize>> = HashMap::new();
  for (idx, chunk) in ready.iter().enumerate() {
    if picked[idx] {
      continue;
    }
    let sid = source_id(chunk);
    let entry = buckets.entry(sid).or_insert_with(|| {
      source_order.push(sid);
      VecDeque::new()
    });
    entry.push_back(idx);
  }

  loop {
    let mut progressed = false;
    for sid in &source_order {
      let Some(queue) = buckets.get_mut(sid) else {
        continue;
      };
      while let Some(idx) = queue.pop_front() {
        let len = content(&ready[idx]).len();
        if len <= remaining {
          picked[idx] = true;
          remaining -= len;
          progressed = true;
          break;
        }
      }
    }
    if !progressed {
      break;
    }
  }

  let mut selected: Vec<T> = ready
    .iter()
    .enumerate()
    .filter(|(idx, _)| picked[*idx])
    .map(|(_, chunk)| chunk.clone())
    .collect();

  // Safety fallback when every single chunk exceeds `budget`.
  if selected.is_empty()
    && let Some(mut first) = ready.into_iter().next()
  {
    let cut = content(&first)
      .char_indices()
      .take_while(|(byte_idx, ch)| byte_idx + ch.len_utf8() <= budget)
      .map(|(byte_idx, ch)| byte_idx + ch.len_utf8())
      .last()
      .unwrap_or(0);
    if truncate_content(&mut first, cut) {
      selected.push(first);
    }
  }

  selected
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn generators_catalogue_and_planner_sanitization_work() {
    assert!(find_generator_template("report").is_ok());
    assert!(find_generator_template("nonexistent_kind").is_err());

    let raw = r#"{"rationale":"Good mix","items":[{"kind":"report","reason":"Overview","parameters":[{"key":"focus","value":"latency"},{"key":"chunks","value":"blocked"}]},{"kind":"bogus","reason":"skip","parameters":[]}]}"#;
    let (rationale, items) =
      sanitize_studio_plan_reply(raw, 4, |_, _| true).expect("valid studio plan");
    assert_eq!(rationale, "Good mix");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].kind, "report");
    assert!(!items[0].params.contains_key("chunks"));
  }

  #[test]
  fn split_batch_cost_limit_validates_and_divides() {
    assert_eq!(split_batch_cost_limit(None, 3).expect("none"), None);
    let per_run = split_batch_cost_limit(Some(1.5), 3)
      .expect("valid")
      .expect("some");
    assert!((per_run - 0.5).abs() < 1e-9);
    assert!(split_batch_cost_limit(Some(-1.0), 2).is_err());
  }
}
