//! Native Office OpenXML (`.pptx`) presentation writer for `SlideDeck`.
//!
//! Maps the deterministic `1920×1080` logical layout of `render_slides` directly
//! onto ECMA-376 / ISO-29500 PresentationML & DrawingML (`12,192,000 × 6,858,000`
//! EMU widescreen `16:9`, exact integer factor `1 px = 6350 EMU`).
//!
//! Every visual primitive — cards, syntax-highlighted code blocks with callout
//! pills, comparison tables (`<a:tbl>`), DAG flowcharts, memory slot diagrams,
//! mind maps, embedded raster/vector figures (`<p:pic>` + `asvg:svgBlip`), and
//! speaker notes (`notesSlideN.xml`) — is emitted as a native editable Office
//! object with zero external process dependencies.

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::io::{Cursor, Write};
use std::path::Path;

use serde_json::Value;
use zip::CompressionMethod;
use zip::write::SimpleFileOptions;

use crate::nodes::render::svg_to_png;
use crate::nodes::render_slides::{
  BulletItem, CodeCallout, FlowEdge, FlowNode, MemPointer, MemRegion, MmPlaced, Palette, Panel,
  Rect, Slide, SlideDeck, box_anchor_pair, char_px, layout_flowchart_boxes, measure_px, place_mm,
  tokenize_code_line, wrap_text, xml_escape,
};

/// 16:9 widescreen slide dimensions in EMU (`13.333" × 7.5"` = `1920 × 6350` by `1080 × 6350`).
const SLIDE_CX: i64 = 12_192_000;
const SLIDE_CY: i64 = 6_858_000;
const NOTES_CX: i64 = 6_858_000;
const NOTES_CY: i64 = 9_144_000;
const PX_TO_EMU: f32 = 6350.0;

/// Safe cross-platform font stack (Arial / Courier New + PingFang SC East Asian slot).
const LATIN_SANS: &str = "Arial";
const LATIN_MONO: &str = "Courier New";
const EA_FONT: &str = "PingFang SC";

#[inline]
fn emu(px: f32) -> i64 {
  (px * PX_TO_EMU).round() as i64
}

#[inline]
fn emu_pos(px: f32) -> i64 {
  emu(px).max(0)
}

/// Convert logical `1920×1080` SVG pixel font size into OpenXML hundredths-of-a-point (`sz`).
/// At `13.333"` width (`960 pt` across `1920 px`), `1 px = 0.50 pt = 50` hundredths of a point.
#[inline]
fn pt100(px_font: f32) -> u32 {
  ((px_font * 50.0).round() as u32).clamp(800, 5400)
}

/// Normalize `#RRGGBB` or `RRGGBB` into strict 6-digit uppercase hex required by `ST_HexColorRGB`.
fn hex6(raw: &str) -> String {
  let trimmed = raw.trim().trim_start_matches('#');
  if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
    trimmed.to_ascii_uppercase()
  } else {
    "64748B".to_string()
  }
}

#[derive(Clone)]
struct TextRun {
  text: String,
  font_px: f32,
  color: String,
  bold: bool,
  italic: bool,
  mono: bool,
}

impl TextRun {
  fn sans(text: impl Into<String>, font_px: f32, color: &str, bold: bool) -> Self {
    Self {
      text: text.into(),
      font_px,
      color: hex6(color),
      bold,
      italic: false,
      mono: false,
    }
  }

  fn mono(text: impl Into<String>, font_px: f32, color: &str, bold: bool) -> Self {
    Self {
      text: text.into(),
      font_px,
      color: hex6(color),
      bold,
      italic: false,
      mono: true,
    }
  }
}

#[derive(Clone)]
struct Paragraph {
  align: &'static str, // "l" | "ctr" | "r"
  space_after_pt100: u32,
  line_pct: u32,
  runs: Vec<TextRun>,
}

impl Paragraph {
  fn single(align: &'static str, run: TextRun) -> Self {
    Self {
      align,
      space_after_pt100: 0,
      line_pct: 120_000,
      runs: vec![run],
    }
  }
}

#[derive(Clone, Copy)]
struct FillStyle<'a> {
  color: &'a str,
  /// Opacity in `0..=100_000` (`100_000` = solid).
  alpha: u32,
}

#[derive(Clone, Copy)]
struct StrokeStyle<'a> {
  color: &'a str,
  width_px: f32,
  alpha: u32,
  dashed: bool,
}

#[derive(Clone, Copy)]
struct TextInsets {
  l_px: f32,
  t_px: f32,
  r_px: f32,
  b_px: f32,
}

impl TextInsets {
  const ZERO: Self = Self {
    l_px: 0.0,
    t_px: 0.0,
    r_px: 0.0,
    b_px: 0.0,
  };

  const CARD: Self = Self {
    l_px: 20.0,
    t_px: 14.0,
    r_px: 18.0,
    b_px: 12.0,
  };
}

struct MediaAsset {
  filename: String,
  ext: &'static str,
  bytes: Vec<u8>,
}

struct SlideRel {
  rid: String,
  rel_type: &'static str,
  target: String,
}

struct SlideXmlBuilder {
  bg_hex: String,
  next_shape_id: u32,
  next_rel_idx: u32,
  rels: Vec<SlideRel>,
  body: String,
}

impl SlideXmlBuilder {
  fn new(bg_color: &str) -> Self {
    Self {
      bg_hex: hex6(bg_color),
      next_shape_id: 2, // id=1 is reserved for root `<p:nvGrpSpPr>`
      next_rel_idx: 2,  // rId1 is reserved for `slideLayout1.xml`
      rels: vec![SlideRel {
        rid: "rId1".to_string(),
        rel_type: "http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout",
        target: "../slideLayouts/slideLayout1.xml".to_string(),
      }],
      body: String::with_capacity(16_384),
    }
  }

  fn alloc_id(&mut self) -> u32 {
    let id = self.next_shape_id;
    self.next_shape_id += 1;
    id
  }

  fn add_rel(&mut self, rel_type: &'static str, target: String) -> String {
    let rid = format!("rId{}", self.next_rel_idx);
    self.next_rel_idx += 1;
    self.rels.push(SlideRel {
      rid: rid.clone(),
      rel_type,
      target,
    });
    rid
  }

  #[allow(clippy::too_many_arguments)]
  fn add_shape(
    &mut self,
    name: &str,
    r: Rect,
    geom: &str,
    round_adj: Option<u32>,
    fill: Option<FillStyle<'_>>,
    stroke: Option<StrokeStyle<'_>>,
    anchor: &str,
    insets: TextInsets,
    autofit: bool,
    paragraphs: &[Paragraph],
  ) {
    let id = self.alloc_id();
    let x = emu_pos(r.x);
    let y = emu_pos(r.y);
    let cx = emu(r.w).max(12_700);
    let cy = emu(r.h).max(12_700);

    let _ = write!(
      self.body,
      "<p:sp>\
        <p:nvSpPr>\
          <p:cNvPr id=\"{id}\" name=\"{}\"/>\
          <p:cNvSpPr/>\
          <p:nvPr/>\
        </p:nvSpPr>\
        <p:spPr>\
          <a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
          <a:prstGeom prst=\"{geom}\">",
      xml_escape(name)
    );
    if let Some(adj) = round_adj {
      let _ = write!(
        self.body,
        "<a:avLst><a:gd name=\"adj\" fmla=\"val {adj}\"/></a:avLst>"
      );
    } else {
      self.body.push_str("<a:avLst/>");
    }
    self.body.push_str("</a:prstGeom>");

    match fill {
      Some(f) if f.alpha > 0 => {
        let col = hex6(f.color);
        if f.alpha >= 100_000 {
          let _ = write!(
            self.body,
            "<a:solidFill><a:srgbClr val=\"{col}\"/></a:solidFill>"
          );
        } else {
          let _ = write!(
            self.body,
            "<a:solidFill><a:srgbClr val=\"{col}\"><a:alpha val=\"{}\"/></a:srgbClr></a:solidFill>",
            f.alpha
          );
        }
      }
      _ => self.body.push_str("<a:noFill/>"),
    }

    if let Some(st) = stroke {
      let w_emu = (st.width_px * 9525.0).round().max(6350.0) as i64;
      let col = hex6(st.color);
      let _ = write!(
        self.body,
        "<a:ln w=\"{w_emu}\"><a:solidFill><a:srgbClr val=\"{col}\">"
      );
      if st.alpha < 100_000 {
        let _ = write!(self.body, "<a:alpha val=\"{}\"/>", st.alpha);
      }
      self.body.push_str("</a:srgbClr></a:solidFill>");
      if st.dashed {
        self.body.push_str("<a:prstDash val=\"dash\"/>");
      }
      self.body.push_str("</a:ln>");
    }

    self.body.push_str("</p:spPr><p:txBody>");
    let fit_xml = if autofit {
      "<a:normAutofit/>"
    } else {
      "<a:noAutofit/>"
    };
    let _ = write!(
      self.body,
      "<a:bodyPr wrap=\"square\" lIns=\"{}\" tIns=\"{}\" rIns=\"{}\" bIns=\"{}\" anchor=\"{anchor}\">{fit_xml}</a:bodyPr><a:lstStyle/>",
      emu_pos(insets.l_px),
      emu_pos(insets.t_px),
      emu_pos(insets.r_px),
      emu_pos(insets.b_px),
    );

    if paragraphs.is_empty() {
      self
        .body
        .push_str("<a:p><a:endParaRPr lang=\"zh-CN\" altLang=\"en-US\"/></a:p>");
    } else {
      for para in paragraphs {
        write_paragraph_xml(&mut self.body, para);
      }
    }
    self.body.push_str("</p:txBody></p:sp>");
  }

  fn add_connector(
    &mut self,
    name: &str,
    p1: (f32, f32),
    p2: (f32, f32),
    stroke: StrokeStyle<'_>,
    arrow_end: bool,
    bent: bool,
  ) {
    let id = self.alloc_id();
    let x1 = emu_pos(p1.0);
    let y1 = emu_pos(p1.1);
    let x2 = emu_pos(p2.0);
    let y2 = emu_pos(p2.1);

    let off_x = x1.min(x2);
    let off_y = y1.min(y2);
    let ext_cx = (x2 - x1).abs().max(6350);
    let ext_cy = (y2 - y1).abs().max(6350);
    let flip_h = if x2 < x1 { " flipH=\"1\"" } else { "" };
    let flip_v = if y2 < y1 { " flipV=\"1\"" } else { "" };
    let prst = if bent && ext_cx > 25_400 && ext_cy > 25_400 {
      "bentConnector3"
    } else {
      "straightConnector1"
    };
    let w_emu = (stroke.width_px * 9525.0).round().max(9525.0) as i64;
    let col = hex6(stroke.color);

    let _ = write!(
      self.body,
      "<p:cxnSp>\
        <p:nvCxnSpPr>\
          <p:cNvPr id=\"{id}\" name=\"{}\"/>\
          <p:cNvCxnSpPr/>\
          <p:nvPr/>\
        </p:nvCxnSpPr>\
        <p:spPr>\
          <a:xfrm{flip_h}{flip_v}><a:off x=\"{off_x}\" y=\"{off_y}\"/><a:ext cx=\"{ext_cx}\" cy=\"{ext_cy}\"/></a:xfrm>\
          <a:prstGeom prst=\"{prst}\"><a:avLst/></a:prstGeom>\
          <a:ln w=\"{w_emu}\">\
            <a:solidFill><a:srgbClr val=\"{col}\"/>",
      xml_escape(name)
    );
    if stroke.alpha < 100_000 {
      // Rewrite srgbClr with alpha child when translucent
      let trim_len = self.body.len() - 2;
      self.body.truncate(trim_len);
      let _ = write!(
        self.body,
        "><a:alpha val=\"{}\"/></a:srgbClr>",
        stroke.alpha
      );
    }
    self.body.push_str("</a:solidFill>");
    if stroke.dashed {
      self.body.push_str("<a:prstDash val=\"dash\"/>");
    }
    if arrow_end {
      self
        .body
        .push_str("<a:tailEnd type=\"triangle\" w=\"med\" len=\"med\"/>");
    }
    self.body.push_str("</a:ln></p:spPr></p:cxnSp>");
  }

