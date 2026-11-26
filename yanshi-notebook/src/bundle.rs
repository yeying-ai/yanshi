//! Portable `NotebookExportBundle` DTOs, import validation, ID remapping,
//! deliverable byte synthesis, Markdown brief rendering, and deliverable `.zip`
//! archive packaging.

use std::collections::{HashMap, HashSet};
use std::path::{Path as FsPath, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::ingest::MAX_SOURCE_BYTES;

/// Schema version for portable `NotebookExportBundle` JSON documents.
pub const NOTEBOOK_BUNDLE_VERSION: u32 = 1;
/// Maximum number of sources permitted in an imported bundle.
pub const MAX_IMPORT_SOURCES: usize = 200;
/// Maximum number of chunks per source permitted in an imported bundle.
pub const MAX_IMPORT_CHUNKS_PER_SOURCE: usize = 2_000;

fn default_json_object() -> Value {
  json!({})
}

fn default_ready_status() -> String {
  "ready".to_string()
}

/// Portable notebook metadata inside [`NotebookExportBundle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookMeta {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<DateTime<Utc>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub updated_at: Option<DateTime<Utc>>,
}

/// Portable source chunk entry inside [`ExportedNotebookSource`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedSourceChunk {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub chunk_id: Option<String>,
  #[serde(default)]
  pub ordinal: i32,
  pub content: String,
  #[serde(default = "default_json_object")]
  pub metadata: Value,
}

/// Portable notebook source entry inside [`NotebookExportBundle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookSource {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub kind: String,
  pub title: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub uri: Option<String>,
  #[serde(default = "default_ready_status")]
  pub status: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub error: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub raw_text: Option<String>,
  #[serde(default)]
  pub chunks: Vec<ExportedSourceChunk>,
}

/// Portable deliverable entry inside [`NotebookExportBundle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookDeliverable {
  pub artifact_id: Uuid,
  pub run_id: Uuid,
  pub kind: String,
  pub name: String,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub mime_type: Option<String>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub size_bytes: Option<i64>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub sha256: Option<String>,
  pub created_at: DateTime<Utc>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub deliverable: Option<Value>,
}

/// Portable grounded Q&A turn inside [`NotebookExportBundle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedNotebookMessage {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub id: Option<Uuid>,
  pub question: String,
  pub answer: String,
  #[serde(default)]
  pub model: String,
  #[serde(default)]
  pub citations: Value,
  #[serde(default)]
  pub used_chunk_count: i32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub created_at: Option<DateTime<Utc>>,
}

/// Complete portable notebook archive serialized by `/export` and accepted by `/import`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotebookExportBundle {
  pub version: u32,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub exported_at: Option<DateTime<Utc>>,
  pub notebook: ExportedNotebookMeta,
  #[serde(default)]
  pub sources: Vec<ExportedNotebookSource>,
  #[serde(default)]
  pub deliverables: Vec<ExportedNotebookDeliverable>,
  #[serde(default)]
  pub messages: Vec<ExportedNotebookMessage>,
  #[serde(default)]
  pub markdown_brief: String,
}

/// Request payload for `POST /v1/notebooks/import`, accepting either a bare
/// [`NotebookExportBundle`] or `{ bundle, title? }`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ImportNotebookRequest {
  Wrapped {
    bundle: NotebookExportBundle,
    #[serde(default)]
    title: Option<String>,
  },
  Direct(NotebookExportBundle),
}

/// Validation error returned by [`validate_notebook_import_bundle`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportValidationError {
  BadRequest(String),
  PayloadTooLarge(String),
}

/// Validated notebook source kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotebookSourceKind {
  Url,
  File,
  Text,
  Recording,
}

impl NotebookSourceKind {
  /// Wire representation (`"url"`, `"file"`, `"text"`, `"recording"`).
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Url => "url",
      Self::File => "file",
      Self::Text => "text",
      Self::Recording => "recording",
    }
  }
}

