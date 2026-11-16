//! Deterministic technical slide deck renderer (`render_slides`).
//!
//! Turns structured JSON slide specifications — code snippets with line-level
//! spotlighting, architecture/decision flowcharts, pointer-linked memory
//! layouts, comparison matrices, multi-branch mind maps, and progressive
//! Keynote-style build steps — into crisp 1920×1080 vector SVGs, rasterised
//! PNG frames, Pandoc `.pptx` Markdown, Typst `.pdf` source, and a
//! self-contained interactive HTML presentation player.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};

use crate::common::artifacts::write_artifact;
use crate::nodes::render::svg_to_png;

const CANVAS_W: f32 = 1920.0;
const CANVAS_H: f32 = 1080.0;
const SANS_FONT: &str = "'PingFang SC', 'Hiragino Sans GB', 'STHeiti', 'Heiti SC', 'Noto Sans CJK SC', 'WenQuanYi Micro Hei', -apple-system, 'Segoe UI', Helvetica, Arial, sans-serif";
const MONO_FONT: &str = "'SF Mono', Menlo, Monaco, 'DejaVu Sans Mono', 'Liberation Mono', Consolas, 'PingFang SC', 'Hiragino Sans GB', 'Noto Sans CJK SC', monospace";

/// Full slide deck specification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlideDeck {
  pub title: String,
  #[serde(default)]
  pub subtitle: Option<String>,
  #[serde(default)]
  pub author: Option<String>,
  #[serde(default)]
  pub theme: Option<String>,
  #[serde(alias = "chapters")]
  pub slides: Vec<Slide>,
}

/// One slide (which expands into 1..=N progressive build frames for video).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Slide {
  #[serde(default)]
  pub section: Option<String>,
  pub title: String,
  #[serde(default)]
  pub subtitle: Option<String>,
  pub left: Panel,
  #[serde(default)]
  pub right: Option<Panel>,
  #[serde(default)]
  pub split_ratio: Option<f32>,
  #[serde(default)]
  pub takeaway: Option<String>,
  #[serde(default)]
  pub notes: Option<String>,
  #[serde(default, deserialize_with = "null_default")]
  pub steps: Vec<SlideStep>,
}

fn deserialize_code_lines<'de, D>(deserializer: D) -> Result<Option<(usize, usize)>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let val = Option::<Value>::deserialize(deserializer)?;
  let Some(v) = val else {
    return Ok(None);
  };
  match v {
    Value::Null => Ok(None),
    Value::Number(n) => {
      let line = n.as_u64().unwrap_or(0) as usize;
      Ok((line > 0).then_some((line, line)))
    }
    Value::Array(arr) => {
      let nums: Vec<usize> = arr
        .iter()
        .filter_map(|x| x.as_u64().map(|n| n as usize))
        .filter(|&n| n > 0)
        .collect();
      match nums.as_slice() {
        [] => Ok(None),
        [single] => Ok(Some((*single, *single))),
        [first, rest @ ..] => {
          let end = rest.last().copied().unwrap_or(*first);
          Ok(Some((*first.min(&end), *first.max(&end))))
        }
      }
    }
    Value::Object(map) => {
      let start = map
        .get("start")
        .or_else(|| map.get("from"))
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
      let end = map
        .get("end")
        .or_else(|| map.get("to"))
        .and_then(Value::as_u64)
        .unwrap_or(start as u64) as usize;
      if start > 0 && end > 0 {
        Ok(Some((start.min(end), start.max(end))))
      } else {
        Ok(None)
      }
    }
    _ => Ok(None),
  }
}

/// One progressive build step within a slide, synchronized with narration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SlideStep {
  pub caption: String,
  #[serde(default, alias = "narration")]
  pub spoken: Option<String>,
  #[serde(default)]
  pub left_reveal: Option<usize>,
  #[serde(default)]
  pub right_reveal: Option<usize>,
  #[serde(default, deserialize_with = "deserialize_code_lines")]
  pub code_lines: Option<(usize, usize)>,
  #[serde(default, deserialize_with = "null_default")]
  pub focus_ids: Vec<String>,
  #[serde(default)]
  pub show_takeaway: Option<bool>,
}

impl SlideStep {
  /// Spoken narration script for TTS (falls back to `caption` when `spoken` is absent).
  pub fn spoken_text(&self) -> &str {
    self
      .spoken
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .unwrap_or(self.caption.as_str())
  }
}

/// Visual rendering options for Keynote-grade video frames.
#[derive(Debug, Clone, Copy, Default)]
pub struct SlideRenderOptions {
  /// Global lecture progress in `0.0..=1.0`, rendered as a bottom edge progress bar.
  pub progress_ratio: Option<f32>,
  /// Reserve bottom-right PiP presenter safe zone so diagrams and bottom callouts are unobstructed.
  pub pip_safe_zone: bool,
  /// Keep bottom bar focused on persistent `takeaway` (useful when burning external ASS subtitles).
  pub takeaway_only_bar: bool,
}

