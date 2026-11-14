//! Google Veo text-to-video client (Gemini API, not Vertex AI).
//!
//! Authenticates identically to [`super::google::GoogleProvider`]
//! (`x-goog-api-key` header against `generativelanguage.googleapis.com`)
//! but speaks the long-running-operation shape Veo uses instead of the
//! synchronous `generateContent` shape chat uses:
//!
//! - `submit`: `POST /v1beta/models/{model}:predictLongRunning` →
//!   `{"name": "operations/..."}`.
//! - `poll`: `GET /v1beta/{operation name}` → `{"done": bool, ...}`,
//!   either `response.generateVideoResponse.generatedSamples[..].video.uri`
//!   on success or an `error` object on failure.
//!
//! The returned video `uri` may require the same API key to fetch bytes
//! from later (it is not always a plain public URL) — downloading and
//! persisting video bytes is out of scope here, same as how the image
//! modality traits only ever hand back the vendor's URL/base64 as-is.

use super::google_endpoint::{GoogleEndpoint, strip_model_prefix};
use super::modality::{
  GeneratedVideo, Text2VideoProvider, Text2VideoRequest, VideoGenerationResponse,
  VideoGenerationStatus, VideoGenerationTask,
};
use crate::{LLMError, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};

pub struct GoogleVeoClient {
  client: Client,
  api_key: String,
  endpoint: GoogleEndpoint,
}

impl GoogleVeoClient {
  pub fn new(api_key: &str, base_url: Option<String>) -> Result<Self> {
    Self::with_client(super::default_http_client()?, api_key, base_url)
  }

  /// Construct with a caller-supplied [`reqwest::Client`]. See
  /// [`crate::providers::OpenAIProvider::with_client`] for the rationale.
  pub fn with_client(client: Client, api_key: &str, base_url: Option<String>) -> Result<Self> {
    if api_key.is_empty() {
      return Err(LLMError::MissingApiKey {
        provider: "google".to_string(),
      });
    }

    let endpoint = GoogleEndpoint::new(api_key, base_url);

    Ok(Self {
      client,
      api_key: api_key.to_string(),
      endpoint,
    })
  }

  fn build_headers(&self) -> Result<reqwest::header::HeaderMap> {
    self.endpoint.build_headers(&self.api_key)
  }

  fn build_request_body(&self, request: &Text2VideoRequest) -> Value {
    let mut instance = serde_json::Map::new();
    instance.insert("prompt".to_string(), Value::String(request.prompt.clone()));
    if let Some(ref image) = request.image {
      instance.insert(
        "image".to_string(),
        json!({
          "bytesBase64Encoded": image.bytes_base64_encoded,
          "mimeType": image.mime_type,
        }),
      );
    }

    let mut parameters = serde_json::Map::new();
    if let Some(ref aspect_ratio) = request.aspect_ratio {
      parameters.insert(
        "aspectRatio".to_string(),
        Value::String(aspect_ratio.clone()),
      );
    }
    if let Some(duration_seconds) = request.duration_seconds {
      parameters.insert(
        "durationSeconds".to_string(),
        Value::Number(duration_seconds.into()),
      );
    }
    if let Some(ref resolution) = request.resolution {
      parameters.insert("resolution".to_string(), Value::String(resolution.clone()));
    }
    if let Some(seed) = request.seed {
      parameters.insert("seed".to_string(), Value::Number(seed.into()));
    }
    if let Some(Value::Object(extra)) = &request.extra {
      parameters.extend(extra.clone());
    }

    json!({
      "instances": [instance],
      "parameters": Value::Object(parameters),
    })
  }
}

/// `POST .../{model}:predictLongRunning` response shape.
#[derive(Debug, Deserialize)]
struct SubmitResponse {
  name: String,
}

/// `GET .../{operation name}` (Gemini API) or `POST ...:fetchPredictOperation`
/// (Vertex AI) response shape. `response`/`error` are mutually exclusive and
/// only present once `done` is `true`.
#[derive(Debug, Deserialize)]
struct OperationResponse {
  #[serde(default)]
  done: bool,
  #[serde(default)]
  response: Option<Value>,
  #[serde(default)]
  error: Option<OperationError>,
}

#[derive(Debug, Deserialize)]
struct OperationError {
  message: String,
}

fn push_video_item(videos: &mut Vec<GeneratedVideo>, item: &Value) {
  let b64 = item
    .get("bytesBase64Encoded")
    .and_then(Value::as_str)
    .map(ToString::to_string);
  let url = item
    .get("uri")
    .or_else(|| item.get("gcsUri"))
    .and_then(Value::as_str)
    .map(ToString::to_string);
  if b64.is_some() || url.is_some() {
    videos.push(GeneratedVideo {
      url,
      b64_data: b64,
      duration_seconds: None,
    });
  }
}

