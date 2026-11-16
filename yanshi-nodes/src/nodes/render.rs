//! In-process renderers (Y3.1, `docs/RFC_CONTENT_STUDIO.md` §5): turn the
//! structured content an LLM or upstream node produced into a
//! deliverable file, with no external binary.
//!
//! - `render_table`: rows (JSON array of objects or of arrays) → CSV or
//!   XLSX.
//! - `render_svg`: SVG text → PNG (resvg; system fonts for `<text>`).
//! - `render_markdown`: Markdown → one self-contained HTML page.
//!
//! Each reads its inputs at execute time (YAML `parameters` arrive as
//! inputs, `input_mapping` adds upstream outputs), accepts the content
//! as a JSON value, a JSON-encoded string, or a `FlowValue::File` from an
//! upstream node, and returns `FlowValue::File` under
//! `<id>_output` via the shared artifact writer.

use std::collections::HashMap;

use async_trait::async_trait;
use serde_json::Value;
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};

use crate::common::artifacts::write_artifact;

const XLSX_MIME: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
/// Upper bound on a rendered PNG's pixel count (8192 × 8192): an SVG
/// declaring an enormous canvas must not exhaust memory.
const MAX_PNG_PIXELS: u64 = 8192 * 8192;

fn input_error(node: &str, message: impl std::fmt::Display) -> YanshiError {
  YanshiError::NodeInputError {
    message: format!("{node}: {message}"),
  }
}

/// The text of input `key`: a JSON string, any other JSON value as JSON
/// text, or the contents of a `File` an upstream node produced.
async fn text_input(
  node: &str,
  inputs: &AsyncNodeInputs,
  key: &str,
) -> Result<String, YanshiError> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::String(text))) => Ok(text.clone()),
    Some(FlowValue::Json(other)) => Ok(other.to_string()),
    Some(FlowValue::File { path, .. }) => tokio::fs::read_to_string(path).await.map_err(|e| {
      input_error(
        node,
        format!("cannot read '{key}' from {}: {e}", path.display()),
      )
    }),
    Some(FlowValue::Url { url, .. }) => Err(input_error(
      node,
      format!("'{key}' is a URL ({url}); fetch it with an http node first"),
    )),
    None => Err(input_error(node, format!("missing input '{key}'"))),
  }
}

fn optional_str<'a>(inputs: &'a AsyncNodeInputs, key: &str) -> Option<&'a str> {
  match inputs.get(key) {
    Some(FlowValue::Json(Value::String(s))) if !s.trim().is_empty() => Some(s.trim()),
    _ => None,
  }
}

fn output(node: &str, value: FlowValue) -> HashMap<String, FlowValue> {
  HashMap::from([(format!("{node}_output"), value)])
}

// ── render_table ────────────────────────────────────────────────────────

/// `render_table`: inputs `rows` (required), `format` (`csv` default or
/// `xlsx`), `columns` (optional header order for object rows).
pub struct RenderTableNode {
  id: String,
}

impl RenderTableNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

/// A table: a header (empty for array rows without `columns`) and cells.
struct Table {
  header: Vec<String>,
  rows: Vec<Vec<Value>>,
}

fn table_from(rows: Value, columns: Option<Vec<String>>) -> Result<Table, String> {
  let rows = match rows {
    Value::String(text) => {
      serde_json::from_str(&text).map_err(|e| format!("rows is not JSON: {e}"))?
    }
    other => other,
  };
  let Value::Array(rows) = rows else {
    return Err("rows must be a JSON array".to_string());
  };
  if rows.iter().all(Value::is_object) {
    let header = match columns {
      Some(columns) => columns,
      None => {
        let mut header: Vec<String> = Vec::new();
        for row in &rows {
          if let Value::Object(map) = row {
            for key in map.keys() {
              if !header.contains(key) {
                header.push(key.clone());
              }
            }
          }
        }
        header
      }
    };
    let cells = rows
      .iter()
      .map(|row| {
        header
          .iter()
          .map(|key| row.get(key).cloned().unwrap_or(Value::Null))
          .collect()
      })
      .collect();
    Ok(Table {
      header,
      rows: cells,
    })
  } else if rows.iter().all(Value::is_array) {
    let cells = rows
      .into_iter()
      .map(|row| match row {
        Value::Array(cells) => cells,
        _ => unreachable!("every row was checked to be an array above"),
      })
      .collect();
    Ok(Table {
      header: columns.unwrap_or_default(),
      rows: cells,
    })
  } else {
    Err("rows must be all objects or all arrays".to_string())
  }
}

fn cell_text(value: &Value) -> String {
  match value {
    Value::Null => String::new(),
    Value::String(s) => s.clone(),
    other => other.to_string(),
  }
}

fn table_csv(table: &Table) -> Result<Vec<u8>, String> {
  let mut writer = csv::Writer::from_writer(Vec::new());
  if !table.header.is_empty() {
    writer
      .write_record(&table.header)
      .map_err(|e| e.to_string())?;
  }
  for row in &table.rows {
    writer
      .write_record(row.iter().map(cell_text))
      .map_err(|e| e.to_string())?;
  }
  writer.into_inner().map_err(|e| e.to_string())
}

fn table_xlsx(table: &Table) -> Result<Vec<u8>, String> {
  use rust_xlsxwriter::{Format, Workbook};
  let mut workbook = Workbook::new();
  let sheet = workbook.add_worksheet();
  let bold = Format::new().set_bold();
  let mut first_row: u32 = 0;
  if !table.header.is_empty() {
    for (col, name) in table.header.iter().enumerate() {
      sheet
        .write_string_with_format(0, col_index(col)?, name, &bold)
        .map_err(|e| e.to_string())?;
    }
    first_row = 1;
  }
  for (r, row) in table.rows.iter().enumerate() {
    let r = first_row + u32::try_from(r).map_err(|_| "too many rows for xlsx".to_string())?;
    for (c, value) in row.iter().enumerate() {
      let c = col_index(c)?;
      let written = match value {
        Value::Null => continue,
        Value::Number(n) => match n.as_f64() {
          Some(f) => sheet.write_number(r, c, f).map(|_| ()),
          None => sheet.write_string(r, c, n.to_string()).map(|_| ()),
        },
        Value::Bool(b) => sheet.write_boolean(r, c, *b).map(|_| ()),
        other => sheet.write_string(r, c, cell_text(other)).map(|_| ()),
      };
      written.map_err(|e| e.to_string())?;
    }
  }
  workbook.save_to_buffer().map_err(|e| e.to_string())
}

fn col_index(col: usize) -> Result<u16, String> {
  u16::try_from(col).map_err(|_| "too many columns for xlsx".to_string())
}

#[async_trait]
impl AsyncNode for RenderTableNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let rows = match inputs.get("rows") {
      Some(FlowValue::Json(value)) => value.clone(),
      Some(_) => Value::String(text_input(&self.id, inputs, "rows").await?),
      None => return Err(input_error(&self.id, "missing input 'rows'")),
    };
    let columns = match inputs.get("columns") {
      Some(FlowValue::Json(Value::Array(items))) => Some(items.iter().map(cell_text).collect()),
      _ => None,
    };
    let table = table_from(rows, columns).map_err(|e| input_error(&self.id, e))?;
    let (bytes, mime) = match optional_str(inputs, "format").unwrap_or("csv") {
      "csv" => (table_csv(&table), "text/csv"),
      "xlsx" => (table_xlsx(&table), XLSX_MIME),
      other => {
        return Err(input_error(
          &self.id,
          format!("unknown format '{other}': expected 'csv' or 'xlsx'"),
        ));
      }
    };
    let bytes = bytes.map_err(|e| YanshiError::NodeExecutionFailed {
      message: format!("{}: rendering the table failed: {e}", self.id),
    })?;
    Ok(output(
      &self.id,
      write_artifact(&self.id, &bytes, mime).await?,
    ))
  }
}

// ── render_svg ──────────────────────────────────────────────────────────

/// `render_svg`: inputs `svg` (required), `width` (optional output
/// width in pixels; height follows the aspect ratio) and `format`
/// (`png`, the default; or `svg`: the SVG itself, checked to parse, for
/// a template-built graphic that ships its vector source).
pub struct RenderSvgNode {
  id: String,
}

impl RenderSvgNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

/// Parse `svg` the way [`svg_to_png`] would, without rasterising: the
/// `figure` node's check that LLM-written SVG is renderable.
pub fn check_svg(svg: &str) -> Result<(), String> {
  let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default())
    .map_err(|e| format!("invalid SVG: {e}"))?;
  let size = tree.size();
  if size.width() <= 0.0 || size.height() <= 0.0 {
    return Err("SVG has an empty canvas; set width and height".to_string());
  }
  Ok(())
}

/// SVG text → PNG bytes (also used by `figure`'s optional PNG copy).
pub fn svg_to_png(svg: &str, width: Option<u32>) -> Result<Vec<u8>, String> {
  use resvg::{tiny_skia, usvg};
  // System fonts, loaded once per process: `<text>` renders nothing
  // without them, and scanning the font directories is slow.
  static FONTS: std::sync::OnceLock<std::sync::Arc<usvg::fontdb::Database>> =
    std::sync::OnceLock::new();
  let fonts = FONTS.get_or_init(|| {
    let mut db = usvg::fontdb::Database::new();
    db.load_system_fonts();
    std::sync::Arc::new(db)
  });
  let options = usvg::Options {
    fontdb: fonts.clone(),
    ..usvg::Options::default()
  };
  let tree = usvg::Tree::from_str(svg, &options).map_err(|e| format!("invalid SVG: {e}"))?;
  let size = tree.size();
  let scale = match width {
    Some(w) if w > 0 => w as f32 / size.width(),
    _ => 1.0,
  };
  let out_w = (size.width() * scale).ceil() as u32;
  let out_h = (size.height() * scale).ceil() as u32;
  if out_w == 0 || out_h == 0 || u64::from(out_w) * u64::from(out_h) > MAX_PNG_PIXELS {
    return Err(format!(
      "output size {out_w}x{out_h} is outside 1..={MAX_PNG_PIXELS} pixels"
    ));
  }
  let mut pixmap =
    tiny_skia::Pixmap::new(out_w, out_h).ok_or_else(|| "cannot allocate the canvas".to_string())?;
  resvg::render(
    &tree,
    tiny_skia::Transform::from_scale(scale, scale),
    &mut pixmap.as_mut(),
  );
  pixmap
    .encode_png()
    .map_err(|e| format!("PNG encoding failed: {e}"))
}

#[async_trait]
impl AsyncNode for RenderSvgNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let svg = text_input(&self.id, inputs, "svg").await?;
    match optional_str(inputs, "format").unwrap_or("png") {
      "png" => {}
      "svg" => {
        check_svg(&svg).map_err(|e| input_error(&self.id, e))?;
        return Ok(output(
          &self.id,
          write_artifact(&self.id, svg.as_bytes(), "image/svg+xml").await?,
        ));
      }
      other => {
        return Err(input_error(
          &self.id,
          format!("unknown format '{other}': expected png or svg"),
        ));
      }
    }
    let width = match inputs.get("width") {
      Some(FlowValue::Json(Value::Number(n))) => n.as_u64().and_then(|w| u32::try_from(w).ok()),
      _ => None,
    };
    let png = tokio::task::spawn_blocking(move || svg_to_png(&svg, width))
      .await
      .map_err(|e| YanshiError::NodeExecutionFailed {
        message: format!("{}: SVG render task failed: {e}", self.id),
      })?
      .map_err(|e| input_error(&self.id, e))?;
    Ok(output(
      &self.id,
      write_artifact(&self.id, &png, "image/png").await?,
    ))
  }
}

// ── render_markdown ─────────────────────────────────────────────────────

/// `render_markdown`: inputs `markdown` (required), `title` (optional)
/// and `format` (`html`, the default: one page with its stylesheet
/// inline; or `md`: the Markdown itself as a `.md` file, for a
/// deliverable that ships its source).
pub struct RenderMarkdownNode {
  id: String,
}

impl RenderMarkdownNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

const PAGE_STYLE: &str = "body{max-width:46rem;margin:2rem auto;padding:0 1rem;\
font:16px/1.6 -apple-system,BlinkMacSystemFont,'Segoe UI',Helvetica,Arial,sans-serif;color:#1f2328}\
h1,h2,h3{line-height:1.25}code,pre{font-family:ui-monospace,SFMono-Regular,Menlo,monospace}\
pre{background:#f6f8fa;padding:1rem;overflow:auto;border-radius:6px}\
table{border-collapse:collapse}th,td{border:1px solid #d0d7de;padding:.4rem .8rem}\
blockquote{margin:0;padding:0 1rem;color:#59636e;border-left:.25rem solid #d0d7de}\
img{max-width:100%}";

/// Markdown → a standalone page. Raw HTML in the source is rendered as
/// text, not markup: the page may be served from the gateway's own
/// origin (`GET /v1/runs/{id}/artifacts/{artifact_id}`), and LLM-written
/// Markdown must not be able to inject script there.
fn markdown_page(markdown: &str, title: Option<&str>) -> String {
  use pulldown_cmark::{Event, Options, Parser, html};
  let options = Options::ENABLE_TABLES
    | Options::ENABLE_FOOTNOTES
    | Options::ENABLE_STRIKETHROUGH
    | Options::ENABLE_TASKLISTS;
  let parser = Parser::new_ext(markdown, options).map(|event| match event {
    Event::Html(raw) | Event::InlineHtml(raw) => Event::Text(raw),
    other => other,
  });
  let mut body = String::new();
  html::push_html(&mut body, parser);
  let title = title
    .map(str::to_string)
    .or_else(|| {
      markdown
        .lines()
        .find_map(|line| line.strip_prefix("# ").map(|t| t.trim().to_string()))
    })
    .unwrap_or_else(|| "Document".to_string());
  format!(
    "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{}</title><style>{PAGE_STYLE}</style></head>\n<body>\n{body}</body></html>\n",
    escape_html(&title)
  )
}

fn escape_html(text: &str) -> String {
  text
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

#[async_trait]
impl AsyncNode for RenderMarkdownNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let markdown = text_input(&self.id, inputs, "markdown").await?;
    let (bytes, mime) = match optional_str(inputs, "format").unwrap_or("html") {
      "html" => (
        markdown_page(&markdown, optional_str(inputs, "title")).into_bytes(),
        "text/html",
      ),
      "md" => (markdown.into_bytes(), "text/markdown"),
      other => {
        return Err(input_error(
          &self.id,
          format!("unknown format '{other}': expected 'html' or 'md'"),
        ));
      }
    };
    Ok(output(
      &self.id,
      write_artifact(&self.id, &bytes, mime).await?,
    ))
  }
}

