//! Text-to-Speech (text → audio) provider trait.

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Request to synthesise speech audio from text.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TtsRequest {
  /// Model identifier (e.g. `"step-tts-mini"`, `"eleven-monolingual-v1"`).
  pub model: String,
  /// Text to synthesise. Provider-specific length caps apply (e.g.
  /// StepFun caps at 1000 chars per call).
  pub input: String,
  /// Voice identifier. Format is vendor-specific (StepFun uses
  /// preset names like `"cixingnansheng"`; ElevenLabs uses 20-char
  /// UUIDs).
  pub voice: String,
  /// Audio container format. Common values: `"wav" | "mp3" | "flac"
  /// | "opus"`. Provider-specific supersets are allowed.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub response_format: Option<String>,
  /// Playback speed multiplier (e.g. `1.0` = normal, `0.5` = half).
  /// Range and behaviour are vendor-specific.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub speed: Option<f32>,
  /// Output volume multiplier. Vendor-specific.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub volume: Option<f32>,
  /// Sample rate hint in Hz. Vendor-specific.
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sample_rate: Option<u32>,
  /// Optional natural-language delivery/prosody direction (supported by
  /// Gemini Native TTS and OpenAI `gpt-4o-mini-tts`; safely ignored by
  /// providers whose wire protocol accepts plain transcript text only).
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub instructions: Option<String>,
}

/// Synthesised audio bytes for a [`TtsRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TtsResponse {
  /// Raw audio bytes in the requested format.
  pub audio: Vec<u8>,
  /// MIME type derived from the request's `response_format` —
  /// callers writing to disk can use this without re-parsing.
  pub mime_type: String,
}

/// Provider trait for TTS endpoints.
#[async_trait]
pub trait TtsProvider: Send + Sync {
  /// Short provider identifier, e.g. `"stepfun"` or `"openai"`.
  fn name(&self) -> &str;

  /// Synthesise `request` into audio bytes.
  async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse>;
}
