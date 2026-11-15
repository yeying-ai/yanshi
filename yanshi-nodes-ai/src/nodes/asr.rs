//! ASR Node - Transcribes audio to text or SubRip (`.srt`) subtitles using a
//! configured ASR model, and passes pre-extracted `.srt`/`.vtt` subtitle files
//! straight through with zero vendor calls.

use std::collections::HashMap;
use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_llm::{AsrRequest, Yanshi};
use yanshi_nodes::common::{
  artifacts,
  utils::{flow_value_to_string, load_bytes_from_source},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum ASRResponseFormat {
  Json,
  #[default]
  Text,
  Srt,
  Vtt,
}

impl ASRResponseFormat {
  pub fn from_str_opt(raw: &str) -> Option<Self> {
    match raw.trim().to_ascii_lowercase().as_str() {
      "json" | "verbose_json" => Some(Self::Json),
      "text" | "txt" => Some(Self::Text),
      "srt" => Some(Self::Srt),
      "vtt" | "webvtt" => Some(Self::Vtt),
      _ => None,
    }
  }

  pub fn as_wire_str(&self) -> &'static str {
    match self {
      Self::Json => "json",
      Self::Text => "text",
      Self::Srt => "srt",
      Self::Vtt => "vtt",
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ASRNode {
  pub name: String,
  pub model: String,
  pub audio_source: String,
  pub response_format: ASRResponseFormat,
  pub output_key: String,
  pub input_keys: Vec<String>,
}

impl ASRNode {
  pub fn new(name: &str, model: &str, audio_source: &str) -> Self {
    Self {
      name: name.to_string(),
      model: model.to_string(),
      audio_source: audio_source.to_string(),
      response_format: Default::default(),
      output_key: format!("{}_output", name),
      input_keys: vec![],
    }
  }

  pub fn with_response_format(mut self, response_format: ASRResponseFormat) -> Self {
    self.response_format = response_format;
    self
  }
}

enum ResolvedAsrInput {
  SubtitleText(String),
  Audio { bytes: Vec<u8>, filename: String },
}

#[async_trait]
impl AsyncNode for ASRNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    tracing::debug!(name = %self.name, "executing ASRNode");

    let str_in = |key: &str| match inputs.get(key) {
      Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.trim().to_string()),
      _ => None,
    };

    let format = str_in("response_format")
      .and_then(|s| ASRResponseFormat::from_str_opt(&s))
      .unwrap_or_else(|| self.response_format.clone());
    let format_str = format.as_wire_str();
    let language = str_in("language");
    let prompt = str_in("prompt");

    let resolved = self.resolve_audio_or_subtitles(inputs).await?;

    let (primary_output, plain_text, srt_text) = match resolved {
      ResolvedAsrInput::SubtitleText(raw_subs) => {
        let srt = normalize_to_srt(&raw_subs, 30.0);
        let plain = srt_to_timestamped_text(&srt);
        let primary = match format {
          ASRResponseFormat::Srt => srt.clone(),
          _ => plain.clone(),
        };
        (primary, plain, srt)
      }
      ResolvedAsrInput::Audio { bytes, filename } => {
        let model = str_in("model").unwrap_or_else(|| self.model.clone());
        if model.trim().is_empty() {
          return Err(YanshiError::NodeInputError {
            message: format!("{}: missing required 'model' parameter or input", self.name),
          });
        }

        Yanshi::ensure_init()
          .await
          .map_err(|e| YanshiError::ConfigurationError {
            message: format!("Failed to initialize Yanshi LLM service: {}", e),
          })?;

        let provider = Yanshi::asr(&model)
          .await
          .map_err(|e| YanshiError::ConfigurationError {
            message: format!("Failed to resolve ASR provider for model '{model}': {e}"),
          })?;

        let est_seconds = estimate_audio_seconds(&bytes, &filename);
        let request = AsrRequest {
          model: model.clone(),
          response_format: format_str.to_string(),
          audio_data: bytes,
          filename,
          language,
          temperature: None,
          prompt,
        };

        tracing::debug!(provider = %provider.name(), model = %model, "transcribing audio via provider");
        let asr_response = provider
          .transcribe(request)
          .await
          .map_err(|e| crate::vendor_error::vendor_error("ASR transcription failed", e))?;
        crate::cost::record(&model, |p| p.cost_for_seconds(est_seconds))?;

        let raw = asr_response.text;
        let srt = normalize_to_srt(&raw, est_seconds);
        let plain = if raw.contains("-->") {
          srt_to_timestamped_text(&srt)
        } else {
          raw.trim().to_string()
        };
        let primary = match format {
          ASRResponseFormat::Srt => srt.clone(),
          _ => raw,
        };
        (primary, plain, srt)
      }
    };

    let srt_file =
      artifacts::write_artifact(&self.name, srt_text.as_bytes(), "application/x-subrip").await?;

    tracing::debug!("ASRNode execution successful");
    let mut outputs = HashMap::new();
    outputs.insert(
      self.output_key.clone(),
      FlowValue::Json(Value::String(primary_output.clone())),
    );
    outputs.insert(
      "output".to_string(),
      FlowValue::Json(Value::String(primary_output)),
    );
    outputs.insert(
      "text".to_string(),
      FlowValue::Json(Value::String(plain_text)),
    );
    outputs.insert("srt".to_string(), FlowValue::Json(Value::String(srt_text)));
    outputs.insert("srt_file".to_string(), srt_file.clone());
    outputs.insert(format!("{}_srt_file", self.name), srt_file);

    Ok(outputs)
  }
}