// ── render_mindmap ──────────────────────────────────────────────────────

/// `render_mindmap` (Y4.2): a tree `{"label": "...", "children": [...]}`
/// (or a JSON string of one) → a left-to-right mind map as SVG (the
/// default; `render_svg` turns it into a PNG), or `format: html`, an
/// interactive markmap page. The HTML loads `markmap-autoloader` from a
/// CDN when opened; the SVG is fully self-contained.
pub struct RenderMindmapNode {
  id: String,
}

impl RenderMindmapNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

const MINDMAP_MAX_NODES: usize = 400;
const MINDMAP_MAX_DEPTH: usize = 8;
/// One branch colour per top-level child (Okabe–Ito, colour-blind safe).
const BRANCH_COLOURS: &[&str] = &[
  "#0072B2", "#D55E00", "#009E73", "#CC79A7", "#E69F00", "#56B4E9", "#7A6F00",
];

/// A laid-out node: depth, vertical centre, label and branch.
#[derive(Debug)]
struct Placed {
  depth: usize,
  y: f32,
  label: String,
  parent: Option<usize>,
  branch: usize,
}

fn tree_label(node: &Value) -> Result<String, String> {
  if let Some(s) = node.as_str() {
    let trimmed = s.trim();
    if !trimmed.is_empty() {
      return Ok(trimmed.to_string());
    }
  }
  ["label", "title", "name", "topic", "text", "heading", "node"]
    .iter()
    .find_map(|k| node.get(*k).and_then(Value::as_str))
    .map(|label| label.trim().to_string())
    .filter(|label| !label.is_empty())
    .ok_or_else(|| "every node needs a non-empty \"label\"".to_string())
}

fn tree_children(node: &Value) -> Vec<&Value> {
  node
    .get("children")
    .or_else(|| node.get("items"))
    .or_else(|| node.get("subtopics"))
    .and_then(Value::as_array)
    .map(|arr| arr.iter().filter(|c| tree_label(c).is_ok()).collect())
    .unwrap_or_default()
}

/// Approximate rendered width of `text` at 14px: CJK and other wide
/// characters take about one em, the rest about 0.55 em.
fn label_width(text: &str) -> f32 {
  text
    .chars()
    .map(|c| if (c as u32) >= 0x2E80 { 14.0 } else { 7.7 })
    .sum()
}

/// Lay the tree out: leaves stacked 30px apart, each parent centred on
/// its children. Returns nodes in depth-first order.
fn layout_tree(root: &Value) -> Result<Vec<Placed>, String> {
  fn visit(
    node: &Value,
    depth: usize,
    parent: Option<usize>,
    branch: usize,
    next_leaf: &mut f32,
    out: &mut Vec<Placed>,
  ) -> Result<f32, String> {
    if depth >= MINDMAP_MAX_DEPTH {
      return Err(format!(
        "the tree is deeper than {MINDMAP_MAX_DEPTH} levels"
      ));
    }
    if out.len() >= MINDMAP_MAX_NODES {
      return Err(format!("the tree has more than {MINDMAP_MAX_NODES} nodes"));
    }
    let index = out.len();
    out.push(Placed {
      depth,
      y: 0.0,
      label: tree_label(node)?,
      parent,
      branch,
    });
    let children = tree_children(node);
    let y = if children.is_empty() {
      let y = *next_leaf;
      *next_leaf += 30.0;
      y
    } else {
      let mut ys = Vec::with_capacity(children.len());
      for (i, child) in children.iter().enumerate() {
        let child_branch = if depth == 0 { i } else { branch };
        ys.push(visit(
          child,
          depth + 1,
          Some(index),
          child_branch,
          next_leaf,
          out,
        )?);
      }
      (ys[0] + ys[ys.len() - 1]) / 2.0
    };
    out[index].y = y;
    Ok(y)
  }
  let mut placed = Vec::new();
  let mut next_leaf = 20.0;
  visit(root, 0, None, 0, &mut next_leaf, &mut placed)?;
  Ok(placed)
}

fn mindmap_svg(root: &Value) -> Result<String, String> {
  let placed = layout_tree(root)?;
  let depth_count = placed.iter().map(|p| p.depth).max().unwrap_or(0) + 1;
  // Column x positions: each column as wide as its widest label.
  let mut column_width = vec![0.0f32; depth_count];
  for p in &placed {
    column_width[p.depth] = column_width[p.depth].max(label_width(&p.label) + 24.0);
  }
  let mut column_x = vec![20.0f32; depth_count];
  for d in 1..depth_count {
    column_x[d] = column_x[d - 1] + column_width[d - 1] + 48.0;
  }
  let width = column_x[depth_count - 1] + column_width[depth_count - 1] + 20.0;
  let height = placed.iter().map(|p| p.y).fold(0.0, f32::max) + 30.0;
  let colour = |p: &Placed| {
    if p.depth == 0 {
      "#222222"
    } else {
      BRANCH_COLOURS[p.branch % BRANCH_COLOURS.len()]
    }
  };

  let mut svg = format!(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w:.0}\" height=\"{h:.0}\" viewBox=\"0 0 {w:.0} {h:.0}\" \
font-family=\"-apple-system, 'Segoe UI', 'PingFang SC', 'Noto Sans CJK SC', 'Microsoft YaHei', sans-serif\">\n\
<rect width=\"100%\" height=\"100%\" fill=\"#ffffff\"/>\n",
    w = width,
    h = height
  );
  for p in &placed {
    if let Some(parent) = p.parent.map(|i| &placed[i]) {
      let x1 = column_x[parent.depth] + label_width(&parent.label) + 16.0;
      let x2 = column_x[p.depth] - 4.0;
      let mid = (x1 + x2) / 2.0;
      svg.push_str(&format!(
        "<path d=\"M{x1:.1},{y1:.1} C{mid:.1},{y1:.1} {mid:.1},{y2:.1} {x2:.1},{y2:.1}\" fill=\"none\" stroke=\"{c}\" stroke-width=\"{sw}\" stroke-opacity=\"0.75\"/>\n",
        y1 = parent.y,
        y2 = p.y,
        c = colour(p),
        sw = if p.depth == 1 { 2.5 } else { 1.5 },
      ));
    }
  }
  for p in &placed {
    let x = column_x[p.depth];
    let w = label_width(&p.label) + 12.0;
    let (fill, text_fill, weight) = if p.depth == 0 {
      ("#222222", "#ffffff", "600")
    } else {
      (
        "#ffffff",
        "#1f2328",
        if p.depth == 1 { "600" } else { "400" },
      )
    };
    svg.push_str(&format!(
      "<rect x=\"{x:.1}\" y=\"{ry:.1}\" width=\"{w:.1}\" height=\"22\" rx=\"6\" fill=\"{fill}\" stroke=\"{c}\" stroke-width=\"1.2\"/>\n\
<text x=\"{tx:.1}\" y=\"{ty:.1}\" font-size=\"14\" font-weight=\"{weight}\" fill=\"{text_fill}\">{label}</text>\n",
      ry = p.y - 11.0,
      c = colour(p),
      tx = x + 6.0,
      ty = p.y + 5.0,
      label = escape_html(&p.label),
    ));
  }
  svg.push_str("</svg>\n");
  Ok(svg)
}

/// The tree as nested Markdown headings / list, the markmap input format.
fn mindmap_markdown(root: &Value) -> Result<String, String> {
  fn visit(node: &Value, depth: usize, out: &mut String) -> Result<(), String> {
    let label = tree_label(node)?;
    if depth < 3 {
      out.push_str(&format!("{} {label}\n", "#".repeat(depth + 1)));
    } else {
      out.push_str(&format!("{}- {label}\n", "  ".repeat(depth - 3)));
    }
    for child in tree_children(node) {
      visit(child, depth + 1, out)?;
    }
    Ok(())
  }
  layout_tree(root)?; // same size / depth limits as the SVG
  let mut markdown = String::new();
  visit(root, 0, &mut markdown)?;
  Ok(markdown)
}

fn mindmap_page(root: &Value) -> Result<String, String> {
  let markdown = mindmap_markdown(root)?;
  let title = tree_label(root)?;
  // `</script` inside the template would end it early.
  let safe = markdown.replace("</", "<\\/");
  Ok(format!(
    "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{}</title><style>html,body,.markmap{{margin:0;width:100%;height:100%}}</style>\
<script src=\"https://cdn.jsdelivr.net/npm/markmap-autoloader@0.18.12\"></script></head>\n\
<body><div class=\"markmap\"><script type=\"text/template\">\n{safe}</script></div></body></html>\n",
    escape_html(&title)
  ))
}

#[async_trait]
impl AsyncNode for RenderMindmapNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let tree = match inputs.get("tree") {
      Some(FlowValue::Json(Value::String(text))) => serde_json::from_str(text)
        .map_err(|e| input_error(&self.id, format!("tree is not JSON: {e}")))?,
      Some(FlowValue::Json(value)) => value.clone(),
      Some(_) => serde_json::from_str(&text_input(&self.id, inputs, "tree").await?)
        .map_err(|e| input_error(&self.id, format!("tree is not JSON: {e}")))?,
      None => return Err(input_error(&self.id, "missing input 'tree'")),
    };
    let (text, mime) = match optional_str(inputs, "format").unwrap_or("svg") {
      "svg" => (mindmap_svg(&tree), "image/svg+xml"),
      "html" => (mindmap_page(&tree), "text/html"),
      "md" => (mindmap_markdown(&tree), "text/markdown"),
      other => {
        return Err(input_error(
          &self.id,
          format!("unknown format '{other}': expected svg, html or md"),
        ));
      }
    };
    let text = text.map_err(|e| input_error(&self.id, e))?;
    Ok(output(
      &self.id,
      write_artifact(&self.id, text.as_bytes(), mime).await?,
    ))
  }
}

// ── render_study (flashcards / quiz) ────────────────────────────────────

/// `render_study` (Y6.2): `flashcards` (`[{front, back}]`) or a `quiz`
/// (`[{question, options, answer, explanation}]`, `answer` the 0-based
/// index of the right option) → one self-contained interactive HTML page.
/// Every model-written string is HTML-escaped here; the page's only
/// script is the fixed one below (flip a card / check an answer).
pub struct RenderStudyNode {
  id: String,
}

impl RenderStudyNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

const STUDY_STYLE: &str = "body{max-width:46rem;margin:2rem auto;padding:0 1rem;font:16px/1.5 -apple-system,'Segoe UI','PingFang SC',sans-serif;color:#1f2328}\
.card{border:1px solid #d0d7de;border-radius:10px;padding:1rem 1.25rem;margin:.75rem 0;cursor:pointer}\
.card .back{display:none;color:#0b5394;margin-top:.5rem}.card.open .back{display:block}\
.q{border:1px solid #d0d7de;border-radius:10px;padding:1rem 1.25rem;margin:1rem 0}\
.q label{display:block;margin:.3rem 0;cursor:pointer}.q .why{display:none;margin-top:.5rem}\
.q.done .why{display:block}.right{color:#1a7f37;font-weight:600}.wrong{color:#cf222e}";

const STUDY_SCRIPT: &str = "document.querySelectorAll('.card').forEach(c=>c.addEventListener('click',()=>c.classList.toggle('open')));\
document.querySelectorAll('.q').forEach(q=>q.querySelectorAll('input').forEach(i=>i.addEventListener('change',()=>{\
q.classList.add('done');q.querySelectorAll('label').forEach(l=>{const o=l.querySelector('input');\
l.classList.remove('right','wrong');if(o.dataset.right==='1')l.classList.add('right');else if(o.checked)l.classList.add('wrong');});})));";

fn study_field<'a>(item: &'a Value, key: &str, index: usize) -> Result<&'a str, String> {
  item
    .get(key)
    .and_then(Value::as_str)
    .filter(|s| !s.trim().is_empty())
    .ok_or_else(|| format!("item {index} needs a non-empty \"{key}\""))
}

fn study_page(kind: &str, title: &str, items: &[Value]) -> Result<String, String> {
  if items.is_empty() {
    return Err("there are no items".to_string());
  }
  let mut body = String::new();
  match kind {
    "flashcards" => {
      for (i, card) in items.iter().enumerate() {
        body.push_str(&format!(
          "<div class=\"card\"><div class=\"front\">{}</div><div class=\"back\">{}</div></div>\n",
          escape_html(study_field(card, "front", i)?),
          escape_html(study_field(card, "back", i)?)
        ));
      }
    }
    "quiz" => {
      for (i, q) in items.iter().enumerate() {
        let options = q
          .get("options")
          .and_then(Value::as_array)
          .filter(|o| (2..=8).contains(&o.len()))
          .ok_or_else(|| format!("question {i} needs 2-8 options"))?;
        let answer = q
          .get("answer")
          .and_then(Value::as_u64)
          .and_then(|a| usize::try_from(a).ok())
          .filter(|a| *a < options.len())
          .ok_or_else(|| format!("question {i}: \"answer\" must index one of its options"))?;
        body.push_str(&format!(
          "<div class=\"q\"><p><strong>{}. {}</strong></p>\n",
          i + 1,
          escape_html(study_field(q, "question", i)?)
        ));
        for (j, option) in options.iter().enumerate() {
          let text = option
            .as_str()
            .ok_or_else(|| format!("question {i}: options must be strings"))?;
          body.push_str(&format!(
            "<label><input type=\"radio\" name=\"q{i}\" data-right=\"{}\"> {}</label>\n",
            u8::from(j == answer),
            escape_html(text)
          ));
        }
        if let Some(why) = q.get("explanation").and_then(Value::as_str) {
          body.push_str(&format!("<div class=\"why\">{}</div>", escape_html(why)));
        }
        body.push_str("</div>\n");
      }
    }
    other => {
      return Err(format!(
        "unknown kind '{other}': expected flashcards or quiz"
      ));
    }
  }
  Ok(format!(
    "<!doctype html>\n<html><head><meta charset=\"utf-8\">\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\
<title>{t}</title><style>{STUDY_STYLE}</style></head>\n<body><h1>{t}</h1>\n{body}<script>{STUDY_SCRIPT}</script></body></html>\n",
    t = escape_html(title)
  ))
}