  fn add_picture(
    &mut self,
    name: &str,
    r: Rect,
    primary_media_filename: &str,
    svg_media_filename: Option<&str>,
  ) {
    let id = self.alloc_id();
    let rid_img = self.add_rel(
      "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
      format!("../media/{primary_media_filename}"),
    );
    let rid_svg = svg_media_filename.map(|svg_name| {
      self.add_rel(
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image",
        format!("../media/{svg_name}"),
      )
    });

    let x = emu_pos(r.x);
    let y = emu_pos(r.y);
    let cx = emu(r.w).max(25_400);
    let cy = emu(r.h).max(25_400);

    let _ = write!(
      self.body,
      "<p:pic>\
        <p:nvPicPr>\
          <p:cNvPr id=\"{id}\" name=\"{}\"/>\
          <p:cNvPicPr><a:picLocks noChangeAspect=\"1\"/></p:cNvPicPr>\
          <p:nvPr/>\
        </p:nvPicPr>\
        <p:blipFill>\
          <a:blip r:embed=\"{rid_img}\">",
      xml_escape(name)
    );
    if let Some(svg_rel) = rid_svg {
      let _ = write!(
        self.body,
        "<a:extLst>\
          <a:ext uri=\"{{96DAC541-7B7A-43D3-8B79-37D633B846F1}}\">\
            <asvg:svgBlip xmlns:asvg=\"http://schemas.microsoft.com/office/drawing/2016/SVG/main\" r:embed=\"{svg_rel}\"/>\
          </a:ext>\
        </a:extLst>"
      );
    }
    let _ = write!(
      self.body,
      "</a:blip>\
          <a:stretch><a:fillRect/></a:stretch>\
        </p:blipFill>\
        <p:spPr>\
          <a:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm>\
          <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
        </p:spPr>\
      </p:pic>"
    );
  }

  fn add_table(
    &mut self,
    name: &str,
    r: Rect,
    highlight_col: Option<usize>,
    columns: &[String],
    rows: &[Vec<String>],
    p: &Palette,
  ) {
    if columns.is_empty() {
      return;
    }
    let id = self.alloc_id();
    let ncols = columns.len();
    let max_rows = rows.len().min(10);
    let header_h_px = 52.0_f32;
    let body_avail_px = (r.h - header_h_px).max(52.0);
    let row_h_px = if max_rows > 0 {
      (body_avail_px / max_rows as f32).clamp(46.0, 96.0)
    } else {
      52.0
    };
    let total_h_px = header_h_px + row_h_px * max_rows as f32;

    let x = emu_pos(r.x);
    let y = emu_pos(r.y);
    let cx = emu(r.w).max(63_500);
    let cy = emu(total_h_px).max(63_500);
    let col_w = cx / ncols as i64;

    let _ = write!(
      self.body,
      "<p:graphicFrame>\
        <p:nvGraphicFramePr>\
          <p:cNvPr id=\"{id}\" name=\"{}\"/>\
          <p:cNvGraphicFramePr><a:graphicFrameLocks noGrp=\"1\"/></p:cNvGraphicFramePr>\
          <p:nvPr/>\
        </p:nvGraphicFramePr>\
        <p:xfrm><a:off x=\"{x}\" y=\"{y}\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></p:xfrm>\
        <a:graphic>\
          <a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/table\">\
            <a:tbl>\
              <a:tblPr firstRow=\"1\" bandRow=\"1\"/>\
              <a:tblGrid>",
      xml_escape(name)
    );
    for ci in 0..ncols {
      let w = if ci + 1 == ncols {
        cx - col_w * (ncols as i64 - 1)
      } else {
        col_w
      };
      let _ = write!(self.body, "<a:gridCol w=\"{w}\"/>");
    }
    self.body.push_str("</a:tblGrid>");

    // Header row
    let _ = write!(self.body, "<a:tr h=\"{}\">", emu(header_h_px));
    for (ci, col_title) in columns.iter().enumerate() {
      let is_hi = highlight_col == Some(ci);
      let text_col = if is_hi { p.accent } else { p.text };
      let fill_col = if is_hi { p.surface } else { p.surface_alt };
      let border_col = if is_hi { p.accent } else { p.border };
      write_table_cell(
        &mut self.body,
        col_title,
        18.0,
        text_col,
        true,
        fill_col,
        border_col,
      );
    }
    self.body.push_str("</a:tr>");

    // Data rows
    for (ri, row) in rows.iter().take(max_rows).enumerate() {
      let _ = write!(self.body, "<a:tr h=\"{}\">", emu(row_h_px));
      for ci in 0..ncols {
        let cell_str = row.get(ci).map_or("", String::as_str).trim();
        let is_hi = highlight_col == Some(ci);
        let text_col = if ci == 0 {
          p.text
        } else if cell_str.starts_with('✓') || cell_str.starts_with('✅') {
          p.green
        } else if cell_str.starts_with('✗') || cell_str.starts_with('❌') {
          p.rose
        } else if cell_str.starts_with('⚠') {
          p.amber
        } else {
          p.text_sub
        };
        let fill_col = if is_hi || ri % 2 == 1 {
          p.surface_alt
        } else {
          p.surface
        };
        let border_col = if is_hi { p.accent } else { p.border };
        write_table_cell(
          &mut self.body,
          cell_str,
          16.0,
          text_col,
          ci == 0 || is_hi,
          fill_col,
          border_col,
        );
      }
      self.body.push_str("</a:tr>");
    }

    self
      .body
      .push_str("</a:tbl></a:graphicData></a:graphic></p:graphicFrame>");
  }

  fn finish(mut self, notes_slide_idx: Option<usize>) -> (String, String) {
    if let Some(n_idx) = notes_slide_idx {
      self.add_rel(
        "http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesSlide",
        format!("../notesSlides/notesSlide{n_idx}.xml"),
      );
    }
    let slide_xml = format!(
      "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <p:sld xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
             xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
        <p:cSld>\
          <p:bg>\
            <p:bgPr>\
              <a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>\
              <a:effectLst/>\
            </p:bgPr>\
          </p:bg>\
          <p:spTree>\
            <p:nvGrpSpPr>\
              <p:cNvPr id=\"1\" name=\"\"/>\
              <p:cNvGrpSpPr/>\
              <p:nvPr/>\
            </p:nvGrpSpPr>\
            <p:grpSpPr>\
              <a:xfrm>\
                <a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/>\
                <a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/>\
              </a:xfrm>\
            </p:grpSpPr>\
            {}\
          </p:spTree>\
        </p:cSld>\
        <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>\
      </p:sld>",
      self.bg_hex, self.body
    );

    let mut rels_xml = String::from(
      "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    for rel in &self.rels {
      let _ = write!(
        rels_xml,
        "<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/>",
        rel.rid,
        rel.rel_type,
        xml_escape(&rel.target)
      );
    }
    rels_xml.push_str("</Relationships>");
    (slide_xml, rels_xml)
  }
}

fn write_paragraph_xml(out: &mut String, para: &Paragraph) {
  let _ = write!(
    out,
    "<a:p><a:pPr algn=\"{}\"><a:lnSpc><a:spcPct val=\"{}\"/></a:lnSpc>",
    para.align, para.line_pct
  );
  if para.space_after_pt100 > 0 {
    let _ = write!(
      out,
      "<a:spcAft><a:spcPts val=\"{}\"/></a:spcAft>",
      para.space_after_pt100
    );
  }
  out.push_str("<a:buNone/></a:pPr>");

  if para.runs.is_empty() {
    out.push_str("<a:endParaRPr lang=\"zh-CN\" altLang=\"en-US\"/>");
  } else {
    for run in &para.runs {
      if run.text.is_empty() {
        continue;
      }
      let sz = pt100(run.font_px);
      let b = if run.bold { " b=\"1\"" } else { "" };
      let i = if run.italic { " i=\"1\"" } else { "" };
      let latin = if run.mono { LATIN_MONO } else { LATIN_SANS };
      let needs_preserve = run.text.starts_with(char::is_whitespace)
        || run.text.ends_with(char::is_whitespace)
        || run.text.contains("  ");
      let space_attr = if needs_preserve {
        " xml:space=\"preserve\""
      } else {
        ""
      };
      let _ = write!(
        out,
        "<a:r>\
          <a:rPr lang=\"zh-CN\" altLang=\"en-US\" sz=\"{sz}\"{b}{i} dirty=\"0\">\
            <a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill>\
            <a:latin typeface=\"{latin}\"/>\
            <a:ea typeface=\"{EA_FONT}\"/>\
            <a:cs typeface=\"Arial\"/>\
          </a:rPr>\
          <a:t{space_attr}>{}</a:t>\
        </a:r>",
        run.color,
        xml_escape(&run.text)
      );
    }
  }
  out.push_str("</a:p>");
}

fn write_table_cell(
  out: &mut String,
  text: &str,
  font_px: f32,
  text_color: &str,
  bold: bool,
  fill_color: &str,
  border_color: &str,
) {
  let para = Paragraph {
    align: "l",
    space_after_pt100: 0,
    line_pct: 120_000,
    runs: vec![TextRun::sans(text, font_px, text_color, bold)],
  };
  out.push_str("<a:tc><a:txBody><a:bodyPr/><a:lstStyle/>");
  write_paragraph_xml(out, &para);
  out.push_str("</a:txBody>");
  let bcol = hex6(border_color);
  let fcol = hex6(fill_color);
  let mar_lr = emu(14.0);
  let mar_tb = emu(8.0);
  let _ = write!(
    out,
    "<a:tcPr marL=\"{mar_lr}\" marR=\"{mar_lr}\" marT=\"{mar_tb}\" marB=\"{mar_tb}\" anchor=\"ctr\">\
      <a:lnL w=\"9525\"><a:solidFill><a:srgbClr val=\"{bcol}\"/></a:solidFill></a:lnL>\
      <a:lnR w=\"9525\"><a:solidFill><a:srgbClr val=\"{bcol}\"/></a:solidFill></a:lnR>\
      <a:lnT w=\"9525\"><a:solidFill><a:srgbClr val=\"{bcol}\"/></a:solidFill></a:lnT>\
      <a:lnB w=\"9525\"><a:solidFill><a:srgbClr val=\"{bcol}\"/></a:solidFill></a:lnB>\
      <a:solidFill><a:srgbClr val=\"{fcol}\"/></a:solidFill>\
    </a:tcPr></a:tc>"
  );
}

/// Options controlling native `.pptx` generation.
#[derive(Debug, Clone, Copy, Default)]
pub struct PptxExportOptions {
  /// Emit section opener divider slides when `slide.section` transitions.
  pub section_cards: bool,
}

/// Build a complete, ECMA-376 compliant, editable `.pptx` byte archive from `deck`.
pub fn deck_to_pptx_bytes(deck: &SlideDeck, opts: PptxExportOptions) -> Result<Vec<u8>, String> {
  let p = Palette::from_theme(deck.theme.as_deref());

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

  let mut media_assets: Vec<MediaAsset> = Vec::new();
  let mut media_dedup: HashMap<String, usize> = HashMap::new();
  let mut slides_out: Vec<(String, String, Option<String>)> =
    Vec::with_capacity(deck.slides.len() + 4);

  // 1. Cover Slide (`slide1.xml`).
  {
    let sb = build_cover_slide(deck, &sections, &p);
    let cover_notes = deck
      .subtitle
      .as_deref()
      .filter(|s| !s.trim().is_empty())
      .map(|sub| format!("{} — {}", deck.title, sub));
    let notes_idx = cover_notes.as_ref().map(|_| slides_out.len() + 1);
    let (s_xml, r_xml) = sb.finish(notes_idx);
    slides_out.push((s_xml, r_xml, cover_notes));
  }

  // 2. Content Slides (+ optional Section Divider Slides).
  let mut prev_section: Option<String> = None;
  for (si, slide) in deck.slides.iter().enumerate() {
    let cur_section = slide
      .section
      .clone()
      .filter(|sec| !sec.trim().is_empty())
      .unwrap_or_else(|| format!("{:02}", si + 1));
    let emit_section_card = opts.section_cards
      && deck.slides.len() >= 2
      && prev_section.as_deref() != Some(cur_section.as_str());
    prev_section = Some(cur_section.clone());

    if emit_section_card {
      let sec_idx = sections.iter().position(|s| s == &cur_section).unwrap_or(0);
      let sb = build_section_slide(deck, sec_idx, &sections, slide, &p);
      let sec_note = Some(format!("章节过渡：{} · {}", cur_section, slide.title));
      let notes_idx = Some(slides_out.len() + 1);
      let (s_xml, r_xml) = sb.finish(notes_idx);
      slides_out.push((s_xml, r_xml, sec_note));
    }

    let sb = build_content_slide(
      deck,
      si,
      slide,
      &sections,
      &p,
      &mut media_assets,
      &mut media_dedup,
    );
    let speaker_notes = compose_speaker_notes(slide);
    let notes_idx = speaker_notes.as_ref().map(|_| slides_out.len() + 1);
    let (s_xml, r_xml) = sb.finish(notes_idx);
    slides_out.push((s_xml, r_xml, speaker_notes));
  }

  pack_ooxml_zip(deck, &p, &slides_out, &media_assets)
}

