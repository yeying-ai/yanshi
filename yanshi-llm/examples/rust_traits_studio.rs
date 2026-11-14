//! End-to-end multimodal course generator: "Rust Traits Deep Dive".
//!
//! Demonstrates Yanshi's unified multimodal Rust SDK across five capabilities
//! in a single reproducible pipeline:
//!   1. Chat LLM (`Yanshi::model`) — live compiler/architect commentary per chapter
//!   2. Text-to-Image (`Yanshi::text2image_for`) — 6 × 16:9 chapter illustrations
//!   3. Vision VLM (`MultimodalMessage` + `add_image_data`) — visual metaphor review
//!   4. Image-to-Video (`Yanshi::text2video_for` + `VideoInputImage`) — 3 × Veo I2V clips
//!   5. Text-to-Speech (`Yanshi::tts`) — 6 × WAV Chinese narration + FFmpeg MP4/SRT stitch
//!
//! ## Offline mode (default — zero API keys, safe for CI)
//! ```bash
//! cargo run -p yanshi-llm --example rust_traits_studio
//! ```
//!
//! ## Live production mode (Google Gemini / Vertex AI via `GEMINI_API_KEY`)
//! ```bash
//! YANSHI_LIVE_PROVIDER=1 \
//! YANSHI_OUTPUT_DIR=runs/rust-traits-course \
//!   cargo run -p yanshi-llm --example rust_traits_studio
//! ```

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use yanshi_llm::{
  MultimodalMessage, Text2ImageRequest, Text2VideoRequest, TtsRequest, VideoInputImage, Yanshi,
};

struct StudioModels {
  live: bool,
  chat: String,
  image: String,
  video: String,
  tts: String,
  voice: String,
}

struct ChapterSpec {
  index: usize,
  slug: &'static str,
  heading: &'static str,
  subtitle: &'static str,
  narration: &'static str,
  visual_prompt: &'static str,
  motion_prompt: Option<&'static str>,
  markdown: &'static str,
}

struct RenderedChapter {
  spec: ChapterSpec,
  architect_note: String,
  vlm_caption: String,
  image_rel: String,
  video_rel: Option<String>,
  audio_rel: String,
  audio_seconds: f64,
}

#[derive(Clone)]
struct SubtitleCue {
  start_secs: f64,
  end_secs: f64,
  text: String,
}

const VISUAL_STYLE: &str = "dark slate systems-engineering architectural blueprint illustration, \
  glowing rust-copper (#e46b39) and cyan (#38bdf8) energy conduits, crisp isometric precision, \
  subtle depth of field, cinematic 16:9 composition, strictly no text, no letters, no numbers";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let live = env::var("YANSHI_LIVE_PROVIDER")
    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    .unwrap_or(false);

  let output_dir = env::var_os("YANSHI_OUTPUT_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      if live {
        PathBuf::from("runs/rust-traits-course")
      } else {
        env::temp_dir().join("yanshi-rust-traits-mock")
      }
    });
  let assets_dir = output_dir.join("assets");
  tokio::fs::create_dir_all(&assets_dir).await?;

  let models = init_models(live, &output_dir).await?;
  println!(
    "=== Rust Traits Multimodal Studio ({}) ===",
    if models.live {
      "LIVE Vertex AI / Gemini"
    } else {
      "OFFLINE Mock Provider"
    }
  );
  println!("Output directory: {}", output_dir.display());

  let specs = curriculum();
  let image_provider = Yanshi::text2image_for(&models.image).await?;
  let video_provider = Yanshi::text2video_for(&models.video).await?;
  let tts_provider = Yanshi::tts(&models.tts).await?;

  let mut chapters = Vec::with_capacity(specs.len());

  let force_regen = env::var("YANSHI_FORCE_REGEN")
    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    .unwrap_or(false);

  for spec in specs {
    println!("\n[Chapter {}/6] {}", spec.index, spec.heading);

    // 1. Live LLM architect commentary (or deterministic mock response).
    // Note: reasoning-capable models (e.g. gemini-4-hydrogen) spend part of
    // maxOutputTokens on internal thoughts, so keep max_tokens generous and
    // avoid rigid character-counting constraints that trigger long CoT loops.
    let commentary_prompt = format!(
      "你是资深 Rust 编译器与系统架构师。请针对《{}》这一节，用三句精炼深刻、一针见血的中文给出\
       “架构师实战判据”（不要使用代码块或标题，直接输出核心洞见）：\n{}",
      spec.heading, spec.subtitle
    );
    let architect_note = Yanshi::model(&models.chat)
      .system("你是严谨、克制的 Rust 系统工程专家，只输出高密度技术洞察。")
      .prompt(&commentary_prompt)
      .temperature(0.4)
      .max_tokens(16384)
      .execute()
      .await?;
    println!(
      "  ✓ Architect commentary ready ({} chars)",
      architect_note.trim().chars().count()
    );

    // 2. 16:9 HD chapter illustration (reuses cached asset in live mode unless YANSHI_FORCE_REGEN=1).
    let image_filename = format!("ch{:02}.png", spec.index);
    let image_path = assets_dir.join(&image_filename);
    let image_bytes = if models.live && !force_regen && file_non_empty(&image_path) {
      let cached = tokio::fs::read(&image_path).await?;
      println!(
        "  ↺ Reusing cached illustration: assets/{} ({} KB)",
        image_filename,
        cached.len() / 1024
      );
      cached
    } else {
      let full_visual_prompt = format!("{}. Style: {}.", spec.visual_prompt, VISUAL_STYLE);
      let image_resp = image_provider
        .generate(Text2ImageRequest {
          model: models.image.clone(),
          prompt: full_visual_prompt,
          size: Some("16:9".to_string()),
          n: Some(1),
          response_format: Some("b64_json".to_string()),
          seed: None,
          steps: None,
          cfg_scale: None,
        })
        .await?;
      let first_image = image_resp
        .images
        .first()
        .ok_or_else(|| anyhow::anyhow!("image provider returned empty images list"))?;
      let generated = materialise_image_bytes(first_image).await?;
      tokio::fs::write(&image_path, &generated).await?;
      println!(
        "  ✓ Illustration saved: assets/{} ({} KB)",
        image_filename,
        generated.len() / 1024
      );
      generated
    };

    // 3. Multimodal VLM visual metaphor review of the generated keyframe.
    let vlm_message = MultimodalMessage::user()
      .add_text(format!(
        "这张配图对应 Rust Traits 教程的第 {} 章《{}》。请用两句优美凝练的中文解说画面中的几何结构与光线隐喻是如何呼应本章底层机制的（不要带标题或前缀）。",
        spec.index, spec.heading
      ))
      .add_image_data(BASE64.encode(&image_bytes), "image/png".to_string())
      .build();
    let vlm_caption = Yanshi::model(&models.chat)
      .multimodal_prompt(vlm_message)
      .temperature(0.3)
      .max_tokens(16384)
      .execute()
      .await?;
    println!(
      "  ✓ VLM visual review ready ({} chars)",
      vlm_caption.trim().chars().count()
    );

    // 4. Selected Veo 3.1 Image-to-Video (I2V) 6s mechanism animation.
    let video_rel = if let Some(motion_prompt) = spec.motion_prompt {
      let video_filename = format!("ch{:02}_{}.mp4", spec.index, spec.slug);
      let video_path = assets_dir.join(&video_filename);
      if models.live && !force_regen && file_non_empty(&video_path) {
        let meta_len = std::fs::metadata(&video_path).map(|m| m.len()).unwrap_or(0);
        println!(
          "  ↺ Reusing cached I2V motion clip: assets/{} ({} KB)",
          video_filename,
          meta_len / 1024
        );
      } else {
        println!("  ⏳ Submitting I2V clip: assets/{} ...", video_filename);
        let video_resp = video_provider
          .generate_and_wait(
            Text2VideoRequest {
              model: models.video.clone(),
              prompt: motion_prompt.to_string(),
              image: Some(VideoInputImage {
                bytes_base64_encoded: BASE64.encode(&image_bytes),
                mime_type: "image/png".to_string(),
              }),
              aspect_ratio: Some("16:9".to_string()),
              duration_seconds: Some(6),
              resolution: None,
              seed: None,
              extra: None,
            },
            Duration::from_secs(if models.live { 8 } else { 1 }),
            Duration::from_secs(420),
          )
          .await?;
        let first_video = video_resp
          .videos
          .first()
          .ok_or_else(|| anyhow::anyhow!("video provider returned empty videos list"))?;
        let video_bytes = video_provider
          .fetch_video_bytes(first_video, 200 * 1024 * 1024)
          .await?;
        tokio::fs::write(&video_path, &video_bytes).await?;
        println!(
          "  ✓ I2V motion clip saved: assets/{} ({} KB)",
          video_filename,
          video_bytes.len() / 1024
        );
      }
      Some(format!("assets/{video_filename}"))
    } else {
      None
    };

    // 5. Chinese narration voiceover (RIFF/WAV).
    let audio_filename = format!("ch{:02}.wav", spec.index);
    let audio_path = assets_dir.join(&audio_filename);
    let audio_bytes = if models.live && !force_regen && file_non_empty(&audio_path) {
      let cached = tokio::fs::read(&audio_path).await?;
      let secs = wav_duration_seconds(&cached)
        .unwrap_or_else(|| (spec.narration.chars().count() as f64 * 0.22).max(1.0));
      println!(
        "  ↺ Reusing cached narration audio: assets/{} ({:.1}s, {} KB)",
        audio_filename,
        secs,
        cached.len() / 1024
      );
      cached
    } else {
      let tts_resp = tts_provider
        .synthesize(TtsRequest {
          model: models.tts.clone(),
          input: spec.narration.to_string(),
          voice: models.voice.clone(),
          response_format: Some("wav".to_string()),
          speed: None,
          volume: None,
          sample_rate: None,
        })
        .await?;
      tokio::fs::write(&audio_path, &tts_resp.audio).await?;
      let secs = wav_duration_seconds(&tts_resp.audio)
        .unwrap_or_else(|| (spec.narration.chars().count() as f64 * 0.22).max(1.0));
      println!(
        "  ✓ Narration audio saved: assets/{} ({:.1}s, {} KB)",
        audio_filename,
        secs,
        tts_resp.audio.len() / 1024
      );
      tts_resp.audio
    };
    let audio_seconds = wav_duration_seconds(&audio_bytes)
      .unwrap_or_else(|| (spec.narration.chars().count() as f64 * 0.22).max(1.0));

    chapters.push(RenderedChapter {
      spec,
      architect_note: architect_note.trim().to_string(),
      vlm_caption: vlm_caption.trim().to_string(),
      image_rel: format!("assets/{image_filename}"),
      video_rel,
      audio_rel: format!("assets/{audio_filename}"),
      audio_seconds,
    });
  }

  // 6. Subtitle tracks (.srt + .vtt) and full narrated MP4 assembly via FFmpeg.
  let cues = build_subtitle_cues(&chapters);
  let srt_text = render_srt(&cues);
  let vtt_text = render_vtt(&cues);
  let srt_path = output_dir.join("full_course.srt");
  let vtt_path = output_dir.join("full_course.vtt");
  tokio::fs::write(&srt_path, &srt_text).await?;
  tokio::fs::write(&vtt_path, &vtt_text).await?;

  let mp4_path = output_dir.join("full_course.mp4");
  let full_video_built = stitch_full_course_mp4(&output_dir, &chapters, &srt_path, &mp4_path)?;

  // 7. Markdown + self-contained responsive HTML portal.
  let markdown_doc = build_markdown_course(&chapters, full_video_built);
  let md_path = output_dir.join("tutorial.md");
  tokio::fs::write(&md_path, &markdown_doc).await?;

  let html_doc = build_html_course(&chapters, &vtt_text, full_video_built);
  let html_path = output_dir.join("index.html");
  tokio::fs::write(&html_path, &html_doc).await?;

  let total_seconds: f64 = chapters.iter().map(|c| c.audio_seconds).sum();
  println!("\n=== Course Generation Complete ===");
  println!("  • Markdown guide : {}", md_path.display());
  println!("  • HTML portal    : {}", html_path.display());
  println!("  • Subtitles      : {}", srt_path.display());
  if full_video_built {
    println!(
      "  • Full course MP4: {} ({:.1}s across 6 chapters)",
      mp4_path.display(),
      total_seconds
    );
  } else {
    println!("  • Full course MP4: skipped (install `ffmpeg` on PATH to stitch MP4)");
  }

  Ok(())
}