#[async_trait]
impl AsyncNode for RenderStudyNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let kind = optional_str(inputs, "kind")
      .ok_or_else(|| input_error(&self.id, "missing input 'kind' (flashcards or quiz)"))?;
    let items = match inputs.get("items") {
      Some(FlowValue::Json(Value::Array(items))) => items.clone(),
      Some(FlowValue::Json(Value::String(text))) => serde_json::from_str::<Vec<Value>>(text)
        .map_err(|e| input_error(&self.id, format!("items is not a JSON array: {e}")))?,
      _ => {
        return Err(input_error(
          &self.id,
          "missing input 'items' (a JSON array)",
        ));
      }
    };
    let title = optional_str(inputs, "title").unwrap_or(match kind {
      "quiz" => "Quiz",
      _ => "Flashcards",
    });
    let page = study_page(kind, title, &items).map_err(|e| input_error(&self.id, e))?;
    Ok(output(
      &self.id,
      write_artifact(&self.id, page.as_bytes(), "text/html").await?,
    ))
  }
}

// ── render_audio (external: ffmpeg) ────────────────────────────────────

/// `render_audio` (Y4.3): join audio clips in order and loudness-normalise
/// the result (two-pass EBU R128) into one `mp3` / `wav` / `m4a` file.
/// Inputs: `clips` (array of `File` values or paths, in order),
/// `target_lufs` (default -16), `format` (default `mp3`). Outputs
/// `<id>_output` (File) and `<id>_lufs` (measured loudness, `null` for
/// silence, which is left unnormalised).
///
/// Every clip must resolve inside the run's artifact root: in the gateway
/// the workflow is tenant-written, and this node must not become a way
/// to read host files. ffmpeg runs through `common::sandboxed` with read
/// access to that root only. The same operations are exposed to agents
/// by the `media-tools` MCP server (`examples/ecosystem/mcp/media-tools`);
/// this node is the DAG path, which needs no MCP feature or root flag.
pub struct RenderAudioNode {
  id: String,
}

impl RenderAudioNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

/// (extension, MIME type, encoder arguments) per output `format`.
fn audio_format(format: &str) -> Option<(&'static str, &'static str, &'static [&'static str])> {
  Some(match format {
    "mp3" => ("mp3", "audio/mpeg", &["-c:a", "libmp3lame", "-b:a", "192k"]),
    "wav" => ("wav", "audio/wav", &["-c:a", "pcm_s16le"]),
    "m4a" => ("m4a", "audio/mp4", &["-c:a", "aac", "-b:a", "192k"]),
    _ => return None,
  })
}

/// The loudnorm JSON block ffmpeg prints at the end of stderr.
fn loudnorm_stats(stderr: &str) -> Option<serde_json::Map<String, Value>> {
  let start = stderr.rfind("\"input_i\"")?;
  let open = stderr[..start].rfind('{')?;
  let close = open + stderr[open..].find('}')?;
  serde_json::from_str::<Value>(&stderr[open..=close])
    .ok()?
    .as_object()
    .cloned()
}

fn stat_f64(stats: &serde_json::Map<String, Value>, key: &str) -> Option<f64> {
  stats
    .get(key)
    .and_then(Value::as_str)
    .and_then(|v| v.parse::<f64>().ok())
    .filter(|v| v.is_finite())
}

/// A JSON path (a string, or a serialized `File` object with a `path`).
fn json_path(node: &str, value: &Value, what: &str) -> Result<std::path::PathBuf, YanshiError> {
  match value {
    Value::String(path) => Ok(std::path::PathBuf::from(path)),
    Value::Object(file) => file
      .get("path")
      .and_then(Value::as_str)
      .map(std::path::PathBuf::from)
      .ok_or_else(|| input_error(node, format!("each {what} needs a path"))),
    other => Err(input_error(node, format!("{what} {other} is not a path"))),
  }
}

/// Resolve `path` and require it inside the run's artifact root: in the
/// gateway the workflow is tenant-written, and a media node must not
/// become a way to read host files.
fn inside_artifact_root(
  node: &str,
  path: &std::path::Path,
) -> Result<std::path::PathBuf, YanshiError> {
  let root = yanshi_core::run_context::current_artifact_root()
    .ok_or_else(|| input_error(node, "this node runs only inside a workflow run"))?;
  let root = root.canonicalize().unwrap_or(root);
  let resolved = path
    .canonicalize()
    .map_err(|e| input_error(node, format!("{}: {e}", path.display())))?;
  if resolved.starts_with(&root) {
    Ok(resolved)
  } else {
    Err(input_error(
      node,
      format!("{} is outside this run's artifacts", path.display()),
    ))
  }
}

/// The run's artifact root as seen from `node`'s artifact directory, and
/// a fresh scratch directory under the latter (removed by the caller).
async fn media_workdirs(
  node: &str,
  prefix: &str,
) -> Result<(std::path::PathBuf, std::path::PathBuf), YanshiError> {
  let dir = crate::common::artifacts::artifact_dir(node);
  let root = dir
    .parent()
    .and_then(std::path::Path::parent)
    .map(std::path::Path::to_path_buf)
    .ok_or_else(|| input_error(node, "no artifact root"))?;
  let workdir = dir.join(format!(".{prefix}-{}", uuid::Uuid::new_v4()));
  tokio::fs::create_dir_all(&workdir)
    .await
    .map_err(|e| input_error(node, format!("cannot create {}: {e}", workdir.display())))?;
  Ok((root, workdir))
}

fn media_tool(node: &str, name: &str) -> Result<std::path::PathBuf, YanshiError> {
  crate::common::sandboxed::find_tool(name).ok_or_else(|| YanshiError::NodeExecutionFailed {
    message: format!("{node}: {name} is not installed (`yanshi doctor`)"),
  })
}

/// Run an ffmpeg-family `program` under the OS sandbox: read access to
/// the run's artifact `root`, writes confined to `workdir`.
async fn run_media_tool(
  node: &str,
  program: &std::path::Path,
  workdir: &std::path::Path,
  root: &std::path::Path,
  args: Vec<std::ffi::OsString>,
) -> Result<std::process::Output, YanshiError> {
  use crate::common::sandboxed;
  let output = sandboxed::run(sandboxed::SandboxedRun {
    node,
    program: program.to_path_buf(),
    args,
    workdir,
    // Fonts: libass reads them to burn subtitles in (Y8.6).
    read_paths: std::iter::once(root.to_path_buf())
      .chain(sandboxed::font_dirs())
      .collect(),
    write_paths: Vec::new(),
    env: Vec::new(),
    timeout: std::time::Duration::from_secs(600),
  })
  .await?;
  if !output.status.success() {
    let name = program
      .file_name()
      .map(|n| n.to_string_lossy().into_owned())
      .unwrap_or_default();
    return Err(YanshiError::NodeExecutionFailed {
      message: format!(
        "{node}: {name} exited with {}: {}",
        output.status,
        sandboxed::output_tail(&output, 12)
      ),
    });
  }
  Ok(output)
}

fn ffmpeg_args(args: Vec<std::ffi::OsString>) -> Vec<std::ffi::OsString> {
  let mut full: Vec<std::ffi::OsString> =
    ["-hide_banner", "-nostdin", "-y"].map(Into::into).to_vec();
  full.extend(args);
  full
}

impl RenderAudioNode {
  fn clip_paths(&self, inputs: &AsyncNodeInputs) -> Result<Vec<std::path::PathBuf>, YanshiError> {
    let raw: Vec<std::path::PathBuf> = match inputs.get("clips") {
      Some(FlowValue::Json(Value::Array(items))) => items
        .iter()
        .map(|item| json_path(&self.id, item, "clip"))
        .collect::<Result<_, _>>()?,
      Some(FlowValue::File { path, .. }) => vec![path.clone()],
      _ => {
        return Err(input_error(
          &self.id,
          "missing input 'clips' (an array of files)",
        ));
      }
    };
    if raw.is_empty() {
      return Err(input_error(&self.id, "'clips' is empty"));
    }
    raw
      .iter()
      .map(|clip| inside_artifact_root(&self.id, clip))
      .collect()
  }

  async fn ffmpeg(
    &self,
    ffmpeg: &std::path::Path,
    workdir: &std::path::Path,
    root: &std::path::Path,
    args: Vec<std::ffi::OsString>,
  ) -> Result<std::process::Output, YanshiError> {
    run_media_tool(&self.id, ffmpeg, workdir, root, ffmpeg_args(args)).await
  }
}

#[async_trait]
impl AsyncNode for RenderAudioNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let clips = self.clip_paths(inputs)?;
    let format = optional_str(inputs, "format").unwrap_or("mp3");
    let (extension, mime, encoder) = audio_format(format).ok_or_else(|| {
      input_error(
        &self.id,
        format!("unknown format '{format}': expected mp3, wav or m4a"),
      )
    })?;
    let target = match inputs.get("target_lufs") {
      Some(FlowValue::Json(Value::Number(n))) => n.as_f64().unwrap_or(-16.0),
      _ => -16.0,
    };
    let ffmpeg = media_tool(&self.id, "ffmpeg")?;
    let (root, workdir) = media_workdirs(&self.id, "audio").await?;

    let produced = async {
      // 1. Join, re-encoding to one PCM stream.
      let joined = workdir.join("joined.wav");
      let mut args: Vec<std::ffi::OsString> = Vec::new();
      for clip in &clips {
        args.push("-i".into());
        args.push(clip.clone().into());
      }
      let streams: String = (0..clips.len()).map(|i| format!("[{i}:a]")).collect();
      args.extend(
        [
          "-filter_complex".to_string(),
          format!("{streams}concat=n={}:v=0:a=1[a]", clips.len()),
          "-map".to_string(),
          "[a]".to_string(),
        ]
        .map(Into::into),
      );
      args.push(joined.clone().into());
      self.ffmpeg(&ffmpeg, &workdir, &root, args).await?;

      // 2. Measure; 3. normalise (skipped for silence) and encode.
      let base = format!("loudnorm=I={target}:TP=-1.5:LRA=11");
      let measure = self
        .ffmpeg(
          &ffmpeg,
          &workdir,
          &root,
          vec![
            "-i".into(),
            joined.clone().into(),
            "-af".into(),
            format!("{base}:print_format=json").into(),
            "-f".into(),
            "null".into(),
            "-".into(),
          ],
        )
        .await?;
      let measured = loudnorm_stats(&String::from_utf8_lossy(&measure.stderr));
      let out = workdir.join(format!("out.{extension}"));
      let mut encode: Vec<std::ffi::OsString> = vec!["-i".into(), joined.clone().into()];
      let mut lufs = None;
      if let Some(stats) = &measured
        && let (Some(i), Some(tp), Some(lra), Some(thresh), Some(offset)) = (
          stat_f64(stats, "input_i"),
          stat_f64(stats, "input_tp"),
          stat_f64(stats, "input_lra"),
          stat_f64(stats, "input_thresh"),
          stat_f64(stats, "target_offset"),
        )
      {
        encode.push("-af".into());
        encode.push(
          format!(
            "{base}:measured_I={i}:measured_TP={tp}:measured_LRA={lra}:measured_thresh={thresh}:offset={offset}:linear=true:print_format=json"
          )
          .into(),
        );
        lufs = Some(target);
      }
      encode.extend(["-ar", "44100"].map(Into::into));
      encode.extend(encoder.iter().map(Into::into));
      encode.push(out.clone().into());
      let encoded = self.ffmpeg(&ffmpeg, &workdir, &root, encode).await?;
      if lufs.is_some() {
        lufs = loudnorm_stats(&String::from_utf8_lossy(&encoded.stderr))
          .and_then(|stats| stat_f64(&stats, "output_i"))
          .or(lufs);
      }
      let bytes = tokio::fs::read(&out)
        .await
        .map_err(|e| YanshiError::NodeExecutionFailed {
          message: format!("{}: ffmpeg wrote no output: {e}", self.id),
        })?;
      Ok::<_, YanshiError>((bytes, lufs))
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    let (bytes, lufs) = produced?;
    let mut outputs = output(&self.id, write_artifact(&self.id, &bytes, mime).await?);
    outputs.insert(
      format!("{}_lufs", self.id),
      FlowValue::Json(lufs.map_or(Value::Null, |v| serde_json::json!(v))),
    );
    Ok(outputs)
  }
}

// ── render_video (external: ffmpeg) ────────────────────────────────────

/// `render_video` (Y6.4): a narrated slideshow. Input `scenes`, in order:
/// `[{image | video, audio, caption?}]`, each a file of this run. A scene
/// shows its still `image` (or loops its `video` clip) for exactly as
/// long as its `audio` plays. `width` × `height` (default 1280 × 720,
/// even numbers) letterbox every picture. Captions become a soft
/// subtitle track (`mov_text`, toggled in the player) and a separate
/// `.srt`; with `burn_subtitles: true` (Y8.6) they are drawn into the
/// picture instead, which needs ffmpeg's `subtitles` filter (libass). An
/// ffmpeg without it is an error, never a quiet fall back to the track.
///
/// Outputs `<id>_output` (MP4), `<id>_subtitles` (SRT, when any scene
/// has a caption) and `<id>_seconds`. Inputs must be inside the run's
/// artifact root, and ffmpeg / ffprobe run sandboxed, as for
/// `render_audio`.
pub struct RenderVideoNode {
  id: String,
}

impl RenderVideoNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

struct SceneFrame {
  picture: std::path::PathBuf,
  caption: String,
  audio: Option<std::path::PathBuf>,
}

struct Scene {
  picture: std::path::PathBuf,
  moving: bool,
  frames: Vec<SceneFrame>,
  audio: Option<std::path::PathBuf>,
  caption: String,
  pause_after_ms: Option<u64>,
  is_slide_start: bool,
  is_slide_end: bool,
  hide_pip: bool,
}

struct VideoBeat {
  picture: std::path::PathBuf,
  moving: bool,
  frames: Vec<SceneFrame>,
  audio: std::path::PathBuf,
  caption: String,
  pause_secs: f64,
  fade_in: bool,
  fade_out: bool,
  hide_pip: bool,
}

#[derive(Debug, Clone, PartialEq)]
struct TimedCue {
  start: f64,
  end: f64,
  text: String,
}

const VIDEO_FPS: u32 = 25;

/// `HH:MM:SS,mmm`.
fn srt_time(seconds: f64) -> String {
  let ms = (seconds.max(0.0) * 1000.0).round() as u64;
  format!(
    "{:02}:{:02}:{:02},{:03}",
    ms / 3_600_000,
    ms / 60_000 % 60,
    ms / 1000 % 60,
    ms % 1000
  )
}

/// `HH:MM:SS.mmm` for WebVTT.
fn vtt_time(seconds: f64) -> String {
  let ms = (seconds.max(0.0) * 1000.0).round() as u64;
  format!(
    "{:02}:{:02}:{:02}.{:03}",
    ms / 3_600_000,
    ms / 60_000 % 60,
    ms / 1000 % 60,
    ms % 1000
  )
}