/// Parse a source kind string (`url` / `file` / `text` / `recording`).
pub fn parse_source_kind(raw: &str) -> Option<NotebookSourceKind> {
  match raw.trim() {
    "url" => Some(NotebookSourceKind::Url),
    "file" => Some(NotebookSourceKind::File),
    "text" => Some(NotebookSourceKind::Text),
    "recording" => Some(NotebookSourceKind::Recording),
    _ => None,
  }
}

/// Validated chunk ready to be persisted during notebook import.
#[derive(Debug)]
pub struct ValidatedImportChunk {
  pub old_chunk_id: Option<String>,
  pub old_ordinal: i32,
  pub content: String,
  pub metadata: Value,
}

/// Validated source ready to be persisted during notebook import.
#[derive(Debug)]
pub struct ValidatedImportSource {
  pub original_id: Option<Uuid>,
  pub kind: NotebookSourceKind,
  pub title: String,
  pub uri: Option<String>,
  pub status: String,
  pub error: Option<String>,
  pub sha256: Option<String>,
  pub raw_text: Option<String>,
  pub chunks: Vec<ValidatedImportChunk>,
}

/// Validated grounded Q&A turn ready to be persisted during notebook import.
#[derive(Debug)]
pub struct ValidatedImportMessage {
  pub question: String,
  pub answer: String,
  pub model: String,
  pub citations: Value,
  pub used_chunk_count: i32,
}

/// Validated deliverable metadata ready to be synthesized/restored during import.
#[derive(Debug)]
pub struct ValidatedImportDeliverable {
  pub old_run_id: Uuid,
  pub kind: String,
  pub name: String,
  pub mime_type: Option<String>,
  pub deliverable: Option<Value>,
}

/// Validated notebook import bundle.
#[derive(Debug)]
pub struct ValidatedImportBundle {
  pub title: String,
  pub sources: Vec<ValidatedImportSource>,
  pub messages: Vec<ValidatedImportMessage>,
  pub deliverables: Vec<ValidatedImportDeliverable>,
}

