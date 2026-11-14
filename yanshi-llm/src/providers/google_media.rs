//! Google text-to-image + TTS provider, both via `generateContent`.
//!
//! Unlike the deprecated Imagen `:predict` endpoint (shutting down
//! 2026-08-17 per Google's own docs — not implemented here) and the
//! beta Interactions API (`/v1beta/interactions`, a different
//! request/response shape Google's own docs say to avoid for "stable
//! production deployments"), both modalities here go through the same
//! `POST {base_url}/v1beta/models/{model}:generateContent` endpoint
//! `GoogleProvider` (chat) already speaks — just with
//! `generationConfig.responseModalities` set to `["IMAGE"]` or
//! `["AUDIO"]`. The response carries generated media as base64
//! `inlineData` parts (`candidates[0].content.parts[].inlineData.{data,
//! mimeType}`), which this module decodes into the modality traits'
//! `Vec<u8>`/URL-or-b64_json shapes.
//!
//! No `AsrProvider` here: Gemini has no dedicated transcription REST
//! endpoint — audio input goes through `generateContent` as multimodal
//! chat content, not an ASR-shaped request/response.

use super::google_endpoint::GoogleEndpoint;
use super::modality::{
  GeneratedImage, Image2ImageProvider, Image2ImageRequest, ImageEditProvider, ImageEditRequest,
  ImageGenerationResponse, Text2ImageProvider, Text2ImageRequest, TtsProvider, TtsRequest,
  TtsResponse,
};
use crate::{LLMError, Result};
use async_trait::async_trait;
use base64::Engine;
use reqwest::Client;
use serde::Deserialize;
use serde_json::{Value, json};

const MAX_SOURCE_IMAGE_BYTES: u64 = 20 * 1024 * 1024;

pub struct GoogleMediaProvider {
  client: Client,
  api_key: String,
  endpoint: GoogleEndpoint,
}

impl std::fmt::Debug for GoogleMediaProvider {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("GoogleMediaProvider")
      .field("base_url", &self.endpoint.base_url)
      .field("is_vertex", &self.endpoint.is_vertex())
      .field("api_key", &"<redacted>")
      .finish()
  }
}

impl GoogleMediaProvider {
  pub fn new(api_key: &str, base_url: Option<String>) -> Result<Self> {
    Self::with_client(super::default_http_client()?, api_key, base_url)
  }

  /// Construct with a caller-supplied [`reqwest::Client`]. Mirrors
  /// `GoogleVeoClient::with_client`.
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

  /// Shared core for `generateContent`-backed media modalities: POST
  /// `generateContent` with the given `parts` and `generation_config`,
  /// then collect every response part carrying `inlineData`.
  async fn generate_content_parts(
    &self,
    model: &str,
    parts: Vec<Value>,
    generation_config: Value,
  ) -> Result<Vec<InlineDataPart>> {
    let url = self
      .endpoint
      .model_action_url(model, "generateContent", None);
    let body = json!({
      "contents": [{"role": "user", "parts": parts}],
      "generationConfig": generation_config,
    });

    let response = self
      .client
      .post(&url)
      .headers(self.build_headers()?)
      .json(&body)
      .send()
      .await?;

    if !response.status().is_success() {
      let status_code = response.status().as_u16();
      let message = response.text().await.unwrap_or_default();
      return Err(LLMError::HttpError {
        status_code,
        message,
      });
    }

    parse_generate_content_media_response(&response.text().await?)
  }

  async fn generate_content_media(
    &self,
    model: &str,
    text: &str,
    generation_config: Value,
  ) -> Result<Vec<InlineDataPart>> {
    self
      .generate_content_parts(model, vec![json!({"text": text})], generation_config)
      .await
  }
}

/// A single `inlineData` part from a `generateContent` response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InlineDataPart {
  /// Base64-encoded media bytes, as returned by the API (not yet decoded).
  pub(crate) data: String,
  pub(crate) mime_type: String,
}

#[derive(Debug, Deserialize)]
struct GenerateContentResponse {
  #[serde(default)]
  candidates: Vec<Candidate>,
  #[serde(default, rename = "promptFeedback")]
  prompt_feedback: Option<PromptFeedback>,
}

