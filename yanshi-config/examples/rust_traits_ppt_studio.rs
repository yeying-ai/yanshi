//! Keynote/PPT-style 1080p technical lecture generator for "Rust Traits 深度讲解".
//!
//! Renders 9 structured engineering slides across 22 progressive build frames
//! (syntax-highlighted code spotlighting, compiler flowcharts, 16-byte fat-pointer
//! memory layouts, OOP/C comparison matrices, decision trees, and a panorama mind
//! map), synchronizes them sentence-by-sentence with Chinese TTS voiceovers into a
//! 1920×1080 MP4 + SRT/VTT subtitles, and exports an editable `.pptx`, a 16:9
//! `.pdf` handout, and a synchronized Web Slide + Video Keynote player.
//!
//! ## Default offline mode (uses cached WAVs if present, else mock TTS)
//! ```bash
//! cargo run -p yanshi-config --example rust_traits_ppt_studio
//! ```
//!
//! ## Build production bundle into `runs/rust-traits-ppt` (reusing cached live WAVs or live Vertex TTS)
//! ```bash
//! YANSHI_OUTPUT_DIR=runs/rust-traits-ppt \
//! YANSHI_AUDIO_CACHE_DIR=runs/rust-traits-course/assets \
//!   cargo run -p yanshi-config --example rust_traits_ppt_studio
//! ```

use std::env;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::json;
use yanshi_llm::{TtsRequest, Yanshi};
use yanshi_nodes::nodes::render::svg_to_png;
use yanshi_nodes::nodes::render_slides::{
  BulletItem, CodeCallout, FlowEdge, FlowNode, MemPointer, MemRegion, MemSlot, Panel, Slide,
  SlideDeck, SlideFrameMeta, SlideStep, deck_to_interactive_html, deck_to_pandoc_markdown,
  deck_to_typst_source, effective_steps, render_slide_svg,
};

struct ChapterAudioPlan {
  chapter_index: usize,
  audio_file: &'static str,
  narration: &'static str,
  slide_indices: &'static [usize],
}

#[derive(Clone)]
struct SubtitleCue {
  start_secs: f64,
  end_secs: f64,
  text: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
  let live = env::var("YANSHI_LIVE_PROVIDER")
    .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    .unwrap_or(false);

  let output_dir = env::var_os("YANSHI_OUTPUT_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| {
      if live || Path::new("runs/rust-traits-course/assets/ch01.wav").is_file() {
        PathBuf::from("runs/rust-traits-ppt")
      } else {
        env::temp_dir().join("yanshi-rust-traits-ppt-mock")
      }
    });
  let slides_dir = output_dir.join("slides");
  let audio_dir = output_dir.join("audio");
  tokio::fs::create_dir_all(&slides_dir).await?;
  tokio::fs::create_dir_all(&audio_dir).await?;

  let audio_cache_dir = env::var_os("YANSHI_AUDIO_CACHE_DIR")
    .map(PathBuf::from)
    .unwrap_or_else(|| PathBuf::from("runs/rust-traits-course/assets"));

  println!("=== Rust Traits 1080p PPT Lecture Studio ===");
  println!("Output directory: {}", output_dir.display());

  let deck = build_rust_traits_deck();
  let chapters = chapter_audio_plans();

  // 1. Render all progressive 1920x1080 PNG + SVG frames and completed slide PNGs.
  let mut per_slide_frames: Vec<Vec<(PathBuf, String, String)>> =
    Vec::with_capacity(deck.slides.len());
  let mut completed_slide_pngs: Vec<PathBuf> = Vec::with_capacity(deck.slides.len());
  let mut completed_slide_rels: Vec<String> = Vec::with_capacity(deck.slides.len());

  for (si, slide) in deck.slides.iter().enumerate() {
    let steps = effective_steps(slide);
    let mut step_vec = Vec::with_capacity(steps.len());
    println!(
      "  [Slide {:02}/{}] {} ({} build steps)",
      si + 1,
      deck.slides.len(),
      slide.title,
      steps.len()
    );

    for (sti, step) in steps.iter().enumerate() {
      let stem = format!("s{:02}_step{:02}", si + 1, sti + 1);
      let svg_path = slides_dir.join(format!("{stem}.svg"));
      let png_path = slides_dir.join(format!("{stem}.png"));
      let rel_png = format!("slides/{stem}.png");

      let svg = render_slide_svg(&deck, si, Some(sti)).map_err(|e| anyhow::anyhow!(e))?;
      let png = svg_to_png(&svg, Some(1920)).map_err(|e| anyhow::anyhow!(e))?;
      tokio::fs::write(&svg_path, svg.as_bytes()).await?;
      tokio::fs::write(&png_path, &png).await?;

      if sti + 1 == steps.len() {
        completed_slide_pngs.push(png_path.clone());
        completed_slide_rels.push(rel_png.clone());
      }
      step_vec.push((png_path, rel_png, step.caption.clone()));
    }
    per_slide_frames.push(step_vec);
  }

  // 2. Prepare chapter WAV narrations (reuse cached Vertex WAVs when present, else TTS).
  let mut need_tts = false;
  for ch in &chapters {
    let target = audio_dir.join(ch.audio_file);
    let cached = audio_cache_dir.join(ch.audio_file);
    if !file_non_empty(&target) && file_non_empty(&cached) {
      tokio::fs::copy(&cached, &target).await?;
      println!(
        "  [Audio Ch{:02}] reused {}",
        ch.chapter_index,
        cached.display()
      );
    }
    if !file_non_empty(&target) {
      need_tts = true;
    }
  }

  if need_tts {
    let (tts_model, voice) = init_tts(live, &output_dir).await?;
    let tts_provider = Yanshi::tts(&tts_model).await?;
    for ch in &chapters {
      let target = audio_dir.join(ch.audio_file);
      if !file_non_empty(&target) {
        println!(
          "  [Audio Ch{:02}] synthesizing via {tts_model}...",
          ch.chapter_index
        );
        let audio = tts_provider
          .synthesize(TtsRequest {
            model: tts_model.clone(),
            input: ch.narration.to_string(),
            voice: voice.clone(),
            response_format: Some("wav".to_string()),
            speed: None,
            volume: None,
            sample_rate: None,
          })
          .await?;
        tokio::fs::write(&target, &audio.audio).await?;
      }
    }
  }

  // 3. Compute exact per-step timing from each chapter's WAV duration.
  let mut chapter_segments: Vec<ChapterSegment> = Vec::new();
  let mut cues: Vec<SubtitleCue> = Vec::new();
  let mut html_meta: Vec<SlideFrameMeta> = Vec::new();
  let mut global_cursor = 0.0_f64;

  for ch in &chapters {
    let wav_path = audio_dir.join(ch.audio_file);
    let wav_bytes = tokio::fs::read(&wav_path).await?;
    let duration = wav_duration_seconds(&wav_bytes).unwrap_or(18.0);

    let mut ch_frames: Vec<(usize, usize, PathBuf, String, String)> = Vec::new();
    for &si in ch.slide_indices {
      for (sti, (p, rel, cap)) in per_slide_frames[si].iter().enumerate() {
        ch_frames.push((si, sti, p.clone(), rel.clone(), cap.clone()));
      }
    }
    let total_weight: usize = ch_frames
      .iter()
      .map(|(_, _, _, _, cap)| cap.chars().count().max(10))
      .sum();

    let mut timed_frames = Vec::with_capacity(ch_frames.len());
    let mut ch_elapsed = 0.0_f64;
    for (idx, (si, sti, path, rel, cap)) in ch_frames.iter().enumerate() {
      let share = if idx + 1 == ch_frames.len() {
        (duration - ch_elapsed).max(0.4)
      } else {
        duration * (cap.chars().count().max(10) as f64) / (total_weight.max(1) as f64)
      };
      let frame_start = global_cursor + ch_elapsed;
      html_meta.push(SlideFrameMeta {
        slide_idx: *si,
        step_idx: *sti,
        title: deck.slides[*si].title.clone(),
        caption: cap.clone(),
        image_src: rel.clone(),
        start_secs: frame_start,
      });

      // Split each frame's caption into natural sentence cues inside [frame_start, frame_start + share].
      let sentences = split_sentences_zh(cap);
      let s_total: usize = sentences.iter().map(|s| s.chars().count().max(1)).sum();
      let mut s_cursor = frame_start;
      for (s_idx, s) in sentences.iter().enumerate() {
        let s_dur = if s_idx + 1 == sentences.len() {
          (frame_start + share - s_cursor).max(0.2)
        } else {
          share * (s.chars().count().max(1) as f64) / (s_total.max(1) as f64)
        };
        cues.push(SubtitleCue {
          start_secs: s_cursor,
          end_secs: s_cursor + s_dur,
          text: s.clone(),
        });
        s_cursor += s_dur;
      }

      timed_frames.push((path.clone(), share));
      ch_elapsed += share;
    }

    global_cursor += duration;
    chapter_segments.push((wav_path, duration, timed_frames));
  }

  // 4. Write SRT + WebVTT and stitch 1920x1080 MP4 via FFmpeg.
  let srt_text = format_srt(&cues);
  let vtt_text = format_vtt(&cues);
  let srt_path = output_dir.join("full_course.srt");
  let vtt_path = output_dir.join("full_course.vtt");
  tokio::fs::write(&srt_path, &srt_text).await?;
  tokio::fs::write(&vtt_path, &vtt_text).await?;

  let mp4_path = output_dir.join("full_course.mp4");
  let mp4_built = stitch_1080p_mp4(&output_dir, &chapter_segments, &srt_path, &mp4_path)?;

  // 5. Export editable PowerPoint (.pptx) via pandoc and 16:9 Handout (.pdf) via typst.
  let png_refs: Vec<&Path> = completed_slide_pngs.iter().map(PathBuf::as_path).collect();
  let pandoc_md = deck_to_pandoc_markdown(&deck, &png_refs);
  let deck_md_path = output_dir.join("slides_deck.md");
  tokio::fs::write(&deck_md_path, &pandoc_md).await?;

  let pptx_path = output_dir.join("rust_traits_slides.pptx");
  let pptx_built = export_pptx(&deck_md_path, &pptx_path, &output_dir);

  let typst_src = deck_to_typst_source(&deck, &completed_slide_rels);
  let typst_path = output_dir.join("handout.typ");
  tokio::fs::write(&typst_path, &typst_src).await?;
  let pdf_path = output_dir.join("rust_traits_handout.pdf");
  let pdf_built = export_typst_pdf(&typst_path, &pdf_path, &output_dir);

  // Copy companion Markdown study guide if available from previous run, or emit one.
  let study_md_path = output_dir.join("tutorial.md");
  let cached_md = Path::new("runs/rust-traits-course/tutorial.md");
  if !file_non_empty(&study_md_path) {
    if file_non_empty(cached_md) {
      let raw = tokio::fs::read_to_string(cached_md).await?;
      tokio::fs::write(&study_md_path, raw).await?;
    } else {
      tokio::fs::write(&study_md_path, &pandoc_md).await?;
    }
  }

  // 6. Write synchronized dual-mode Web Slide + Video Keynote player (`index.html`).
  let vtt_uri = format!(
    "data:text/vtt;base64,{}",
    encode_base64(vtt_text.as_bytes())
  );
  let mut links: Vec<(&str, &str)> = Vec::new();
  if pptx_built {
    links.push(("📥 下载 PowerPoint (.pptx)", "rust_traits_slides.pptx"));
  }
  if pdf_built {
    links.push(("📄 打开 16:9 高清讲义 (.pdf)", "rust_traits_handout.pdf"));
  }
  links.push(("📝 深度文字教程 (.md)", "tutorial.md"));
  links.push(("💬 字幕文件 (.srt)", "full_course.srt"));

  let html = deck_to_interactive_html(
    &deck,
    &html_meta,
    mp4_built.then_some("full_course.mp4"),
    Some(&vtt_uri),
    &links,
  );
  let html_path = output_dir.join("index.html");
  tokio::fs::write(&html_path, &html).await?;

  println!("\n=== Deliverables Ready ===");
  println!(
    "  • Slide frames   : {} × 1920×1080 PNGs across {} slides ({})",
    html_meta.len(),
    deck.slides.len(),
    slides_dir.display()
  );
  if mp4_built {
    println!(
      "  • 1080p Video    : {} ({:.1}s, H.264 + AAC + Chinese SRT)",
      mp4_path.display(),
      global_cursor
    );
  }
  if pptx_built {
    println!("  • Editable PPTX  : {}", pptx_path.display());
  }
  if pdf_built {
    println!("  • 16:9 PDF Deck  : {}", pdf_path.display());
  }
  println!("  • Web Player     : {}", html_path.display());

  Ok(())
}