fn subtitle_visual_units(s: &str) -> usize {
  s.chars()
    .map(|c| if (c as u32) >= 0x2E80 { 2 } else { 1 })
    .sum()
}

/// Wrap long single-line subtitle text at a balanced clause boundary so burned/player captions stay <= 2 lines.
fn wrap_subtitle_line(sentence: &str, max_halfwidth_units: usize) -> String {
  let trimmed = sentence.trim();
  let total_units = subtitle_visual_units(trimmed);
  if total_units <= max_halfwidth_units || trimmed.contains('\n') {
    return trimmed.to_string();
  }
  let chars: Vec<char> = trimmed.chars().collect();
  let count = chars.len();
  let mid = count / 2;
  let mut best_split: Option<usize> = None;
  let mut best_dist = usize::MAX;
  for (idx, &ch) in chars.iter().enumerate() {
    let is_break = matches!(ch, '，' | '、' | '：' | '；' | ',' | ':' | ';')
      || (ch == ' ' && idx > 6 && idx + 6 < count);
    if is_break && idx >= 5 && idx + 5 < count {
      let dist = idx.abs_diff(mid);
      if dist < best_dist {
        best_dist = dist;
        best_split = Some(idx + 1);
      }
    }
  }
  if let Some(split_at) = best_split {
    let left: String = chars[..split_at].iter().collect();
    let right: String = chars[split_at..].iter().collect();
    format!("{}\n{}", left.trim(), right.trim())
  } else {
    trimmed.to_string()
  }
}

/// Split a caption into sentences (Latin and CJK end marks), so one
/// long narration becomes several short cues.
fn caption_sentences(caption: &str) -> Vec<String> {
  let chars: Vec<char> = caption.chars().collect();
  let mut out = Vec::new();
  let mut current = String::new();
  for (i, &c) in chars.iter().enumerate() {
    current.push(c);
    let cjk_end = matches!(c, '。' | '！' | '？' | '；');
    // `.!?` ends a sentence only before whitespace and a word that does
    // not start lowercase or with a digit, so "e.g. 3.5 m" stays whole.
    let latin_end = matches!(c, '.' | '!' | '?')
      && match chars.get(i + 1) {
        None => true,
        Some(next) if next.is_whitespace() => chars[i + 1..]
          .iter()
          .find(|n| !n.is_whitespace())
          .is_none_or(|n| !(n.is_lowercase() || n.is_ascii_digit())),
        Some(_) => false,
      };
    if cjk_end || latin_end {
      let sentence = current.trim().to_string();
      if !sentence.is_empty() {
        out.push(sentence);
      }
      current.clear();
    }
  }
  let rest = current.trim();
  if !rest.is_empty() {
    out.push(rest.to_string());
  }
  out
}

/// SRT for scenes of the given durations: each caption's sentences
/// share its scene's time in proportion to their length.
#[cfg(test)]
fn build_srt(captions: &[&str], durations: &[f64]) -> String {
  let mut windows = Vec::with_capacity(captions.len());
  let mut cursor = 0.0;
  for (&cap, &dur) in captions.iter().zip(durations) {
    windows.push((cap, cursor, cursor + dur));
    cursor += dur;
  }
  cues_to_srt(&build_timed_cues(&windows))
}

/// Build subtitle cues anchored strictly to `[speech_start, speech_end]` windows
/// so silent breathing pauses between steps/slides do not hold stale captions.
fn build_timed_cues(windows: &[(&str, f64, f64)]) -> Vec<TimedCue> {
  let mut cues = Vec::new();
  for &(caption, start, end) in windows {
    let trimmed = caption.trim();
    if trimmed.is_empty() || end <= start {
      continue;
    }
    let duration = (end - start).max(0.05);
    let sentences = caption_sentences(trimmed);
    let total: usize = sentences.iter().map(|s| s.chars().count().max(1)).sum();
    let mut t = start;
    for (idx, sentence) in sentences.iter().enumerate() {
      let share = if idx + 1 == sentences.len() {
        (end - t).max(0.05)
      } else {
        duration * sentence.chars().count().max(1) as f64 / total.max(1) as f64
      };
      cues.push(TimedCue {
        start: t,
        end: t + share,
        text: wrap_subtitle_line(sentence, 54),
      });
      t += share;
    }
  }
  cues
}

fn cues_to_srt(cues: &[TimedCue]) -> String {
  let mut srt = String::new();
  for (idx, cue) in cues.iter().enumerate() {
    srt.push_str(&format!(
      "{}\n{} --> {}\n{}\n\n",
      idx + 1,
      srt_time(cue.start),
      srt_time(cue.end),
      cue.text
    ));
  }
  srt
}

fn cues_to_vtt(cues: &[TimedCue]) -> String {
  let mut vtt = String::from("WEBVTT\n\n");
  for (idx, cue) in cues.iter().enumerate() {
    vtt.push_str(&format!(
      "{}\n{} --> {}\n{}\n\n",
      idx + 1,
      vtt_time(cue.start),
      vtt_time(cue.end),
      cue.text
    ));
  }
  vtt
}

fn ass_time(seconds: f64) -> String {
  let cs = (seconds.max(0.0) * 100.0).round() as u64;
  let h = cs / 360_000;
  let m = (cs % 360_000) / 6_000;
  let s = (cs % 6_000) / 100;
  let c = cs % 100;
  format!("{h}:{m:02}:{s:02}.{c:02}")
}

fn cues_to_ass(cues: &[TimedCue], width: u32, height: u32, margin_l: u32, margin_r: u32) -> String {
  let font_sz = ((height as f64 * 0.031).round() as u32).clamp(16, 44);
  let margin_v = ((height as f64 * 0.024).round() as u32).max(12);
  let pad = ((height as f64 * 0.010).round() as u32).clamp(6, 14);
  let mut ass = format!(
    "[Script Info]\n\
     ScriptType: v4.00+\n\
     PlayResX: {width}\n\
     PlayResY: {height}\n\
     WrapStyle: 0\n\
     ScaledBorderAndShadow: yes\n\n\
     [V4+ Styles]\n\
     Format: Name, Fontname, Fontsize, PrimaryColour, SecondaryColour, OutlineColour, BackColour, Bold, Italic, Underline, StrikeOut, ScaleX, ScaleY, Spacing, Angle, BorderStyle, Outline, Shadow, Alignment, MarginL, MarginR, MarginV, Encoding\n\
     Style: Default,sans-serif,{font_sz},&H00F8FAFC,&H000000FF,&H780B1120,&H780B1120,-1,0,0,0,100,100,0,0,3,{pad},0,2,{margin_l},{margin_r},{margin_v},1\n\n\
     [Events]\n\
     Format: Layer, Start, End, Style, Name, MarginL, MarginR, MarginV, Effect, Text\n"
  );
  for cue in cues {
    let escaped = cue
      .text
      .replace('\\', "\\\\")
      .replace('{', "\\{")
      .replace('}', "\\}")
      .replace('\n', "\\N");
    ass.push_str(&format!(
      "Dialogue: 0,{},{},Default,,0,0,0,,{}\n",
      ass_time(cue.start),
      ass_time(cue.end),
      escaped
    ));
  }
  ass
}

/// Render crisp vector grayscale alpha mask + studio accent border ring PNGs for PiP presenter overlay.
fn render_pip_overlay_chrome(pip_w: u32, pip_h: u32) -> Result<(Vec<u8>, Vec<u8>), String> {
  let rx = (pip_h as f32 * 0.09).clamp(10.0, 22.0);
  let mask_svg = format!(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pip_w}\" height=\"{pip_h}\" viewBox=\"0 0 {pip_w} {pip_h}\">\
       <rect width=\"{pip_w}\" height=\"{pip_h}\" fill=\"#000000\"/>\
       <rect x=\"0\" y=\"0\" width=\"{pip_w}\" height=\"{pip_h}\" rx=\"{rx:.1}\" fill=\"#FFFFFF\"/>\
     </svg>"
  );
  let mask_png = svg_to_png(&mask_svg, Some(pip_w))?;

  let bw = pip_w + 8;
  let bh = pip_h + 8;
  let brx = rx + 3.0;
  let border_svg = format!(
    "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{bw}\" height=\"{bh}\" viewBox=\"0 0 {bw} {bh}\">\
       <defs>\
         <linearGradient id=\"pipRing\" x1=\"0\" y1=\"0\" x2=\"1\" y2=\"1\">\
           <stop offset=\"0%\" stop-color=\"#F97316\"/>\
           <stop offset=\"55%\" stop-color=\"#38BDF8\"/>\
           <stop offset=\"100%\" stop-color=\"#A78BFA\"/>\
         </linearGradient>\
       </defs>\
       <rect x=\"2.5\" y=\"2.5\" width=\"{:.1}\" height=\"{:.1}\" rx=\"{brx:.1}\" fill=\"none\" stroke=\"url(#pipRing)\" stroke-width=\"3\"/>\
     </svg>",
    bw as f32 - 5.0,
    bh as f32 - 5.0
  );
  let border_png = svg_to_png(&border_svg, Some(bw))?;
  Ok((mask_png, border_png))
}

/// Whether `ffmpeg -filters` output lists the libass `subtitles` filter
/// (a line like ` ... subtitles         V->V       Render text subtitles ...`).
fn has_subtitles_filter(listing: &str) -> bool {
  listing
    .lines()
    .any(|line| line.split_whitespace().nth(1) == Some("subtitles"))
}

impl RenderVideoNode {
  fn optional_artifact_file(
    &self,
    inputs: &AsyncNodeInputs,
    key: &str,
  ) -> Result<Option<std::path::PathBuf>, YanshiError> {
    let Some(val) = inputs.get(key) else {
      return Ok(None);
    };
    match val {
      FlowValue::Json(Value::Null) => Ok(None),
      FlowValue::Json(Value::String(s)) if s.trim().is_empty() => Ok(None),
      FlowValue::File { path, .. } => {
        if path.as_os_str().is_empty() {
          Ok(None)
        } else {
          Ok(Some(inside_artifact_root(&self.id, path)?))
        }
      }
      FlowValue::Json(v) => {
        let p = json_path(&self.id, v, key)?;
        if p.as_os_str().is_empty() {
          Ok(None)
        } else {
          Ok(Some(inside_artifact_root(&self.id, &p)?))
        }
      }
      _ => Ok(None),
    }
  }

  fn optional_ms(&self, inputs: &AsyncNodeInputs, key: &str, default_ms: u64) -> u64 {
    match inputs.get(key) {
      Some(FlowValue::Json(Value::Number(n))) => n.as_u64().unwrap_or(default_ms).min(10_000),
      Some(FlowValue::Json(Value::String(s))) => {
        s.trim().parse::<u64>().unwrap_or(default_ms).min(10_000)
      }
      _ => default_ms,
    }
  }

  fn scenes(&self, inputs: &AsyncNodeInputs) -> Result<Vec<Scene>, YanshiError> {
    let items = match inputs.get("scenes") {
      Some(FlowValue::Json(Value::Array(items))) if !items.is_empty() => items,
      _ => {
        return Err(input_error(
          &self.id,
          "missing input 'scenes' (a non-empty array of {image | video | frames, audio, caption})",
        ));
      }
    };
    let n_items = items.len();
    items
      .iter()
      .enumerate()
      .map(|(i, item)| {
        let mut frames = Vec::new();
        if let Some(Value::Array(raw_frames)) = item.get("frames") {
          for (fi, frame_item) in raw_frames.iter().enumerate() {
            let img_val = frame_item.get("image").ok_or_else(|| {
              input_error(&self.id, format!("scene {i} frame {fi} needs an image"))
            })?;
            let pic = inside_artifact_root(&self.id, &json_path(&self.id, img_val, "picture")?)?;
            let cap = frame_item
              .get("caption")
              .and_then(Value::as_str)
              .unwrap_or_default()
              .trim()
              .to_string();
            let frame_audio = match frame_item.get("audio") {
              Some(a) if !a.is_null() => Some(inside_artifact_root(
                &self.id,
                &json_path(&self.id, a, "audio")?,
              )?),
              _ => None,
            };
            frames.push(SceneFrame {
              picture: pic,
              caption: cap,
              audio: frame_audio,
            });
          }
        }

        let (picture, moving) = if let Some(first) = frames.first() {
          (first.picture.clone(), false)
        } else {
          let (raw_pic, mv) = match (item.get("image"), item.get("video")) {
            (_, Some(video))
              if !video.is_null() && video.as_str().is_none_or(|s| !s.trim().is_empty()) =>
            {
              (video, true)
            }
            (Some(image), _) if !image.is_null() => (image, false),
            _ => {
              return Err(input_error(
                &self.id,
                format!("scene {i} needs an image, video, or non-empty frames"),
              ));
            }
          };
          (
            inside_artifact_root(&self.id, &json_path(&self.id, raw_pic, "picture")?)?,
            mv,
          )
        };

        let all_frames_have_audio = !frames.is_empty() && frames.iter().all(|f| f.audio.is_some());
        let scene_audio = match item.get("audio") {
          Some(a) if !a.is_null() => Some(inside_artifact_root(
            &self.id,
            &json_path(&self.id, a, "audio")?,
          )?),
          _ if all_frames_have_audio => None,
          _ => return Err(input_error(&self.id, format!("scene {i} needs audio"))),
        };

        let pause_after_ms = item
          .get("pause_after_ms")
          .and_then(Value::as_u64)
          .map(|v| v.min(10_000));
        let is_slide_start = item
          .get("is_slide_start")
          .and_then(Value::as_bool)
          .unwrap_or(true);
        let is_slide_end = item
          .get("is_slide_end")
          .or_else(|| item.get("transition_after"))
          .and_then(Value::as_bool)
          .unwrap_or(i + 1 < n_items || pause_after_ms.is_some());
        let hide_pip = item
          .get("hide_pip")
          .and_then(Value::as_bool)
          .unwrap_or(false);

        Ok(Scene {
          picture,
          moving,
          frames,
          audio: scene_audio,
          caption: item
            .get("caption")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string(),
          pause_after_ms,
          is_slide_start,
          is_slide_end,
          hide_pip,
        })
      })
      .collect()
  }