impl ASRNode {
  async fn resolve_audio_or_subtitles(
    &self,
    inputs: &AsyncNodeInputs,
  ) -> Result<ResolvedAsrInput, YanshiError> {
    if let Some(val) = inputs.get("audio_source") {
      match val {
        FlowValue::File { path, mime_type } => {
          return load_file_payload(&self.name, path, mime_type.as_deref()).await;
        }
        FlowValue::Json(Value::Object(obj)) => {
          if let Some(path_str) = obj.get("path").and_then(Value::as_str) {
            let mime = obj.get("mime_type").and_then(Value::as_str);
            return load_file_payload(&self.name, Path::new(path_str), mime).await;
          }
        }
        FlowValue::Json(Value::String(s)) if !s.trim().is_empty() => {
          let trimmed = s.trim();
          if trimmed.starts_with("data:") {
            let filename = filename_for_data_uri(trimmed);
            let bytes = load_bytes_from_source_str(trimmed, inputs).await?;
            return Ok(ResolvedAsrInput::Audio { bytes, filename });
          }
          if let Some(referenced) = inputs.get(trimmed)
            && let FlowValue::File { path, mime_type } = referenced
          {
            return load_file_payload(&self.name, path, mime_type.as_deref()).await;
          }
          let path_candidate = Path::new(trimmed);
          if is_subtitle_extension(path_candidate) && path_candidate.is_file() {
            return load_file_payload(&self.name, path_candidate, Some("application/x-subrip"))
              .await;
          }
          let bytes = load_bytes_from_source(trimmed, inputs).await?;
          let filename = path_candidate
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .filter(|n| n.contains('.'))
            .unwrap_or_else(|| "audio.mp3".to_string());
          return Ok(ResolvedAsrInput::Audio { bytes, filename });
        }
        _ => {}
      }
    }

    let mut resolved_source = self.audio_source.clone();
    for key in &self.input_keys {
      if let Some(value) = inputs.get(key) {
        let placeholder = format!("{{{{{}}}}}", key);
        resolved_source = resolved_source.replace(&placeholder, &flow_value_to_string(value));
      }
    }
    if resolved_source.trim().is_empty() {
      return Err(YanshiError::NodeInputError {
        message: format!("{}: missing required 'audio_source' input", self.name),
      });
    }
    let bytes = load_bytes_from_source(&resolved_source, inputs).await?;
    let filename = if resolved_source.starts_with("data:") {
      filename_for_data_uri(&resolved_source)
    } else {
      Path::new(&resolved_source)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|n| n.contains('.'))
        .unwrap_or_else(|| "audio.wav".to_string())
    };
    Ok(ResolvedAsrInput::Audio { bytes, filename })
  }
}

