//! ImageToImage Node - Transforms an image based on a text prompt.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_llm::{Yanshi, providers::modality::Image2ImageRequest as ModalityImage2ImageRequest};
use yanshi_nodes::common::utils::{
  flow_value_to_string, load_data_uri_from_flow_value, load_data_uri_from_input_or_value,
  load_data_uri_from_source, load_data_uri_from_str,
};

use crate::artifacts::{self, ArtifactOutputFormat};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageToImageNode {
  pub name: String,
  pub model: String,
  pub prompt: String,
  pub source_image: String,
  #[serde(default)]
  pub reference_images: Vec<String>,
  pub source_weight: f32,
  pub size: Option<String>,
  pub n: Option<u32>,
  pub response_format: Option<String>,
  pub seed: Option<i32>,
  pub steps: Option<u32>,
  pub cfg_scale: Option<f32>,
  pub output_key: String,
  pub input_keys: Vec<String>,
  /// Y1.1: see [`ArtifactOutputFormat`].
  #[serde(default)]
  pub output_format: ArtifactOutputFormat,
}

impl ImageToImageNode {
  pub fn new(name: &str, model: &str, prompt: &str, source_image: &str) -> Self {
    Self {
      name: name.to_string(),
      model: model.to_string(),
      prompt: prompt.to_string(),
      source_image: source_image.to_string(),
      reference_images: Vec::new(),
      source_weight: 0.8,
      size: None,
      n: Some(1),
      response_format: Some("b64_json".to_string()),
      seed: None,
      steps: None,
      cfg_scale: None,
      output_key: format!("{}_output", name),
      input_keys: vec![],
      output_format: ArtifactOutputFormat::default(),
    }
  }

  pub fn with_output_format(mut self, format: ArtifactOutputFormat) -> Self {
    self.output_format = format;
    self
  }

  pub fn with_size(mut self, size: impl Into<String>) -> Self {
    let s = size.into();
    if !s.trim().is_empty() {
      self.size = Some(s);
    }
    self
  }

  pub fn with_source_weight(mut self, weight: f32) -> Self {
    self.source_weight = weight;
    self
  }

  pub fn with_seed(mut self, seed: i32) -> Self {
    self.seed = Some(seed);
    self
  }

  pub fn with_reference_images(mut self, refs: Vec<String>) -> Self {
    self.reference_images = refs;
    self
  }

  pub(crate) fn resolve_prompt(&self, inputs: &AsyncNodeInputs) -> Result<String, YanshiError> {
    if !self.prompt.trim().is_empty() {
      let mut resolved = self.prompt.clone();
      for (key, value) in inputs {
        let placeholder = format!("{{{{{}}}}}", key);
        if resolved.contains(&placeholder) {
          resolved = resolved.replace(&placeholder, &flow_value_to_string(value));
        }
      }
      return Ok(resolved);
    }
    if let Some(value) = inputs.get("prompt") {
      let s = flow_value_to_string(value);
      if !s.trim().is_empty() {
        return Ok(s);
      }
    }
    Err(YanshiError::NodeInputError {
      message: format!("image_to_image '{}': required 'prompt' is empty", self.name),
    })
  }

  pub(crate) async fn resolve_source_and_references(
    &self,
    inputs: &AsyncNodeInputs,
  ) -> Result<(String, Vec<String>), YanshiError> {
    let source_url = if let Some(val) = inputs.get("source_image") {
      load_data_uri_from_input_or_value(val, "source_image", inputs).await?
    } else if !self.source_image.trim().is_empty() {
      load_data_uri_from_source(&self.source_image, inputs).await?
    } else {
      return Err(YanshiError::NodeInputError {
        message: format!(
          "image_to_image '{}': required 'source_image' is missing",
          self.name
        ),
      });
    };

    let mut reference_urls = Vec::new();
    if let Some(val) = inputs.get("reference_images") {
      match val {
        FlowValue::Json(Value::Array(items)) => {
          for item in items {
            let fv = FlowValue::Json(item.clone());
            if let FlowValue::Json(Value::String(s)) = &fv
              && s.trim().is_empty()
            {
              continue;
            }
            reference_urls
              .push(load_data_uri_from_input_or_value(&fv, "reference_images", inputs).await?);
          }
        }
        FlowValue::Json(Value::Null) => {}
        other => {
          let s = flow_value_to_string(other);
          if !s.trim().is_empty() {
            reference_urls
              .push(load_data_uri_from_input_or_value(other, "reference_images", inputs).await?);
          }
        }
      }
    } else {
      for raw in &self.reference_images {
        if raw.trim().is_empty() {
          continue;
        }
        if let Some(val) = inputs.get(raw.as_str()) {
          reference_urls.push(load_data_uri_from_flow_value(val, raw).await?);
        } else {
          reference_urls.push(load_data_uri_from_str(raw).await?);
        }
      }
    }

    Ok((source_url, reference_urls))
  }
}