fn null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
  D: serde::Deserializer<'de>,
  T: Default + Deserialize<'de>,
{
  Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Structured visual primitive occupying a slide column or full stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Panel {
  Bullets {
    #[serde(default)]
    title: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    grid: bool,
    #[serde(default, deserialize_with = "null_default")]
    items: Vec<BulletItem>,
  },
  Code {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    language: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    code: String,
    #[serde(default, deserialize_with = "null_default")]
    callouts: Vec<CodeCallout>,
  },
  Flowchart {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    direction: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    nodes: Vec<FlowNode>,
    #[serde(default, deserialize_with = "null_default")]
    edges: Vec<FlowEdge>,
  },
  Memory {
    #[serde(default)]
    title: Option<String>,
    #[serde(default, deserialize_with = "null_default")]
    regions: Vec<MemRegion>,
    #[serde(default, deserialize_with = "null_default")]
    pointers: Vec<MemPointer>,
  },
  Table {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    highlight_col: Option<usize>,
    #[serde(default, deserialize_with = "null_default")]
    columns: Vec<String>,
    #[serde(default, deserialize_with = "null_default")]
    rows: Vec<Vec<String>>,
  },
  Mindmap {
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    root: Value,
  },
  Image {
    #[serde(default)]
    title: Option<String>,
    src: String,
    #[serde(default)]
    caption: Option<String>,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulletItem {
  #[serde(default)]
  pub badge: Option<String>,
  pub heading: String,
  #[serde(default, deserialize_with = "null_default")]
  pub body: String,
  #[serde(default)]
  pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeCallout {
  pub line: usize,
  pub text: String,
  #[serde(default)]
  pub tone: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowNode {
  pub id: String,
  pub label: String,
  #[serde(default)]
  pub sublabel: Option<String>,
  #[serde(default)]
  pub tone: Option<String>,
  #[serde(default)]
  pub col: Option<usize>,
  #[serde(default)]
  pub row: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEdge {
  pub from: String,
  pub to: String,
  #[serde(default)]
  pub label: Option<String>,
  #[serde(default, deserialize_with = "null_default")]
  pub dashed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemRegion {
  pub name: String,
  #[serde(default)]
  pub subtitle: Option<String>,
  #[serde(default)]
  pub tone: Option<String>,
  pub slots: Vec<MemSlot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemSlot {
  pub id: String,
  #[serde(default)]
  pub offset: Option<String>,
  pub label: String,
  pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemPointer {
  pub from: String,
  pub to: String,
  #[serde(default)]
  pub label: Option<String>,
  #[serde(default)]
  pub tone: Option<String>,
}

#[derive(Clone, Copy)]
struct Palette {
  bg: &'static str,
  surface: &'static str,
  surface_alt: &'static str,
  code_bg: &'static str,
  border: &'static str,
  text: &'static str,
  text_sub: &'static str,
  text_muted: &'static str,
  accent: &'static str,
  cyan: &'static str,
  green: &'static str,
  amber: &'static str,
  violet: &'static str,
  rose: &'static str,
  dark: bool,
}

impl Palette {
  fn from_theme(theme: Option<&str>) -> Self {
    match theme.unwrap_or("dark") {
      "light" => Self {
        bg: "#F8FAFC",
        surface: "#FFFFFF",
        surface_alt: "#F1F5F9",
        code_bg: "#0F172A",
        border: "#CBD5E1",
        text: "#0F172A",
        text_sub: "#334155",
        text_muted: "#64748B",
        accent: "#EA580C",
        cyan: "#0284C7",
        green: "#059669",
        amber: "#D97706",
        violet: "#7C3AED",
        rose: "#E11D48",
        dark: false,
      },
      _ => Self {
        bg: "#0B1120",
        surface: "#111A2E",
        surface_alt: "#17233D",
        code_bg: "#090E1A",
        border: "#263759",
        text: "#F8FAFC",
        text_sub: "#CBD5E1",
        text_muted: "#8CA0BE",
        accent: "#F97316",
        cyan: "#38BDF8",
        green: "#34D399",
        amber: "#FBBF24",
        violet: "#A78BFA",
        rose: "#FB7185",
        dark: true,
      },
    }
  }

  fn tone_color(&self, tone: Option<&str>, fallback_idx: usize) -> &'static str {
    match tone.unwrap_or("") {
      "rust" | "orange" | "accent" => self.accent,
      "blue" | "cyan" | "sky" => self.cyan,
      "green" | "emerald" | "ok" => self.green,
      "amber" | "yellow" | "warn" => self.amber,
      "violet" | "purple" => self.violet,
      "rose" | "red" | "danger" => self.rose,
      _ => {
        let cycle = [
          self.accent,
          self.cyan,
          self.green,
          self.violet,
          self.amber,
          self.rose,
        ];
        cycle[fallback_idx % cycle.len()]
      }
    }
  }
}

fn xml_escape(raw: &str) -> String {
  let mut out = String::with_capacity(raw.len() + 8);
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

fn is_wide_char(c: char) -> bool {
  let u = c as u32;
  (0x2E80..=0x9FFF).contains(&u)
    || (0xAC00..=0xD7AF).contains(&u)
    || (0xF900..=0xFAFF).contains(&u)
    || (0xFE30..=0xFE6F).contains(&u)
    || (0xFF01..=0xFF60).contains(&u)
    || u >= 0x1F300
}

fn char_px(c: char, font_size: f32, mono: bool) -> f32 {
  if is_wide_char(c) {
    font_size * 1.03
  } else if mono {
    font_size * 0.61
  } else if c.is_ascii_uppercase() {
    font_size * 0.65
  } else if c == ' ' {
    font_size * 0.30
  } else {
    font_size * 0.54
  }
}

fn measure_px(text: &str, font_size: f32, mono: bool) -> f32 {
  text.chars().map(|c| char_px(c, font_size, mono)).sum()
}

/// Wrap CJK and Latin mixed text cleanly within `max_px`, clamping to `max_lines`.
fn wrap_text(text: &str, max_px: f32, font_size: f32, max_lines: usize) -> Vec<String> {
  if max_lines == 0 {
    return Vec::new();
  }
  let mut lines: Vec<String> = Vec::new();
  for paragraph in text.split('\n') {
    let trimmed = paragraph.trim_end();
    if trimmed.is_empty() {
      continue;
    }
    let mut cur = String::new();
    let mut cur_w = 0.0_f32;
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
      let c = chars[i];
      if !is_wide_char(c) && !c.is_whitespace() {
        // Group contiguous ASCII word tokens so English identifiers don't split mid-word.
        let mut word = String::new();
        let mut word_w = 0.0_f32;
        while i < chars.len() && !is_wide_char(chars[i]) && !chars[i].is_whitespace() {
          word.push(chars[i]);
          word_w += char_px(chars[i], font_size, false);
          i += 1;
        }
        if cur_w + word_w <= max_px || cur.is_empty() {
          if word_w > max_px && cur.is_empty() {
            // Very long token: slice by character width.
            for wc in word.chars() {
              let cw = char_px(wc, font_size, false);
              if cur_w + cw > max_px && !cur.is_empty() {
                lines.push(cur.trim_end().to_string());
                cur.clear();
                cur_w = 0.0;
              }
              cur.push(wc);
              cur_w += cw;
            }
          } else {
            cur.push_str(&word);
            cur_w += word_w;
          }
        } else {
          lines.push(cur.trim_end().to_string());
          cur = word;
          cur_w = word_w;
        }
      } else if c.is_whitespace() {
        if !cur.is_empty() && cur_w + char_px(' ', font_size, false) <= max_px {
          cur.push(' ');
          cur_w += char_px(' ', font_size, false);
        }
        i += 1;
      } else {
        let cw = char_px(c, font_size, false);
        let no_leading_punct = matches!(
          c,
          '，' | '。' | '：' | '；' | '！' | '？' | '）' | '》' | '、' | '”'
        );
        if cur_w + cw > max_px && !cur.is_empty() && !no_leading_punct {
          lines.push(cur.trim_end().to_string());
          cur.clear();
          cur_w = 0.0;
        }
        cur.push(c);
        cur_w += cw;
        i += 1;
      }
    }
    if !cur.trim().is_empty() {
      lines.push(cur.trim_end().to_string());
    }
  }
  if lines.len() > max_lines {
    lines.truncate(max_lines);
    if let Some(last) = lines.last_mut() {
      if last.chars().count() > 2 {
        last.pop();
      }
      last.push('…');
    }
  }
  lines
}

fn split_sentences(text: &str) -> Vec<String> {
  let chars: Vec<char> = text.chars().collect();
  let mut out = Vec::new();
  let mut cur = String::new();
  for (i, &c) in chars.iter().enumerate() {
    cur.push(c);
    let cjk = matches!(c, '。' | '！' | '？' | '；');
    let latin = matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace());
    if cjk || latin {
      let s = cur.trim().to_string();
      if !s.is_empty() {
        out.push(s);
      }
      cur.clear();
    }
  }
  let rest = cur.trim();
  if !rest.is_empty() {
    out.push(rest.to_string());
  }
  out
}

fn panel_item_count(panel: &Panel) -> usize {
  match panel {
    Panel::Bullets { items, .. } => items.len().max(1),
    Panel::Code { code, .. } => code.lines().count().max(1),
    Panel::Flowchart { nodes, .. } => nodes.len().max(1),
    Panel::Memory { regions, .. } => regions.len().max(1),
    Panel::Table { rows, .. } => rows.len().max(1),
    Panel::Mindmap { root, .. } => root
      .get("children")
      .and_then(Value::as_array)
      .map_or(1, |c| c.len().max(1)),
    Panel::Image { .. } => 1,
  }
}

/// Partition `code` into `target_steps` contiguous 1-based `(start, end)` line ranges,
/// preferring blank-line paragraph boundaries so each progressive step highlights a coherent block.
fn code_logical_blocks(code: &str, target_steps: usize) -> Vec<(usize, usize)> {
  let lines: Vec<&str> = code.lines().collect();
  let total = lines.len();
  if total == 0 || target_steps == 0 {
    return Vec::new();
  }
  let mut raw_blocks: Vec<(usize, usize)> = Vec::new();
  let mut block_start: Option<usize> = None;
  for (idx, line) in lines.iter().enumerate() {
    let ln = idx + 1;
    if line.trim().is_empty() {
      if let Some(st) = block_start.take() {
        raw_blocks.push((st, ln - 1));
      }
    } else if block_start.is_none() {
      block_start = Some(ln);
    }
  }
  if let Some(st) = block_start {
    raw_blocks.push((st, total));
  }
  if raw_blocks.len() == target_steps {
    return raw_blocks;
  }
  if raw_blocks.len() > target_steps {
    let mut merged = Vec::with_capacity(target_steps);
    for k in 0..target_steps {
      let from_idx = k * raw_blocks.len() / target_steps;
      let to_idx = ((k + 1) * raw_blocks.len() / target_steps)
        .saturating_sub(1)
        .max(from_idx);
      merged.push((raw_blocks[from_idx].0, raw_blocks[to_idx].1));
    }
    return merged;
  }
  (0..target_steps)
    .map(|k| {
      let start = (k * total / target_steps) + 1;
      let end = (((k + 1) * total).div_ceil(target_steps))
        .max(start)
        .min(total);
      (start, end)
    })
    .collect()
}

fn find_code_source(slide: &Slide) -> Option<&str> {
  match &slide.left {
    Panel::Code { code, .. } if !code.trim().is_empty() => Some(code.as_str()),
    _ => match &slide.right {
      Some(Panel::Code { code, .. }) if !code.trim().is_empty() => Some(code.as_str()),
      _ => None,
    },
  }
}

fn auto_focus_ids_for_step(slide: &Slide, step_idx: usize, total_steps: usize) -> Vec<String> {
  let mut ids = Vec::new();
  for panel in [Some(&slide.left), slide.right.as_ref()]
    .into_iter()
    .flatten()
  {
    match panel {
      Panel::Flowchart { nodes, .. } if !nodes.is_empty() => {
        let prev = (nodes.len() * step_idx).div_ceil(total_steps);
        let curr = ((nodes.len() * (step_idx + 1)).div_ceil(total_steps)).max(1);
        let range = if curr > prev {
          prev..curr
        } else {
          curr.saturating_sub(1)..curr
        };
        for n in &nodes[range] {
          ids.push(n.id.clone());
        }
      }
      Panel::Memory { regions, .. } if !regions.is_empty() => {
        let reg_idx = (step_idx * regions.len() / total_steps).min(regions.len() - 1);
        for slot in &regions[reg_idx].slots {
          ids.push(slot.id.clone());
        }
      }
      _ => {}
    }
  }
  ids
}

/// Derive progressive build steps automatically when `slide.steps` is empty,
/// and backfill missing `code_lines` / `focus_ids` when all authored steps omit them.
pub fn effective_steps(slide: &Slide) -> Vec<SlideStep> {
  if !slide.steps.is_empty() {
    let mut authored = slide.steps.clone();
    let n = authored.len();
    if n >= 2 {
      if authored.iter().all(|s| s.code_lines.is_none())
        && let Some(code) = find_code_source(slide)
      {
        let blocks = code_logical_blocks(code, n);
        for (k, st) in authored.iter_mut().enumerate() {
          st.code_lines = blocks.get(k).copied();
        }
      }
      if authored.iter().all(|s| s.focus_ids.is_empty()) {
        for (k, st) in authored.iter_mut().enumerate() {
          st.focus_ids = auto_focus_ids_for_step(slide, k, n);
        }
      }
    }
    return authored;
  }
  let notes = slide
    .notes
    .as_deref()
    .or(slide.takeaway.as_deref())
    .unwrap_or(&slide.title);
  let sentences = split_sentences(notes);
  if sentences.len() <= 1 {
    return vec![SlideStep {
      caption: notes.trim().to_string(),
      spoken: None,
      left_reveal: None,
      right_reveal: None,
      code_lines: None,
      focus_ids: Vec::new(),
      show_takeaway: Some(true),
    }];
  }
  let target_steps = sentences.len().clamp(2, 4);
  let mut grouped = vec![String::new(); target_steps];
  for (idx, s) in sentences.iter().enumerate() {
    let bucket = idx * target_steps / sentences.len();
    if !grouped[bucket].is_empty() {
      grouped[bucket].push(' ');
    }
    grouped[bucket].push_str(s);
  }
  let left_n = panel_item_count(&slide.left);
  let right_n = slide.right.as_ref().map_or(0, panel_item_count);
  let code_blocks = find_code_source(slide)
    .map(|c| code_logical_blocks(c, target_steps))
    .unwrap_or_default();
  grouped
    .into_iter()
    .enumerate()
    .map(|(k, caption)| {
      let step_num = k + 1;
      let is_last = step_num == target_steps;
      let left_rev = ((left_n * step_num).div_ceil(target_steps)).max(1);
      let right_rev = if right_n > 0 {
        Some(((right_n * step_num).div_ceil(target_steps)).max(1))
      } else {
        None
      };
      SlideStep {
        caption,
        spoken: None,
        left_reveal: Some(left_rev),
        right_reveal: right_rev,
        code_lines: code_blocks.get(k).copied(),
        focus_ids: auto_focus_ids_for_step(slide, k, target_steps),
        show_takeaway: Some(is_last),
      }
    })
    .collect()
}

/// Render a Keynote-style 1920×1080 chapter opener / transition card SVG.
pub fn render_section_card_svg(
  deck: &SlideDeck,
  section_idx: usize,
  sections: &[String],
  slide: &Slide,
  progress_ratio: f32,
  pip_safe_zone: bool,
) -> String {
  let p = Palette::from_theme(deck.theme.as_deref());
  let sec_label = slide
    .section
    .as_deref()
    .filter(|s| !s.trim().is_empty())
    .unwrap_or(slide.title.as_str());
  let mut svg = String::with_capacity(6_144);
  let _ = writeln!(
    svg,
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{CANVAS_W:.0}\" height=\"{CANVAS_H:.0}\" viewBox=\"0 0 {CANVAS_W:.0} {CANVAS_H:.0}\" font-family=\"{SANS_FONT}\">\n\
     <defs>\
       <linearGradient id=\"topBar\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\">\
         <stop offset=\"0%\" stop-color=\"{}\"/>\
         <stop offset=\"55%\" stop-color=\"{}\"/>\
         <stop offset=\"100%\" stop-color=\"{}\"/>\
       </linearGradient>\
       <radialGradient id=\"heroGlow\" cx=\"24%\" cy=\"42%\" r=\"65%\">\
         <stop offset=\"0%\" stop-color=\"{}\" stop-opacity=\"0.16\"/>\
         <stop offset=\"55%\" stop-color=\"{}\" stop-opacity=\"0.06\"/>\
         <stop offset=\"100%\" stop-color=\"{}\" stop-opacity=\"0\"/>\
       </radialGradient>\
     </defs>\n\
     <rect width=\"{CANVAS_W:.0}\" height=\"{CANVAS_H:.0}\" fill=\"{}\"/>\n\
     <rect width=\"{CANVAS_W:.0}\" height=\"{CANVAS_H:.0}\" fill=\"url(#heroGlow)\"/>\n\
     <rect width=\"{CANVAS_W:.0}\" height=\"6\" fill=\"url(#topBar)\"/>",
    p.accent, p.cyan, p.violet, p.accent, p.cyan, p.bg, p.bg
  );

  // Subtle architectural watermark number on the right (shifted above PiP safe zone when enabled).
  let wm_y = if pip_safe_zone { 520.0 } else { 640.0 };
  let _ = writeln!(
    svg,
    "<text x=\"1790\" y=\"{wm_y:.0}\" text-anchor=\"end\" font-family=\"{MONO_FONT}\" font-size=\"280\" font-weight=\"800\" fill=\"{}\" fill-opacity=\"0.08\">{:02}</text>",
    p.cyan,
    section_idx + 1
  );

  // Deck header eyebrow.
  let _ = writeln!(
    svg,
    "<text x=\"136\" y=\"210\" font-size=\"20\" font-weight=\"600\" fill=\"{}\">{}</text>\n\
     <rect x=\"136\" y=\"248\" width=\"240\" height=\"46\" rx=\"23\" fill=\"{}\" fill-opacity=\"0.18\" stroke=\"{}\" stroke-width=\"1.6\"/>\n\
     <text x=\"256\" y=\"279\" text-anchor=\"middle\" font-family=\"{MONO_FONT}\" font-size=\"19\" font-weight=\"700\" fill=\"{}\">CHAPTER {:02} / {:02}</text>",
    p.text_muted,
    xml_escape(&deck.title),
    p.accent,
    p.accent,
    p.accent,
    section_idx + 1,
    sections.len().max(1)
  );

  // Big section & chapter title.
  let max_title_w = if pip_safe_zone { 1360.0 } else { 1560.0 };
  let title_lines = wrap_text(&slide.title, max_title_w, 56.0, 2);
  for (li, line) in title_lines.iter().enumerate() {
    let ty = 392.0 + li as f32 * 72.0;
    let _ = writeln!(
      svg,
      "<text x=\"136\" y=\"{ty:.1}\" font-size=\"56\" font-weight=\"800\" fill=\"{}\">{}</text>",
      p.text,
      xml_escape(line)
    );
  }

  let sub_base_y = 392.0 + title_lines.len().max(1) as f32 * 72.0 + 18.0;
  let sub_text = slide
    .subtitle
    .as_deref()
    .or(slide.takeaway.as_deref())
    .unwrap_or(sec_label);
  let sub_lines = wrap_text(sub_text, max_title_w, 26.0, 2);
  for (li, line) in sub_lines.iter().enumerate() {
    let sy = sub_base_y + li as f32 * 38.0;
    let _ = writeln!(
      svg,
      "<text x=\"136\" y=\"{sy:.1}\" font-size=\"26\" fill=\"{}\">{}</text>",
      p.text_sub,
      xml_escape(line)
    );
  }

  // Chapter roadmap pills along the lower stage.
  let mut rx = 136.0_f32;
  let ry = 830.0_f32;
  let right_cap = if pip_safe_zone { 1460.0 } else { 1780.0 };
  for (idx, sec) in sections.iter().take(7).enumerate() {
    let active = idx == section_idx;
    let passed = idx < section_idx;
    let pw = (measure_px(sec, 17.0, false) + 38.0).min(240.0);
    if rx + pw > right_cap {
      break;
    }
    let (fill, stroke, tcol) = if active {
      (p.accent, p.accent, "#FFFFFF")
    } else if passed {
      (p.surface_alt, p.cyan, p.text_sub)
    } else {
      (p.surface, p.border, p.text_muted)
    };
    let clipped = wrap_text(sec, pw - 24.0, 17.0, 1);
    let txt = clipped.first().map_or(sec.as_str(), String::as_str);
    let _ = writeln!(
      svg,
      "<rect x=\"{rx:.1}\" y=\"{ry:.1}\" width=\"{pw:.1}\" height=\"42\" rx=\"21\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.5\"/>\n\
       <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"17\" font-weight=\"{}\" fill=\"{tcol}\">{}</text>",
      rx + pw * 0.5,
      ry + 27.0,
      if active { "700" } else { "500" },
      xml_escape(txt)
    );
    rx += pw + 14.0;
  }

  render_bottom_progress_bar(&mut svg, progress_ratio, &p);
  svg.push_str("</svg>\n");
  svg
}

fn render_bottom_progress_bar(svg: &mut String, ratio: f32, p: &Palette) {
  let clamped = ratio.clamp(0.02, 1.0);
  let fill_w = (CANVAS_W * clamped).round();
  let _ = writeln!(
    svg,
    "<rect x=\"0\" y=\"1074\" width=\"{CANVAS_W:.0}\" height=\"6\" fill=\"{}\" fill-opacity=\"0.45\"/>\n\
     <rect x=\"0\" y=\"1074\" width=\"{fill_w:.1}\" height=\"6\" fill=\"url(#topBar)\"/>",
    p.surface_alt
  );
}

/// Render a single slide (at optional build `step_idx`; `None` renders completed state) to a 1920×1080 SVG.
pub fn render_slide_svg(
  deck: &SlideDeck,
  slide_idx: usize,
  step_idx: Option<usize>,
) -> Result<String, String> {
  render_slide_svg_with_options(deck, slide_idx, step_idx, SlideRenderOptions::default())
}

/// Render a single slide with explicit broadcast layout options (`progress_ratio`, `pip_safe_zone`, etc.).
pub fn render_slide_svg_with_options(
  deck: &SlideDeck,
  slide_idx: usize,
  step_idx: Option<usize>,
  opts: SlideRenderOptions,
) -> Result<String, String> {
  let slide = deck
    .slides
    .get(slide_idx)
    .ok_or_else(|| format!("slide index {slide_idx} out of range"))?;
  let p = Palette::from_theme(deck.theme.as_deref());
  let steps = effective_steps(slide);
  let active_step = step_idx.and_then(|i| steps.get(i));

  let mut svg = String::with_capacity(16_384);
  let _ = writeln!(
    svg,
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{CANVAS_W:.0}\" height=\"{CANVAS_H:.0}\" viewBox=\"0 0 {CANVAS_W:.0} {CANVAS_H:.0}\" font-family=\"{SANS_FONT}\">"
  );

  // Defs: gradients and arrowheads.
  let _ = writeln!(
    svg,
    "<defs>\
      <linearGradient id=\"topBar\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"0\">\
        <stop offset=\"0%\" stop-color=\"{}\"/>\
        <stop offset=\"55%\" stop-color=\"{}\"/>\
        <stop offset=\"100%\" stop-color=\"{}\"/>\
      </linearGradient>\
      <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\">\
        <path d=\"M 0 1.5 L 9 5 L 0 8.5 z\" fill=\"{}\"/>\
      </marker>\
      <marker id=\"arrowAccent\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\">\
        <path d=\"M 0 1.5 L 9 5 L 0 8.5 z\" fill=\"{}\"/>\
      </marker>\
      <marker id=\"arrowGreen\" viewBox=\"0 0 10 10\" refX=\"8.5\" refY=\"5\" markerWidth=\"7\" markerHeight=\"7\" orient=\"auto-start-reverse\">\
        <path d=\"M 0 1.5 L 9 5 L 0 8.5 z\" fill=\"{}\"/>\
      </marker>\
    </defs>",
    p.accent, p.cyan, p.violet, p.cyan, p.accent, p.green
  );

  // Canvas background + top gradient accent bar.
  let _ = writeln!(
    svg,
    "<rect width=\"{CANVAS_W:.0}\" height=\"{CANVAS_H:.0}\" fill=\"{}\"/>\n\
     <rect width=\"{CANVAS_W:.0}\" height=\"5\" fill=\"url(#topBar)\"/>",
    p.bg
  );

  // Top progress navigation bar (y = 18..56).
  render_top_nav(&mut svg, deck, slide_idx, step_idx, steps.len(), &p);

  // Slide title & subtitle (y = 74..168).
  let title_lines = wrap_text(&slide.title, 1680.0, 38.0, 1);
  let title_str = title_lines
    .first()
    .map_or(slide.title.as_str(), String::as_str);
  let _ = writeln!(
    svg,
    "<text x=\"72\" y=\"114\" font-size=\"38\" font-weight=\"700\" fill=\"{}\">{}</text>",
    p.text,
    xml_escape(title_str)
  );
  if let Some(sub) = &slide.subtitle {
    let sub_lines = wrap_text(sub, 1740.0, 21.0, 1);
    if let Some(s) = sub_lines.first() {
      let _ = writeln!(
        svg,
        "<text x=\"72\" y=\"150\" font-size=\"21\" fill=\"{}\">{}</text>",
        p.text_muted,
        xml_escape(s)
      );
    }
  }
  let _ = writeln!(
    svg,
    "<line x1=\"72\" y1=\"170\" x2=\"1848\" y2=\"170\" stroke=\"{}\" stroke-width=\"1.5\"/>",
    p.border
  );

  // Main stage (y = 188..942). When `pip_safe_zone` is active, reserve bottom-right `[1496..1872, 812..1054]`
  // so the presenter PiP window never covers right-column diagrams or the bottom takeaway/caption bar.
  let stage_y = 188.0_f32;
  let has_footer = slide.takeaway.is_some() || active_step.is_some();
  let stage_h: f32 = if has_footer { 754.0 } else { 836.0 };

  if let Some(right_panel) = &slide.right {
    let ratio = slide.split_ratio.unwrap_or(0.50).clamp(0.32, 0.68);
    let gap = 32.0_f32;
    let total_w = 1776.0_f32;
    let left_w = ((total_w - gap) * ratio).round();
    let right_w = total_w - gap - left_w;
    let right_h = if opts.pip_safe_zone {
      (stage_h - 134.0).max(480.0)
    } else {
      stage_h
    };
    render_panel(
      &mut svg,
      &slide.left,
      Rect {
        x: 72.0,
        y: stage_y,
        w: left_w,
        h: stage_h,
      },
      active_step.and_then(|s| s.left_reveal),
      active_step,
      &p,
    );
    render_panel(
      &mut svg,
      right_panel,
      Rect {
        x: 72.0 + left_w + gap,
        y: stage_y,
        w: right_w,
        h: right_h,
      },
      active_step.and_then(|s| s.right_reveal),
      active_step,
      &p,
    );
  } else {
    let single_w = if opts.pip_safe_zone { 1408.0 } else { 1776.0 };
    render_panel(
      &mut svg,
      &slide.left,
      Rect {
        x: 72.0,
        y: stage_y,
        w: single_w,
        h: stage_h,
      },
      active_step.and_then(|s| s.left_reveal),
      active_step,
      &p,
    );
  }

  // Bottom callout / narration spotlight bar (y = 960..1046).
  if has_footer {
    let show_takeaway =
      opts.takeaway_only_bar || active_step.and_then(|s| s.show_takeaway).unwrap_or(true);
    let (badge, banner_text, banner_color) = if show_takeaway && slide.takeaway.is_some() {
      (
        "核心要点",
        slide.takeaway.as_deref().unwrap_or(""),
        p.accent,
      )
    } else if let Some(st) = active_step {
      ("当前讲解", st.caption.as_str(), p.cyan)
    } else {
      (
        "核心要点",
        slide.takeaway.as_deref().unwrap_or(""),
        p.accent,
      )
    };
    let footer_w: f32 = if opts.pip_safe_zone { 1408.0 } else { 1776.0 };
    let text_max_w = (footer_w - 186.0).max(400.0);
    let _ = writeln!(
      svg,
      "<rect x=\"72\" y=\"960\" width=\"{footer_w:.1}\" height=\"86\" rx=\"14\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>\n\
       <rect x=\"90\" y=\"981\" width=\"108\" height=\"44\" rx=\"9\" fill=\"{}\" fill-opacity=\"0.18\" stroke=\"{}\" stroke-width=\"1.2\"/>\n\
       <text x=\"144\" y=\"1010\" text-anchor=\"middle\" font-size=\"18\" font-weight=\"700\" fill=\"{}\">{}</text>",
      p.surface,
      banner_color,
      banner_color,
      banner_color,
      banner_color,
      xml_escape(badge)
    );
    let lines = wrap_text(banner_text, text_max_w, 20.0, 2);
    if lines.len() == 1 {
      let _ = writeln!(
        svg,
        "<text x=\"220\" y=\"1010\" font-size=\"21\" font-weight=\"600\" fill=\"{}\">{}</text>",
        p.text,
        xml_escape(&lines[0])
      );
    } else if lines.len() >= 2 {
      let _ = writeln!(
        svg,
        "<text x=\"220\" y=\"996\" font-size=\"19\" font-weight=\"600\" fill=\"{}\">{}</text>\n\
         <text x=\"220\" y=\"1024\" font-size=\"19\" fill=\"{}\">{}</text>",
        p.text,
        xml_escape(&lines[0]),
        p.text_sub,
        xml_escape(&lines[1])
      );
    }
  }

  let default_ratio = match step_idx {
    Some(st) => ((slide_idx as f32 + (st + 1) as f32 / steps.len().max(1) as f32)
      / deck.slides.len().max(1) as f32)
      .clamp(0.02, 1.0),
    None => ((slide_idx + 1) as f32 / deck.slides.len().max(1) as f32).clamp(0.02, 1.0),
  };
  render_bottom_progress_bar(&mut svg, opts.progress_ratio.unwrap_or(default_ratio), &p);

  svg.push_str("</svg>\n");
  Ok(svg)
}

#[derive(Clone, Copy)]
struct Rect {
  x: f32,
  y: f32,
  w: f32,
  h: f32,
}

fn render_top_nav(
  svg: &mut String,
  deck: &SlideDeck,
  slide_idx: usize,
  step_idx: Option<usize>,
  step_total: usize,
  p: &Palette,
) {
  let mut sections: Vec<String> = Vec::new();
  for (idx, s) in deck.slides.iter().enumerate() {
    let label = s
      .section
      .clone()
      .unwrap_or_else(|| format!("{:02}", idx + 1));
    if !sections.contains(&label) {
      sections.push(label);
    }
  }
  let active_section = deck.slides[slide_idx]
    .section
    .clone()
    .unwrap_or_else(|| format!("{:02}", slide_idx + 1));

  let mut cursor_x = 72.0_f32;
  let deck_badge = wrap_text(&deck.title, 340.0, 16.0, 1);
  if let Some(t) = deck_badge.first() {
    let bw = measure_px(t, 16.0, false) + 28.0;
    let _ = writeln!(
      svg,
      "<rect x=\"{cursor_x:.1}\" y=\"20\" width=\"{bw:.1}\" height=\"34\" rx=\"8\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>\n\
       <text x=\"{:.1}\" y=\"43\" font-size=\"16\" font-weight=\"700\" fill=\"{}\">{}</text>",
      p.surface_alt,
      p.border,
      cursor_x + 14.0,
      p.accent,
      xml_escape(t)
    );
    cursor_x += bw + 16.0;
  }

  for sec in sections.iter().take(8) {
    let is_active = *sec == active_section;
    let pw = (measure_px(sec, 15.0, false) + 26.0).min(210.0);
    if cursor_x + pw > 1580.0 {
      break;
    }
    let (fill, stroke, text_col, weight) = if is_active {
      (p.accent, p.accent, "#FFFFFF", "700")
    } else {
      (p.surface, p.border, p.text_muted, "500")
    };
    let label_short = wrap_text(sec, pw - 20.0, 15.0, 1);
    let label_str = label_short.first().map_or(sec.as_str(), String::as_str);
    let _ = writeln!(
      svg,
      "<rect x=\"{cursor_x:.1}\" y=\"21\" width=\"{pw:.1}\" height=\"32\" rx=\"16\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"1.2\"/>\n\
       <text x=\"{:.1}\" y=\"42\" text-anchor=\"middle\" font-size=\"15\" font-weight=\"{weight}\" fill=\"{text_col}\">{}</text>",
      cursor_x + pw / 2.0,
      xml_escape(label_str)
    );
    cursor_x += pw + 10.0;
  }

  // Right-side slide counter & step dots.
  let counter = match step_idx {
    Some(st) if step_total > 1 => format!(
      "P {:02}/{:02} · Step {}/{}",
      slide_idx + 1,
      deck.slides.len(),
      st + 1,
      step_total
    ),
    _ => format!("P {:02}/{:02}", slide_idx + 1, deck.slides.len()),
  };
  let _ = writeln!(
    svg,
    "<text x=\"1848\" y=\"43\" text-anchor=\"end\" font-family=\"{MONO_FONT}\" font-size=\"16\" font-weight=\"600\" fill=\"{}\">{}</text>",
    p.text_muted,
    xml_escape(&counter)
  );
}

fn render_panel_shell(svg: &mut String, r: Rect, title: Option<&str>, p: &Palette) -> Rect {
  let _ = writeln!(
    svg,
    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.5\"/>",
    r.x, r.y, r.w, r.h, p.surface, p.border
  );
  if let Some(t) = title.filter(|s| !s.trim().is_empty()) {
    let lines = wrap_text(t, r.w - 48.0, 20.0, 1);
    let label = lines.first().map_or(t, String::as_str);
    let _ = writeln!(
      svg,
      "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"6\" height=\"22\" rx=\"3\" fill=\"{}\"/>\n\
       <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"20\" font-weight=\"700\" fill=\"{}\">{}</text>\n\
       <line x1=\"{:.1}\" y1=\"{:.1}\" x2=\"{:.1}\" y2=\"{:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
      r.x + 22.0,
      r.y + 18.0,
      p.cyan,
      r.x + 38.0,
      r.y + 36.0,
      p.text,
      xml_escape(label),
      r.x + 20.0,
      r.y + 52.0,
      r.x + r.w - 20.0,
      r.y + 52.0,
      p.border
    );
    Rect {
      x: r.x + 22.0,
      y: r.y + 66.0,
      w: r.w - 44.0,
      h: r.h - 84.0,
    }
  } else {
    Rect {
      x: r.x + 22.0,
      y: r.y + 22.0,
      w: r.w - 44.0,
      h: r.h - 44.0,
    }
  }
}

fn render_panel(
  svg: &mut String,
  panel: &Panel,
  r: Rect,
  reveal: Option<usize>,
  step: Option<&SlideStep>,
  p: &Palette,
) {
  match panel {
    Panel::Bullets { title, grid, items } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_bullets(svg, inner, *grid, items, reveal, p);
    }
    Panel::Code {
      title,
      language,
      code,
      callouts,
    } => {
      render_code(
        svg,
        r,
        title.as_deref(),
        language.as_deref(),
        code,
        callouts,
        step.and_then(|s| s.code_lines),
        p,
      );
    }
    Panel::Flowchart {
      title,
      direction,
      nodes,
      edges,
    } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_flowchart(
        svg,
        inner,
        direction.as_deref().unwrap_or("TB"),
        nodes,
        edges,
        reveal,
        step.map_or(&[], |s| s.focus_ids.as_slice()),
        p,
      );
    }
    Panel::Memory {
      title,
      regions,
      pointers,
    } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_memory(
        svg,
        inner,
        regions,
        pointers,
        reveal,
        step.map_or(&[], |s| s.focus_ids.as_slice()),
        p,
      );
    }
    Panel::Table {
      title,
      highlight_col,
      columns,
      rows,
    } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_table(svg, inner, *highlight_col, columns, rows, reveal, p);
    }
    Panel::Mindmap { title, root } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_mindmap_panel(svg, inner, root, reveal, p);
    }
    Panel::Image {
      title,
      src,
      caption,
    } => {
      let inner = render_panel_shell(svg, r, title.as_deref(), p);
      render_image_panel(svg, inner, src, caption.as_deref(), p);
    }
  }
}

fn resolve_artifact_image_data_uri(src: &str) -> Option<String> {
  let trimmed = src.trim();
  if trimmed.starts_with("data:image/") {
    return Some(trimmed.to_string());
  }
  let root = yanshi_core::run_context::current_artifact_root()?;
  let canonical_root = root.canonicalize().ok()?;
  let canonical_file = Path::new(trimmed).canonicalize().ok()?;
  if !canonical_file.starts_with(&canonical_root) {
    return None;
  }
  let bytes = std::fs::read(&canonical_file).ok()?;
  let ext = canonical_file
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or("png")
    .to_ascii_lowercase();
  let ext_mime = match ext.as_str() {
    "jpg" | "jpeg" => "image/jpeg",
    "webp" => "image/webp",
    "gif" => "image/gif",
    "svg" => "image/svg+xml",
    _ => "image/png",
  };
  let mime = crate::common::artifacts::sniff_image_mime(&bytes).unwrap_or(ext_mime);
  use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
  Some(format!("data:{mime};base64,{}", B64.encode(&bytes)))
}

fn render_image_panel(svg: &mut String, r: Rect, src: &str, caption: Option<&str>, p: &Palette) {
  let caption_text = caption.map(str::trim).filter(|s| !s.is_empty());
  let caption_h = if caption_text.is_some() { 38.0 } else { 0.0 };
  let img_h = (r.h - caption_h).max(80.0);
  let clip_id = format!("img-clip-{}-{}", r.x as i32, r.y as i32);
  let _ = writeln!(
    svg,
    "<defs><clipPath id=\"{clip_id}\"><rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\"/></clipPath></defs>",
    r.x, r.y, r.w, img_h
  );
  let _ = writeln!(
    svg,
    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.2\"/>",
    r.x, r.y, r.w, img_h, p.surface_alt, p.border
  );
  if let Some(data_uri) = resolve_artifact_image_data_uri(src) {
    let pad = 6.0;
    let _ = writeln!(
      svg,
      "<image x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" preserveAspectRatio=\"xMidYMid meet\" clip-path=\"url(#{clip_id})\" href=\"{}\"/>",
      r.x + pad,
      r.y + pad,
      (r.w - pad * 2.0).max(20.0),
      (img_h - pad * 2.0).max(20.0),
      xml_escape(&data_uri)
    );
  } else {
    let _ = writeln!(
      svg,
      "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"{SANS_FONT}\" font-size=\"18\" fill=\"{}\">插图资源已就绪</text>",
      r.x + r.w * 0.5,
      r.y + img_h * 0.5,
      p.text_muted
    );
  }
  if let Some(cap) = caption_text {
    let _ = writeln!(
      svg,
      "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"{SANS_FONT}\" font-size=\"15\" fill=\"{}\">{}</text>",
      r.x + r.w * 0.5,
      r.y + img_h + 26.0,
      p.text_sub,
      xml_escape(cap)
    );
  }
}

// ── Panel 1: Bullets / 2×2 Cards ────────────────────────────────────────

fn render_bullets(
  svg: &mut String,
  r: Rect,
  grid: bool,
  items: &[BulletItem],
  reveal: Option<usize>,
  p: &Palette,
) {
  if items.is_empty() {
    return;
  }
  let visible = reveal.unwrap_or(items.len());
  if grid && items.len() >= 2 {
    let cols = 2_usize;
    let rows = items.len().div_ceil(cols);
    let gap = 16.0_f32;
    let cw = (r.w - gap) / 2.0;
    let ch = ((r.h - gap * (rows.saturating_sub(1) as f32)) / rows as f32).max(90.0);
    for (idx, item) in items.iter().enumerate() {
      let col = idx % cols;
      let row = idx / cols;
      let cx = r.x + col as f32 * (cw + gap);
      let cy = r.y + row as f32 * (ch + gap);
      draw_bullet_card(
        svg,
        Rect {
          x: cx,
          y: cy,
          w: cw,
          h: ch,
        },
        idx,
        item,
        idx < visible,
        reveal.is_some() && idx + 1 == visible,
        p,
      );
    }
  } else {
    let n = items.len();
    let gap = 14.0_f32;
    let ch = ((r.h - gap * (n.saturating_sub(1) as f32)) / n as f32).clamp(76.0, 175.0);
    for (idx, item) in items.iter().enumerate() {
      let cy = r.y + idx as f32 * (ch + gap);
      if cy + ch > r.y + r.h + 4.0 {
        break;
      }
      draw_bullet_card(
        svg,
        Rect {
          x: r.x,
          y: cy,
          w: r.w,
          h: ch,
        },
        idx,
        item,
        idx < visible,
        reveal.is_some() && idx + 1 == visible,
        p,
      );
    }
  }
}

fn draw_bullet_card(
  svg: &mut String,
  r: Rect,
  idx: usize,
  item: &BulletItem,
  revealed: bool,
  spotlight: bool,
  p: &Palette,
) {
  let color = p.tone_color(item.tone.as_deref(), idx);
  let opacity = if !revealed {
    0.28
  } else if spotlight {
    1.0
  } else {
    0.88
  };
  let border_col = if spotlight { color } else { p.border };
  let border_w = if spotlight { 2.6 } else { 1.2 };
  let _ = writeln!(svg, "<g opacity=\"{opacity:.2}\">");
  if spotlight {
    let _ = writeln!(
      svg,
      "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"15\" fill=\"none\" stroke=\"{color}\" stroke-opacity=\"0.35\" stroke-width=\"4.5\"/>",
      r.x - 3.0,
      r.y - 3.0,
      r.w + 6.0,
      r.h + 6.0
    );
  }
  let _ = writeln!(
    svg,
    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"{}\" stroke=\"{border_col}\" stroke-width=\"{border_w}\"/>\n\
     <rect x=\"{:.1}\" y=\"{:.1}\" width=\"5\" height=\"{:.1}\" rx=\"2.5\" fill=\"{color}\"/>",
    r.x,
    r.y,
    r.w,
    r.h,
    p.surface_alt,
    r.x + 12.0,
    r.y + 14.0,
    (r.h - 28.0).max(20.0)
  );

  let mut title_x = r.x + 28.0;
  if let Some(badge) = item.badge.as_deref().filter(|b| !b.is_empty()) {
    let bw = (measure_px(badge, 15.0, true) + 20.0).min(170.0);
    let _ = writeln!(
      svg,
      "<rect x=\"{title_x:.1}\" y=\"{:.1}\" width=\"{bw:.1}\" height=\"28\" rx=\"7\" fill=\"{color}\" fill-opacity=\"0.20\" stroke=\"{color}\" stroke-width=\"1\"/>\n\
       <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"{MONO_FONT}\" font-size=\"15\" font-weight=\"700\" fill=\"{color}\">{}</text>",
      r.y + 14.0,
      title_x + bw / 2.0,
      r.y + 33.0,
      xml_escape(badge)
    );
    title_x += bw + 12.0;
  }

  let head_lines = wrap_text(
    &item.heading,
    (r.x + r.w - title_x - 16.0).max(80.0),
    21.0,
    1,
  );
  if let Some(hl) = head_lines.first() {
    let _ = writeln!(
      svg,
      "<text x=\"{title_x:.1}\" y=\"{:.1}\" font-size=\"21\" font-weight=\"700\" fill=\"{}\">{}</text>",
      r.y + 35.0,
      p.text,
      xml_escape(hl)
    );
  }

  if !item.body.trim().is_empty() && r.h >= 78.0 {
    let max_body_lines = (((r.h - 54.0) / 25.0).floor() as usize).clamp(1, 4);
    let body_lines = wrap_text(&item.body, r.w - 44.0, 17.5, max_body_lines);
    for (li, line) in body_lines.iter().enumerate() {
      let ly = r.y + 66.0 + li as f32 * 25.0;
      let _ = writeln!(
        svg,
        "<text x=\"{:.1}\" y=\"{ly:.1}\" font-size=\"17.5\" fill=\"{}\">{}</text>",
        r.x + 28.0,
        p.text_sub,
        xml_escape(line)
      );
    }
  }
  svg.push_str("</g>\n");
}

// ── Panel 2: Syntax-Highlighted Code Editor Window ──────────────────────

#[allow(clippy::too_many_arguments)]
fn render_code(
  svg: &mut String,
  r: Rect,
  title: Option<&str>,
  language: Option<&str>,
  code: &str,
  callouts: &[CodeCallout],
  active_lines: Option<(usize, usize)>,
  p: &Palette,
) {
  let _ = writeln!(
    svg,
    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"16\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.6\"/>\n\
     <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"46\" rx=\"16\" fill=\"#131C31\"/>\n\
     <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"16\" fill=\"#131C31\"/>\n\
     <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6.5\" fill=\"#FB7185\"/>\n\
     <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6.5\" fill=\"#FBBF24\"/>\n\
     <circle cx=\"{:.1}\" cy=\"{:.1}\" r=\"6.5\" fill=\"#34D399\"/>",
    r.x,
    r.y,
    r.w,
    r.h,
    p.code_bg,
    p.border,
    r.x,
    r.y,
    r.w,
    r.x,
    r.y + 30.0,
    r.w,
    r.x + 26.0,
    r.y + 23.0,
    r.x + 48.0,
    r.y + 23.0,
    r.x + 70.0,
    r.y + 23.0,
  );

  let tab = title
    .filter(|t| !t.is_empty())
    .unwrap_or_else(|| language.unwrap_or("rust"));
  let _ = writeln!(
    svg,
    "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{MONO_FONT}\" font-size=\"16\" font-weight=\"600\" fill=\"#CBD5E1\">{}</text>",
    r.x + 96.0,
    r.y + 29.0,
    xml_escape(tab)
  );
  if let Some(lang) = language.filter(|l| !l.is_empty()) {
    let _ = writeln!(
      svg,
      "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"{MONO_FONT}\" font-size=\"14\" font-weight=\"700\" fill=\"{}\">{}</text>",
      r.x + r.w - 22.0,
      r.y + 28.0,
      p.accent,
      xml_escape(&lang.to_uppercase())
    );
  }

  let raw_lines: Vec<&str> = code.lines().collect();
  let count = raw_lines.len().max(1);
  let avail_h = (r.h - 68.0).max(120.0);
  let line_h = (avail_h / count as f32).clamp(21.0, 31.0);
  let font_sz = (line_h * 0.66).clamp(14.0, 19.5);
  let max_visible_lines = (avail_h / line_h).floor() as usize;
  let gutter_x = r.x + 48.0;
  let code_x = r.x + 64.0;
  let max_chars_w = r.w - 84.0;

  for (idx, &raw) in raw_lines.iter().take(max_visible_lines).enumerate() {
    let line_no = idx + 1;
    let top_y = r.y + 56.0 + idx as f32 * line_h;
    let baseline_y = top_y + line_h * 0.74;
    let in_focus = active_lines.is_none_or(|(a, b)| line_no >= a && line_no <= b);
    let spotlight_line = active_lines.is_some_and(|(a, b)| line_no >= a && line_no <= b);
    let row_opacity = if in_focus { 1.0 } else { 0.48 };

    if spotlight_line {
      let _ = writeln!(
        svg,
        "<rect x=\"{:.1}\" y=\"{top_y:.1}\" width=\"{:.1}\" height=\"{line_h:.1}\" fill=\"{}\" fill-opacity=\"0.17\"/>\n\
         <rect x=\"{:.1}\" y=\"{top_y:.1}\" width=\"5\" height=\"{line_h:.1}\" fill=\"{}\"/>",
        r.x + 4.0,
        r.w - 8.0,
        p.accent,
        r.x + 4.0,
        p.accent
      );
    }

    let active_callout = callouts
      .iter()
      .find(|c| c.line == line_no)
      .filter(|_| in_focus);
    let callout_layout = active_callout.map(|callout| {
      let badge_col = p.tone_color(callout.tone.as_deref(), 1);
      let max_badge_w = (r.w * 0.34).clamp(140.0, 280.0);
      let cw = (measure_px(&callout.text, 13.5, false) + 22.0).min(max_badge_w);
      let bx = r.x + r.w - cw - 16.0;
      (callout, badge_col, cw, bx)
    });

    // Reserve right gutter when an inline callout badge sits on this line so badge never overlaps code.
    let line_max_w = match &callout_layout {
      Some((_, _, cw, _)) => (max_chars_w - cw - 22.0).max(140.0),
      None => max_chars_w,
    };

    let mut clipped = String::new();
    let mut w_acc = 0.0_f32;
    for ch in raw.chars() {
      let cw = char_px(ch, font_sz, true);
      if w_acc + cw > line_max_w {
        clipped.push('…');
        w_acc += char_px('…', font_sz, true);
        break;
      }
      clipped.push(ch);
      w_acc += cw;
    }

    let line_num_col = if spotlight_line { p.accent } else { "#64748B" };
    let line_num_weight = if spotlight_line { "700" } else { "400" };
    let _ = writeln!(
      svg,
      "<g opacity=\"{row_opacity:.2}\">\n\
       <text x=\"{gutter_x:.1}\" y=\"{baseline_y:.1}\" text-anchor=\"end\" font-family=\"{MONO_FONT}\" font-size=\"{:.1}\" font-weight=\"{line_num_weight}\" fill=\"{line_num_col}\">{line_no}</text>\n\
       <text x=\"{code_x:.1}\" y=\"{baseline_y:.1}\" xml:space=\"preserve\" font-family=\"{MONO_FONT}\" font-size=\"{font_sz:.1}\">{}</text>",
      (font_sz - 2.0).max(12.0),
      highlight_code_spans(&clipped)
    );

    if let Some((callout, badge_col, cw, bx)) = callout_layout {
      let label = wrap_text(&callout.text, cw - 16.0, 13.5, 1);
      if let Some(txt) = label.first() {
        let leader_x1 = code_x + w_acc + 8.0;
        let leader_x2 = bx - 6.0;
        let mid_y = top_y + line_h * 0.52;
        if leader_x2 - leader_x1 >= 14.0 {
          let _ = writeln!(
            svg,
            "<line x1=\"{leader_x1:.1}\" y1=\"{mid_y:.1}\" x2=\"{leader_x2:.1}\" y2=\"{mid_y:.1}\" stroke=\"{badge_col}\" stroke-opacity=\"0.55\" stroke-width=\"1.2\" stroke-dasharray=\"4 3\"/>"
          );
        }
        let _ = writeln!(
          svg,
          "<rect x=\"{bx:.1}\" y=\"{:.1}\" width=\"{cw:.1}\" height=\"{:.1}\" rx=\"6\" fill=\"#0F172A\" stroke=\"{badge_col}\" stroke-width=\"1.3\"/>\n\
           <text x=\"{:.1}\" y=\"{baseline_y:.1}\" text-anchor=\"middle\" font-family=\"{SANS_FONT}\" font-size=\"13.5\" font-weight=\"600\" fill=\"{badge_col}\">{}</text>",
          top_y + 2.0,
          (line_h - 4.0).max(18.0),
          bx + cw / 2.0,
          xml_escape(txt)
        );
      }
    }
    svg.push_str("</g>\n");
  }
}

fn find_line_comment_start(line: &str) -> Option<usize> {
  let bytes = line.as_bytes();
  let mut in_str = false;
  let mut i = 0;
  while i < bytes.len() {
    let b = bytes[i];
    if b == b'"' && (i == 0 || bytes[i - 1] != b'\\') {
      in_str = !in_str;
      i += 1;
      continue;
    }
    if !in_str {
      if b == b'/' && bytes.get(i + 1) == Some(&b'/') {
        return Some(i);
      }
      if b == b'#' && bytes.get(i + 1) != Some(&b'[') && bytes.get(i + 1) != Some(&b'!') {
        return Some(i);
      }
      if b == b'-'
        && bytes.get(i + 1) == Some(&b'-')
        && (i == 0 || bytes[i - 1].is_ascii_whitespace())
      {
        return Some(i);
      }
    }
    i += 1;
  }
  None
}

fn highlight_code_spans(line: &str) -> String {
  const KEYWORDS: &[&str] = &[
    "pub",
    "fn",
    "trait",
    "impl",
    "for",
    "where",
    "struct",
    "enum",
    "type",
    "let",
    "mut",
    "dyn",
    "const",
    "static",
    "async",
    "await",
    "return",
    "self",
    "Self",
    "match",
    "if",
    "else",
    "use",
    "mod",
    "crate",
    "super",
    "unsafe",
    "extern",
    "void",
    "class",
    "interface",
    "extends",
    "implements",
    "virtual",
    "override",
    "typedef",
    "int",
    "char",
    "size_t",
    "def",
    "from",
    "import",
    "as",
    "with",
    "yield",
    "lambda",
    "func",
    "package",
    "go",
    "defer",
    "chan",
    "select",
    "var",
    "export",
    "function",
    "private",
    "protected",
    "public",
    "readonly",
    "new",
    "try",
    "catch",
    "throw",
    "SELECT",
    "FROM",
    "WHERE",
    "JOIN",
    "INSERT",
    "UPDATE",
    "CREATE",
    "TABLE",
    "INDEX",
  ];
  let (code_part, comment_part) = match find_line_comment_start(line) {
    Some(idx) => (&line[..idx], Some(&line[idx..])),
    None => (line, None),
  };

  let mut out = String::new();
  let chars: Vec<char> = code_part.chars().collect();
  let mut i = 0;
  while i < chars.len() {
    let c = chars[i];
    if c == '#' && chars.get(i + 1) == Some(&'[') {
      let start = i;
      while i < chars.len() && chars[i] != ']' {
        i += 1;
      }
      if i < chars.len() {
        i += 1;
      }
      let attr: String = chars[start..i].iter().collect();
      let _ = write!(out, "<tspan fill=\"#D2A8FF\">{}</tspan>", xml_escape(&attr));
    } else if c == '"' {
      let start = i;
      i += 1;
      while i < chars.len() {
        if chars[i] == '"' && chars[i - 1] != '\\' {
          i += 1;
          break;
        }
        i += 1;
      }
      let lit: String = chars[start..i].iter().collect();
      let _ = write!(out, "<tspan fill=\"#A5D6FF\">{}</tspan>", xml_escape(&lit));
    } else if c.is_ascii_alphabetic() || c == '_' {
      let start = i;
      while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
        i += 1;
      }
      let ident: String = chars[start..i].iter().collect();
      let color = if KEYWORDS.contains(&ident.as_str()) {
        "#FF7B72"
      } else if ident.chars().next().is_some_and(|f| f.is_ascii_uppercase()) {
        "#FFA657"
      } else if chars.get(i) == Some(&'(') || chars.get(i) == Some(&'!') {
        "#79C0FF"
      } else {
        "#E2E8F0"
      };
      let _ = write!(
        out,
        "<tspan fill=\"{color}\">{}</tspan>",
        xml_escape(&ident)
      );
    } else if c.is_ascii_digit() {
      let start = i;
      while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
        i += 1;
      }
      let num: String = chars[start..i].iter().collect();
      let _ = write!(out, "<tspan fill=\"#79C0FF\">{}</tspan>", xml_escape(&num));
    } else {
      let start = i;
      while i < chars.len()
        && !chars[i].is_ascii_alphanumeric()
        && chars[i] != '_'
        && chars[i] != '"'
        && !(chars[i] == '#' && chars.get(i + 1) == Some(&'['))
      {
        i += 1;
      }
      let sym: String = chars[start..i].iter().collect();
      let _ = write!(out, "<tspan fill=\"#94A3B8\">{}</tspan>", xml_escape(&sym));
    }
  }
  if let Some(cmt) = comment_part {
    let _ = write!(out, "<tspan fill=\"#6E7681\">{}</tspan>", xml_escape(cmt));
  }
  out
}