async fn load_bytes_from_source_str(
  data_uri: &str,
  inputs: &AsyncNodeInputs,
) -> Result<Vec<u8>, YanshiError> {
  let mut tmp = AsyncNodeInputs::new();
  tmp.insert(
    "__inline".to_string(),
    FlowValue::Json(Value::String(data_uri.to_string())),
  );
  let _ = inputs;
  load_bytes_from_source("__inline", &tmp).await
}

async fn load_file_payload(
  node_name: &str,
  path: &Path,
  mime_type: Option<&str>,
) -> Result<ResolvedAsrInput, YanshiError> {
  let bytes = tokio::fs::read(path)
    .await
    .map_err(|err| YanshiError::NodeInputError {
      message: format!("{node_name}: failed to read '{}': {err}", path.display()),
    })?;
  let is_sub = mime_type.is_some_and(|m| {
    let lower = m.to_ascii_lowercase();
    lower.contains("subrip") || lower.contains("vtt")
  }) || is_subtitle_extension(path);

  if is_sub {
    return Ok(ResolvedAsrInput::SubtitleText(
      String::from_utf8_lossy(&bytes).into_owned(),
    ));
  }
  let filename = path
    .file_name()
    .map(|n| n.to_string_lossy().into_owned())
    .unwrap_or_else(|| "audio.mp3".to_string());
  Ok(ResolvedAsrInput::Audio { bytes, filename })
}

fn is_subtitle_extension(path: &Path) -> bool {
  path
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "srt" | "vtt"))
}

fn filename_for_data_uri(uri: &str) -> String {
  let header = uri
    .strip_prefix("data:")
    .and_then(|rest| rest.split(';').next())
    .unwrap_or_default()
    .to_ascii_lowercase();
  let ext = match header.as_str() {
    "audio/wav" | "audio/x-wav" | "audio/wave" => "wav",
    "audio/flac" | "audio/x-flac" => "flac",
    "audio/ogg" | "audio/opus" => "ogg",
    "audio/mp4" | "audio/m4a" => "m4a",
    "audio/webm" => "webm",
    _ => "mp3",
  };
  format!("audio.{ext}")
}

fn estimate_audio_seconds(bytes: &[u8], filename: &str) -> f64 {
  if let Some(wav_secs) = crate::cost::wav_duration_seconds(bytes) {
    return wav_secs.max(0.1);
  }
  // `video_ingest` encodes MP3 at 48 kbps (~6,000 bytes/sec); general audio
  // averages ~16 KB/sec at 128 kbps.
  let rate_bytes_per_sec = if filename.to_ascii_lowercase().ends_with(".mp3") {
    6_000.0
  } else {
    16_000.0
  };
  ((bytes.len() as f64) / rate_bytes_per_sec).max(1.0)
}

/// Convert raw transcript or WebVTT into valid SubRip (`.srt`) cues.
pub fn normalize_to_srt(raw: &str, total_duration_secs: f64) -> String {
  let normalized = raw.replace("\r\n", "\n");
  let trimmed = normalized.trim();
  if trimmed.is_empty() {
    return "1\n00:00:00,000 --> 00:00:02,000\n[No speech detected]\n".to_string();
  }

  if trimmed.starts_with("WEBVTT") {
    return vtt_to_srt(trimmed);
  }

  if trimmed.contains("-->") {
    return format!("{trimmed}\n");
  }

  let sentences = split_sentences(trimmed);
  let count = sentences.len().max(1);
  let total = total_duration_secs.max((count as f64) * 3.0);
  let step = total / (count as f64);

  let mut out = String::new();
  for (idx, cue) in sentences.iter().enumerate() {
    let start = (idx as f64) * step;
    let end = ((idx + 1) as f64 * step).max(start + 1.0);
    out.push_str(&format!(
      "{}\n{} --> {}\n{}\n\n",
      idx + 1,
      format_srt_timestamp(start),
      format_srt_timestamp(end),
      cue.trim()
    ));
  }
  out.trim_end().to_string() + "\n"
}