async fn init_tts(live: bool, output_dir: &Path) -> anyhow::Result<(String, String)> {
  if live {
    if let Some(home) = dirs::home_dir() {
      let env_path = home.join(".yanshi").join(".env");
      if env_path.is_file()
        && let Ok(content) = std::fs::read_to_string(&env_path)
      {
        for line in content.lines() {
          if let Some((k, v)) = line.trim().split_once('=')
            && !k.starts_with('#')
            && env::var_os(k.trim()).is_none()
          {
            // SAFETY: single-threaded startup before spawning tasks.
            unsafe {
              env::set_var(k.trim(), v.trim().trim_matches('"'));
            }
          }
        }
      }
    }
    Yanshi::init_with_builtin_config().await?;
    Ok((
      env::var("YANSHI_TTS_MODEL").unwrap_or_else(|_| "gemini-2.5-flash-preview-tts".to_string()),
      env::var("YANSHI_TTS_VOICE").unwrap_or_else(|_| "Kore".to_string()),
    ))
  } else {
    let cfg = output_dir.join("mock_models.yml");
    tokio::fs::write(
      &cfg,
      "models:\n  mock-tts:\n    vendor: mock\n    type: tts\n    model_id: mock-tts\nproviders:\n  mock:\n    api_key_env: MOCK_API_KEY\n",
    )
    .await?;
    let cfg_str = cfg
      .to_str()
      .ok_or_else(|| anyhow::anyhow!("non-utf8 mock path"))?;
    Yanshi::init_with_config(cfg_str).await?;
    Ok(("mock-tts".to_string(), "Kore".to_string()))
  }
}

type ChapterSegment = (PathBuf, f64, Vec<(PathBuf, f64)>);

fn stitch_1080p_mp4(
  output_dir: &Path,
  chapters: &[ChapterSegment],
  srt_path: &Path,
  mp4_path: &Path,
) -> anyhow::Result<bool> {
  if which::which("ffmpeg").is_err() {
    return Ok(false);
  }
  let work = output_dir.join(".ffmpeg-ppt-work");
  std::fs::create_dir_all(&work)?;
  let vf = "scale=1920:1080:force_original_aspect_ratio=decrease,pad=1920:1080:(ow-iw)/2:(oh-ih)/2:color=0x0B1120,setsar=1,fps=25,format=yuv420p";

  let res = (|| -> anyhow::Result<()> {
    let mut seg_manifest = String::new();
    for (ci, (wav_path, total_secs, frames)) in chapters.iter().enumerate() {
      let frame_txt = work.join(format!("ch_{ci:02}_frames.txt"));
      let mut list = String::new();
      for (png, dur) in frames {
        let abs = png.canonicalize().unwrap_or_else(|_| png.clone());
        let esc = abs.to_string_lossy().replace('\'', "'\\''");
        let _ = writeln!(list, "file '{esc}'\nduration {dur:.3}");
      }
      if let Some((last_png, _)) = frames.last() {
        let abs = last_png.canonicalize().unwrap_or_else(|_| last_png.clone());
        let esc = abs.to_string_lossy().replace('\'', "'\\''");
        let _ = writeln!(list, "file '{esc}'");
      }
      std::fs::write(&frame_txt, list)?;

      let seg_mp4 = work.join(format!("seg_{ci:02}.mp4"));
      let status = Command::new("ffmpeg")
        .args(["-y", "-hide_banner", "-loglevel", "error"])
        .args(["-f", "concat", "-safe", "0", "-i"])
        .arg(&frame_txt)
        .arg("-i")
        .arg(wav_path)
        .args(["-map", "0:v:0", "-map", "1:a:0", "-vf", vf])
        .args(["-c:v", "libx264", "-preset", "veryfast", "-crf", "20"])
        .args(["-c:a", "aac", "-b:a", "160k", "-ar", "44100", "-ac", "2"])
        .args(["-t", &format!("{total_secs:.3}")])
        .arg(&seg_mp4)
        .status()?;
      if !status.success() {
        anyhow::bail!("ffmpeg failed encoding chapter segment {ci}");
      }
      let _ = writeln!(seg_manifest, "file 'seg_{ci:02}.mp4'");
    }

    let manifest_path = work.join("segments.txt");
    std::fs::write(&manifest_path, seg_manifest)?;
    let status = Command::new("ffmpeg")
      .args(["-y", "-hide_banner", "-loglevel", "error"])
      .args(["-f", "concat", "-safe", "0", "-i"])
      .arg(&manifest_path)
      .arg("-i")
      .arg(srt_path)
      .args([
        "-map",
        "0:v",
        "-map",
        "0:a",
        "-map",
        "1:0",
        "-c:v",
        "copy",
        "-c:a",
        "copy",
        "-c:s",
        "mov_text",
        "-metadata:s:s:0",
        "language=zho",
        "-movflags",
        "+faststart",
      ])
      .arg(mp4_path)
      .status()?;
    if !status.success() {
      anyhow::bail!("ffmpeg failed muxing final 1080p MP4");
    }
    Ok(())
  })();

  let _ = std::fs::remove_dir_all(&work);
  res?;
  Ok(true)
}

