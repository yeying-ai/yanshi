//! DeepSeek native speech modality provider (`POST /v1/tools/tts` & `POST /v1/tools/asr`).
//!
//! DeepSeek exposes TTS and ASR under `/v1/tools/*` using the same
//! `DEEPSEEK_API_KEY` as chat completions. Because the chat registry entry
//! defaults `base_url` to `https://api.deepseek.com/v1`, [`deepseek_tools_url`]
//! normalizes both `https://api.deepseek.com` and `https://api.deepseek.com/v1`
//! onto `{root}/v1/tools/{op}`.

use crate::{
  LLMError, Result,
  providers::modality::{
    AsrProvider, AsrRequest, AsrResponse, TtsProvider, TtsRequest, TtsResponse,
  },
};
use async_trait::async_trait;
use reqwest::{
  Client,
  multipart::{Form, Part},
};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

const DEFAULT_DEEPSEEK_BASE_URL: &str = "https://api.deepseek.com/v1";
const MAX_TTS_AUDIO_BYTES: u64 = 64 * 1024 * 1024;
const FFMPEG_TRANSCODE_TIMEOUT_SECS: u64 = 60;

pub struct DeepSeekMediaProvider {
  client: Client,
  api_key: String,
  base_url: String,
}

impl std::fmt::Debug for DeepSeekMediaProvider {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DeepSeekMediaProvider")
      .field("base_url", &self.base_url)
      .field("api_key", &"<redacted>")
      .finish()
  }
}

impl DeepSeekMediaProvider {
  pub fn new(api_key: &str, base_url: Option<String>) -> Result<Self> {
    Self::with_client(super::default_http_client()?, api_key, base_url)
  }

  /// Construct with a caller-supplied [`reqwest::Client`] (e.g. `.no_proxy()` in loopback tests).
  pub fn with_client(client: Client, api_key: &str, base_url: Option<String>) -> Result<Self> {
    let trimmed_key = api_key.trim();
    if trimmed_key.is_empty() {
      return Err(LLMError::MissingApiKey {
        provider: "deepseek".to_string(),
      });
    }
    let base_url = base_url
      .map(|u| u.trim().to_string())
      .filter(|u| !u.is_empty())
      .unwrap_or_else(|| DEFAULT_DEEPSEEK_BASE_URL.to_string());
    Ok(Self {
      client,
      api_key: trimmed_key.to_string(),
      base_url,
    })
  }

  fn tools_url(&self, op: &str) -> String {
    deepseek_tools_url(&self.base_url, op)
  }

  fn auth_headers(&self) -> Result<reqwest::header::HeaderMap> {
    use reqwest::header::{AUTHORIZATION, HeaderMap, HeaderValue};
    let mut headers = HeaderMap::new();
    headers.insert(
      AUTHORIZATION,
      HeaderValue::from_str(&format!("Bearer {}", self.api_key)).map_err(|err| {
        LLMError::ConfigurationError {
          message: format!("DeepSeek API key contains invalid header characters: {err}"),
        }
      })?,
    );
    crate::trace_context::inject_into_headers(&mut headers);
    Ok(headers)
  }
}

/// Normalize `https://api.deepseek.com` or `https://api.deepseek.com/v1` to
/// `{root}/v1/tools/{op}` without duplicating `/v1`.
pub(crate) fn deepseek_tools_url(base_url: &str, op: &str) -> String {
  let trimmed = base_url.trim().trim_end_matches('/');
  let root = trimmed.strip_suffix("/v1").unwrap_or(trimmed);
  let op = op.trim_start_matches('/');
  format!("{root}/v1/tools/{op}")
}

/// Map arbitrary voice names onto DeepSeek's documented `voice_id` enum
/// (`"stella"` female, `"tide"` male) so cross-vendor workflows (`alloy`,
/// `echo`, `host`, `guest`, etc.) work out-of-the-box without HTTP 400 errors.
pub(crate) fn normalize_deepseek_voice(raw: &str) -> &'static str {
  match raw.trim().to_ascii_lowercase().as_str() {
    "tide" | "male" | "echo" | "onyx" | "fable" | "ash" | "ballad" | "verse" | "dan" | "puck"
    | "charon" | "fenrir" | "enceladus" | "guest" | "con" | "speaker_b" | "host_b" => "tide",
    _ => "stella",
  }
}