fn compose_speaker_notes(slide: &Slide) -> Option<String> {
  let mut buf = String::new();
  if let Some(notes) = slide
    .notes
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    buf.push_str(notes);
  }
  let spoken_steps: Vec<&str> = slide
    .steps
    .iter()
    .map(|st| st.spoken_text().trim())
    .filter(|s| !s.is_empty())
    .collect();
  if !spoken_steps.is_empty() {
    if !buf.is_empty() {
      buf.push_str("\n\n【分段讲演提词】\n");
    }
    for (idx, line) in spoken_steps.iter().enumerate() {
      let _ = writeln!(buf, "{}. {}", idx + 1, line);
    }
  }
  let trimmed = buf.trim().to_string();
  (!trimmed.is_empty()).then_some(trimmed)
}

// ── Cover & Section Divider Builders ────────────────────────────────────

fn build_cover_slide(deck: &SlideDeck, sections: &[String], p: &Palette) -> SlideXmlBuilder {
  let mut sb = SlideXmlBuilder::new(p.bg);

  // Subtle rounded backdrop card on the stage
  sb.add_shape(
    "Cover Frame",
    Rect {
      x: 96.0,
      y: 112.0,
      w: 1728.0,
      h: 856.0,
    },
    "roundRect",
    Some(3500),
    Some(FillStyle {
      color: p.surface,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.5,
      alpha: 100_000,
      dashed: false,
    }),
    "t",
    TextInsets::ZERO,
    false,
    &[],
  );

  // Top brand / topic pill
  let badge_text = format!("KEYNOTE DECK · {} SLIDES", deck.slides.len());
  let pill_w = (measure_px(&badge_text, 15.0, true) + 36.0).clamp(210.0, 420.0);
  sb.add_shape(
    "Cover Badge",
    Rect {
      x: 168.0,
      y: 186.0,
      w: pill_w,
      h: 44.0,
    },
    "roundRect",
    Some(18_000),
    Some(FillStyle {
      color: p.accent,
      alpha: 18_000,
    }),
    Some(StrokeStyle {
      color: p.accent,
      width_px: 1.3,
      alpha: 100_000,
      dashed: false,
    }),
    "ctr",
    TextInsets::ZERO,
    false,
    &[Paragraph::single(
      "ctr",
      TextRun::mono(badge_text, 15.0, p.accent, true),
    )],
  );

  // Main Deck Title
  sb.add_shape(
    "Deck Title",
    Rect {
      x: 168.0,
      y: 268.0,
      w: 1560.0,
      h: 220.0,
    },
    "rect",
    None,
    None,
    None,
    "t",
    TextInsets::ZERO,
    true,
    &[Paragraph {
      align: "l",
      space_after_pt100: 600,
      line_pct: 118_000,
      runs: vec![TextRun::sans(&deck.title, 62.0, p.text, true)],
    }],
  );

  // Subtitle
  if let Some(sub) = deck
    .subtitle
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    sb.add_shape(
      "Deck Subtitle",
      Rect {
        x: 168.0,
        y: 504.0,
        w: 1480.0,
        h: 130.0,
      },
      "rect",
      None,
      None,
      None,
      "t",
      TextInsets::ZERO,
      true,
      &[Paragraph {
        align: "l",
        space_after_pt100: 0,
        line_pct: 130_000,
        runs: vec![TextRun::sans(sub, 28.0, p.text_sub, false)],
      }],
    );
  }

  // Chapter overview pills along lower band
  let mut cx = 168.0_f32;
  for (idx, sec) in sections.iter().take(6).enumerate() {
    let starts_numbered = sec
      .trim_start()
      .chars()
      .next()
      .is_some_and(|c| c.is_ascii_digit());
    let label = if starts_numbered {
      sec.clone()
    } else {
      format!("{:02}  {}", idx + 1, sec)
    };
    let pw = (measure_px(&label, 15.5, false) + 34.0).clamp(120.0, 290.0);
    if cx + pw > 1740.0 {
      break;
    }
    let col = p.tone_color(None, idx);
    sb.add_shape(
      &format!("Section Pill {}", idx + 1),
      Rect {
        x: cx,
        y: 720.0,
        w: pw,
        h: 46.0,
      },
      "roundRect",
      Some(12_000),
      Some(FillStyle {
        color: p.surface_alt,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: col,
        width_px: 1.2,
        alpha: 75_000,
        dashed: false,
      }),
      "ctr",
      TextInsets {
        l_px: 12.0,
        t_px: 4.0,
        r_px: 12.0,
        b_px: 4.0,
      },
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(label, 15.5, p.text_sub, true),
      )],
    );
    cx += pw + 16.0;
  }

  // Author footer
  let author = deck
    .author
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or("Yanshi Content Studio");
  sb.add_shape(
    "Deck Author",
    Rect {
      x: 168.0,
      y: 836.0,
      w: 960.0,
      h: 48.0,
    },
    "rect",
    None,
    None,
    None,
    "ctr",
    TextInsets::ZERO,
    false,
    &[Paragraph::single(
      "l",
      TextRun::sans(author, 18.0, p.text_muted, false),
    )],
  );

  sb
}

fn build_section_slide(
  deck: &SlideDeck,
  sec_idx: usize,
  sections: &[String],
  first_slide: &Slide,
  p: &Palette,
) -> SlideXmlBuilder {
  let mut sb = SlideXmlBuilder::new(p.bg);
  let active_sec = sections.get(sec_idx).map_or("01", String::as_str);

  sb.add_shape(
    "Section Card",
    Rect {
      x: 132.0,
      y: 164.0,
      w: 1656.0,
      h: 752.0,
    },
    "roundRect",
    Some(4000),
    Some(FillStyle {
      color: p.surface,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.5,
      alpha: 100_000,
      dashed: false,
    }),
    "t",
    TextInsets::ZERO,
    false,
    &[],
  );

  let badge = format!("SECTION {:02} / {:02}", sec_idx + 1, sections.len().max(1));
  sb.add_shape(
    "Section Number",
    Rect {
      x: 210.0,
      y: 256.0,
      w: 260.0,
      h: 46.0,
    },
    "roundRect",
    Some(18_000),
    Some(FillStyle {
      color: p.accent,
      alpha: 18_000,
    }),
    Some(StrokeStyle {
      color: p.accent,
      width_px: 1.3,
      alpha: 100_000,
      dashed: false,
    }),
    "ctr",
    TextInsets::ZERO,
    false,
    &[Paragraph::single(
      "ctr",
      TextRun::mono(badge, 15.5, p.accent, true),
    )],
  );

  sb.add_shape(
    "Section Heading",
    Rect {
      x: 210.0,
      y: 336.0,
      w: 1460.0,
      h: 140.0,
    },
    "rect",
    None,
    None,
    None,
    "t",
    TextInsets::ZERO,
    true,
    &[Paragraph::single(
      "l",
      TextRun::sans(active_sec, 54.0, p.text, true),
    )],
  );

  let lead = first_slide
    .subtitle
    .as_deref()
    .filter(|s| !s.trim().is_empty())
    .unwrap_or(first_slide.title.as_str());
  sb.add_shape(
    "Section Lead",
    Rect {
      x: 210.0,
      y: 492.0,
      w: 1420.0,
      h: 110.0,
    },
    "rect",
    None,
    None,
    None,
    "t",
    TextInsets::ZERO,
    true,
    &[Paragraph::single(
      "l",
      TextRun::sans(
        format!("{} · {}", deck.title, lead),
        24.0,
        p.text_sub,
        false,
      ),
    )],
  );

  sb
}

// ── Content Slide & Panel Builders ──────────────────────────────────────

