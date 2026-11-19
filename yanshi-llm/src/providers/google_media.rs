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
  AsrProvider, AsrRequest, AsrResponse, GeneratedImage, Image2ImageProvider, Image2ImageRequest,
  ImageEditProvider, ImageEditRequest, ImageGenerationResponse, Text2ImageProvider,
  Text2ImageRequest, TtsProvider, TtsRequest, TtsResponse,
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
  ///
  /// Retries transient transport failures and HTTP 429 / 5xx responses
  /// (honoring Google's `"retryDelay": "NNs"` hint on RPM quota windows).
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

    const RETRY_DELAYS: [Option<u64>; 5] = [Some(2), Some(6), Some(15), Some(30), None];
    let mut last_err: Option<LLMError> = None;

    for next_delay in RETRY_DELAYS {
      let headers = self.build_headers()?;
      match self
        .client
        .post(&url)
        .headers(headers)
        .json(&body)
        .send()
        .await
      {
        Ok(response) if response.status().is_success() => {
          let text = response.text().await?;
          return parse_generate_content_media_response(&text);
        }
        Ok(response) => {
          let status_code = response.status().as_u16();
          let message = response.text().await.unwrap_or_default();
          let retryable = matches!(status_code, 429 | 500 | 502 | 503 | 504);
          if retryable && let Some(fallback_secs) = next_delay {
            let wait_secs = parse_google_retry_delay_secs(&message).unwrap_or(fallback_secs);
            tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
            last_err = Some(LLMError::HttpError {
              status_code,
              message,
            });
            continue;
          }
          return Err(LLMError::HttpError {
            status_code,
            message,
          });
        }
        Err(err) => {
          if let Some(wait_secs) = next_delay {
            tokio::time::sleep(std::time::Duration::from_secs(wait_secs)).await;
            last_err = Some(err.into());
            continue;
          }
          return Err(err.into());
        }
      }
    }

    Err(last_err.unwrap_or_else(|| LLMError::InternalError {
      message: "Google media request exhausted retry budget".to_string(),
    }))
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
  #[serde(default)]
  text: Option<String>,
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