async fn init_models(live: bool, output_dir: &Path) -> anyhow::Result<StudioModels> {
  if live {
    if let Some(home) = dirs::home_dir() {
      let env_file = home.join(".yanshi").join(".env");
      if env_file.exists() {
        dotenvy::from_path(&env_file).ok();
      }
    }
    if env::var_os("YANSHI_MODELS_CONFIG").is_some() {
      Yanshi::init().await?;
    } else {
      Yanshi::init_with_builtin_config().await?;
    }
    Ok(StudioModels {
      live: true,
      chat: env::var("YANSHI_CHAT_MODEL").unwrap_or_else(|_| "gemini-4-hydrogen".to_string()),
      image: env::var("YANSHI_IMAGE_MODEL")
        .unwrap_or_else(|_| "gemini-3.1-flash-image".to_string()),
      video: env::var("YANSHI_VIDEO_MODEL")
        .unwrap_or_else(|_| "veo-3.1-fast-generate-001".to_string()),
      tts: env::var("YANSHI_TTS_MODEL")
        .unwrap_or_else(|_| "gemini-2.5-flash-preview-tts".to_string()),
      voice: env::var("YANSHI_TTS_VOICE").unwrap_or_else(|_| "Kore".to_string()),
    })
  } else {
    let mock_cfg = output_dir.join("mock_models.yml");
    let yaml = concat!(
      "models:\n",
      "  mock-chat:\n",
      "    vendor: mock\n",
      "    type: chat\n",
      "    accepts: [text, image]\n",
      "    supports_multimodal: true\n",
      "    model_id: mock-chat\n",
      "  mock-t2i:\n",
      "    vendor: mock\n",
      "    type: text_to_image\n",
      "    model_id: mock-t2i\n",
      "  mock-t2v:\n",
      "    vendor: mock\n",
      "    type: text_to_video\n",
      "    accepts: [text, image]\n",
      "    model_id: mock-t2v\n",
      "  mock-tts:\n",
      "    vendor: mock\n",
      "    type: tts\n",
      "    model_id: mock-tts\n",
      "providers:\n",
      "  mock:\n",
      "    api_key_env: MOCK_API_KEY\n",
    );
    tokio::fs::write(&mock_cfg, yaml).await?;
    let cfg_str = mock_cfg
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("non-UTF-8 mock config path"))?;
    Yanshi::init_with_config(cfg_str).await?;
    Ok(StudioModels {
      live: false,
      chat: "mock-chat".to_string(),
      image: "mock-t2i".to_string(),
      video: "mock-t2v".to_string(),
      tts: "mock-tts".to_string(),
      voice: "Kore".to_string(),
    })
  }
}

fn file_non_empty(path: &Path) -> bool {
  std::fs::metadata(path)
    .map(|m| m.is_file() && m.len() > 0)
    .unwrap_or(false)
}

async fn materialise_image_bytes(image: &yanshi_llm::GeneratedImage) -> anyhow::Result<Vec<u8>> {
  if let Some(b64) = &image.b64_json {
    let cleaned = b64
      .split_once(',')
      .map(|(_, body)| body)
      .unwrap_or(b64.as_str());
    return Ok(BASE64.decode(cleaned.trim())?);
  }
  if let Some(url) = &image.url {
    let (bytes, _) =
      yanshi_llm::providers::modality::download_with_cap_default(url, 32 * 1024 * 1024).await?;
    return Ok(bytes);
  }
  anyhow::bail!("generated image contained neither b64_json nor url")
}

/// Parse exact playback duration in seconds from a RIFF/WAVE byte buffer.
fn wav_duration_seconds(bytes: &[u8]) -> Option<f64> {
  if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    return None;
  }
  let mut cursor = 12usize;
  let mut byte_rate: Option<u32> = None;
  let mut data_len: Option<u32> = None;

  while cursor + 8 <= bytes.len() {
    let chunk_id = &bytes[cursor..cursor + 4];
    let chunk_size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().ok()?);
    let body_start = cursor + 8;
    if chunk_id == b"fmt " && body_start + 12 <= bytes.len() {
      let rate = u32::from_le_bytes(bytes[body_start + 8..body_start + 12].try_into().ok()?);
      if rate > 0 {
        byte_rate = Some(rate);
      }
    } else if chunk_id == b"data" {
      let available = (bytes.len().saturating_sub(body_start)) as u32;
      data_len = Some(chunk_size.min(available));
      break;
    }
    let padded = (chunk_size as usize) + ((chunk_size as usize) & 1);
    cursor = body_start.checked_add(padded)?;
  }

  let secs = f64::from(data_len?) / f64::from(byte_rate?);
  (secs.is_finite() && secs > 0.0).then_some(secs)
}

fn split_chinese_sentences(text: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut buf = String::new();
  for ch in text.chars() {
    buf.push(ch);
    if matches!(ch, '。' | '！' | '？' | '；' | '\n') {
      let trimmed = buf.trim();
      if !trimmed.is_empty() {
        out.push(trimmed.to_string());
      }
      buf.clear();
    }
  }
  let tail = buf.trim();
  if !tail.is_empty() {
    out.push(tail.to_string());
  }
  out
}

fn build_subtitle_cues(chapters: &[RenderedChapter]) -> Vec<SubtitleCue> {
  let mut cues = Vec::new();
  let mut timeline = 0.0f64;
  for chapter in chapters {
    let sentences = split_chinese_sentences(chapter.spec.narration);
    let total_chars: usize = sentences.iter().map(|s| s.chars().count()).sum();
    let mut cursor = timeline;
    for (idx, sentence) in sentences.iter().enumerate() {
      let share = if idx + 1 == sentences.len() {
        (timeline + chapter.audio_seconds - cursor).max(0.2)
      } else {
        chapter.audio_seconds * (sentence.chars().count() as f64) / (total_chars.max(1) as f64)
      };
      cues.push(SubtitleCue {
        start_secs: cursor,
        end_secs: cursor + share,
        text: format!("【第{}章】{}", chapter.spec.index, sentence),
      });
      cursor += share;
    }
    timeline += chapter.audio_seconds;
  }
  cues
}

fn format_timestamp(seconds: f64, decimal_sep: char) -> String {
  let total_ms = (seconds.max(0.0) * 1000.0).round() as u64;
  let ms = total_ms % 1000;
  let total_secs = total_ms / 1000;
  let s = total_secs % 60;
  let m = (total_secs / 60) % 60;
  let h = total_secs / 3600;
  format!("{h:02}:{m:02}:{s:02}{decimal_sep}{ms:03}")
}