fn export_pptx(md_path: &Path, pptx_path: &Path, resource_dir: &Path) -> bool {
  let Ok(pandoc) = which::which("pandoc") else {
    return false;
  };
  Command::new(pandoc)
    .arg(md_path)
    .args([
      "-f",
      "markdown-raw_html",
      "--standalone",
      "-t",
      "pptx",
      "-o",
    ])
    .arg(pptx_path)
    .arg("--resource-path")
    .arg(resource_dir)
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

fn export_typst_pdf(typ_path: &Path, pdf_path: &Path, root_dir: &Path) -> bool {
  let Ok(typst) = which::which("typst") else {
    return false;
  };
  Command::new(typst)
    .arg("compile")
    .arg("--root")
    .arg(root_dir)
    .arg(typ_path)
    .arg(pdf_path)
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
}

fn file_non_empty(path: &Path) -> bool {
  std::fs::metadata(path)
    .map(|m| m.is_file() && m.len() > 0)
    .unwrap_or(false)
}

fn wav_duration_seconds(bytes: &[u8]) -> Option<f64> {
  if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
    return None;
  }
  let mut cursor = 12_usize;
  let mut byte_rate: Option<u32> = None;
  let mut data_len: Option<u32> = None;
  while cursor + 8 <= bytes.len() {
    let cid = &bytes[cursor..cursor + 4];
    let csz = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().ok()?);
    let body = cursor + 8;
    if cid == b"fmt " && body + 12 <= bytes.len() {
      let rate = u32::from_le_bytes(bytes[body + 8..body + 12].try_into().ok()?);
      if rate > 0 {
        byte_rate = Some(rate);
      }
    } else if cid == b"data" {
      let avail = (bytes.len().saturating_sub(body)) as u32;
      data_len = Some(csz.min(avail));
      break;
    }
    let padded = (csz as usize) + ((csz as usize) & 1);
    cursor = body.checked_add(padded)?;
  }
  let secs = f64::from(data_len?) / f64::from(byte_rate?);
  (secs.is_finite() && secs > 0.0).then_some(secs)
}

fn split_sentences_zh(text: &str) -> Vec<String> {
  let mut out = Vec::new();
  let mut cur = String::new();
  for ch in text.chars() {
    cur.push(ch);
    if matches!(ch, '。' | '！' | '？' | '；') {
      let t = cur.trim();
      if !t.is_empty() {
        out.push(t.to_string());
      }
      cur.clear();
    }
  }
  let tail = cur.trim();
  if !tail.is_empty() {
    out.push(tail.to_string());
  }
  out
}

fn fmt_ts(secs: f64, sep: char) -> String {
  let ms_all = (secs.max(0.0) * 1000.0).round() as u64;
  let ms = ms_all % 1000;
  let s_all = ms_all / 1000;
  format!(
    "{:02}:{:02}:{:02}{sep}{ms:03}",
    s_all / 3600,
    (s_all / 60) % 60,
    s_all % 60
  )
}

fn format_srt(cues: &[SubtitleCue]) -> String {
  let mut out = String::new();
  for (i, c) in cues.iter().enumerate() {
    let _ = writeln!(
      out,
      "{}\n{} --> {}\n{}\n",
      i + 1,
      fmt_ts(c.start_secs, ','),
      fmt_ts(c.end_secs, ','),
      c.text
    );
  }
  out
}

fn format_vtt(cues: &[SubtitleCue]) -> String {
  let mut out = String::from("WEBVTT\n\n");
  for (i, c) in cues.iter().enumerate() {
    let _ = writeln!(
      out,
      "{}\n{} --> {}\n{}\n",
      i + 1,
      fmt_ts(c.start_secs, '.'),
      fmt_ts(c.end_secs, '.'),
      c.text
    );
  }
  out
}

fn encode_base64(input: &[u8]) -> String {
  const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
  for chunk in input.chunks(3) {
    let b0 = chunk[0];
    let b1 = *chunk.get(1).unwrap_or(&0);
    let b2 = *chunk.get(2).unwrap_or(&0);
    out.push(TABLE[(b0 >> 2) as usize] as char);
    out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
    if chunk.len() > 1 {
      out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
    } else {
      out.push('=');
    }
    if chunk.len() > 2 {
      out.push(TABLE[(b2 & 0x3f) as usize] as char);
    } else {
      out.push('=');
    }
  }
  out
}

// ── Chapter Audio → Slide Mapping ───────────────────────────────────────

fn chapter_audio_plans() -> Vec<ChapterAudioPlan> {
  vec![
    ChapterAudioPlan {
      chapter_index: 1,
      audio_file: "ch01.wav",
      narration: "欢迎来到 Rust Traits 深度精讲。要真正读懂 Rust，首先要忘掉面向对象里的基类继承。Trait 的思想源于 Haskell 的 Typeclass：它不是数据的父节点，而是一份正交的共享行为契约。结构体负责规定字节在内存里怎么摆放，而 Trait 负责声明这个类型向外界承诺哪些能力。正因为数据与行为彻底解耦，任何独立演化的类型都能在不改动内存布局的前提下，安全地接入标准库与生态系统的通用算法。",
      slide_indices: &[0],
    },
    ChapterAudioPlan {
      chapter_index: 2,
      audio_file: "ch02.wav",
      narration: "第二章我们看如何精确描述一个 Trait。除了普通的方法签名，Rust 提供了四项关键表达工具。第一，通过借用、可变借用或按值获取所有权的接收者，直接在签名上钉死所有权流转；第二，用少量核心方法驱动大量默认实现，就像标准库迭代器只要求你写一个 next，就白送几十个组合子；第三，区分关联类型与泛型参数：输入多态用泛型，输出唯一用关联类型；第四，用 Supertrait 声明前置契约。",
      slide_indices: &[1, 2],
    },
    ChapterAudioPlan {
      chapter_index: 3,
      audio_file: "ch03.wav",
      narration: "第三章进入 Trait 的核心引擎室：静态分发与动态分发。当你写泛型约束或 impl Trait 时，Rust 编译器在编译期执行单态化，为每个具体类型生成专属机器码，函数可以直接内联，实现真正的零成本抽象。而当你需要在一个集合里混装多种运行时才确定的类型时，借用 dyn Trait 会构造一个十六字节的胖指针：前八字节指向真实数据，后八字节指向只读虚表。数据本身依然没有对象头负担，鱼与熊掌各归其位。",
      slide_indices: &[3, 4],
    },
    ChapterAudioPlan {
      chapter_index: 4,
      audio_file: "ch04.wav",
      narration: "第四章我们把视野拉开，横向对比面向对象类继承与 C 语言函数指针。传统面向对象把数据继承与接口多态绑死在脆弱的基类树上，不仅容易陷入菱形继承泥潭，还迫使每个对象实例都背负隐藏的对象头或虚表指针。而在 C 语言里，像 Linux 内核的文件操作表虽然用函数指针结构体加 void 星号实现了零开销解耦，但 void 星号彻底擦除了类型与生命周期，一次错误的强制转换就是内存破坏。Rust Trait 恰好融合了两者的精华：既有组合优于继承的正交弹性，又有超越手写虚表的编译期绝对安全。",
      slide_indices: &[5],
    },
    ChapterAudioPlan {
      chapter_index: 5,
      audio_file: "ch05.wav",
      narration: "第五章聚焦生产实战中最容易踩坑的四个问题。第一是孤儿规则：为了防止链接期实现冲突，Trait 和目标类型至少要有一个定义在当前包内，需要跨包桥接时请使用零开销的 Newtype 包装。第二是 dyn 兼容性：含有泛型方法或按值返回 Self 的 Trait 无法直接生成单张静态虚表。第三是单态化膨胀：对外暴露泛型便利接口时，应当把核心重逻辑下沉到非泛型的内部函数里减少二进制体积。第四条最重要：在第二个真实实现出现之前，绝不要为了显得高级而提前抽空壳 Trait。",
      slide_indices: &[6, 7],
    },
    ChapterAudioPlan {
      chapter_index: 6,
      audio_file: "ch06.wav",
      narration: "最后一章，让我们跳出语法细节，思考 Rust Trait 背后的工程哲学。传统分类学总在追问一个事物是什么，一旦业务演化跨越树状层级，重构就举步维艰；而 Trait 只追问它在当前上下文里能履行什么契约。更令人着迷的是像 Send 和 Sync 这样的标记 Trait：它们连一行运行期指令都没有，却把并发数据竞争从凌晨三点的线上崩溃，前移成了编译期的一条清晰报错。用最薄、最正交的契约切开复杂系统的变化边界，这就是 Rust 献给现代系统工程的最高智慧。",
      slide_indices: &[8, 9],
    },
  ]
}