async fn resolve_source_image(_client: &Client, source_url: &str) -> Result<(String, String)> {
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

  let (bytes, content_type) =
    super::modality::download_public_url_with_cap(source_url, MAX_SOURCE_IMAGE_BYTES).await?;
  let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
  let mime = content_type
    .filter(|m| m.starts_with("image/"))
    .unwrap_or_else(|| {
      mime_from_filename(source_url.split('?').next().unwrap_or(source_url)).to_string()
    });
  Ok((mime, b64))
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
    let mut parts = Vec::with_capacity(2 + request.reference_urls.len());
    parts.push(json!({
      "inlineData": {
        "mimeType": mime_type,
        "data": b64_data,
      }
    }));
    for ref_url in &request.reference_urls {
      if ref_url.trim().is_empty() {
        continue;
      }
      let (ref_mime, ref_b64) = resolve_source_image(&self.client, ref_url).await?;
      parts.push(json!({
        "inlineData": {
          "mimeType": ref_mime,
          "data": ref_b64,
        }
      }));
    }
    parts.push(json!({"text": request.prompt}));
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

const GOOGLE_PREBUILT_VOICES: &[&str] = &[
  "Zephyr",
  "Puck",
  "Charon",
  "Kore",
  "Fenrir",
  "Leda",
  "Orus",
  "Aoede",
  "Callirrhoe",
  "Autonoe",
  "Enceladus",
  "Iapetus",
  "Umbriel",
  "Algieba",
  "Despina",
  "Erinome",
  "Algenib",
  "Rasalgethi",
  "Laomedeia",
  "Achernar",
  "Alnilam",
  "Schedar",
  "Gacrux",
  "Pulcherrima",
  "Achird",
  "Zubenelgenubi",
  "Vindemiatrix",
  "Sadachbia",
  "Sadaltager",
  "Sulafat",
];

pub(crate) fn normalize_google_tts_voice(voice: &str) -> String {
  let trimmed = voice.trim();
  if trimmed.is_empty()
    || trimmed.eq_ignore_ascii_case("auto")
    || trimmed.eq_ignore_ascii_case("default")
  {
    return "Kore".to_string();
  }
  if let Some(&canonical) = GOOGLE_PREBUILT_VOICES
    .iter()
    .find(|v| v.eq_ignore_ascii_case(trimmed))
  {
    return canonical.to_string();
  }
  let lower = trimmed.to_ascii_lowercase();
  if matches!(
    lower.as_str(),
    "ethan" | "dylan" | "cixingnansheng" | "onyx" | "echo" | "ash" | "fable"
  ) || lower.starts_with("male-")
  {
    return "Charon".to_string();
  }
  if matches!(
    lower.as_str(),
    "cherry"
      | "serena"
      | "chelsie"
      | "sunny"
      | "jada"
      | "wenrounvsheng"
      | "alloy"
      | "coral"
      | "nova"
      | "shimmer"
      | "ballad"
      | "sage"
      | "verse"
  ) || lower.starts_with("female-")
  {
    return "Kore".to_string();
  }
  trimmed.to_string()
}

pub(crate) fn parse_google_retry_delay_secs(body: &str) -> Option<u64> {
  let idx = body.find("\"retryDelay\"")?;
  let rest = &body[idx + "\"retryDelay\"".len()..];
  let q1 = rest.find('"')?;
  let inner = &rest[q1 + 1..];
  let q2 = inner.find('"')?;
  let raw = inner[..q2].trim().trim_end_matches('s');
  let parsed: f64 = raw.parse().ok()?;
  if parsed.is_finite() && parsed > 0.0 {
    Some((parsed.ceil() as u64 + 1).clamp(2, 65))
  } else {
    None
  }
}

pub(crate) fn build_google_tts_prompt(input: &str, instructions: Option<&str>) -> String {
  match instructions.map(str::trim).filter(|s| !s.is_empty()) {
    Some(dir) => format!("Say in the following vocal style ({dir}):\n{input}"),
    None => input.to_string(),
  }
}

#[async_trait]
impl TtsProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
    let voice_name = normalize_google_tts_voice(&request.voice);
    let has_instructions = request
      .instructions
      .as_deref()
      .map(str::trim)
      .is_some_and(|s| !s.is_empty());
    let prompt = build_google_tts_prompt(&request.input, request.instructions.as_deref());
    let generation_config = json!({
      "responseModalities": ["AUDIO"],
      "speechConfig": {
        "voiceConfig": {
          "prebuiltVoiceConfig": { "voiceName": voice_name }
        }
      }
    });
    let parts = match self
      .generate_content_media(&request.model, &prompt, generation_config.clone())
      .await
    {
      Ok(parts) => parts,
      Err(LLMError::ResponseParsingError { .. }) if has_instructions => {
        // Fallback to plain narration text if the vocal-style prefix caused
        // Gemini TTS to stop with finishReason: OTHER without audio bytes.
        self
          .generate_content_media(&request.model, &request.input, generation_config)
          .await?
      }
      Err(err) => return Err(err),
    };

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

fn audio_mime_from_filename(filename: &str) -> &'static str {
  let lower = filename.to_ascii_lowercase();
  if lower.ends_with(".mp3") || lower.ends_with(".mpeg") || lower.ends_with(".mpga") {
    "audio/mp3"
  } else if lower.ends_with(".wav") {
    "audio/wav"
  } else if lower.ends_with(".flac") {
    "audio/flac"
  } else if lower.ends_with(".ogg") || lower.ends_with(".opus") {
    "audio/ogg"
  } else if lower.ends_with(".aac") {
    "audio/aac"
  } else if lower.ends_with(".m4a") || lower.ends_with(".mp4") {
    "audio/mp4"
  } else if lower.ends_with(".webm") {
    "audio/webm"
  } else {
    "audio/mp3"
  }
}