fn build_content_slide(
  deck: &SlideDeck,
  slide_idx: usize,
  slide: &Slide,
  sections: &[String],
  p: &Palette,
  media_assets: &mut Vec<MediaAsset>,
  media_dedup: &mut HashMap<String, usize>,
) -> SlideXmlBuilder {
  let mut sb = SlideXmlBuilder::new(p.bg);

  // 1. Top Navigation Pills (y = 22..56)
  build_top_nav(&mut sb, deck, slide_idx, sections, p);

  // 2. Slide Title & Subtitle (y = 74..168 — clean whitespace, no underline bar)
  let title_h = if slide
    .subtitle
    .as_deref()
    .is_some_and(|s| !s.trim().is_empty())
  {
    58.0
  } else {
    76.0
  };
  sb.add_shape(
    "Slide Title",
    Rect {
      x: 72.0,
      y: 72.0,
      w: 1776.0,
      h: title_h,
    },
    "rect",
    None,
    None,
    None,
    "ctr",
    TextInsets::ZERO,
    true,
    &[Paragraph::single(
      "l",
      TextRun::sans(&slide.title, 38.0, p.text, true),
    )],
  );

  if let Some(sub) = slide
    .subtitle
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    sb.add_shape(
      "Slide Subtitle",
      Rect {
        x: 72.0,
        y: 132.0,
        w: 1776.0,
        h: 36.0,
      },
      "rect",
      None,
      None,
      None,
      "ctr",
      TextInsets::ZERO,
      true,
      &[Paragraph::single(
        "l",
        TextRun::sans(sub, 20.5, p.text_muted, false),
      )],
    );
  }

  // 3. Stage Panels (y = 186..942 when takeaway exists, or 186..1024 otherwise)
  let has_takeaway = slide
    .takeaway
    .as_deref()
    .is_some_and(|t| !t.trim().is_empty());
  let stage_y = 186.0_f32;
  let stage_h = if has_takeaway { 754.0 } else { 836.0 };

  if let Some(right_panel) = &slide.right {
    let ratio = slide.split_ratio.unwrap_or(0.50).clamp(0.32, 0.68);
    let gap = 32.0_f32;
    let total_w = 1776.0_f32;
    let left_w = ((total_w - gap) * ratio).round();
    let right_w = total_w - gap - left_w;

    build_panel(
      &mut sb,
      &slide.left,
      Rect {
        x: 72.0,
        y: stage_y,
        w: left_w,
        h: stage_h,
      },
      p,
      media_assets,
      media_dedup,
    );
    build_panel(
      &mut sb,
      right_panel,
      Rect {
        x: 72.0 + left_w + gap,
        y: stage_y,
        w: right_w,
        h: stage_h,
      },
      p,
      media_assets,
      media_dedup,
    );
  } else {
    build_panel(
      &mut sb,
      &slide.left,
      Rect {
        x: 72.0,
        y: stage_y,
        w: 1776.0,
        h: stage_h,
      },
      p,
      media_assets,
      media_dedup,
    );
  }

  // 4. Bottom Takeaway Callout Card (y = 958..1044)
  if let Some(tk) = slide
    .takeaway
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
  {
    sb.add_shape(
      "Takeaway Box",
      Rect {
        x: 72.0,
        y: 958.0,
        w: 1776.0,
        h: 86.0,
      },
      "roundRect",
      Some(10_000),
      Some(FillStyle {
        color: p.surface,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: p.accent,
        width_px: 1.4,
        alpha: 85_000,
        dashed: false,
      }),
      "ctr",
      TextInsets::ZERO,
      false,
      &[],
    );
    sb.add_shape(
      "Takeaway Badge",
      Rect {
        x: 92.0,
        y: 979.0,
        w: 112.0,
        h: 44.0,
      },
      "roundRect",
      Some(15_000),
      Some(FillStyle {
        color: p.accent,
        alpha: 20_000,
      }),
      Some(StrokeStyle {
        color: p.accent,
        width_px: 1.2,
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets::ZERO,
      false,
      &[Paragraph::single(
        "ctr",
        TextRun::sans("核心要点", 17.0, p.accent, true),
      )],
    );
    sb.add_shape(
      "Takeaway Text",
      Rect {
        x: 222.0,
        y: 966.0,
        w: 1604.0,
        h: 70.0,
      },
      "rect",
      None,
      None,
      None,
      "ctr",
      TextInsets::ZERO,
      true,
      &[Paragraph::single(
        "l",
        TextRun::sans(tk, 20.0, p.text, true),
      )],
    );
  }

  sb
}

fn build_top_nav(
  sb: &mut SlideXmlBuilder,
  deck: &SlideDeck,
  slide_idx: usize,
  sections: &[String],
  p: &Palette,
) {
  let active_section = deck.slides[slide_idx]
    .section
    .clone()
    .filter(|sec| !sec.trim().is_empty())
    .unwrap_or_else(|| format!("{:02}", slide_idx + 1));

  let mut cursor_x = 72.0_f32;
  let deck_short = wrap_text(&deck.title, 340.0, 15.5, 1);
  if let Some(t) = deck_short.first() {
    let bw = (measure_px(t, 15.5, false) + 30.0).clamp(96.0, 368.0);
    sb.add_shape(
      "Deck Tag",
      Rect {
        x: cursor_x,
        y: 22.0,
        w: bw,
        h: 34.0,
      },
      "roundRect",
      Some(14_000),
      Some(FillStyle {
        color: p.surface_alt,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: p.border,
        width_px: 1.0,
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets {
        l_px: 10.0,
        t_px: 2.0,
        r_px: 10.0,
        b_px: 2.0,
      },
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(t, 15.0, p.accent, true),
      )],
    );
    cursor_x += bw + 14.0;
  }

  for (sec_idx, sec) in sections.iter().take(8).enumerate() {
    let is_active = *sec == active_section;
    let display = if is_active {
      sec.clone()
    } else {
      format!("{:02}", sec_idx + 1)
    };
    let max_pw = if is_active { 310.0 } else { 62.0 };
    let short = wrap_text(&display, max_pw - 22.0, 14.0, 1);
    let lbl = short.first().map_or(display.as_str(), String::as_str);
    let pw = (measure_px(lbl, 14.0, false) + 26.0).clamp(46.0, max_pw);
    if cursor_x + pw > 1620.0 {
      break;
    }
    let (fill_c, border_c, txt_c) = if is_active {
      (p.accent, p.accent, "#FFFFFF")
    } else {
      (p.surface, p.border, p.text_muted)
    };
    sb.add_shape(
      &format!("Nav {}", sec_idx + 1),
      Rect {
        x: cursor_x,
        y: 23.0,
        w: pw,
        h: 32.0,
      },
      "roundRect",
      Some(18_000),
      Some(FillStyle {
        color: fill_c,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: border_c,
        width_px: 1.0,
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets::ZERO,
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(lbl, 14.0, txt_c, is_active),
      )],
    );
    cursor_x += pw + 10.0;
  }

  let page_tag = format!("{:02} / {:02}", slide_idx + 1, deck.slides.len());
  sb.add_shape(
    "Slide Counter",
    Rect {
      x: 1712.0,
      y: 22.0,
      w: 136.0,
      h: 34.0,
    },
    "roundRect",
    Some(15_000),
    Some(FillStyle {
      color: p.surface,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.0,
      alpha: 100_000,
      dashed: false,
    }),
    "ctr",
    TextInsets::ZERO,
    false,
    &[Paragraph::single(
      "ctr",
      TextRun::mono(page_tag, 14.5, p.text_muted, true),
    )],
  );
}

fn build_panel_shell(sb: &mut SlideXmlBuilder, r: Rect, title: Option<&str>, p: &Palette) -> Rect {
  sb.add_shape(
    title.unwrap_or("Panel Container"),
    r,
    "roundRect",
    Some(4000),
    Some(FillStyle {
      color: p.surface,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.3,
      alpha: 100_000,
      dashed: false,
    }),
    "t",
    TextInsets::ZERO,
    false,
    &[],
  );

  if let Some(t) = title.map(str::trim).filter(|s| !s.is_empty()) {
    sb.add_shape(
      "Panel Header",
      Rect {
        x: r.x + 22.0,
        y: r.y + 14.0,
        w: (r.w - 44.0).max(80.0),
        h: 32.0,
      },
      "rect",
      None,
      None,
      None,
      "ctr",
      TextInsets::ZERO,
      true,
      &[Paragraph::single("l", TextRun::sans(t, 18.0, p.cyan, true))],
    );
    Rect {
      x: r.x + 20.0,
      y: r.y + 56.0,
      w: (r.w - 40.0).max(80.0),
      h: (r.h - 74.0).max(80.0),
    }
  } else {
    Rect {
      x: r.x + 20.0,
      y: r.y + 20.0,
      w: (r.w - 40.0).max(80.0),
      h: (r.h - 40.0).max(80.0),
    }
  }
}

fn build_panel(
  sb: &mut SlideXmlBuilder,
  panel: &Panel,
  r: Rect,
  p: &Palette,
  media_assets: &mut Vec<MediaAsset>,
  media_dedup: &mut HashMap<String, usize>,
) {
  match panel {
    Panel::Bullets { title, grid, items } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      build_bullets_panel(sb, inner, *grid, items, p);
    }
    Panel::Code {
      title,
      language,
      code,
      callouts,
    } => {
      build_code_panel(
        sb,
        r,
        title.as_deref(),
        language.as_deref(),
        code,
        callouts,
        p,
      );
    }
    Panel::Flowchart {
      title,
      direction,
      nodes,
      edges,
    } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      build_flowchart_panel(
        sb,
        inner,
        direction.as_deref().unwrap_or("TB"),
        nodes,
        edges,
        p,
      );
    }
    Panel::Memory {
      title,
      regions,
      pointers,
    } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      build_memory_panel(sb, inner, regions, pointers, p);
    }
    Panel::Table {
      title,
      highlight_col,
      columns,
      rows,
    } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      sb.add_table("Comparison Table", inner, *highlight_col, columns, rows, p);
    }
    Panel::Mindmap { title, root } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      build_mindmap_panel(sb, inner, root, p);
    }
    Panel::Image {
      title,
      src,
      caption,
    } => {
      let inner = build_panel_shell(sb, r, title.as_deref(), p);
      build_image_panel(
        sb,
        inner,
        src,
        caption.as_deref(),
        p,
        media_assets,
        media_dedup,
      );
    }
  }
}

fn build_bullets_panel(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  grid: bool,
  items: &[BulletItem],
  p: &Palette,
) {
  if items.is_empty() {
    return;
  }
  if grid && items.len() >= 2 {
    let cols = 2_usize;
    let rows = items.len().div_ceil(cols);
    let gap = 16.0_f32;
    let cw = (r.w - gap) / 2.0;
    let ch = ((r.h - gap * (rows.saturating_sub(1) as f32)) / rows as f32).max(92.0);
    for (idx, item) in items.iter().enumerate() {
      let col = idx % cols;
      let row = idx / cols;
      let card_r = Rect {
        x: r.x + col as f32 * (cw + gap),
        y: r.y + row as f32 * (ch + gap),
        w: cw,
        h: ch,
      };
      add_bullet_card_shape(sb, card_r, idx, item, p);
    }
  } else {
    let n = items.len();
    let gap = 14.0_f32;
    let ch = ((r.h - gap * (n.saturating_sub(1) as f32)) / n as f32).clamp(78.0, 182.0);
    for (idx, item) in items.iter().enumerate() {
      let cy = r.y + idx as f32 * (ch + gap);
      if cy + ch > r.y + r.h + 6.0 {
        break;
      }
      add_bullet_card_shape(
        sb,
        Rect {
          x: r.x,
          y: cy,
          w: r.w,
          h: ch,
        },
        idx,
        item,
        p,
      );
    }
  }
}

fn add_bullet_card_shape(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  idx: usize,
  item: &BulletItem,
  p: &Palette,
) {
  let color = p.tone_color(item.tone.as_deref(), idx);
  let mut head_runs = Vec::new();
  if let Some(badge) = item
    .badge
    .as_deref()
    .map(str::trim)
    .filter(|b| !b.is_empty())
  {
    head_runs.push(TextRun::mono(format!("[{badge}]  "), 16.0, color, true));
  }
  head_runs.push(TextRun::sans(&item.heading, 20.5, p.text, true));

  let mut paras = vec![Paragraph {
    align: "l",
    space_after_pt100: if item.body.trim().is_empty() { 0 } else { 350 },
    line_pct: 120_000,
    runs: head_runs,
  }];
  if !item.body.trim().is_empty() {
    paras.push(Paragraph {
      align: "l",
      space_after_pt100: 0,
      line_pct: 128_000,
      runs: vec![TextRun::sans(item.body.trim(), 17.0, p.text_sub, false)],
    });
  }

  sb.add_shape(
    &format!("Card {}", idx + 1),
    r,
    "roundRect",
    Some(6500),
    Some(FillStyle {
      color: p.surface_alt,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color,
      width_px: 1.3,
      alpha: 68_000,
      dashed: false,
    }),
    "ctr",
    TextInsets::CARD,
    true,
    &paras,
  );
}

#[allow(clippy::too_many_arguments)]
fn build_code_panel(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  title: Option<&str>,
  language: Option<&str>,
  code: &str,
  callouts: &[CodeCallout],
  p: &Palette,
) {
  // Outer code window frame
  sb.add_shape(
    "Code Editor Window",
    r,
    "roundRect",
    Some(4500),
    Some(FillStyle {
      color: p.code_bg,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.5,
      alpha: 100_000,
      dashed: false,
    }),
    "t",
    TextInsets::ZERO,
    false,
    &[],
  );

  // Window header bar
  let tab = title
    .filter(|t| !t.trim().is_empty())
    .unwrap_or_else(|| language.unwrap_or("code"));
  let lang_label = language
    .filter(|l| !l.trim().is_empty())
    .map(|l| format!("  ·  {}", l.to_ascii_uppercase()))
    .unwrap_or_default();
  sb.add_shape(
    "Code Window Header",
    Rect {
      x: r.x + 2.0,
      y: r.y + 2.0,
      w: r.w - 4.0,
      h: 44.0,
    },
    "roundRect",
    Some(10_000),
    Some(FillStyle {
      color: "#131C31",
      alpha: 100_000,
    }),
    None,
    "ctr",
    TextInsets {
      l_px: 20.0,
      t_px: 4.0,
      r_px: 20.0,
      b_px: 4.0,
    },
    false,
    &[Paragraph {
      align: "l",
      space_after_pt100: 0,
      line_pct: 115_000,
      runs: vec![
        TextRun::mono("● ", 15.0, "#FB7185", true),
        TextRun::mono("● ", 15.0, "#FBBF24", true),
        TextRun::mono("●   ", 15.0, "#34D399", true),
        TextRun::mono(tab, 15.5, "#CBD5E1", true),
        TextRun::mono(lang_label, 13.5, p.accent, true),
      ],
    }],
  );

  let raw_lines: Vec<&str> = code.lines().collect();
  let count = raw_lines.len().max(1);
  let avail_h = (r.h - 68.0).max(120.0);
  let line_h = (avail_h / count as f32).clamp(21.0, 30.0);
  let font_sz = (line_h * 0.64).clamp(13.5, 18.5);
  let max_visible_lines = ((avail_h / line_h).floor() as usize).max(1);
  let has_callouts = !callouts.is_empty();
  let code_box_w = if has_callouts {
    (r.w * 0.68).max(260.0)
  } else {
    (r.w - 28.0).max(160.0)
  };

  let mut code_paras = Vec::with_capacity(raw_lines.len().min(max_visible_lines));
  for (idx, &raw) in raw_lines.iter().take(max_visible_lines).enumerate() {
    let line_no = idx + 1;
    let mut runs = Vec::new();
    runs.push(TextRun::mono(
      format!("{line_no:>2}  "),
      (font_sz - 1.5).max(11.5),
      "#64748B",
      false,
    ));
    let tokens = tokenize_code_line(raw);
    if tokens.is_empty() {
      runs.push(TextRun::mono(" ", font_sz, "#E2E8F0", false));
    } else {
      for (tok, col) in tokens {
        runs.push(TextRun::mono(tok, font_sz, col, false));
      }
    }
    code_paras.push(Paragraph {
      align: "l",
      space_after_pt100: 0,
      line_pct: 122_000,
      runs,
    });
  }

  sb.add_shape(
    "Code Body",
    Rect {
      x: r.x + 14.0,
      y: r.y + 54.0,
      w: code_box_w,
      h: (r.h - 66.0).max(80.0),
    },
    "rect",
    None,
    None,
    None,
    "t",
    TextInsets {
      l_px: 8.0,
      t_px: 4.0,
      r_px: 8.0,
      b_px: 4.0,
    },
    true,
    &code_paras,
  );

  // Native editable callout pills + dashed connectors
  for (ci, callout) in callouts.iter().enumerate() {
    if callout.line == 0 || callout.line > max_visible_lines {
      continue;
    }
    let badge_col = p.tone_color(callout.tone.as_deref(), ci + 1);
    let max_bw = (r.w * 0.30).clamp(150.0, 270.0);
    let bw = (measure_px(&callout.text, 13.5, false) + 26.0).clamp(120.0, max_bw);
    let bx = r.x + r.w - bw - 16.0;
    let top_y = r.y + 56.0 + (callout.line - 1) as f32 * line_h;
    let mid_y = top_y + line_h * 0.5;
    let raw_line = raw_lines.get(callout.line - 1).copied().unwrap_or("");
    let line_end_x =
      (r.x + 64.0 + raw_line.chars().count() as f32 * char_px('m', font_sz, true)).min(bx - 18.0);
    if bx - line_end_x >= 16.0 {
      sb.add_connector(
        &format!("Callout Leader {}", ci + 1),
        (line_end_x + 6.0, mid_y),
        (bx - 4.0, mid_y),
        StrokeStyle {
          color: badge_col,
          width_px: 1.2,
          alpha: 100_000,
          dashed: true,
        },
        false,
        false,
      );
    }
    sb.add_shape(
      &format!("Code Callout {}", ci + 1),
      Rect {
        x: bx,
        y: top_y + 1.0,
        w: bw,
        h: line_h.max(24.0),
      },
      "roundRect",
      Some(16_000),
      Some(FillStyle {
        color: "#0F172A",
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: badge_col,
        width_px: 1.3,
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets {
        l_px: 8.0,
        t_px: 2.0,
        r_px: 8.0,
        b_px: 2.0,
      },
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(&callout.text, 13.5, badge_col, true),
      )],
    );
  }
}