// ── Panel 3: Architecture / Flowchart Diagram ───────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_flowchart(
  svg: &mut String,
  r: Rect,
  direction: &str,
  nodes: &[FlowNode],
  edges: &[FlowEdge],
  reveal: Option<usize>,
  focus_ids: &[String],
  p: &Palette,
) {
  if nodes.is_empty() {
    return;
  }
  let lr = direction.eq_ignore_ascii_case("LR");
  let any_explicit = nodes.iter().any(|n| n.col.is_some() || n.row.is_some());

  let mut coords: Vec<(usize, usize)> = Vec::with_capacity(nodes.len());
  if any_explicit {
    for (i, n) in nodes.iter().enumerate() {
      coords.push((n.col.unwrap_or(0), n.row.unwrap_or(i)));
    }
  } else if lr {
    let per_row = nodes.len().clamp(1, 4);
    for i in 0..nodes.len() {
      coords.push((i % per_row, i / per_row));
    }
  } else {
    let cols = if nodes.len() > 5 { 2 } else { 1 };
    for i in 0..nodes.len() {
      coords.push((i % cols, i / cols));
    }
  }

  let max_col = coords.iter().map(|(c, _)| *c).max().unwrap_or(0) + 1;
  let max_row = coords.iter().map(|(_, rw)| *rw).max().unwrap_or(0) + 1;
  let gap_x = if max_col > 1 { 46.0 } else { 0.0 };
  let gap_y = if max_row > 1 { 38.0 } else { 0.0 };
  let cell_w = ((r.w - gap_x * (max_col.saturating_sub(1) as f32)) / max_col as f32).min(460.0);
  let cell_h = ((r.h - gap_y * (max_row.saturating_sub(1) as f32)) / max_row as f32).min(132.0);
  let used_w = cell_w * max_col as f32 + gap_x * (max_col.saturating_sub(1) as f32);
  let used_h = cell_h * max_row as f32 + gap_y * (max_row.saturating_sub(1) as f32);
  let start_x = r.x + ((r.w - used_w) / 2.0).max(0.0);
  let start_y = r.y + ((r.h - used_h) / 2.0).max(0.0);

  let mut boxes: HashMap<&str, Rect> = HashMap::new();
  for (n, &(c, rw)) in nodes.iter().zip(&coords) {
    boxes.insert(
      n.id.as_str(),
      Rect {
        x: start_x + c as f32 * (cell_w + gap_x),
        y: start_y + rw as f32 * (cell_h + gap_y),
        w: cell_w,
        h: cell_h,
      },
    );
  }

  let visible_n = reveal.unwrap_or(nodes.len());
  let visible_ids: std::collections::HashSet<&str> = nodes
    .iter()
    .take(visible_n)
    .map(|n| n.id.as_str())
    .collect();

  // Pass 1: draw edge curves beneath boxes so arrowheads land cleanly at box borders.
  let mut pending_labels: Vec<(f32, f32, f32, String)> = Vec::new();
  for edge in edges {
    let (Some(&a), Some(&b)) = (boxes.get(edge.from.as_str()), boxes.get(edge.to.as_str())) else {
      continue;
    };
    let active = visible_ids.contains(edge.from.as_str()) && visible_ids.contains(edge.to.as_str());
    let focused =
      focus_ids.is_empty() || focus_ids.iter().any(|f| f == &edge.from || f == &edge.to);
    let opacity = if !active {
      0.26
    } else if focused {
      0.96
    } else {
      0.58
    };
    let (path_d, label_pt) = flowchart_edge_geom(a, b);
    let dash = if edge.dashed {
      " stroke-dasharray=\"7 5\""
    } else {
      ""
    };
    let _ = writeln!(
      svg,
      "<path d=\"{path_d}\" fill=\"none\" stroke=\"{}\" stroke-width=\"2.2\" opacity=\"{opacity:.2}\"{dash} marker-end=\"url(#arrow)\"/>",
      p.cyan
    );
    if let Some(lbl) = edge.label.as_deref().filter(|s| !s.is_empty()) {
      pending_labels.push((label_pt.0, label_pt.1, opacity, lbl.to_string()));
    }
  }

  // Pass 2: draw node boxes.
  for (idx, n) in nodes.iter().enumerate() {
    let Some(&nb) = boxes.get(n.id.as_str()) else {
      continue;
    };
    let color = p.tone_color(n.tone.as_deref(), idx);
    let revealed = idx < visible_n;
    let is_focus = (!focus_ids.is_empty() && focus_ids.iter().any(|f| f == &n.id))
      || (focus_ids.is_empty() && reveal.is_some() && idx + 1 == visible_n);
    let opacity = if !revealed {
      0.32
    } else if !focus_ids.is_empty() && !is_focus {
      0.74
    } else {
      1.0
    };
    let stroke_w = if is_focus { 2.8 } else { 1.6 };
    let _ = writeln!(svg, "<g opacity=\"{opacity:.2}\">");
    if is_focus && revealed {
      let _ = writeln!(
        svg,
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"15\" fill=\"none\" stroke=\"{color}\" stroke-opacity=\"0.38\" stroke-width=\"5\"/>",
        nb.x - 3.5,
        nb.y - 3.5,
        nb.w + 7.0,
        nb.h + 7.0
      );
    }
    let _ = writeln!(
      svg,
      "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"{}\" stroke=\"{color}\" stroke-width=\"{stroke_w}\"/>\n\
       <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"5\" rx=\"2.5\" fill=\"{color}\"/>",
      nb.x,
      nb.y,
      nb.w,
      nb.h,
      p.surface_alt,
      nb.x + 10.0,
      nb.y + 6.0,
      (nb.w - 20.0).max(12.0),
    );
    let lbl_lines = wrap_text(&n.label, nb.w - 24.0, 18.5, 2);
    let has_sub = n.sublabel.as_deref().is_some_and(|s| !s.is_empty());
    let base_y = if has_sub {
      nb.y + nb.h * 0.42
    } else {
      nb.y + nb.h * 0.56
    };
    for (li, line) in lbl_lines.iter().enumerate() {
      let y = base_y + (li as f32 - (lbl_lines.len().saturating_sub(1) as f32) * 0.5) * 22.0;
      let _ = writeln!(
        svg,
        "<text x=\"{:.1}\" y=\"{y:.1}\" text-anchor=\"middle\" font-size=\"18.5\" font-weight=\"700\" fill=\"{}\">{}</text>",
        nb.x + nb.w / 2.0,
        p.text,
        xml_escape(line)
      );
    }
    if let Some(sub) = n.sublabel.as_deref().filter(|s| !s.is_empty()) {
      let sub_lines = wrap_text(sub, nb.w - 24.0, 14.5, 1);
      if let Some(sl) = sub_lines.first() {
        let _ = writeln!(
          svg,
          "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-family=\"{MONO_FONT}\" font-size=\"14.5\" fill=\"{}\">{}</text>",
          nb.x + nb.w / 2.0,
          nb.y + nb.h * 0.78,
          p.text_muted,
          xml_escape(sl)
        );
      }
    }
    svg.push_str("</g>\n");
  }

  // Pass 3: de-overlap edge label pills and clamp inside panel bounds so labels never collide.
  let mut placed_pills: Vec<(f32, f32, f32)> = Vec::with_capacity(pending_labels.len());
  for (raw_lx, raw_ly, opacity, lbl) in pending_labels {
    let lw = (measure_px(&lbl, 13.5, false) + 16.0).min(180.0);
    let mut lx = raw_lx.clamp(
      r.x + lw * 0.5 + 4.0,
      (r.x + r.w - lw * 0.5 - 4.0).max(r.x + lw * 0.5 + 4.0),
    );
    let mut ly = raw_ly.clamp(r.y + 13.0, (r.y + r.h - 13.0).max(r.y + 13.0));
    for _ in 0..4 {
      let mut collided = false;
      for &(px, py, pw) in &placed_pills {
        if (lx - px).abs() < (lw + pw) * 0.52 && (ly - py).abs() < 25.0 {
          if ly + 26.0 <= r.y + r.h - 13.0 {
            ly += 26.0;
          } else {
            ly = (ly - 26.0).max(r.y + 13.0);
            lx = (lx + 18.0).min((r.x + r.w - lw * 0.5 - 4.0).max(lx));
          }
          collided = true;
        }
      }
      if !collided {
        break;
      }
    }
    placed_pills.push((lx, ly, lw));
    let clipped = wrap_text(&lbl, lw - 10.0, 13.5, 1);
    if let Some(txt) = clipped.first() {
      let _ = writeln!(
        svg,
        "<g opacity=\"{opacity:.2}\">\
         <rect x=\"{:.1}\" y=\"{:.1}\" width=\"{lw:.1}\" height=\"22\" rx=\"6\" fill=\"{}\" stroke=\"{}\" stroke-width=\"1.1\"/>\
         <text x=\"{lx:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13.5\" font-weight=\"600\" fill=\"{}\">{}</text>\
         </g>",
        lx - lw / 2.0,
        ly - 11.0,
        p.bg,
        p.cyan,
        ly + 4.5,
        p.text_sub,
        xml_escape(txt)
      );
    }
  }
}