/// Parse an [`OperationResponse`] into the trait's [`VideoGenerationStatus`].
/// Supports both Gemini Developer API (`generateVideoResponse.generatedSamples`)
/// and Vertex AI (`videos`, `generatedVideos`, `predictions`).
fn parse_operation_response(op: OperationResponse) -> Result<VideoGenerationStatus> {
  if let Some(error) = op.error {
    return Ok(VideoGenerationStatus::Failed {
      message: error.message,
    });
  }
  if !op.done {
    return Ok(VideoGenerationStatus::Pending);
  }
  let response = op.response.ok_or_else(|| LLMError::ResponseParsingError {
    message: "Veo operation marked done but carried neither `response` nor `error`".to_string(),
  })?;

  let mut videos = Vec::new();
  if let Some(samples) = response
    .pointer("/generateVideoResponse/generatedSamples")
    .and_then(Value::as_array)
  {
    for sample in samples {
      if let Some(video) = sample.get("video") {
        push_video_item(&mut videos, video);
      }
    }
  }
  if let Some(items) = response.get("videos").and_then(Value::as_array) {
    for item in items {
      push_video_item(&mut videos, item);
    }
  }
  if let Some(items) = response.get("generatedVideos").and_then(Value::as_array) {
    for item in items {
      if let Some(video) = item.get("video") {
        push_video_item(&mut videos, video);
      } else {
        push_video_item(&mut videos, item);
      }
    }
  }
  if let Some(items) = response.get("predictions").and_then(Value::as_array) {
    for item in items {
      push_video_item(&mut videos, item);
    }
  }

  Ok(VideoGenerationStatus::Completed(VideoGenerationResponse {
    created: 0,
    videos,
    metadata: Some(response),
  }))
}

fn parse_vertex_op_location_and_model(task_id: &str) -> (Option<&str>, Option<&str>) {
  let parts: Vec<&str> = task_id.split('/').collect();
  let location = parts.windows(2).find(|w| w[0] == "locations").map(|w| w[1]);
  let model = parts.windows(2).find(|w| w[0] == "models").map(|w| w[1]);
  (location, model)
}

#[async_trait]
impl Text2VideoProvider for GoogleVeoClient {
  fn name(&self) -> &str {
    "google"
  }

  async fn submit(&self, request: Text2VideoRequest) -> Result<VideoGenerationTask> {
    let url = self
      .endpoint
      .model_action_url(&request.model, "predictLongRunning", None);
    let body = self.build_request_body(&request);

    let response = self
      .client
      .post(&url)
      .headers(self.build_headers()?)
      .json(&body)
      .send()
      .await?;

    if !response.status().is_success() {
      let status_code = response.status().as_u16();
      let error_text = response.text().await.unwrap_or_default();
      return Err(LLMError::HttpError {
        status_code,
        message: error_text,
      });
    }

    let submitted: SubmitResponse = response.json().await?;
    Ok(VideoGenerationTask {
      task_id: submitted.name,
      metadata: Some(json!({
        "model": strip_model_prefix(&request.model),
      })),
    })
  }

  async fn poll(&self, task: &VideoGenerationTask) -> Result<VideoGenerationStatus> {
    let response = if self.endpoint.is_vertex() || task.task_id.starts_with("projects/") {
      let (op_loc, op_model) = parse_vertex_op_location_and_model(&task.task_id);
      let meta_model = task
        .metadata
        .as_ref()
        .and_then(|m| m.get("model"))
        .and_then(Value::as_str);
      let model = op_model
        .or(meta_model)
        .ok_or_else(|| LLMError::ConfigurationError {
          message: format!(
            "Cannot determine Vertex AI model for Veo operation '{}'",
            task.task_id
          ),
        })?;
      let poll_url =
        self
          .endpoint
          .model_action_url_with_location(model, "fetchPredictOperation", None, op_loc);
      self
        .client
        .post(&poll_url)
        .headers(self.build_headers()?)
        .json(&json!({ "operationName": task.task_id }))
        .send()
        .await?
    } else {
      let url = format!("{}/v1beta/{}", self.endpoint.base_url, task.task_id);
      self
        .client
        .get(&url)
        .headers(self.build_headers()?)
        .send()
        .await?
    };

    if !response.status().is_success() {
      let status_code = response.status().as_u16();
      let error_text = response.text().await.unwrap_or_default();
      return Err(LLMError::HttpError {
        status_code,
        message: error_text,
      });
    }

    let op: OperationResponse = response.json().await?;
    parse_operation_response(op)
  }

  /// Veo's `video.uri` is served by the same API host and needs the
  /// same `x-goog-api-key` header; a plain GET returns 403.
  async fn fetch_video_bytes(&self, video: &GeneratedVideo, max_bytes: u64) -> Result<Vec<u8>> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    if let Some(b64) = &video.b64_data {
      return STANDARD
        .decode(b64)
        .map_err(|err| LLMError::ResponseParsingError {
          message: format!("video b64_data is not valid base64: {err}"),
        });
    }
    let url = video
      .url
      .as_deref()
      .ok_or_else(|| LLMError::ResponseParsingError {
        message: "generated video carries neither a url nor b64_data".to_string(),
      })?;
    let mut headers = self.build_headers()?;
    headers.remove(reqwest::header::CONTENT_TYPE);
    super::modality::download_with_cap(&self.client, url, headers, max_bytes).await
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn client_creation_requires_api_key() {
    assert!(GoogleVeoClient::new("", None).is_err());
    assert!(GoogleVeoClient::new("test-key", None).is_ok());
  }