fn build_flowchart_panel(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  direction: &str,
  nodes: &[FlowNode],
  edges: &[FlowEdge],
  p: &Palette,
) {
  if nodes.is_empty() {
    return;
  }
  let boxes = layout_flowchart_boxes(r, direction, nodes, edges);

  // Connectors first so node cards sit cleanly on top
  for (ei, edge) in edges.iter().enumerate() {
    let (Some(&a), Some(&b)) = (boxes.get(&edge.from), boxes.get(&edge.to)) else {
      continue;
    };
    let (p1, p2) = box_anchor_pair(a, b);
    sb.add_connector(
      &format!("Flow Edge {}", ei + 1),
      p1,
      p2,
      StrokeStyle {
        color: p.cyan,
        width_px: 2.0,
        alpha: 100_000,
        dashed: edge.dashed,
      },
      true,
      true,
    );
    if let Some(lbl) = edge
      .label
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
    {
      let lw = (measure_px(lbl, 13.5, false) + 22.0).clamp(64.0, 210.0);
      let mx = (p1.0 + p2.0) * 0.5;
      let my = (p1.1 + p2.1) * 0.5;
      sb.add_shape(
        &format!("Edge Label {}", ei + 1),
        Rect {
          x: mx - lw * 0.5,
          y: my - 13.0,
          w: lw,
          h: 26.0,
        },
        "roundRect",
        Some(16_000),
        Some(FillStyle {
          color: p.bg,
          alpha: 100_000,
        }),
        Some(StrokeStyle {
          color: p.cyan,
          width_px: 1.0,
          alpha: 85_000,
          dashed: false,
        }),
        "ctr",
        TextInsets::ZERO,
        true,
        &[Paragraph::single(
          "ctr",
          TextRun::sans(lbl, 13.0, p.cyan, true),
        )],
      );
    }
  }

  for (idx, n) in nodes.iter().enumerate() {
    let Some(&nb) = boxes.get(&n.id) else {
      continue;
    };
    let color = p.tone_color(n.tone.as_deref(), idx);
    let mut paras = vec![Paragraph::single(
      "ctr",
      TextRun::sans(&n.label, 18.5, p.text, true),
    )];
    if let Some(sub) = n
      .sublabel
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
    {
      paras.push(Paragraph::single(
        "ctr",
        TextRun::sans(sub, 14.0, p.text_muted, false),
      ));
    }
    sb.add_shape(
      &format!("Flow Node {}", n.id),
      nb,
      "roundRect",
      Some(8500),
      Some(FillStyle {
        color: p.surface_alt,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color,
        width_px: 1.8,
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets {
        l_px: 12.0,
        t_px: 8.0,
        r_px: 12.0,
        b_px: 8.0,
      },
      true,
      &paras,
    );
  }
}

fn build_memory_panel(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  regions: &[MemRegion],
  pointers: &[MemPointer],
  p: &Palette,
) {
  if regions.is_empty() {
    return;
  }
  let cols = regions.len();
  let gap = if cols > 1 { 44.0 } else { 0.0 };
  let rw = (r.w - gap * (cols.saturating_sub(1) as f32)) / cols as f32;
  let mut slot_rects: HashMap<String, Rect> = HashMap::new();

  for (ri, reg) in regions.iter().enumerate() {
    let rx = r.x + ri as f32 * (rw + gap);
    let color = p.tone_color(reg.tone.as_deref(), ri);
    let has_sub = reg
      .subtitle
      .as_deref()
      .is_some_and(|s| !s.trim().is_empty());
    let header_h = if has_sub { 58.0_f32 } else { 46.0_f32 };

    sb.add_shape(
      &format!("Memory Region {}", reg.name),
      Rect {
        x: rx,
        y: r.y,
        w: rw,
        h: r.h,
      },
      "roundRect",
      Some(5000),
      Some(FillStyle {
        color: p.code_bg,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color,
        width_px: 1.6,
        alpha: 100_000,
        dashed: false,
      }),
      "t",
      TextInsets::ZERO,
      false,
      &[],
    );

    let mut hdr_paras = vec![Paragraph::single(
      "l",
      TextRun::sans(&reg.name, 17.0, color, true),
    )];
    if let Some(sub) = reg
      .subtitle
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
    {
      hdr_paras.push(Paragraph::single(
        "l",
        TextRun::mono(sub, 12.5, p.text_muted, false),
      ));
    }
    sb.add_shape(
      &format!("Region Header {}", reg.name),
      Rect {
        x: rx + 2.0,
        y: r.y + 2.0,
        w: (rw - 4.0).max(60.0),
        h: header_h,
      },
      "roundRect",
      Some(9000),
      Some(FillStyle {
        color,
        alpha: 18_000,
      }),
      None,
      "ctr",
      TextInsets {
        l_px: 14.0,
        t_px: 4.0,
        r_px: 12.0,
        b_px: 4.0,
      },
      true,
      &hdr_paras,
    );

    let slot_count = reg.slots.len().max(1);
    let slot_gap = 12.0_f32;
    let avail_h = (r.h - header_h - 28.0).max(80.0);
    let sh = ((avail_h - slot_gap * (slot_count.saturating_sub(1) as f32)) / slot_count as f32)
      .clamp(54.0, 96.0);

    for (si, slot) in reg.slots.iter().enumerate() {
      let sy = r.y + header_h + 14.0 + si as f32 * (sh + slot_gap);
      let sr = Rect {
        x: rx + 10.0,
        y: sy,
        w: (rw - 20.0).max(64.0),
        h: sh,
      };
      slot_rects.insert(slot.id.clone(), sr);

      let mut top_runs = Vec::new();
      if let Some(off) = slot
        .offset
        .as_deref()
        .map(str::trim)
        .filter(|o| !o.is_empty())
      {
        top_runs.push(TextRun::mono(format!("{off}  "), 12.5, p.text_muted, false));
      }
      top_runs.push(TextRun::mono(&slot.label, 15.5, p.text, true));

      let mut slot_paras = vec![Paragraph {
        align: "l",
        space_after_pt100: 150,
        line_pct: 115_000,
        runs: top_runs,
      }];
      if !slot.value.trim().is_empty() {
        slot_paras.push(Paragraph::single(
          "l",
          TextRun::mono(&slot.value, 13.5, color, false),
        ));
      }

      sb.add_shape(
        &format!("Slot {}", slot.id),
        sr,
        "roundRect",
        Some(7000),
        Some(FillStyle {
          color: p.surface_alt,
          alpha: 100_000,
        }),
        Some(StrokeStyle {
          color: p.border,
          width_px: 1.2,
          alpha: 100_000,
          dashed: false,
        }),
        "ctr",
        TextInsets {
          l_px: 12.0,
          t_px: 6.0,
          r_px: 10.0,
          b_px: 6.0,
        },
        true,
        &slot_paras,
      );
    }
  }

  for (pi, ptr) in pointers.iter().enumerate() {
    let (Some(&a), Some(&b)) = (slot_rects.get(&ptr.from), slot_rects.get(&ptr.to)) else {
      continue;
    };
    let stroke = match ptr.tone.as_deref() {
      Some("green") => p.green,
      Some("rust" | "orange" | "accent") => p.accent,
      _ if pi % 2 == 0 => p.accent,
      _ => p.cyan,
    };
    let (p1, p2) = box_anchor_pair(a, b);
    sb.add_connector(
      &format!("Pointer {}", pi + 1),
      p1,
      p2,
      StrokeStyle {
        color: stroke,
        width_px: 2.2,
        alpha: 100_000,
        dashed: false,
      },
      true,
      true,
    );
    if let Some(lbl) = ptr
      .label
      .as_deref()
      .map(str::trim)
      .filter(|l| !l.is_empty())
    {
      let lw = (measure_px(lbl, 13.0, false) + 18.0).clamp(56.0, 170.0);
      let mx = (p1.0 + p2.0) * 0.5;
      let my = (p1.1 + p2.1) * 0.5;
      sb.add_shape(
        &format!("Pointer Label {}", pi + 1),
        Rect {
          x: mx - lw * 0.5,
          y: my - 12.0,
          w: lw,
          h: 24.0,
        },
        "roundRect",
        Some(15_000),
        Some(FillStyle {
          color: p.bg,
          alpha: 100_000,
        }),
        Some(StrokeStyle {
          color: stroke,
          width_px: 1.0,
          alpha: 100_000,
          dashed: false,
        }),
        "ctr",
        TextInsets::ZERO,
        true,
        &[Paragraph::single(
          "ctr",
          TextRun::sans(lbl, 12.5, stroke, true),
        )],
      );
    }
  }
}

fn build_mindmap_panel(sb: &mut SlideXmlBuilder, r: Rect, root: &Value, p: &Palette) {
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
  let node_font = |d: usize| match d {
    0 => 20.0,
    1 => 17.0,
    _ => 15.0,
  };
  let node_pad = |d: usize| if d == 0 { 40.0 } else { 30.0 };
  let node_h = |d: usize| match d {
    0 => 44.0,
    1 => 36.0,
    _ => 32.0,
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
  let mut right_anchor = vec![0.0_f32; max_depth];
  let mut left_anchor = vec![0.0_f32; max_depth];
  if max_depth > 1 {
    right_anchor[1] = root_w * 0.5 + 56.0;
    left_anchor[1] = -root_w * 0.5 - 56.0;
    for d in 2..max_depth {
      right_anchor[d] = right_anchor[d - 1] + right_col_w[d - 1] + 46.0;
      left_anchor[d] = left_anchor[d - 1] - left_col_w[d - 1] - 46.0;
    }
  }

  let mut raw_rects: Vec<(f32, f32, f32, f32)> = Vec::with_capacity(placed.len());
  let (mut min_x, mut max_x, mut min_y, mut max_y) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
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
    raw_rects.push((bx, by, nw, nh));
  }

  let nat_w = (max_x - min_x + 24.0).max(120.0);
  let nat_h = (max_y - min_y + 24.0).max(80.0);
  let scale = (r.w / nat_w).min(r.h / nat_h).clamp(0.45, 1.30);
  let cx_nat = (min_x + max_x) * 0.5;
  let cy_nat = (min_y + max_y) * 0.5;
  let tx = r.x + r.w * 0.5 - cx_nat * scale;
  let ty = r.y + r.h * 0.5 - cy_nat * scale;

  let scaled: Vec<Rect> = raw_rects
    .iter()
    .map(|&(bx, by, nw, nh)| Rect {
      x: tx + bx * scale,
      y: ty + by * scale,
      w: (nw * scale).max(48.0),
      h: (nh * scale).max(26.0),
    })
    .collect();

  for (ni, n) in placed.iter().enumerate() {
    if let Some(pi) = n.parent {
      let pr = scaled[pi];
      let nr = scaled[ni];
      let branch_col = p.tone_color(None, n.branch);
      let p1 = if n.side >= 0 {
        (pr.x + pr.w, pr.y + pr.h * 0.5)
      } else {
        (pr.x, pr.y + pr.h * 0.5)
      };
      let p2 = if n.side >= 0 {
        (nr.x, nr.y + nr.h * 0.5)
      } else {
        (nr.x + nr.w, nr.y + nr.h * 0.5)
      };
      sb.add_connector(
        &format!("Mindmap Branch {ni}"),
        p1,
        p2,
        StrokeStyle {
          color: branch_col,
          width_px: if n.depth == 1 { 2.2 } else { 1.5 },
          alpha: 100_000,
          dashed: false,
        },
        false,
        true,
      );
    }
  }

  for (ni, n) in placed.iter().enumerate() {
    let nr = scaled[ni];
    let branch_col = if n.depth == 0 {
      p.accent
    } else {
      p.tone_color(None, n.branch)
    };
    let (fill, tfill) = if n.depth == 0 {
      (p.accent, "#FFFFFF")
    } else if p.dark {
      (p.surface_alt, p.text)
    } else {
      ("#FFFFFF", p.text)
    };
    let fs = (node_font(n.depth) * scale).clamp(11.0, 22.0);
    sb.add_shape(
      &format!("Mindmap Node {ni}"),
      nr,
      "roundRect",
      Some(14_000),
      Some(FillStyle {
        color: fill,
        alpha: 100_000,
      }),
      Some(StrokeStyle {
        color: branch_col,
        width_px: if n.depth == 0 { 2.0 } else { 1.4 },
        alpha: 100_000,
        dashed: false,
      }),
      "ctr",
      TextInsets {
        l_px: 6.0,
        t_px: 2.0,
        r_px: 6.0,
        b_px: 2.0,
      },
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(&n.label, fs, tfill, n.depth <= 1),
      )],
    );
  }
}