fn flowchart_edge_geom(a: Rect, b: Rect) -> (String, (f32, f32)) {
  let acx = a.x + a.w / 2.0;
  let acy = a.y + a.h / 2.0;
  let bcx = b.x + b.w / 2.0;
  let bcy = b.y + b.h / 2.0;
  let dx = bcx - acx;
  let dy = bcy - acy;

  // Diagonal downward transition across columns: route through open vertical corridor.
  if dy > a.h * 0.45 && dx.abs() > a.w * 0.45 {
    if dx > 0.0 {
      let x1 = a.x + a.w * 0.78;
      let y1 = a.y + a.h;
      let x2 = bcx;
      let y2 = b.y - 2.0;
      let cy1 = y1 + (y2 - y1) * 0.55;
      let cy2 = y2 - (y2 - y1) * 0.45;
      let path = format!("M {x1:.1} {y1:.1} C {x1:.1} {cy1:.1}, {x2:.1} {cy2:.1}, {x2:.1} {y2:.1}");
      let lx = x1 * 0.35 + x2 * 0.65;
      let ly = (y1 + y2) * 0.5 - 8.0;
      return (path, (lx, ly));
    }
    let x1 = acx;
    let y1 = a.y + a.h;
    let x2 = b.x + b.w + 2.0;
    let y2 = bcy;
    let path = format!(
      "M {x1:.1} {y1:.1} C {x1:.1} {y2:.1}, {:.1} {y2:.1}, {x2:.1} {y2:.1}",
      (x1 + x2) * 0.5
    );
    let lx = x1 * 0.65 + x2 * 0.35;
    let ly = y1 * 0.55 + y2 * 0.45;
    return (path, (lx, ly));
  }

  let ((x1, y1), (x2, y2)) = box_anchor_pair(a, b);
  let mid_x = (x1 + x2) / 2.0;
  let mid_y = (y1 + y2) / 2.0;
  if (x2 - x1).abs() > (y2 - y1).abs() {
    let path =
      format!("M {x1:.1} {y1:.1} C {mid_x:.1} {y1:.1}, {mid_x:.1} {y2:.1}, {x2:.1} {y2:.1}");
    let ly = if (x2 - x1).abs() < 92.0 && (y2 - y1).abs() < 24.0 {
      mid_y - 24.0
    } else {
      mid_y
    };
    (path, (mid_x, ly))
  } else {
    let path =
      format!("M {x1:.1} {y1:.1} C {x1:.1} {mid_y:.1}, {x2:.1} {mid_y:.1}, {x2:.1} {y2:.1}");
    (path, (mid_x, mid_y))
  }
}