fn vtt_to_srt(vtt: &str) -> String {
  let mut cues = Vec::<(String, Vec<String>)>::new();
  let mut current_time: Option<String> = None;
  let mut current_lines = Vec::<String>::new();

  for line in vtt.lines() {
    let t = line.trim();
    if t.starts_with("WEBVTT")
      || t.starts_with("Kind:")
      || t.starts_with("Language:")
      || t.starts_with("NOTE")
    {
      continue;
    }
    if t.is_empty() {
      if let Some(ts) = current_time.take()
        && !current_lines.is_empty()
      {
        cues.push((ts, std::mem::take(&mut current_lines)));
      }
      continue;
    }
    if t.contains("-->") {
      if let Some(ts) = current_time.take()
        && !current_lines.is_empty()
      {
        cues.push((ts, std::mem::take(&mut current_lines)));
      }
      let parts: Vec<&str> = t.split("-->").collect();
      if parts.len() >= 2 {
        let start = normalize_vtt_ts(parts[0]);
        let end = normalize_vtt_ts(parts[1].split_whitespace().next().unwrap_or(parts[1]));
        current_time = Some(format!("{start} --> {end}"));
      }
    } else if current_time.is_some() {
      current_lines.push(strip_vtt_tags(t));
    }
  }
  if let Some(ts) = current_time.take()
    && !current_lines.is_empty()
  {
    cues.push((ts, current_lines));
  }

  let mut srt = String::new();
  for (idx, (ts, lines)) in cues.iter().enumerate() {
    let text = lines.join("\n");
    if text.trim().is_empty() {
      continue;
    }
    srt.push_str(&format!("{}\n{ts}\n{}\n\n", idx + 1, text.trim()));
  }
  if srt.is_empty() {
    "1\n00:00:00,000 --> 00:00:02,000\n[Empty subtitle stream]\n".to_string()
  } else {
    srt.trim_end().to_string() + "\n"
  }
}

fn normalize_vtt_ts(raw: &str) -> String {
  let clean = raw.trim().replace('.', ",");
  let colon_count = clean.chars().filter(|c| *c == ':').count();
  if colon_count == 1 {
    format!("00:{clean}")
  } else {
    clean
  }
}

fn strip_vtt_tags(line: &str) -> String {
  let mut out = String::with_capacity(line.len());
  let mut in_tag = false;
  for ch in line.chars() {
    match ch {
      '<' => in_tag = true,
      '>' => in_tag = false,
      _ if !in_tag => out.push(ch),
      _ => {}
    }
  }
  out
}

pub fn srt_to_timestamped_text(srt: &str) -> String {
  let mut lines_out = Vec::new();
  let mut current_ts: Option<String> = None;
  let mut buf = Vec::new();

  for line in srt.replace("\r\n", "\n").lines() {
    let t = line.trim();
    if t.is_empty() {
      if !buf.is_empty() {
        let text = buf.join(" ");
        if let Some(ts) = current_ts.take() {
          lines_out.push(format!("[{ts}] {text}"));
        } else {
          lines_out.push(text);
        }
        buf.clear();
      }
      continue;
    }
    if t.chars().all(|c| c.is_ascii_digit()) && current_ts.is_none() && buf.is_empty() {
      continue;
    }
    if let Some((start, _)) = t.split_once("-->") {
      let short_ts = start.trim().split(',').next().unwrap_or(start.trim());
      current_ts = Some(short_ts.to_string());
    } else {
      buf.push(t.to_string());
    }
  }
  if !buf.is_empty() {
    let text = buf.join(" ");
    if let Some(ts) = current_ts {
      lines_out.push(format!("[{ts}] {text}"));
    } else {
      lines_out.push(text);
    }
  }
  lines_out.join("\n")
}