fn build_image_panel(
  sb: &mut SlideXmlBuilder,
  r: Rect,
  src: &str,
  caption: Option<&str>,
  p: &Palette,
  media_assets: &mut Vec<MediaAsset>,
  media_dedup: &mut HashMap<String, usize>,
) {
  let caption_text = caption.map(str::trim).filter(|s| !s.is_empty());
  let caption_h = if caption_text.is_some() { 38.0 } else { 0.0 };
  let frame_h = (r.h - caption_h).max(80.0);

  sb.add_shape(
    "Figure Backdrop",
    Rect {
      x: r.x,
      y: r.y,
      w: r.w,
      h: frame_h,
    },
    "roundRect",
    Some(5000),
    Some(FillStyle {
      color: p.surface_alt,
      alpha: 100_000,
    }),
    Some(StrokeStyle {
      color: p.border,
      width_px: 1.2,
      alpha: 100_000,
      dashed: false,
    }),
    "ctr",
    TextInsets::ZERO,
    false,
    &[],
  );

  if let Some(resolved) = load_run_media(src, media_assets, media_dedup) {
    let pad = 10.0_f32;
    let box_w = (r.w - pad * 2.0).max(40.0);
    let box_h = (frame_h - pad * 2.0).max(40.0);
    let (fit_w, fit_h) = fit_aspect(resolved.aspect_w_over_h, box_w, box_h);
    let pic_rect = Rect {
      x: r.x + pad + (box_w - fit_w) * 0.5,
      y: r.y + pad + (box_h - fit_h) * 0.5,
      w: fit_w,
      h: fit_h,
    };
    sb.add_picture(
      "Slide Figure",
      pic_rect,
      &resolved.primary_filename,
      resolved.svg_filename.as_deref(),
    );
  } else {
    sb.add_shape(
      "Figure Placeholder",
      Rect {
        x: r.x + 16.0,
        y: r.y + 16.0,
        w: (r.w - 32.0).max(40.0),
        h: (frame_h - 32.0).max(40.0),
      },
      "rect",
      None,
      None,
      None,
      "ctr",
      TextInsets::ZERO,
      false,
      &[Paragraph::single(
        "ctr",
        TextRun::sans("插图资源已就绪", 18.0, p.text_muted, false),
      )],
    );
  }

  if let Some(cap) = caption_text {
    sb.add_shape(
      "Figure Caption",
      Rect {
        x: r.x,
        y: r.y + frame_h + 6.0,
        w: r.w,
        h: 32.0,
      },
      "rect",
      None,
      None,
      None,
      "ctr",
      TextInsets::ZERO,
      true,
      &[Paragraph::single(
        "ctr",
        TextRun::sans(cap, 15.0, p.text_sub, false),
      )],
    );
  }
}

struct ResolvedMedia {
  primary_filename: String,
  svg_filename: Option<String>,
  aspect_w_over_h: f32,
}

fn register_media_bytes(
  key: &str,
  ext: &'static str,
  bytes: Vec<u8>,
  media_assets: &mut Vec<MediaAsset>,
  media_dedup: &mut HashMap<String, usize>,
) -> String {
  if let Some(&idx) = media_dedup.get(key) {
    return media_assets[idx].filename.clone();
  }
  let idx = media_assets.len();
  let filename = format!("image{}.{ext}", idx + 1);
  media_assets.push(MediaAsset {
    filename: filename.clone(),
    ext,
    bytes,
  });
  media_dedup.insert(key.to_string(), idx);
  filename
}

fn data_uri_digest_key(bytes: &[u8]) -> String {
  let mut crc = flate2::Crc::new();
  crc.update(bytes);
  let mut fnv: u64 = 0xcbf2_9ce4_8422_2325;
  for &b in bytes {
    fnv ^= u64::from(b);
    fnv = fnv.wrapping_mul(0x0100_0000_01b3);
  }
  format!("data:{}:{:08x}:{fnv:016x}", bytes.len(), crc.sum())
}

fn load_run_media(
  src: &str,
  media_assets: &mut Vec<MediaAsset>,
  media_dedup: &mut HashMap<String, usize>,
) -> Option<ResolvedMedia> {
  let trimmed = src.trim();
  let (raw_bytes, cache_key) = if let Some(rest) = trimmed.strip_prefix("data:image/") {
    let (_meta, b64) = rest.split_once(";base64,")?;
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
    let decoded = B64.decode(b64.trim()).ok()?;
    let key = data_uri_digest_key(&decoded);
    (decoded, key)
  } else {
    let root = yanshi_core::run_context::current_artifact_root()?;
    let canonical_root = root.canonicalize().ok()?;
    let canonical_file = Path::new(trimmed).canonicalize().ok()?;
    if !canonical_file.starts_with(&canonical_root) {
      return None;
    }
    let bytes = std::fs::read(&canonical_file).ok()?;
    (bytes, canonical_file.to_string_lossy().into_owned())
  };

  let mime = crate::common::artifacts::sniff_image_mime(&raw_bytes).unwrap_or("image/png");
  if mime == "image/svg+xml" {
    let svg_str = std::str::from_utf8(&raw_bytes).ok()?;
    let png_bytes = svg_to_png(svg_str, Some(1600)).ok()?;
    let aspect = sniff_raster_dimensions(&png_bytes)
      .map(|(w, h)| w as f32 / (h.max(1) as f32))
      .unwrap_or(16.0 / 9.0);
    let png_name = register_media_bytes(
      &format!("{cache_key}#png"),
      "png",
      png_bytes,
      media_assets,
      media_dedup,
    );
    let svg_name = register_media_bytes(
      &format!("{cache_key}#svg"),
      "svg",
      raw_bytes,
      media_assets,
      media_dedup,
    );
    return Some(ResolvedMedia {
      primary_filename: png_name,
      svg_filename: Some(svg_name),
      aspect_w_over_h: aspect,
    });
  }

  let ext = match mime {
    "image/jpeg" => "jpg",
    "image/gif" => "gif",
    "image/webp" => "webp",
    _ => "png",
  };
  let aspect = sniff_raster_dimensions(&raw_bytes)
    .map(|(w, h)| w as f32 / (h.max(1) as f32))
    .unwrap_or(16.0 / 9.0);
  let primary_filename =
    register_media_bytes(&cache_key, ext, raw_bytes, media_assets, media_dedup);
  Some(ResolvedMedia {
    primary_filename,
    svg_filename: None,
    aspect_w_over_h: aspect,
  })
}

fn fit_aspect(aspect: f32, max_w: f32, max_h: f32) -> (f32, f32) {
  let safe_aspect = if aspect.is_finite() && aspect > 0.05 {
    aspect
  } else {
    16.0 / 9.0
  };
  let box_aspect = max_w / max_h.max(1.0);
  if safe_aspect >= box_aspect {
    (max_w, (max_w / safe_aspect).max(20.0))
  } else {
    ((max_h * safe_aspect).max(20.0), max_h)
  }
}

fn sniff_raster_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
  // PNG: 8-byte signature + length(4) + "IHDR"(4) + width(4 BE) + height(4 BE)
  if bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
    let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    if w > 0 && h > 0 {
      return Some((w, h));
    }
  }
  // GIF87a / GIF89a: 6-byte magic + width(2 LE) + height(2 LE)
  if bytes.len() >= 10 && (bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a")) {
    let w = u16::from_le_bytes([bytes[6], bytes[7]]) as u32;
    let h = u16::from_le_bytes([bytes[8], bytes[9]]) as u32;
    if w > 0 && h > 0 {
      return Some((w, h));
    }
  }
  // JPEG: scan markers for SOF0..SOF3 / SOF5..SOF7 / SOF9..SOF11 / SOF13..SOF15
  if bytes.len() >= 4 && bytes[0] == 0xFF && bytes[1] == 0xD8 {
    let mut i = 2;
    while i + 8 < bytes.len() {
      if bytes[i] != 0xFF {
        i += 1;
        continue;
      }
      while i < bytes.len() && bytes[i] == 0xFF {
        i += 1;
      }
      if i + 7 >= bytes.len() {
        break;
      }
      let marker = bytes[i];
      i += 1;
      if marker == 0xD9 || marker == 0xDA {
        break;
      }
      let seg_len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
      if seg_len < 2 || i + seg_len > bytes.len() {
        break;
      }
      if matches!(
        marker,
        0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
      ) && seg_len >= 7
      {
        let h = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
        let w = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
        if w > 0 && h > 0 {
          return Some((w, h));
        }
      }
      i += seg_len;
    }
  }
  // WebP VP8X / VP8L / VP8
  if bytes.len() >= 30 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
    match &bytes[12..16] {
      b"VP8X" => {
        let w = 1 + u32::from_le_bytes([bytes[24], bytes[25], bytes[26], 0]);
        let h = 1 + u32::from_le_bytes([bytes[27], bytes[28], bytes[29], 0]);
        return Some((w, h));
      }
      b"VP8L" if bytes[20] == 0x2F => {
        let bits = u32::from_le_bytes([bytes[21], bytes[22], bytes[23], bytes[24]]);
        let w = (bits & 0x3FFF) + 1;
        let h = ((bits >> 14) & 0x3FFF) + 1;
        return Some((w, h));
      }
      b"VP8 " if bytes[23] == 0x9D && bytes[24] == 0x01 && bytes[25] == 0x2A => {
        let w = (u16::from_le_bytes([bytes[26], bytes[27]]) & 0x3FFF) as u32;
        let h = (u16::from_le_bytes([bytes[28], bytes[29]]) & 0x3FFF) as u32;
        if w > 0 && h > 0 {
          return Some((w, h));
        }
      }
      _ => {}
    }
  }
  None
}

// ── OOXML Package Assembly ──────────────────────────────────────────────