/// Validate a portable [`NotebookExportBundle`] before persisting into DB or local disk.
pub fn validate_notebook_import_bundle(
  bundle: &NotebookExportBundle,
  title_override: Option<&str>,
) -> Result<ValidatedImportBundle, ImportValidationError> {
  if bundle.version != NOTEBOOK_BUNDLE_VERSION {
    return Err(ImportValidationError::BadRequest(format!(
      "unsupported notebook bundle version {} (expected {})",
      bundle.version, NOTEBOOK_BUNDLE_VERSION
    )));
  }
  let resolved_title = title_override
    .map(str::trim)
    .filter(|t| !t.is_empty())
    .unwrap_or_else(|| bundle.notebook.title.trim());
  if resolved_title.is_empty() {
    return Err(ImportValidationError::BadRequest(
      "notebook title must not be empty".to_string(),
    ));
  }
  if bundle.sources.len() > MAX_IMPORT_SOURCES {
    return Err(ImportValidationError::BadRequest(format!(
      "bundle contains {} sources, exceeding limit of {MAX_IMPORT_SOURCES}",
      bundle.sources.len()
    )));
  }

  let mut total_bytes: usize = 0;
  let mut validated_sources = Vec::with_capacity(bundle.sources.len());

  for (idx, src) in bundle.sources.iter().enumerate() {
    let Some(kind) = parse_source_kind(&src.kind) else {
      return Err(ImportValidationError::BadRequest(format!(
        "source #{idx} has unsupported kind '{}': expected url, file, text, or recording",
        src.kind
      )));
    };
    if src.chunks.len() > MAX_IMPORT_CHUNKS_PER_SOURCE {
      return Err(ImportValidationError::BadRequest(format!(
        "source #{idx} contains {} chunks, exceeding limit of {MAX_IMPORT_CHUNKS_PER_SOURCE}",
        src.chunks.len()
      )));
    }
    let source_title = {
      let t = src.title.trim();
      if !t.is_empty() {
        t.to_string()
      } else if let Some(uri) = src.uri.as_deref().map(str::trim).filter(|u| !u.is_empty()) {
        uri.to_string()
      } else {
        format!("source-{}", idx + 1)
      }
    };

    let mut ordered_chunks: Vec<ValidatedImportChunk> = Vec::with_capacity(src.chunks.len());
    for chunk in &src.chunks {
      let content = chunk.content.trim().to_string();
      if content.is_empty() {
        continue;
      }
      total_bytes = total_bytes.saturating_add(content.len());
      if total_bytes > MAX_SOURCE_BYTES {
        return Err(ImportValidationError::PayloadTooLarge(format!(
          "imported chunk text exceeds {MAX_SOURCE_BYTES} bytes"
        )));
      }
      let meta = if chunk.metadata.is_object() {
        chunk.metadata.clone()
      } else {
        json!({})
      };
      ordered_chunks.push(ValidatedImportChunk {
        old_chunk_id: chunk
          .chunk_id
          .as_deref()
          .map(str::trim)
          .filter(|c| !c.is_empty())
          .map(str::to_string),
        old_ordinal: chunk.ordinal,
        content,
        metadata: meta,
      });
    }
    ordered_chunks.sort_by_key(|c| c.old_ordinal);

    validated_sources.push(ValidatedImportSource {
      original_id: src.id,
      kind,
      title: source_title,
      uri: src
        .uri
        .as_deref()
        .map(str::trim)
        .filter(|u| !u.is_empty())
        .map(str::to_string),
      status: src.status.trim().to_ascii_lowercase(),
      error: src
        .error
        .as_deref()
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .map(str::to_string),
      sha256: src
        .sha256
        .as_deref()
        .map(str::trim)
        .filter(|h| !h.is_empty())
        .map(str::to_string),
      raw_text: src
        .raw_text
        .as_deref()
        .filter(|t| !t.trim().is_empty())
        .map(str::to_string),
      chunks: ordered_chunks,
    });
  }

  let mut validated_messages = Vec::new();
  for msg in bundle.messages.iter().take(200) {
    let q = msg.question.trim();
    let a = msg.answer.trim();
    if q.is_empty() || a.is_empty() {
      continue;
    }
    let model = if msg.model.trim().is_empty() {
      "imported".to_string()
    } else {
      msg.model.trim().to_string()
    };
    let citations = if msg.citations.is_array() {
      msg.citations.clone()
    } else {
      json!([])
    };
    validated_messages.push(ValidatedImportMessage {
      question: q.to_string(),
      answer: a.to_string(),
      model,
      citations,
      used_chunk_count: msg.used_chunk_count.max(0),
    });
  }

  let mut validated_deliverables = Vec::new();
  for deliv in bundle.deliverables.iter().take(200) {
    let raw_kind = deliv.kind.trim();
    if raw_kind.is_empty() {
      continue;
    }
    let kind = sanitize_zip_segment(raw_kind, "deliverable");
    let name = {
      let n = deliv.name.trim();
      if n.is_empty() {
        format!("{kind}.json")
      } else {
        n.to_string()
      }
    };
    validated_deliverables.push(ValidatedImportDeliverable {
      old_run_id: deliv.run_id,
      kind,
      name,
      mime_type: deliv
        .mime_type
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string),
      deliverable: deliv.deliverable.clone().filter(Value::is_object),
    });
  }

  Ok(ValidatedImportBundle {
    title: resolved_title.to_string(),
    sources: validated_sources,
    messages: validated_messages,
    deliverables: validated_deliverables,
  })
}