fn split_sentences(text: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut current = String::new();
  for ch in text.chars() {
    current.push(ch);
    if matches!(ch, '。' | '！' | '？' | '.' | '!' | '?' | '\n') && current.chars().count() >= 8
    {
      let trimmed = current.trim().to_string();
      if !trimmed.is_empty() {
        out.push(trimmed);
      }
      current.clear();
    }
  }
  let tail = current.trim();
  if !tail.is_empty() {
    out.push(tail.to_string());
  }
  if out.is_empty() {
    vec![text.trim().to_string()]
  } else {
    out
  }
}

fn format_srt_timestamp(seconds: f64) -> String {
  let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
  let ms = total_ms % 1000;
  let total_s = total_ms / 1000;
  let s = total_s % 60;
  let total_m = total_s / 60;
  let m = total_m % 60;
  let h = total_m / 60;
  format!("{h:02}:{m:02}:{s:02},{ms:03}")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn converts_vtt_and_plain_text_into_valid_srt_and_timestamped_lines() {
    let vtt = "WEBVTT\n\n00:01.250 --> 00:04.000 align:start\n<c>Hello</c> world!\n\n00:04.500 --> 00:08.000\nSecond cue.\n";
    let srt = normalize_to_srt(vtt, 10.0);
    assert!(srt.contains("1\n00:00:01,250 --> 00:00:04,000\nHello world!"));
    assert!(srt.contains("2\n00:00:04,500 --> 00:00:08,000\nSecond cue."));

    let plain = srt_to_timestamped_text(&srt);
    assert!(plain.contains("[00:00:01] Hello world!"));
    assert!(plain.contains("[00:00:04] Second cue."));

    let wrapped = normalize_to_srt("第一句重要结论。第二句实验数据支撑。", 8.0);
    assert!(wrapped.contains("1\n00:00:00,000 --> 00:00:04,000\n第一句重要结论。"));
    assert!(wrapped.contains("2\n00:00:04,000 --> 00:00:08,000\n第二句实验数据支撑。"));
  }

  #[tokio::test]
  async fn passes_through_pre_extracted_srt_file_without_vendor_call() {
    let dir = tempfile::tempdir().unwrap();
    let srt_path = dir.path().join("native.srt");
    std::fs::write(
      &srt_path,
      "1\n00:00:00,000 --> 00:00:03,000\nNative subtitle cue\n",
    )
    .unwrap();

    let node = ASRNode::new("transcribe", "", "").with_response_format(ASRResponseFormat::Srt);
    let mut inputs = AsyncNodeInputs::new();
    inputs.insert(
      "audio_source".to_string(),
      FlowValue::File {
        path: srt_path,
        mime_type: Some("application/x-subrip".to_string()),
      },
    );

    let out = node.execute(&inputs).await.unwrap();
    let FlowValue::Json(Value::String(ref text)) = out["text"] else {
      panic!("expected string text");
    };
    assert!(text.contains("[00:00:00] Native subtitle cue"));
    assert!(matches!(out["srt_file"], FlowValue::File { .. }));
  }

  #[tokio::test]
  #[ignore] // Requires STEP_API_KEY environment variable
  async fn test_asr_node_integration() {
    if std::env::var("STEP_API_KEY").is_err() {
      return;
    }

    let node = ASRNode {
      name: "test_asr".to_string(),
      model: "step-asr".to_string(),
      audio_source: "test.wav".to_string(),
      response_format: ASRResponseFormat::Text,
      output_key: "transcript_output".to_string(),
      input_keys: vec!["test.wav".to_string()],
    };

    let mut inputs = AsyncNodeInputs::new();
    let audio_data =
      "data:audio/wav;base64,UklGRiQAAABXQVZFZm10IBAAAAABAAEARKwAAIhYAQACABgAZGF0YQAAAAA=";
    inputs.insert(
      "test.wav".to_string(),
      FlowValue::Json(Value::String(audio_data.to_string())),
    );
    let result = node.execute(&inputs).await;
    assert!(result.is_ok(), "Node execution failed: {:?}", result.err());
  }
}