#[derive(Debug, Deserialize)]
struct PromptFeedback {
  #[serde(default, rename = "blockReason")]
  block_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
  #[serde(default)]
  content: Option<CandidateContent>,
  #[serde(default, rename = "finishReason")]
  finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
  #[serde(default)]
  parts: Vec<CandidatePart>,
}

#[derive(Debug, Deserialize)]
struct CandidatePart {
  #[serde(default, rename = "inlineData", alias = "inline_data")]
  inline_data: Option<InlineData>,
}

#[derive(Debug, Deserialize)]
struct InlineData {
  data: String,
  #[serde(rename = "mimeType", alias = "mime_type")]
  mime_type: String,
}

/// Parse a `generateContent` response body, collecting every part that
/// carries `inlineData` (or `inline_data`). Pure function, independently
/// unit-tested.
pub(crate) fn parse_generate_content_media_response(body: &str) -> Result<Vec<InlineDataPart>> {
  let parsed: GenerateContentResponse =
    serde_json::from_str(body).map_err(|e| LLMError::ResponseParsingError {
      message: format!("Google generateContent media response JSON parse failed: {e}"),
    })?;

  if let Some(reason) = parsed
    .prompt_feedback
    .as_ref()
    .and_then(|f| f.block_reason.as_deref())
  {
    return Err(LLMError::ResponseParsingError {
      message: format!("Google generateContent request blocked: {reason}"),
    });
  }

  let finish_reason = parsed
    .candidates
    .first()
    .and_then(|c| c.finish_reason.clone());

  let parts: Vec<InlineDataPart> = parsed
    .candidates
    .into_iter()
    .filter_map(|c| c.content)
    .flat_map(|c| c.parts)
    .filter_map(|p| p.inline_data)
    .map(|inline| InlineDataPart {
      data: inline.data,
      mime_type: inline.mime_type,
    })
    .collect();

  if parts.is_empty() {
    let reason_note = finish_reason
      .map(|r| format!(" (finishReason: {r})"))
      .unwrap_or_default();
    return Err(LLMError::ResponseParsingError {
      message: format!(
        "Google generateContent response carried no inlineData media parts{reason_note}. Body: {body}"
      ),
    });
  }

  Ok(parts)
}

fn size_to_aspect_ratio(size: Option<&str>) -> Option<&'static str> {
  match size?.trim() {
    "16:9" | "1920x1080" | "1280x720" | "1344x768" | "1792x1024" => Some("16:9"),
    "9:16" | "1080x1920" | "720x1280" | "768x1344" | "1024x1792" => Some("9:16"),
    "1:1" | "1024x1024" | "512x512" | "256x256" => Some("1:1"),
    "4:3" | "1024x768" | "1152x896" => Some("4:3"),
    "3:4" | "768x1024" | "896x1152" => Some("3:4"),
    _ => None,
  }
}

fn image_generation_config(size: Option<&str>) -> Value {
  let mut cfg = serde_json::Map::new();
  cfg.insert("responseModalities".to_string(), json!(["TEXT", "IMAGE"]));
  if let Some(ratio) = size_to_aspect_ratio(size) {
    cfg.insert("imageConfig".to_string(), json!({ "aspectRatio": ratio }));
  }
  Value::Object(cfg)
}

fn parts_to_image_response(parts: Vec<InlineDataPart>) -> ImageGenerationResponse {
  let images = parts
    .into_iter()
    .map(|part| GeneratedImage {
      url: None,
      b64_json: Some(part.data),
      seed: None,
    })
    .collect();

  ImageGenerationResponse {
    created: 0,
    images,
    metadata: None,
  }
}

fn mime_from_filename(filename: &str) -> &'static str {
  let lower = filename.to_ascii_lowercase();
  if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
    "image/jpeg"
  } else if lower.ends_with(".webp") {
    "image/webp"
  } else if lower.ends_with(".gif") {
    "image/gif"
  } else {
    "image/png"
  }
}