  fn expand_beats(
    &self,
    scenes: Vec<Scene>,
    step_pause_ms: u64,
    slide_pause_ms: u64,
    fade_transitions: bool,
  ) -> Vec<VideoBeat> {
    let mut beats = Vec::new();
    for scene in scenes {
      if !scene.frames.is_empty() && scene.frames.iter().all(|f| f.audio.is_some()) {
        let nf = scene.frames.len();
        for (fi, frame) in scene.frames.into_iter().enumerate() {
          let Some(audio) = frame.audio else {
            continue;
          };
          let is_first = fi == 0;
          let is_last = fi + 1 == nf;
          let pause_ms = if is_last {
            scene.pause_after_ms.unwrap_or(slide_pause_ms)
          } else {
            step_pause_ms
          };
          let caption = if frame.caption.is_empty() {
            scene.caption.clone()
          } else {
            frame.caption
          };
          beats.push(VideoBeat {
            picture: frame.picture,
            moving: false,
            frames: Vec::new(),
            audio,
            caption,
            pause_secs: pause_ms as f64 / 1000.0,
            fade_in: fade_transitions && is_first,
            fade_out: fade_transitions && is_last,
            hide_pip: scene.hide_pip,
          });
        }
      } else if let Some(audio) = scene.audio {
        let pause_ms = scene.pause_after_ms.unwrap_or(if scene.is_slide_end {
          slide_pause_ms
        } else {
          step_pause_ms
        });
        beats.push(VideoBeat {
          picture: scene.picture,
          moving: scene.moving,
          frames: scene.frames,
          audio,
          caption: scene.caption,
          pause_secs: pause_ms as f64 / 1000.0,
          fade_in: fade_transitions && scene.is_slide_start,
          fade_out: fade_transitions && scene.is_slide_end,
          hide_pip: scene.hide_pip,
        });
      }
    }
    let total = beats.len();
    for (idx, b) in beats.iter_mut().enumerate() {
      if idx == 0 {
        b.fade_in = false;
      }
      if idx + 1 == total {
        b.fade_out = false;
      }
    }
    beats
  }

  fn dimension(
    &self,
    inputs: &AsyncNodeInputs,
    key: &str,
    default: u32,
  ) -> Result<u32, YanshiError> {
    let value = match inputs.get(key) {
      Some(FlowValue::Json(Value::Number(n))) => n
        .as_u64()
        .and_then(|v| u32::try_from(v).ok())
        .ok_or_else(|| input_error(&self.id, format!("'{key}' must be a positive integer")))?,
      _ => default,
    };
    if !(16..=4096).contains(&value) || value % 2 != 0 {
      return Err(input_error(
        &self.id,
        format!("'{key}' must be an even number of pixels in 16..=4096"),
      ));
    }
    Ok(value)
  }
}

#[async_trait]
impl AsyncNode for RenderVideoNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let scenes = self.scenes(inputs)?;
    let width = self.dimension(inputs, "width", 1280)?;
    let height = self.dimension(inputs, "height", 720)?;
    let step_pause_ms = self.optional_ms(inputs, "step_pause_ms", 0);
    let slide_pause_ms = self.optional_ms(inputs, "slide_pause_ms", 0);
    let fade_transitions = matches!(
      optional_str(inputs, "transition"),
      Some("fade" | "crossfade" | "smooth")
    );
    let pip_video = self.optional_artifact_file(inputs, "pip_video")?;
    let pip_image = self.optional_artifact_file(inputs, "pip_image")?;
    let pip_source: Option<(std::path::PathBuf, bool)> = match (pip_video, pip_image) {
      (Some(v), _) => Some((v, true)),
      (None, Some(img)) => Some((img, false)),
      (None, None) => None,
    };
    let pip_position = optional_str(inputs, "pip_position").unwrap_or("bottom_right");
    let pip_scale = match inputs.get("pip_scale") {
      Some(FlowValue::Json(Value::Number(n))) => n.as_f64().unwrap_or(0.185).clamp(0.12, 0.35),
      _ => 0.185,
    };

    let beats = self.expand_beats(scenes, step_pause_ms, slide_pause_ms, fade_transitions);
    if beats.is_empty() {
      return Err(input_error(&self.id, "scenes produced no playable beats"));
    }

    let ffmpeg = media_tool(&self.id, "ffmpeg")?;
    let ffprobe = media_tool(&self.id, "ffprobe")?;
    let burn = match inputs.get("burn_subtitles") {
      Some(FlowValue::Json(Value::Bool(b))) => *b,
      Some(FlowValue::Json(Value::String(s))) => {
        matches!(s.trim().to_ascii_lowercase().as_str(), "true" | "1" | "yes")
      }
      _ => false,
    };
    let (root, workdir) = media_workdirs(&self.id, "video").await?;

    let produced = async {
      if burn {
        let filters = run_media_tool(
          &self.id,
          &ffmpeg,
          &workdir,
          &root,
          vec!["-hide_banner".into(), "-filters".into()],
        )
        .await?;
        if !has_subtitles_filter(&String::from_utf8_lossy(&filters.stdout)) {
          return Err(input_error(
            &self.id,
            "burn_subtitles needs an ffmpeg built with libass (its `subtitles` filter), and this \
             one has none; leave burn_subtitles off for a soft subtitle track",
          ));
        }
      }

      // 1. Probe exact speech duration of every beat's audio.
      let mut speech_durations = Vec::with_capacity(beats.len());
      for beat in &beats {
        let probe = run_media_tool(
          &self.id,
          &ffprobe,
          &workdir,
          &root,
          vec![
            "-v".into(),
            "error".into(),
            "-show_entries".into(),
            "format=duration".into(),
            "-of".into(),
            "csv=p=0".into(),
            beat.audio.clone().into(),
          ],
        )
        .await?;
        let seconds = String::from_utf8_lossy(&probe.stdout)
          .trim()
          .parse::<f64>()
          .ok()
          .filter(|s| s.is_finite() && *s > 0.0)
          .ok_or_else(|| {
            input_error(
              &self.id,
              format!("{} has no measurable duration", beat.audio.display()),
            )
          })?;
        speech_durations.push(seconds);
      }

      // 2. Encode one normalized segment per beat (exact speech duration + optional breathing pause,
      // plus optional slide-boundary dip-to-slate fade with zero audio/subtitle drift).
      let base_fit = format!(
        "scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:color=0x0B1120,setsar=1,fps={VIDEO_FPS}"
      );
      let mut list = String::new();
      let mut cue_windows: Vec<(&str, f64, f64)> = Vec::new();
      let mut pip_visible_ranges: Vec<(f64, f64)> = Vec::new();
      let mut timeline_cursor = 0.0_f64;

      for (i, (beat, &speech_secs)) in beats.iter().zip(&speech_durations).enumerate() {
        let total_seg_secs = speech_secs + beat.pause_secs;
        let segment = workdir.join(format!("scene-{i}.mp4"));

        let mut args: Vec<std::ffi::OsString> = if beat.frames.len() >= 2 {
          let total_weight: usize = beat
            .frames
            .iter()
            .map(|f| f.caption.chars().count().max(8))
            .sum();
          let mut frame_list = String::new();
          let mut sub_cursor = timeline_cursor;
          for (fi, frame) in beat.frames.iter().enumerate() {
            let speech_share =
              speech_secs * frame.caption.chars().count().max(8) as f64 / total_weight.max(1) as f64;
            let hold_share = if fi + 1 == beat.frames.len() {
              speech_share + beat.pause_secs
            } else {
              speech_share
            };
            let escaped = frame.picture.to_string_lossy().replace('\'', "'\\''");
            frame_list.push_str(&format!("file '{escaped}'\nduration {hold_share:.3}\n"));
            let cap = if frame.caption.is_empty() {
              beat.caption.as_str()
            } else {
              frame.caption.as_str()
            };
            cue_windows.push((cap, sub_cursor, sub_cursor + speech_share));
            sub_cursor += hold_share;
          }
          if let Some(last) = beat.frames.last() {
            let escaped = last.picture.to_string_lossy().replace('\'', "'\\''");
            frame_list.push_str(&format!("file '{escaped}'\n"));
          }
          let frames_file = workdir.join(format!("frames-{i}.txt"));
          tokio::fs::write(&frames_file, frame_list)
            .await
            .map_err(|e| input_error(&self.id, format!("cannot write frame list: {e}")))?;
          vec![
            "-f".into(),
            "concat".into(),
            "-safe".into(),
            "0".into(),
            "-i".into(),
            frames_file.into(),
          ]
        } else {
          let single_cap = beat
            .frames
            .first()
            .filter(|f| !f.caption.is_empty())
            .map_or(beat.caption.as_str(), |f| f.caption.as_str());
          cue_windows.push((single_cap, timeline_cursor, timeline_cursor + speech_secs));
          let mut prefix: Vec<std::ffi::OsString> = if beat.moving {
            vec!["-stream_loop".into(), "-1".into()]
          } else {
            vec!["-loop".into(), "1".into()]
          };
          prefix.push("-i".into());
          prefix.push(beat.picture.clone().into());
          prefix
        };

        if !beat.hide_pip {
          if let Some(last_range) = pip_visible_ranges.last_mut()
            && (last_range.1 - timeline_cursor).abs() < 0.01
          {
            last_range.1 = timeline_cursor + total_seg_secs;
          } else {
            pip_visible_ranges.push((timeline_cursor, timeline_cursor + total_seg_secs));
          }
        }
        timeline_cursor += total_seg_secs;

        let fade_d = 0.18_f64.min(total_seg_secs * 0.25);
        let mut vf = base_fit.clone();
        if beat.fade_in && fade_d >= 0.06 {
          vf.push_str(&format!(",fade=t=in:st=0:d={fade_d:.3}:c=#0B1120"));
        }
        if beat.fade_out && fade_d >= 0.06 {
          let st = (total_seg_secs - fade_d).max(0.0);
          vf.push_str(&format!(",fade=t=out:st={st:.3}:d={fade_d:.3}:c=#0B1120"));
        }
        vf.push_str(",format=yuv420p");

        args.extend([
          "-i".into(),
          beat.audio.clone().into(),
          "-map".into(),
          "0:v:0".into(),
          "-map".into(),
          "1:a:0".into(),
          "-vf".into(),
          vf.into(),
        ]);
        if beat.pause_secs > 0.005 {
          args.extend([
            "-af".into(),
            format!("apad=pad_dur={:.3}", beat.pause_secs).into(),
          ]);
        }
        args.extend([
          "-c:v".into(),
          "libx264".into(),
          "-preset".into(),
          "veryfast".into(),
          "-c:a".into(),
          "aac".into(),
          "-b:a".into(),
          "160k".into(),
          "-ar".into(),
          "44100".into(),
          "-ac".into(),
          "2".into(),
          "-t".into(),
          format!("{total_seg_secs:.3}").into(),
          segment.clone().into(),
        ]);
        run_media_tool(&self.id, &ffmpeg, &workdir, &root, ffmpeg_args(args)).await?;
        list.push_str(&format!("file 'scene-{i}.mp4'\n"));
      }

      let list_path = workdir.join("scenes.txt");
      tokio::fs::write(&list_path, list)
        .await
        .map_err(|e| input_error(&self.id, format!("cannot write the scene list: {e}")))?;

      // 3. Build SRT + WebVTT cues; 4. Final join (+ continuous PiP presenter overlay and/or burned captions).
      let cues = build_timed_cues(&cue_windows);
      let (srt, vtt) = if !cues.is_empty() {
        (Some(cues_to_srt(&cues)), Some(cues_to_vtt(&cues)))
      } else {
        (None, None)
      };

      let out = workdir.join("out.mp4");
      let mut join: Vec<std::ffi::OsString> = vec![
        "-f".into(),
        "concat".into(),
        "-safe".into(),
        "0".into(),
        "-i".into(),
        list_path.into(),
      ];
      let burning = burn && srt.is_some();
      let active_pip = pip_source.as_ref().filter(|_| !pip_visible_ranges.is_empty());

      if let Some(srt_text) = &srt {
        let srt_path = workdir.join("captions.srt");
        tokio::fs::write(&srt_path, srt_text)
          .await
          .map_err(|e| input_error(&self.id, format!("cannot write the captions: {e}")))?;
      }

      if let Some((pip_path, pip_moving)) = active_pip {
        let pip_w = (((width as f64 * pip_scale).round() as u32) / 2 * 2).max(64);
        let pip_h = (((pip_w as f64 * 9.0 / 16.0).round() as u32) / 2 * 2).max(48);
        let (mask_png, border_png) = render_pip_overlay_chrome(pip_w, pip_h)
          .map_err(|e| input_error(&self.id, format!("pip chrome render failed: {e}")))?;
        let mask_path = workdir.join("pip_mask.png");
        let border_path = workdir.join("pip_border.png");
        tokio::fs::write(&mask_path, &mask_png)
          .await
          .map_err(|e| input_error(&self.id, format!("cannot write pip mask: {e}")))?;
        tokio::fs::write(&border_path, &border_png)
          .await
          .map_err(|e| input_error(&self.id, format!("cannot write pip border: {e}")))?;

        let margin_x = ((width as f64 * 0.025).round() as u32).max(8);
        let margin_y = ((height as f64 * 0.028).round() as u32).max(8);
        let (pip_x, pip_y) = match pip_position {
          "bottom_left" => (margin_x, height.saturating_sub(pip_h + margin_y)),
          "top_right" => (width.saturating_sub(pip_w + margin_x), margin_y * 2),
          _ => (
            width.saturating_sub(pip_w + margin_x),
            height.saturating_sub(pip_h + margin_y),
          ),
        };
        let border_x = pip_x.saturating_sub(4);
        let border_y = pip_y.saturating_sub(4);

        if *pip_moving {
          join.extend(["-stream_loop".into(), "-1".into(), "-i".into(), pip_path.clone().into()]);
        } else {
          join.extend([
            "-loop".into(),
            "1".into(),
            "-framerate".into(),
            VIDEO_FPS.to_string().into(),
            "-i".into(),
            pip_path.clone().into(),
          ]);
        }
        join.extend([
          "-loop".into(),
          "1".into(),
          "-framerate".into(),
          VIDEO_FPS.to_string().into(),
          "-i".into(),
          mask_path.into(),
          "-loop".into(),
          "1".into(),
          "-framerate".into(),
          VIDEO_FPS.to_string().into(),
          "-i".into(),
          border_path.into(),
        ]);

        let enable_clause = if pip_visible_ranges.len() == 1
          && pip_visible_ranges[0].0 <= 0.01
          && (pip_visible_ranges[0].1 - timeline_cursor).abs() <= 0.05
        {
          String::new()
        } else {
          let expr = pip_visible_ranges
            .iter()
            .map(|(a, b)| format!("between(t,{a:.2},{b:.2})"))
            .collect::<Vec<_>>()
            .join("+");
          format!(":enable='{expr}'")
        };

        let sub_tail = if burning {
          let margin_l = ((width as f64 * 0.040).round() as u32).max(24);
          let margin_r = if pip_position == "bottom_right" {
            pip_w + margin_x * 2 + 28
          } else {
            margin_l
          };
          let ass_text = cues_to_ass(&cues, width, height, margin_l, margin_r);
          tokio::fs::write(workdir.join("captions.ass"), ass_text)
            .await
            .map_err(|e| input_error(&self.id, format!("cannot write ass captions: {e}")))?;
          ";[v_pip]ass=captions.ass[vout]".to_string()
        } else {
          String::new()
        };
        let final_v = if burning { "[vout]" } else { "[v_pip]" };

        let fc = format!(
          "[1:v]scale={pip_w}:{pip_h}:force_original_aspect_ratio=increase,crop={pip_w}:{pip_h},setsar=1,fps={VIDEO_FPS},format=rgba[pip_raw];\
           [2:v]scale={pip_w}:{pip_h},format=gray[pip_mask];\
           [pip_raw][pip_mask]alphamerge[pip_masked];\
           [0:v][pip_masked]overlay={pip_x}:{pip_y}:shortest=1{enable_clause}[v_pip0];\
           [v_pip0][3:v]overlay={border_x}:{border_y}:shortest=1{enable_clause}[v_pip]{sub_tail}"
        );

        let mut srt_input_idx = None;
        if !burning && srt.is_some() {
          join.extend(["-i".into(), workdir.join("captions.srt").into()]);
          srt_input_idx = Some(4_usize);
        }

        join.extend([
          "-filter_complex".into(),
          fc.into(),
          "-map".into(),
          final_v.into(),
          "-map".into(),
          "0:a".into(),
        ]);
        if let Some(si) = srt_input_idx {
          join.extend([
            "-map".into(),
            format!("{si}:0").into(),
            "-c:s".into(),
            "mov_text".into(),
          ]);
        }
        join.extend(
          [
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
            "-t",
          ]
          .map(Into::into),
        );
        join.push(format!("{timeline_cursor:.3}").into());
      } else if burning {
        let margin_h = ((width as f64 * 0.05).round() as u32).max(24);
        let ass_text = cues_to_ass(&cues, width, height, margin_h, margin_h);
        tokio::fs::write(workdir.join("captions.ass"), ass_text)
          .await
          .map_err(|e| input_error(&self.id, format!("cannot write ass captions: {e}")))?;
        let vf_sub = "ass=captions.ass";
        join.extend(
          [
            "-vf",
            vf_sub,
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-pix_fmt",
            "yuv420p",
          ]
          .map(Into::into),
        );
      } else {
        if srt.is_some() {
          join.extend([
            "-i".into(),
            workdir.join("captions.srt").into(),
            "-map".into(),
            "0:v".into(),
            "-map".into(),
            "0:a".into(),
            "-map".into(),
            "1:0".into(),
            "-c:s".into(),
            "mov_text".into(),
          ]);
        }
        join.extend(["-c:v", "copy"].map(Into::into));
      }

      join.extend(["-c:a", "copy", "-movflags", "+faststart"].map(Into::into));
      join.push(out.clone().into());
      run_media_tool(&self.id, &ffmpeg, &workdir, &root, ffmpeg_args(join)).await?;
      let bytes = tokio::fs::read(&out)
        .await
        .map_err(|e| YanshiError::NodeExecutionFailed {
          message: format!("{}: ffmpeg wrote no output: {e}", self.id),
        })?;
      Ok::<_, YanshiError>((bytes, srt, vtt, timeline_cursor))
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&workdir).await;
    let (bytes, srt, vtt, seconds) = produced?;
    let mut outputs = output(
      &self.id,
      write_artifact(&self.id, &bytes, "video/mp4").await?,
    );
    if let Some(srt) = srt {
      outputs.insert(
        format!("{}_subtitles", self.id),
        write_artifact(&self.id, srt.as_bytes(), "application/x-subrip").await?,
      );
    }
    if let Some(vtt) = vtt {
      outputs.insert(
        format!("{}_vtt", self.id),
        write_artifact(&self.id, vtt.as_bytes(), "text/vtt").await?,
      );
    }
    outputs.insert(
      format!("{}_seconds", self.id),
      FlowValue::Json(serde_json::json!((seconds * 1000.0).round() / 1000.0)),
    );
    Ok(outputs)
  }
}

