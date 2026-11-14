use anyhow::Result;
use base64::Engine;
use tokio::fs;
use yanshi_llm::{Yanshi, multimodal::MultimodalMessage};

pub async fn execute(
  image_path: String,
  prompt: String,
  model: Option<String>,
  temperature: Option<f32>,
  max_tokens: Option<u32>,
  output: Option<String>,
) -> Result<()> {
  let model = model.unwrap_or_else(|| "gemini-4-hydrogen".to_string());

  Yanshi::init().await?;

  println!("👁️  Yanshi Image Understanding");
  println!("Model: {}", model);
  println!("Image: {}", image_path);
  println!("Prompt: {}", prompt);
  if let Some(output) = &output {
    println!("Output: {}", output);
  }
  println!();

  if !std::path::Path::new(&image_path).exists() {
    return Err(anyhow::anyhow!("Image file not found: {}", image_path));
  }

  println!("📸 Reading image file...");
  let image_bytes = fs::read(&image_path).await?;
  let image_base64 = base64::engine::general_purpose::STANDARD.encode(&image_bytes);

  let image_format = match image_path.to_lowercase().split('.').next_back() {
    Some("jpg") | Some("jpeg") => "image/jpeg",
    Some("png") => "image/png",
    Some("gif") => "image/gif",
    Some("webp") => "image/webp",
    Some("bmp") => "image/bmp",
    _ => "image/png",
  };

  println!("🖼️  Image format: {}", image_format);
  println!("💾 Image size: {} bytes", image_bytes.len());

  let data_uri = format!("data:{image_format};base64,{image_base64}");
  let message = MultimodalMessage::user()
    .add_text(&prompt)
    .add_image_url(&data_uri)
    .build();

  println!("🚀 Analyzing image...");
  let start_time = std::time::Instant::now();

  let mut request = Yanshi::model(&model).multimodal_prompt(message);
  if let Some(temp) = temperature {
    request = request.temperature(temp);
  }
  if let Some(tokens) = max_tokens {
    request = request.max_tokens(tokens);
  }
  let response_text = request.execute().await?;

  let duration = start_time.elapsed();
  println!("✅ Analysis completed in {:?}", duration);
  println!();

  println!("📝 Analysis Results:");
  println!("===================");
  println!("{}", response_text);
  println!();

  if let Some(output_path) = output {
    println!("💾 Saving results to: {}", output_path);
    let output_content = format!(
      "# Image Analysis Results\n\n**Model:** {}\n**Image:** {}\n**Prompt:** {}\n\n**Analysis:**\n{}\n",
      model, image_path, prompt, response_text
    );

    fs::write(&output_path, &output_content).await?;
    println!("✅ Results saved successfully");
  }

  println!("🎉 Image understanding completed successfully!");
  Ok(())
}
