//! ImageUnderstand Node - Specialized node for multimodal image understanding using vision models

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_llm::{Yanshi, multimodal::MultimodalMessage};
use yanshi_nodes::common::utils::{
  flow_value_to_string, load_data_uri_from_flow_value, load_data_uri_from_source,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageUnderstandNode {
  pub name: String,
  pub model: String,
  pub text_prompt: String,
  pub image_source: String,
  #[serde(default)]
  pub reference_image: Option<String>,
  pub system_message: Option<String>,
  pub temperature: Option<f32>,
  pub max_tokens: Option<u32>,
  pub output_key: String,
  pub input_keys: Vec<String>,
}

impl ImageUnderstandNode {
  pub fn new(name: &str, model: &str, text_prompt: &str, image_source: &str) -> Self {
    Self {
      name: name.to_string(),
      model: model.to_string(),
      text_prompt: text_prompt.to_string(),
      image_source: image_source.to_string(),
      reference_image: None,
      system_message: None,
      temperature: None,
      max_tokens: None,
      output_key: format!("{}_output", name),
      input_keys: vec![],
    }
  }

  pub fn with_reference_image(mut self, reference: impl Into<String>) -> Self {
    let r = reference.into();
    if !r.trim().is_empty() {
      self.reference_image = Some(r);
    }
    self
  }

  pub fn with_system_message(mut self, system: impl Into<String>) -> Self {
    let s = system.into();
    if !s.trim().is_empty() {
      self.system_message = Some(s);
    }
    self
  }

  pub fn with_temperature(mut self, temperature: f32) -> Self {
    self.temperature = Some(temperature);
    self
  }

  pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
    self.max_tokens = Some(max_tokens);
    self
  }

  pub(crate) fn resolve_prompt(&self, inputs: &AsyncNodeInputs) -> Result<String, YanshiError> {
    if !self.text_prompt.trim().is_empty() {
      let mut resolved = self.text_prompt.clone();
      for (key, value) in inputs {
        let placeholder = format!("{{{{{}}}}}", key);
        if resolved.contains(&placeholder) {
          resolved = resolved.replace(&placeholder, &flow_value_to_string(value));
        }
      }
      return Ok(resolved);
    }
    for key in ["text_prompt", "prompt"] {
      if let Some(val) = inputs.get(key) {
        let s = flow_value_to_string(val);
        if !s.trim().is_empty() {
          return Ok(s);
        }
      }
    }
    Err(YanshiError::NodeInputError {
      message: format!(
        "image_understand '{}': required 'text_prompt' is empty",
        self.name
      ),
    })
  }

  pub(crate) async fn resolve_images(
    &self,
    inputs: &AsyncNodeInputs,
  ) -> Result<(Option<String>, String), YanshiError> {
    let primary = if let Some(val) = inputs.get("image_source") {
      load_data_uri_from_flow_value(val, "image_source").await?
    } else if !self.image_source.trim().is_empty() {
      load_data_uri_from_source(&self.image_source, inputs).await?
    } else {
      return Err(YanshiError::NodeInputError {
        message: format!(
          "image_understand '{}': required 'image_source' is missing",
          self.name
        ),
      });
    };

    let reference = if let Some(val) = inputs.get("reference_image") {
      let raw_str = flow_value_to_string(val);
      if raw_str.trim().is_empty() || matches!(val, FlowValue::Json(Value::Null)) {
        None
      } else {
        Some(load_data_uri_from_flow_value(val, "reference_image").await?)
      }
    } else if let Some(ref_src) = self
      .reference_image
      .as_deref()
      .filter(|s| !s.trim().is_empty())
    {
      Some(load_data_uri_from_source(ref_src, inputs).await?)
    } else {
      None
    };

    Ok((reference, primary))
  }
}

