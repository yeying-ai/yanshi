use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use std::path::Path;
use std::time::Duration;
use tokio::fs;
use yanshi_llm::{Text2VideoRequest, VideoInputImage, Yanshi};

const MAX_VIDEO_DOWNLOAD_BYTES: u64 = 100 * 1024 * 1024;

#[allow(clippy::too_many_arguments)]
pub async fn execute(
  prompt: String,
  model: Option<String>,
  output: String,
  image: Option<String>,
  aspect_ratio: Option<String>,
  duration: Option<u32>,
  resolution: Option<String>,
  seed: Option<i64>,
  poll_interval: u64,
  max_wait: u64,
) -> Result<()> {
  let model = model.unwrap_or_else(|| "veo-3.1-fast-generate-001".to_string());
  Yanshi::init().await?;

  println!("🎬 Yanshi Video Generation");
  println!("Model: {}", model);
  println!("Prompt: {}", prompt);
  if let Some(ref img) = image {
    println!("First-frame image: {}", img);
  }
  println!("Output: {}", output);
  println!();

  let input_image = match image {
    Some(ref path_or_uri) => Some(load_input_image(path_or_uri).await?),
    None => None,
  };

  let provider = Yanshi::text2video_for(&model).await?;
  println!(
    "🎞️  Submitting video job via provider '{}' (model '{}')...",
    provider.name(),
    model
  );

  let request = Text2VideoRequest {
    model: model.clone(),
    prompt,
    image: input_image,
    aspect_ratio,
    duration_seconds: duration,
    resolution,
    seed,
    extra: None,
  };

  let start_time = std::time::Instant::now();
  let response = provider
    .generate_and_wait(
      request,
      Duration::from_secs(poll_interval.max(1)),
      Duration::from_secs(max_wait.max(1)),
    )
    .await?;

  let first_video = response
    .videos
    .first()
    .ok_or_else(|| anyhow::anyhow!("No video returned from provider"))?;

  let bytes = provider
    .fetch_video_bytes(first_video, MAX_VIDEO_DOWNLOAD_BYTES)
    .await?;

  if let Some(parent) = Path::new(&output)
    .parent()
    .filter(|p| !p.as_os_str().is_empty())
  {
    fs::create_dir_all(parent).await?;
  }
  fs::write(&output, &bytes).await?;

  println!(
    "✅ Video generated in {:?} ({} KB) -> {}",
    start_time.elapsed(),
    bytes.len() / 1024,
    output
  );
  Ok(())
}

async fn load_input_image(path_or_uri: &str) -> Result<VideoInputImage> {
  if let Some(rest) = path_or_uri.strip_prefix("data:") {
    let (mime, b64) = rest
      .split_once(";base64,")
      .ok_or_else(|| anyhow::anyhow!("data: URI must use ;base64,"))?;
    return Ok(VideoInputImage {
      bytes_base64_encoded: b64.to_string(),
      mime_type: if mime.is_empty() {
        "image/png".to_string()
      } else {
        mime.to_string()
      },
    });
  }
  let bytes = fs::read(path_or_uri).await?;
  let lower = path_or_uri.to_ascii_lowercase();
  let mime_type = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
    "image/jpeg"
  } else if lower.ends_with(".webp") {
    "image/webp"
  } else {
    "image/png"
  };
  Ok(VideoInputImage {
    bytes_base64_encoded: general_purpose::STANDARD.encode(&bytes),
    mime_type: mime_type.to_string(),
  })
}