fn render_srt(cues: &[SubtitleCue]) -> String {
  let mut out = String::new();
  for (idx, cue) in cues.iter().enumerate() {
    let _ = writeln!(
      out,
      "{}\n{} --> {}\n{}\n",
      idx + 1,
      format_timestamp(cue.start_secs, ','),
      format_timestamp(cue.end_secs, ','),
      cue.text
    );
  }
  out
}

fn render_vtt(cues: &[SubtitleCue]) -> String {
  let mut out = String::from("WEBVTT\n\n");
  for (idx, cue) in cues.iter().enumerate() {
    let _ = writeln!(
      out,
      "{}\n{} --> {}\n{}\n",
      idx + 1,
      format_timestamp(cue.start_secs, '.'),
      format_timestamp(cue.end_secs, '.'),
      cue.text
    );
  }
  out
}

fn ffmpeg_available() -> bool {
  Command::new("ffmpeg")
    .arg("-version")
    .output()
    .map(|o| o.status.success())
    .unwrap_or(false)
}

fn stitch_full_course_mp4(
  output_dir: &Path,
  chapters: &[RenderedChapter],
  srt_path: &Path,
  mp4_path: &Path,
) -> anyhow::Result<bool> {
  if !ffmpeg_available() {
    return Ok(false);
  }

  let work_dir = output_dir.join(".ffmpeg-work");
  std::fs::create_dir_all(&work_dir)?;

  let filter = "scale=1280:720:force_original_aspect_ratio=decrease,\
    pad=1280:720:(ow-iw)/2:(oh-ih)/2:color=0x0b1120,setsar=1,fps=25,format=yuv420p";

  let mut concat_manifest = String::new();

  let stitch_result = (|| -> anyhow::Result<()> {
    for (idx, chapter) in chapters.iter().enumerate() {
      let segment_name = format!("seg_{idx:02}.mp4");
      let segment_path = work_dir.join(&segment_name);
      let audio_path = output_dir.join(&chapter.audio_rel);

      let mut cmd = Command::new("ffmpeg");
      cmd
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error");

      if let Some(video_rel) = &chapter.video_rel {
        cmd
          .arg("-stream_loop")
          .arg("-1")
          .arg("-i")
          .arg(output_dir.join(video_rel));
      } else {
        cmd
          .arg("-loop")
          .arg("1")
          .arg("-i")
          .arg(output_dir.join(&chapter.image_rel));
      }

      let status = cmd
        .arg("-i")
        .arg(&audio_path)
        .arg("-map")
        .arg("0:v:0")
        .arg("-map")
        .arg("1:a:0")
        .arg("-vf")
        .arg(filter)
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("veryfast")
        .arg("-c:a")
        .arg("aac")
        .arg("-b:a")
        .arg("160k")
        .arg("-ar")
        .arg("44100")
        .arg("-ac")
        .arg("2")
        .arg("-t")
        .arg(format!("{:.3}", chapter.audio_seconds))
        .arg(&segment_path)
        .status()?;

      if !status.success() {
        anyhow::bail!("ffmpeg failed while encoding segment {}", idx + 1);
      }
      let _ = writeln!(concat_manifest, "file '{segment_name}'");
    }

    let list_path = work_dir.join("concat.txt");
    std::fs::write(&list_path, &concat_manifest)?;

    let status = Command::new("ffmpeg")
      .arg("-y")
      .arg("-hide_banner")
      .arg("-loglevel")
      .arg("error")
      .arg("-f")
      .arg("concat")
      .arg("-safe")
      .arg("0")
      .arg("-i")
      .arg(&list_path)
      .arg("-i")
      .arg(srt_path)
      .arg("-map")
      .arg("0:v")
      .arg("-map")
      .arg("0:a")
      .arg("-map")
      .arg("1:0")
      .arg("-c:v")
      .arg("copy")
      .arg("-c:a")
      .arg("copy")
      .arg("-c:s")
      .arg("mov_text")
      .arg("-metadata:s:s:0")
      .arg("language=zho")
      .arg("-movflags")
      .arg("+faststart")
      .arg(mp4_path)
      .status()?;

    if !status.success() {
      anyhow::bail!("ffmpeg failed while muxing full_course.mp4");
    }
    Ok(())
  })();

  let _ = std::fs::remove_dir_all(&work_dir);
  stitch_result?;
  Ok(true)
}

fn build_markdown_course(chapters: &[RenderedChapter], full_video_built: bool) -> String {
  let total_secs: f64 = chapters.iter().map(|c| c.audio_seconds).sum();
  let mut md = String::new();
  let _ = writeln!(
    md,
    "# Rust Traits 深度讲解：从类型契约到零成本系统架构\n\n\
     > 本专题由 **Yanshi 多模态管线**生成，涵盖 **6 章系统级深度讲义**、**6 幅 16:9 架构概念图**、\
     **3 段核心机制 Veo 动态演示视频**与**全程中文 TTS 配音及字幕轨**。\n"
  );
  if full_video_built {
    let _ = writeln!(
      md,
      "- **完整配音字幕长视频**：[`full_course.mp4`](full_course.mp4)（总时长 `{total_secs:.1}s`）"
    );
  }
  let _ = writeln!(
    md,
    "- **SRT 外挂字幕轨**：[`full_course.srt`](full_course.srt)\n\
     - **WebVTT 网页字幕轨**：[`full_course.vtt`](full_course.vtt)\n\
     - **交互式网页专题页**：[`index.html`](index.html)\n\n\
     ## 课程目录\n"
  );

  for ch in chapters {
    let motion_badge = if ch.video_rel.is_some() {
      " · 🎬 含 Veo 动态机制视频"
    } else {
      ""
    };
    let _ = writeln!(
      md,
      "{}. **{}** — {}{}",
      ch.spec.index, ch.spec.heading, ch.spec.subtitle, motion_badge
    );
  }
  let _ = writeln!(md, "\n---\n");

  for ch in chapters {
    let _ = writeln!(
      md,
      "## 第 {} 章：{}\n\n*{}*\n\n![{}]({})\n\n> 🖼️ **视觉隐喻导读（VLM 审片）**：{}\n",
      ch.spec.index,
      ch.spec.heading,
      ch.spec.subtitle,
      ch.spec.heading,
      ch.image_rel,
      ch.vlm_caption.replace('\n', " ")
    );
    if let Some(video_rel) = &ch.video_rel {
      let _ = writeln!(
        md,
        "> 🎬 **本章动态机制视频（Veo 3.1 I2V）**：[`{video_rel}`]({video_rel})\n"
      );
    }
    let _ = writeln!(
      md,
      "> 🎙️ **语音旁白（`{}` · {:.1}s）**：{}\n\n\
       > 💡 **架构师实战判据**：{}\n\n{}\n\n---\n",
      ch.audio_rel,
      ch.audio_seconds,
      ch.spec.narration,
      ch.architect_note.replace('\n', " "),
      ch.spec.markdown
    );
  }

  md
}

fn escape_html(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len());
  for ch in raw.chars() {
    match ch {
      '&' => out.push_str("&amp;"),
      '<' => out.push_str("&lt;"),
      '>' => out.push_str("&gt;"),
      '"' => out.push_str("&quot;"),
      '\'' => out.push_str("&#39;"),
      _ => out.push(ch),
    }
  }
  out
}

fn format_inline_markdown(line: &str) -> String {
  let escaped = escape_html(line);
  let mut out = String::new();
  let mut in_code = false;
  for part in escaped.split('`') {
    if in_code {
      out.push_str("<code>");
      out.push_str(part);
      out.push_str("</code>");
    } else {
      let mut bold_toggle = false;
      for seg in part.split("**") {
        if bold_toggle {
          out.push_str("<strong>");
          out.push_str(seg);
          out.push_str("</strong>");
        } else {
          out.push_str(seg);
        }
        bold_toggle = !bold_toggle;
      }
    }
    in_code = !in_code;
  }
  out
}