#[async_trait]
impl AsyncNode for ImageToImageNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    tracing::debug!(name = %self.name, "executing ImageToImageNode");

    let string_input = |key: &str| match inputs.get(key) {
      Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.clone()),
      _ => None,
    };

    let model = string_input("model").unwrap_or_else(|| self.model.clone());
    if model.trim().is_empty() {
      return Err(YanshiError::NodeInputError {
        message: format!("image_to_image '{}': required 'model' is empty", self.name),
      });
    }
    let resolved_prompt = self.resolve_prompt(inputs)?;
    let (source_url, reference_urls) = self.resolve_source_and_references(inputs).await?;
    let size = string_input("size").or_else(|| self.size.clone());
    let source_weight = match inputs.get("source_weight") {
      Some(FlowValue::Json(Value::Number(n))) => {
        n.as_f64().map(|v| v as f32).unwrap_or(self.source_weight)
      }
      _ => self.source_weight,
    };

    Yanshi::ensure_init()
      .await
      .map_err(|e| YanshiError::ConfigurationError {
        message: format!("Failed to initialize Yanshi LLM service: {}", e),
      })?;

    // P-LLM.3: route through the modality dispatcher.
    let provider =
      Yanshi::image2image(&model)
        .await
        .map_err(|e| YanshiError::ConfigurationError {
          message: format!(
            "Failed to resolve image-to-image provider for '{}': {}",
            model, e
          ),
        })?;

    let request = ModalityImage2ImageRequest {
      model: model.clone(),
      prompt: resolved_prompt,
      source_url,
      reference_urls,
      source_weight,
      size,
      n: self.n,
      response_format: self.response_format.clone(),
      seed: self.seed,
      steps: self.steps,
      cfg_scale: self.cfg_scale,
    };

    tracing::debug!(provider = %provider.name(), "transforming via provider");
    let response = provider
      .transform(request)
      .await
      .map_err(|e| crate::vendor_error::vendor_error("Image-to-image transform failed", e))?;
    crate::cost::record(&model, |p| p.cost_for_images(response.images.len()))?;

    let first_image = response
      .images
      .first()
      .ok_or_else(|| YanshiError::AsyncExecutionError {
        message: "No images returned in response".to_string(),
      })?;
    let value = artifacts::image_output(&self.name, first_image, self.output_format).await?;

    tracing::debug!("ImageToImageNode execution successful");
    let mut outputs = HashMap::new();
    outputs.insert(self.output_key.clone(), value);

    Ok(outputs)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[tokio::test]
  async fn resolves_runtime_prompt_and_serialized_file_references() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cast_path = dir.path().join("cast.png");
    let prop_path = dir.path().join("prop.png");
    tokio::fs::write(&cast_path, b"\x89PNG\r\n\x1a\n")
      .await
      .expect("write cast");
    tokio::fs::write(&prop_path, b"\x89PNG\r\n\x1a\n")
      .await
      .expect("write prop");

    let node = ImageToImageNode::new("shot", "", "", "");
    let mut inputs = AsyncNodeInputs::new();
    inputs.insert(
      "prompt".to_string(),
      FlowValue::Json(json!("Episode 2 frame")),
    );
    inputs.insert(
      "source_image".to_string(),
      FlowValue::Json(json!({ "path": cast_path.to_string_lossy() })),
    );
    inputs.insert(
      "reference_images".to_string(),
      FlowValue::Json(json!([prop_path.to_string_lossy()])),
    );

    let prompt = node.resolve_prompt(&inputs).expect("prompt");
    assert_eq!(prompt, "Episode 2 frame");
    let (source_url, refs) = node
      .resolve_source_and_references(&inputs)
      .await
      .expect("sources");
    assert!(source_url.starts_with("data:image/png;base64,"));
    assert_eq!(refs.len(), 1);
    assert!(refs[0].starts_with("data:image/png;base64,"));
  }
}
