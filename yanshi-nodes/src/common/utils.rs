use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use yanshi_core::{error::YanshiError, value::FlowValue};

pub fn flow_value_to_string(value: &FlowValue) -> String {
  match value {
    FlowValue::Json(Value::String(s)) => s.clone(),
    FlowValue::Json(v) => v.to_string().trim_matches('"').to_string(),
    FlowValue::File { path, .. } => path.to_string_lossy().to_string(),
    FlowValue::Url { url, .. } => url.clone(),
  }
}

async fn read_file_as_data_uri(path: &std::path::Path) -> Result<String, YanshiError> {
  let data = tokio::fs::read(path)
    .await
    .map_err(|e| YanshiError::NodeInputError {
      message: format!("Failed to read file at {:?}: {}", path, e),
    })?;
  let mime_type = mime_guess::from_path(path).first_or_octet_stream();
  Ok(format!(
    "data:{};base64,{}",
    mime_type,
    STANDARD.encode(data)
  ))
}

pub async fn load_data_uri_from_str(source: &str) -> Result<String, YanshiError> {
  let trimmed = source.trim();
  if trimmed.starts_with("http://")
    || trimmed.starts_with("https://")
    || trimmed.starts_with("data:")
  {
    return Ok(trimmed.to_string());
  }
  read_file_as_data_uri(std::path::Path::new(trimmed)).await
}

pub async fn load_data_uri_from_flow_value(
  value: &FlowValue,
  label: &str,
) -> Result<String, YanshiError> {
  match value {
    FlowValue::Json(Value::String(s)) => load_data_uri_from_str(s).await,
    FlowValue::File { path, .. } => read_file_as_data_uri(path).await,
    FlowValue::Url { url, .. } => Ok(url.clone()),
    FlowValue::Json(Value::Object(obj)) => {
      if let Some(path_str) = obj
        .get("path")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
      {
        return read_file_as_data_uri(std::path::Path::new(path_str.trim())).await;
      }
      if let Some(url_str) = obj
        .get("url")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty())
      {
        return Ok(url_str.trim().to_string());
      }
      Err(YanshiError::NodeInputError {
        message: format!(
          "Unsupported JSON object for source '{}': expected 'path' or 'url' field",
          label
        ),
      })
    }
    _ => Err(YanshiError::NodeInputError {
      message: format!("Unsupported FlowValue type for source '{}'", label),
    }),
  }
}

pub async fn load_data_uri_from_source(
  source: &str,
  inputs: &yanshi_core::async_node::AsyncNodeInputs,
) -> Result<String, YanshiError> {
  if !source.is_empty()
    && let Some(value) = inputs.get(source)
  {
    return load_data_uri_from_flow_value(value, source).await;
  }
  load_data_uri_from_str(source).await
}

pub async fn load_data_uri_from_input_or_value(
  value: &FlowValue,
  key: &str,
  inputs: &yanshi_core::async_node::AsyncNodeInputs,
) -> Result<String, YanshiError> {
  if let FlowValue::Json(Value::String(s)) = value {
    let trimmed = s.trim();
    if !trimmed.is_empty() && trimmed != key && inputs.contains_key(trimmed) {
      return load_data_uri_from_source(trimmed, inputs).await;
    }
  }
  load_data_uri_from_flow_value(value, key).await
}

pub async fn load_bytes_from_source(
  source: &str,
  inputs: &yanshi_core::async_node::AsyncNodeInputs,
) -> Result<Vec<u8>, YanshiError> {
  if let Some(value) = inputs.get(source) {
    return match value {
      FlowValue::Json(Value::String(s)) => {
        if let Some(data) = s.strip_prefix("data:") {
          let parts: Vec<&str> = data.split(";base64,").collect();
          if parts.len() == 2 {
            return STANDARD
              .decode(parts[1])
              .map_err(|e| YanshiError::NodeInputError {
                message: format!("Invalid base64 data in input '{}': {}", source, e),
              });
          }
        }
        Err(YanshiError::NodeInputError {
          message: format!(
            "Unsupported string format for source '{}'. Expected base64 data URI.",
            source
          ),
        })
      }
      FlowValue::File { path, .. } => {
        tokio::fs::read(path)
          .await
          .map_err(|e| YanshiError::NodeInputError {
            message: format!("Failed to read file at {:?}: {}", path, e),
          })
      }
      _ => Err(YanshiError::NodeInputError {
        message: format!("Unsupported FlowValue type for source '{}'", source),
      }),
    };
  }

  if source.starts_with("http") {
    let response = reqwest::get(source)
      .await
      .map_err(|e| YanshiError::NodeInputError {
        message: format!("Failed to download data from URL {}: {}", source, e),
      })?;
    return response
      .bytes()
      .await
      .map(|b| b.to_vec())
      .map_err(|e| YanshiError::NodeInputError {
        message: format!("Failed to read bytes from URL {}: {}", source, e),
      });
  }

  tokio::fs::read(source)
    .await
    .map_err(|e| YanshiError::NodeInputError {
      message: format!("Failed to read file at {}: {}", source, e),
    })
}