fn pack_ooxml_zip(
  deck: &SlideDeck,
  p: &Palette,
  slides: &[(String, String, Option<String>)],
  media_assets: &[MediaAsset],
) -> Result<Vec<u8>, String> {
  let cursor = Cursor::new(Vec::with_capacity(65_536));
  let mut zip = zip::ZipWriter::new(cursor);
  let xml_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
  let bin_opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);

  let mut write_entry = |path: &str, data: &[u8], opts: SimpleFileOptions| -> Result<(), String> {
    zip
      .start_file(path, opts)
      .map_err(|e| format!("zip start_file {path}: {e}"))?;
    zip
      .write_all(data)
      .map_err(|e| format!("zip write {path}: {e}"))
  };

  let mut media_exts: BTreeSet<&'static str> = BTreeSet::new();
  for m in media_assets {
    media_exts.insert(m.ext);
  }

  // [Content_Types].xml
  let mut ct = String::from(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\">\
      <Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/>\
      <Default Extension=\"xml\" ContentType=\"application/xml\"/>",
  );
  for ext in media_exts {
    let mime = match ext {
      "jpg" => "image/jpeg",
      "gif" => "image/gif",
      "webp" => "image/webp",
      "svg" => "image/svg+xml",
      _ => "image/png",
    };
    let _ = write!(ct, "<Default Extension=\"{ext}\" ContentType=\"{mime}\"/>");
  }
  ct.push_str(
    "<Override PartName=\"/docProps/core.xml\" ContentType=\"application/vnd.openxmlformats-package.core-properties+xml\"/>\
     <Override PartName=\"/docProps/app.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.extended-properties+xml\"/>\
     <Override PartName=\"/ppt/presentation.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presentation.main+xml\"/>\
     <Override PartName=\"/ppt/presProps.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.presProps+xml\"/>\
     <Override PartName=\"/ppt/viewProps.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.viewProps+xml\"/>\
     <Override PartName=\"/ppt/tableStyles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.tableStyles+xml\"/>\
     <Override PartName=\"/ppt/slideMasters/slideMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideMaster+xml\"/>\
     <Override PartName=\"/ppt/slideLayouts/slideLayout1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slideLayout+xml\"/>\
     <Override PartName=\"/ppt/notesMasters/notesMaster1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesMaster+xml\"/>\
     <Override PartName=\"/ppt/theme/theme1.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>\
     <Override PartName=\"/ppt/theme/theme2.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.theme+xml\"/>",
  );
  for (i, (_, _, notes)) in slides.iter().enumerate() {
    let n = i + 1;
    let _ = write!(
      ct,
      "<Override PartName=\"/ppt/slides/slide{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.slide+xml\"/>"
    );
    if notes.is_some() {
      let _ = write!(
        ct,
        "<Override PartName=\"/ppt/notesSlides/notesSlide{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.presentationml.notesSlide+xml\"/>"
      );
    }
  }
  ct.push_str("</Types>");
  write_entry("[Content_Types].xml", ct.as_bytes(), xml_opts)?;

  // _rels/.rels
  let root_rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
      <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"ppt/presentation.xml\"/>\
      <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties\" Target=\"docProps/core.xml\"/>\
      <Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/extended-properties\" Target=\"docProps/app.xml\"/>\
    </Relationships>";
  write_entry("_rels/.rels", root_rels.as_bytes(), xml_opts)?;

  // docProps/core.xml & docProps/app.xml
  let author = deck.author.as_deref().unwrap_or("Yanshi Studio");
  let core_xml = format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <cp:coreProperties xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" \
                       xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
                       xmlns:dcterms=\"http://purl.org/dc/terms/\" \
                       xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\">\
      <dc:title>{}</dc:title>\
      <dc:creator>{}</dc:creator>\
      <cp:lastModifiedBy>{}</cp:lastModifiedBy>\
    </cp:coreProperties>",
    xml_escape(&deck.title),
    xml_escape(author),
    xml_escape(author),
  );
  write_entry("docProps/core.xml", core_xml.as_bytes(), xml_opts)?;

  let app_xml = format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <Properties xmlns=\"http://schemas.openxmlformats.org/officeDocument/2006/extended-properties\" \
                xmlns:vt=\"http://schemas.openxmlformats.org/officeDocument/2006/docPropsVTypes\">\
      <Application>Yanshi Content Studio</Application>\
      <Slides>{}</Slides>\
      <Notes>{}</Notes>\
    </Properties>",
    slides.len(),
    slides.iter().filter(|(_, _, n)| n.is_some()).count(),
  );
  write_entry("docProps/app.xml", app_xml.as_bytes(), xml_opts)?;

  // ppt/presentation.xml & ppt/_rels/presentation.xml.rels
  let mut pres_rels = String::from(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
      <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"slideMasters/slideMaster1.xml\"/>\
      <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster\" Target=\"notesMasters/notesMaster1.xml\"/>\
      <Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/presProps\" Target=\"presProps.xml\"/>\
      <Relationship Id=\"rId4\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/viewProps\" Target=\"viewProps.xml\"/>\
      <Relationship Id=\"rId5\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/tableStyles\" Target=\"tableStyles.xml\"/>",
  );
  let mut sld_id_lst = String::new();
  for i in 0..slides.len() {
    let n = i + 1;
    let rid = format!("rId{}", 6 + i);
    let sld_id = 256 + i as u32;
    let _ = write!(
      pres_rels,
      "<Relationship Id=\"{rid}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"slides/slide{n}.xml\"/>"
    );
    let _ = write!(sld_id_lst, "<p:sldId id=\"{sld_id}\" r:id=\"{rid}\"/>");
  }
  pres_rels.push_str("</Relationships>");
  write_entry(
    "ppt/_rels/presentation.xml.rels",
    pres_rels.as_bytes(),
    xml_opts,
  )?;

  let pres_xml = format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <p:presentation xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                    xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                    xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" \
                    saveSubsetFonts=\"1\">\
      <p:sldMasterIdLst><p:sldMasterId id=\"2147483648\" r:id=\"rId1\"/></p:sldMasterIdLst>\
      <p:notesMasterIdLst><p:notesMasterId r:id=\"rId2\"/></p:notesMasterIdLst>\
      <p:sldIdLst>{sld_id_lst}</p:sldIdLst>\
      <p:sldSz cx=\"{SLIDE_CX}\" cy=\"{SLIDE_CY}\"/>\
      <p:notesSz cx=\"{NOTES_CX}\" cy=\"{NOTES_CY}\"/>\
      <p:defaultTextStyle>\
        <a:defPPr><a:defRPr lang=\"zh-CN\"/></a:defPPr>\
      </p:defaultTextStyle>\
    </p:presentation>"
  );
  write_entry("ppt/presentation.xml", pres_xml.as_bytes(), xml_opts)?;

  // Auxiliary presentation parts
  write_entry(
    "ppt/presProps.xml",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <p:presentationPr xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                        xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                        xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\"/>",
    xml_opts,
  )?;
  write_entry(
    "ppt/viewProps.xml",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <p:viewPr xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
        <p:normalViewPr><p:restoredLeft sz=\"15620\"/><p:restoredTop sz=\"94660\"/></p:normalViewPr>\
      </p:viewPr>",
    xml_opts,
  )?;
  write_entry(
    "ppt/tableStyles.xml",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <a:tblStyleLst xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" def=\"{5C22544A-7EE6-4342-B048-85BDC9FD1C3A}\"/>",
    xml_opts,
  )?;

  // Two distinct theme parts (`theme1.xml` for slideMaster1, `theme2.xml` for notesMaster1)
  // so no two masters share a theme part in PowerPoint.
  let theme1 = build_theme_xml("Yanshi Slide Theme", p);
  let theme2 = build_theme_xml("Yanshi Notes Theme", p);
  write_entry("ppt/theme/theme1.xml", theme1.as_bytes(), xml_opts)?;
  write_entry("ppt/theme/theme2.xml", theme2.as_bytes(), xml_opts)?;

  // slideMaster1.xml & slideLayout1.xml
  write_entry(
    "ppt/slideMasters/_rels/slideMaster1.xml.rels",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
        <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideLayout\" Target=\"../slideLayouts/slideLayout1.xml\"/>\
        <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme1.xml\"/>\
      </Relationships>",
    xml_opts,
  )?;
  let master_xml = format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <p:sldMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                 xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                 xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
      <p:cSld>\
        <p:bg><p:bgPr><a:solidFill><a:srgbClr val=\"{}\"/></a:solidFill><a:effectLst/></p:bgPr></p:bg>\
        <p:spTree>\
          <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
          <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>\
        </p:spTree>\
      </p:cSld>\
      <p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
      <p:sldLayoutIdLst><p:sldLayoutId id=\"2147483649\" r:id=\"rId1\"/></p:sldLayoutIdLst>\
      <p:txStyles>\
        <p:titleStyle><a:lvl1pPr algn=\"l\"><a:defRPr sz=\"3600\" b=\"1\"/></a:lvl1pPr></p:titleStyle>\
        <p:bodyStyle><a:lvl1pPr algn=\"l\"><a:defRPr sz=\"1800\"/></a:lvl1pPr></p:bodyStyle>\
        <p:otherStyle><a:lvl1pPr algn=\"l\"><a:defRPr sz=\"1600\"/></a:lvl1pPr></p:otherStyle>\
      </p:txStyles>\
    </p:sldMaster>",
    hex6(p.bg)
  );
  write_entry(
    "ppt/slideMasters/slideMaster1.xml",
    master_xml.as_bytes(),
    xml_opts,
  )?;

  write_entry(
    "ppt/slideLayouts/_rels/slideLayout1.xml.rels",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
        <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slideMaster\" Target=\"../slideMasters/slideMaster1.xml\"/>\
      </Relationships>",
    xml_opts,
  )?;
  write_entry(
    "ppt/slideLayouts/slideLayout1.xml",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <p:sldLayout xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                   xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                   xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\" \
                   type=\"blank\" preserve=\"1\">\
        <p:cSld name=\"Blank\">\
          <p:spTree>\
            <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
            <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>\
          </p:spTree>\
        </p:cSld>\
        <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>\
      </p:sldLayout>",
    xml_opts,
  )?;

  // notesMaster1.xml
  write_entry(
    "ppt/notesMasters/_rels/notesMaster1.xml.rels",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
        <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme\" Target=\"../theme/theme2.xml\"/>\
      </Relationships>",
    xml_opts,
  )?;
  write_entry(
    "ppt/notesMasters/notesMaster1.xml",
    b"<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
      <p:notesMaster xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
                     xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
                     xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
        <p:cSld>\
          <p:spTree>\
            <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
            <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>\
          </p:spTree>\
        </p:cSld>\
        <p:clrMap bg1=\"lt1\" tx1=\"dk1\" bg2=\"lt2\" tx2=\"dk2\" accent1=\"accent1\" accent2=\"accent2\" accent3=\"accent3\" accent4=\"accent4\" accent5=\"accent5\" accent6=\"accent6\" hlink=\"hlink\" folHlink=\"folHlink\"/>\
        <p:notesStyle>\
          <a:lvl1pPr algn=\"l\"><a:defRPr sz=\"1200\"/></a:lvl1pPr>\
        </p:notesStyle>\
      </p:notesMaster>",
    xml_opts,
  )?;

  // Slides + NotesSlides
  for (idx, (slide_xml, slide_rels_xml, notes_opt)) in slides.iter().enumerate() {
    let n = idx + 1;
    write_entry(
      &format!("ppt/slides/slide{n}.xml"),
      slide_xml.as_bytes(),
      xml_opts,
    )?;
    write_entry(
      &format!("ppt/slides/_rels/slide{n}.xml.rels"),
      slide_rels_xml.as_bytes(),
      xml_opts,
    )?;
    if let Some(notes_text) = notes_opt {
      let notes_xml = build_notes_slide_xml(notes_text);
      let notes_rels = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
        <Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">\
          <Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/notesMaster\" Target=\"../notesMasters/notesMaster1.xml\"/>\
          <Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/slide\" Target=\"../slides/slide{n}.xml\"/>\
        </Relationships>"
      );
      write_entry(
        &format!("ppt/notesSlides/notesSlide{n}.xml"),
        notes_xml.as_bytes(),
        xml_opts,
      )?;
      write_entry(
        &format!("ppt/notesSlides/_rels/notesSlide{n}.xml.rels"),
        notes_rels.as_bytes(),
        xml_opts,
      )?;
    }
  }

  // Media binaries
  for asset in media_assets {
    write_entry(
      &format!("ppt/media/{}", asset.filename),
      &asset.bytes,
      bin_opts,
    )?;
  }

  let finished = zip.finish().map_err(|e| format!("zip finish error: {e}"))?;
  Ok(finished.into_inner())
}

fn build_notes_slide_xml(notes: &str) -> String {
  let mut paras_xml = String::new();
  for line in notes.lines() {
    let trimmed = line.trim_end();
    if trimmed.is_empty() {
      paras_xml.push_str("<a:p><a:endParaRPr lang=\"zh-CN\" altLang=\"en-US\"/></a:p>");
    } else {
      let p = Paragraph::single("l", TextRun::sans(trimmed, 24.0, "#1E293B", false));
      write_paragraph_xml(&mut paras_xml, &p);
    }
  }
  if paras_xml.is_empty() {
    paras_xml.push_str("<a:p><a:endParaRPr lang=\"zh-CN\" altLang=\"en-US\"/></a:p>");
  }
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <p:notes xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" \
             xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\" \
             xmlns:p=\"http://schemas.openxmlformats.org/presentationml/2006/main\">\
      <p:cSld>\
        <p:spTree>\
          <p:nvGrpSpPr><p:cNvPr id=\"1\" name=\"\"/><p:cNvGrpSpPr/><p:nvPr/></p:nvGrpSpPr>\
          <p:grpSpPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"0\" cy=\"0\"/><a:chOff x=\"0\" y=\"0\"/><a:chExt cx=\"0\" cy=\"0\"/></a:xfrm></p:grpSpPr>\
          <p:sp>\
            <p:nvSpPr>\
              <p:cNvPr id=\"2\" name=\"Slide Image Placeholder\"/>\
              <p:cNvSpPr><a:spLocks noGrp=\"1\" noRot=\"1\" noChangeAspect=\"1\"/></p:cNvSpPr>\
              <p:nvPr><p:ph type=\"sldImg\"/></p:nvPr>\
            </p:nvSpPr>\
            <p:spPr>\
              <a:xfrm><a:off x=\"685800\" y=\"457200\"/><a:ext cx=\"5486400\" cy=\"3086100\"/></a:xfrm>\
              <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
            </p:spPr>\
          </p:sp>\
          <p:sp>\
            <p:nvSpPr>\
              <p:cNvPr id=\"3\" name=\"Notes Placeholder\"/>\
              <p:cNvSpPr><a:spLocks noGrp=\"1\"/></p:cNvSpPr>\
              <p:nvPr><p:ph type=\"body\" idx=\"1\"/></p:nvPr>\
            </p:nvSpPr>\
            <p:spPr>\
              <a:xfrm><a:off x=\"685800\" y=\"3886200\"/><a:ext cx=\"5486400\" cy=\"4572000\"/></a:xfrm>\
              <a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom>\
            </p:spPr>\
            <p:txBody>\
              <a:bodyPr/>\
              <a:lstStyle/>\
              {paras_xml}\
            </p:txBody>\
          </p:sp>\
        </p:spTree>\
      </p:cSld>\
      <p:clrMapOvr><a:masterClrMapping/></p:clrMapOvr>\
    </p:notes>"
  )
}

