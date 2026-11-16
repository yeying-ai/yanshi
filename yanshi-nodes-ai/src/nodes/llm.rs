use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_llm::{Yanshi, providers::ContentBlock};
use yanshi_nodes::common::artifacts;

/// A chat-model call. `output` is the reply's text (or its parsed JSON
/// under `json_schema`); a reply that also carries images (Y6.6, e.g. a
/// Gemini image-output model) adds `images`, the images written as this
/// node's artifacts.
#[derive(Debug, Clone, Default)]
pub struct LlmNode {
  /// Names the artifact directory for reply images (`llm` when unset).
  id: Option<String>,
}

impl LlmNode {
  pub fn with_id(id: impl Into<String>) -> Self {
    Self {
      id: Some(id.into()),
    }
  }
}

#[async_trait]
impl AsyncNode for LlmNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let prompt = get_string_input(inputs, "prompt")?;
    let model = get_optional_string_input(inputs, "model")?;
    let system = get_optional_string_input(inputs, "system")?;

    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("Failed to initialize Yanshi LLM service: {}", e),
      })?;

    let model_name = model.unwrap_or_default().to_string();
    // Y4.1: `json_schema` (a JSON Schema object) constrains the reply and
    // makes `output` the parsed JSON value rather than a string, so a
    // generator's outline can feed a Map node directly.
    let schema = match inputs.get("json_schema") {
      None => None,
      Some(FlowValue::Json(Value::Object(schema))) => Some(Value::Object(schema.clone())),
      Some(FlowValue::Json(Value::String(text))) => Some(serde_json::from_str(text).map_err(
        |e| YanshiError::NodeInputError {
          message: format!("Input 'json_schema' is not valid JSON: {e}"),
        },
      )?),
      Some(_) => {
        return Err(YanshiError::NodeInputError {
          message: "Input 'json_schema' must be a JSON Schema object".to_string(),
        });
      }
    };
    // Embed the schema contract in the user prompt as well so providers that
    // fall back to `json_object` / `responseMimeType: application/json` (or
    // ignore nested `json_schema` constraints) still see exact field names.
    let effective_prompt = match &schema {
      Some(s) => format!(
        "{prompt}\n\nOutput ONLY valid JSON strictly conforming to this JSON Schema (no markdown fences, no extra prose):\n{}",
        serde_json::to_string(s).unwrap_or_default()
      ),
      None => prompt.to_string(),
    };
    let mut request = Yanshi::model(&model_name).prompt(&effective_prompt);

    if let Some(sys) = system {
      request = request.system(sys);
    }
    if let Some(temp) = get_optional_f64_input(inputs, "temperature")? {
      request = request.temperature(temp as f32);
    }
    if let Some(max_tokens) = get_optional_u64_input(inputs, "max_tokens")? {
      request = request.max_tokens(max_tokens as u32);
    }
    if let Some(schema) = &schema {
      request = request.json_schema("output", schema.clone());
    }

    tracing::debug!("executing LLM request");
    let response = request
      .execute_full()
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("LLM execution failed: {}", e),
      })?;
    tracing::debug!("LLM response received");

    let image_count = response
      .media
      .iter()
      .filter(|block| matches!(block, ContentBlock::Image { .. }))
      .count();
    crate::cost::record(&model_name, |p| {
      let token_cost = match &response.usage {
        Some(u) => p.cost_for_call(u.prompt_tokens, u.completion_tokens),
        None => 0.0,
      };
      token_cost + p.cost_for_images(image_count)
    })?;

    let output = match schema {
      Some(_) => parse_json_reply(&response.content)?,
      None => Value::String(response.content),
    };
    let mut outputs = HashMap::new();
    outputs.insert("output".to_string(), FlowValue::Json(output));

    let node = self.id.as_deref().unwrap_or("llm");
    let mut images = Vec::new();
    for block in &response.media {
      if let ContentBlock::Image { data, media_type } = block {
        let file = artifacts::write_artifact(node, data, media_type).await?;
        images.push(
          serde_json::to_value(&file).map_err(|e| YanshiError::NodeExecutionFailed {
            message: format!("{node}: cannot describe a reply image: {e}"),
          })?,
        );
      }
    }
    if !images.is_empty() {
      outputs.insert("images".to_string(), FlowValue::Json(Value::Array(images)));
    }

    Ok(outputs)
  }
}