fn box_anchor_pair(a: Rect, b: Rect) -> ((f32, f32), (f32, f32)) {
  let acx = a.x + a.w / 2.0;
  let acy = a.y + a.h / 2.0;
  let bcx = b.x + b.w / 2.0;
  let bcy = b.y + b.h / 2.0;
  let dx = bcx - acx;
  let dy = bcy - acy;
  if dx.abs() > dy.abs() * 1.15 {
    if dx >= 0.0 {
      ((a.x + a.w, acy), (b.x - 2.0, bcy))
    } else {
      ((a.x, acy), (b.x + b.w + 2.0, bcy))
    }
  } else if dy >= 0.0 {
    ((acx, a.y + a.h), (bcx, b.y - 2.0))
  } else {
    ((acx, a.y), (bcx, b.y + b.h + 2.0))
  }
}

// ── Panel 4: Pointer-Linked Memory Layout Diagram ───────────────────────

fn render_memory(
  svg: &mut String,
  r: Rect,
  regions: &[MemRegion],
  pointers: &[MemPointer],
  reveal: Option<usize>,
  focus_ids: &[String],
  p: &Palette,
) {
  if regions.is_empty() {
    return;
  }
  let cols = regions.len();
  let gap = if cols > 1 { 44.0 } else { 0.0 };
  let rw = (r.w - gap * (cols.saturating_sub(1) as f32)) / cols as f32;
  let visible_regions = reveal.unwrap_or(cols);

  let mut slot_rects: HashMap<&str, (Rect, usize)> = HashMap::new();

  for (ri, reg) in regions.iter().enumerate() {
    let rx = r.x + ri as f32 * (rw + gap);
    let color = p.tone_color(reg.tone.as_deref(), ri);
    let reg_visible = ri < visible_regions;
    let reg_opacity = if reg_visible { 1.0 } else { 0.28 };

    let _ = writeln!(
      svg,
      "<g opacity=\"{reg_opacity:.2}\">\n\
       <rect x=\"{rx:.1}\" y=\"{:.1}\" width=\"{rw:.1}\" height=\"{:.1}\" rx=\"12\" fill=\"{}\" stroke=\"{color}\" stroke-width=\"1.8\"/>\n\
       <rect x=\"{rx:.1}\" y=\"{:.1}\" width=\"{rw:.1}\" height=\"48\" rx=\"12\" fill=\"{color}\" fill-opacity=\"0.18\"/>\n\
       <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"18\" font-weight=\"700\" fill=\"{color}\">{}</text>",
      r.y,
      r.h,
      p.code_bg,
      r.y,
      rx + 16.0,
      r.y + 31.0,
      xml_escape(&reg.name)
    );
    if let Some(sub) = reg.subtitle.as_deref().filter(|s| !s.is_empty()) {
      let _ = writeln!(
        svg,
        "<text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"end\" font-family=\"{MONO_FONT}\" font-size=\"13.5\" fill=\"{}\">{}</text>",
        rx + rw - 14.0,
        r.y + 30.0,
        p.text_muted,
        xml_escape(sub)
      );
    }

    let slot_count = reg.slots.len().max(1);
    let slot_gap = 12.0_f32;
    let avail_h = (r.h - 76.0).max(80.0);
    let sh = ((avail_h - slot_gap * (slot_count.saturating_sub(1) as f32)) / slot_count as f32)
      .clamp(54.0, 96.0);

    for (si, slot) in reg.slots.iter().enumerate() {
      let sy = r.y + 62.0 + si as f32 * (sh + slot_gap);
      let sr = Rect {
        x: rx + 12.0,
        y: sy,
        w: rw - 24.0,
        h: sh,
      };
      slot_rects.insert(slot.id.as_str(), (sr, ri));
      let slot_focused = focus_ids.iter().any(|f| f == &slot.id);
      let s_border = if slot_focused { p.accent } else { p.border };
      let s_bw = if slot_focused { 2.4 } else { 1.2 };
      let _ = writeln!(
        svg,
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" rx=\"8\" fill=\"{}\" stroke=\"{s_border}\" stroke-width=\"{s_bw}\"/>",
        sr.x, sr.y, sr.w, sr.h, p.surface_alt
      );
      if let Some(off) = slot.offset.as_deref().filter(|o| !o.is_empty()) {
        let _ = writeln!(
          svg,
          "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{MONO_FONT}\" font-size=\"13\" fill=\"{}\">{}</text>",
          sr.x + 12.0,
          sr.y + 22.0,
          p.text_muted,
          xml_escape(off)
        );
      }
      let label_x = if slot.offset.is_some() {
        sr.x + 76.0
      } else {
        sr.x + 14.0
      };
      let _ = writeln!(
        svg,
        "<text x=\"{label_x:.1}\" y=\"{:.1}\" font-family=\"{MONO_FONT}\" font-size=\"16\" font-weight=\"700\" fill=\"{}\">{}</text>",
        sr.y + 23.0,
        p.text,
        xml_escape(&slot.label)
      );
      let val_lines = wrap_text(&slot.value, sr.w - 24.0, 14.5, 1);
      if let Some(vl) = val_lines.first() {
        let _ = writeln!(
          svg,
          "<text x=\"{:.1}\" y=\"{:.1}\" font-family=\"{MONO_FONT}\" font-size=\"14.5\" fill=\"{color}\">{}</text>",
          sr.x + 14.0,
          sr.y + sh - 13.0,
          xml_escape(vl)
        );
      }
    }
    svg.push_str("</g>\n");
  }

  for (pi, ptr) in pointers.iter().enumerate() {
    let (Some(&(a, ra)), Some(&(b, rb))) = (
      slot_rects.get(ptr.from.as_str()),
      slot_rects.get(ptr.to.as_str()),
    ) else {
      continue;
    };
    let active = ra < visible_regions && rb < visible_regions;
    let opacity = if active { 0.96 } else { 0.12 };
    let marker = match ptr.tone.as_deref() {
      Some("green") => "arrowGreen",
      Some("rust" | "orange" | "accent") => "arrowAccent",
      _ => {
        if pi % 2 == 0 {
          "arrowAccent"
        } else {
          "arrow"
        }
      }
    };
    let stroke = match marker {
      "arrowGreen" => p.green,
      "arrowAccent" => p.accent,
      _ => p.cyan,
    };
    let ((x1, y1), (x2, y2)) = box_anchor_pair(a, b);
    let mid_x = (x1 + x2) / 2.0;
    let _ = write!(
      svg,
      "<g opacity=\"{opacity:.2}\">\
       <circle cx=\"{x1:.1}\" cy=\"{y1:.1}\" r=\"4.5\" fill=\"{stroke}\"/>\
       <path d=\"M {x1:.1} {y1:.1} C {mid_x:.1} {y1:.1}, {mid_x:.1} {y2:.1}, {x2:.1} {y2:.1}\" fill=\"none\" stroke=\"{stroke}\" stroke-width=\"2.4\" marker-end=\"url(#{marker})\"/>"
    );
    if let Some(lbl) = ptr.label.as_deref().filter(|l| !l.is_empty()) {
      let lw = (measure_px(lbl, 13.0, false) + 14.0).min(160.0);
      let my = (y1 + y2) / 2.0;
      let _ = write!(
        svg,
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{lw:.1}\" height=\"21\" rx=\"5\" fill=\"{}\" stroke=\"{stroke}\" stroke-width=\"1\"/>\
         <text x=\"{mid_x:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"13\" font-weight=\"600\" fill=\"{stroke}\">{}</text>",
        mid_x - lw / 2.0,
        my - 10.5,
        p.bg,
        my + 4.5,
        xml_escape(lbl)
      );
    }
    svg.push_str("</g>\n");
  }
}

// ── Panel 5: Comparison Matrix Table ────────────────────────────────────

