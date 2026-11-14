//! `text_to_video` — text-to-video generation `Tool` adapter.
//!
//! Thin wrapper over `yanshi_llm::Yanshi::text2video_for(model)` — see
//! `tts.rs`'s module docs for the shared design rationale.
//!
//! Unlike the other five tools, video generation is an async job API
//! (`Text2VideoProvider::submit` + `::poll`, minutes not seconds) rather
//! than a synchronous request/response — see that trait's module docs in
//! `yanshi-llm/src/providers/modality/text_to_video.rs`. This tool uses
//! the trait's `generate_and_wait` convenience default, blocking the
//! calling tool call for up to `max_wait_secs` rather than exposing a
//! separate submit/poll tool pair — simpler for a first cut, at the cost
//! of tying up an agent turn for the duration of generation. The
//! `poll_interval_secs`-spaced `tokio::time::sleep` loop inside
//! `generate_and_wait` is cleanly cancellable (dropped, not aborted mid-
//! syscall) under the `Tool::execute` cancellation contract documented on
//! that trait.

use std::time::Duration;

use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use serde_json::{Value, json};
use std::path::Path;
use yanshi_llm::{Text2VideoRequest, VideoInputImage, Yanshi};
use yanshi_tool::{Tool, ToolError, ToolMetadata, ToolOutput, ToolOutputPart};

use crate::common::{
  map_resolution_error, modality_tool_metadata, optional_str, optional_u32, required_str,
};

const DEFAULT_POLL_INTERVAL_SECS: u64 = 5;
const DEFAULT_MAX_WAIT_SECS: u64 = 300;
/// Bounds on the LLM-chosen polling. Polling runs locally, so nothing else
/// stops a `0` interval from hammering the vendor or a huge wait from
/// parking the call (eval 2026-10-07 L6).
const POLL_INTERVAL_SECS: std::ops::RangeInclusive<u64> = 1..=60;
const MAX_WAIT_SECS: std::ops::RangeInclusive<u64> = 1..=1800;

/// Generate a video from a text prompt via any text-to-video-capable
/// model declared in the model registry, waiting for the (typically
/// minutes-long) generation job to finish.
pub struct Text2VideoTool;

impl Text2VideoTool {
  pub fn new() -> Self {
    Self
  }
}

impl Default for Text2VideoTool {
  fn default() -> Self {
    Self::new()
  }
}

#[async_trait]
impl Tool for Text2VideoTool {
  fn name(&self) -> &str {
    "text_to_video"
  }

  fn description(&self) -> &str {
    "Generate a video from a text prompt using a configured text-to-video \
     model. Blocks until the (typically minutes-long) generation job \
     completes or a timeout elapses. Returns the video as a URL reference \
     or inline base64 data."
  }

  fn parameters_schema(&self) -> Value {
    json!({
      "type": "object",
      "properties": {
        "model": {
          "type": "string",
          "description": "Text-to-video model name from the model registry (e.g. \"veo-3.1-generate-preview\")."
        },
        "prompt": {
          "type": "string",
          "description": "Text prompt describing the desired video."
        },
        "aspect_ratio": {
          "type": "string",
          "description": "Output aspect ratio, vendor-specific format (e.g. \"16:9\")."
        },
        "duration_seconds": {
          "type": "integer",
          "description": "Output duration in seconds. Vendor-specific allowed values."
        },
        "resolution": {
          "type": "string",
          "description": "Output resolution, vendor-specific format (e.g. \"1080p\")."
        },
        "seed": {
          "type": "integer",
          "description": "Random seed for reproducible generation. Vendor support varies."
        },
        "image_path": {
          "type": "string",
          "description": "Optional local image file path or data: URI to use as the first frame for Image-to-Video (I2V)."
        },
        "output_path": {
          "type": "string",
          "description": "Optional local file path to save the generated MP4 video instead of returning inline base64 data."
        },
        "poll_interval_secs": {
          "type": "integer",
          "minimum": 1,
          "maximum": 60,
          "description": "How often to poll job status, in seconds (1-60). Defaults to 5."
        },
        "max_wait_secs": {
          "type": "integer",
          "minimum": 1,
          "maximum": 1800,
          "description": "Maximum time to wait for the job to finish, in seconds (1-1800), before failing. Defaults to 300 (5 minutes)."
        }
      },
      "required": ["model", "prompt"]
    })
  }

  fn metadata(&self) -> ToolMetadata {
    modality_tool_metadata()
  }