fn build_theme_xml(name: &str, p: &Palette) -> String {
  format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
    <a:theme xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\" name=\"{}\">\
      <a:themeElements>\
        <a:clrScheme name=\"Yanshi\">\
          <a:dk1><a:srgbClr val=\"0F172A\"/></a:dk1>\
          <a:lt1><a:srgbClr val=\"F8FAFC\"/></a:lt1>\
          <a:dk2><a:srgbClr val=\"1E293B\"/></a:dk2>\
          <a:lt2><a:srgbClr val=\"E2E8F0\"/></a:lt2>\
          <a:accent1><a:srgbClr val=\"{}\"/></a:accent1>\
          <a:accent2><a:srgbClr val=\"{}\"/></a:accent2>\
          <a:accent3><a:srgbClr val=\"{}\"/></a:accent3>\
          <a:accent4><a:srgbClr val=\"{}\"/></a:accent4>\
          <a:accent5><a:srgbClr val=\"{}\"/></a:accent5>\
          <a:accent6><a:srgbClr val=\"{}\"/></a:accent6>\
          <a:hlink><a:srgbClr val=\"{}\"/></a:hlink>\
          <a:folHlink><a:srgbClr val=\"{}\"/></a:folHlink>\
        </a:clrScheme>\
        <a:fontScheme name=\"YanshiFonts\">\
          <a:majorFont>\
            <a:latin typeface=\"{LATIN_SANS}\"/>\
            <a:ea typeface=\"{EA_FONT}\"/>\
            <a:cs typeface=\"Arial\"/>\
          </a:majorFont>\
          <a:minorFont>\
            <a:latin typeface=\"{LATIN_SANS}\"/>\
            <a:ea typeface=\"{EA_FONT}\"/>\
            <a:cs typeface=\"Arial\"/>\
          </a:minorFont>\
        </a:fontScheme>\
        <a:fmtScheme name=\"YanshiFmt\">\
          <a:fillStyleLst>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
          </a:fillStyleLst>\
          <a:lnStyleLst>\
            <a:ln w=\"9525\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>\
            <a:ln w=\"19050\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>\
            <a:ln w=\"28575\" cap=\"flat\" cmpd=\"sng\" algn=\"ctr\"><a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill><a:prstDash val=\"solid\"/></a:ln>\
          </a:lnStyleLst>\
          <a:effectStyleLst>\
            <a:effectStyle><a:effectLst/></a:effectStyle>\
            <a:effectStyle><a:effectLst/></a:effectStyle>\
            <a:effectStyle><a:effectLst/></a:effectStyle>\
          </a:effectStyleLst>\
          <a:bgFillStyleLst>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
            <a:solidFill><a:schemeClr val=\"phClr\"/></a:solidFill>\
          </a:bgFillStyleLst>\
        </a:fmtScheme>\
      </a:themeElements>\
      <a:objectDefaults/>\
      <a:extraClrSchemeLst/>\
    </a:theme>",
    xml_escape(name),
    hex6(p.accent),
    hex6(p.cyan),
    hex6(p.green),
    hex6(p.violet),
    hex6(p.amber),
    hex6(p.rose),
    hex6(p.cyan),
    hex6(p.violet),
  )
}

#[cfg(test)]
mod tests {
  use std::io::Read as _;

  use serde_json::json;
  use yanshi_core::run_context;

  use super::*;

  #[tokio::test]
  async fn builds_editable_ooxml_pptx_covering_all_panels_svg_and_notes() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let fig_dir = tmp.path().join("artifacts/fig");
    std::fs::create_dir_all(&fig_dir).expect("fig dir");
    let svg_path = fig_dir.join("arch.svg");
    std::fs::write(
      &svg_path,
      r##"<svg xmlns="http://www.w3.org/2000/svg" width="800" height="450"><rect width="800" height="450" fill="#0F172A"/><text x="80" y="220" font-size="36" fill="#38BDF8">Zero-Copy Pipeline</text></svg>"##,
    )
    .expect("write svg");

    let deck_val = json!({
      "title": "Rust 零成本抽象与系统架构",
      "subtitle": "原生可编辑 OpenXML 幻灯片全组件测试",
      "author": "Yanshi Architecture Team",
      "theme": "dark",
      "slides": [
        {
          "section": "01 核心机制",
          "title": "静态单态化 vs 动态虚表",
          "subtitle": "代码语法高亮、批注胶囊与内存胖指针布局",
          "left": {
            "kind": "code",
            "title": "dispatch.rs",
            "language": "rust",
            "code": "pub trait Compute {\n  fn run(&self, x: i32) -> i32;\n}\n\npub fn exec_dyn(op: &dyn Compute) -> i32 {\n  op.run(42) // vtable 间接寻址\n}",
            "callouts": [
              {"line": 1, "text": "零字段行为契约", "tone": "rust"},
              {"line": 6, "text": "胖指针双槽间接调用", "tone": "cyan"}
            ]
          },
          "right": {
            "kind": "memory",
            "title": "16-Byte Fat Pointer",
            "regions": [
              {
                "name": "Stack",
                "subtitle": "&dyn Compute",
                "tone": "cyan",
                "slots": [
                  {"id": "dp", "offset": "+0x00", "label": "data_ptr", "value": "0x7ffee410"},
                  {"id": "vp", "offset": "+0x08", "label": "vtable_ptr", "value": "0x1004a200"}
                ]
              },
              {
                "name": "RODATA",
                "subtitle": "vtable",
                "tone": "rust",
                "slots": [
                  {"id": "vt", "offset": "+0x18", "label": "Compute::run", "value": "fn_addr"}
                ]
              }
            ],
            "pointers": [
              {"from": "vp", "to": "vt", "label": "vtable", "tone": "rust"}
            ]
          },
          "takeaway": "泛型单态化消除虚表跳转，Trait Object 提供运行期异构集合能力。",
          "notes": "第一页重点对比编译期展开与运行期双指针内存结构。"
        },
        {
          "section": "02 架构与对比",
          "title": "编译管线、选型矩阵与矢量插图",
          "left": {
            "kind": "table",
            "title": "分发方式工程权衡",
            "highlight_col": 1,
            "columns": ["维度", "impl Trait", "&dyn Trait"],
            "rows": [
              ["内联优化", "✓ 支持 LLVM 跨函数内联", "✗ 间接调用阻断内联"],
              ["二进制体积", "⚠ 按类型膨胀", "✓ 单份机器码"]
            ]
          },
          "right": {
            "kind": "image",
            "title": "系统核心架构图 (SVG)",
            "src": svg_path.to_string_lossy(),
            "caption": "D2 矢量架构图双层嵌入（PNG + asvg:svgBlip）"
          },
          "takeaway": "热路径选泛型，插件边界选 dyn Trait。",
          "notes": "第二页展示原生 PPT 表格与矢量 SVG 配图。"
        },
        {
          "section": "02 架构与对比",
          "title": "DAG 流程与知识全景导图",
          "left": {
            "kind": "flowchart",
            "title": "执行流水线",
            "direction": "TB",
            "nodes": [
              {"id": "n1", "label": "Sources", "sublabel": "URL / Repo / PDF", "tone": "cyan"},
              {"id": "n2", "label": "Planner", "sublabel": "Structured Deck", "tone": "rust"},
              {"id": "n3", "label": "OOXML PPTX", "sublabel": "Editable Shapes", "tone": "green"}
            ],
            "edges": [
              {"from": "n1", "to": "n2", "label": "extract"},
              {"from": "n2", "to": "n3", "label": "render", "dashed": true}
            ]
          },
          "right": {
            "kind": "mindmap",
            "title": "知识脉络",
            "root": {
              "label": "Content Studio",
              "children": [
                {"label": "PPTX", "children": [{"label": "Shapes"}, {"label": "Tables"}]},
                {"label": "Video", "children": [{"label": "1080p"}, {"label": "TTS"}]}
              ]
            }
          },
          "notes": "第三页验证原生连接线与导图节点。"
        }
      ]
    });

    let deck: SlideDeck = serde_json::from_value(deck_val).expect("valid SlideDeck");
    let pptx_bytes = run_context::with_artifact_root(tmp.path().to_path_buf(), async {
      deck_to_pptx_bytes(
        &deck,
        PptxExportOptions {
          section_cards: true,
        },
      )
    })
    .await
    .expect("pptx bytes");
    if let Ok(dump_path) = std::env::var("YANSHI_DUMP_SAMPLE_PPTX") {
      std::fs::write(dump_path, &pptx_bytes).expect("dump sample pptx");
    }

    let mut archive = zip::ZipArchive::new(Cursor::new(&pptx_bytes)).expect("valid zip");
    let names: Vec<String> = archive
      .file_names()
      .map(|s| s.expect("utf-8").to_string())
      .collect();

    // 1 cover + 2 section cards ("01 核心机制", "02 架构与对比") + 3 content slides = 6 slides
    let slide_count = names
      .iter()
      .filter(|n| n.starts_with("ppt/slides/slide") && n.ends_with(".xml"))
      .count();
    assert_eq!(slide_count, 6, "unexpected slide count in {names:?}");

    // Embedded both raster fallback PNG and vector SVG extension
    assert!(
      names
        .iter()
        .any(|n| n.starts_with("ppt/media/") && n.ends_with(".png"))
    );
    assert!(
      names
        .iter()
        .any(|n| n.starts_with("ppt/media/") && n.ends_with(".svg"))
    );

    // Verify native editable text exists inside slide XML (not flattened into a bitmap!)
    let mut all_slides_xml = String::new();
    for i in 1..=slide_count {
      archive
        .by_name(&format!("ppt/slides/slide{i}.xml"))
        .expect("slide xml")
        .read_to_string(&mut all_slides_xml)
        .expect("utf-8");
    }
    assert!(all_slides_xml.contains("静态单态化 vs 动态虚表"));
    assert!(all_slides_xml.contains("Compute"));
    assert!(all_slides_xml.contains("胖指针双槽间接调用"));
    assert!(all_slides_xml.contains("<a:tbl>"));
    assert!(all_slides_xml.contains("<p:cxnSp>"));
    assert!(all_slides_xml.contains("asvg:svgBlip"));

    // Verify speaker notes live in native notesSlides
    let mut all_notes_xml = String::new();
    for name in names
      .iter()
      .filter(|n| n.starts_with("ppt/notesSlides/notesSlide") && n.ends_with(".xml"))
    {
      archive
        .by_name(name)
        .expect("notes xml")
        .read_to_string(&mut all_notes_xml)
        .expect("utf-8");
    }
    assert!(all_notes_xml.contains("第一页重点对比编译期展开与运行期双指针内存结构"));
  }

  #[test]
  fn distinct_data_uri_images_sharing_64_byte_prefix_do_not_collide_in_media_dedup() {
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
    let svg_a = format!(
      "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"120\"><!-- {} --><rect width=\"200\" height=\"120\" fill=\"#112233\"/></svg>",
      "A".repeat(96)
    );
    let svg_b = format!(
      "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"200\" height=\"120\"><!-- {} --><rect width=\"200\" height=\"120\" fill=\"#994422\"/></svg>",
      "A".repeat(96)
    );
    let uri_a = format!("data:image/svg+xml;base64,{}", B64.encode(svg_a.as_bytes()));
    let uri_b = format!("data:image/svg+xml;base64,{}", B64.encode(svg_b.as_bytes()));

    let mut media_assets = Vec::new();
    let mut media_dedup = HashMap::new();
    let res_a = load_run_media(&uri_a, &mut media_assets, &mut media_dedup).expect("media A");
    let res_b = load_run_media(&uri_b, &mut media_assets, &mut media_dedup).expect("media B");
    assert_ne!(
      res_a.primary_filename, res_b.primary_filename,
      "distinct images sharing identical first 64 base64 chars must not collide"
    );
  }
}