fn markdown_to_html(md: &str) -> String {
  let mut html = String::new();
  let mut in_code = false;
  let mut in_list = false;
  let mut in_table = false;
  let mut table_header_done = false;

  for line in md.lines() {
    if let Some(fence) = line.strip_prefix("```") {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      if in_table {
        html.push_str("</tbody></table></div>\n");
        in_table = false;
      }
      if in_code {
        html.push_str("</code></pre>\n");
        in_code = false;
      } else {
        let code_lang = fence.trim();
        let _ = writeln!(
          html,
          "<div class=\"code-header\"><span>{}</span></div><pre><code class=\"lang-{}\">",
          if code_lang.is_empty() {
            "text"
          } else {
            code_lang
          },
          escape_html(code_lang)
        );
        in_code = true;
      }
      continue;
    }

    if in_code {
      let _ = writeln!(html, "{}", escape_html(line));
      continue;
    }

    let trimmed = line.trim();
    if trimmed.is_empty() {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      if in_table {
        html.push_str("</tbody></table></div>\n");
        in_table = false;
      }
      continue;
    }

    if trimmed.starts_with('|') && trimmed.ends_with('|') {
      let cells: Vec<&str> = trimmed[1..trimmed.len() - 1]
        .split('|')
        .map(str::trim)
        .collect();
      let is_divider = cells
        .iter()
        .all(|c| !c.is_empty() && c.chars().all(|ch| matches!(ch, '-' | ':' | ' ')));
      if is_divider {
        continue;
      }
      if !in_table {
        html.push_str("<div class=\"table-wrap\"><table><thead><tr>");
        for cell in &cells {
          let _ = write!(html, "<th>{}</th>", format_inline_markdown(cell));
        }
        html.push_str("</tr></thead><tbody>\n");
        in_table = true;
        table_header_done = true;
      } else if table_header_done {
        html.push_str("<tr>");
        for cell in &cells {
          let _ = write!(html, "<td>{}</td>", format_inline_markdown(cell));
        }
        html.push_str("</tr>\n");
      }
      continue;
    }

    if in_table {
      html.push_str("</tbody></table></div>\n");
      in_table = false;
    }

    if let Some(h3) = trimmed.strip_prefix("### ") {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      let _ = writeln!(html, "<h3>{}</h3>", format_inline_markdown(h3));
    } else if let Some(h4) = trimmed.strip_prefix("#### ") {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      let _ = writeln!(html, "<h4>{}</h4>", format_inline_markdown(h4));
    } else if let Some(bullet) = trimmed.strip_prefix("- ") {
      if !in_list {
        html.push_str("<ul>\n");
        in_list = true;
      }
      let _ = writeln!(html, "  <li>{}</li>", format_inline_markdown(bullet));
    } else if let Some(quote) = trimmed.strip_prefix("> ") {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      let _ = writeln!(
        html,
        "<blockquote>{}</blockquote>",
        format_inline_markdown(quote)
      );
    } else {
      if in_list {
        html.push_str("</ul>\n");
        in_list = false;
      }
      let _ = writeln!(html, "<p>{}</p>", format_inline_markdown(trimmed));
    }
  }

  if in_code {
    html.push_str("</code></pre>\n");
  }
  if in_list {
    html.push_str("</ul>\n");
  }
  if in_table {
    html.push_str("</tbody></table></div>\n");
  }
  html
}