async fn resolve_source_image(client: &Client, source_url: &str) -> Result<(String, String)> {
  if let Some(rest) = source_url.strip_prefix("data:") {
    let (header, data) =
      rest
        .split_once(";base64,")
        .ok_or_else(|| LLMError::ConfigurationError {
          message: "data: URI for Google image-to-image must use ;base64,".to_string(),
        })?;
    let mime = if header.is_empty() {
      "image/png"
    } else {
      header
    };
    return Ok((mime.to_string(), data.to_string()));
  }

  let bytes = super::modality::download_with_cap(
    client,
    source_url,
    reqwest::header::HeaderMap::new(),
    MAX_SOURCE_IMAGE_BYTES,
  )
  .await?;
  let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
  let mime = mime_from_filename(source_url.split('?').next().unwrap_or(source_url));
  Ok((mime.to_string(), b64))
}

#[async_trait]
impl Text2ImageProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn generate(&self, request: Text2ImageRequest) -> Result<ImageGenerationResponse> {
    let generation_config = image_generation_config(request.size.as_deref());
    let parts = self
      .generate_content_media(&request.model, &request.prompt, generation_config)
      .await?;
    Ok(parts_to_image_response(parts))
  }
}

#[async_trait]
impl Image2ImageProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn transform(&self, request: Image2ImageRequest) -> Result<ImageGenerationResponse> {
    let (mime_type, b64_data) = resolve_source_image(&self.client, &request.source_url).await?;
    let parts = vec![
      json!({
        "inlineData": {
          "mimeType": mime_type,
          "data": b64_data,
        }
      }),
      json!({"text": request.prompt}),
    ];
    let generation_config = image_generation_config(request.size.as_deref());
    let out_parts = self
      .generate_content_parts(&request.model, parts, generation_config)
      .await?;
    Ok(parts_to_image_response(out_parts))
  }
}

#[async_trait]
impl ImageEditProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn edit(&self, request: ImageEditRequest) -> Result<ImageGenerationResponse> {
    let mime_type = mime_from_filename(&request.image_filename);
    let b64_data = base64::engine::general_purpose::STANDARD.encode(&request.image_data);
    let parts = vec![
      json!({
        "inlineData": {
          "mimeType": mime_type,
          "data": b64_data,
        }
      }),
      json!({"text": request.prompt}),
    ];
    let generation_config = image_generation_config(request.size.as_deref());
    let out_parts = self
      .generate_content_parts(&request.model, parts, generation_config)
      .await?;
    Ok(parts_to_image_response(out_parts))
  }
}

#[async_trait]
impl TtsProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
    let generation_config = json!({
      "responseModalities": ["AUDIO"],
      "speechConfig": {
        "voiceConfig": {
          "prebuiltVoiceConfig": { "voiceName": request.voice }
        }
      }
    });
    let parts = self
      .generate_content_media(&request.model, &request.input, generation_config)
      .await?;

    let first = parts
      .into_iter()
      .next()
      .ok_or_else(|| LLMError::ResponseParsingError {
        message: "Google generateContent response carried no inlineData media parts".to_string(),
      })?;

    let raw_audio = base64::engine::general_purpose::STANDARD
      .decode(&first.data)
      .map_err(|e| LLMError::ResponseParsingError {
        message: format!("Google TTS response inlineData.data was not valid base64: {e}"),
      })?;

    let (audio, mime_type) = normalize_tts_audio(raw_audio, first.mime_type);

    Ok(TtsResponse { audio, mime_type })
  }
}