fn normalize_deepseek_tts_format(raw: Option<&str>) -> (&'static str, &'static str) {
  match raw.map(|s| s.trim().to_ascii_lowercase()).as_deref() {
    Some("pcm") => ("pcm", "audio/pcm"),
    _ => ("wav", "audio/wav"),
  }
}

fn is_wav_or_ogg_opus(bytes: &[u8], filename: &str) -> Option<&'static str> {
  if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE" {
    return Some("audio/wav");
  }
  if bytes.starts_with(b"OggS") {
    return Some("audio/ogg");
  }
  let ext = std::path::Path::new(filename)
    .extension()
    .and_then(|e| e.to_str())
    .map(str::to_ascii_lowercase);
  match ext.as_deref() {
    Some("wav") => Some("audio/wav"),
    Some("ogg" | "opus") => Some("audio/ogg"),
    _ => None,
  }
}

/// DeepSeek `/v1/tools/asr` documents 16-bit PCM WAV or Ogg Opus uploads.
/// When callers pass `.mp3` / `.m4a` / `.webm` recordings and `ffmpeg` is
/// available on `PATH`, transcode to 16 kHz mono PCM WAV; otherwise fall back
/// to uploading the raw bytes verbatim.
async fn prepare_asr_upload(audio_data: &[u8], filename: &str) -> (Vec<u8>, String, &'static str) {
  if let Some(mime) = is_wav_or_ogg_opus(audio_data, filename) {
    let safe_name = if filename.trim().is_empty() {
      if mime == "audio/ogg" {
        "speech.ogg".to_string()
      } else {
        "speech.wav".to_string()
      }
    } else {
      filename.to_string()
    };
    return (audio_data.to_vec(), safe_name, mime);
  }

  let ext = std::path::Path::new(filename)
    .extension()
    .and_then(|e| e.to_str())
    .filter(|e| e.chars().all(|c| c.is_ascii_alphanumeric()))
    .unwrap_or("bin");
  let id = uuid::Uuid::new_v4();
  let tmp_dir = std::env::temp_dir();
  let in_path = tmp_dir.join(format!("yanshi-deepseek-asr-in-{id}.{ext}"));
  let out_path = tmp_dir.join(format!("yanshi-deepseek-asr-out-{id}.wav"));

  if tokio::fs::write(&in_path, audio_data).await.is_ok() {
    let mut cmd = tokio::process::Command::new("ffmpeg");
    cmd
      .args(["-nostdin", "-y", "-v", "error", "-i"])
      .arg(&in_path)
      .args(["-ac", "1", "-ar", "16000", "-c:a", "pcm_s16le", "-f", "wav"])
      .arg(&out_path)
      .kill_on_drop(true);

    let converted = match tokio::time::timeout(
      Duration::from_secs(FFMPEG_TRANSCODE_TIMEOUT_SECS),
      cmd.output(),
    )
    .await
    {
      Ok(Ok(out)) if out.status.success() => tokio::fs::read(&out_path).await.ok(),
      _ => None,
    };
    let _ = tokio::fs::remove_file(&in_path).await;
    let _ = tokio::fs::remove_file(&out_path).await;

    if let Some(wav_bytes) = converted.filter(|b| !b.is_empty()) {
      return (wav_bytes, "speech.wav".to_string(), "audio/wav");
    }
  } else {
    let _ = tokio::fs::remove_file(&in_path).await;
  }

  let fallback_name = if filename.trim().is_empty() {
    "speech.wav".to_string()
  } else {
    filename.to_string()
  };
  (
    audio_data.to_vec(),
    fallback_name,
    "application/octet-stream",
  )
}

#[derive(Debug, Deserialize)]
struct DeepSeekAsrWireResponse {
  #[serde(default)]
  request_id: String,
  text: String,
  #[serde(default)]
  duration_ms: Option<u64>,
}