fn build_html_course(
  chapters: &[RenderedChapter],
  vtt_text: &str,
  full_video_built: bool,
) -> String {
  let total_secs: f64 = chapters.iter().map(|c| c.audio_seconds).sum();
  let vtt_data_uri = format!(
    "data:text/vtt;base64,{}",
    BASE64.encode(vtt_text.as_bytes())
  );

  let mut nav_items = String::new();
  let mut chapter_cards = String::new();

  for ch in chapters {
    let badge = if ch.video_rel.is_some() {
      "<span class=\"pill pill-video\">Veo I2V</span>"
    } else {
      "<span class=\"pill\">16:9 图解</span>"
    };
    let _ = writeln!(
      nav_items,
      "        <a class=\"nav-link\" href=\"#ch-{idx}\"><span class=\"nav-num\">0{idx}</span>\
       <span class=\"nav-title\">{title}</span>{badge}</a>",
      idx = ch.spec.index,
      title = escape_html(ch.spec.heading),
      badge = badge
    );

    let video_panel = if let Some(video_rel) = &ch.video_rel {
      format!(
        "          <div class=\"media-card\">\n\
                   <div class=\"media-tag\">Veo 3.1 首帧锚定动态推演 (6s I2V Loop)</div>\n\
                   <video controls loop muted playsinline preload=\"metadata\" poster=\"{img}\">\n\
                     <source src=\"{vid}\" type=\"video/mp4\" />\n\
                   </video>\n\
                 </div>\n",
        img = escape_html(&ch.image_rel),
        vid = escape_html(video_rel),
      )
    } else {
      String::new()
    };

    let media_grid_class = if ch.video_rel.is_some() {
      "media-grid two-up"
    } else {
      "media-grid"
    };

    let body_html = markdown_to_html(ch.spec.markdown);

    let _ = writeln!(
      chapter_cards,
      "        <section class=\"chapter-card\" id=\"ch-{idx}\">\n\
                 <div class=\"chapter-kicker\">CHAPTER 0{idx} · {dur:.1}s 语音精讲</div>\n\
                 <h2>{idx}. {title}</h2>\n\
                 <p class=\"chapter-sub\">{sub}</p>\n\
                 <div class=\"{grid_class}\">\n\
                   <div class=\"media-card\">\n\
                     <div class=\"media-tag\">16:9 系统架构概念图 (Gemini Image)</div>\n\
                     <img src=\"{img}\" alt=\"{title}\" loading=\"lazy\" />\n\
                     <p class=\"vlm-caption\"><strong>VLM 视觉审片：</strong>{vlm}</p>\n\
                   </div>\n\
       {video_panel}\
                 </div>\n\
                 <div class=\"audio-bar\">\n\
                   <div class=\"audio-meta\">\n\
                     <span class=\"audio-badge\">🎙️ 本章中文语音导读</span>\n\
                     <p>{narration}</p>\n\
                   </div>\n\
                   <audio controls preload=\"none\" src=\"{aud}\"></audio>\n\
                 </div>\n\
                 <div class=\"architect-box\">\n\
                   <div class=\"architect-title\">⚡ 编译器与架构师实战判据（AI 实时深度洞察）</div>\n\
                   <p>{arch}</p>\n\
                 </div>\n\
                 <div class=\"prose\">\n\
       {body}\
                 </div>\n\
               </section>",
      idx = ch.spec.index,
      dur = ch.audio_seconds,
      title = escape_html(ch.spec.heading),
      sub = escape_html(ch.spec.subtitle),
      grid_class = media_grid_class,
      img = escape_html(&ch.image_rel),
      vlm = escape_html(&ch.vlm_caption),
      video_panel = video_panel,
      narration = escape_html(ch.spec.narration),
      aud = escape_html(&ch.audio_rel),
      arch = escape_html(&ch.architect_note),
      body = body_html
    );
  }

  let hero_cinema = if full_video_built {
    format!(
      "      <section class=\"cinema-panel\" id=\"full-video\">\n\
               <div class=\"cinema-header\">\n\
                 <div>\n\
                   <span class=\"eyebrow\">FULL NARRATED MASTERCLASS · {total_secs:.1}s</span>\n\
                   <h2>全 6 章带配音与内嵌中文字幕完整讲解视频</h2>\n\
                 </div>\n\
                 <a class=\"download-btn\" href=\"full_course.srt\" download>下载 .SRT 字幕</a>\n\
               </div>\n\
               <video class=\"master-video\" controls preload=\"metadata\" poster=\"assets/ch03.png\">\n\
                 <source src=\"full_course.mp4\" type=\"video/mp4\" />\n\
                 <track kind=\"subtitles\" srclang=\"zh\" label=\"中文讲解字幕\" src=\"{vtt_data_uri}\" default />\n\
               </video>\n\
             </section>\n"
    )
  } else {
    String::new()
  };

  format!(
    "<!doctype html>\n\
<html lang=\"zh-CN\">\n\
<head>\n\
  <meta charset=\"utf-8\" />\n\
  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n\
  <title>Rust Traits 深度讲解 — Yanshi 多模态系统工程专题</title>\n\
  <style>\n\
    :root {{\n\
      --bg: #070b14;\n\
      --panel: #0f172a;\n\
      --panel-soft: #131e35;\n\
      --border: #1e293b;\n\
      --accent: #ea580c;\n\
      --accent-glow: #fb923c;\n\
      --cyan: #38bdf8;\n\
      --text: #e2e8f0;\n\
      --muted: #94a3b8;\n\
      --code-bg: #060911;\n\
    }}\n\
    * {{ box-sizing: border-box; }}\n\
    html {{ scroll-behavior: smooth; }}\n\
    body {{\n\
      margin: 0;\n\
      background: radial-gradient(circle at 15% 0%, #172036 0%, var(--bg) 55%);\n\
      color: var(--text);\n\
      font-family: -apple-system, BlinkMacSystemFont, \"PingFang SC\", \"Hiragino Sans GB\", \"Microsoft YaHei\", sans-serif;\n\
      line-height: 1.72;\n\
    }}\n\
    .hero {{\n\
      border-bottom: 1px solid var(--border);\n\
      padding: 3.5rem 1.5rem 2.75rem;\n\
      background: linear-gradient(180deg, rgba(234, 88, 12, 0.11), rgba(15, 23, 42, 0));\n\
    }}\n\
    .container {{ max-width: 1280px; margin: 0 auto; }}\n\
    .eyebrow {{\n\
      display: inline-block;\n\
      font-size: 0.76rem;\n\
      font-weight: 700;\n\
      letter-spacing: 0.12em;\n\
      color: var(--accent-glow);\n\
      text-transform: uppercase;\n\
    }}\n\
    .hero h1 {{\n\
      font-size: clamp(1.9rem, 3.6vw, 2.85rem);\n\
      margin: 0.55rem 0 0.75rem;\n\
      line-height: 1.22;\n\
    }}\n\
    .hero-lead {{ max-width: 860px; color: var(--muted); font-size: 1.04rem; margin: 0 0 1.4rem; }}\n\
    .stats-row {{ display: flex; flex-wrap: wrap; gap: 0.75rem; }}\n\
    .stat-chip {{\n\
      background: rgba(15, 23, 42, 0.85);\n\
      border: 1px solid var(--border);\n\
      border-radius: 999px;\n\
      padding: 0.36rem 0.9rem;\n\
      font-size: 0.84rem;\n\
      color: #cbd5e1;\n\
    }}\n\
    .layout {{\n\
      display: grid;\n\
      grid-template-columns: 280px minmax(0, 1fr);\n\
      gap: 2rem;\n\
      padding: 2rem 1.5rem 4rem;\n\
      max-width: 1280px;\n\
      margin: 0 auto;\n\
    }}\n\
    @media (max-width: 980px) {{\n\
      .layout {{ grid-template-columns: 1fr; }}\n\
      .sidebar {{ position: static !important; }}\n\
    }}\n\
    .sidebar {{\n\
      position: sticky;\n\
      top: 1.25rem;\n\
      align-self: start;\n\
      background: var(--panel);\n\
      border: 1px solid var(--border);\n\
      border-radius: 14px;\n\
      padding: 1rem;\n\
    }}\n\
    .sidebar-title {{ font-size: 0.8rem; letter-spacing: 0.08em; color: var(--muted); margin: 0 0 0.75rem; font-weight: 700; }}\n\
    .nav-link {{\n\
      display: flex;\n\
      align-items: center;\n\
      gap: 0.55rem;\n\
      padding: 0.55rem 0.65rem;\n\
      border-radius: 9px;\n\
      color: var(--text);\n\
      text-decoration: none;\n\
      font-size: 0.88rem;\n\
      transition: background 0.15s;\n\
    }}\n\
    .nav-link:hover {{ background: var(--panel-soft); }}\n\
    .nav-num {{ color: var(--accent-glow); font-family: ui-monospace, SFMono-Regular, Menlo, monospace; font-size: 0.78rem; }}\n\
    .nav-title {{ flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }}\n\
    .pill {{\n\
      font-size: 0.68rem;\n\
      padding: 0.1rem 0.42rem;\n\
      border-radius: 6px;\n\
      background: #1e293b;\n\
      color: var(--muted);\n\
    }}\n\
    .pill-video {{ background: rgba(56, 189, 248, 0.16); color: var(--cyan); border: 1px solid rgba(56, 189, 248, 0.35); }}\n\
    .cinema-panel, .chapter-card {{\n\
      background: var(--panel);\n\
      border: 1px solid var(--border);\n\
      border-radius: 16px;\n\
      padding: 1.75rem;\n\
      margin-bottom: 2rem;\n\
      box-shadow: 0 18px 40px rgba(0, 0, 0, 0.28);\n\
    }}\n\
    .cinema-header {{ display: flex; justify-content: space-between; align-items: flex-end; gap: 1rem; margin-bottom: 1rem; flex-wrap: wrap; }}\n\
    .cinema-header h2 {{ margin: 0.25rem 0 0; font-size: 1.35rem; }}\n\
    .download-btn {{\n\
      color: var(--cyan);\n\
      border: 1px solid rgba(56, 189, 248, 0.4);\n\
      padding: 0.4rem 0.85rem;\n\
      border-radius: 8px;\n\
      text-decoration: none;\n\
      font-size: 0.84rem;\n\
    }}\n\
    .master-video {{ width: 100%; border-radius: 12px; background: #000; border: 1px solid var(--border); }}\n\
    .chapter-kicker {{ color: var(--accent-glow); font-size: 0.76rem; font-weight: 700; letter-spacing: 0.1em; }}\n\
    .chapter-card h2 {{ margin: 0.35rem 0 0.35rem; font-size: 1.62rem; }}\n\
    .chapter-sub {{ color: var(--muted); margin: 0 0 1.25rem; font-size: 0.98rem; }}\n\
    .media-grid {{ display: grid; grid-template-columns: 1fr; gap: 1rem; margin-bottom: 1.25rem; }}\n\
    .media-grid.two-up {{ grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); }}\n\
    .media-card {{\n\
      background: var(--code-bg);\n\
      border: 1px solid var(--border);\n\
      border-radius: 12px;\n\
      overflow: hidden;\n\
    }}\n\
    .media-tag {{ padding: 0.5rem 0.85rem; font-size: 0.76rem; color: var(--muted); border-bottom: 1px solid var(--border); background: rgba(15, 23, 42, 0.7); }}\n\
    .media-card img, .media-card video {{ display: block; width: 100%; aspect-ratio: 16 / 9; object-fit: cover; background: #000; }}\n\
    .vlm-caption {{ margin: 0; padding: 0.65rem 0.85rem; font-size: 0.84rem; color: #cbd5e1; border-top: 1px solid var(--border); }}\n\
    .audio-bar {{\n\
      display: flex;\n\
      flex-wrap: wrap;\n\
      align-items: center;\n\
      justify-content: space-between;\n\
      gap: 1rem;\n\
      background: var(--panel-soft);\n\
      border: 1px solid var(--border);\n\
      border-radius: 12px;\n\
      padding: 0.9rem 1.1rem;\n\
      margin-bottom: 1.15rem;\n\
    }}\n\
    .audio-meta {{ flex: 1; min-width: 240px; }}\n\
    .audio-badge {{ font-size: 0.78rem; font-weight: 700; color: var(--cyan); display: block; margin-bottom: 0.25rem; }}\n\
    .audio-meta p {{ margin: 0; font-size: 0.88rem; color: #cbd5e1; }}\n\
    .architect-box {{\n\
      border-left: 3px solid var(--accent);\n\
      background: rgba(234, 88, 12, 0.08);\n\
      border-radius: 0 10px 10px 0;\n\
      padding: 0.85rem 1.1rem;\n\
      margin-bottom: 1.4rem;\n\
    }}\n\
    .architect-title {{ font-size: 0.82rem; font-weight: 700; color: var(--accent-glow); margin-bottom: 0.25rem; }}\n\
    .architect-box p {{ margin: 0; font-size: 0.92rem; }}\n\
    .prose h3 {{ font-size: 1.2rem; margin: 1.6rem 0 0.6rem; color: #f8fafc; }}\n\
    .prose h4 {{ font-size: 1.02rem; margin: 1.2rem 0 0.45rem; color: #cbd5e1; }}\n\
    .prose p, .prose li {{ color: #dbe4f0; font-size: 0.97rem; }}\n\
    .prose ul {{ padding-left: 1.35rem; }}\n\
    .prose code {{\n\
      background: rgba(56, 189, 248, 0.12);\n\
      color: #7dd3fc;\n\
      padding: 0.12rem 0.38rem;\n\
      border-radius: 5px;\n\
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace;\n\
      font-size: 0.88em;\n\
    }}\n\
    .code-header {{\n\
      background: #111827;\n\
      border: 1px solid var(--border);\n\
      border-bottom: none;\n\
      border-radius: 10px 10px 0 0;\n\
      padding: 0.35rem 0.85rem;\n\
      font-size: 0.74rem;\n\
      color: var(--muted);\n\
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;\n\
      margin-top: 1rem;\n\
    }}\n\
    .prose pre {{\n\
      margin: 0 0 1.2rem;\n\
      padding: 1rem;\n\
      background: var(--code-bg);\n\
      border: 1px solid var(--border);\n\
      border-radius: 0 0 10px 10px;\n\
      overflow-x: auto;\n\
      line-height: 1.52;\n\
    }}\n\
    .prose pre code {{ background: none; color: #e2e8f0; padding: 0; font-size: 0.86rem; }}\n\
    .table-wrap {{ overflow-x: auto; margin: 1rem 0 1.3rem; }}\n\
    table {{ width: 100%; border-collapse: collapse; font-size: 0.9rem; }}\n\
    th, td {{ border: 1px solid var(--border); padding: 0.6rem 0.8rem; text-align: left; }}\n\
    th {{ background: #131e35; color: #f8fafc; }}\n\
    blockquote {{\n\
      margin: 1rem 0;\n\
      padding: 0.65rem 1rem;\n\
      border-left: 3px solid var(--cyan);\n\
      background: rgba(56, 189, 248, 0.07);\n\
      color: #cbd5e1;\n\
    }}\n\
  </style>\n\
</head>\n\
<body>\n\
  <header class=\"hero\">\n\
    <div class=\"container\">\n\
      <span class=\"eyebrow\">YANSHI MULTIMODAL ENGINEERING STUDIO</span>\n\
      <h1>Rust Traits 深度讲解：从契约本质到零成本系统架构</h1>\n\
      <p class=\"hero-lead\">\n\
        一套面向系统工程师的多模态深度教程：系统剖析 Rust Trait 的 Typeclass 渊源、语法刻画手段、\n\
        单态化静态分发与胖指针动态虚表，深度对比 OOP 继承树与 C 语言 <code>void*</code> 函数指针，\n\
        拆解孤儿规则与对象安全边界，并沉淀面向复杂工程的抽象哲学。\n\
      </p>\n\
      <div class=\"stats-row\">\n\
        <span class=\"stat-chip\">📚 6 章体系化深度讲义</span>\n\
        <span class=\"stat-chip\">🖼️ 6 幅 16:9 架构概念配图</span>\n\
        <span class=\"stat-chip\">🎬 3 段 Veo 3.1 核心机制视频</span>\n\
        <span class=\"stat-chip\">🎙️ 全程中文 TTS 旁白 + 双格式字幕 ({total_secs:.1}s)</span>\n\
      </div>\n\
    </div>\n\
  </header>\n\
  <div class=\"layout\">\n\
    <aside class=\"sidebar\">\n\
      <div class=\"sidebar-title\">课程章节导航</div>\n\
{nav_items}\
    </aside>\n\
    <main>\n\
{hero_cinema}\
{chapter_cards}\
    </main>\n\
  </div>\n\
</body>\n\
</html>\n"
  )
}