// ── render_document (external: pandoc, typst for PDF) ─────────────────

/// `render_document` (Y3.3): Markdown → `docx` / `pptx` / `html` /
/// `epub` / `pdf` through pandoc; PDF uses `--pdf-engine=typst` (one
/// binary, no TeX). Inputs: `markdown` (required), `format` (required),
/// `title`, `toc` (bool), `theme` (a reference document's name in
/// `YANSHI_THEMES_DIR`, for pptx / docx). The Markdown may embed images
/// from this run's artifacts (e.g. `figure` SVGs) by absolute path.
///
/// pandoc (and typst under it) runs under the platform OS sandbox
/// backend, writing only to this node's artifact directory and the temp
/// dir; see `run_pandoc`. Raw HTML and `@foo` Pandoc citations in the source
/// are read as literal text (`-f markdown-raw_html-citations`), so untrusted
/// HTML cannot inject script and `@mention` / `@Annotation` tokens never emit
/// `#cite(...)` without a bibliography.
pub struct RenderDocumentNode {
  id: String,
}

impl RenderDocumentNode {
  pub fn new(id: impl Into<String>) -> Self {
    Self { id: id.into() }
  }
}

/// (pandoc extension, MIME type) for each supported `format`.
fn document_format(format: &str) -> Option<(&'static str, &'static str)> {
  Some(match format {
    "docx" => (
      "docx",
      "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ),
    "pptx" => (
      "pptx",
      "application/vnd.openxmlformats-officedocument.presentationml.presentation",
    ),
    "html" => ("html", "text/html"),
    "epub" => ("epub", "application/epub+zip"),
    "pdf" => ("pdf", "application/pdf"),
    _ => return None,
  })
}

/// `YANSHI_THEMES_DIR`: operator-installed pandoc reference documents
/// (`<name>.pptx` / `<name>.docx`), selected by a node's `theme` input.
fn themes_dir() -> Option<std::path::PathBuf> {
  std::env::var_os("YANSHI_THEMES_DIR")
    .map(std::path::PathBuf::from)
    .filter(|dir| dir.is_dir())
}

/// The reference document for `theme` and `extension`, if one is
/// installed. A theme is a bare name: it cannot point elsewhere on the
/// host, and an unknown one is an error rather than a silent default.
fn theme_reference(
  node: &str,
  theme: &str,
  extension: &str,
) -> Result<std::path::PathBuf, YanshiError> {
  let bare = !theme.is_empty()
    && theme
      .chars()
      .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
  if !bare {
    return Err(input_error(
      node,
      format!("theme '{theme}' must be a bare name"),
    ));
  }
  let dir = themes_dir()
    .ok_or_else(|| input_error(node, "a theme needs YANSHI_THEMES_DIR on this host"))?;
  let path = dir.join(format!("{theme}.{extension}"));
  if path.is_file() {
    Ok(path)
  } else {
    Err(input_error(
      node,
      format!("no theme '{theme}' for .{extension} in {}", dir.display()),
    ))
  }
}

/// Run pandoc (and, for PDF, typst under it) through
/// [`crate::common::sandboxed`]: fonts and typst's install tree readable,
/// writes confined to `workdir` and the temp dir.
async fn run_pandoc(
  node: &str,
  workdir: &std::path::Path,
  args: &[std::ffi::OsString],
  needs_typst: bool,
) -> Result<(), YanshiError> {
  use crate::common::sandboxed;
  let not_installed = |tool: &str| YanshiError::NodeExecutionFailed {
    message: format!("{node}: {tool} is not on PATH (`yanshi doctor` lists renderer binaries)"),
  };
  let pandoc = sandboxed::find_tool("pandoc").ok_or_else(|| not_installed("pandoc"))?;
  let mut read_paths = sandboxed::font_dirs();
  if needs_typst {
    let typst =
      sandboxed::find_tool("typst").ok_or_else(|| not_installed("typst (the PDF engine)"))?;
    read_paths.extend(sandboxed::install_prefixes(&typst));
  }
  // Y6.1: a document may embed this run's own artifacts (figures);
  // nothing else of the host's files.
  read_paths.extend(yanshi_core::run_context::current_artifact_root());
  read_paths.extend(themes_dir());
  let output = sandboxed::run(sandboxed::SandboxedRun {
    node,
    program: pandoc,
    args: args.to_vec(),
    workdir,
    read_paths,
    write_paths: Vec::new(),
    env: Vec::new(),
    timeout: std::time::Duration::from_secs(300),
  })
  .await?;
  if !output.status.success() {
    return Err(YanshiError::NodeExecutionFailed {
      message: format!(
        "{node}: pandoc exited with {}: {}",
        output.status,
        sandboxed::output_tail(&output, 12)
      ),
    });
  }
  Ok(())
}

/// Rewrite absolute run-artifact image paths inside staged Markdown into
/// `workdir`-relative paths (`../<sibling_node>/<file>`) so both Pandoc's
/// archive builders (`pptx`/`docx`/`html`) and Typst (`--root=<art_root>`,
/// where leading `/` means root-relative rather than filesystem `/`) resolve
/// sibling node artifacts identically.
fn relativize_artifact_paths(
  markdown: &str,
  workdir: &std::path::Path,
  art_root: Option<&std::path::Path>,
) -> String {
  let mut rules: Vec<(String, &'static str)> = Vec::new();
  let mut push_dir = |dir: &std::path::Path, rel: &'static str| {
    let raw = format!("{}/", dir.to_string_lossy().trim_end_matches('/'));
    if raw.len() > 1 && !rules.iter().any(|(p, _)| p == &raw) {
      rules.push((raw, rel));
    }
    if let Ok(canon) = dir.canonicalize() {
      let c = format!("{}/", canon.to_string_lossy().trim_end_matches('/'));
      if c.len() > 1 && !rules.iter().any(|(p, _)| p == &c) {
        rules.push((c, rel));
      }
    }
  };
  // `workdir` is `<art_root>/artifacts/<node_id>`, so `<art_root>/artifacts/`
  // is `../` and `<art_root>/` (for any root-level files) is `../../`.
  if let Some(artifacts_dir) = workdir.parent() {
    push_dir(artifacts_dir, "../");
  }
  if let Some(root) = art_root {
    let root_artifacts = root.join("artifacts");
    push_dir(&root_artifacts, "../");
    push_dir(root, "../../");
  }
  let mut out = markdown.to_string();
  for (prefix, rel) in rules {
    out = out.replace(&prefix, rel);
  }
  out
}

#[async_trait]
impl AsyncNode for RenderDocumentNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let markdown = text_input(&self.id, inputs, "markdown").await?;
    let format = optional_str(inputs, "format")
      .ok_or_else(|| input_error(&self.id, "missing input 'format'"))?;
    let (extension, mime) = document_format(format).ok_or_else(|| {
      input_error(
        &self.id,
        format!("unknown format '{format}': expected docx, pptx, html, epub or pdf"),
      )
    })?;

    let workdir = crate::common::artifacts::artifact_dir(&self.id);
    tokio::fs::create_dir_all(&workdir).await.map_err(|e| {
      input_error(
        &self.id,
        format!("cannot create {}: {e}", workdir.display()),
      )
    })?;
    let stem = format!(".render-{}", uuid::Uuid::new_v4());
    let source = workdir.join(format!("{stem}.md"));
    let target = workdir.join(format!("{stem}.{extension}"));
    let metadata = workdir.join(format!("{stem}.json"));
    let cleanup = [source.clone(), target.clone(), metadata.clone()];

    let art_root = yanshi_core::run_context::current_artifact_root();
    let staged_markdown = relativize_artifact_paths(&markdown, &workdir, art_root.as_deref());

    let rendered = async {
      tokio::fs::write(&source, staged_markdown.as_bytes())
        .await
        .map_err(|e| input_error(&self.id, format!("cannot stage the source: {e}")))?;
      let mut res_dirs: Vec<&std::path::Path> = vec![workdir.as_path()];
      if let Some(artifacts_dir) = workdir.parent() {
        res_dirs.push(artifacts_dir);
      }
      if let Some(root) = art_root.as_deref() {
        res_dirs.push(root);
      }
      let resource_path: std::ffi::OsString =
        std::env::join_paths(res_dirs).unwrap_or_else(|_| workdir.clone().into_os_string());
      let mut args: Vec<std::ffi::OsString> = vec![
        source.clone().into(),
        "-f".into(),
        "markdown-raw_html-citations".into(),
        "--standalone".into(),
        "-o".into(),
        target.clone().into(),
        "--resource-path".into(),
        resource_path,
      ];
      if let Some(title) = optional_str(inputs, "title") {
        tokio::fs::write(&metadata, serde_json::json!({ "title": title }).to_string())
          .await
          .map_err(|e| input_error(&self.id, format!("cannot stage metadata: {e}")))?;
        args.push("--metadata-file".into());
        args.push(metadata.clone().into());
      }
      if matches!(inputs.get("toc"), Some(FlowValue::Json(Value::Bool(true)))) {
        args.push("--toc".into());
      }
      match extension {
        "html" => args.push("--embed-resources".into()),
        "pdf" => {
          args.push("--pdf-engine=typst".into());
          let typst_root = art_root.as_deref().unwrap_or(workdir.as_path());
          let mut root_opt = std::ffi::OsString::from("--pdf-engine-opt=--root=");
          root_opt.push(typst_root.as_os_str());
          args.push(root_opt);
        }
        _ => {}
      }
      if let Some(theme) = optional_str(inputs, "theme")
        && matches!(extension, "pptx" | "docx")
      {
        args.push("--reference-doc".into());
        args.push(theme_reference(&self.id, theme, extension)?.into());
      }
      run_pandoc(&self.id, &workdir, &args, extension == "pdf").await?;
      tokio::fs::read(&target)
        .await
        .map_err(|e| YanshiError::NodeExecutionFailed {
          message: format!("{}: pandoc wrote no output: {e}", self.id),
        })
    }
    .await;
    for path in cleanup {
      let _ = tokio::fs::remove_file(path).await;
    }
    let bytes = rendered?;
    Ok(output(
      &self.id,
      write_artifact(&self.id, &bytes, mime).await?,
    ))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;
  use yanshi_core::run_context;

  fn inputs(pairs: &[(&str, Value)]) -> AsyncNodeInputs {
    pairs
      .iter()
      .map(|(k, v)| (k.to_string(), FlowValue::Json(v.clone())))
      .collect()
  }

  async fn run(node: &dyn AsyncNode, inputs: AsyncNodeInputs) -> (std::path::PathBuf, String) {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.keep();
    let outputs = run_context::with_artifact_root(root, node.execute(&inputs))
      .await
      .expect("renders");
    let (_, value) = outputs.into_iter().next().expect("one output");
    let FlowValue::File { path, mime_type } = value else {
      panic!("expected a File");
    };
    (path, mime_type.unwrap_or_default())
  }

  #[tokio::test]
  async fn table_csv_round_trips_object_rows_in_first_seen_column_order() {
    let rows = json!([{"name": "Ada", "year": 1843}, {"name": "Grace", "note": "COBOL"}]);
    let (path, mime) = run(&RenderTableNode::new("t"), inputs(&[("rows", rows)])).await;
    assert_eq!(mime, "text/csv");
    let mut reader = csv::Reader::from_path(&path).expect("csv");
    let header: Vec<String> = reader
      .headers()
      .expect("header")
      .iter()
      .map(String::from)
      .collect();
    assert_eq!(header.len(), 3);
    let records: Vec<Vec<String>> = reader
      .records()
      .map(|r| r.expect("record").iter().map(String::from).collect())
      .collect();
    let col = |name: &str| header.iter().position(|h| h == name).expect("column");
    assert_eq!(records[0][col("name")], "Ada");
    assert_eq!(records[0][col("year")], "1843");
    assert_eq!(records[1][col("note")], "COBOL");
    assert_eq!(records[1][col("year")], "", "missing cells are empty");
  }

  #[tokio::test]
  async fn table_accepts_a_json_string_and_explicit_columns() {
    let rows = Value::String(r#"[["a", 1], ["b", 2]]"#.to_string());
    let (path, _) = run(
      &RenderTableNode::new("t"),
      inputs(&[("rows", rows), ("columns", json!(["letter", "n"]))]),
    )
    .await;
    let text = std::fs::read_to_string(path).expect("csv");
    assert_eq!(text, "letter,n\na,1\nb,2\n");
  }

  #[tokio::test]
  async fn table_xlsx_is_a_zip_workbook_with_the_cells() {
    let rows = json!([{"city": "Lisbon", "pop": 545000}]);
    let (path, mime) = run(
      &RenderTableNode::new("t"),
      inputs(&[("rows", rows), ("format", json!("xlsx"))]),
    )
    .await;
    assert_eq!(mime, XLSX_MIME);
    assert_eq!(path.extension().and_then(|e| e.to_str()), Some("xlsx"));
    let mut archive =
      zip::ZipArchive::new(std::fs::File::open(&path).expect("xlsx")).expect("a zip container");
    let mut read = |name: &str| {
      let mut text = String::new();
      std::io::Read::read_to_string(&mut archive.by_name(name).expect(name), &mut text)
        .expect("utf-8 xml");
      text
    };
    let sheet = read("xl/worksheets/sheet1.xml");
    let strings = read("xl/sharedStrings.xml");
    assert!(
      strings.contains("city") && strings.contains("Lisbon"),
      "{strings}"
    );
    assert!(
      sheet.contains("<v>545000</v>"),
      "the number is stored as a number cell: {sheet}"
    );
  }

  #[tokio::test]
  async fn table_rejects_mixed_rows() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderTableNode::new("t").execute(&inputs(&[("rows", json!([{"a": 1}, [1]]))])),
    )
    .await
    .expect_err("mixed rows");
    assert!(
      err.to_string().contains("all objects or all arrays"),
      "{err}"
    );
  }

  const SQUARE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">