  async fn execute(&self, params: Value) -> Result<ToolOutput, ToolError> {
    let model = required_str(&params, "model")?;
    let prompt = required_str(&params, "prompt")?.to_string();
    let poll_interval = Duration::from_secs(bounded_secs(
      &params,
      "poll_interval_secs",
      DEFAULT_POLL_INTERVAL_SECS,
      POLL_INTERVAL_SECS,
    )?);
    let max_wait = Duration::from_secs(bounded_secs(
      &params,
      "max_wait_secs",
      DEFAULT_MAX_WAIT_SECS,
      MAX_WAIT_SECS,
    )?);

    let image = match optional_str(&params, "image_path") {
      Some(path_or_uri) => Some(load_input_image(&path_or_uri).await?),
      None => None,
    };
    let output_path = optional_str(&params, "output_path");

    let provider = Yanshi::text2video_for(model)
      .await
      .map_err(|err| map_resolution_error("failed to resolve text-to-video model", err))?;

    let request = Text2VideoRequest {
      model: model.to_string(),
      prompt,
      image,
      aspect_ratio: optional_str(&params, "aspect_ratio"),
      duration_seconds: optional_u32(&params, "duration_seconds"),
      resolution: optional_str(&params, "resolution"),
      seed: params["seed"].as_i64(),
      extra: None,
    };

    match provider
      .generate_and_wait(request, poll_interval, max_wait)
      .await
    {
      Ok(response) => {
        if let Some(ref out_path) = output_path {
          let Some(first_video) = response.videos.first() else {
            return Ok(ToolOutput::error("video generation returned no videos"));
          };
          const MAX_TOOL_VIDEO_BYTES: u64 = 100 * 1024 * 1024;
          let bytes = match provider
            .fetch_video_bytes(first_video, MAX_TOOL_VIDEO_BYTES)
            .await
          {
            Ok(b) => b,
            Err(err) => {
              return Ok(ToolOutput::error(format!(
                "failed to fetch generated video: {err}"
              )));
            }
          };
          if let Some(parent) = Path::new(out_path)
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
          {
            tokio::fs::create_dir_all(parent)
              .await
              .map_err(|err| ToolError::ExecutionFailed {
                message: format!(
                  "failed to create output directory '{}': {err}",
                  parent.display()
                ),
              })?;
          }
          tokio::fs::write(out_path, &bytes)
            .await
            .map_err(|err| ToolError::ExecutionFailed {
              message: format!("failed to write video to '{out_path}': {err}"),
            })?;
          return Ok(ToolOutput::success(
            json!({
              "output_path": out_path,
              "bytes": bytes.len(),
              "mime_type": "video/mp4",
              "duration_seconds": first_video.duration_seconds,
            })
            .to_string(),
          ));
        }

        let count = response.videos.len();
        let parts: Vec<ToolOutputPart> = response
          .videos
          .into_iter()
          .map(|video| match (video.url, video.b64_data) {
            (Some(url), _) => ToolOutputPart::Resource {
              uri: url,
              mime_type: Some("video/mp4".to_string()),
              text: None,
            },
            (None, Some(b64)) => ToolOutputPart::Resource {
              uri: format!("data:video/mp4;base64,{b64}"),
              mime_type: Some("video/mp4".to_string()),
              text: None,
            },
            (None, None) => ToolOutputPart::Text {
              text: "(video entry carried neither a url nor base64 data)".to_string(),
            },
          })
          .collect();
        Ok(ToolOutput::success_parts(
          format!("Generated {count} video(s)."),
          parts,
        ))
      }
      Err(err) => Ok(ToolOutput::error(format!("video generation failed: {err}"))),
    }
  }
}

async fn load_input_image(path_or_uri: &str) -> Result<VideoInputImage, ToolError> {
  if let Some(rest) = path_or_uri.strip_prefix("data:") {
    let (mime, b64) = rest
      .split_once(";base64,")
      .ok_or_else(|| ToolError::InvalidParams {
        message: "data: URI for `image_path` must use ;base64,".to_string(),
      })?;
    return Ok(VideoInputImage {
      bytes_base64_encoded: b64.to_string(),
      mime_type: if mime.is_empty() {
        "image/png".to_string()
      } else {
        mime.to_string()
      },
    });
  }
  let bytes = tokio::fs::read(path_or_uri)
    .await
    .map_err(|err| ToolError::InvalidParams {
      message: format!("failed to read `image_path` '{path_or_uri}': {err}"),
    })?;
  let lower = path_or_uri.to_ascii_lowercase();
  let mime_type = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
    "image/jpeg"
  } else if lower.ends_with(".webp") {
    "image/webp"
  } else {
    "image/png"
  };
  Ok(VideoInputImage {
    bytes_base64_encoded: BASE64.encode(&bytes),
    mime_type: mime_type.to_string(),
  })
}

fn bounded_secs(
  params: &Value,
  field: &str,
  default: u64,
  range: std::ops::RangeInclusive<u64>,
) -> Result<u64, ToolError> {
  let Some(raw) = params.get(field).filter(|value| !value.is_null()) else {
    return Ok(default);
  };
  raw
    .as_u64()
    .filter(|secs| range.contains(secs))
    .ok_or_else(|| ToolError::InvalidParams {
      message: format!(
        "`{field}` must be an integer from {} to {}, got {raw}",
        range.start(),
        range.end()
      ),
    })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn execute_rejects_missing_prompt() {
    let tool = Text2VideoTool::new();
    let err = tool
      .execute(json!({"model": "veo-3.1-generate-preview"}))
      .await
      .expect_err("missing prompt must fail params validation");
    assert!(matches!(err, ToolError::InvalidParams { .. }));
  }

  #[tokio::test]
  async fn execute_surfaces_unknown_model_as_invalid_params() {
    let tool = Text2VideoTool::new();
    let err = tool
      .execute(json!({"model": "definitely-not-a-real-model", "prompt": "a cat running"}))
      .await
      .expect_err("unknown model must fail resolution, not panic or hang");
    assert!(matches!(err, ToolError::InvalidParams { .. }));
  }

  /// Eval 2026-10-07 L6: `poll_interval_secs: 0` busy-polled the vendor and
  /// a huge `max_wait_secs` parked the tool call indefinitely.
  #[tokio::test]
  async fn execute_rejects_out_of_range_polling() {
    let tool = Text2VideoTool::new();
    for (field, value) in [
      ("poll_interval_secs", 0),
      ("poll_interval_secs", 61),
      ("max_wait_secs", 0),
      ("max_wait_secs", 1_801),
    ] {
      let err = tool
        .execute(json!({"model": "veo-3.1-generate-preview", "prompt": "a cat", field: value}))
        .await
        .expect_err("out-of-range polling must fail params validation");
      assert!(
        matches!(&err, ToolError::InvalidParams { message } if message.contains(field)),
        "{field}={value}: {err:?}"
      );
    }
  }
}