/// The JSON in a schema-constrained reply. Providers that ignore or
/// downgrade `json_schema` may prepend `<think>...</think>`, prose, or wrap
/// the payload inside a ` ```json ` code fence.
fn parse_json_reply(reply: &str) -> Result<Value, YanshiError> {
  let trimmed = reply.trim();
  if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
    return Ok(v);
  }

  let without_think = if let Some(start) = trimmed.find("<think>")
    && let Some(end) = trimmed[start..].find("</think>")
  {
    format!(
      "{}{}",
      &trimmed[..start],
      &trimmed[start + end + "</think>".len()..]
    )
  } else {
    trimmed.to_string()
  };
  let s = without_think.trim();
  if let Ok(v) = serde_json::from_str::<Value>(s) {
    return Ok(v);
  }

  if let Some(fence_start) = s.find("```") {
    let after_ticks = &s[fence_start + 3..];
    let body_start = after_ticks.find('\n').map_or(0, |nl| nl + 1);
    let body = &after_ticks[body_start..];
    let candidate = match body.find("```") {
      Some(end) => &body[..end],
      None => body,
    };
    if let Ok(v) = serde_json::from_str::<Value>(candidate.trim()) {
      return Ok(v);
    }
  }

  for (open, close) in [('{', '}'), ('[', ']')] {
    if let (Some(first), Some(last)) = (s.find(open), s.rfind(close))
      && last > first
      && let Ok(v) = serde_json::from_str::<Value>(&s[first..=last])
    {
      return Ok(v);
    }
  }

  serde_json::from_str(s).map_err(|e| YanshiError::NodeExecutionFailed {
    message: format!("the model did not return JSON for json_schema: {e}"),
  })
}

fn get_string_input<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Result<&'a str, YanshiError> {
  inputs
    .get(key)
    .and_then(|v| match v {
      FlowValue::Json(Value::String(s)) => Some(s.as_str()),
      _ => None,
    })
    .ok_or_else(|| YanshiError::NodeInputError {
      message: format!(
        "Required string input '{}' is missing or has wrong type",
        key
      ),
    })
}

fn get_optional_string_input<'a>(
  inputs: &'a AsyncNodeInputs,
  key: &str,
) -> Result<Option<&'a str>, YanshiError> {
  match inputs.get(key) {
    None => Ok(None),
    Some(v) => match v {
      FlowValue::Json(Value::String(s)) => Ok(Some(s.as_str())),
      _ => Err(YanshiError::NodeInputError {
        message: format!("Input '{}' has wrong type, expected a string", key),
      }),
    },
  }
}

fn get_optional_f64_input(inputs: &AsyncNodeInputs, key: &str) -> Result<Option<f64>, YanshiError> {
  match inputs.get(key) {
    None => Ok(None),
    Some(v) => match v {
      FlowValue::Json(Value::Number(n)) => Ok(n.as_f64()),
      _ => Err(YanshiError::NodeInputError {
        message: format!("Input '{}' has wrong type, expected a number", key),
      }),
    },
  }
}

fn get_optional_u64_input(inputs: &AsyncNodeInputs, key: &str) -> Result<Option<u64>, YanshiError> {
  match inputs.get(key) {
    None => Ok(None),
    Some(v) => match v {
      FlowValue::Json(Value::Number(n)) => Ok(n.as_u64()),
      _ => Err(YanshiError::NodeInputError {
        message: format!("Input '{}' has wrong type, expected a number", key),
      }),
    },
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[tokio::test]
  async fn test_llm_node_async_execution() {
    let node = LlmNode::default();
    let mut inputs = AsyncNodeInputs::new();
    inputs.insert(
      "prompt".to_string(),
      FlowValue::Json(json!("What is 2+2? Respond with only the number.")),
    );
    inputs.insert("temperature".to_string(), FlowValue::Json(json!(0.0)));

    let result = node.execute(&inputs).await;

    if let Ok(outputs) = result {
      let response_val = outputs.get("output").unwrap();
      if let FlowValue::Json(Value::String(s)) = response_val {
        assert!(s.contains('4'));
      }
    } else {
      println!(
        "LLM node execution failed (as expected without API key): {:?}",
        result.err().unwrap()
      );
    }
  }

  #[test]
  fn json_replies_parse_with_or_without_a_fence() {
    assert_eq!(parse_json_reply(r#"{"a": 1}"#).unwrap(), json!({"a": 1}));
    assert_eq!(
      parse_json_reply("```json\n[1, 2]\n```").unwrap(),
      json!([1, 2])
    );
    assert_eq!(
      parse_json_reply(
        "<think>reasoning</think>\nHere is JSON:\n```json\n{\"ok\":true}\n```\nDone."
      )
      .unwrap(),
      json!({"ok": true})
    );
    assert_eq!(
      parse_json_reply("Sure, result below:\n{\"k\": \"v\"}\nthanks!").unwrap(),
      json!({"k": "v"})
    );
    assert!(parse_json_reply("not json").is_err());
  }
}
