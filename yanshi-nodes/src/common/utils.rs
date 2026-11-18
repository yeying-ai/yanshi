use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use yanshi_core::{error::YanshiError, value::FlowValue};
use yanshi_tools::{SandboxPolicy, ToolError, builtin::HttpTool};

/// Upper bound on remote audio / image payloads fetched by modality nodes (50 MiB).
const MAX_REMOTE_MEDIA_BYTES: usize = 50 * 1024 * 1024;

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

/// Fetch `url` through `HttpTool`'s default `SandboxPolicy` so workflow
/// modality nodes inherit pinned-DNS SSRF blocking (loopback, RFC1918,
/// cloud metadata, IPv4-mapped IPv6) and bounded streaming reads (`F-A4-1`).
async fn fetch_remote_media(url: &str) -> Result<(Vec<u8>, String), YanshiError> {
  let tool =
    HttpTool::new(Arc::new(SandboxPolicy::default())).map_err(|e| YanshiError::NodeInputError {
      message: format!("Failed to initialize HTTP client for '{}': {}", url, e),
    })?;
  let response = tool
    .send_bytes(
      &json!({ "url": url, "method": "GET" }),
      Some(MAX_REMOTE_MEDIA_BYTES),
    )
    .await
    .map_err(|err| match err {
      ToolError::PolicyDenied { .. }
      | ToolError::PolicyDeniedAndStop { .. }
      | ToolError::SandboxViolation { .. }
      | ToolError::InvalidParams { .. } => YanshiError::NodeInputError {
        message: format!("Remote media URL '{}' rejected: {}", url, err),
      },
      ToolError::HttpError { ref message } if message.contains("exceed") => {
        YanshiError::NodeInputError {
          message: format!("Remote media URL '{}' rejected: {}", url, err),
        }
      }
      other => YanshiError::NetworkError {
        message: format!("Failed to download media from '{}': {}", url, other),
      },
    })?;

  let status = response.status.as_u16();
  if !response.status.is_success() {
    let message = format!(
      "Remote media URL '{}' returned HTTP {}",
      url, response.status
    );
    return if status == 429 || status >= 500 {
      Err(YanshiError::NetworkError { message })
    } else {
      Err(YanshiError::NodeInputError { message })
    };
  }

  let header_mime = response
    .content_type
    .as_deref()
    .and_then(|ct| ct.split(';').next())
    .map(str::trim)
    .filter(|ct| !ct.is_empty() && *ct != "application/octet-stream")
    .map(ToOwned::to_owned);
  let guessed_mime = url
    .split(['?', '#'])
    .next()
    .map(|clean| {
      mime_guess::from_path(clean)
        .first_or_octet_stream()
        .to_string()
    })
    .unwrap_or_else(|| "application/octet-stream".to_string());
  let mime = header_mime.unwrap_or(guessed_mime);
  Ok((response.body, mime))
}

pub async fn load_data_uri_from_str(source: &str) -> Result<String, YanshiError> {
  let trimmed = source.trim();
  if trimmed.starts_with("data:") {
    return Ok(trimmed.to_string());
  }
  if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
    let (bytes, mime) = fetch_remote_media(trimmed).await?;
    return Ok(format!("data:{};base64,{}", mime, STANDARD.encode(bytes)));
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
    FlowValue::Url { url, .. } => load_data_uri_from_str(url).await,
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
        return load_data_uri_from_str(url_str.trim()).await;
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
        let trimmed = s.trim();
        if let Some(data) = trimmed.strip_prefix("data:") {
          let parts: Vec<&str> = data.split(";base64,").collect();
          if parts.len() == 2 {
            return STANDARD
              .decode(parts[1])
              .map_err(|e| YanshiError::NodeInputError {
                message: format!("Invalid base64 data in input '{}': {}", source, e),
              });
          }
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
          return fetch_remote_media(trimmed).await.map(|(bytes, _)| bytes);
        }
        Err(YanshiError::NodeInputError {
          message: format!(
            "Unsupported string format for source '{}'. Expected base64 data URI or http(s) URL.",
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
      FlowValue::Url { url, .. } => fetch_remote_media(url.trim()).await.map(|(bytes, _)| bytes),
      _ => Err(YanshiError::NodeInputError {
        message: format!("Unsupported FlowValue type for source '{}'", source),
      }),
    };
  }

  let trimmed = source.trim();
  if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
    return fetch_remote_media(trimmed).await.map(|(bytes, _)| bytes);
  }

  tokio::fs::read(trimmed)
    .await
    .map_err(|e| YanshiError::NodeInputError {
      message: format!("Failed to read file at {}: {}", trimmed, e),
    })
}

#[cfg(test)]
mod tests {
  use super::*;
  use yanshi_core::async_node::AsyncNodeInputs;

  #[tokio::test]
  async fn load_bytes_from_source_blocks_loopback_and_metadata_ssrf() {
    let inputs = AsyncNodeInputs::new();
    for blocked in [
      "http://127.0.0.1:9/secret.wav",
      "http://169.254.169.254/latest/meta-data/",
    ] {
      let err = load_bytes_from_source(blocked, &inputs)
        .await
        .expect_err("loopback/metadata URL must be rejected before connecting");
      let msg = err.to_string().to_ascii_lowercase();
      assert!(
        msg.contains("private")
          || msg.contains("loopback")
          || msg.contains("blocked")
          || msg.contains("denied")
          || msg.contains("ssrf"),
        "expected SSRF policy rejection for {blocked}, got: {err}"
      );
    }
  }

  #[tokio::test]
  async fn load_data_uri_from_str_blocks_loopback_ssrf() {
    let err = load_data_uri_from_str("http://127.0.0.1:9/secret.png")
      .await
      .expect_err("loopback image URL must be rejected by SSRF filter");
    let msg = err.to_string().to_ascii_lowercase();
    assert!(
      msg.contains("private")
        || msg.contains("loopback")
        || msg.contains("blocked")
        || msg.contains("denied")
        || msg.contains("ssrf"),
      "expected SSRF policy rejection, got: {err}"
    );
  }
}