  #[test]
  fn client_creation_defaults_base_url() {
    let client = GoogleVeoClient::new("test-key", None).unwrap();
    assert_eq!(
      client.endpoint.base_url,
      "https://generativelanguage.googleapis.com"
    );
  }

  #[test]
  fn parses_pending_operation() {
    let op: OperationResponse = serde_json::from_value(json!({
      "name": "operations/abc123",
      "done": false,
    }))
    .unwrap();
    let status = parse_operation_response(op).unwrap();
    assert!(matches!(status, VideoGenerationStatus::Pending));
  }

  #[test]
  fn parses_completed_operation_and_extracts_video_url() {
    let op: OperationResponse = serde_json::from_value(json!({
      "name": "operations/abc123",
      "done": true,
      "response": {
        "generateVideoResponse": {
          "generatedSamples": [
            { "video": { "uri": "https://example.com/video.mp4" } }
          ]
        }
      }
    }))
    .unwrap();
    let status = parse_operation_response(op).unwrap();
    match status {
      VideoGenerationStatus::Completed(response) => {
        assert_eq!(response.videos.len(), 1);
        assert_eq!(
          response.videos[0].url.as_deref(),
          Some("https://example.com/video.mp4")
        );
      }
      other => panic!("expected Completed, got {other:?}"),
    }
  }

  #[test]
  fn parses_failed_operation() {
    let op: OperationResponse = serde_json::from_value(json!({
      "name": "operations/abc123",
      "done": true,
      "error": { "code": 3, "message": "prompt violates content policy" }
    }))
    .unwrap();
    let status = parse_operation_response(op).unwrap();
    match status {
      VideoGenerationStatus::Failed { message } => {
        assert_eq!(message, "prompt violates content policy");
      }
      other => panic!("expected Failed, got {other:?}"),
    }
  }

  #[test]
  fn done_without_response_or_error_is_a_parsing_error() {
    let op: OperationResponse = serde_json::from_value(json!({
      "name": "operations/abc123",
      "done": true,
    }))
    .unwrap();
    assert!(parse_operation_response(op).is_err());
  }

  #[test]
  fn request_body_maps_common_fields_and_merges_extra() {
    let client = GoogleVeoClient::new("test-key", None).unwrap();
    let request = Text2VideoRequest {
      model: "veo-3.1-generate-preview".to_string(),
      prompt: "a cat playing piano".to_string(),
      image: Some(crate::VideoInputImage {
        bytes_base64_encoded: "aGVsbG8=".to_string(),
        mime_type: "image/png".to_string(),
      }),
      aspect_ratio: Some("16:9".to_string()),
      duration_seconds: Some(8),
      resolution: Some("1080p".to_string()),
      seed: Some(42),
      extra: Some(json!({ "personGeneration": "allow_adult" })),
    };
    let body = client.build_request_body(&request);
    assert_eq!(body["instances"][0]["prompt"], "a cat playing piano");
    assert_eq!(
      body["instances"][0]["image"]["bytesBase64Encoded"],
      "aGVsbG8="
    );
    assert_eq!(body["instances"][0]["image"]["mimeType"], "image/png");
    assert_eq!(body["parameters"]["aspectRatio"], "16:9");
    assert_eq!(body["parameters"]["durationSeconds"], 8);
    assert_eq!(body["parameters"]["resolution"], "1080p");
    assert_eq!(body["parameters"]["seed"], 42);
    assert_eq!(body["parameters"]["personGeneration"], "allow_adult");
  }

  #[test]
  fn parses_vertex_completed_operation_with_inline_videos() {
    let op: OperationResponse = serde_json::from_value(json!({
      "done": true,
      "response": {
        "videos": [
          { "bytesBase64Encoded": "AAAA", "mimeType": "video/mp4" }
        ]
      }
    }))
    .unwrap();
    let status = parse_operation_response(op).unwrap();
    match status {
      VideoGenerationStatus::Completed(response) => {
        assert_eq!(response.videos.len(), 1);
        assert_eq!(response.videos[0].b64_data.as_deref(), Some("AAAA"));
      }
      other => panic!("expected Completed, got {other:?}"),
    }
  }

  #[test]
  fn parses_vertex_op_location_and_model() {
    let op_name = "projects/340214391376/locations/global/publishers/google/models/veo-3.1-fast-generate-001/operations/op-123";
    let (loc, model) = parse_vertex_op_location_and_model(op_name);
    assert_eq!(loc, Some("global"));
    assert_eq!(model, Some("veo-3.1-fast-generate-001"));
  }
}