/// Recursively rewrite `source_id` and `chunk_id` fields inside imported JSON payloads.
pub fn remap_imported_json_refs(
  raw: &Value,
  source_id_map: &HashMap<Uuid, Uuid>,
  chunk_id_map: &HashMap<String, String>,
) -> Value {
  match raw {
    Value::Array(arr) => Value::Array(
      arr
        .iter()
        .map(|item| remap_imported_json_refs(item, source_id_map, chunk_id_map))
        .collect(),
    ),
    Value::Object(obj) => {
      let mut next = serde_json::Map::with_capacity(obj.len());
      for (k, v) in obj {
        if k == "source_id"
          && let Some(old_src) = v.as_str().and_then(|s| Uuid::parse_str(s).ok())
          && let Some(new_src) = source_id_map.get(&old_src)
        {
          next.insert(k.clone(), json!(new_src.to_string()));
          continue;
        }
        if k == "chunk_id"
          && let Some(cid) = v.as_str()
        {
          if let Some(mapped) = chunk_id_map.get(cid) {
            next.insert(k.clone(), json!(mapped));
            continue;
          }
          if let Some((prefix, ordinal)) = cid.split_once(':')
            && let Ok(old_src) = Uuid::parse_str(prefix)
            && let Some(new_src) = source_id_map.get(&old_src)
          {
            next.insert(k.clone(), json!(format!("{new_src}:{ordinal}")));
            continue;
          }
        }
        next.insert(
          k.clone(),
          remap_imported_json_refs(v, source_id_map, chunk_id_map),
        );
      }
      Value::Object(next)
    }
    other => other.clone(),
  }
}

/// Whether an imported deliverable artifact can be synthesized into a previewable text file.
pub fn is_synthesizable_text_deliverable(name: &str, mime_type: Option<&str>) -> bool {
  let lower_name = name.to_ascii_lowercase();
  let lower_mime = mime_type.unwrap_or_default().to_ascii_lowercase();
  lower_name.ends_with(".html")
    || lower_name.ends_with(".htm")
    || lower_name.ends_with(".md")
    || lower_name.ends_with(".csv")
    || lower_name.ends_with(".tsv")
    || lower_name.ends_with(".json")
    || lower_mime.starts_with("text/")
    || lower_mime == "application/json"
}