<rect width="20" height="20" fill="#ff0000"/><rect x="20" width="20" height="20" fill="#0000ff"/></svg>"##;

  #[tokio::test]
  async fn svg_renders_to_a_png_of_the_declared_size() {
    let (path, mime) = run(&RenderSvgNode::new("s"), inputs(&[("svg", json!(SQUARE))])).await;
    assert_eq!(mime, "image/png");
    let png = resvg::tiny_skia::Pixmap::load_png(&path).expect("decodes as PNG");
    assert_eq!((png.width(), png.height()), (40, 20));
    let left = png.pixel(5, 10).expect("pixel");
    let right = png.pixel(35, 10).expect("pixel");
    assert_eq!((left.red(), left.blue()), (255, 0), "left half is red");
    assert_eq!((right.red(), right.blue()), (0, 255), "right half is blue");
  }

  #[tokio::test]
  async fn svg_width_scales_the_output() {
    let (path, _) = run(
      &RenderSvgNode::new("s"),
      inputs(&[("svg", json!(SQUARE)), ("width", json!(400))]),
    )
    .await;
    let png = resvg::tiny_skia::Pixmap::load_png(&path).expect("png");
    assert_eq!((png.width(), png.height()), (400, 200));
  }

  #[tokio::test]
  async fn svg_rejects_a_huge_canvas() {
    let huge = r#"<svg xmlns="http://www.w3.org/2000/svg" width="100000" height="100000"/>"#;
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderSvgNode::new("s").execute(&inputs(&[("svg", json!(huge))])),
    )
    .await
    .expect_err("too big");
    assert!(err.to_string().contains("pixels"), "{err}");
  }

  #[tokio::test]
  async fn markdown_page_is_standalone_and_escapes_raw_html() {
    let markdown = "# Tides\n\n| a | b |\n|---|---|\n| 1 | 2 |\n\n<script>alert(1)</script>\n";
    let (path, mime) = run(
      &RenderMarkdownNode::new("m"),
      inputs(&[("markdown", json!(markdown))]),
    )
    .await;
    assert_eq!(mime, "text/html");
    let html = std::fs::read_to_string(path).expect("html");
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("<title>Tides</title>"));
    assert!(html.contains("<h1>Tides</h1>"));
    assert!(html.contains("<table>") && html.contains("<td>1</td>"));
    assert!(html.contains("<style>"), "stylesheet is inline");
    assert!(!html.contains("<script>"), "raw HTML must not pass through");
    assert!(html.contains("&lt;script&gt;"));
  }

  /// Y3.3 acceptance. Needs `pandoc` and `typst` on PATH:
  /// `cargo test -p yanshi-nodes --lib render_document -- --ignored`.
  #[tokio::test]
  #[ignore = "needs pandoc and typst"]
  async fn render_document_pdf_has_pages_and_docx_reads_back() {
    let markdown = "# 潮汐\n\nTides follow **the moon**. 潮汐由月球引力引起（支持 @提及 与 @Transactional）。\n\n<script>x</script>\n";
    let (pdf, mime) = run(
      &RenderDocumentNode::new("doc"),
      inputs(&[
        ("markdown", json!(markdown)),
        ("format", json!("pdf")),
        ("title", json!("It's a \"report\"")),
      ]),
    )
    .await;
    assert_eq!(mime, "application/pdf");
    let bytes = std::fs::read(&pdf).expect("pdf");
    assert!(bytes.starts_with(b"%PDF"));
    let text = String::from_utf8_lossy(&bytes);
    let pages: u32 = text
      .split("/Count ")
      .skip(1)
      .filter_map(|rest| {
        rest
          .split(|c: char| !c.is_ascii_digit())
          .next()?
          .parse()
          .ok()
      })
      .max()
      .unwrap_or(0);
    assert!(pages > 0, "PDF page count must be > 0");

    let (docx, mime) = run(
      &RenderDocumentNode::new("doc"),
      inputs(&[("markdown", json!(markdown)), ("format", json!("docx"))]),
    )
    .await;
    assert!(mime.ends_with("wordprocessingml.document"));
    let back = std::process::Command::new("pandoc")
      .arg(&docx)
      .args(["-t", "plain"])
      .output()
      .expect("pandoc reads the docx back");
    let body = String::from_utf8_lossy(&back.stdout);
    assert!(body.contains("潮汐由月球引力引起"), "{body}");
    assert!(
      body.contains("<script>x</script>"),
      "raw HTML stays literal text: {body}"
    );
    let leftovers: Vec<_> = std::fs::read_dir(docx.parent().expect("dir"))
      .expect("dir")
      .flatten()
      .filter(|e| e.file_name().to_string_lossy().starts_with(".render-"))
      .collect();
    assert!(leftovers.is_empty(), "staging files are removed");
  }

  #[tokio::test]
  async fn render_document_rejects_an_unknown_format_before_running_pandoc() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let err = run_context::with_artifact_root(
      tmp.path().to_path_buf(),
      RenderDocumentNode::new("d").execute(&inputs(&[
        ("markdown", json!("# x")),
        ("format", json!("odt")),
      ])),
    )
    .await
    .expect_err("unknown format");
    assert!(err.to_string().contains("unknown format 'odt'"), "{err}");
  }

  fn sample_tree() -> Value {
    json!({"label": "潮汐 Tides", "children": [
      {"label": "Causes", "children": [{"label": "Moon"}, {"label": "Sun"}]},
      {"label": "Effects", "children": [{"label": "Coastal flooding"}]},
      {"label": "Rhythm"}
    ]})
  }

  #[tokio::test]
  async fn mindmap_svg_lays_out_every_node_and_renders_to_png() {
    let (path, mime) = run(
      &RenderMindmapNode::new("mm"),
      inputs(&[("tree", sample_tree())]),
    )
    .await;
    assert_eq!(mime, "image/svg+xml");
    let svg = std::fs::read_to_string(&path).expect("svg");
    for label in [
      "潮汐 Tides",
      "Causes",
      "Moon",
      "Sun",
      "Effects",
      "Coastal flooding",
      "Rhythm",
    ] {
      assert!(svg.contains(&format!(">{label}</text>")), "{label} missing");
    }
    assert_eq!(
      svg.matches("<path ").count(),
      6,
      "one link per non-root node"
    );
    let png = svg_to_png(&svg, None).expect("the SVG renders");
    assert_eq!(&png[1..4], b"PNG");
  }

  #[tokio::test]
  async fn mindmap_parents_sit_between_their_children() {
    let placed = layout_tree(&sample_tree()).expect("layout");
    let y = |label: &str| placed.iter().find(|p| p.label == label).expect(label).y;
    assert!((y("Causes") - (y("Moon") + y("Sun")) / 2.0).abs() < 0.01);
    assert!(y("Moon") < y("Sun") && y("Sun") < y("Coastal flooding"));
  }

  #[tokio::test]
  async fn mindmap_html_embeds_markmap_markdown() {
    let (path, mime) = run(
      &RenderMindmapNode::new("mm"),
      inputs(&[("tree", sample_tree()), ("format", json!("html"))]),
    )
    .await;
    assert_eq!(mime, "text/html");
    let html = std::fs::read_to_string(path).expect("html");
    assert!(html.contains("markmap-autoloader"));
    assert!(html.contains("# 潮汐 Tides\n## Causes\n### Moon"), "{html}");
  }

  #[test]
  fn mindmap_rejects_unlabelled_and_oversized_trees() {
    assert!(layout_tree(&json!({"children": []})).is_err());
    let mut deep = json!({"label": "leaf"});
    for _ in 0..10 {
      deep = json!({"label": "n", "children": [deep]});
    }
    assert!(layout_tree(&deep).unwrap_err().contains("deeper"));
  }

  /// Needs ffmpeg: `cargo test -p yanshi-nodes --lib render_audio -- --ignored`.
  #[tokio::test]
  #[ignore = "needs ffmpeg"]
  async fn render_audio_joins_and_normalises_clips() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let clips_dir = root.join("artifacts").join("speak");
    std::fs::create_dir_all(&clips_dir).expect("dir");
    for (name, freq) in [("1.wav", 440), ("2.wav", 660)] {
      let status = std::process::Command::new("ffmpeg")
        .args([
          "-hide_banner",
          "-loglevel",
          "error",
          "-y",
          "-f",
          "lavfi",
          "-i",
        ])
        .arg(format!("sine=frequency={freq}:duration=1"))
        .arg(clips_dir.join(name))
        .status()
        .expect("ffmpeg");
      assert!(status.success());
    }
    let clips = json!([clips_dir.join("1.wav"), clips_dir.join("2.wav")]);
    let outputs = run_context::with_artifact_root(
      root.clone(),
      RenderAudioNode::new("mix").execute(&inputs(&[("clips", clips)])),
    )
    .await
    .expect("mixes");
    let FlowValue::File { path, mime_type } = &outputs["mix_output"] else {
      panic!("expected File");
    };
    assert_eq!(mime_type.as_deref(), Some("audio/mpeg"));
    assert!(std::fs::metadata(path).expect("mp3").len() > 1000);
    let FlowValue::Json(lufs) = &outputs["mix_lufs"] else {
      panic!("lufs");
    };
    assert!(
      (lufs.as_f64().expect("measured") + 16.0).abs() < 1.5,
      "{lufs}"
    );

    // A clip outside the run's artifacts is refused.
    let outside = tempfile::NamedTempFile::new().expect("tmp");
    let err = run_context::with_artifact_root(
      root,
      RenderAudioNode::new("mix").execute(&inputs(&[("clips", json!([outside.path()]))])),
    )
    .await
    .expect_err("outside");
    assert!(
      err.to_string().contains("outside this run's artifacts"),
      "{err}"
    );
  }

  #[test]
  fn loudnorm_stats_are_read_from_the_last_json_block() {
    let stderr = "frame=...\n[Parsed_loudnorm_0 @ 0x1] \n{\n\t\"input_i\" : \"-23.10\",\n\t\"output_i\" : \"-16.02\"\n}\n";
    let stats = loudnorm_stats(stderr).expect("stats");
    assert_eq!(stat_f64(&stats, "input_i"), Some(-23.1));
    assert_eq!(stat_f64(&stats, "output_i"), Some(-16.02));
    let silence = loudnorm_stats("{\n\"input_i\" : \"-inf\"\n}").expect("stats");
    assert_eq!(
      stat_f64(&silence, "input_i"),
      None,
      "silence is not normalised"
    );
  }

  #[test]
  fn a_theme_is_a_bare_name_inside_the_themes_dir() {
    for bad in ["../etc/passwd", "a/b", "", "x.pptx"] {
      assert!(theme_reference("d", bad, "pptx").is_err(), "{bad:?}");
    }
  }

  #[test]
  fn study_pages_escape_model_text_and_mark_the_right_answer() {
    let page = study_page(
      "quiz",
      "Tides <quiz>",
      &[json!({"question": "What pulls the sea? <script>x()</script>", "options": ["The moon", "The wind"], "answer": 0, "explanation": "Gravity."})],
    )
    .expect("page");
    assert!(!page.contains("<script>x()"), "model text is escaped");
    assert!(page.contains("&lt;script&gt;x()"));
    assert!(
      page.contains("data-right=\"1\"> The moon") && page.contains("data-right=\"0\"> The wind")
    );
    assert!(page.contains("<title>Tides &lt;quiz&gt;</title>"));
    let cards = study_page(
      "flashcards",
      "Cards",
      &[json!({"front": "Neap tide", "back": "At quarter moon"})],
    )
    .expect("cards");
    assert!(cards.contains("<div class=\"front\">Neap tide</div>"));
  }

  #[test]
  fn study_pages_reject_bad_items() {
    assert!(
      study_page(
        "quiz",
        "t",
        &[json!({"question": "q", "options": ["a", "b"], "answer": 2})]
      )
      .is_err(),
      "answer out of range"
    );
    assert!(
      study_page(
        "quiz",
        "t",
        &[json!({"question": "q", "options": ["a"], "answer": 0})]
      )
      .is_err(),
      "one option"
    );
    assert!(
      study_page("flashcards", "t", &[json!({"front": "x"})]).is_err(),
      "no back"
    );
    assert!(study_page("flashcards", "t", &[]).is_err(), "empty");
  }

  #[test]
  fn captions_split_into_timed_sentence_cues() {
    assert_eq!(
      caption_sentences("Tides rise. They fall again! 潮起。潮落"),
      vec!["Tides rise.", "They fall again!", "潮起。", "潮落"]
    );
    assert_eq!(caption_sentences("e.g. 3.5 m"), vec!["e.g. 3.5 m"]);
    let srt = build_srt(&["One. Two two.", "", "Three."], &[3.0, 1.0, 2.0]);
    assert_eq!(
      srt,
      "1\n00:00:00,000 --> 00:00:01,000\nOne.\n\n2\n00:00:01,000 --> 00:00:03,000\nTwo two.\n\n3\n00:00:04,000 --> 00:00:06,000\nThree.\n\n"
    );
    assert_eq!(srt_time(3725.5), "01:02:05,500");
  }

  #[test]
  fn the_subtitles_filter_is_found_by_name() {
    let with = " T.. ass               V->V       Render ASS subtitles onto input video using the libass library.\n ... subtitles         V->V       Render text subtitles onto input video using the libass library.\n";
    let without = " T.C drawbox           V->V       Draw a colored box on the input video.\n";
    assert!(has_subtitles_filter(with));
    assert!(!has_subtitles_filter(without));
  }

  /// Y8.6. Needs ffmpeg; checks which kind it is: with libass the
  /// captions are burned in (no subtitle stream, the .srt still written),
  /// without it the node says so instead of falling back.
  #[tokio::test]
  #[ignore = "needs ffmpeg"]
  async fn render_video_burns_subtitles_in_or_says_why_it_cannot() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let dir = root.join("artifacts").join("scene");
    std::fs::create_dir_all(&dir).expect("dir");
    for (args, out) in [
      (["-f", "lavfi", "-i", "color=c=blue:s=320x180:d=1"], "a.png"),
      (
        ["-f", "lavfi", "-i", "sine=frequency=440:duration=1"],
        "a.wav",
      ),
    ] {
      let mut command = std::process::Command::new("ffmpeg");
      command
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args);
      if out.ends_with(".png") {
        command.args(["-frames:v", "1"]);
      }
      assert!(
        command
          .arg(dir.join(out))
          .status()
          .expect("ffmpeg")
          .success()
      );
    }
    let listing = std::process::Command::new("ffmpeg")
      .args(["-hide_banner", "-filters"])
      .output()
      .expect("ffmpeg");
    let libass = has_subtitles_filter(&String::from_utf8_lossy(&listing.stdout));
    let result = run_context::with_artifact_root(
      root.clone(),
      RenderVideoNode::new("film").execute(&inputs(&[
        (
          "scenes",
          json!([{"image": dir.join("a.png"), "audio": dir.join("a.wav"), "caption": "Burned in."}]),
        ),
        ("width", json!(320)),
        ("height", json!(180)),
        ("burn_subtitles", json!(true)),
      ])),
    )
    .await;
    if !libass {
      let error = result.expect_err("no libass").to_string();
      assert!(error.contains("built with libass"), "{error}");
      return;
    }
    let outputs = result.expect("burns");
    let FlowValue::File { path, .. } = &outputs["film_output"] else {
      panic!("mp4");
    };
    let probe = std::process::Command::new("ffprobe")
      .args([
        "-v",
        "error",
        "-show_entries",
        "stream=codec_type",
        "-of",
        "csv=p=0",
      ])
      .arg(path)
      .output()
      .expect("ffprobe");
    let streams = String::from_utf8_lossy(&probe.stdout);
    assert!(
      streams.contains("video") && !streams.contains("subtitle"),
      "{streams}"
    );
    // The frame is one flat grey unless the caption was drawn (libass
    // skips text silently when it can't read a font). Measured at this
    // size: 0 pixels off the background plain, ~100 with the caption.
    let frame = std::process::Command::new("ffmpeg")
      .args(["-hide_banner", "-loglevel", "error", "-ss", "0.5", "-i"])
      .arg(path)
      .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray", "-"])
      .output()
      .expect("ffmpeg");
    let mut sorted = frame.stdout.clone();
    sorted.sort_unstable();
    let background = sorted.get(sorted.len() / 2).copied().unwrap_or(0);
    let drawn = frame
      .stdout
      .iter()
      .filter(|&&v| v.abs_diff(background) > 40)
      .count();
    assert!(drawn > 50, "no caption pixels in the frame ({drawn})");
    assert!(matches!(outputs["film_subtitles"], FlowValue::File { .. }));
  }

  /// Needs ffmpeg: `cargo test -p yanshi-nodes --lib render_video -- --ignored`.
  #[tokio::test]
  #[ignore = "needs ffmpeg"]
  async fn render_video_joins_scenes_with_a_subtitle_track() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let dir = root.join("artifacts").join("scene");
    std::fs::create_dir_all(&dir).expect("dir");
    let make = |args: &[&str], out: &str| {
      let status = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .arg(dir.join(out))
        .status()
        .expect("ffmpeg");
      assert!(status.success(), "{out}");
    };
    make(
      &[
        "-f",
        "lavfi",
        "-i",
        "color=c=red:s=301x200:d=1",
        "-frames:v",
        "1",
      ],
      "a.png",
    );
    make(&["-f", "lavfi", "-i", "testsrc=s=320x240:d=0.5"], "b.mp4");
    make(
      &["-f", "lavfi", "-i", "sine=frequency=440:duration=1.2"],
      "a.wav",
    );
    make(
      &["-f", "lavfi", "-i", "sine=frequency=660:duration=0.8"],
      "b.wav",
    );
    let scenes = json!([
      {"image": dir.join("a.png"), "audio": dir.join("a.wav"), "caption": "A red card. It stays."},
      {"video": dir.join("b.mp4"), "audio": dir.join("b.wav"), "caption": "A test pattern."}
    ]);
    let outputs = run_context::with_artifact_root(
      root.clone(),
      RenderVideoNode::new("film").execute(&inputs(&[
        ("scenes", scenes),
        ("width", json!(640)),
        ("height", json!(360)),
      ])),
    )
    .await
    .expect("renders");
    let FlowValue::File { path, mime_type } = &outputs["film_output"] else {
      panic!("expected File");
    };
    assert_eq!(mime_type.as_deref(), Some("video/mp4"));
    let FlowValue::Json(seconds) = &outputs["film_seconds"] else {
      panic!("seconds");
    };
    assert!(
      (seconds.as_f64().expect("number") - 2.0).abs() < 0.05,
      "{seconds}"
    );
    let probe = std::process::Command::new("ffprobe")
      .args([
        "-v",
        "error",
        "-show_entries",
        "stream=codec_type,width,height",
        "-of",
        "csv=p=0",
      ])
      .arg(path)
      .output()
      .expect("ffprobe");
    let streams = String::from_utf8_lossy(&probe.stdout);
    assert!(streams.contains("video,640,360"), "{streams}");
    assert!(
      streams.contains("audio") && streams.contains("subtitle"),
      "{streams}"
    );
    let FlowValue::File { path: srt, .. } = &outputs["film_subtitles"] else {
      panic!("srt");
    };
    let srt = std::fs::read_to_string(srt).expect("srt");
    assert!(
      srt.contains("00:00:01,200 --> 00:00:02,000\nA test pattern."),
      "{srt}"
    );

    // Odd sizes and pictures outside the run are refused.
    let bad = inputs(&[
      (
        "scenes",
        json!([{"image": dir.join("a.png"), "audio": dir.join("a.wav")}]),
      ),
      ("width", json!(641)),
    ]);
    let err =
      run_context::with_artifact_root(root.clone(), RenderVideoNode::new("film").execute(&bad))
        .await
        .expect_err("odd width");
    assert!(err.to_string().contains("even"), "{err}");
    let outside = tempfile::NamedTempFile::new().expect("tmp");
    let err = run_context::with_artifact_root(
      root,
      RenderVideoNode::new("film").execute(&inputs(&[(
        "scenes",
        json!([{"image": outside.path(), "audio": dir.join("a.wav")}]),
      )])),
    )
    .await
    .expect_err("outside");
    assert!(
      err.to_string().contains("outside this run's artifacts"),
      "{err}"
    );
  }

  #[test]
  fn mindmap_accepts_common_label_aliases_string_leaves_and_skips_empty_children() {
    let tree = json!({
      "name": "Pingora Gateway",
      "children": [
        { "topic": "Zero-Downtime Upgrade", "children": ["FD Passing", "Graceful Drain", {}] },
        { "heading": "Tokio Work-Stealing" },
        {}
      ]
    });
    let svg = mindmap_svg(&tree).expect("svg");
    assert!(svg.contains("Pingora Gateway"));
    assert!(svg.contains("FD Passing"));
    let html = mindmap_page(&tree).expect("html");
    assert!(html.contains("Zero-Downtime Upgrade"));
  }

  #[test]
  fn relativize_artifact_paths_maps_sibling_artifacts_and_root_files() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path();
    let slides_dir = root.join("artifacts").join("slides_render");
    let workdir = root.join("artifacts").join("deck_pptx");
    std::fs::create_dir_all(&slides_dir).expect("mkdir");
    std::fs::create_dir_all(&workdir).expect("mkdir");
    let md = format!(
      "![slide]({}/1.png)\n![raw]({}/cover.png)\n",
      slides_dir.display(),
      root.display()
    );
    let staged = relativize_artifact_paths(&md, &workdir, Some(root));
    assert_eq!(
      staged,
      "![slide](../slides_render/1.png)\n![raw](../../cover.png)\n"
    );
  }

  #[test]
  fn timed_cues_exclude_breathing_pauses_and_wrap_long_clauses_to_vtt() {
    let windows = [
      (
        "第一句先看左侧结构体定义，它只描述字段在栈和堆上的真实字节布局而不绑定虚表指针。",
        0.0,
        2.4,
      ),
      ("第二句切到右侧流程图。", 2.7, 4.2),
    ];
    let cues = build_timed_cues(&windows);
    assert_eq!(cues.len(), 2);
    assert!(
      (cues[0].end - 2.4).abs() < 1e-6,
      "cue ends before 0.3s pause"
    );
    assert!(
      (cues[1].start - 2.7).abs() < 1e-6,
      "next cue starts after pause"
    );
    assert!(
      cues[0].text.contains('\n'),
      "long clause wrapped into two balanced lines: {:?}",
      cues[0].text
    );
    let vtt = cues_to_vtt(&cues);
    assert!(vtt.starts_with("WEBVTT\n\n1\n00:00:00.000 --> 00:00:02.400\n"));
  }

  #[tokio::test]
  #[ignore = "needs ffmpeg"]
  async fn render_video_supports_step_pauses_fade_transitions_and_pip_overlay() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = tmp.path().to_path_buf();
    let dir = root.join("artifacts").join("scene");
    std::fs::create_dir_all(&dir).expect("dir");
    let make = |args: &[&str], out: &str| {
      let status = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args(args)
        .arg(dir.join(out))
        .status()
        .expect("ffmpeg");
      assert!(status.success(), "{out}");
    };
    make(
      &[
        "-f",
        "lavfi",
        "-i",
        "color=c=navy:s=640x360:d=1",
        "-frames:v",
        "1",
      ],
      "step1.png",
    );
    make(
      &[
        "-f",
        "lavfi",
        "-i",
        "color=c=teal:s=640x360:d=1",
        "-frames:v",
        "1",
      ],
      "step2.png",
    );
    make(
      &[
        "-f",
        "lavfi",
        "-i",
        "color=c=orange:s=320x180:d=1",
        "-frames:v",
        "1",
      ],
      "avatar.png",
    );
    make(
      &["-f", "lavfi", "-i", "sine=frequency=440:duration=1.0"],
      "s1.wav",
    );
    make(
      &["-f", "lavfi", "-i", "sine=frequency=550:duration=1.0"],
      "s2.wav",
    );

    let scenes = json!([
      {
        "image": dir.join("step1.png"),
        "audio": dir.join("s1.wav"),
        "caption": "Step one spotlight.",
        "is_slide_start": true,
        "is_slide_end": false
      },
      {
        "image": dir.join("step2.png"),
        "audio": dir.join("s2.wav"),
        "caption": "Slide end transition.",
        "is_slide_start": false,
        "is_slide_end": true
      }
    ]);

    let outputs = run_context::with_artifact_root(
      root.clone(),
      RenderVideoNode::new("talk").execute(&inputs(&[
        ("scenes", scenes),
        ("width", json!(640)),
        ("height", json!(360)),
        ("step_pause_ms", json!(250)),
        ("slide_pause_ms", json!(500)),
        ("transition", json!("fade")),
        ("pip_image", json!(dir.join("avatar.png"))),
        ("pip_scale", json!(0.22)),
      ])),
    )
    .await
    .expect("renders with pauses, fade, and PiP");

    let FlowValue::Json(seconds) = &outputs["talk_seconds"] else {
      panic!("seconds");
    };
    // 1.0 + 0.25 + 1.0 + 0.50 = 2.75s
    assert!(
      (seconds.as_f64().expect("f64") - 2.75).abs() < 0.08,
      "expected ~2.75s with breathing pauses, got {seconds}"
    );
    assert!(matches!(outputs["talk_vtt"], FlowValue::File { .. }));
    let FlowValue::File { path: srt_path, .. } = &outputs["talk_subtitles"] else {
      panic!("srt");
    };
    let srt_text = std::fs::read_to_string(srt_path).expect("read srt");
    assert!(
      srt_text.contains("00:00:00,000 --> 00:00:01,000"),
      "first cue ends before 250ms pause: {srt_text}"
    );
    assert!(
      srt_text.contains("00:00:01,250 --> 00:00:02,250"),
      "second cue starts at 1.25s and ends before 500ms slide pause: {srt_text}"
    );
  }
}