pub(crate) fn parse_generate_content_text_response(body: &str) -> Result<String> {
  let parsed: GenerateContentResponse =
    serde_json::from_str(body).map_err(|e| LLMError::ResponseParsingError {
      message: format!("Google generateContent text response JSON parse failed: {e}"),
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

  let text: String = parsed
    .candidates
    .into_iter()
    .filter_map(|c| c.content)
    .flat_map(|c| c.parts)
    .filter_map(|p| p.text)
    .collect::<Vec<_>>()
    .join("");

  if text.trim().is_empty() {
    let reason_note = finish_reason
      .map(|r| format!(" (finishReason: {r})"))
      .unwrap_or_default();
    return Err(LLMError::ResponseParsingError {
      message: format!(
        "Google generateContent ASR response carried no text parts{reason_note}. Body: {body}"
      ),
    });
  }

  Ok(strip_code_fence(text.trim()))
}

fn strip_code_fence(raw: &str) -> String {
  let trimmed = raw.trim();
  if let Some(after_open) = trimmed.strip_prefix("```")
    && let Some((_, inner)) = after_open.split_once('\n')
    && let Some(stripped) = inner.trim_end().strip_suffix("```")
  {
    return stripped.trim().to_string();
  }
  trimmed.to_string()
}

fn build_google_asr_prompt(request: &AsrRequest) -> String {
  let mut instructions = match request.response_format.as_str() {
    "srt" => "Transcribe the spoken audio verbatim into valid SubRip (.srt) subtitle format \
       with sequential cue numbers and `HH:MM:SS,mmm --> HH:MM:SS,mmm` timestamps. \
       Output ONLY the raw .srt content with no markdown code fences or commentary."
      .to_string(),
    "vtt" => "Transcribe the spoken audio verbatim into valid WebVTT (.vtt) format starting \
       with `WEBVTT`. Output ONLY the raw .vtt content with no markdown fences."
      .to_string(),
    _ => "Transcribe the spoken audio verbatim. Output ONLY the transcript text.".to_string(),
  };
  if let Some(lang) = request.language.as_deref().filter(|s| !s.trim().is_empty()) {
    instructions.push_str(&format!(" Primary spoken language: {lang}."));
  }
  if let Some(hint) = request.prompt.as_deref().filter(|s| !s.trim().is_empty()) {
    instructions.push_str(&format!(" Domain terminology / hint: {hint}"));
  }
  instructions
}

#[async_trait]
impl AsrProvider for GoogleMediaProvider {
  fn name(&self) -> &str {
    "google"
  }

  async fn transcribe(&self, request: AsrRequest) -> Result<AsrResponse> {
    let effective_model = crate::registry::ModelRegistry::global()
      .get_model(&request.model)
      .ok()
      .and_then(|cfg| cfg.model_id.clone())
      .unwrap_or_else(|| request.model.clone());

    let url = self
      .endpoint
      .model_action_url(&effective_model, "generateContent", None);
    let mime_type = audio_mime_from_filename(&request.filename);
    let b64_audio = base64::engine::general_purpose::STANDARD.encode(&request.audio_data);
    let prompt = build_google_asr_prompt(&request);

    let mut gen_cfg = serde_json::Map::new();
    gen_cfg.insert(
      "temperature".to_string(),
      json!(request.temperature.unwrap_or(0.0)),
    );

    let body = json!({
      "contents": [{
        "role": "user",
        "parts": [
          {
            "inlineData": {
              "mimeType": mime_type,
              "data": b64_audio,
            }
          },
          { "text": prompt }
        ]
      }],
      "generationConfig": Value::Object(gen_cfg),
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

    let raw_body = response.text().await?;
    let text = parse_generate_content_text_response(&raw_body)?;
    if request.response_format == "json" {
      let meta = json!({ "text": text, "provider": "google", "model": effective_model });
      return Ok(AsrResponse {
        text,
        metadata: Some(meta),
      });
    }
    Ok(AsrResponse {
      text,
      metadata: None,
    })
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

  #[test]
  fn parse_text_response_strips_srt_code_fences() {
    let body = json!({
      "candidates": [{
        "content": {
          "parts": [{
            "text": "```srt\n1\n00:00:00,000 --> 00:00:02,500\nHello Yanshi\n```"
          }]
        }
      }]
    })
    .to_string();
    let parsed = parse_generate_content_text_response(&body).expect("parse ok");
    assert_eq!(parsed, "1\n00:00:00,000 --> 00:00:02,500\nHello Yanshi");
  }

  #[test]
  fn google_tts_voice_and_director_prompt_normalize_cleanly() {
    assert_eq!(normalize_google_tts_voice("kore"), "Kore");
    assert_eq!(normalize_google_tts_voice("auto"), "Kore");
    assert_eq!(normalize_google_tts_voice("Cherry"), "Kore");
    assert_eq!(normalize_google_tts_voice("Ethan"), "Charon");
    assert_eq!(normalize_google_tts_voice("Puck"), "Puck");
    assert_eq!(build_google_tts_prompt("你好", None), "你好");
    assert_eq!(build_google_tts_prompt("你好", Some("   ")), "你好");
    let styled = build_google_tts_prompt("请看左侧代码", Some("资深架构师自然演讲语气"));
    assert!(styled.contains("资深架构师自然演讲语气"));
    assert!(styled.ends_with("\n请看左侧代码"));
    assert_eq!(
      parse_google_retry_delay_secs(r#"{"error":{"details":[{"retryDelay":"22.4s"}]}}"#),
      Some(24)
    );
    assert_eq!(parse_google_retry_delay_secs("{}"), None);
  }

  #[tokio::test]
  async fn image2image_sends_source_and_multiple_reference_images_in_order() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
      .await
      .expect("bind");
    let addr = listener.local_addr().expect("addr");
    let server = tokio::spawn(async move {
      let (mut stream, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 16384];
      let n = stream.read(&mut buf).await.expect("read");
      let req_text = String::from_utf8_lossy(&buf[..n]).to_string();
      let body_start = req_text.find("\r\n\r\n").expect("headers") + 4;
      let payload: Value = serde_json::from_str(&req_text[body_start..]).expect("json");
      let parts = payload["contents"][0]["parts"].as_array().expect("parts");
      assert_eq!(parts.len(), 4);
      assert_eq!(parts[0]["inlineData"]["data"], "c291cmNl");
      assert_eq!(parts[1]["inlineData"]["data"], "cmVmMQ==");
      assert_eq!(parts[2]["inlineData"]["data"], "cmVmMg==");
      assert_eq!(parts[3]["text"], "Episode 3 rooftop scene");

      let resp_body = json!({
        "candidates": [{
          "content": {
            "parts": [{
              "inlineData": { "mimeType": "image/png", "data": "b3V0" }
            }]
          }
        }]
      })
      .to_string();
      let http_resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        resp_body.len(),
        resp_body
      );
      stream
        .write_all(http_resp.as_bytes())
        .await
        .expect("write response");
    });

    let no_proxy_client = reqwest::Client::builder()
      .no_proxy()
      .build()
      .expect("test client");
    let provider =
      GoogleMediaProvider::with_client(no_proxy_client, "test-key", Some(format!("http://{addr}")))
        .expect("provider");

    let out = provider
      .transform(Image2ImageRequest {
        model: "gemini-3.1-flash-image".to_string(),
        prompt: "Episode 3 rooftop scene".to_string(),
        source_url: "data:image/png;base64,c291cmNl".to_string(),
        reference_urls: vec![
          "data:image/png;base64,cmVmMQ==".to_string(),
          "data:image/jpeg;base64,cmVmMg==".to_string(),
        ],
        source_weight: 0.85,
        size: Some("16:9".to_string()),
        n: Some(1),
        response_format: None,
        seed: None,
        steps: None,
        cfg_scale: None,
      })
      .await
      .expect("transform");

    assert_eq!(out.images.len(), 1);
    assert_eq!(out.images[0].b64_json.as_deref(), Some("b3V0"));
    server.await.expect("server join");
  }
}