/// Escape minimal HTML special characters (`&`, `<`, `>`, `"`).
pub fn html_escape_min(raw: &str) -> String {
  raw
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// Synthesize previewable bytes (`(filename, mime_type, bytes)`) from an imported deliverable JSON object.
pub fn synthesize_imported_deliverable_bytes(
  kind: &str,
  name: &str,
  mime_type: Option<&str>,
  deliverable: Option<&Value>,
) -> (String, String, Vec<u8>) {
  let safe_base: String = name
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
        c
      } else {
        '_'
      }
    })
    .collect();
  let lower_name = safe_base.to_ascii_lowercase();
  let lower_mime = mime_type.unwrap_or_default().to_ascii_lowercase();
  let title = deliverable
    .and_then(|d| d.get("title"))
    .and_then(Value::as_str)
    .filter(|t| !t.trim().is_empty())
    .unwrap_or(kind);
  let content = deliverable.and_then(|d| d.get("content"));

  if (lower_name.ends_with(".csv") || lower_mime == "text/csv")
    && let Some(cols) = content
      .and_then(|c| c.get("columns"))
      .and_then(Value::as_array)
  {
    let col_keys: Vec<(String, String)> = cols
      .iter()
      .filter_map(|col| {
        if let Some(s) = col.as_str() {
          Some((s.to_string(), s.to_string()))
        } else {
          let key = col.get("key").or_else(|| col.get("name"))?.as_str()?;
          let label = col
            .get("label")
            .or_else(|| col.get("title"))
            .and_then(Value::as_str)
            .unwrap_or(key);
          Some((key.to_string(), label.to_string()))
        }
      })
      .collect();
    let esc_csv = |cell: &str| {
      if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
      } else {
        cell.to_string()
      }
    };
    let mut lines = Vec::new();
    if !col_keys.is_empty() {
      lines.push(
        col_keys
          .iter()
          .map(|(_, label)| esc_csv(label))
          .collect::<Vec<_>>()
          .join(","),
      );
      if let Some(rows) = content
        .and_then(|c| c.get("rows"))
        .and_then(Value::as_array)
      {
        for row in rows {
          let cells = if let Some(obj) = row.as_object() {
            let values_map = obj.get("values").and_then(Value::as_object).unwrap_or(obj);
            col_keys
              .iter()
              .map(|(k, _)| match values_map.get(k) {
                Some(Value::String(s)) => esc_csv(s),
                Some(Value::Null) | None => String::new(),
                Some(other) => esc_csv(&other.to_string()),
              })
              .collect::<Vec<_>>()
          } else if let Some(arr) = row.as_array() {
            arr
              .iter()
              .map(|v| {
                v.as_str()
                  .map(esc_csv)
                  .unwrap_or_else(|| esc_csv(&v.to_string()))
              })
              .collect::<Vec<_>>()
          } else {
            Vec::new()
          };
          if !cells.is_empty() {
            lines.push(cells.join(","));
          }
        }
      }
    }
    let out_name = if lower_name.ends_with(".csv") {
      safe_base
    } else {
      format!("{kind}.csv")
    };
    return (
      out_name,
      "text/csv".to_string(),
      lines.join("\n").into_bytes(),
    );
  }

  if lower_name.ends_with(".md") || lower_mime == "text/markdown" {
    let mut md = format!("# {title}\n\n");
    if let Some(sections) = content
      .and_then(|c| c.get("sections"))
      .and_then(Value::as_array)
    {
      for sec in sections {
        if let Some(h) = sec
          .get("heading")
          .or_else(|| sec.get("title"))
          .and_then(Value::as_str)
        {
          md.push_str(&format!("## {h}\n\n"));
        }
        if let Some(body) = sec
          .get("body")
          .or_else(|| sec.get("content"))
          .and_then(Value::as_str)
        {
          md.push_str(body);
          md.push_str("\n\n");
        }
      }
    } else if let Some(raw_md) = content
      .and_then(|c| c.get("markdown"))
      .and_then(Value::as_str)
    {
      md.push_str(raw_md);
      md.push('\n');
    } else if let Some(deliv) = deliverable {
      md.push_str("```json\n");
      md.push_str(&serde_json::to_string_pretty(deliv).unwrap_or_default());
      md.push_str("\n```\n");
    }
    let out_name = if lower_name.ends_with(".md") {
      safe_base
    } else {
      format!("{kind}.md")
    };
    return (out_name, "text/markdown".to_string(), md.into_bytes());
  }

  if lower_name.ends_with(".html") || lower_name.ends_with(".htm") || lower_mime == "text/html" {
    let mut body_html = String::new();
    if let Some(sections) = content
      .and_then(|c| c.get("sections"))
      .and_then(Value::as_array)
    {
      for sec in sections {
        let h = sec
          .get("heading")
          .or_else(|| sec.get("title"))
          .and_then(Value::as_str)
          .unwrap_or("Section");
        let b = sec
          .get("body")
          .or_else(|| sec.get("content"))
          .and_then(Value::as_str)
          .unwrap_or_default();
        body_html.push_str(&format!(
          "<section><h2>{}</h2><p>{}</p></section>",
          html_escape_min(h),
          html_escape_min(b).replace('\n', "<br/>")
        ));
      }
    } else if let Some(deliv) = deliverable {
      body_html.push_str(&format!(
        "<pre>{}</pre>",
        html_escape_min(&serde_json::to_string_pretty(deliv).unwrap_or_default())
      ));
    }
    let page = format!(
      "<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\"/><title>{}</title>\
       <style>body{{font-family:system-ui,sans-serif;background:#0f172a;color:#e2e8f0;margin:24px;line-height:1.6}}\
       h1{{font-size:1.35rem;margin-bottom:12px}}h2{{font-size:1.05rem;color:#93c5fd}}pre{{white-space:pre-wrap;background:#1e293b;padding:14px;border-radius:8px}}</style>\
       </head><body><h1>{}</h1>{body_html}</body></html>",
      html_escape_min(title),
      html_escape_min(title)
    );
    let out_name = if lower_name.ends_with(".html") || lower_name.ends_with(".htm") {
      safe_base
    } else {
      format!("{kind}.html")
    };
    return (out_name, "text/html".to_string(), page.into_bytes());
  }

  let json_payload = deliverable
    .cloned()
    .unwrap_or_else(|| json!({ "kind": kind, "title": title, "imported_from": name }));
  let stem = FsPath::new(&safe_base)
    .file_stem()
    .and_then(|s| s.to_str())
    .filter(|s| !s.is_empty())
    .unwrap_or(kind);
  (
    format!("{stem}.json"),
    "application/json".to_string(),
    serde_json::to_vec_pretty(&json_payload).unwrap_or_else(|_| b"{}".to_vec()),
  )
}

