//! Typed client, studio generator catalogue, media classification, and pure
//! helper functions for the 24-route Yanshi Notebook API (`/v1/notebooks/*`).

use std::collections::{BTreeMap, HashMap, HashSet};

use serde::{Deserialize, Serialize};

use crate::api::{
  encode_uri_component, join_api_url, request_json, request_no_content, resolve_api_base,
};

// ── Domain Types ─────────────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Notebook {
  pub id: String,
  #[serde(default)]
  pub tenant_id: String,
  pub title: String,
  #[serde(default)]
  pub created_at: String,
  #[serde(default)]
  pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookSource {
  pub id: String,
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub tenant_id: String,
  pub kind: String,
  pub title: String,
  #[serde(default)]
  pub uri: Option<String>,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub error: Option<String>,
  #[serde(default)]
  pub sha256: Option<String>,
  #[serde(default)]
  pub created_at: String,
  #[serde(default)]
  pub updated_at: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookDetail {
  pub notebook: Notebook,
  #[serde(default)]
  pub sources: Vec<NotebookSource>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookSearchHit {
  pub source_id: String,
  pub chunk_id: String,
  pub content: String,
  #[serde(default)]
  pub score: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookChunk {
  pub chunk_id: String,
  pub source_id: String,
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub ordinal: usize,
  pub content: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookSourceChunks {
  #[serde(default)]
  pub notebook_id: String,
  pub source: NotebookSource,
  #[serde(default)]
  pub raw_text: Option<String>,
  #[serde(default)]
  pub chunks: Vec<NotebookChunk>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverableCitation {
  #[serde(default)]
  pub source_id: String,
  #[serde(default)]
  pub chunk_id: String,
  #[serde(default)]
  pub quote: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookDeliverable {
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub title: String,
  #[serde(default)]
  pub content: Option<serde_json::Value>,
  #[serde(default)]
  pub citations: Vec<DeliverableCitation>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookAskResponse {
  #[serde(default)]
  pub message_id: Option<String>,
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub question: String,
  #[serde(default)]
  pub answer: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub citations: Vec<DeliverableCitation>,
  #[serde(default)]
  pub used_chunk_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookMessage {
  pub id: String,
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub question: String,
  #[serde(default)]
  pub answer: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub citations: Vec<DeliverableCitation>,
  #[serde(default)]
  pub used_chunk_count: usize,
  #[serde(default)]
  pub created_at: Option<String>,
}

impl From<NotebookMessage> for NotebookAskResponse {
  fn from(m: NotebookMessage) -> Self {
    Self {
      message_id: Some(m.id),
      notebook_id: m.notebook_id,
      question: m.question,
      answer: m.answer,
      model: m.model,
      citations: m.citations,
      used_chunk_count: m.used_chunk_count,
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookArtifact {
  #[serde(default)]
  pub kind: String,
  #[serde(default)]
  pub deliverable: Option<NotebookDeliverable>,
  pub id: String,
  pub run_id: String,
  #[serde(default)]
  pub node_id: String,
  pub name: String,
  #[serde(default)]
  pub path_or_url: String,
  #[serde(default)]
  pub mime_type: Option<String>,
  #[serde(default)]
  pub created_at: String,
  #[serde(default)]
  pub size_bytes: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookPlannedItem {
  pub kind: String,
  #[serde(default)]
  pub reason: Option<String>,
  #[serde(default)]
  pub params: serde_json::Value,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookPlanResponse {
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub goal: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub rationale: String,
  #[serde(default)]
  pub items: Vec<NotebookPlannedItem>,
  #[serde(default)]
  pub used_chunk_count: usize,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookGenerateResponse {
  pub run_id: String,
  pub kind: String,
  #[serde(default)]
  pub status: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookBatchGenerateResponse {
  #[serde(default)]
  pub notebook_id: String,
  #[serde(default)]
  pub runs: Vec<NotebookGenerateResponse>,
  #[serde(default)]
  pub per_run_cost_limit_usd: Option<f64>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct NotebookImportResponse {
  pub notebook: Notebook,
  #[serde(default)]
  pub imported_sources: usize,
  #[serde(default)]
  pub imported_chunks: usize,
  #[serde(default)]
  pub imported_deliverables: Option<usize>,
  #[serde(default)]
  pub imported_messages: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunStatusSummary {
  pub id: String,
  #[serde(default)]
  pub status: String,
  #[serde(default)]
  pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrackedGeneratorRun {
  pub run_id: String,
  pub kind: String,
  pub status: String,
  pub completed_nodes: usize,
  pub total_cost_usd: f64,
}

// ── Media Preview Classification ─────────────────────────────────────────────

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MediaPreviewKind {
  Image,
  Audio,
  Video,
  Html,
  Pdf,
  Table,
  Markdown,
  Document,
  Subtitle,
}

impl MediaPreviewKind {
  pub fn badge_label(self) -> &'static str {
    match self {
      Self::Video => "视频 MP4",
      Self::Image => "矢量 / 图像",
      Self::Html => "交互页面 HTML",
      Self::Pdf => "排版文档 PDF",
      Self::Table => "数据表格 CSV/TSV",
      Self::Markdown => "文本 / Markdown",
      Self::Audio => "口播音频",
      Self::Document => "可编辑套件文档",
      Self::Subtitle => "字幕时间轴",
    }
  }

  pub fn wants_text_body(self) -> bool {
    matches!(self, Self::Subtitle | Self::Markdown | Self::Table)
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaPreviewItem {
  pub key: String,
  pub node_id: String,
  pub label: String,
  pub kind: MediaPreviewKind,
  pub api_path: String,
  pub direct_url: String,
  pub mime_type: Option<String>,
  pub size_bytes: Option<u64>,
}

pub fn classify_media_path(file_path: &str, mime_type: Option<&str>) -> Option<MediaPreviewKind> {
  let mime = mime_type.unwrap_or("").to_ascii_lowercase();
  if mime.starts_with("image/") {
    return Some(MediaPreviewKind::Image);
  }
  if mime.starts_with("audio/") {
    return Some(MediaPreviewKind::Audio);
  }
  if mime.starts_with("video/") {
    return Some(MediaPreviewKind::Video);
  }
  if mime.starts_with("text/html") {
    return Some(MediaPreviewKind::Html);
  }
  if mime.contains("pdf") {
    return Some(MediaPreviewKind::Pdf);
  }
  if mime.starts_with("text/csv") || mime.starts_with("text/tab-separated-values") {
    return Some(MediaPreviewKind::Table);
  }
  if mime.contains("subrip") || mime.starts_with("text/vtt") {
    return Some(MediaPreviewKind::Subtitle);
  }
  if mime.contains("presentation")
    || mime.contains("wordprocessing")
    || mime.contains("spreadsheet")
  {
    return Some(MediaPreviewKind::Document);
  }

  let clean = file_path
    .split('?')
    .next()
    .unwrap_or(file_path)
    .split('#')
    .next()
    .unwrap_or(file_path);
  let ext = clean
    .rsplit_once('.')
    .map(|(_, e)| e.to_ascii_lowercase())
    .unwrap_or_default();

  if mime.starts_with("text/markdown")
    || mime.starts_with("application/json")
    || mime.starts_with("application/sql")
    || mime.starts_with("text/plain")
  {
    return match ext.as_str() {
      "csv" | "tsv" => Some(MediaPreviewKind::Table),
      "srt" | "vtt" => Some(MediaPreviewKind::Subtitle),
      _ => Some(MediaPreviewKind::Markdown),
    };
  }

  match ext.as_str() {
    "png" | "jpg" | "jpeg" | "webp" | "gif" | "svg" => Some(MediaPreviewKind::Image),
    "wav" | "mp3" | "ogg" | "m4a" => Some(MediaPreviewKind::Audio),
    "mp4" | "webm" => Some(MediaPreviewKind::Video),
    "html" | "htm" => Some(MediaPreviewKind::Html),
    "pdf" => Some(MediaPreviewKind::Pdf),
    "csv" | "tsv" => Some(MediaPreviewKind::Table),
    "md" | "markdown" | "json" | "sql" | "txt" => Some(MediaPreviewKind::Markdown),
    "pptx" | "docx" | "xlsx" => Some(MediaPreviewKind::Document),
    "srt" | "vtt" => Some(MediaPreviewKind::Subtitle),
    _ => None,
  }
}

pub fn append_inline_query(api_path: &str) -> String {
  if !api_path.contains("/artifacts/") || api_path.ends_with(".zip") {
    return api_path.to_string();
  }
  if api_path.contains("?inline=") || api_path.contains("&inline=") {
    return api_path.to_string();
  }
  if api_path.contains('?') {
    format!("{api_path}&inline=1")
  } else {
    format!("{api_path}?inline=1")
  }
}

pub fn notebook_artifact_to_media_item(artifact: &NotebookArtifact) -> MediaPreviewItem {
  let api_path = format!(
    "/v1/runs/{}/artifacts/{}",
    encode_uri_component(&artifact.run_id),
    encode_uri_component(&artifact.id),
  );
  let inline_path = append_inline_query(&api_path);
  let direct_url = join_api_url(&resolve_api_base(), &inline_path);
  let kind = classify_media_path(&artifact.name, artifact.mime_type.as_deref())
    .unwrap_or(MediaPreviewKind::Document);
  MediaPreviewItem {
    key: format!("{}:{}", artifact.kind, artifact.id),
    node_id: artifact.node_id.clone(),
    label: artifact.name.clone(),
    kind,
    api_path,
    direct_url,
    mime_type: artifact.mime_type.clone(),
    size_bytes: artifact.size_bytes,
  }
}

pub fn format_bytes(size: Option<u64>) -> String {
  match size {
    None => String::new(),
    Some(b) if b < 1024 => format!("{b} B"),
    Some(b) if b < 1024 * 1024 => format!("{:.1} KB", b as f64 / 1024.0),
    Some(b) => format!("{:.2} MB", b as f64 / (1024.0 * 1024.0)),
  }
}

// ── Pure Helpers ─────────────────────────────────────────────────────────────

pub fn encode_base64(input: &[u8]) -> String {
  const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
  for chunk in input.chunks(3) {
    let b0 = chunk[0] as u32;
    let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
    let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
    let n = (b0 << 16) | (b1 << 8) | b2;
    out.push(TABLE[((n >> 18) & 0x3f) as usize] as char);
    out.push(TABLE[((n >> 12) & 0x3f) as usize] as char);
    if chunk.len() > 1 {
      out.push(TABLE[((n >> 6) & 0x3f) as usize] as char);
    } else {
      out.push('=');
    }
    if chunk.len() > 2 {
      out.push(TABLE[(n & 0x3f) as usize] as char);
    } else {
      out.push('=');
    }
  }
  out
}

pub fn dedupe_deliverable_citations(citations: &[DeliverableCitation]) -> Vec<DeliverableCitation> {
  let mut seen = HashSet::new();
  let mut out = Vec::new();
  for cit in citations {
    let chunk_id = cit.chunk_id.trim();
    let quote = cit.quote.trim();
    if quote.is_empty() {
      continue;
    }
    let key = format!("{chunk_id}::{quote}");
    if seen.insert(key) {
      out.push(DeliverableCitation {
        source_id: cit.source_id.clone(),
        chunk_id: chunk_id.to_string(),
        quote: quote.to_string(),
      });
    }
  }
  out
}

pub fn parse_cost_limit_usd_draft(raw: &str) -> Result<Option<f64>, String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return Ok(None);
  }
  let num = trimmed
    .parse::<f64>()
    .map_err(|_| "单次预算上限（USD）必须是大于 0 的有效数字。".to_string())?;
  if !num.is_finite() || num <= 0.0 {
    return Err("单次预算上限（USD）必须是大于 0 的有效数字。".to_string());
  }
  Ok(Some(num))
}

pub fn resolve_scoped_source_ids(
  sources: &[NotebookSource],
  excluded_ids: &HashSet<String>,
) -> Option<Vec<String>> {
  if excluded_ids.is_empty() {
    return None;
  }
  let ready: Vec<&NotebookSource> = sources.iter().filter(|s| s.status == "ready").collect();
  let selected: Vec<String> = ready
    .iter()
    .filter(|s| !excluded_ids.contains(&s.id))
    .map(|s| s.id.clone())
    .collect();
  if selected.len() == ready.len() {
    None
  } else {
    Some(selected)
  }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookAskTurnPayload {
  pub question: String,
  pub answer: String,
}

pub fn build_ask_history_payload(
  turns: &[NotebookAskResponse],
  max_turns: usize,
) -> Vec<NotebookAskTurnPayload> {
  let cleaned: Vec<NotebookAskTurnPayload> = turns
    .iter()
    .map(|t| NotebookAskTurnPayload {
      question: t.question.trim().to_string(),
      answer: t.answer.trim().to_string(),
    })
    .filter(|t| !t.question.is_empty() || !t.answer.is_empty())
    .collect();
  let keep = max_turns.max(1);
  let skip = cleaned.len().saturating_sub(keep);
  cleaned.into_iter().skip(skip).collect()
}

pub fn format_ask_result_as_note_text(
  result: &NotebookAskResponse,
  source_title_by_id: &HashMap<String, String>,
) -> String {
  let base = format!(
    "问题：{}\n\n回答：\n{}",
    result.question.trim(),
    result.answer.trim()
  );
  let citations = dedupe_deliverable_citations(&result.citations);
  if citations.is_empty() {
    return base;
  }
  let lines: Vec<String> = citations
    .iter()
    .map(|cit| {
      let mut parts = Vec::new();
      if !cit.chunk_id.is_empty() {
        parts.push(format!("[{}]", cit.chunk_id));
      }
      if let Some(title) = source_title_by_id.get(&cit.source_id) {
        parts.push(format!("({title})"));
      }
      if parts.is_empty() {
        format!("- “{}”", cit.quote)
      } else {
        format!("- {} “{}”", parts.join(" "), cit.quote)
      }
    })
    .collect();
  format!("{base}\n\n引用来源：\n{}", lines.join("\n"))
}

pub fn source_status_label(status: &str) -> &str {
  match status {
    "ready" => "已就绪",
    "ingesting" => "解析分块中…",
    "pending" => "排队入库…",
    "failed" => "入库失败",
    other => other,
  }
}

pub fn is_run_terminal(status: &str) -> bool {
  matches!(
    status.to_ascii_lowercase().as_str(),
    "succeeded" | "completed" | "failed" | "cancelled" | "canceled"
  )
}

// ── Studio Generator Catalogue (12 built-in notebook generators) ─────────────

pub const MAX_STUDIO_BATCH_ITEMS: usize = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParamFieldKind {
  Text,
  Number { min: i64, max: i64 },
  Checkbox,
}

#[derive(Clone, Copy, Debug)]
pub struct GeneratorParamField {
  pub key: &'static str,
  pub label: &'static str,
  pub kind: ParamFieldKind,
  pub default_str: &'static str,
  pub default_bool: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct StudioGeneratorSpec {
  pub kind: &'static str,
  pub title: &'static str,
  pub deliverable_badge: &'static str,
  pub summary: &'static str,
  pub fields: &'static [GeneratorParamField],
}

pub const STUDIO_GENERATORS: &[StudioGeneratorSpec] = &[
  StudioGeneratorSpec {
    kind: "report",
    title: "溯源研究报告",
    deliverable_badge: "MD · HTML · PDF",
    summary: "规划章节大纲并并行撰写正文，自动附带来源分块引用脚注。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "报告聚焦主题",
        kind: ParamFieldKind::Text,
        default_str: "the main points, with their supporting evidence",
        default_bool: false,
      },
      GeneratorParamField {
        key: "sections",
        label: "章节数量",
        kind: ParamFieldKind::Number { min: 2, max: 10 },
        default_str: "4",
        default_bool: false,
      },
      GeneratorParamField {
        key: "pdf",
        label: "同时排版 PDF（需本机安装 pandoc 与 typst）",
        kind: ParamFieldKind::Checkbox,
        default_str: "false",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "mindmap",
    title: "结构化思维导图",
    deliverable_badge: "SVG · PNG · 交互 HTML",
    summary: "提炼多层级主题树，同时导出矢量图、高清位图与可折叠交互导图页。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "导图梳理主线",
        kind: ParamFieldKind::Text,
        default_str: "the main ideas and how they relate",
        default_bool: false,
      },
      GeneratorParamField {
        key: "depth",
        label: "展开层级深度 (2–4)",
        kind: ParamFieldKind::Number { min: 2, max: 5 },
        default_str: "3",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "flashcards",
    title: "翻转记忆闪卡",
    deliverable_badge: "交互 HTML · JSON",
    summary: "按知识点生成一问一答记忆卡片，产出零依赖可点击翻转复习页。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "闪卡考察重点",
        kind: ParamFieldKind::Text,
        default_str: "the key facts and ideas",
        default_bool: false,
      },
      GeneratorParamField {
        key: "count",
        label: "卡片张数 (5–30)",
        kind: ParamFieldKind::Number { min: 3, max: 30 },
        default_str: "12",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "quiz",
    title: "互动自测题库",
    deliverable_badge: "自测 HTML · JSON",
    summary: "生成带解析与出处引用的多项选择题，支持页面内即时判分与复盘。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "测验出题范围",
        kind: ParamFieldKind::Text,
        default_str: "the key facts and ideas",
        default_bool: false,
      },
      GeneratorParamField {
        key: "count",
        label: "题目数量 (3–20)",
        kind: ParamFieldKind::Number { min: 3, max: 20 },
        default_str: "8",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "infographic",
    title: "核心指标信息长图",
    deliverable_badge: "SVG · PNG",
    summary: "浓缩关键数字指标、分栏要点与核心结论，渲染出版级视觉信息长图。",
    fields: &[GeneratorParamField {
      key: "focus",
      label: "信息图突出重点",
      kind: ParamFieldKind::Text,
      default_str: "the key numbers and ideas",
      default_bool: false,
    }],
  },
  StudioGeneratorSpec {
    kind: "data_table",
    title: "结构化数据提取表",
    deliverable_badge: "XLSX · CSV · 预览 HTML",
    summary: "从资料原文中抽取对比维度、数值指标与结构化事实，附带逐行出处引用并导出电子表格。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "提取维度与对比重点",
        kind: ParamFieldKind::Text,
        default_str: "the key entities, metrics, comparisons, and structured facts",
        default_bool: false,
      },
      GeneratorParamField {
        key: "max_rows",
        label: "最大提取行数 (5–50)",
        kind: ParamFieldKind::Number { min: 5, max: 50 },
        default_str: "20",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "slides",
    title: "演示文稿课件",
    deliverable_badge: "PPTX · 矢量配图",
    summary: "生成带演讲备注与自动渲染 SVG 示意图的结构化演示幻灯片（原生 PPTX / 16:9 PDF）。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "演示核心论点",
        kind: ParamFieldKind::Text,
        default_str: "the main ideas, for a general audience",
        default_bool: false,
      },
      GeneratorParamField {
        key: "slides",
        label: "内容页数",
        kind: ParamFieldKind::Number { min: 3, max: 20 },
        default_str: "6",
        default_bool: false,
      },
      GeneratorParamField {
        key: "figure_languages",
        label: "配图语言",
        kind: ParamFieldKind::Text,
        default_str: "svg",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "audio_overview",
    title: "双人对话播客概览",
    deliverable_badge: "MP3 音频",
    summary: "编写主持人与嘉宾对谈台本，并行合成双声部语音并响度标准化拼接（需 TTS + ffmpeg）。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "对谈讨论焦点",
        kind: ParamFieldKind::Text,
        default_str: "the main ideas, in plain language",
        default_bool: false,
      },
      GeneratorParamField {
        key: "minutes",
        label: "预计时长（分钟）",
        kind: ParamFieldKind::Number { min: 1, max: 15 },
        default_str: "3",
        default_bool: false,
      },
      GeneratorParamField {
        key: "language",
        label: "对谈语言",
        kind: ParamFieldKind::Text,
        default_str: "Chinese",
        default_bool: false,
      },
      GeneratorParamField {
        key: "tts_model",
        label: "TTS 语音模型名",
        kind: ParamFieldKind::Text,
        default_str: "tts-1",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "video_overview",
    title: "多镜头配音视频概览",
    deliverable_badge: "MP4 · SRT 字幕",
    summary: "编排分镜解说词，并行生成插画画面与旁白语音，合成软字幕 MP4 视频（需生图 + TTS + ffmpeg）。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "视频讲解主线",
        kind: ParamFieldKind::Text,
        default_str: "the main ideas, in plain language",
        default_bool: false,
      },
      GeneratorParamField {
        key: "scene_count",
        label: "分镜数量 (4–10)",
        kind: ParamFieldKind::Number { min: 2, max: 12 },
        default_str: "6",
        default_bool: false,
      },
      GeneratorParamField {
        key: "language",
        label: "旁白语言",
        kind: ParamFieldKind::Text,
        default_str: "Chinese",
        default_bool: false,
      },
      GeneratorParamField {
        key: "image_model",
        label: "文生图模型名",
        kind: ParamFieldKind::Text,
        default_str: "gpt-image-2",
        default_bool: false,
      },
      GeneratorParamField {
        key: "tts_model",
        label: "TTS 语音模型名",
        kind: ParamFieldKind::Text,
        default_str: "tts-1",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "illustrated_slides",
    title: "AI 插图与架构图幻灯片",
    deliverable_badge: "HTML · PPTX · PDF",
    summary: "结合笔记本分块规划大纲、编译 D2 架构图与艺术指导插画，输出原生 OOXML 可编辑 PPTX 与 16:9 讲义 PDF。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "演示切入角度",
        kind: ParamFieldKind::Text,
        default_str: "核心原理、架构机制与关键结论",
        default_bool: false,
      },
      GeneratorParamField {
        key: "slide_count",
        label: "幻灯片页数 (4–10)",
        kind: ParamFieldKind::Number { min: 3, max: 10 },
        default_str: "6",
        default_bool: false,
      },
      GeneratorParamField {
        key: "enable_illustrations",
        label: "调用 AI 插画节点生成定制配图",
        kind: ParamFieldKind::Checkbox,
        default_str: "true",
        default_bool: true,
      },
      GeneratorParamField {
        key: "image_model",
        label: "文生图模型名",
        kind: ParamFieldKind::Text,
        default_str: "gemini-3.1-flash-image",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "tech_slides_video",
    title: "1080p 演播级技术精讲视频",
    deliverable_badge: "MP4 · PPTX · PDF",
    summary: "围绕笔记本资料编排双栏渐进动画幻灯片与视线引导口播稿，合成毫秒级卡点 1080p 讲课视频与同步课件。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "精讲聚焦主线",
        kind: ParamFieldKind::Text,
        default_str: "底层机制、关键数据与工程最佳实践",
        default_bool: false,
      },
      GeneratorParamField {
        key: "slide_count",
        label: "正片章节页数 (4–10)",
        kind: ParamFieldKind::Number { min: 3, max: 10 },
        default_str: "6",
        default_bool: false,
      },
      GeneratorParamField {
        key: "tts_model",
        label: "TTS 语音模型名",
        kind: ParamFieldKind::Text,
        default_str: "gemini-2.5-flash-preview-tts",
        default_bool: false,
      },
      GeneratorParamField {
        key: "image_model",
        label: "文生图模型名",
        kind: ParamFieldKind::Text,
        default_str: "gemini-3.1-flash-image",
        default_bool: false,
      },
    ],
  },
  StudioGeneratorSpec {
    kind: "debate_podcast_show",
    title: "三声线多智能体圆桌辩论电台",
    deliverable_badge: "MP3 · 思辨特刊 PDF",
    summary: "正方开拓派、反方审慎派与金牌主持人基于笔记本资料多轮真刀真枪辩论，混音输出广播级三声线播客与特刊 PDF。",
    fields: &[
      GeneratorParamField {
        key: "focus",
        label: "圆桌核心辩题",
        kind: ParamFieldKind::Text,
        default_str: "围绕笔记本资料中的核心命题与潜在权衡展开正反交锋",
        default_bool: false,
      },
      GeneratorParamField {
        key: "turn_count",
        label: "电台对话轮次数 (8–18)",
        kind: ParamFieldKind::Number { min: 8, max: 18 },
        default_str: "12",
        default_bool: false,
      },
      GeneratorParamField {
        key: "tts_model",
        label: "TTS 语音模型名",
        kind: ParamFieldKind::Text,
        default_str: "gemini-2.5-flash-preview-tts",
        default_bool: false,
      },
      GeneratorParamField {
        key: "enable_cover_art",
        label: "生成单集概念封面插画",
        kind: ParamFieldKind::Checkbox,
        default_str: "true",
        default_bool: true,
      },
    ],
  },
];

pub type GeneratorDraftMap = BTreeMap<String, BTreeMap<String, String>>;

pub fn build_initial_generator_drafts() -> GeneratorDraftMap {
  let mut out = BTreeMap::new();
  for spec in STUDIO_GENERATORS {
    let mut entry = BTreeMap::new();
    for field in spec.fields {
      let default_val = match field.kind {
        ParamFieldKind::Checkbox => {
          if field.default_bool {
            "true".to_string()
          } else {
            "false".to_string()
          }
        }
        _ => field.default_str.to_string(),
      };
      entry.insert(field.key.to_string(), default_val);
    }
    out.insert(spec.kind.to_string(), entry);
  }
  out
}

pub fn build_generator_params(
  spec: &StudioGeneratorSpec,
  raw: Option<&BTreeMap<String, String>>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
  let mut params = serde_json::Map::new();
  for field in spec.fields {
    let val = raw.and_then(|m| m.get(field.key)).map(String::as_str);
    match field.kind {
      ParamFieldKind::Checkbox => {
        let checked = val.map(|v| v == "true").unwrap_or(field.default_bool);
        params.insert(field.key.to_string(), serde_json::Value::Bool(checked));
      }
      ParamFieldKind::Number { .. } => {
        let text = val.unwrap_or(field.default_str).trim();
        let parsed = text.parse::<f64>().map_err(|_| {
          format!(
            "「{}」的参数「{}」必须是有效数字。",
            spec.title, field.label
          )
        })?;
        if !parsed.is_finite() {
          return Err(format!(
            "「{}」的参数「{}」必须是有效数字。",
            spec.title, field.label
          ));
        }
        let json_num = if parsed.fract() == 0.0 {
          serde_json::json!(parsed as i64)
        } else {
          serde_json::json!(parsed)
        };
        params.insert(field.key.to_string(), json_num);
      }
      ParamFieldKind::Text => {
        let text = val.unwrap_or(field.default_str).trim();
        if !text.is_empty() {
          params.insert(
            field.key.to_string(),
            serde_json::Value::String(text.to_string()),
          );
        }
      }
    }
  }
  Ok(params)
}

// ── Async Gateway Calls ──────────────────────────────────────────────────────

pub async fn list_notebooks(tenant: &str, token: &str) -> Result<Vec<Notebook>, String> {
  #[derive(Deserialize)]
  struct Envelope {
    #[serde(default)]
    notebooks: Vec<Notebook>,
  }
  let env: Envelope = request_json("GET", "/v1/notebooks", token, tenant, None).await?;
  Ok(env.notebooks)
}

pub async fn create_notebook(title: &str, tenant: &str, token: &str) -> Result<Notebook, String> {
  let body = serde_json::json!({ "title": title });
  request_json("POST", "/v1/notebooks", token, tenant, Some(&body)).await
}

pub async fn get_notebook_detail(
  notebook_id: &str,
  tenant: &str,
  token: &str,
) -> Result<NotebookDetail, String> {
  let path = format!("/v1/notebooks/{}", encode_uri_component(notebook_id));
  request_json("GET", &path, token, tenant, None).await
}

pub async fn rename_notebook(
  notebook_id: &str,
  title: &str,
  tenant: &str,
  token: &str,
) -> Result<Notebook, String> {
  let path = format!("/v1/notebooks/{}", encode_uri_component(notebook_id));
  let body = serde_json::json!({ "title": title });
  request_json("PATCH", &path, token, tenant, Some(&body)).await
}

pub async fn delete_notebook(notebook_id: &str, tenant: &str, token: &str) -> Result<(), String> {
  let path = format!("/v1/notebooks/{}", encode_uri_component(notebook_id));
  request_no_content("DELETE", &path, token, tenant, None).await
}

pub async fn export_notebook_bundle(
  notebook_id: &str,
  tenant: &str,
  token: &str,
) -> Result<serde_json::Value, String> {
  let path = format!("/v1/notebooks/{}/export", encode_uri_component(notebook_id));
  request_json("GET", &path, token, tenant, None).await
}

pub async fn import_notebook_bundle(
  bundle: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookImportResponse, String> {
  request_json("POST", "/v1/notebooks/import", token, tenant, Some(bundle)).await
}

pub async fn create_notebook_source(
  notebook_id: &str,
  payload: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookSource, String> {
  let path = format!(
    "/v1/notebooks/{}/sources",
    encode_uri_component(notebook_id)
  );
  request_json("POST", &path, token, tenant, Some(payload)).await
}

pub async fn patch_notebook_source(
  notebook_id: &str,
  source_id: &str,
  payload: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookSource, String> {
  let path = format!(
    "/v1/notebooks/{}/sources/{}",
    encode_uri_component(notebook_id),
    encode_uri_component(source_id),
  );
  request_json("PATCH", &path, token, tenant, Some(payload)).await
}

pub async fn retry_notebook_source(
  notebook_id: &str,
  source_id: &str,
  tenant: &str,
  token: &str,
) -> Result<NotebookSource, String> {
  let path = format!(
    "/v1/notebooks/{}/sources/{}/retry",
    encode_uri_component(notebook_id),
    encode_uri_component(source_id),
  );
  request_json("POST", &path, token, tenant, None).await
}

pub async fn delete_notebook_source(
  notebook_id: &str,
  source_id: &str,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let path = format!(
    "/v1/notebooks/{}/sources/{}",
    encode_uri_component(notebook_id),
    encode_uri_component(source_id),
  );
  request_no_content("DELETE", &path, token, tenant, None).await
}

pub async fn get_source_chunks(
  notebook_id: &str,
  source_id: &str,
  tenant: &str,
  token: &str,
) -> Result<NotebookSourceChunks, String> {
  let path = format!(
    "/v1/notebooks/{}/sources/{}/chunks",
    encode_uri_component(notebook_id),
    encode_uri_component(source_id),
  );
  request_json("GET", &path, token, tenant, None).await
}

pub async fn get_chunk(
  notebook_id: &str,
  chunk_id: &str,
  tenant: &str,
  token: &str,
) -> Result<NotebookChunk, String> {
  let path = format!(
    "/v1/notebooks/{}/chunks/{}",
    encode_uri_component(notebook_id),
    encode_uri_component(chunk_id),
  );
  request_json("GET", &path, token, tenant, None).await
}

pub async fn search_notebook(
  notebook_id: &str,
  query: &str,
  top_k: usize,
  scoped_source_ids: Option<&[String]>,
  tenant: &str,
  token: &str,
) -> Result<Vec<NotebookSearchHit>, String> {
  #[derive(Deserialize)]
  struct Envelope {
    #[serde(default)]
    chunks: Vec<NotebookSearchHit>,
  }
  let scope_param = match scoped_source_ids {
    Some(ids) if !ids.is_empty() => format!("&source_ids={}", encode_uri_component(&ids.join(","))),
    _ => String::new(),
  };
  let path = format!(
    "/v1/notebooks/{}/search?q={}&top_k={top_k}{scope_param}",
    encode_uri_component(notebook_id),
    encode_uri_component(query),
  );
  let env: Envelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.chunks)
}

pub async fn list_notebook_messages(
  notebook_id: &str,
  tenant: &str,
  token: &str,
) -> Result<Vec<NotebookMessage>, String> {
  #[derive(Deserialize)]
  struct Envelope {
    #[serde(default)]
    messages: Vec<NotebookMessage>,
  }
  let path = format!(
    "/v1/notebooks/{}/messages",
    encode_uri_component(notebook_id)
  );
  let env: Envelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.messages)
}

pub async fn clear_notebook_messages(
  notebook_id: &str,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let path = format!(
    "/v1/notebooks/{}/messages",
    encode_uri_component(notebook_id)
  );
  request_no_content("DELETE", &path, token, tenant, None).await
}

pub async fn ask_notebook(
  notebook_id: &str,
  body: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookAskResponse, String> {
  let path = format!("/v1/notebooks/{}/ask", encode_uri_component(notebook_id));
  request_json("POST", &path, token, tenant, Some(body)).await
}

pub async fn plan_notebook(
  notebook_id: &str,
  body: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookPlanResponse, String> {
  let path = format!("/v1/notebooks/{}/plan", encode_uri_component(notebook_id));
  request_json("POST", &path, token, tenant, Some(body)).await
}

pub async fn generate_notebook(
  notebook_id: &str,
  body: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookGenerateResponse, String> {
  let path = format!(
    "/v1/notebooks/{}/generate",
    encode_uri_component(notebook_id)
  );
  request_json("POST", &path, token, tenant, Some(body)).await
}

pub async fn generate_notebook_batch(
  notebook_id: &str,
  body: &serde_json::Value,
  tenant: &str,
  token: &str,
) -> Result<NotebookBatchGenerateResponse, String> {
  let path = format!(
    "/v1/notebooks/{}/generate-batch",
    encode_uri_component(notebook_id)
  );
  request_json("POST", &path, token, tenant, Some(body)).await
}

pub async fn list_notebook_artifacts(
  notebook_id: &str,
  tenant: &str,
  token: &str,
) -> Result<Vec<NotebookArtifact>, String> {
  #[derive(Deserialize)]
  struct Envelope {
    #[serde(default)]
    artifacts: Vec<NotebookArtifact>,
  }
  let path = format!(
    "/v1/notebooks/{}/artifacts",
    encode_uri_component(notebook_id)
  );
  let env: Envelope = request_json("GET", &path, token, tenant, None).await?;
  Ok(env.artifacts)
}

pub async fn delete_notebook_artifact(
  notebook_id: &str,
  artifact_id: &str,
  tenant: &str,
  token: &str,
) -> Result<(), String> {
  let path = format!(
    "/v1/notebooks/{}/artifacts/{}",
    encode_uri_component(notebook_id),
    encode_uri_component(artifact_id),
  );
  request_no_content("DELETE", &path, token, tenant, None).await
}

pub async fn get_run_status(
  run_id: &str,
  tenant: &str,
  token: &str,
) -> Result<RunStatusSummary, String> {
  let path = format!("/v1/runs/{}", encode_uri_component(run_id));
  let raw: serde_json::Value = request_json("GET", &path, token, tenant, None).await?;
  let rec = raw.get("run").unwrap_or(&raw);
  let id = rec
    .get("id")
    .and_then(|v| v.as_str())
    .unwrap_or(run_id)
    .to_string();
  let status = rec
    .get("status")
    .and_then(|v| v.as_str())
    .unwrap_or("running")
    .to_string();
  let error = rec
    .get("error")
    .and_then(|v| v.as_str())
    .map(ToString::to_string);
  Ok(RunStatusSummary { id, status, error })
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn base64_encoder_matches_rfc4648_vectors() {
    assert_eq!(encode_base64(b""), "");
    assert_eq!(encode_base64(b"f"), "Zg==");
    assert_eq!(encode_base64(b"fo"), "Zm8=");
    assert_eq!(encode_base64(b"foo"), "Zm9v");
    assert_eq!(encode_base64(b"foobar"), "Zm9vYmFy");
    assert_eq!(encode_base64("你好".as_bytes()), "5L2g5aW9");
  }

  #[test]
  fn media_classification_and_inline_query_work() {
    assert_eq!(
      classify_media_path("slides.pptx", None),
      Some(MediaPreviewKind::Document)
    );
    assert_eq!(
      classify_media_path("data.csv", Some("text/plain")),
      Some(MediaPreviewKind::Table)
    );
    assert_eq!(
      classify_media_path("deck.pdf", Some("application/pdf")),
      Some(MediaPreviewKind::Pdf)
    );
    assert_eq!(
      append_inline_query("/v1/runs/r1/artifacts/a1"),
      "/v1/runs/r1/artifacts/a1?inline=1"
    );
    assert_eq!(
      append_inline_query("/v1/runs/r1/artifacts/a1?inline=1"),
      "/v1/runs/r1/artifacts/a1?inline=1"
    );
    assert_eq!(
      append_inline_query("/v1/notebooks/n1/artifacts.zip"),
      "/v1/notebooks/n1/artifacts.zip"
    );
  }

  #[test]
  fn citation_dedupe_cost_limit_and_scope_helpers_work() {
    let citations = vec![
      DeliverableCitation {
        source_id: "s1".to_string(),
        chunk_id: "s1:0".to_string(),
        quote: "alpha".to_string(),
      },
      DeliverableCitation {
        source_id: "s1".to_string(),
        chunk_id: "s1:0".to_string(),
        quote: " alpha ".to_string(),
      },
      DeliverableCitation {
        source_id: "s1".to_string(),
        chunk_id: "s1:1".to_string(),
        quote: " ".to_string(),
      },
    ];
    assert_eq!(dedupe_deliverable_citations(&citations).len(), 1);

    assert_eq!(parse_cost_limit_usd_draft(""), Ok(None));
    assert_eq!(parse_cost_limit_usd_draft("2.5"), Ok(Some(2.5)));
    assert!(parse_cost_limit_usd_draft("0").is_err());
    assert!(parse_cost_limit_usd_draft("-1").is_err());

    let sources = vec![
      NotebookSource {
        id: "s1".to_string(),
        status: "ready".to_string(),
        ..Default::default()
      },
      NotebookSource {
        id: "s2".to_string(),
        status: "ready".to_string(),
        ..Default::default()
      },
    ];
    let mut excluded = HashSet::new();
    assert_eq!(resolve_scoped_source_ids(&sources, &excluded), None);
    excluded.insert("s2".to_string());
    assert_eq!(
      resolve_scoped_source_ids(&sources, &excluded),
      Some(vec!["s1".to_string()])
    );
  }

  #[test]
  fn generator_params_builder_validates_types() {
    let drafts = build_initial_generator_drafts();
    let spec = &STUDIO_GENERATORS[0]; // report
    let built = build_generator_params(spec, drafts.get(spec.kind)).expect("report params");
    assert_eq!(built.get("sections").and_then(|v| v.as_i64()), Some(4));
    assert_eq!(built.get("pdf").and_then(|v| v.as_bool()), Some(false));

    let mut bad = BTreeMap::new();
    bad.insert("sections".to_string(), "not-a-number".to_string());
    assert!(build_generator_params(spec, Some(&bad)).is_err());
  }
}