#[async_trait]
impl AsyncNode for ImageUnderstandNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    tracing::debug!(name = %self.name, "executing ImageUnderstandNode");

    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("Failed to initialize Yanshi LLM service: {}", e),
      })?;

    let string_input = |key: &str| match inputs.get(key) {
      Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.clone()),
      _ => None,
    };

    let model = string_input("model").unwrap_or_else(|| self.model.clone());
    if model.trim().is_empty() {
      return Err(YanshiError::NodeInputError {
        message: format!(
          "image_understand '{}': required 'model' is empty",
          self.name
        ),
      });
    }
    let resolved_prompt = self.resolve_prompt(inputs)?;
    let (reference_uri, image_data_uri) = self.resolve_images(inputs).await?;

    let mut msg_builder = MultimodalMessage::user().add_text(resolved_prompt);
    if let Some(ref_uri) = reference_uri {
      msg_builder = msg_builder.add_image_url(ref_uri);
    }
    let message = msg_builder.add_image_url(image_data_uri).build();

    let mut request = Yanshi::model(&model).multimodal_prompt(message);

    if let Some(system_message) = string_input("system").or_else(|| self.system_message.clone()) {
      request = request.system(&system_message);
    }
    if let Some(temp) = self.temperature {
      request = request.temperature(temp);
    }
    if let Some(max_tokens) = self.max_tokens {
      request = request.max_tokens(max_tokens);
    }

    let response = request
      .execute_full()
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("LLM execution failed: {}", e),
      })?;

    crate::cost::record(&model, |p| match &response.usage {
      Some(u) => p.cost_for_call(u.prompt_tokens, u.completion_tokens),
      None => 0.0,
    })?;

    tracing::debug!("ImageUnderstandNode execution successful");
    let mut outputs = HashMap::new();
    outputs.insert(
      self.output_key.clone(),
      FlowValue::Json(Value::String(response.content)),
    );

    Ok(outputs)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[tokio::test]
  async fn resolves_dual_reference_and_candidate_images_from_inputs() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cast = dir.path().join("cast.png");
    let shot = dir.path().join("shot.png");
    tokio::fs::write(&cast, b"\x89PNG\r\n\x1a\n")
      .await
      .expect("write cast");
    tokio::fs::write(&shot, b"\x89PNG\r\n\x1a\n")
      .await
      .expect("write shot");

    let node = ImageUnderstandNode::new("critic", "", "Compare {{role}} consistency", "");
    let mut inputs = AsyncNodeInputs::new();
    inputs.insert("role".to_string(), FlowValue::Json(json!("lead detective")));
    inputs.insert(
      "reference_image".to_string(),
      FlowValue::Json(json!({ "path": cast.to_string_lossy() })),
    );
    inputs.insert(
      "image_source".to_string(),
      FlowValue::File {
        path: shot,
        mime_type: Some("image/png".to_string()),
      },
    );

    assert_eq!(
      node.resolve_prompt(&inputs).expect("prompt"),
      "Compare lead detective consistency"
    );
    let (ref_uri, shot_uri) = node.resolve_images(&inputs).await.expect("images");
    assert!(
      ref_uri
        .expect("ref present")
        .starts_with("data:image/png;base64,")
    );
    assert!(shot_uri.starts_with("data:image/png;base64,"));
  }

  #[tokio::test]
  async fn test_image_understand_node_integration() {
    if std::env::var("STEP_API_KEY").is_err() {
      println!("Skipping ImageUnderstand integration test: STEP_API_KEY not set.");
      return;
    }

    let node = ImageUnderstandNode::new(
      "test_vision",
      "step-1o-turbo-vision", // This model should be available via the StepFun endpoint
      "What is in this image? A single word answer is fine.",
      "image.png",
    );

    let mut inputs = AsyncNodeInputs::new();
    let image_data = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAAAAACPAi4CAAAABGdBTUEAALGPC/xhBQAAAA1JREFUGFdjYGBgYAAAAAUAAYcA/wAAAABJRU5ErkJggg==";
    inputs.insert(
      "image.png".to_string(),
      FlowValue::Json(Value::String(image_data.to_string())),
    );

    let result = node.execute(&inputs).await;
    assert!(result.is_ok(), "Node execution failed: {:?}", result.err());

    let outputs = result.unwrap();
    let output_value = outputs.get("test_vision_output").unwrap();
    if let FlowValue::Json(Value::String(s)) = output_value {
      // The image is black, so the answer should be something like "black" or "nothing".
      println!("Vision model output: {}", s);
      assert!(
        !s.is_empty(),
        "Vision model should have returned a description."
      );
    } else {
      panic!("Output was not a FlowValue::Json(Value::String(...))");
    }
  }
}