/// Borrowed source view for [`render_notebook_markdown_brief`].
pub struct BriefSource<'a> {
  pub id: Uuid,
  pub title: &'a str,
  pub kind: &'a str,
  pub status: &'a str,
  pub uri: Option<&'a str>,
}

/// Borrowed chunk view for [`render_notebook_markdown_brief`].
pub struct BriefChunk<'a> {
  pub source_id: Uuid,
  pub chunk_id: &'a str,
  pub ordinal: i32,
}

/// Borrowed artifact view for [`render_notebook_markdown_brief`].
pub struct BriefArtifact<'a> {
  pub run_id: Uuid,
  pub kind: &'a str,
  pub name: &'a str,
  pub deliverable: Option<&'a Value>,
}

/// Borrowed Q&A message view for [`render_notebook_markdown_brief`].
pub struct BriefMessage<'a> {
  pub question: &'a str,
  pub answer: &'a str,
}

/// Render a human-readable Markdown research dossier summarizing a notebook's
/// ingested sources, generated deliverables, verbatim cited quotes, and Q&A turns.
pub fn render_notebook_markdown_brief(
  title: &str,
  sources: &[BriefSource<'_>],
  chunks: &[BriefChunk<'_>],
  artifacts: &[BriefArtifact<'_>],
  messages: &[BriefMessage<'_>],
) -> String {
  let mut chunk_counts: HashMap<Uuid, usize> = HashMap::new();
  let mut chunk_lookup: HashMap<&str, (Uuid, i32)> = HashMap::with_capacity(chunks.len());
  for c in chunks {
    *chunk_counts.entry(c.source_id).or_insert(0) += 1;
    chunk_lookup.insert(c.chunk_id, (c.source_id, c.ordinal));
  }
  let source_titles: HashMap<Uuid, &str> = sources.iter().map(|s| (s.id, s.title)).collect();

  let mut lines = vec![
    format!("# {title}"),
    String::new(),
    format!("## Sources ({})", sources.len()),
  ];
  if sources.is_empty() {
    lines.push("- *(no sources ingested)*".to_string());
  } else {
    for src in sources {
      let count = chunk_counts.get(&src.id).copied().unwrap_or(0);
      let uri_suffix = src
        .uri
        .filter(|u| !u.is_empty() && *u != src.title)
        .map(|u| format!(" — `{u}`"))
        .unwrap_or_default();
      lines.push(format!(
        "- **{}** (`{}`, status: `{}`, {} chunks){}",
        src.title, src.kind, src.status, count, uri_suffix
      ));
    }
  }
  lines.push(String::new());

  // Group artifacts by `(run_id, kind)` so multi-file generators (`report.md`,
  // `report.html`, `report.pdf`) present their shared `Deliverable` citations once.
  let mut group_order: Vec<(Uuid, String)> = Vec::new();
  let mut groups: HashMap<(Uuid, String), Vec<&BriefArtifact<'_>>> = HashMap::new();
  for item in artifacts {
    let key = (item.run_id, item.kind.to_string());
    let entry = groups.entry(key.clone()).or_insert_with(|| {
      group_order.push(key);
      Vec::new()
    });
    entry.push(item);
  }

  lines.push(format!("## Deliverables ({})", group_order.len()));
  if group_order.is_empty() {
    lines.push("- *(no deliverables generated)*".to_string());
  } else {
    for key in group_order {
      let Some(items) = groups.get(&key) else {
        continue;
      };
      let first = items[0];
      let deliverable = items.iter().find_map(|a| a.deliverable);
      let heading = deliverable
        .and_then(|d| d.get("title"))
        .and_then(Value::as_str)
        .filter(|t| !t.trim().is_empty())
        .unwrap_or(first.kind);
      lines.push(String::new());
      lines.push(format!("### {heading} (`{}`)", first.kind));
      let files_list: Vec<String> = items.iter().map(|a| format!("`{}`", a.name)).collect();
      lines.push(format!("- Files: {}", files_list.join(", ")));

      if let Some(citations) = deliverable
        .and_then(|d| d.get("citations"))
        .and_then(Value::as_array)
        .filter(|arr| !arr.is_empty())
      {
        lines.push("- Citations:".to_string());
        let mut seen_quotes: HashSet<(String, String)> = HashSet::new();
        for cit in citations {
          let cid = cit
            .get("chunk_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
          let quote = cit
            .get("quote")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim();
          if cid.is_empty() && quote.is_empty() {
            continue;
          }
          if !seen_quotes.insert((cid.to_string(), quote.to_string())) {
            continue;
          }
          let provenance = if let Some(&(src_id, ordinal)) = chunk_lookup.get(cid) {
            let src_title = source_titles.get(&src_id).copied().unwrap_or("source");
            format!("{src_title} #{ordinal} (`{cid}`)")
          } else {
            format!("`{cid}`")
          };
          if quote.is_empty() {
            lines.push(format!("  - {provenance}"));
          } else {
            lines.push(format!("  - \"{quote}\" — *{provenance}*"));
          }
        }
      }
    }
  }
  if !messages.is_empty() {
    lines.push(String::new());
    lines.push(format!("## Grounded Q&A ({})", messages.len()));
    for (idx, msg) in messages.iter().enumerate() {
      lines.push(String::new());
      lines.push(format!("### Q{}: {}", idx + 1, msg.question));
      lines.push(msg.answer.to_string());
    }
  }
  lines.push(String::new());
  lines.join("\n")
}

/// Sanitize a single ZIP archive path segment to prevent path traversal or invalid characters.
pub fn sanitize_zip_segment(raw: &str, fallback: &str) -> String {
  let cleaned: String = raw
    .chars()
    .map(|c| {
      if c.is_control() || matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
        '_'
      } else {
        c
      }
    })
    .collect::<String>()
    .trim_matches(|c: char| c == '.' || c.is_whitespace())
    .to_string();
  if cleaned.is_empty() {
    fallback.to_string()
  } else {
    cleaned
  }
}

/// Deduplicate an archive entry path (`<dir>/<filename>`) by appending `_2`, `_3`, ... before extension.
pub fn deduplicate_entry_path(seen: &mut HashSet<String>, dir: &str, filename: &str) -> String {
  let primary = format!("{dir}/{filename}");
  if seen.insert(primary.clone()) {
    return primary;
  }
  let (stem, ext) = match filename.rsplit_once('.') {
    Some((s, e)) if !s.is_empty() => (s, Some(e)),
    _ => (filename, None),
  };
  let mut idx = 2usize;
  loop {
    let candidate = match ext {
      Some(e) => format!("{dir}/{stem}_{idx}.{e}"),
      None => format!("{dir}/{stem}_{idx}"),
    };
    if seen.insert(candidate.clone()) {
      return candidate;
    }
    idx += 1;
  }
}

/// Artifact entry descriptor for [`pack_notebook_artifacts_zip_blocking`].
#[derive(Debug, Clone)]
pub struct ZipArtifactItem {
  pub artifact_id: Uuid,
  pub run_id: Uuid,
  pub kind: String,
  pub name: String,
  pub path_or_url: String,
  pub mime_type: Option<String>,
  pub deliverable: Option<Value>,
}

/// Pack readable notebook deliverable files + `BRIEFING.md` + `notebook_artifacts.json`
/// into an anonymous temporary `.zip` file, returning `(file_at_start, total_bytes)`.
pub fn pack_notebook_artifacts_zip_blocking(
  notebook_id: Uuid,
  notebook_title: String,
  brief_markdown: String,
  artifacts: Vec<ZipArtifactItem>,
) -> std::io::Result<Option<(std::fs::File, u64)>> {
  use std::io::{Read, Seek, SeekFrom, Write};

  let tmp = tempfile::tempfile()?;
  let mut writer = zip::ZipWriter::new(tmp);
  let options =
    zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

  let mut seen_paths = HashSet::new();
  seen_paths.insert("BRIEFING.md".to_string());
  seen_paths.insert("notebook_artifacts.json".to_string());

  let mut manifest_items = Vec::new();
  let mut buf = vec![0u8; 64 * 1024];

  for item in artifacts {
    let disk_path = PathBuf::from(&item.path_or_url);
    let mut file = match std::fs::File::open(&disk_path) {
      Ok(f) => f,
      Err(_) => continue,
    };
    let safe_kind = sanitize_zip_segment(&item.kind, "deliverable");
    let safe_name = sanitize_zip_segment(&item.name, "output.bin");
    let entry_path = deduplicate_entry_path(&mut seen_paths, &safe_kind, &safe_name);

    writer
      .start_file(&entry_path, options)
      .map_err(std::io::Error::other)?;
    let mut hasher = Sha256::new();
    let mut size_bytes: u64 = 0;
    loop {
      let n = file.read(&mut buf)?;
      if n == 0 {
        break;
      }
      writer.write_all(&buf[..n])?;
      hasher.update(&buf[..n]);
      size_bytes += n as u64;
    }
    let sha256 = format!("{:x}", hasher.finalize());
    manifest_items.push(json!({
      "artifact_id": item.artifact_id,
      "run_id": item.run_id,
      "kind": item.kind,
      "name": safe_name,
      "entry_path": entry_path,
      "mime_type": item.mime_type,
      "size_bytes": size_bytes,
      "sha256": sha256,
      "deliverable": item.deliverable,
    }));
  }

  if manifest_items.is_empty() {
    return Ok(None);
  }

  writer
    .start_file("BRIEFING.md", options)
    .map_err(std::io::Error::other)?;
  writer.write_all(brief_markdown.as_bytes())?;

  let manifest = json!({
    "notebook_id": notebook_id,
    "title": notebook_title,
    "artifacts": manifest_items,
  });
  let manifest_bytes = serde_json::to_vec_pretty(&manifest).map_err(std::io::Error::other)?;
  writer
    .start_file("notebook_artifacts.json", options)
    .map_err(std::io::Error::other)?;
  writer.write_all(&manifest_bytes)?;

  let mut finished = writer.finish().map_err(std::io::Error::other)?;
  let total_len = finished.seek(SeekFrom::End(0))?;
  finished.seek(SeekFrom::Start(0))?;
  Ok(Some((finished, total_len)))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn remap_imported_json_refs_rewrites_source_and_chunk_ids() {
    let old_src = Uuid::new_v4();
    let new_src = Uuid::new_v4();
    let mut src_map = HashMap::new();
    src_map.insert(old_src, new_src);
    let mut chunk_map = HashMap::new();
    chunk_map.insert("legacy-id".to_string(), format!("{new_src}:0"));

    let input = json!({
      "citations": [
        { "source_id": old_src.to_string(), "chunk_id": format!("{old_src}:2") },
        { "source_id": old_src.to_string(), "chunk_id": "legacy-id" }
      ]
    });
    let remapped = remap_imported_json_refs(&input, &src_map, &chunk_map);
    assert_eq!(remapped["citations"][0]["source_id"], new_src.to_string());
    assert_eq!(remapped["citations"][0]["chunk_id"], format!("{new_src}:2"));
    assert_eq!(remapped["citations"][1]["chunk_id"], format!("{new_src}:0"));
  }

  #[test]
  fn zip_segment_and_deduplication_are_safe() {
    assert_eq!(
      sanitize_zip_segment("../../evil:name?", "fb"),
      "_.._evil_name_"
    );
    assert_eq!(sanitize_zip_segment("...", "fb"), "fb");
    let mut seen = HashSet::new();
    assert_eq!(
      deduplicate_entry_path(&mut seen, "report", "out.md"),
      "report/out.md"
    );
    assert_eq!(
      deduplicate_entry_path(&mut seen, "report", "out.md"),
      "report/out_2.md"
    );
  }
}