/// Gemini TTS returns headerless 16-bit signed LE mono PCM labeled
/// `audio/L16;codec=pcm;rate=24000` (or `audio/pcm`). Wrapping it in a
/// standard 44-byte RIFF/WAVE header makes downstream consumers (`ffprobe`,
/// `ffmpeg`, `wav_duration_seconds`, media players) recognize it directly.
fn normalize_tts_audio(raw_audio: Vec<u8>, mime_type: String) -> (Vec<u8>, String) {
  let lower = mime_type.to_ascii_lowercase();
  if !lower.starts_with("audio/l16") && !lower.starts_with("audio/pcm") {
    return (raw_audio, mime_type);
  }
  if raw_audio.starts_with(b"RIFF") {
    return (raw_audio, "audio/wav".to_string());
  }

  let sample_rate = lower
    .split(';')
    .filter_map(|seg| seg.trim().strip_prefix("rate="))
    .find_map(|r| r.parse::<u32>().ok())
    .unwrap_or(24_000);
  let channels: u16 = 1;
  let bits_per_sample: u16 = 16;
  let byte_rate = sample_rate * u32::from(channels) * u32::from(bits_per_sample / 8);
  let block_align = channels * (bits_per_sample / 8);
  let data_len = u32::try_from(raw_audio.len()).unwrap_or(u32::MAX);
  let riff_chunk_size = 36u32.saturating_add(data_len);

  let mut wav = Vec::with_capacity(44 + raw_audio.len());
  wav.extend_from_slice(b"RIFF");
  wav.extend_from_slice(&riff_chunk_size.to_le_bytes());
  wav.extend_from_slice(b"WAVEfmt ");
  wav.extend_from_slice(&16u32.to_le_bytes());
  wav.extend_from_slice(&1u16.to_le_bytes());
  wav.extend_from_slice(&channels.to_le_bytes());
  wav.extend_from_slice(&sample_rate.to_le_bytes());
  wav.extend_from_slice(&byte_rate.to_le_bytes());
  wav.extend_from_slice(&block_align.to_le_bytes());
  wav.extend_from_slice(&bits_per_sample.to_le_bytes());
  wav.extend_from_slice(b"data");
  wav.extend_from_slice(&data_len.to_le_bytes());
  wav.extend_from_slice(&raw_audio);

  (wav, "audio/wav".to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_api_key_is_rejected_at_construction() {
    let err = GoogleMediaProvider::new("", None).unwrap_err();
    assert!(matches!(err, LLMError::MissingApiKey { ref provider } if provider == "google"));
  }

  #[test]
  fn parse_response_collects_single_inline_data_part() {
    let body = json!({
      "candidates": [{
        "content": {
          "parts": [{
            "inlineData": { "data": "aGVsbG8=", "mimeType": "audio/pcm" }
          }]
        }
      }]
    })
    .to_string();
    let parts = parse_generate_content_media_response(&body).expect("parse ok");
    assert_eq!(parts.len(), 1);
    assert_eq!(parts[0].data, "aGVsbG8=");
    assert_eq!(parts[0].mime_type, "audio/pcm");
  }

  #[test]
  fn parse_response_collects_multiple_inline_data_parts() {
    let body = json!({
      "candidates": [{
        "content": {
          "parts": [
            { "inlineData": { "data": "aGVsbG8=", "mimeType": "image/png" } },
            { "inlineData": { "data": "d29ybGQ=", "mimeType": "image/png" } }
          ]
        }
      }]
    })
    .to_string();
    let parts = parse_generate_content_media_response(&body).expect("parse ok");
    assert_eq!(parts.len(), 2);
    assert_eq!(parts[1].data, "d29ybGQ=");
  }

  #[test]
  fn parse_response_with_no_inline_data_returns_typed_error() {
    let body = json!({
      "candidates": [{
        "content": { "parts": [{ "text": "no media here" }] }
      }]
    })
    .to_string();
    let err = parse_generate_content_media_response(&body).unwrap_err();
    assert!(err.to_string().contains("no inlineData media parts"));
  }

  #[test]
  fn parse_response_with_invalid_json_returns_typed_error() {
    let err = parse_generate_content_media_response("{not json").unwrap_err();
    assert!(err.to_string().contains("JSON parse failed"));
  }

  #[test]
  fn parse_response_with_empty_candidates_returns_typed_error() {
    let body = json!({ "candidates": [] }).to_string();
    let err = parse_generate_content_media_response(&body).unwrap_err();
    assert!(err.to_string().contains("no inlineData media parts"));
  }

  #[test]
  fn normalize_tts_audio_wraps_l16_pcm_with_riff_wav_header() {
    let pcm = vec![0u8; 48_000];
    let (wav, mime) = normalize_tts_audio(pcm, "audio/L16;codec=pcm;rate=24000".to_string());
    assert_eq!(mime, "audio/wav");
    assert_eq!(wav.len(), 44 + 48_000);
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    let sample_rate = u32::from_le_bytes([wav[24], wav[25], wav[26], wav[27]]);
    assert_eq!(sample_rate, 24_000);
  }
}