fn curriculum() -> Vec<ChapterSpec> {
  vec![
    ChapterSpec {
      index: 1,
      slug: "intro",
      heading: "介绍：Traits 的本质 —— 共享行为契约与 Typeclass 思想",
      subtitle: "剥离数据布局与能力契约，理解 Rust 零成本多态的根基",
      narration: "欢迎来到 Rust Traits 深度精讲。要真正读懂 Rust，首先要忘掉面向对象里的基类继承。\
        Trait 的思想源于 Haskell 的 Typeclass：它不是数据的父节点，而是一份正交的共享行为契约。\
        结构体负责规定字节在内存里怎么摆放，而 Trait 负责声明这个类型向外界承诺哪些能力。\
        正因为数据与行为彻底解耦，任何独立演化的类型都能在不改动内存布局的前提下，安全地接入标准库与生态系统的通用算法。",
      visual_prompt: "Two orthogonal geometric planes intersecting cleanly in mid-air: on the lower \
        horizontal slate plane sit solid metallic prism blocks representing raw memory structs, and \
        above them a luminous copper-and-cyan hexagonal socket interface locks cleanly onto diverse \
        shapes without altering their internal geometry",
      motion_prompt: None,
      markdown: r#"### 1.1 什么是 Trait？先把“基类”从脑海里拿掉

在 C++ 或 Java 里，当我们想让多个类型共享某种能力时，第一反应往往是定义一个基类 `BaseStream`，再让子类继承它。但继承同时捆绑了两件本该正交的事：**内存字段布局的复用**与**对外接口的多态承诺**。

Rust 的 **Trait** 走了完全不同的一条路——它直接吸收了函数式语言 Haskell 中 **Typeclass（类型类）** 与 ML 模块签名的思想：

- **`struct` / `enum` 只回答“内存里存什么”**：字段在栈或堆上占多少字节、如何对齐，编译期一清二楚，绝不暗藏虚表指针或对象头。
- **`trait` 只回答“类型能履行什么契约”**：它是一组方法签名、关联类型与常量约束的集合，代表一份由编译器严格核验的**能力证书（Capability Certificate）**。

```rust
use std::io::Write;

/// 一份纯粹的序列化帧契约：不关心底层是网络报文还是磁盘日志
pub trait WireFrame {
  /// 帧协议魔数标识
  const MAGIC: u16;

  /// 编码后的精确字节长度
  fn encoded_len(&self) -> usize;

  /// 将帧内容写入任意实现了 Write 契约的目标缓冲区
  fn encode_into<W: Write>(&self, writer: &mut W) -> std::io::Result<usize>;
}

pub struct Heartbeat {
  pub node_id: u32,
  pub epoch_ms: u64,
}

impl WireFrame for Heartbeat {
  const MAGIC: u16 = 0x5953;

  fn encoded_len(&self) -> usize {
    2 + 4 + 8
  }

  fn encode_into<W: Write>(&self, writer: &mut W) -> std::io::Result<usize> {
    writer.write_all(&Self::MAGIC.to_be_bytes())?;
    writer.write_all(&self.node_id.to_be_bytes())?;
    writer.write_all(&self.epoch_ms.to_be_bytes())?;
    Ok(self.encoded_len())
  }
}
```

### 1.2 Rust 语言本身就建立在标准库 Trait 之上

Rust 并没有把语言特性硬编码进特殊语法黑盒，而是通过标准库 Trait 开放给所有用户类型：

| 语言机制 / 语法糖 | 背后驱动的 Trait | 核心职责 |
| --- | --- | --- |
| `for item in collection` | `IntoIterator` / `Iterator` | 惰性序列产出与零开销组合子链 |
| `a + b`、`buf[idx]` | `std::ops::{Add, Index}` | 运算符重载与受控索引边界 |
| 离开作用域自动释放资源 | `Drop` | RAII 确定性析构（文件句柄、锁、Socket） |
| 跨线程转移 / 并发共享引用 | `Send` / `Sync` | 编译期并发数据竞争免疫证明 |
| `?` 错误向上传播转换 | `From` / `Into` | 跨模块错误类型的无损桥接 |
"#,
    },
    ChapterSpec {
      index: 2,
      slug: "syntax",
      heading: "描述方式：方法接收者、默认实现、关联类型与 Supertraits",
      subtitle: "精确刻画行为边界的四把手术刀，学会在关联类型与泛型参数之间做正确抉择",
      narration: "第二章我们看如何精确描述一个 Trait。除了普通的方法签名，Rust 提供了四项关键表达工具。\
        第一，通过借用、可变借用或按值获取所有权的接收者，直接在签名上钉死所有权流转；\
        第二，用少量核心方法驱动大量默认实现，就像标准库迭代器只要求你写一个 next，就白送几十个组合子；\
        第三，区分关联类型与泛型参数：输入多态用泛型，输出唯一用关联类型；\
        第四，用 Supertrait 声明前置契约。",
      visual_prompt: "An intricate precision horology workbench in isometric dark slate: a central \
        glowing copper master gear drives a cascade of smaller cyan satellite gears automatically, \
        with four distinct interlocking socket silhouettes carved cleanly into brushed titanium",
      motion_prompt: None,
      markdown: r#"### 2.1 四种接收者形态：把所有权语义写进签名

Trait 方法的第一个参数决定了调用方与实例之间的所有权关系：

- `&self`：共享只读借用，允许多个并发读取者同时观测状态；
- `&mut self`：独占可变借用，编译器保证同一时刻绝无别名引用；
- `self`（按值消费）：常用于 Builder 终结、连接关闭或**类型状态机（Typestate）跃迁**，调用后旧状态在编译期消失，杜绝二次使用；
- 无 `self`（关联函数）：充当工厂构造器（如 `Default::default()`）或静态元信息查询。

### 2.2 最小核 + 默认方法（Default Methods）

优秀的 Trait 往往遵循**最小实现核原则**：只要求实现者提供一个最原子的钩子，其余高阶方法全部在 Trait 内部给出默认实现（必要时允许特定类型覆盖以做特化加速）。

```rust
pub trait ByteSource {
  /// 唯一必填核：拉取下一块字节切片，到达末尾返回 None
  fn next_chunk(&mut self) -> Result<Option<&[u8]>, std::io::Error>;

  /// 默认组合方法：基于 next_chunk 统计总字节数，实现者无需手写但可按需覆写（如文件可直接查 metadata）
  fn count_bytes(&mut self) -> Result<u64, std::io::Error> {
    let mut total = 0_u64;
    while let Some(chunk) = self.next_chunk()? {
      total += chunk.len() as u64;
    }
    Ok(total)
  }
}
```

### 2.3 关键抉择：关联类型（Associated Types）vs 泛型参数（Generic Traits）

初学者最常困惑的问题是：什么时候写 `trait Parser<Out>`，什么时候写 `trait Parser { type Out; }`？

| 维度 | 泛型参数 `trait Convert<Target>` | 关联类型 `trait Service { type Response; }` |
| --- | --- | --- |
| 映射关系 | **一对多**：同一个类型可以实现多次（`From<&str>` 与 `From<String>`） | **一对一（函数依赖）**：给定实现类型 `S`，`S::Response` 唯一确定 |
| 调用方负担 | 调用处经常需要显式标注类型参数 `T: Convert<u32> + Convert<u64>` | 约束极其干净：`fn call<S: Service>(s: S) -> S::Response` |
| 典型代表 | `From<T>`、`AsRef<T>`、`Add<Rhs>` | `Iterator::Item`、`Future::Output`、`Deref::Target` |

```rust
use std::fmt::Debug;

/// Supertrait 约束：任何实现 StorageEngine 的后端，必须天然满足跨线程安全共享 (Send + Sync)
pub trait StorageEngine: Send + Sync {
  /// 关联类型：每个具体存储引擎绑定唯一一种错误类型与事务句柄类型
  type Error: std::error::Error + Send + Sync + 'static;
  type Tx<'a>: Debug
  where
    Self: 'a;

  fn begin(&self) -> Result<Self::Tx<'_>, Self::Error>;
}
```
"#,
    },
    ChapterSpec {
      index: 3,
      slug: "dispatch",
      heading: "应用方法：Trait Bounds、静态单态化 vs `&dyn Trait` 胖指针与 Blanket Impl",
      subtitle: "看清编译期代码展开与 16 字节胖指针虚表跳转的每一处机器指令细节",
      narration: "第三章进入 Trait 的核心引擎室：静态分发与动态分发。\
        当你写泛型约束或 impl Trait 时，Rust 编译器在编译期执行单态化，为每个具体类型生成专属机器码，函数可以直接内联，实现真正的零成本抽象。\
        而当你需要在一个集合里混装多种运行时才确定的类型时，借用 dyn Trait 会构造一个十六字节的胖指针：前八字节指向真实数据，后八字节指向只读虚表。\
        数据本身依然没有对象头负担，鱼与熊掌各归其位。",
      visual_prompt: "Split architectural diagram in glowing dark slate: on the left side, a single \
        crystalline prism splits a copper laser beam into three parallel dedicated high-speed tracks \
        representing compile-time monomorphization; on the right side, a sleek dual-channel 16-byte \
        connector cable routes one wire to a data block and the second wire to an illuminated cyan \
        lookup table matrix",
      motion_prompt: Some(
        "Smooth cinematic camera push-in: on the left, glowing copper light pulses split cleanly \
         through a crystal prism into three parallel zero-latency tracks; simultaneously on the right, \
         cyan data packets flow along a dual-wire connector into a luminous lookup table grid and \
         dispatch cleanly to target modules, crisp tech motion graphics",
      ),
      markdown: r#"### 3.1 静态分发（Static Dispatch）：编译期单态化（Monomorphization）

当使用泛型约束 `<T: Checksum>` 或参数位置 `impl Checksum` 时，编译器会扫描所有实际调用点，为每一种具体类型单独克隆并特化出一份机器码（例如 `verify_Crc32` 与 `verify_Blake3`）：

- **零间接跳转**：调用地址在编译期已知，LLVM 可直接将短小的 Trait 方法**内联（Inline）**进热点循环，并触发自动向量化（SIMD）。
- **栈上紧凑存放**：变量大小编译期精确已知，无需 `Box` 堆分配。

### 3.2 动态分发（Dynamic Dispatch）：`&dyn Trait` 16 字节胖指针剖析

当我们需要在同一个 `Vec<Box<dyn Tool>>` 中装载运行期配置决定的异构工具集时，静态单态化无法预先枚举所有类型，此时登场的是 **Trait Object（`&dyn Trait` / `Box<dyn Trait>`）**。

在 64 位平台上，`&T` 是 **8 字节**窄指针；而 `&dyn Checksum` 是 **16 字节胖指针（Fat Pointer）**：

```text
  &dyn Checksum (栈上 16 Bytes)
  ┌───────────────────┬───────────────────┐
  │ data_ptr (8B)     │ vtable_ptr (8B)   │
  └─────────┬─────────┴─────────┬─────────┘
            │                   │
            ▼                   ▼  (.rodata 只读数据段，每个 (Type, Trait) 对全局唯一一份)
     ┌──────────────┐    ┌──────────────────────────────┐
     │ Crc32Hasher  │    │ drop_in_place: fn(*mut ())   │
     │ state: u32   │    │ size: 4, align: 4            │
     │ (仅 4 Bytes! │    │ digest: fn(*const ()) -> u64 │
     │  无隐藏 vptr)│    └──────────────────────────────┘
     └──────────────┘
```

```rust
pub trait Checksum: Send + Sync {
  fn digest(&self, payload: &[u8]) -> u64;
}

pub struct Xor64 {
  pub seed: u64,
}

impl Checksum for Xor64 {
  fn digest(&self, payload: &[u8]) -> u64 {
    payload
      .iter()
      .fold(self.seed, |acc, &b| acc.rotate_left(7) ^ u64::from(b))
  }
}

/// 静态分发：编译期单态化，digest 可被 LLVM 直接内联进热点循环
pub fn verify_fast<C: Checksum>(hasher: &C, payload: &[u8], expected: u64) -> bool {
  hasher.digest(payload) == expected
}

/// 动态分发：只生成一份机器码，支持异构插件注册表，通过胖指针 vtable 间接调用
pub fn run_pipeline(stages: &[Box<dyn Checksum>], payload: &[u8]) -> Vec<u64> {
  stages.iter().map(|stage| stage.digest(payload)).collect()
}
```

### 3.3 Blanket Implementation（一揽子泛型实现）

Rust 允许“为所有满足契约 `A` 的类型 `T` 统一实现契约 `B`”。标准库中最著名的例子莫过于：

```rust
// 标准库内置：只要你的类型实现了 Display，就自动免费获得 ToString::to_string()！
// impl<T: fmt::Display + ?Sized> ToString for T { ... }

pub trait JsonSummary {
  fn to_summary_json(&self) -> String;
}

/// 一揽子实现：任何实现了 Display 的类型自动具备 JsonSummary 能力
impl<T: std::fmt::Display + ?Sized> JsonSummary for T {
  fn to_summary_json(&self) -> String {
    format!("{{\"summary\":\"{}\"}}", self)
  }
}
```
"#,
    },
    ChapterSpec {
      index: 4,
      slug: "comparison",
      heading: "核心优点：深度对比 OOP 类继承体系与 C 语言 `void*` 函数指针",
      subtitle: "告别菱形继承与对象头税，同时用编译期借用检查消灭 C 手写虚表的未定义行为",
      narration: "第四章我们把视野拉开，横向对比面向对象类继承与 C 语言函数指针。\
        传统面向对象把数据继承与接口多态绑死在脆弱的基类树上，不仅容易陷入菱形继承泥潭，还迫使每个对象实例都背负隐藏的对象头或虚表指针。\
        而在 C 语言里，像 Linux 内核的文件操作表虽然用函数指针结构体加 void 星号实现了零开销解耦，但 void 星号彻底擦除了类型与生命周期，一次错误的强制转换就是内存破坏。\
        Rust Trait 恰好融合了两者的精华：既有组合优于继承的正交弹性，又有超越手写虚表的编译期绝对安全。",
      visual_prompt: "Three architectural columns side by side on a dark obsidian platform: on the \
        left, a tangled fragile glass tree cracking under weight (OOP inheritance); in the middle, \
        exposed raw copper wires sparking dangerously across an unguarded gap (C void pointer); on \
        the right, modular interlocking titanium blocks snapped into clean glowing cyan magnetic \
        rails (Rust Traits)",
      motion_prompt: Some(
        "Subtle dramatic architectural camera pan from left to right: the tangled glass hierarchy on \
         the left flickers with stress fractures, the raw unguarded wires in the center emit brief \
         amber sparks, while on the right the modular titanium blocks glide and lock seamlessly into \
         clean glowing cyan magnetic rails with steady energy flow",
      ),
      markdown: r#"### 4.1 全景对比：OOP 类继承 vs C 语言 `void*` 虚表 vs Rust Trait

| 评估维度 | OOP 类继承（Java / C++ `virtual`） | C 语言（`void *ctx` + 函数指针表） | Rust Trait 体系 |
| --- | --- | --- | --- |
| **数据与行为耦合度** | 强耦合在继承树上，易引发脆弱基类与菱形继承 | 完全解耦（结构体存状态，`ops` 存函数指针） | **正交解耦**（`struct` 管布局，`trait` 管契约） |
| **事后扩展第三方类型** | 不修改原类源码无法让已有类实现新 `interface` | 可手写新包装表，但需人工维护胶水转换 | **原生支持**：可为上游类型实现本地新 Trait |
| **实例内存开销** | C++ 含虚函数类每实例 `+8B vptr`；Java 每对象 `+12~16B` 对象头 | 按需传入，原结构体零额外头部 | **静态路径 0 字节开销**；仅借用为 `&dyn` 时指针本身变为 16B |
| **类型与生命周期安全** | 运行期安全（GC 或虚表受编译器保护），但难防空指针与数据竞争 | **极度危险**：`void*` 强转错类型或悬垂指针直接导致内存破坏（UB） | **编译期静态证明**：类型匹配、生命周期 `'a` 与 `Send/Sync` 全部零成本校验 |

### 4.2 代码级对照：同一个“可拔插压缩器”在三种范式下的真实面貌

#### （1）C 语言写法：零开销但 `void*` 完全脱离编译器保护
```c
typedef struct {
  size_t (*compress)(const void *ctx, const uint8_t *in, size_t len, uint8_t *out);
} compressor_ops_t;

typedef struct {
  const void *ctx;               // 危险：如果把 lz4_ctx 误传给 zstd_ops，编译完全通过，运行期直接踩内存！
  const compressor_ops_t *ops;   // 危险：若 ctx 指向已释放的栈变量，编译器毫无感知
} compressor_handle_t;
```

#### （2）传统 OOP 写法：被迫继承基类、背负虚表与引用间接层
```java
// 若 ThirdPartyLz4 来自外部 jar 包，无法直接让它 extends AbstractCompressor，只能再包一层 Adapter 对象（额外堆分配）
public abstract class AbstractCompressor {
  protected final int level;
  protected AbstractCompressor(int level) { this.level = level; }
  public abstract int compress(byte[] in, byte[] out);
}
```

#### （3）Rust Trait 写法：兼具 C 的裸金属紧凑布局与编译期铁壁防御
```rust
pub trait Compressor {
  fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressError>;
}

#[derive(Debug)]
pub struct CompressError;

/// 纯数据结构体：sizeof::<RleCompressor>() == 1 字节，零 vptr 税！
pub struct RleCompressor {
  pub min_run: u8,
}

impl Compressor for RleCompressor {
  fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressError> {
    let n = input.len().min(output.len());
    output[..n].copy_from_slice(&input[..n]);
    Ok(n)
  }
}

/// 甚至可以直接为外部标准库类型或切片赋予 Compressor 契约（事后扩展能力）！
impl Compressor for () {
  fn compress(&self, input: &[u8], output: &mut [u8]) -> Result<usize, CompressError> {
    if output.len() < input.len() {
      return Err(CompressError);
    }
    output[..input.len()].copy_from_slice(input);
    Ok(input.len())
  }
}
```
"#,
    },
    ChapterSpec {
      index: 5,
      slug: "pitfalls",
      heading: "实战注意事项：孤儿规则、`dyn` 对象安全、单态化膨胀与过早抽象反模式",
      subtitle: "避开四大真实生产暗礁，写出克制、高内聚的地道 Rust",
      narration: "第五章聚焦生产实战中最容易踩坑的四个问题。\
        第一是孤儿规则：为了防止链接期实现冲突，Trait 和目标类型至少要有一个定义在当前包内，需要跨包桥接时请使用零开销的 Newtype 包装。\
        第二是 dyn 兼容性：含有泛型方法或按值返回 Self 的 Trait 无法直接生成单张静态虚表。\
        第三是单态化膨胀：对外暴露泛型便利接口时，应当把核心重逻辑下沉到非泛型的内部函数里减少二进制体积。\
        第四条最重要：在第二个真实实现出现之前，绝不要为了显得高级而提前抽空壳 Trait。",
      visual_prompt: "Four precision engineering boundary gauges and safety interlocks on a dark \
        slate control console: a transparent hexagonal wrapper shell around an external core, a \
        clean vtable gate filter, a funnel condensing wide parallel streams into a single dense \
        core channel, and a minimalist architectural balance scale",
      motion_prompt: None,
      markdown: r#"### 5.1 孤儿规则（Orphan Rule / Coherence）与 Newtype 破局

Rust 编译器强制执行**一致性原则（Coherence）**：当你编写 `impl SomeTrait for SomeType` 时，**`SomeTrait` 或 `SomeType` 至少有一个必须定义在当前 Crate 中**。
否则，假设 Crate A 和 Crate B 都为标准库的 `Vec<u8>` 实现了 `serde::Serialize`，当下游同时引入 A 和 B 时，编译器将无法决定调用哪一份实现（即“哈希表与序列化分裂灾难”）。

**地道解法 —— 零开销 `#[repr(transparent)]` Newtype 模式**：
```rust
use std::fmt;

/// 包装外部类型 Vec<u8>，由于 HexBytes 定义在当前 Crate，孤儿规则合法放行！
/// #[repr(transparent)] 保证内存布局与底层 Vec<u8> 100% 相同，零运行期包装成本。
#[repr(transparent)]
pub struct HexBytes(pub Vec<u8>);

impl fmt::Display for HexBytes {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in &self.0 {
      write!(f, "{byte:02x}")?;
    }
    Ok(())
  }
}
```

### 5.2 `dyn` 兼容性（原称 Object Safety 对象安全）

并非所有 Trait 都能写成 `Box<dyn MyTrait>`。因为 `dyn Trait` 依赖一张**编译期固定槽位数量**的静态虚表，以下形态会破坏虚表构造：

1. **方法带有泛型类型参数** `fn encode<W: Write>(&self, w: &mut W)` —— `W` 有无穷多种可能，单张虚表放不下无限个函数指针槽位；
2. **方法返回裸 `Self` 或按值接收未限定 `Self`**（如 `Clone::clone(&self) -> Self`）—— 调用方只握有擦除了类型的指针，编译期不知道该在栈上为返回值预留多少字节；
3. **Trait 含有无接收者的关联函数或关联常量**。

**破局技巧**：给不需要进虚表的辅助方法加上 `where Self: Sized` 守卫，或者在虚表边界内也将参数改为 `&mut dyn Write`：
```rust
pub trait Plugin: Send + Sync {
  fn name(&self) -> &str;

  /// 加上 `where Self: Sized` 后，该方法从 dyn Plugin 虚表中豁免，Box<dyn Plugin> 恢复合法！
  fn into_arc(self) -> std::sync::Arc<Self>
  where
    Self: Sized,
  {
    std::sync::Arc::new(self)
  }
}
```

### 5.3 单态化代码膨胀治理（Monomorphization Diet / 内层非泛型化）

如果一个 200 行的复杂函数签名写着 `pub fn load_config<P: AsRef<Path>>(path: P)`，当工程里分别用 `&str`、`String`、`PathBuf`、`&Path` 调用它时，这 200 行逻辑会被重复编译和塞进二进制 **4 遍**，既拖慢编译速度又污染 CPU 指令缓存（I-Cache）！

**标准库惯用手法（一行薄壳转发 + 单份非泛型内核）**：
```rust
use std::path::Path;

/// 对外保持极佳的人体工程学（支持 &str / PathBuf / &Path）—— 但函数体只有 1 行，内联即消失
pub fn load_config<P: AsRef<Path>>(path: P) -> std::io::Result<String> {
  load_config_inner(path.as_ref())
}

/// 真正重达数百行的解析逻辑只针对 `&Path` 编译唯一一份机器码！
fn load_config_inner(path: &Path) -> std::io::Result<String> {
  std::fs::read_to_string(path)
}
```

### 5.4 工程品味红线：拒绝“只有一个实现的 Trait”

从 Java / Go 转入 Rust 的团队最常见的过度设计，就是给每一个业务 Service 都先抽一个 `UserServiceTrait` 再写唯一的 `UserServiceImpl`：

- **代价现在立刻支付**：代码跳转翻倍、生命周期与 `Send + Sync + 'static` 约束传染、报错信息变复杂；
- **收益等第二个实现才兑现**：如果直到项目下线都只有 Postgres 一个实现，这层 Trait 就是纯粹的噪音。
- **黄金判据**：**业务内部模块直接写具体 `struct`；只有站在真实的变化边界上（例如跨 L0 窄腰契约层、多厂商 LLM Provider 适配、或必须隔离外部昂贵 I/O 的边界）且存在第二个真实实现时，才提取 Trait。**
"#,
    },
    ChapterSpec {
      index: 6,
      slug: "philosophy",
      heading: "总结深思：从“它是什么”到“它能做什么” —— 把运行期不确定性前移为编译期证明",
      subtitle: "Marker Traits 定理证明与窄腰契约内核的系统架构之道",
      narration: "最后一章，让我们跳出语法细节，思考 Rust Trait 背后的工程哲学。\
        传统分类学总在追问一个事物是什么，一旦业务演化跨越树状层级，重构就举步维艰；而 Trait 只追问它在当前上下文里能履行什么契约。\
        更令人着迷的是像 Send 和 Sync 这样的标记 Trait：它们连一行运行期指令都没有，却把并发数据竞争从凌晨三点的线上崩溃，前移成了编译期的一条清晰报错。\
        用最薄、最正交的契约切开复杂系统的变化边界，这就是 Rust 献给现代系统工程的最高智慧。",
      visual_prompt: "A monumental dark-slate architectural suspension bridge whose central narrow-waist \
        keystone ring glows with warm rust-copper light: diverse chaotic streams of raw energy enter \
        from below, pass through the crisp geometric contract ring, and emerge above as harmonious \
        parallel beams of pure cyan light",
      motion_prompt: Some(
        "Majestic slow upward tilt and pull-back: chaotic swirling streams of particles pass through \
         a luminous rust-copper narrow-waist keystone ring at the center of an architectural monument \
         and emerge above as calm, perfectly aligned beams of cyan light soaring into a clear horizon",
      ),
      markdown: r#"### 6.1 本体论的转向：从“分类树（Is-A）”到“正交契约投影（Can-Do）”

人类早期软件工程受生物分类学影响很深，总想提前画出一棵完美的继承树（动物 → 鸟类 → 会飞的鸟 → 企鹅？失灵了）。现实世界的需求变化恰恰最擅长打碎预先设定的分类层级。

Rust Trait 完成了从**本体论（它是什么）**向**契约论（它在当前场景下承诺什么行为）**的跃迁：
- 在日志模块眼里，你的结构体只是一个 `&dyn Display`；
- 在异步执行器眼里，它是一个 `Future<Output = Result<Response, Error>> + Send`；
- 在连接池回收器眼里，它只是一个实现了 `Drop` 的资源守卫。
同一个具体数据结构，在不同子系统中投射出正交的能力切面，彼此零干扰、零侵入。

### 6.2 Marker Traits：零字节、零指令的“编译期定理证明器”

如果审视 `std::marker::{Send, Sync, Copy, Unpin}`，你会惊讶地发现它们的定义体是**空的**——没有任何方法签名，编译后不生成哪怕一条汇编指令：

```rust
use std::marker::PhantomData;

/// 编译期状态机：用零尺寸类型 + Trait 契约，让“未鉴权就执行危险命令”在语法上无法编译！
pub struct Unverified;
pub struct Verified;

pub trait CanExecute {}
impl CanExecute for Verified {}

pub struct CommandRequest<State> {
  pub sql: String,
  _state: PhantomData<State>,
}

impl CommandRequest<Unverified> {
  pub fn new(sql: impl Into<String>) -> Self {
    Self {
      sql: sql.into(),
      _state: PhantomData,
    }
  }

  pub fn verify(self, token: &str) -> Result<CommandRequest<Verified>, &'static str> {
    if token.starts_with("valid_") {
      Ok(CommandRequest {
        sql: self.sql,
        _state: PhantomData,
      })
    } else {
      Err("invalid token")
    }
  }
}

/// 只有满足 CanExecute 契约的状态才能调用 run —— 未调用 verify() 的请求在编译期即被彻底拦截！
impl<S: CanExecute> CommandRequest<S> {
  pub fn execute(&self) -> &str {
    &self.sql
  }
}
```

这就是 Rust 最深邃的工程美学：**凡是能在编译期通过类型契约消解的非法状态，绝不留到运行期靠 `if` 检查、单元测试和值班报警去兜底。**

### 6.3 架构启示：窄腰契约内核（Narrow-Waist Contract Kernel）

回看 **Yanshi** 自身的五层架构设计，正是 Trait 工程哲学的真实缩影：
- **L0 窄腰契约层**只放零运行期逻辑的纯粹 Trait（`yanshi-graph::AsyncNode`、`yanshi-tool::Tool`、`yanshi-llm::Text2ImageProvider` / `Text2VideoProvider` / `TtsProvider`）；
- 上层 DAG 编排器、ReAct 智能体与 CLI 完全面向这些窄契约编程，互不依赖具体厂商与传输细节；
- 新增 Google Vertex AI Veo 视频或 Gemini TTS 能力时，只需在 L2 实现对应 Trait 并注册分发，全栈 YAML 节点、CLI 与 SDK 示例即刻无缝贯通。

> **结语**：写好 Rust Trait 的最高境界，不是炫耀复杂的泛型体操，而是**用最薄的一纸契约，切开系统里最真实的变化边界**。
"#,
    },
  ]
}