#[async_trait]
impl TtsProvider for DeepSeekMediaProvider {
  fn name(&self) -> &str {
    "deepseek"
  }

  async fn synthesize(&self, request: TtsRequest) -> Result<TtsResponse> {
    if request.input.trim().is_empty() {
      return Err(LLMError::ConfigurationError {
        message: "DeepSeek TTS `input` text must not be empty".to_string(),
      });
    }
    let url = self.tools_url("tts");
    let voice_id = normalize_deepseek_voice(&request.voice);
    let (wire_format, mime_type) =
      normalize_deepseek_tts_format(request.response_format.as_deref());
    let body = json!({
      "text": request.input,
      "voice_id": voice_id,
      "format": wire_format,
      "filter_markdown": true,
    });

    let mut response = self
      .client
      .post(&url)
      .headers(self.auth_headers()?)
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

    if let Some(declared) = response.content_length()
      && declared > MAX_TTS_AUDIO_BYTES
    {
      return Err(LLMError::ResponseParsingError {
        message: format!(
          "DeepSeek TTS response declares {declared} bytes, exceeding {MAX_TTS_AUDIO_BYTES}-byte cap"
        ),
      });
    }

    let mut audio = Vec::new();
    while let Some(chunk) = response.chunk().await? {
      if audio.len() as u64 + chunk.len() as u64 > MAX_TTS_AUDIO_BYTES {
        return Err(LLMError::ResponseParsingError {
          message: format!("DeepSeek TTS stream exceeded {MAX_TTS_AUDIO_BYTES}-byte cap"),
        });
      }
      audio.extend_from_slice(&chunk);
    }

    Ok(TtsResponse {
      audio,
      mime_type: mime_type.to_string(),
    })
  }
}

#[async_trait]
impl AsrProvider for DeepSeekMediaProvider {
  fn name(&self) -> &str {
    "deepseek"
  }

