use anyhow::Result;
use base64::{Engine as _, engine::general_purpose};
use tokio::fs;
use yanshi_llm::{
  Yanshi,
  providers::modality::{
    Image2ImageRequest as ModalityImage2ImageRequest,
    Text2ImageRequest as ModalityText2ImageRequest,
  },
};

#[allow(clippy::too_many_arguments)]
pub async fn execute(
  prompt: String,
  model: Option<String>,
  size: String,
  output: String,
  format: String,
  steps: u32,
  cfg_scale: f32,
  seed: Option<u64>,
  strength: Option<f32>,
  input_image: Option<String>,
) -> Result<()> {
  let model = model.unwrap_or_else(|| "gpt-image-2".to_string());

  // Initialize Yanshi so the registry knows about all configured
  // models. The dispatcher resolves vendor + API key by model name.
  Yanshi::init().await?;

  println!("🎨 Yanshi Image Generation");
  println!("Model: {}", model);
  println!("Prompt: {}", prompt);
  println!("Size: {}", size);
  println!("Format: {}", format);
  println!("Output: {}", output);
  if let Some(seed) = seed {
    println!("Seed: {}", seed);
  }
  if let Some(input) = &input_image {
    println!("Input image: {}", input);
  }
  println!();

  let start_time = std::time::Instant::now();
  let response = if let Some(input_path_or_url) = input_image {
    let source_url = if input_path_or_url.starts_with("http://")
      || input_path_or_url.starts_with("https://")
      || input_path_or_url.starts_with("data:")
    {
      input_path_or_url
    } else {
      let bytes = fs::read(&input_path_or_url).await?;
      let lower = input_path_or_url.to_ascii_lowercase();
      let mime = if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
      } else if lower.ends_with(".webp") {
        "image/webp"
      } else {
        "image/png"
      };
      format!(
        "data:{mime};base64,{}",
        general_purpose::STANDARD.encode(&bytes)
      )
    };

    let provider = Yanshi::image2image(&model).await?;
    println!(
      "🖼️  Transforming via provider '{}' (model '{}')...",
      provider.name(),
      model
    );
    let request = ModalityImage2ImageRequest {
      model: model.clone(),
      prompt,
      source_url,
      source_weight: strength.unwrap_or(0.7),
      size: Some(size.clone()),
      n: Some(1),
      response_format: Some(format.clone()),
      seed: seed.map(|s| s as i32),
      steps: Some(steps),
      cfg_scale: Some(cfg_scale),
    };
    tokio::time::timeout(
      std::time::Duration::from_secs(120),
      provider.transform(request),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Image-to-image generation timed out after 2 minutes"))?
    .map_err(|e| anyhow::anyhow!("Image-to-image generation failed: {}", e))?
  } else {
    let provider = Yanshi::text2image_for(&model).await?;
    println!(
      "🖼️  Generating via provider '{}' (model '{}')...",
      provider.name(),
      model
    );

    let request = ModalityText2ImageRequest {
      model: model.clone(),
      prompt,
      size: Some(size.clone()),
      n: Some(1),
      response_format: Some(format.clone()),
      seed: seed.map(|s| s as i32),
      steps: Some(steps),
      cfg_scale: Some(cfg_scale),
    };

    tokio::time::timeout(
      std::time::Duration::from_secs(120),
      provider.generate(request),
    )
    .await
    .map_err(|_| {
      anyhow::anyhow!(
        "Image generation timed out after 2 minutes. Try again with a simpler prompt or check your internet connection."
      )
    })?
    .map_err(|e| {
      eprintln!("Error: Internal LLM error: {}", e);
      anyhow::anyhow!("Image generation failed: {}", e)
    })?
  };

  let duration = start_time.elapsed();
  println!("✅ Image generated in {:?}", duration);
  println!();

  println!("💾 Saving image to: {}", output);

  let first_image = response
    .images
    .first()
    .ok_or_else(|| anyhow::anyhow!("No image data received in response"))?;

  match format.as_str() {
    "b64_json" => {
      let b64_data = first_image
        .b64_json
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No base64 image data received"))?;
      let image_bytes = general_purpose::STANDARD.decode(b64_data)?;
      fs::write(&output, &image_bytes).await?;
      println!(
        "✅ Image saved as base64 data ({} bytes)",
        image_bytes.len()
      );
    }
    "url" => {
      let url = first_image
        .url
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No image URL received"))?;
      let url_output = format!("{}.url", output);
      fs::write(&url_output, url).await?;
      println!("✅ Image URL saved to: {}", url_output);
      println!("🔗 Image URL: {}", url);
    }
    other => {
      return Err(anyhow::anyhow!("Unsupported format: {}", other));
    }
  }

  if let Some(seed) = first_image.seed {
    println!("🎯 Generation seed: {}", seed);
  }

  println!("🎉 Image generation completed successfully!");
  Ok(())
}