fn render_table(
  svg: &mut String,
  r: Rect,
  highlight_col: Option<usize>,
  columns: &[String],
  rows: &[Vec<String>],
  reveal: Option<usize>,
  p: &Palette,
) {
  if columns.is_empty() {
    return;
  }
  let ncols = columns.len();
  let cw = r.w / ncols as f32;
  let header_h = 54.0_f32;
  let nrows = rows.len().max(1);
  let row_h = ((r.h - header_h) / nrows as f32).clamp(52.0, 108.0);
  let visible_rows = reveal.unwrap_or(rows.len());

  // Header row.
  let _ = writeln!(
    svg,
    "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{header_h:.1}\" rx=\"10\" fill=\"{}\"/>",
    r.x, r.y, r.w, p.surface_alt
  );
  for (ci, col_name) in columns.iter().enumerate() {
    let cx = r.x + ci as f32 * cw;
    let is_hi = highlight_col == Some(ci);
    if is_hi {
      let _ = writeln!(
        svg,
        "<rect x=\"{cx:.1}\" y=\"{:.1}\" width=\"{cw:.1}\" height=\"{:.1}\" rx=\"10\" fill=\"{}\" fill-opacity=\"0.10\" stroke=\"{}\" stroke-width=\"1.5\"/>",
        r.y,
        header_h + row_h * rows.len() as f32,
        p.accent,
        p.accent
      );
    }
    let text_col = if is_hi { p.accent } else { p.text };
    let lines = wrap_text(col_name, cw - 24.0, 18.5, 1);
    if let Some(txt) = lines.first() {
      let _ = writeln!(
        svg,
        "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"18.5\" font-weight=\"700\" fill=\"{text_col}\">{}</text>",
        cx + 16.0,
        r.y + 34.0,
        xml_escape(txt)
      );
    }
  }

  for (ri, row) in rows.iter().enumerate() {
    let ry = r.y + header_h + ri as f32 * row_h;
    if ry + row_h > r.y + r.h + 4.0 {
      break;
    }
    let revealed = ri < visible_rows;
    let spotlight = reveal.is_some() && ri + 1 == visible_rows;
    let opacity = if !revealed { 0.28 } else { 1.0 };

    let _ = writeln!(svg, "<g opacity=\"{opacity:.2}\">");
    if spotlight {
      let _ = writeln!(
        svg,
        "<rect x=\"{:.1}\" y=\"{ry:.1}\" width=\"{:.1}\" height=\"{row_h:.1}\" rx=\"8\" fill=\"{}\" fill-opacity=\"0.12\" stroke=\"{}\" stroke-width=\"1.4\"/>",
        r.x, r.w, p.cyan, p.cyan
      );
    } else if ri % 2 == 1 {
      let _ = writeln!(
        svg,
        "<rect x=\"{:.1}\" y=\"{ry:.1}\" width=\"{:.1}\" height=\"{row_h:.1}\" fill=\"{}\" fill-opacity=\"0.42\"/>",
        r.x, r.w, p.surface_alt
      );
    }
    let _ = writeln!(
      svg,
      "<line x1=\"{:.1}\" y1=\"{ry:.1}\" x2=\"{:.1}\" y2=\"{ry:.1}\" stroke=\"{}\" stroke-width=\"1\"/>",
      r.x,
      r.x + r.w,
      p.border
    );

    for (ci, cell) in row.iter().take(ncols).enumerate() {
      let cx = r.x + ci as f32 * cw;
      let trimmed = cell.trim();
      let color = if ci == 0 {
        p.text
      } else if trimmed.starts_with('✓') || trimmed.starts_with("✅") {
        p.green
      } else if trimmed.starts_with('✗') || trimmed.starts_with("❌") {
        p.rose
      } else if trimmed.starts_with('⚠') {
        p.amber
      } else {
        p.text_sub
      };
      let weight = if ci == 0 || highlight_col == Some(ci) {
        "600"
      } else {
        "400"
      };
      let lines = wrap_text(trimmed, cw - 28.0, 16.5, 2);
      if lines.len() == 1 {
        let _ = writeln!(
          svg,
          "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"16.5\" font-weight=\"{weight}\" fill=\"{color}\">{}</text>",
          cx + 16.0,
          ry + row_h * 0.57,
          xml_escape(&lines[0])
        );
      } else if lines.len() >= 2 {
        let _ = writeln!(
          svg,
          "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"16\" font-weight=\"{weight}\" fill=\"{color}\">{}</text>\n\
           <text x=\"{:.1}\" y=\"{:.1}\" font-size=\"15.5\" fill=\"{}\">{}</text>",
          cx + 16.0,
          ry + row_h * 0.42,
          xml_escape(&lines[0]),
          cx + 16.0,
          ry + row_h * 0.75,
          if ci == 0 { p.text_sub } else { color },
          xml_escape(&lines[1])
        );
      }
    }
    svg.push_str("</g>\n");
  }
}

// ── Panel 6: Presentation Mind Map ──────────────────────────────────────

struct MmPlaced {
  depth: usize,
  side: i8, // -1 left, 0 root, +1 right
  y: f32,
  label: String,
  parent: Option<usize>,
  branch: usize,
}

fn render_mindmap_panel(
  svg: &mut String,
  r: Rect,
  root: &Value,
  reveal: Option<usize>,
  p: &Palette,
) {
  let Some(root_label) = root
    .get("label")
    .or_else(|| root.get("title"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|s| !s.is_empty())
  else {
    return;
  };
  let top_children = root
    .get("children")
    .and_then(Value::as_array)
    .map_or(&[][..], Vec::as_slice);

  let mut placed = vec![MmPlaced {
    depth: 0,
    side: 0,
    y: 0.0,
    label: root_label.to_string(),
    parent: None,
    branch: 0,
  }];

  let bilateral = top_children.len() >= 4;
  let split = if bilateral {
    top_children.len().div_ceil(2)
  } else {
    0
  };

  let mut left_leaf = 0.0_f32;
  let mut left_ys = Vec::new();
  let mut right_leaf = 0.0_f32;
  let mut right_ys = Vec::new();

  for (bi, ch) in top_children.iter().enumerate() {
    if bilateral && bi < split {
      if let Ok(cy) = place_mm(ch, 1, -1, Some(0), bi, &mut left_leaf, &mut placed) {
        left_ys.push(cy);
      }
    } else if let Ok(cy) = place_mm(ch, 1, 1, Some(0), bi, &mut right_leaf, &mut placed) {
      right_ys.push(cy);
    }
  }

  if !left_ys.is_empty() {
    let mid = (left_ys[0] + left_ys[left_ys.len() - 1]) * 0.5;
    for n in placed.iter_mut().filter(|n| n.side < 0) {
      n.y -= mid;
    }
  }
  if !right_ys.is_empty() {
    let mid = (right_ys[0] + right_ys[right_ys.len() - 1]) * 0.5;
    for n in placed.iter_mut().filter(|n| n.side > 0) {
      n.y -= mid;
    }
  }

  let max_depth = placed.iter().map(|n| n.depth).max().unwrap_or(0) + 1;
  let node_font = |d: usize| {
    if d == 0 {
      20.0
    } else if d == 1 {
      17.0
    } else {
      15.5
    }
  };
  let node_pad = |d: usize| if d == 0 { 38.0 } else { 28.0 };
  let node_h = |d: usize| {
    if d == 0 {
      42.0
    } else if d == 1 {
      34.0
    } else {
      30.0
    }
  };

  let root_w = measure_px(&placed[0].label, node_font(0), false) + node_pad(0);
  let mut right_col_w = vec![0.0_f32; max_depth];
  let mut left_col_w = vec![0.0_f32; max_depth];
  for n in &placed {
    let nw = measure_px(&n.label, node_font(n.depth), false) + node_pad(n.depth);
    if n.side > 0 {
      right_col_w[n.depth] = right_col_w[n.depth].max(nw);
    } else if n.side < 0 {
      left_col_w[n.depth] = left_col_w[n.depth].max(nw);
    }
  }

  let col_gap = 56.0_f32;
  let mut right_anchor = vec![0.0_f32; max_depth]; // left edge of box on right side
  let mut left_anchor = vec![0.0_f32; max_depth]; // right edge of box on left side
  if max_depth > 1 {
    right_anchor[1] = root_w * 0.5 + col_gap;
    left_anchor[1] = -root_w * 0.5 - col_gap;
    for d in 2..max_depth {
      right_anchor[d] = right_anchor[d - 1] + right_col_w[d - 1] + 46.0;
      left_anchor[d] = left_anchor[d - 1] - left_col_w[d - 1] - 46.0;
    }
  }

  let mut rects: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(placed.len());
  let mut min_x = f32::MAX;
  let mut max_x = f32::MIN;
  let mut min_y = f32::MAX;
  let mut max_y = f32::MIN;
  for n in &placed {
    let nw = measure_px(&n.label, node_font(n.depth), false) + node_pad(n.depth);
    let nh = node_h(n.depth);
    let bx = if n.depth == 0 {
      -nw * 0.5
    } else if n.side > 0 {
      right_anchor[n.depth]
    } else {
      left_anchor[n.depth] - nw
    };
    let by = n.y - nh * 0.5;
    min_x = min_x.min(bx);
    max_x = max_x.max(bx + nw);
    min_y = min_y.min(by);
    max_y = max_y.max(by + nh);
    rects.push((bx, by, nw, nh));
  }

  let nat_w = (max_x - min_x + 24.0).max(120.0);
  let nat_h = (max_y - min_y + 24.0).max(80.0);
  let scale = (r.w / nat_w).min(r.h / nat_h).clamp(0.45, 1.38);
  let cx_nat = (min_x + max_x) * 0.5;
  let cy_nat = (min_y + max_y) * 0.5;
  let tx = r.x + r.w * 0.5 - cx_nat * scale;
  let ty = r.y + r.h * 0.5 - cy_nat * scale;

  let visible_branches = reveal.unwrap_or(usize::MAX);
  let _ = writeln!(
    svg,
    "<g transform=\"translate({tx:.1},{ty:.1}) scale({scale:.3})\">"
  );
  for (ni, n) in placed.iter().enumerate() {
    if let Some(pi) = n.parent {
      let parent = &placed[pi];
      let (px, _, pw, _) = rects[pi];
      let (nx, _, nw, _) = rects[ni];
      let branch_col = p.tone_color(None, n.branch);
      let vis = n.depth == 0 || n.branch < visible_branches;
      let op = if vis { 0.85 } else { 0.24 };
      let (x1, x2) = if n.side >= 0 {
        (px + pw, nx)
      } else {
        (px, nx + nw)
      };
      let mid = (x1 + x2) * 0.5;
      let sw = if n.depth == 1 { 2.8 } else { 1.9 };
      let _ = writeln!(
        svg,
        "<path d=\"M{x1:.1},{y1:.1} C{mid:.1},{y1:.1} {mid:.1},{y2:.1} {x2:.1},{y2:.1}\" fill=\"none\" stroke=\"{branch_col}\" stroke-width=\"{sw}\" opacity=\"{op:.2}\"/>",
        y1 = parent.y,
        y2 = n.y
      );
    }
  }
  for (ni, n) in placed.iter().enumerate() {
    let (bx, by, nw, nh) = rects[ni];
    let branch_col = if n.depth == 0 {
      p.accent
    } else {
      p.tone_color(None, n.branch)
    };
    let vis = n.depth == 0 || n.branch < visible_branches;
    let op = if vis { 1.0 } else { 0.28 };
    let (fill, tfill) = if n.depth == 0 {
      (p.accent, "#FFFFFF")
    } else if p.dark {
      (p.surface_alt, p.text)
    } else {
      ("#FFFFFF", p.text)
    };
    let fs = node_font(n.depth);
    let rx = if n.depth == 0 { 11.0 } else { 8.0 };
    let _ = writeln!(
      svg,
      "<g opacity=\"{op:.2}\">\
       <rect x=\"{bx:.1}\" y=\"{by:.1}\" width=\"{nw:.1}\" height=\"{nh:.1}\" rx=\"{rx:.1}\" fill=\"{fill}\" stroke=\"{branch_col}\" stroke-width=\"{}\"/>\
       <text x=\"{:.1}\" y=\"{:.1}\" text-anchor=\"middle\" font-size=\"{fs:.1}\" font-weight=\"{}\" fill=\"{tfill}\">{}</text>\
       </g>",
      if n.depth == 0 { 2.2 } else { 1.6 },
      bx + nw * 0.5,
      n.y + fs * 0.36,
      if n.depth <= 1 { "700" } else { "500" },
      xml_escape(&n.label)
    );
  }
  svg.push_str("</g>\n");
}

fn place_mm(
  node: &Value,
  depth: usize,
  side: i8,
  parent: Option<usize>,
  branch: usize,
  next_leaf: &mut f32,
  out: &mut Vec<MmPlaced>,
) -> Result<f32, ()> {
  if depth >= 7 || out.len() >= 250 {
    return Err(());
  }
  let label = node
    .get("label")
    .or_else(|| node.get("title"))
    .and_then(Value::as_str)
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .ok_or(())?
    .to_string();
  let idx = out.len();
  out.push(MmPlaced {
    depth,
    side,
    y: 0.0,
    label,
    parent,
    branch,
  });
  let children = node
    .get("children")
    .and_then(Value::as_array)
    .map_or(&[][..], Vec::as_slice);
  let y = if children.is_empty() {
    let y = *next_leaf;
    *next_leaf += 44.0;
    y
  } else {
    let mut ys = Vec::with_capacity(children.len());
    for ch in children {
      ys.push(place_mm(
        ch,
        depth + 1,
        side,
        Some(idx),
        branch,
        next_leaf,
        out,
      )?);
    }
    *next_leaf += 10.0;
    (ys[0] + ys[ys.len() - 1]) / 2.0
  };
  out[idx].y = y;
  Ok(y)
}

// ── Document & Interactive HTML Exporters ───────────────────────────────

/// Emit Pandoc slide Markdown with editable text/code columns + embedded high-res slide figures + speaker notes.
pub fn deck_to_pandoc_markdown(deck: &SlideDeck, slide_png_paths: &[&Path]) -> String {
  let mut md = String::new();
  let _ = writeln!(md, "---");
  let _ = writeln!(md, "title: {}", json!(deck.title));
  if let Some(sub) = &deck.subtitle {
    let _ = writeln!(md, "subtitle: {}", json!(sub));
  }
  if let Some(author) = &deck.author {
    let _ = writeln!(md, "author: {}", json!(author));
  }
  let _ = writeln!(md, "---\n");

  for (idx, slide) in deck.slides.iter().enumerate() {
    let _ = writeln!(md, "## {}\n", slide.title);
    if let Some(sub) = &slide.subtitle {
      let _ = writeln!(md, "*{}*\n", sub);
    }
    if let Some(png) = slide_png_paths.get(idx) {
      let _ = writeln!(md, "![]({}){{ width=95% }}\n", png.display());
    }
    // Include editable summary bullets & code snippets on companion detail slides for PowerPoint editing.
    let mut text_notes = String::new();
    collect_editable_markdown(&slide.left, &mut text_notes);
    if let Some(r) = &slide.right {
      collect_editable_markdown(r, &mut text_notes);
    }
    if !text_notes.trim().is_empty() {
      let _ = writeln!(md, "## {}（源码与要点速查）\n", slide.title);
      md.push_str(&text_notes);
      if let Some(tk) = &slide.takeaway {
        let _ = writeln!(md, "\n> **核心要点**：{tk}\n");
      }
    }
    if let Some(notes) = slide.notes.as_deref().filter(|s| !s.trim().is_empty()) {
      let _ = writeln!(md, "::: notes\n{}\n:::\n", notes.trim());
    }
  }
  md
}

fn collect_editable_markdown(panel: &Panel, out: &mut String) {
  match panel {
    Panel::Bullets { items, .. } => {
      for item in items.iter().take(5) {
        if let Some(b) = &item.badge {
          let _ = writeln!(out, "- **`{b}` {}**：{}", item.heading, item.body);
        } else {
          let _ = writeln!(out, "- **{}**：{}", item.heading, item.body);
        }
      }
      out.push('\n');
    }
    Panel::Code { language, code, .. } => {
      let lang = language.as_deref().unwrap_or("rust");
      let clipped: String = code.lines().take(16).collect::<Vec<_>>().join("\n");
      let _ = writeln!(out, "```{lang}\n{clipped}\n```\n");
    }
    Panel::Table { columns, rows, .. } if !columns.is_empty() => {
      let _ = writeln!(out, "| {} |", columns.join(" | "));
      let sep: Vec<&str> = columns.iter().map(|_| "---").collect();
      let _ = writeln!(out, "| {} |", sep.join(" | "));
      for row in rows.iter().take(6) {
        let _ = writeln!(out, "| {} |", row.join(" | "));
      }
      out.push('\n');
    }
    Panel::Image { caption, title, .. } => {
      let heading = title.as_deref().unwrap_or("配图说明");
      if let Some(cap) = caption.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let _ = writeln!(out, "- **{heading}**：{cap}\n");
      }
    }
    _ => {}
  }
}