// ── 10-Slide Technical Keynote Deck (22 Progressive Build Frames) ───────

fn build_rust_traits_deck() -> SlideDeck {
  SlideDeck {
    title: "Rust Traits 深度精讲".to_string(),
    subtitle: Some("契约表达 · 单态化与胖指针虚表 · 对比 OOP/C · 实战边界与类型哲学".to_string()),
    author: Some("Yanshi Content Studio · Systems Engineering Series".to_string()),
    theme: Some("dark".to_string()),
    slides: vec![
      // Slide 1 (Ch 1): Introduction — Data Layout vs Capability Contract
      Slide {
        section: Some("01 介绍".to_string()),
        title: "01 · 什么是 Trait：正交的行为契约（而非基类）".to_string(),
        subtitle: Some("源于 Haskell Typeclass：将「内存字节怎么摆」与「对外承诺什么能力」彻底解耦".to_string()),
        split_ratio: Some(0.50),
        left: Panel::Code {
          title: Some("src/summary.rs — 数据与契约正交".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// 1. 声明共享行为契约（零数据字段）\n",
            "pub trait Summary {\n",
            "  fn summarize(&self) -> String;\n",
            "}\n\n",
            "// 2. 独立结构体只关心自身内存布局\n",
            "pub struct Tweet {\n",
            "  pub user: String,\n",
            "  pub body: String,\n",
            "}\n\n",
            "// 3. 按需挂载契约（无需修改原结构体头）\n",
            "impl Summary for Tweet {\n",
            "  fn summarize(&self) -> String {\n",
            "    format!(\"@{}: {}\", self.user, self.body)\n",
            "  }\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 2,
              text: "纯行为契约".to_string(),
              tone: Some("rust".to_string()),
            },
            CodeCallout {
              line: 7,
              text: "零隐藏对象头".to_string(),
              tone: Some("cyan".to_string()),
            },
            CodeCallout {
              line: 13,
              text: "事后正交接入".to_string(),
              tone: Some("green".to_string()),
            },
          ],
        },
        right: Some(Panel::Flowchart {
          title: Some("正交解耦拓扑：具体类型 ↔ Trait 契约 ↔ 通用算法".to_string()),
          direction: Some("LR".to_string()),
          nodes: vec![
            FlowNode {
              id: "tweet".to_string(),
              label: "struct Tweet".to_string(),
              sublabel: Some("48B 纯栈/堆数据".to_string()),
              tone: Some("cyan".to_string()),
              col: Some(0),
              row: Some(0),
            },
            FlowNode {
              id: "article".to_string(),
              label: "struct Article".to_string(),
              sublabel: Some("72B 独立演化类型".to_string()),
              tone: Some("cyan".to_string()),
              col: Some(0),
              row: Some(1),
            },
            FlowNode {
              id: "ext".to_string(),
              label: "外部库类型".to_string(),
              sublabel: Some("无需改动上游布局".to_string()),
              tone: Some("violet".to_string()),
              col: Some(0),
              row: Some(2),
            },
            FlowNode {
              id: "trait".to_string(),
              label: "trait Summary".to_string(),
              sublabel: Some("Typeclass 能力窄腰".to_string()),
              tone: Some("rust".to_string()),
              col: Some(1),
              row: Some(1),
            },
            FlowNode {
              id: "algo".to_string(),
              label: "通用生态算法".to_string(),
              sublabel: Some("fn notify<T: Summary>".to_string()),
              tone: Some("green".to_string()),
              col: Some(2),
              row: Some(1),
            },
          ],
          edges: vec![
            FlowEdge {
              from: "tweet".to_string(),
              to: "trait".to_string(),
              label: Some("impl".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "article".to_string(),
              to: "trait".to_string(),
              label: Some("impl".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "ext".to_string(),
              to: "trait".to_string(),
              label: Some("按需扩展".to_string()),
              dashed: true,
            },
            FlowEdge {
              from: "trait".to_string(),
              to: "algo".to_string(),
              label: Some("编译期约束".to_string()),
              dashed: false,
            },
          ],
        }),
        takeaway: Some(
          "Struct 定义内存布局（Has-Layout），Trait 声明能力契约（Can-Do）：两者正交，彻底告别脆弱基类树。".to_string(),
        ),
        notes: Some("欢迎来到 Rust Traits 深度精讲。".to_string()),
        steps: vec![
          SlideStep {
            caption: "欢迎来到 Rust Traits 深度精讲。要真正读懂 Rust，首先要忘掉面向对象里的基类继承。".to_string(),
            left_reveal: Some(1),
            right_reveal: Some(1),
            code_lines: Some((1, 4)),
            focus_ids: vec!["trait".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "Trait 的思想源于 Haskell 的 Typeclass：它不是数据的父节点，而是一份正交的共享行为契约。".to_string(),
            left_reveal: Some(2),
            right_reveal: Some(4),
            code_lines: Some((1, 4)),
            focus_ids: vec!["tweet".to_string(), "article".to_string(), "trait".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "结构体负责规定字节在内存里怎么摆放，而 Trait 负责声明这个类型向外界承诺哪些能力。".to_string(),
            left_reveal: Some(3),
            right_reveal: Some(4),
            code_lines: Some((6, 17)),
            focus_ids: vec!["tweet".to_string(), "trait".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "正因为数据与行为彻底解耦，任何独立演化的类型都能在不改动内存布局的前提下，安全地接入标准库与生态系统的通用算法。".to_string(),
            left_reveal: None,
            right_reveal: Some(5),
            code_lines: None,
            focus_ids: vec!["ext".to_string(), "trait".to_string(), "algo".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 2 (Ch 2A): Describing Traits — Ownership Receivers & Default Methods
      Slide {
        section: Some("02 描述方式".to_string()),
        title: "02A · 描述方式：接收者所有权语义与默认方法组合子".to_string(),
        subtitle: Some("在函数签名上钉死所有权流转，并以极小核心方法驱动整套默认能力".to_string()),
        split_ratio: Some(0.52),
        left: Panel::Bullets {
          title: Some("四类接收者签名语义（Ownership Contract）".to_string()),
          grid: true,
          items: vec![
            BulletItem {
              badge: Some("&self".to_string()),
              heading: "共享只读借用".to_string(),
              body: "不获取所有权，允许多个调用方并发只读观测，如 Display::fmt / len()。".to_string(),
              tone: Some("cyan".to_string()),
            },
            BulletItem {
              badge: Some("&mut self".to_string()),
              heading: "独占可变借用".to_string(),
              body: "编译期保证唯一写入口，驱动内部状态推进，如 Iterator::next(&mut self)。".to_string(),
              tone: Some("rust".to_string()),
            },
            BulletItem {
              badge: Some("self".to_string()),
              heading: "按值消费与转化".to_string(),
              body: "转移所有权并销毁旧形态，适合零拷贝转换或状态机迁跃，如 IntoIterator。".to_string(),
              tone: Some("green".to_string()),
            },
            BulletItem {
              badge: Some("Self".to_string()),
              heading: "关联构造与常量".to_string(),
              body: "无实例接收者，指代当前具体实现类型本身，如 Default::default() -> Self。".to_string(),
              tone: Some("violet".to_string()),
            },
          ],
        },
        right: Some(Panel::Code {
          title: Some("std::iter::Iterator — 1 个核心方法撬动 70+ 默认方法".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "pub trait Iterator {\n",
            "  type Item;\n\n",
            "  // 唯一必须实现的最小完备基（Minimal Basis）\n",
            "  fn next(&mut self) -> Option<Self::Item>;\n\n",
            "  // 默认方法：基于 next() 零成本复用整套组合子\n",
            "  fn count(mut self) -> usize\n",
            "  where\n",
            "    Self: Sized,\n",
            "  {\n",
            "    let mut n = 0;\n",
            "    while self.next().is_some() { n += 1; }\n",
            "    n\n",
            "  }\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 5,
              text: "必填核心方法".to_string(),
              tone: Some("rust".to_string()),
            },
            CodeCallout {
              line: 8,
              text: "白送组合子".to_string(),
              tone: Some("green".to_string()),
            },
          ],
        }),
        takeaway: Some(
          "最小完备基原则：只把无法推导的原子操作设为必填方法，其余高频工具全部提供默认实现。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第二章我们看如何精确描述一个 Trait。除了普通的方法签名，Rust 提供了四项关键表达工具。第一，通过借用、可变借用或按值获取所有权的接收者，直接在签名上钉死所有权流转；".to_string(),
            left_reveal: Some(4),
            right_reveal: Some(1),
            code_lines: Some((1, 5)),
            focus_ids: Vec::new(),
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "第二，用少量核心方法驱动大量默认实现，就像标准库迭代器只要求你写一个 next，就白送几十个组合子；".to_string(),
            left_reveal: Some(4),
            right_reveal: None,
            code_lines: Some((7, 16)),
            focus_ids: Vec::new(),
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 3 (Ch 2B): Associated Types vs Generic Type Params + Supertraits
      Slide {
        section: Some("02 描述方式".to_string()),
        title: "02B · 关联类型 vs 泛型参数 & Supertrait 前置契约".to_string(),
        subtitle: Some("输入多态用泛型 Trait<T>，输出唯一用关联类型 type Output".to_string()),
        split_ratio: Some(0.52),
        left: Panel::Code {
          title: Some("associated_vs_generic.rs".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// [泛型参数] 同一类型可实现多次（1 对 N 输入多态）\n",
            "pub trait From<T> {\n",
            "  fn from(value: T) -> Self;\n",
            "}\n",
            "// String 既可 From<&str>，也可 From<char>\n\n",
            "// [关联类型] 每个类型实现唯一（1 对 1 锁定输出）\n",
            "pub trait Iterator {\n",
            "  type Item; // 调用方无需写 I: Iterator<u32>\n",
            "  fn next(&mut self) -> Option<Self::Item>;\n",
            "}\n\n",
            "// [Supertrait] 声明实现本契约必须先满足上游契约\n",
            "pub trait Diagnostic: std::fmt::Display + std::fmt::Debug {\n",
            "  fn code(&self) -> &'static str;\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 2,
              text: "1:N 输入重载".to_string(),
              tone: Some("cyan".to_string()),
            },
            CodeCallout {
              line: 9,
              text: "1:1 唯一输出".to_string(),
              tone: Some("rust".to_string()),
            },
            CodeCallout {
              line: 14,
              text: "前置父契约".to_string(),
              tone: Some("violet".to_string()),
            },
          ],
        },
        right: Some(Panel::Flowchart {
          title: Some("选型决策流 & Supertrait 依赖链".to_string()),
          direction: Some("TB".to_string()),
          nodes: vec![
            FlowNode {
              id: "q".to_string(),
              label: "同一具体类型需要实现多次吗？".to_string(),
              sublabel: Some("如 From<&str> 与 From<String>".to_string()),
              tone: Some("amber".to_string()),
              col: Some(0),
              row: Some(0),
            },
            FlowNode {
              id: "gen".to_string(),
              label: "使用泛型参数 Trait<T>".to_string(),
              sublabel: Some("调用方按输入类型分派".to_string()),
              tone: Some("cyan".to_string()),
              col: Some(0),
              row: Some(1),
            },
            FlowNode {
              id: "assoc".to_string(),
              label: "使用关联类型 type Item".to_string(),
              sublabel: Some("收敛类型推断，拒绝泛型噪音".to_string()),
              tone: Some("rust".to_string()),
              col: Some(1),
              row: Some(1),
            },
            FlowNode {
              id: "sup".to_string(),
              label: "Supertrait: Display + Debug".to_string(),
              sublabel: Some("不是内存继承，而是编译期集合包含".to_string()),
              tone: Some("violet".to_string()),
              col: Some(1),
              row: Some(2),
            },
          ],
          edges: vec![
            FlowEdge {
              from: "q".to_string(),
              to: "gen".to_string(),
              label: Some("是 (1:N)".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "q".to_string(),
              to: "assoc".to_string(),
              label: Some("否 (1:1)".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "assoc".to_string(),
              to: "sup".to_string(),
              label: Some("组合前置能力".to_string()),
              dashed: true,
            },
          ],
        }),
        takeaway: Some(
          "判断口诀：同一目标类型若需按不同参数重载多次选 Trait<T>；若给定类型结果唯一必选关联类型。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第三，区分关联类型与泛型参数：输入多态用泛型，输出唯一用关联类型；".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: Some((1, 11)),
            focus_ids: vec!["q".to_string(), "gen".to_string(), "assoc".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "第四，用 Supertrait 声明前置契约。".to_string(),
            left_reveal: None,
            right_reveal: Some(4),
            code_lines: Some((13, 16)),
            focus_ids: vec!["sup".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 4 (Ch 3A): Static Dispatch & Monomorphization Pipeline
      Slide {
        section: Some("03 应用方法".to_string()),
        title: "03A · 应用方法（一）：编译期单态化与零开销静态分发".to_string(),
        subtitle: Some("T: Trait 与 impl Trait 在编译期展开为专属机器码，支持跨函数 LLVM 内联".to_string()),
        split_ratio: Some(0.47),
        left: Panel::Code {
          title: Some("static_dispatch.rs".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "pub trait Draw {\n",
            "  fn draw(&self) -> u32;\n",
            "}\n\n",
            "// 泛型约束：编译期静态单态化（Monomorphization）\n",
            "pub fn render_batch<T: Draw>(items: &[T]) -> u32 {\n",
            "  items.iter().map(Draw::draw).sum()\n",
            "}\n\n",
            "// 编译器自动生成两份独立专属函数：\n",
            "// fn render_batch_Button(items: &[Button]) -> u32\n",
            "// fn render_batch_Icon(items: &[Icon]) -> u32\n\n",
            "// Blanket Impl：一次赋能所有满足条件的类型\n",
            "impl<T: std::fmt::Display> ToString for T { /* ... */ }"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 6,
              text: "0 虚表跳转".to_string(),
              tone: Some("green".to_string()),
            },
            CodeCallout {
              line: 15,
              text: "全生态覆盖实现".to_string(),
              tone: Some("cyan".to_string()),
            },
          ],
        },
        right: Some(Panel::Flowchart {
          title: Some("rustc 单态化编译管线（Static Dispatch Pipeline）".to_string()),
          direction: Some("TB".to_string()),
          nodes: vec![
            FlowNode {
              id: "src".to_string(),
              label: "泛型源码 fn render<T: Draw>".to_string(),
              sublabel: Some("HIR / THIR 阶段完成 Trait Bound 类型检查".to_string()),
              tone: Some("cyan".to_string()),
              col: Some(0),
              row: Some(0),
            },
            FlowNode {
              id: "m1".to_string(),
              label: "render_Button(&[Button])".to_string(),
              sublabel: Some("MIR 实例化具体类型 #1".to_string()),
              tone: Some("rust".to_string()),
              col: Some(0),
              row: Some(1),
            },
            FlowNode {
              id: "m2".to_string(),
              label: "render_Icon(&[Icon])".to_string(),
              sublabel: Some("MIR 实例化具体类型 #2".to_string()),
              tone: Some("rust".to_string()),
              col: Some(1),
              row: Some(1),
            },
            FlowNode {
              id: "llvm".to_string(),
              label: "LLVM 内联 + SIMD 自动向量化".to_string(),
              sublabel: Some("直接 call / inline，零运行期查表开销".to_string()),
              tone: Some("green".to_string()),
              col: Some(0),
              row: Some(2),
            },
          ],
          edges: vec![
            FlowEdge {
              from: "src".to_string(),
              to: "m1".to_string(),
              label: Some("T = Button".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "src".to_string(),
              to: "m2".to_string(),
              label: Some("T = Icon".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "m1".to_string(),
              to: "llvm".to_string(),
              label: Some("内联优化".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "m2".to_string(),
              to: "llvm".to_string(),
              label: Some("内联优化".to_string()),
              dashed: false,
            },
          ],
        }),
        takeaway: Some(
          "静态分发把多态成本完全留在编译期：热路径循环与数值计算优先使用 T: Trait 泛型静态分发。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第三章进入 Trait 的核心引擎室：静态分发与动态分发。".to_string(),
            left_reveal: None,
            right_reveal: Some(1),
            code_lines: Some((1, 8)),
            focus_ids: vec!["src".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "当你写泛型约束或 impl Trait 时，Rust 编译器在编译期执行单态化，为每个具体类型生成专属机器码，函数可以直接内联，实现真正的零成本抽象。".to_string(),
            left_reveal: None,
            right_reveal: Some(4),
            code_lines: Some((5, 15)),
            focus_ids: vec!["m1".to_string(), "m2".to_string(), "llvm".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 5 (Ch 3B): Dynamic Dispatch & 16-Byte Fat Pointer Memory Layout
      Slide {
        section: Some("03 应用方法".to_string()),
        title: "03B · 应用方法（二）：&dyn Trait 16 字节胖指针与虚表内存模型".to_string(),
        subtitle: Some("按需动态多态：栈上双指针分离数据与虚表，堆实例本身零 vptr 对象头污染".to_string()),
        split_ratio: Some(0.38),
        left: Panel::Code {
          title: Some("dyn_vtable.rs".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// 异构集合：同一 Vec 混装不同大小组件\n",
            "pub struct Screen {\n",
            "  pub views: Vec<Box<dyn Draw>>,\n",
            "}\n\n",
            "impl Screen {\n",
            "  pub fn run(&self) {\n",
            "    for v in &self.views {\n",
            "      // 通过 vtable_ptr 查表间接调用\n",
            "      v.draw();\n",
            "    }\n",
            "  }\n",
            "}\n\n",
            "// size_of::<&Button>()    ==  8 Bytes\n",
            "// size_of::<&dyn Draw>()  == 16 Bytes"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 3,
              text: "异构插件边界".to_string(),
              tone: Some("rust".to_string()),
            },
            CodeCallout {
              line: 16,
              text: "16B Fat Ptr".to_string(),
              tone: Some("cyan".to_string()),
            },
          ],
        },
        right: Some(Panel::Memory {
          title: Some("&dyn Draw 真实内存布局（64-bit 架构）".to_string()),
          regions: vec![
            MemRegion {
              name: "Stack 栈帧".to_string(),
              subtitle: Some("&dyn Draw (16B)".to_string()),
              tone: Some("rust".to_string()),
              slots: vec![
                MemSlot {
                  id: "data_ptr".to_string(),
                  offset: Some("+0x00".to_string()),
                  label: "data_ptr".to_string(),
                  value: "0x6000_01a0 -> 实例地址".to_string(),
                },
                MemSlot {
                  id: "vtable_ptr".to_string(),
                  offset: Some("+0x08".to_string()),
                  label: "vtable_ptr".to_string(),
                  value: "0x1000_8f00 -> 只读虚表".to_string(),
                },
              ],
            },
            MemRegion {
              name: "Heap 堆数据".to_string(),
              subtitle: Some("Button (无 vptr 头!)".to_string()),
              tone: Some("green".to_string()),
              slots: vec![
                MemSlot {
                  id: "btn_w".to_string(),
                  offset: Some("+0x00".to_string()),
                  label: "width: u32".to_string(),
                  value: "120 (纯业务字段)".to_string(),
                },
                MemSlot {
                  id: "btn_lbl".to_string(),
                  offset: Some("+0x08".to_string()),
                  label: "label: String".to_string(),
                  value: "\"Submit\" (24B)".to_string(),
                },
              ],
            },
            MemRegion {
              name: ".rodata 静态虚表".to_string(),
              subtitle: Some("VTable for Button".to_string()),
              tone: Some("cyan".to_string()),
              slots: vec![
                MemSlot {
                  id: "vt_drop".to_string(),
                  offset: Some("+0x00".to_string()),
                  label: "drop_in_place".to_string(),
                  value: "fn(*mut Button)".to_string(),
                },
                MemSlot {
                  id: "vt_meta".to_string(),
                  offset: Some("+0x08".to_string()),
                  label: "size / align".to_string(),
                  value: "size: 32B, align: 8B".to_string(),
                },
                MemSlot {
                  id: "vt_draw".to_string(),
                  offset: Some("+0x18".to_string()),
                  label: "draw".to_string(),
                  value: "Button::draw as fn".to_string(),
                },
              ],
            },
          ],
          pointers: vec![
            MemPointer {
              from: "data_ptr".to_string(),
              to: "btn_w".to_string(),
              label: Some("指向纯数据".to_string()),
              tone: Some("green".to_string()),
            },
            MemPointer {
              from: "vtable_ptr".to_string(),
              to: "vt_draw".to_string(),
              label: Some("查表调用".to_string()),
              tone: Some("cyan".to_string()),
            },
          ],
        }),
        takeaway: Some(
          "Pay-only-for-what-you-use：不借用为 dyn 时对象 0 额外字节；借用为 &dyn Trait 时仅在引用处构造 16B 胖指针。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "而当你需要在一个集合里混装多种运行时才确定的类型时，借用 dyn Trait 会构造一个十六字节的胖指针：前八字节指向真实数据，后八字节指向只读虚表。".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: Some((1, 16)),
            focus_ids: vec!["data_ptr".to_string(), "vtable_ptr".to_string(), "vt_draw".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "数据本身依然没有对象头负担，鱼与熊掌各归其位。".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: None,
            focus_ids: vec!["btn_w".to_string(), "btn_lbl".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 6 (Ch 4): Comparison Matrix vs OOP Inheritance & C void* Function Pointers
      Slide {
        section: Some("04 核心优点".to_string()),
        title: "04 · 横向对比：Rust Traits vs OOP 类继承 vs C 语言 void* 函数指针".to_string(),
        subtitle: Some("兼具组合优于继承的正交弹性，以及超越手写 C 虚表的编译期内存与生命周期安全".to_string()),
        split_ratio: None,
        left: Panel::Table {
          title: Some("三大系统多态范式核心工程指标全维对比".to_string()),
          highlight_col: Some(1),
          columns: vec![
            "评估维度".to_string(),
            "Rust Traits（Typeclass 契约）".to_string(),
            "OOP 类继承（C++ / Java）".to_string(),
            "C 语言（struct ops + void*）".to_string(),
          ],
          rows: vec![
            vec![
              "数据与行为耦合度".to_string(),
              "✓ 完全正交分离，纯组合无菱形继承陷阱".to_string(),
              "✗ 状态与接口绑死基类树，易陷菱形继承泥潭".to_string(),
              "✓ 手工分离结构体与 ops 函数指针表".to_string(),
            ],
            vec![
              "对象实例内存负担".to_string(),
              "✓ 实例 0 对象头；仅 &dyn 借用处占 16B".to_string(),
              "✗ 每个虚类/对象实例强制携带 vptr 或 Object 头".to_string(),
              "✓ 按需存放 ops 指针，无强制隐藏头".to_string(),
            ],
            vec![
              "类型与生命周期安全".to_string(),
              "✓ 编译期严格校验类型、借用读写与生命周期".to_string(),
              "⚠ 类型安全，但共享可变引用易发并发/悬垂问题".to_string(),
              "✗ void* 擦除全部类型，强转写错即段错误/内存破坏".to_string(),
            ],
            vec![
              "为外部类型事后扩展".to_string(),
              "✓ 原生支持在本地包为上游/标准库类型实现本地 Trait".to_string(),
              "✗ 无法修改第三方已编译类的 extends / implements".to_string(),
              "⚠ 需外包一层适配器结构体手工转发".to_string(),
            ],
            vec![
              "热点循环零开销优化".to_string(),
              "✓ 泛型单态化 100% 静态内联 + SIMD 向量化".to_string(),
              "⚠ 依赖 JIT 去虚化或 C++ 模板（缺乏约束报错晦涩）".to_string(),
              "✗ 函数指针间接跳转阻碍编译器跨调用内联".to_string(),
            ],
          ],
        },
        right: None,
        takeaway: Some(
          "架构判据：Rust Traits = Haskell Typeclass 的编译期严谨性 + C 手写 vtable 的可预测零开销内存控制。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第四章我们把视野拉开，横向对比面向对象类继承与 C 语言函数指针。传统面向对象把数据继承与接口多态绑死在脆弱的基类树上，不仅容易陷入菱形继承泥潭，还迫使每个对象实例都背负隐藏的对象头或虚表指针。".to_string(),
            left_reveal: Some(2),
            right_reveal: None,
            code_lines: None,
            focus_ids: Vec::new(),
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "而在 C 语言里，像 Linux 内核的文件操作表虽然用函数指针结构体加 void 星号实现了零开销解耦，但 void 星号彻底擦除了类型与生命周期，一次错误的强制转换就是内存破坏。".to_string(),
            left_reveal: Some(4),
            right_reveal: None,
            code_lines: None,
            focus_ids: Vec::new(),
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "Rust Trait 恰好融合了两者的精华：既有组合优于继承的正交弹性，又有超越手写虚表的编译期绝对安全。".to_string(),
            left_reveal: Some(5),
            right_reveal: None,
            code_lines: None,
            focus_ids: Vec::new(),
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 7 (Ch 5A): Pitfalls — Orphan Rule Newtype & Object Safety Flowchart
      Slide {
        section: Some("05 注意事项".to_string()),
        title: "05A · 实战避坑：孤儿规则 Newtype 桥接与 dyn 对象安全判定".to_string(),
        subtitle: Some("理解相干性（Coherence）与静态虚表构造前提，秒解 E0117 与 E0038 编译报错".to_string()),
        split_ratio: Some(0.49),
        left: Panel::Code {
          title: Some("orphan_and_dyn_safety.rs".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// [ERR] 违反孤儿规则 E0117：Display 与 Vec 均属外部包\n",
            "// impl fmt::Display for Vec<String> { ... }\n\n",
            "// [OK] 零成本修复：#[repr(transparent)] Newtype 包装\n",
            "#[repr(transparent)]\n",
            "pub struct PrettyVec(pub Vec<String>);\n",
            "impl std::fmt::Display for PrettyVec { /* 合法! */ }\n\n",
            "// [OK] dyn 兼容改造：用 where Self: Sized 隔离非虚表方法\n",
            "pub trait Plugin {\n",
            "  fn execute(&self, input: &str) -> String;\n",
            "  fn clone_typed(&self) -> Self\n",
            "  where\n",
            "    Self: Sized; // 对 &dyn Plugin 自动屏蔽此方法\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 4,
              text: "0 额外内存包装".to_string(),
              tone: Some("green".to_string()),
            },
            CodeCallout {
              line: 14,
              text: "保留 dyn 兼容性".to_string(),
              tone: Some("cyan".to_string()),
            },
          ],
        },
        right: Some(Panel::Flowchart {
          title: Some("编译器 dyn Trait 对象安全（Object Safety）判定流".to_string()),
          direction: Some("TB".to_string()),
          nodes: vec![
            FlowNode {
              id: "c1".to_string(),
              label: "1. Supertrait 是否含 Sized？".to_string(),
              sublabel: Some("trait Foo: Sized 禁止擦除大小".to_string()),
              tone: Some("amber".to_string()),
              col: Some(0),
              row: Some(0),
            },
            FlowNode {
              id: "c2".to_string(),
              label: "2. 方法是否按值返回裸 Self？".to_string(),
              sublabel: Some("调用方栈帧无法预知返回值字节数".to_string()),
              tone: Some("amber".to_string()),
              col: Some(0),
              row: Some(1),
            },
            FlowNode {
              id: "c3".to_string(),
              label: "3. 方法是否带泛型参数 fn f<T>？".to_string(),
              sublabel: Some("无限 T 无法塞进固定长度静态 vtable".to_string()),
              tone: Some("rose".to_string()),
              col: Some(0),
              row: Some(2),
            },
            FlowNode {
              id: "ok".to_string(),
              label: "✓ 允许构造 &dyn Trait / Box<dyn Trait>".to_string(),
              sublabel: Some("或给违规方法加 where Self: Sized 豁免".to_string()),
              tone: Some("green".to_string()),
              col: Some(0),
              row: Some(3),
            },
          ],
          edges: vec![
            FlowEdge {
              from: "c1".to_string(),
              to: "c2".to_string(),
              label: Some("否".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "c2".to_string(),
              to: "c3".to_string(),
              label: Some("否".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "c3".to_string(),
              to: "ok".to_string(),
              label: Some("全部通过".to_string()),
              dashed: false,
            },
          ],
        }),
        takeaway: Some(
          "孤儿规则守护全局链接唯一性；对象安全守护固定布局虚表——所有报错背后都是物理内存的确定性约束。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第五章聚焦生产实战中最容易踩坑的四个问题。第一是孤儿规则：为了防止链接期实现冲突，Trait 和目标类型至少要有一个定义在当前包内，需要跨包桥接时请使用零开销的 Newtype 包装。".to_string(),
            left_reveal: None,
            right_reveal: Some(1),
            code_lines: Some((1, 7)),
            focus_ids: vec!["c1".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "第二是 dyn 兼容性：含有泛型方法或按值返回 Self 的 Trait 无法直接生成单张静态虚表。".to_string(),
            left_reveal: None,
            right_reveal: Some(4),
            code_lines: Some((9, 15)),
            focus_ids: vec!["c2".to_string(), "c3".to_string(), "ok".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 8 (Ch 5B): Pitfalls — Monomorphization Bloat Diet & Premature Abstraction Anti-Pattern
      Slide {
        section: Some("05 注意事项".to_string()),
        title: "05B · 实战避坑：单态化膨胀瘦身术与「拒绝单实现抽 Trait」".to_string(),
        subtitle: Some("控制二进制体积与编译耗时，警惕把面向对象过度抽象坏习惯带入 Rust".to_string()),
        split_ratio: Some(0.50),
        left: Panel::Code {
          title: Some("de_generify.rs — 外薄内厚消肿惯用法".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// [OK] 外层：极薄泛型壳（单态化后仅几条指令，零感内联）\n",
            "pub fn load_config<P: AsRef<std::path::Path>>(\n",
            "  path: P,\n",
            ") -> std::io::Result<String> {\n",
            "  load_config_inner(path.as_ref())\n",
            "}\n\n",
            "// [OK] 内层：非泛型重逻辑，全程序只编译生成唯一一份机器码!\n",
            "fn load_config_inner(\n",
            "  path: &std::path::Path,\n",
            ") -> std::io::Result<String> {\n",
            "  let raw = std::fs::read_to_string(path)?;\n",
            "  Ok(raw.trim().to_owned())\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 2,
              text: "人体工学泛型入口".to_string(),
              tone: Some("cyan".to_string()),
            },
            CodeCallout {
              line: 9,
              text: "单份机器码防膨胀".to_string(),
              tone: Some("green".to_string()),
            },
          ],
        },
        right: Some(Panel::Bullets {
          title: Some("生产级工程自律准则（Engineering Taste）".to_string()),
          grid: false,
          items: vec![
            BulletItem {
              badge: Some("ANTI-1".to_string()),
              heading: "拒绝「只有一个实现的 Trait」".to_string(),
              body: "抽象成本现在立刻支付，收益却要等第二个实现出现。单实现先写具体 struct，有真实变化边界再抽 Trait。".to_string(),
              tone: Some("rose".to_string()),
            },
            BulletItem {
              badge: Some("ANTI-2".to_string()),
              heading: "拒绝「巨石上帝 Trait（God Trait）」".to_string(),
              body: "一个 Trait 塞入 15 个无关方法会迫使调用方 mock 整个世界。保持像 Read / Write / AsyncNode 一样的单一职责。".to_string(),
              tone: Some("amber".to_string()),
            },
            BulletItem {
              badge: Some("RULE".to_string()),
              heading: "外薄泛型入口 + 内厚具体核心".to_string(),
              body: "std::fs::read / serde 广泛采用此手法，同时兼顾调用端 AsRef 便利性与极速增量编译。".to_string(),
              tone: Some("green".to_string()),
            },
          ],
        }),
        takeaway: Some(
          "好代码的判据不是「抽了多少层 Trait」，而是删掉冗余包装后，真正解决问题的核心逻辑占了多少。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "第三是单态化膨胀：对外暴露泛型便利接口时，应当把核心重逻辑下沉到非泛型的内部函数里减少二进制体积。".to_string(),
            left_reveal: None,
            right_reveal: Some(1),
            code_lines: Some((1, 14)),
            focus_ids: Vec::new(),
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "第四条最重要：在第二个真实实现出现之前，绝不要为了显得高级而提前抽空壳 Trait。".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: None,
            focus_ids: Vec::new(),
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 9 (Ch 6A): Deep Philosophy — Marker Traits, Typestate & Yanshi L0 Narrow Waist
      Slide {
        section: Some("06 总结深思".to_string()),
        title: "06A · 总结深思：编译期定理证明与 L0 窄腰契约架构".to_string(),
        subtitle: Some("从 Is-A 分类学走向 Can-Do 契约论；用零指令 Marker Trait 消灭线上并发灾难".to_string()),
        split_ratio: Some(0.48),
        left: Panel::Code {
          title: Some("typestate_and_markers.rs".to_string()),
          language: Some("rust".to_string()),
          code: concat!(
            "// 零指令标记契约：编译期证明跨线程并发安全性\n",
            "// pub unsafe auto trait Send {}\n",
            "// pub unsafe auto trait Sync {}\n\n",
            "// Typestate 状态机：非法状态在类型层面不可表达\n",
            "pub trait ConnState {}\n",
            "pub struct Closed;\n",
            "pub struct Authenticated;\n",
            "impl ConnState for Closed {}\n",
            "impl ConnState for Authenticated {}\n\n",
            "pub struct Session<S: ConnState>(std::marker::PhantomData<S>);\n",
            "impl Session<Authenticated> {\n",
            "  pub fn query(&self, sql: &str) { /* 未认证连编译都过不了 */ }\n",
            "}"
          )
          .to_string(),
          callouts: vec![
            CodeCallout {
              line: 2,
              text: "0 运行期指令".to_string(),
              tone: Some("green".to_string()),
            },
            CodeCallout {
              line: 14,
              text: "编译期防呆".to_string(),
              tone: Some("rust".to_string()),
            },
          ],
        },
        right: Some(Panel::Flowchart {
          title: Some("实战印证：Yanshi 五层窄腰契约架构（L0 Narrow Waist）".to_string()),
          direction: Some("TB".to_string()),
          nodes: vec![
            FlowNode {
              id: "up".to_string(),
              label: "L3/L4 编排与产品层".to_string(),
              sublabel: Some("yanshi-agents · yanshi-harness · yanshi-server".to_string()),
              tone: Some("cyan".to_string()),
              col: Some(0),
              row: Some(0),
            },
            FlowNode {
              id: "waist".to_string(),
              label: "L0 核心 Trait 窄腰（零运行时逻辑）".to_string(),
              sublabel: Some("trait AsyncNode · trait Tool · trait AgentRuntime".to_string()),
              tone: Some("rust".to_string()),
              col: Some(0),
              row: Some(1),
            },
            FlowNode {
              id: "low".to_string(),
              label: "L1/L2 执行器与多模态适配器".to_string(),
              sublabel: Some("yanshi-core · yanshi-llm · yanshi-nodes · sandbox".to_string()),
              tone: Some("green".to_string()),
              col: Some(0),
              row: Some(2),
            },
          ],
          edges: vec![
            FlowEdge {
              from: "up".to_string(),
              to: "waist".to_string(),
              label: Some("仅依赖共享契约".to_string()),
              dashed: false,
            },
            FlowEdge {
              from: "low".to_string(),
              to: "waist".to_string(),
              label: Some("独立演进实现".to_string()),
              dashed: false,
            },
          ],
        }),
        takeaway: Some(
          "当不变量被编码进 Trait 签名时，每一次 cargo check 都是在为整个分布式系统做形式化安全证明。".to_string(),
        ),
        notes: None,
        steps: vec![
          SlideStep {
            caption: "最后一章，让我们跳出语法细节，思考 Rust Trait 背后的工程哲学。传统分类学总在追问一个事物是什么，一旦业务演化跨越树状层级，重构就举步维艰；而 Trait 只追问它在当前上下文里能履行什么契约。".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: Some((5, 15)),
            focus_ids: vec!["up".to_string(), "waist".to_string(), "low".to_string()],
            show_takeaway: Some(false),
          },
          SlideStep {
            caption: "更令人着迷的是像 Send 和 Sync 这样的标记 Trait：它们连一行运行期指令都没有，却把并发数据竞争从凌晨三点的线上崩溃，前移成了编译期的一条清晰报错。".to_string(),
            left_reveal: None,
            right_reveal: Some(3),
            code_lines: Some((1, 4)),
            focus_ids: vec!["waist".to_string()],
            show_takeaway: Some(true),
          },
        ],
      },
      // Slide 10 (Ch 6B): Full-Stage Panorama Mind Map
      Slide {
        section: Some("06 总结深思".to_string()),
        title: "06B · 全景复盘：Rust Traits 知识体系思维导图".to_string(),
        subtitle: Some("一张图串联本质、语法、分发机制、横向优劣、实战铁律与架构哲学".to_string()),
        split_ratio: None,
        left: Panel::Mindmap {
          title: Some("Rust Traits 核心工程全景图（Mind Map）".to_string()),
          root: json!({
            "label": "Rust Traits",
            "children": [
              {
                "label": "1. 本质定位",
                "children": [
                  {"label": "Typeclass 共享行为契约"},
                  {"label": "Struct 布局与行为正交解耦"}
                ]
              },
              {
                "label": "2. 描述工具",
                "children": [
                  {"label": "&self / &mut self / self / Self"},
                  {"label": "最小核心方法 + 默认实现"},
                  {"label": "关联类型 (1:1) vs 泛型 (1:N)"},
                  {"label": "Supertraits 前置契约"}
                ]
              },
              {
                "label": "3. 分发引擎",
                "children": [
                  {"label": "静态单态化 → LLVM 内联零开销"},
                  {"label": "&dyn Trait → 16B 胖指针 + VTable"},
                  {"label": "Blanket Impl 批量生态赋能"}
                ]
              },
              {
                "label": "4. 对比优势",
                "children": [
                  {"label": "胜 OOP：无菱形继承 / 零对象头"},
                  {"label": "胜 C void*：编译期类型与借用安全"}
                ]
              },
              {
                "label": "5. 实战边界",
                "children": [
                  {"label": "孤儿规则 + #[repr(transparent)]"},
                  {"label": "dyn 对象安全三条件"},
                  {"label": "外薄内厚防膨胀 / 满两实现再抽 Trait"}
                ]
              },
              {
                "label": "6. 架构哲学",
                "children": [
                  {"label": "Send / Sync 零指令并发证明"},
                  {"label": "L0 窄腰契约隔离系统变化边界"}
                ]
              }
            ]
          }),
        },
        right: None,
        takeaway: Some(
          "用最薄、最正交的契约切开复杂系统的变化边界——这就是 Rust Traits 献给现代系统工程的最高智慧。".to_string(),
        ),
        notes: None,
        steps: vec![SlideStep {
          caption: "用最薄、最正交的契约切开复杂系统的变化边界，这就是 Rust 献给现代系统工程的最高智慧。".to_string(),
          left_reveal: Some(6),
          right_reveal: None,
          code_lines: None,
          focus_ids: Vec::new(),
          show_takeaway: Some(true),
        }],
      },
    ],
  }
}