  async fn transcribe(&self, request: AsrRequest) -> Result<AsrResponse> {
    if request.audio_data.is_empty() {
      return Err(LLMError::ConfigurationError {
        message: "DeepSeek ASR `audio_data` must not be empty".to_string(),
      });
    }
    let url = self.tools_url("asr");
    let (upload_bytes, upload_name, mime) =
      prepare_asr_upload(&request.audio_data, &request.filename).await;

    let file_part = Part::bytes(upload_bytes.clone())
      .file_name(upload_name.clone())
      .mime_str(mime)
      .unwrap_or_else(|_| Part::bytes(upload_bytes).file_name(upload_name));
    let form = Form::new().part("file", file_part);

    let response = self
      .client
      .post(&url)
      .headers(self.auth_headers()?)
      .multipart(form)
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

    let parsed: DeepSeekAsrWireResponse =
      response
        .json()
        .await
        .map_err(|err| LLMError::ResponseParsingError {
          message: format!("Failed to parse DeepSeek ASR response JSON: {err}"),
        })?;

    Ok(AsrResponse {
      text: parsed.text,
      metadata: Some(json!({
        "request_id": parsed.request_id,
        "duration_ms": parsed.duration_ms,
      })),
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  #[test]
  fn deepseek_tools_url_normalizes_root_and_v1_base_urls() {
    assert_eq!(
      deepseek_tools_url("https://api.deepseek.com/v1", "tts"),
      "https://api.deepseek.com/v1/tools/tts"
    );
    assert_eq!(
      deepseek_tools_url("https://api.deepseek.com/", "asr"),
      "https://api.deepseek.com/v1/tools/asr"
    );
    assert_eq!(
      deepseek_tools_url("http://127.0.0.1:18080/v1/", "tts"),
      "http://127.0.0.1:18080/v1/tools/tts"
    );
  }

  #[test]
  fn normalize_deepseek_voice_maps_gender_and_preset_aliases() {
    assert_eq!(normalize_deepseek_voice("stella"), "stella");
    assert_eq!(normalize_deepseek_voice("tide"), "tide");
    assert_eq!(normalize_deepseek_voice("alloy"), "stella");
    assert_eq!(normalize_deepseek_voice("nova"), "stella");
    assert_eq!(normalize_deepseek_voice("echo"), "tide");
    assert_eq!(normalize_deepseek_voice("onyx"), "tide");
    assert_eq!(normalize_deepseek_voice("guest"), "tide");
  }

  #[test]
  fn debug_redacts_api_key() {
    let client = Client::builder().no_proxy().build().expect("test client");
    let provider =
      DeepSeekMediaProvider::with_client(client, "sk-secret-deepseek", None).expect("provider");
    let rendered = format!("{provider:?}");
    assert!(!rendered.contains("sk-secret-deepseek"));
    assert!(rendered.contains("<redacted>"));
  }

  #[tokio::test]
  async fn synthesize_posts_to_v1_tools_tts_and_returns_wav_bytes() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let wav_payload = b"RIFF\x24\x00\x00\x00WAVEfmt ";

    let server = tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 4096];
      let n = socket.read(&mut buf).await.expect("read");
      let req = String::from_utf8_lossy(&buf[..n]).to_string();
      assert!(req.starts_with("POST /v1/tools/tts HTTP/1.1"), "got: {req}");
      assert!(
        req
          .to_ascii_lowercase()
          .contains("authorization: bearer sk-ds-test")
      );
      assert!(req.contains("\"voice_id\":\"tide\""));
      assert!(req.contains("\"format\":\"wav\""));
      assert!(req.contains("\"filter_markdown\":true"));

      let header = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: audio/wav\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        wav_payload.len()
      );
      socket
        .write_all(header.as_bytes())
        .await
        .expect("write header");
      socket.write_all(wav_payload).await.expect("write body");
    });

    let client = Client::builder().no_proxy().build().expect("test client");
    let provider =
      DeepSeekMediaProvider::with_client(client, "sk-ds-test", Some(format!("http://{addr}/v1")))
        .expect("provider");

    let resp = provider
      .synthesize(TtsRequest {
        model: "deepseek-tts".to_string(),
        input: "Hello from DeepSeek.".to_string(),
        voice: "onyx".to_string(),
        response_format: Some("mp3".to_string()),
        ..Default::default()
      })
      .await
      .expect("synthesize ok");

    assert_eq!(resp.mime_type, "audio/wav");
    assert_eq!(resp.audio, wav_payload);
    server.await.expect("server task");
  }

  #[tokio::test]
  async fn transcribe_posts_multipart_to_v1_tools_asr_and_preserves_duration_ms() {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");

    let server = tokio::spawn(async move {
      let (mut socket, _) = listener.accept().await.expect("accept");
      let mut buf = vec![0u8; 8192];
      let n = socket.read(&mut buf).await.expect("read");
      let req = String::from_utf8_lossy(&buf[..n]).to_string();
      assert!(req.starts_with("POST /v1/tools/asr HTTP/1.1"), "got: {req}");
      assert!(req.contains("name=\"file\"; filename=\"speech.wav\""));

      let body = r#"{"request_id":"req-ds-1","text":"今天上海的天气怎么样？","duration_ms":3200}"#;
      let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
      );
      socket.write_all(resp.as_bytes()).await.expect("write");
    });

    let client = Client::builder().no_proxy().build().expect("test client");
    let provider =
      DeepSeekMediaProvider::with_client(client, "sk-ds-test", Some(format!("http://{addr}/v1")))
        .expect("provider");

    let mut fake_wav = b"RIFF\x24\x00\x00\x00WAVEfmt ".to_vec();
    fake_wav.extend_from_slice(&[0u8; 32]);
    let resp = provider
      .transcribe(AsrRequest {
        model: "deepseek-asr".to_string(),
        audio_data: fake_wav,
        filename: "speech.wav".to_string(),
        response_format: "json".to_string(),
        language: None,
        temperature: None,
        prompt: None,
      })
      .await
      .expect("transcribe ok");

    assert_eq!(resp.text, "今天上海的天气怎么样？");
    assert_eq!(
      resp
        .metadata
        .as_ref()
        .and_then(|m| m["duration_ms"].as_u64()),
      Some(3200)
    );
    server.await.expect("server task");
  }
}