/// Emit a 16:9 widescreen Typst handout source embedding every rendered high-res slide + speaker notes.
pub fn deck_to_typst_source(deck: &SlideDeck, slide_rel_paths: &[String]) -> String {
  let mut typ = String::new();
  typ.push_str(
    "#set page(paper: \"presentation-16-9\", margin: (x: 28pt, y: 24pt), fill: rgb(\"#0B1120\"))\n\
     #set text(font: (\"PingFang SC\", \"Hiragino Sans GB\", \"STHeiti\", \"Helvetica Neue\", \"Apple Symbols\", \"Noto Sans CJK SC\"), fill: rgb(\"#F8FAFC\"), size: 13pt)\n\n",
  );
  // Cover page.
  let _ = writeln!(
    typ,
    "#align(center + horizon)[\n  #text(size: 34pt, weight: \"bold\", fill: rgb(\"#F97316\"))[{}]\n  #v(14pt)\n  #text(size: 18pt, fill: rgb(\"#CBD5E1\"))[{}]\n  #v(22pt)\n  #text(size: 13pt, fill: rgb(\"#94A3B8\"))[{}]\n]\n",
    typst_escape(&deck.title),
    typst_escape(deck.subtitle.as_deref().unwrap_or("")),
    typst_escape(
      deck
        .author
        .as_deref()
        .unwrap_or("Generated by Yanshi Content Studio")
    )
  );

  for (idx, rel) in slide_rel_paths.iter().enumerate() {
    let _ = writeln!(
      typ,
      "#pagebreak()\n#page(margin: 0pt)[\n  #image(\"{rel}\", width: 100%, height: 100%, fit: \"contain\")\n]"
    );
    if let Some(slide) = deck.slides.get(idx)
      && let Some(notes) = slide.notes.as_deref().filter(|n| !n.trim().is_empty())
    {
      let _ = writeln!(
        typ,
        "#pagebreak()\n#block(inset: 20pt, radius: 10pt, fill: rgb(\"#111A2E\"), stroke: 1pt + rgb(\"#263759\"), width: 100%)[\n  #text(size: 18pt, weight: \"bold\", fill: rgb(\"#38BDF8\"))[讲义批注 · {}]\n  #v(10pt)\n  #text(size: 14pt, fill: rgb(\"#E2E8F0\"))[{}]\n  #v(12pt)\n  #text(size: 13pt, weight: \"bold\", fill: rgb(\"#F97316\"))[核心结论：{}]\n]",
        typst_escape(&slide.title),
        typst_escape(notes),
        typst_escape(slide.takeaway.as_deref().unwrap_or(""))
      );
    }
  }
  typ
}

fn typst_escape(s: &str) -> String {
  let mut out = String::with_capacity(s.len() + 8);
  for c in s.chars() {
    if matches!(
      c,
      '#' | '[' | ']' | '*' | '_' | '$' | '@' | '<' | '>' | '\\'
    ) {
      out.push('\\');
    }
    out.push(c);
  }
  out
}

/// Self-contained interactive HTML Keynote Slide + Video synchronized player.
pub struct SlideFrameMeta {
  pub slide_idx: usize,
  pub step_idx: usize,
  pub title: String,
  pub caption: String,
  pub image_src: String,
  pub start_secs: f64,
}

pub fn deck_to_interactive_html(
  deck: &SlideDeck,
  frames: &[SlideFrameMeta],
  video_rel: Option<&str>,
  vtt_data_uri: Option<&str>,
  extra_links: &[(&str, &str)],
) -> String {
  let frames_json: Vec<Value> = frames
    .iter()
    .map(|f| {
      json!({
        "slide": f.slide_idx + 1,
        "step": f.step_idx + 1,
        "title": f.title,
        "caption": f.caption,
        "src": f.image_src,
        "start": (f.start_secs * 100.0).round() / 100.0,
      })
    })
    .collect();
  let safe_json = serde_json::to_string(&frames_json)
    .unwrap_or_else(|_| "[]".to_string())
    .replace("</", "<\\/");

  let mut links_html = String::new();
  for &(label, href) in extra_links {
    let _ = write!(
      links_html,
      "<a class=\"btn\" href=\"{}\" target=\"_blank\">{}</a>",
      xml_escape(href),
      xml_escape(label)
    );
  }

  let video_block = if let Some(vrel) = video_rel {
    let track = vtt_data_uri.map_or_else(String::new, |uri| {
      format!("<track kind=\"subtitles\" srclang=\"zh\" label=\"中文字幕\" src=\"{uri}\" default>")
    });
    format!(
      "<div class=\"panel\" id=\"videoPanel\">\
         <video id=\"courseVideo\" controls preload=\"metadata\" src=\"{}\">{track}</video>\
       </div>",
      xml_escape(vrel)
    )
  } else {
    String::new()
  };

  format!(
    "<!doctype html>\n<html lang=\"zh-CN\">\n<head>\n<meta charset=\"utf-8\">\n\
     <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
     <title>{title}</title>\n\
     <style>\n\
       :root {{ --bg:#070c18; --card:#10192d; --border:#243556; --accent:#f97316; --cyan:#38bdf8; --text:#f8fafc; --muted:#94a3b8; }}\n\
       * {{ box-sizing:border-box; }}\n\
       body {{ margin:0; background:var(--bg); color:var(--text); font:15px/1.6 -apple-system,'PingFang SC','Hiragino Sans GB','Noto Sans CJK SC',sans-serif; }}\n\
       header {{ display:flex; flex-wrap:wrap; align-items:center; justify-content:space-between; gap:12px; padding:16px 28px; border-bottom:1px solid var(--border); background:#0b1324; position:sticky; top:0; z-index:10; }}\n\
       h1 {{ margin:0; font-size:20px; }}\n\
       .sub {{ color:var(--muted); font-size:13px; }}\n\
       .toolbar {{ display:flex; flex-wrap:wrap; gap:8px; align-items:center; }}\n\
       .btn {{ background:#172440; color:var(--text); border:1px solid var(--border); padding:7px 14px; border-radius:8px; cursor:pointer; text-decoration:none; font-size:13px; font-weight:600; }}\n\
       .btn.active, .btn:hover {{ border-color:var(--accent); color:#fff; background:#27395e; }}\n\
       main {{ max-width:1440px; margin:20px auto; padding:0 24px; display:grid; grid-template-columns:1fr 340px; gap:20px; }}\n\
       @media (max-width:1080px) {{ main {{ grid-template-columns:1fr; }} }}\n\
       .stage {{ background:var(--card); border:1px solid var(--border); border-radius:14px; padding:14px; }}\n\
       video, #slideImg {{ width:100%; aspect-ratio:16/9; border-radius:10px; background:#000; display:block; object-fit:contain; }}\n\
       .caption-box {{ margin-top:12px; padding:12px 16px; border-radius:10px; background:#0b1324; border-left:4px solid var(--accent); font-size:15px; }}\n\
       .deck-controls {{ display:flex; justify-content:space-between; align-items:center; margin-top:10px; }}\n\
       .playlist {{ background:var(--card); border:1px solid var(--border); border-radius:14px; padding:12px; max-height:78vh; overflow-y:auto; }}\n\
       .step-item {{ padding:10px 12px; border-radius:9px; border:1px solid transparent; cursor:pointer; margin-bottom:6px; transition:0.15s; }}\n\
       .step-item:hover {{ background:#16233f; }}\n\
       .step-item.active {{ background:#1b2b4d; border-color:var(--cyan); }}\n\
       .step-meta {{ font-size:12px; color:var(--cyan); font-family:monospace; font-weight:700; }}\n\
       .step-title {{ font-size:13.5px; font-weight:600; margin:2px 0; }}\n\
       .step-cap {{ font-size:12px; color:var(--muted); display:-webkit-box; -webkit-line-clamp:2; -webkit-box-orient:vertical; overflow:hidden; }}\n\
       .hidden {{ display:none !important; }}\n\
     </style>\n</head>\n<body>\n\
     <header>\n\
       <div><h1>{title}</h1><div class=\"sub\">{subtitle}</div></div>\n\
       <div class=\"toolbar\">\n\
         {mode_buttons}\
         {links_html}\n\
       </div>\n\
     </header>\n\
     <main>\n\
       <div class=\"stage\">\n\
         {video_block}\n\
         <div id=\"slidePanel\" class=\"{slide_panel_cls}\">\n\
           <img id=\"slideImg\" alt=\"Slide\">\n\
           <div class=\"deck-controls\">\n\
             <button class=\"btn\" onclick=\"stepDelta(-1)\">← 上一步 (Prev)</button>\n\
             <span id=\"slideCounter\" class=\"sub\"></span>\n\
             <button class=\"btn\" onclick=\"stepDelta(1)\">下一步 (Next) →</button>\n\
           </div>\n\
         </div>\n\
         <div class=\"caption-box\" id=\"captionBox\"></div>\n\
       </div>\n\
       <aside class=\"playlist\" id=\"playlist\"></aside>\n\
     </main>\n\
     <script>\n\
       const FRAMES = {safe_json};\n\
       let idx = 0;\n\
       const video = document.getElementById('courseVideo');\n\
       const vPanel = document.getElementById('videoPanel');\n\
       const sPanel = document.getElementById('slidePanel');\n\
       const sImg = document.getElementById('slideImg');\n\
       const capBox = document.getElementById('captionBox');\n\
       const counter = document.getElementById('slideCounter');\n\
       const listEl = document.getElementById('playlist');\n\
       function setMode(m) {{\n\
         if (!vPanel) return;\n\
         document.getElementById('btnVideo').classList.toggle('active', m==='video');\n\
         document.getElementById('btnDeck').classList.toggle('active', m==='deck');\n\
         vPanel.classList.toggle('hidden', m!=='video');\n\
         sPanel.classList.toggle('hidden', m!=='deck');\n\
         if (m==='deck' && video) video.pause();\n\
       }}\n\
       function selectFrame(i, seekVideo) {{\n\
         if (!FRAMES.length) return;\n\
         idx = Math.max(0, Math.min(FRAMES.length - 1, i));\n\
         const f = FRAMES[idx];\n\
         sImg.src = f.src;\n\
         capBox.textContent = `[P${{String(f.slide).padStart(2,'0')}} · Step ${{f.step}}] ${{f.caption}}`;\n\
         counter.textContent = `帧 ${{idx + 1}} / ${{FRAMES.length}}（支持键盘 ← → 翻页）`;\n\
         document.querySelectorAll('.step-item').forEach((el, k) => el.classList.toggle('active', k === idx));\n\
         if (seekVideo && video && !vPanel.classList.contains('hidden')) {{\n\
           video.currentTime = f.start + 0.05;\n\
           video.play().catch(()=>{{}});\n\
         }}\n\
       }}\n\
       function stepDelta(d) {{ selectFrame(idx + d, true); }}\n\
       FRAMES.forEach((f, k) => {{\n\
         const d = document.createElement('div');\n\
         d.className = 'step-item';\n\
         const mm = String(Math.floor(f.start / 60)).padStart(2, '0');\n\
         const ss = String(Math.floor(f.start % 60)).padStart(2, '0');\n\
         d.innerHTML = `<div class=\"step-meta\">${{mm}}:${{ss}} · Slide ${{f.slide}}.${{f.step}}</div>` +\n\
           `<div class=\"step-title\"></div><div class=\"step-cap\"></div>`;\n\
         d.querySelector('.step-title').textContent = f.title;\n\
         d.querySelector('.step-cap').textContent = f.caption;\n\
         d.onclick = () => selectFrame(k, true);\n\
         listEl.appendChild(d);\n\
       }});\n\
       if (video) {{\n\
         video.addEventListener('timeupdate', () => {{\n\
           const t = video.currentTime;\n\
           let active = 0;\n\
           for (let k = 0; k < FRAMES.length; k++) {{ if (t >= FRAMES[k].start) active = k; }}\n\
           if (active !== idx) selectFrame(active, false);\n\
         }});\n\
       }}\n\
       window.addEventListener('keydown', (e) => {{\n\
         if (e.key === 'ArrowRight') stepDelta(1);\n\
         if (e.key === 'ArrowLeft') stepDelta(-1);\n\
       }});\n\
       selectFrame(0, false);\n\
     </script>\n</body>\n</html>\n",
    title = xml_escape(&deck.title),
    subtitle = xml_escape(deck.subtitle.as_deref().unwrap_or("")),
    mode_buttons = if video_rel.is_some() {
      "<button id=\"btnVideo\" class=\"btn active\" onclick=\"setMode('video')\">🎬 视频讲解模式</button>\
       <button id=\"btnDeck\" class=\"btn\" onclick=\"setMode('deck')\">📊 交互式幻灯片模式</button>"
    } else {
      ""
    },
    links_html = links_html,
    video_block = video_block,
    slide_panel_cls = if video_rel.is_some() { "hidden" } else { "" },
    safe_json = safe_json,
  )
}

// ── AsyncNode Implementation (`type: render_slides`) ────────────────────

pub struct RenderSlidesNode {
  id: String,
}

impl RenderSlidesNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

fn input_err(node: &str, msg: impl std::fmt::Display) -> YanshiError {
  YanshiError::NodeInputError {
    message: format!("{node}: {msg}"),
  }
}

#[async_trait]
impl AsyncNode for RenderSlidesNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let raw_deck = match inputs.get("deck") {
      Some(FlowValue::Json(Value::String(s))) => serde_json::from_str::<Value>(s)
        .map_err(|e| input_err(&self.id, format!("deck is not valid JSON: {e}")))?,
      Some(FlowValue::Json(v)) => v.clone(),
      Some(FlowValue::File { path, .. }) => {
        let s = tokio::fs::read_to_string(path).await.map_err(|e| {
          input_err(
            &self.id,
            format!("cannot read deck from {}: {e}", path.display()),
          )
        })?;
        serde_json::from_str::<Value>(&s)
          .map_err(|e| input_err(&self.id, format!("deck file is not valid JSON: {e}")))?
      }
      _ => return Err(input_err(&self.id, "missing input 'deck'")),
    };

    let deck: SlideDeck = serde_json::from_value(raw_deck)
      .map_err(|e| input_err(&self.id, format!("invalid slide deck schema: {e}")))?;
    if deck.slides.is_empty() {
      return Err(input_err(&self.id, "deck.slides must not be empty"));
    }

    let out_width = match inputs.get("width") {
      Some(FlowValue::Json(Value::Number(n))) => n
        .as_u64()
        .and_then(|w| u32::try_from(w).ok())
        .unwrap_or(1920),
      _ => 1920,
    };
    let progressive = match inputs.get("progressive") {
      Some(FlowValue::Json(Value::Bool(b))) => *b,
      _ => true,
    };
    let section_cards = match inputs.get("section_cards") {
      Some(FlowValue::Json(Value::Bool(b))) => *b,
      _ => false,
    };
    let pip_safe_zone = match inputs.get("pip_safe_zone") {
      Some(FlowValue::Json(Value::Bool(b))) => *b,
      _ => false,
    };
    let takeaway_only_bar = match inputs.get("takeaway_only_bar") {
      Some(FlowValue::Json(Value::Bool(b))) => *b,
      _ => false,
    };

    let mut sections: Vec<String> = Vec::new();
    for (idx, s) in deck.slides.iter().enumerate() {
      let label = s
        .section
        .clone()
        .filter(|sec| !sec.trim().is_empty())
        .unwrap_or_else(|| format!("{:02}", idx + 1));
      if !sections.contains(&label) {
        sections.push(label);
      }
    }

    let mut slide_groups = Vec::with_capacity(deck.slides.len());
    let mut full_slides = Vec::with_capacity(deck.slides.len());
    let mut full_png_paths = Vec::with_capacity(deck.slides.len());
    let mut flat_steps = Vec::new();
    let mut html_frames = Vec::new();
    let mut est_time = 0.0_f64;
    let mut prev_section: Option<String> = None;

    for (si, slide) in deck.slides.iter().enumerate() {
      let steps = if progressive {
        effective_steps(slide)
      } else {
        vec![SlideStep {
          caption: slide.notes.clone().unwrap_or_else(|| slide.title.clone()),
          ..SlideStep::default()
        }]
      };

      let mut step_entries = Vec::with_capacity(steps.len() + 1);
      let mut last_png_path = None;
      let mut last_svg_path = None;

      let cur_section = slide
        .section
        .clone()
        .filter(|sec| !sec.trim().is_empty())
        .unwrap_or_else(|| format!("{:02}", si + 1));
      let emit_section_card = section_cards
        && deck.slides.len() >= 2
        && prev_section.as_deref() != Some(cur_section.as_str());
      prev_section = Some(cur_section.clone());

      if emit_section_card {
        let sec_idx = sections.iter().position(|s| s == &cur_section).unwrap_or(0);
        let ratio = ((si as f32 + 0.15) / deck.slides.len().max(1) as f32).clamp(0.02, 1.0);
        let card_svg =
          render_section_card_svg(&deck, sec_idx, &sections, slide, ratio, pip_safe_zone);
        let width_copy = out_width;
        let svg_for_png = card_svg.clone();
        let card_png =
          tokio::task::spawn_blocking(move || svg_to_png(&svg_for_png, Some(width_copy)))
            .await
            .map_err(|e| YanshiError::NodeExecutionFailed {
              message: format!("{}: section card raster task failed: {e}", self.id),
            })?
            .map_err(|e| input_err(&self.id, e))?;

        let svg_file = write_artifact(&self.id, card_svg.as_bytes(), "image/svg+xml").await?;
        let png_file = write_artifact(&self.id, &card_png, "image/png").await?;
        let FlowValue::File { path: png_path, .. } = &png_file else {
          unreachable!("write_artifact always returns FlowValue::File");
        };
        let FlowValue::File { path: svg_path, .. } = &svg_file else {
          unreachable!("write_artifact always returns FlowValue::File");
        };

        let card_caption = format!("{cur_section} · {}", slide.title);
        let card_spoken = if si == 0 {
          format!(
            "欢迎来到《{}》技术精讲。首先进入{}：{}。",
            deck.title, cur_section, slide.title
          )
        } else {
          format!("接下来进入{}：{}。", cur_section, slide.title)
        };

        use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
        html_frames.push(SlideFrameMeta {
          slide_idx: si,
          step_idx: 0,
          title: format!("章节导览 · {}", slide.title),
          caption: card_caption.clone(),
          image_src: format!("data:image/png;base64,{}", B64.encode(&card_png)),
          start_secs: est_time,
        });
        est_time += (card_spoken.chars().count() as f64 * 0.18).max(3.0);

        step_entries.push(json!({
          "image": png_path,
          "svg": svg_path,
          "caption": card_caption,
          "spoken": card_spoken,
          "is_section_card": true,
        }));
        flat_steps.push(json!({
          "beat_index": flat_steps.len(),
          "slide": si,
          "step": 0,
          "kind": "section_card",
          "is_section_card": true,
          "is_slide_start": true,
          "is_slide_end": false,
          "section": cur_section,
          "title": slide.title,
          "image": png_path,
          "svg": svg_path,
          "caption": card_caption,
          "spoken": card_spoken,
        }));
      }

      for (sti, step) in steps.iter().enumerate() {
        let step_sel = if progressive { Some(sti) } else { None };
        let ratio = ((si as f32 + (sti + 1) as f32 / steps.len().max(1) as f32)
          / deck.slides.len().max(1) as f32)
          .clamp(0.02, 1.0);
        let opts = SlideRenderOptions {
          progress_ratio: Some(ratio),
          pip_safe_zone,
          takeaway_only_bar,
        };
        let svg = render_slide_svg_with_options(&deck, si, step_sel, opts)
          .map_err(|e| input_err(&self.id, e))?;
        let width_copy = out_width;
        let svg_for_png = svg.clone();
        let png_bytes =
          tokio::task::spawn_blocking(move || svg_to_png(&svg_for_png, Some(width_copy)))
            .await
            .map_err(|e| YanshiError::NodeExecutionFailed {
              message: format!("{}: slide raster task failed: {e}", self.id),
            })?
            .map_err(|e| input_err(&self.id, e))?;

        let svg_file = write_artifact(&self.id, svg.as_bytes(), "image/svg+xml").await?;
        let png_file = write_artifact(&self.id, &png_bytes, "image/png").await?;
        let FlowValue::File { path: png_path, .. } = &png_file else {
          unreachable!("write_artifact always returns FlowValue::File");
        };
        let FlowValue::File { path: svg_path, .. } = &svg_file else {
          unreachable!("write_artifact always returns FlowValue::File");
        };

        let spoken_str = step.spoken_text().to_string();

        // Embed data URI in standalone artifact HTML so it opens cleanly from anywhere.
        use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
        html_frames.push(SlideFrameMeta {
          slide_idx: si,
          step_idx: sti + usize::from(emit_section_card),
          title: slide.title.clone(),
          caption: step.caption.clone(),
          image_src: format!("data:image/png;base64,{}", B64.encode(&png_bytes)),
          start_secs: est_time,
        });
        est_time += (spoken_str.chars().count() as f64 * 0.18).max(4.0);

        last_png_path = Some(png_path.clone());
        last_svg_path = Some(svg_path.clone());
        step_entries.push(json!({
          "image": png_path,
          "svg": svg_path,
          "caption": step.caption,
          "spoken": spoken_str,
          "is_section_card": false,
        }));
        flat_steps.push(json!({
          "beat_index": flat_steps.len(),
          "slide": si,
          "step": sti,
          "kind": "slide_step",
          "is_section_card": false,
          "is_slide_start": sti == 0 && !emit_section_card,
          "is_slide_end": sti + 1 == steps.len(),
          "section": cur_section,
          "title": slide.title,
          "image": png_path,
          "svg": svg_path,
          "caption": step.caption,
          "spoken": spoken_str,
        }));
      }

      if let (Some(png_p), Some(svg_p)) = (last_png_path, last_svg_path) {
        full_png_paths.push(png_p.clone());
        full_slides.push(json!({
          "slide": si,
          "title": slide.title,
          "png": png_p,
          "svg": svg_p,
          "notes": slide.notes.clone().unwrap_or_default(),
        }));
      }

      slide_groups.push(json!({
        "slide": si,
        "title": slide.title,
        "notes": slide.notes.clone().unwrap_or_else(|| slide.title.clone()),
        "frames": step_entries,
      }));
    }

    let png_refs: Vec<&Path> = full_png_paths.iter().map(|p| p.as_path()).collect();
    let pandoc_md = deck_to_pandoc_markdown(&deck, &png_refs);
    let md_file = write_artifact(&self.id, pandoc_md.as_bytes(), "text/markdown").await?;

    let html = deck_to_interactive_html(&deck, &html_frames, None, None, &[]);
    let html_file = write_artifact(&self.id, html.as_bytes(), "text/html").await?;

    Ok(HashMap::from([
      (
        format!("{}_frames", self.id),
        FlowValue::Json(Value::Array(slide_groups)),
      ),
      (
        format!("{}_steps", self.id),
        FlowValue::Json(Value::Array(flat_steps)),
      ),
      (
        format!("{}_slides", self.id),
        FlowValue::Json(Value::Array(full_slides)),
      ),
      (format!("{}_markdown", self.id), md_file),
      (format!("{}_html", self.id), html_file),
    ]))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use yanshi_core::run_context;

  #[tokio::test]
  async fn renders_code_memory_table_flowchart_and_mindmap_to_1080p_pngs() {
    let deck = json!({
      "title": "Rust Traits 技术精讲",
      "subtitle": "零成本抽象与虚表内存模型",
      "slides": [
        {
          "section": "01 核心机制",
          "title": "静态分发 vs &dyn Trait 胖指针",
          "subtitle": "编译期单态化与 16 字节双指针结构对比",
          "left": {
            "kind": "code",
            "title": "dispatch.rs",
            "language": "rust",
            "code": "pub trait Draw {\n  fn draw(&self);\n}\n\npub fn render_dyn(item: &dyn Draw) {\n  item.draw(); // vtable 间接调用\n}",
            "callouts": [{"line": 5, "text": "16B 胖指针", "tone": "rust"}]
          },
          "right": {
            "kind": "memory",
            "title": "&dyn Draw 内存布局",
            "regions": [
              {"name": "Stack", "subtitle": "16 Bytes", "tone": "rust", "slots": [
                {"id": "dp", "offset": "+0x00", "label": "data_ptr", "value": "0x7f8a_1020"},
                {"id": "vp", "offset": "+0x08", "label": "vtable_ptr", "value": "0x1000_4080"}
              ]},
              {"name": "VTable (.rodata)", "subtitle": "Static", "tone": "cyan", "slots": [
                {"id": "vt", "offset": "+0x18", "label": "draw_fn", "value": "Button::draw"}
              ]}
            ],
            "pointers": [
              {"from": "vp", "to": "vt", "label": "查表跳转", "tone": "cyan"}
            ]
          },
          "takeaway": "泛型编译期单态化零虚表开销；&dyn Trait 以 16B 胖指针换取异构集合能力。",
          "notes": "首先看左侧源码定义。接着观察右侧栈上的十六字节胖指针布局。"
        },
        {
          "section": "02 全景对比",
          "title": "横向对比与知识全景",
          "left": {
            "kind": "table",
            "highlight_col": 1,
            "columns": ["维度", "Rust Traits", "C void*"],
            "rows": [
              ["类型安全", "✓ 编译期契约", "✗ 运行期强转"],
              ["零开销多态", "✓ 单态化内联", "✗ 间接跳转"]
            ]
          },
          "right": {
            "kind": "mindmap",
            "root": {
              "label": "Traits",
              "children": [
                {"label": "静态分发", "children": [{"label": "单态化"}]},
                {"label": "动态分发", "children": [{"label": "16B Fat Ptr"}]}
              ]
            }
          },
          "notes": "左侧对比类型安全与开销。右侧汇总知识导图。"
        }
      ]
    });

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let inputs = HashMap::from([("deck".to_string(), FlowValue::Json(deck))]);
    let outputs = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderSlidesNode::new("deck").execute(&inputs),
    )
    .await
    .expect("slides render");

    let FlowValue::Json(Value::Array(groups)) = &outputs["deck_frames"] else {
      panic!("expected deck_frames array");
    };
    assert_eq!(groups.len(), 2);
    let first_frames = groups[0]["frames"].as_array().expect("frames");
    assert_eq!(
      first_frames.len(),
      2,
      "two sentences -> two progressive steps"
    );

    let png_path = first_frames[0]["image"].as_str().expect("png path");
    let pix = resvg::tiny_skia::Pixmap::load_png(png_path).expect("valid PNG");
    assert_eq!((pix.width(), pix.height()), (1920, 1080));

    let FlowValue::File { path: md_path, .. } = &outputs["deck_markdown"] else {
      panic!("expected markdown file");
    };
    let md = std::fs::read_to_string(md_path).expect("md");
    assert!(md.contains("静态分发 vs &dyn Trait 胖指针"));
    assert!(md.contains("::: notes"));
  }

  #[tokio::test]
  async fn renders_image_panel_from_run_artifact_and_blocks_outside_paths() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let outside = tempfile::TempDir::new().expect("outside");
    let mut pix = resvg::tiny_skia::Pixmap::new(64, 64).expect("pixmap");
    pix.fill(resvg::tiny_skia::Color::from_rgba8(56, 189, 248, 255));
    let png_bytes = pix.encode_png().expect("png");
    let inside_img = tmp.path().join("illus.png");
    let outside_img = outside.path().join("secret.png");
    std::fs::write(&inside_img, &png_bytes).expect("write inside");
    std::fs::write(&outside_img, &png_bytes).expect("write outside");

    let deck = json!({
      "title": "插图面板校验",
      "slides": [
        {
          "title": "主题配图与安全边界",
          "left": {
            "kind": "image",
            "title": "AI 概念插图",
            "src": inside_img.to_string_lossy(),
            "caption": "沙箱内产物插图"
          },
          "right": {
            "kind": "image",
            "title": "越界路径拦截",
            "src": outside_img.to_string_lossy()
          },
          "notes": "验证插图内嵌。"
        }
      ]
    });

    let inputs = HashMap::from([
      ("deck".to_string(), FlowValue::Json(deck)),
      ("progressive".to_string(), FlowValue::Json(json!(false))),
    ]);
    let outputs = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderSlidesNode::new("illus").execute(&inputs),
    )
    .await
    .expect("image slide renders");

    let FlowValue::Json(Value::Array(slides)) = &outputs["illus_slides"] else {
      panic!("expected illus_slides");
    };
    let svg_path = slides[0]["svg"].as_str().expect("svg path");
    let svg_text = std::fs::read_to_string(svg_path).expect("svg text");
    assert!(
      svg_text.contains("data:image/png;base64,"),
      "inside artifact embeds as data URI"
    );
    assert!(
      svg_text.contains("插图资源已就绪"),
      "outside path must not be read"
    );
  }

  #[tokio::test]
  async fn renders_section_cards_spoken_steps_and_non_overlapping_code_callouts() {
    let deck = json!({
      "title": "Tokio 调度器精讲",
      "subtitle": "Work-Stealing 核心机制",
      "slides": [
        {
          "section": "01 核心数据结构",
          "title": "Local Queue 与全局注入队列",
          "left": {
            "kind": "code",
            "language": "rust",
            "code": "impl<T: std::fmt::Display + Send + Sync + 'static> WorkerQueue<T> { /* very long generic impl line that used to collide with badge */ }\n\npub fn steal_half(&mut self) -> Option<Task> {\n  self.ring.pop_front()\n}",
            "callouts": [
              {"line": 1, "text": "全生态覆盖实现", "tone": "green"},
              {"line": 3, "text": "无锁窃取一半", "tone": "rust"}
            ]
          },
          "right": {
            "kind": "flowchart",
            "direction": "LR",
            "nodes": [
              {"id": "w1", "label": "Worker 1"},
              {"id": "w2", "label": "Worker 2"}
            ],
            "edges": [
              {"from": "w1", "to": "w2", "label": "steal"},
              {"from": "w2", "to": "w1", "label": "rebalance"}
            ]
          },
          "takeaway": "本地环形队列零锁竞争，空闲线程按半数批量窃取。",
          "steps": [
            {
              "caption": "泛型队列定义与约束",
              "spoken": "请大家先看屏幕左侧第一行，这里定义了工作线程队列的泛型边界。",
              "code_lines": [1]
            },
            {
              "caption": "无锁批量窃取逻辑",
              "spoken": "接着往下看第三行，当线程空闲时会从邻居环形缓冲区窃取一半任务。",
              "code_lines": [3, 5]
            }
          ]
        },
        {
          "section": "02 性能判据",
          "title": "尾延迟优化",
          "left": {
            "kind": "bullets",
            "items": [
              {"badge": "LIFO", "heading": "热缓存槽位", "body": "刚唤醒的子任务直接进入 LIFO slot 命中 L1 Cache"}
            ]
          },
          "notes": "LIFO 槽显著降低消息传递型负载的尾延迟。"
        }
      ]
    });

    let tmp = tempfile::TempDir::new().expect("tempdir");
    let inputs = HashMap::from([
      ("deck".to_string(), FlowValue::Json(deck)),
      ("section_cards".to_string(), FlowValue::Json(json!(true))),
      ("pip_safe_zone".to_string(), FlowValue::Json(json!(true))),
    ]);
    let outputs = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderSlidesNode::new("talk").execute(&inputs),
    )
    .await
    .expect("slides render");

    let FlowValue::Json(Value::Array(steps)) = &outputs["talk_steps"] else {
      panic!("expected talk_steps array");
    };
    // 2 section cards + 2 steps on slide 0 + 1 step on slide 1 = 5 progressive beats.
    assert_eq!(steps.len(), 5);
    assert_eq!(steps[0]["kind"], "section_card");
    assert!(
      steps[1]["spoken"]
        .as_str()
        .expect("spoken")
        .contains("屏幕左侧第一行"),
      "spoken script preserved separately from short caption"
    );
    assert_eq!(steps[1]["caption"], "泛型队列定义与约束");
    // Auto-inferred focus_ids on step 1 even when omitted in JSON.
    let step1_svg = std::fs::read_to_string(steps[1]["svg"].as_str().expect("svg")).expect("read");
    assert!(
      step1_svg.contains("…"),
      "long code line clipped cleanly before callout badge"
    );
    assert!(
      step1_svg.contains("width=\"1408.0\" height=\"86\""),
      "pip_safe_zone narrows footer bar to leave bottom-right presenter dock clear"
    );
  }
}
