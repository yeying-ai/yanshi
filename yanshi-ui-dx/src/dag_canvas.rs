//! Pure-Rust interactive layered SVG + HTML DAG Canvas replacing `@xyflow/react`.
//!
//! Features:
//! - Deterministic left-to-right topological column layout with vertical centering.
//! - Smooth cubic-Bezier SVG edges with color-coded live telemetry, condition dashed
//!   edges, edge labels, arrowheads, and click-to-select + disconnect.
//! - Pan (background pointer drag), zoom (+/- / Fit / 100%), and bottom-right MiniMap.
//! - Editable mode: click-to-connect source→target ports, 27-item categorized node
//!   palette with auto-chain option, and node deletion.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_shadcn::{Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Checkbox};

use crate::studio_api::{NodeRuntimeTelemetry, StudioNodeRunStatus, format_usd_badge};
use crate::studio_yaml::{
  InspectedEdge, InspectedEdgeKind, InspectedNode, NODE_PALETTE_CATEGORIES, node_palette,
};

const COL_WIDTH: f64 = 344.0;
const ROW_HEIGHT: f64 = 164.0;
const NODE_W: f64 = 248.0;
const NODE_H: f64 = 112.0;
const WORLD_PAD: f64 = 64.0;

struct NodePaletteStyle {
  label_suffix: &'static str,
  accent: &'static str,
  tint: &'static str,
}

fn node_type_style(node_type: &str) -> NodePaletteStyle {
  let n = node_type.to_ascii_lowercase();
  if n == "llm" || n.contains("agent") {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#6d4aff",
      tint: "#f3effe",
    };
  }
  if n == "render_slides" || n == "render_study" {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#0b7a8f",
      tint: "#e8f7fa",
    };
  }
  if n == "tts" || n == "render_audio" || n == "asr" {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#0d8a6a",
      tint: "#e7f8f2",
    };
  }
  if n == "map" || n == "while" {
    return NodePaletteStyle {
      label_suffix: " 并发子流",
      accent: "#0f766e",
      tint: "#ecfdf5",
    };
  }
  if n == "render_video" || n == "text_to_video" {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#c84b24",
      tint: "#fff1ec",
    };
  }
  if matches!(
    n.as_str(),
    "illustration" | "text_to_image" | "image_gen" | "image_edit"
  ) {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#be185d",
      tint: "#fdf2f8",
    };
  }
  if matches!(
    n.as_str(),
    "render_document"
      | "render_markdown"
      | "render_mindmap"
      | "render_table"
      | "render_svg"
      | "figure"
      | "markmap"
  ) {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#3b5bdb",
      tint: "#edf2ff",
    };
  }
  if matches!(
    n.as_str(),
    "web_search" | "repo_snapshot" | "pdf_extract" | "video_ingest" | "arxiv"
  ) {
    return NodePaletteStyle {
      label_suffix: "",
      accent: "#b45309",
      tint: "#fef3c7",
    };
  }
  NodePaletteStyle {
    label_suffix: "",
    accent: "#475569",
    tint: "#f1f5f9",
  }
}

fn status_ring_class(status: StudioNodeRunStatus, is_selected: bool) -> &'static str {
  if is_selected {
    return "border-primary ring-2 ring-primary/35 shadow-md";
  }
  match status {
    StudioNodeRunStatus::Running => "border-blue-500 ring-2 ring-blue-500/25 shadow-sm",
    StudioNodeRunStatus::PausedBefore | StudioNodeRunStatus::PausedAfter => {
      "border-amber-500 ring-2 ring-amber-500/30 shadow-sm"
    }
    StudioNodeRunStatus::Succeeded => "border-emerald-500/80",
    StudioNodeRunStatus::Failed => "border-destructive ring-2 ring-destructive/25",
    StudioNodeRunStatus::Skipped => "border-muted-foreground/40 opacity-80",
    StudioNodeRunStatus::Idle => "border-border hover:border-foreground/30",
  }
}

fn status_badge_class(status: StudioNodeRunStatus) -> &'static str {
  match status {
    StudioNodeRunStatus::Running => "text-blue-600 dark:text-blue-400 font-semibold",
    StudioNodeRunStatus::PausedBefore | StudioNodeRunStatus::PausedAfter => {
      "text-amber-600 dark:text-amber-400 font-semibold"
    }
    StudioNodeRunStatus::Succeeded => "text-emerald-600 dark:text-emerald-400 font-medium",
    StudioNodeRunStatus::Failed => "text-destructive font-semibold",
    StudioNodeRunStatus::Skipped => "text-muted-foreground",
    StudioNodeRunStatus::Idle => "text-muted-foreground",
  }
}

fn minimap_node_fill(status: StudioNodeRunStatus) -> &'static str {
  match status {
    StudioNodeRunStatus::PausedBefore | StudioNodeRunStatus::PausedAfter => "#f59e0b",
    StudioNodeRunStatus::Running => "#2563eb",
    StudioNodeRunStatus::Succeeded => "#10b981",
    StudioNodeRunStatus::Failed => "#ef4444",
    StudioNodeRunStatus::Skipped => "#94a3b8",
    StudioNodeRunStatus::Idle => "#94a3b8",
  }
}

/// Computes deterministic `(x, y)` coordinates per node plus total world `(w, h)`.
pub fn compute_dag_layout(
  nodes: &[InspectedNode],
  levels: &[Vec<String>],
) -> (HashMap<String, (f64, f64)>, f64, f64) {
  let max_rows = levels.iter().map(Vec::len).max().unwrap_or(1).max(1);
  let max_span_h = (max_rows.saturating_sub(1)) as f64 * ROW_HEIGHT;
  let mut positions: HashMap<String, (f64, f64)> = HashMap::new();

  for (col_idx, col_ids) in levels.iter().enumerate() {
    let col_span_h = (col_ids.len().saturating_sub(1)) as f64 * ROW_HEIGHT;
    let offset_y = (max_span_h - col_span_h) / 2.0 + WORLD_PAD;
    for (row_idx, id) in col_ids.iter().enumerate() {
      positions.insert(
        id.clone(),
        (
          col_idx as f64 * COL_WIDTH + WORLD_PAD,
          offset_y + row_idx as f64 * ROW_HEIGHT,
        ),
      );
    }
  }

  for (fallback_idx, node) in nodes.iter().enumerate() {
    positions.entry(node.id.clone()).or_insert_with(|| {
      (
        (fallback_idx % 4) as f64 * COL_WIDTH + WORLD_PAD,
        (fallback_idx / 4) as f64 * ROW_HEIGHT + WORLD_PAD,
      )
    });
  }

  let mut max_x = WORLD_PAD * 2.0 + NODE_W;
  let mut max_y = WORLD_PAD * 2.0 + NODE_H;
  for &(x, y) in positions.values() {
    max_x = max_x.max(x + NODE_W + WORLD_PAD * 1.5);
    max_y = max_y.max(y + NODE_H + WORLD_PAD * 1.5);
  }
  (positions, max_x, max_y)
}

#[component]
pub fn WorkflowGraphCanvas(
  nodes: Vec<InspectedNode>,
  edges: Vec<InspectedEdge>,
  levels: Vec<Vec<String>>,
  execution_order: Vec<String>,
  telemetry_by_node: HashMap<String, NodeRuntimeTelemetry>,
  breakpoints: Vec<String>,
  selected_node_id: Option<String>,
  #[props(default)] editable: bool,
  on_select_node: EventHandler<String>,
  on_toggle_breakpoint: EventHandler<String>,
  #[props(default)] on_add_node: Option<EventHandler<(String, Option<String>)>>,
  #[props(default)] on_connect_nodes: Option<EventHandler<(String, String)>>,
  #[props(default)] on_disconnect_edge: Option<EventHandler<(String, String)>>,
  #[props(default)] on_delete_node: Option<EventHandler<String>>,
) -> Element {
  let mut palette_open = use_signal(|| false);
  let mut palette_category = use_signal(|| NODE_PALETTE_CATEGORIES[0].to_string());
  let mut chain_after_selected = use_signal(|| true);
  let mut selected_edge = use_signal(|| None::<(String, String)>);
  let mut connecting_from = use_signal(|| None::<String>);

  // Viewport transform: pan (px) + zoom scale (auto-scaled so 3-4 column DAGs fit cleanly).
  let init_zoom = if levels.len() >= 4 {
    0.60_f64
  } else if levels.len() >= 3 {
    0.66_f64
  } else {
    0.88_f64
  };
  let mut zoom = use_signal(move || init_zoom);
  let mut pan_x = use_signal(|| 6.0_f64);
  let mut pan_y = use_signal(|| 24.0_f64);
  let mut dragging = use_signal(|| false);
  let mut drag_origin = use_signal(|| (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64));

  if nodes.is_empty() {
    return rsx! {
      div {
        class: "h-full flex flex-col items-center justify-center gap-3 p-6 text-center text-sm text-muted-foreground",
        "data-testid": "workflow-graph-canvas-empty",
        p { "当前 YAML 尚未解析出节点，请在左侧选择预设模板、修复语法错误，或直接添加首个节点。" }
        if editable {
          if let Some(add_cb) = on_add_node {
            div { class: "flex items-center gap-2",
              Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                onclick: move |_| add_cb.call(("llm".to_string(), None)),
                "+ 添加 LLM 节点起步"
              }
              Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                onclick: move |_| add_cb.call(("template".to_string(), None)),
                "+ 添加 Template 节点起步"
              }
            }
          }
        }
      }
    };
  }

  let bp_set: HashSet<&str> = breakpoints.iter().map(String::as_str).collect();
  let order_map: HashMap<&str, usize> = execution_order
    .iter()
    .enumerate()
    .map(|(i, id)| (id.as_str(), i))
    .collect();

  let (positions, world_w, world_h) = compute_dag_layout(&nodes, &levels);
  let palette_items: Vec<_> = node_palette()
    .into_iter()
    .filter(|item| item.category == palette_category())
    .collect();

  let zoom_pct = (zoom() * 100.0).round() as i32;
  let cur_selected_edge = selected_edge();
  let cur_connecting = connecting_from();

  rsx! {
    div {
      class: "relative h-full w-full overflow-clip bg-muted/20 select-none",
      style: "background-image: radial-gradient(circle, rgba(148,163,184,0.38) 1.1px, transparent 1.1px); background-size: 20px 20px;",
      "data-testid": "workflow-graph-canvas",
      onmousedown: move |ev: MouseEvent| {
        // Start panning when dragging on canvas background
        let coords = ev.client_coordinates();
        dragging.set(true);
        drag_origin.set((coords.x, coords.y, pan_x(), pan_y()));
      },
      onmousemove: move |ev: MouseEvent| {
        if dragging() {
          let coords = ev.client_coordinates();
          let (sx, sy, px, py) = drag_origin();
          pan_x.set(px + (coords.x - sx));
          pan_y.set(py + (coords.y - sy));
        }
      },
      onmouseup: move |_| dragging.set(false),
      onmouseleave: move |_| dragging.set(false),
      onclick: move |_| {
        selected_edge.set(None);
        if connecting_from.peek().is_some() {
          connecting_from.set(None);
        }
      },

      // Top-left legend & connection status hint
      div {
        class: "absolute top-2.5 left-2.5 z-20 flex flex-wrap items-center gap-2 rounded-md border border-border bg-card/90 backdrop-blur px-2.5 py-1 text-[11px] text-muted-foreground shadow-xs",
        onclick: move |ev| ev.stop_propagation(),
        span { class: "inline-flex items-center gap-1",
          span { class: "inline-block w-3 h-0.5 bg-slate-400" }
          "数据映射边"
        }
        span { class: "inline-flex items-center gap-1",
          span { class: "inline-block w-3 border-t-2 border-dashed border-amber-500" }
          "run_if 条件边"
        }
        span { class: "inline-flex items-center gap-1",
          span { class: "inline-block w-2 h-2 rounded-full bg-rose-500" }
          if editable {
            "点右端口连线 · 点红点设断点"
          } else {
            "点右上圆点设执行前断点"
          }
        }
        if let Some(src) = cur_connecting.clone() {
          Badge { variant: BadgeVariant::Default, class: "text-[11px]".to_string(),
            "正在从 #{src} 连线：请点击目标节点左端口（点空白取消）"
          }
        }
      }

      // Top-right visual DAG builder controls (editable mode)
      if editable {
        div {
          class: "absolute top-2.5 right-2.5 z-20 flex flex-col items-end gap-2",
          onclick: move |ev| ev.stop_propagation(),
          div { class: "flex items-center gap-1.5 rounded-md border border-border bg-card/95 backdrop-blur p-1 shadow-xs",
            if let (Some((from_id, to_id)), Some(disc_cb)) = (cur_selected_edge.clone(), on_disconnect_edge) {
              {
                let f_clk = from_id.clone();
                let t_clk = to_id.clone();
                rsx! {
                  Button {
                    variant: ButtonVariant::Destructive,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2 text-xs".to_string(),
                    "data-testid": "canvas-disconnect-edge-btn",
                    onclick: move |_| {
                      disc_cb.call((f_clk.clone(), t_clk.clone()));
                      selected_edge.set(None);
                    },
                    "✂ 断开 {from_id} → {to_id}"
                  }
                }
              }
            }
            if let (Some(sel_id), Some(del_cb)) = (selected_node_id.clone(), on_delete_node) {
              if nodes.len() > 1 {
                {
                  let sel_clk = sel_id.clone();
                  rsx! {
                    Button {
                      variant: ButtonVariant::Ghost,
                      size: ButtonSize::Sm,
                      class: "h-7 px-2 text-xs text-destructive hover:text-destructive".to_string(),
                      "data-testid": "canvas-delete-node-btn",
                      onclick: move |_| del_cb.call(sel_clk.clone()),
                      "🗑 删除 #{sel_id}"
                    }
                  }
                }
              }
            }
            if on_add_node.is_some() {
              Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                class: "h-7 px-2.5 text-xs".to_string(),
                "data-testid": "canvas-open-palette-btn",
                onclick: move |_| palette_open.set(!palette_open()),
                if palette_open() { "✕ 收起组件库" } else { "+ 添加节点" }
              }
            }
          }

          if palette_open() {
            if let Some(add_cb) = on_add_node {
              div {
                class: "w-80 max-h-[420px] overflow-y-auto rounded-lg border border-border bg-card p-3 shadow-lg space-y-2.5",
                role: "region",
                "aria-label": "可视化添加工作流节点",
                div { class: "flex flex-wrap gap-1",
                  for cat in NODE_PALETTE_CATEGORIES {
                    {
                      let active = palette_category() == *cat;
                      let cat_s = (*cat).to_string();
                      rsx! {
                        Button {
                          key: "{cat}",
                          variant: if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                          size: ButtonSize::Sm,
                          class: "h-6 px-2 text-[11px]".to_string(),
                          onclick: move |_| palette_category.set(cat_s.clone()),
                          "{cat}"
                        }
                      }
                    }
                  }
                }
                if let Some(sel_id) = selected_node_id.clone() {
                  div { class: "flex items-center gap-2 text-xs text-muted-foreground",
                    Checkbox {
                      checked: chain_after_selected(),
                      on_checked_change: move |v| chain_after_selected.set(v),
                    }
                    span { "自动连接在当前选中节点 #{sel_id} 之后" }
                  }
                }
                div { class: "grid grid-cols-1 gap-1.5",
                  for item in palette_items {
                    {
                      let nt = item.node_type.to_string();
                      let sel_opt = selected_node_id.clone();
                      rsx! {
                        button {
                          key: "{item.node_type}",
                          r#type: "button",
                          "data-testid": "palette-add-{item.node_type}",
                          class: "w-full text-left rounded-md border border-border bg-background hover:bg-accent/60 px-2.5 py-2 transition-colors",
                          onclick: move |_| {
                            let after = if chain_after_selected() { sel_opt.clone() } else { None };
                            add_cb.call((nt.clone(), after));
                            palette_open.set(false);
                          },
                          div { class: "text-xs font-semibold text-foreground", "{item.label}" }
                          div { class: "text-[11px] text-muted-foreground leading-snug mt-0.5", "{item.description}" }
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // Transformed DAG World (SVG Edges + HTML Node Cards share exact coordinates)
      div {
        class: "absolute top-0 left-0 origin-top-left",
        style: "width: {world_w}px; height: {world_h}px; transform: translate({pan_x()}px, {pan_y()}px) scale({zoom()});",

        // SVG Edge Layer
        svg {
          class: "absolute inset-0 overflow-visible",
          width: "{world_w}",
          height: "{world_h}",
          defs {
            marker { id: "arrow-idle", marker_width: "8", marker_height: "8", ref_x: "7", ref_y: "4", orient: "auto",
              path { d: "M 0 0 L 8 4 L 0 8 z", fill: "#94a3b8" }
            }
            marker { id: "arrow-ok", marker_width: "8", marker_height: "8", ref_x: "7", ref_y: "4", orient: "auto",
              path { d: "M 0 0 L 8 4 L 0 8 z", fill: "#0d8a6a" }
            }
            marker { id: "arrow-live", marker_width: "8", marker_height: "8", ref_x: "7", ref_y: "4", orient: "auto",
              path { d: "M 0 0 L 8 4 L 0 8 z", fill: "#2563eb" }
            }
            marker { id: "arrow-cond", marker_width: "8", marker_height: "8", ref_x: "7", ref_y: "4", orient: "auto",
              path { d: "M 0 0 L 8 4 L 0 8 z", fill: "#d97706" }
            }
            marker { id: "arrow-selected", marker_width: "8", marker_height: "8", ref_x: "7", ref_y: "4", orient: "auto",
              path { d: "M 0 0 L 8 4 L 0 8 z", fill: "#dc2626" }
            }
          }

          for (idx, edge) in edges.iter().enumerate() {
            {
              let Some(&(sx, sy)) = positions.get(&edge.from) else {
                return rsx! {};
              };
              let Some(&(tx, ty)) = positions.get(&edge.to) else {
                return rsx! {};
              };
              let x1 = sx + NODE_W;
              let y1 = sy + NODE_H / 2.0;
              let x2 = tx;
              let y2 = ty + NODE_H / 2.0;
              let span_cols = ((tx - sx) / COL_WIDTH).round();
              let dx = ((x2 - x1).abs() * 0.42).max(48.0);
              // Route multi-column skip edges cleanly above/below intermediate nodes
              // so neither the curve nor its click hit-target hides behind a card.
              let arch_offset = if span_cols > 1.2 {
                if y1 <= y2 { -112.0 } else { 112.0 }
              } else {
                0.0
              };
              let c1x = x1 + dx;
              let c1y = y1 + arch_offset;
              let c2x = x2 - dx;
              let c2y = y2 + arch_offset;
              let d_attr = format!(
                "M {x1:.1} {y1:.1} C {c1x:.1} {c1y:.1}, {c2x:.1} {c2y:.1}, {x2:.1} {y2:.1}"
              );
              // Exact cubic Bezier midpoint at t = 0.5: 0.125*P0 + 0.375*P1 + 0.375*P2 + 0.125*P3
              let mid_x = 0.125 * x1 + 0.375 * c1x + 0.375 * c2x + 0.125 * x2;
              let mid_y = 0.125 * y1 + 0.375 * c1y + 0.375 * c2y + 0.125 * y2;

              let src_status = telemetry_by_node
                .get(&edge.from)
                .map(|t| t.status)
                .unwrap_or(StudioNodeRunStatus::Idle);
              let tgt_status = telemetry_by_node
                .get(&edge.to)
                .map(|t| t.status)
                .unwrap_or(StudioNodeRunStatus::Idle);
              let is_cond = edge.kind == InspectedEdgeKind::Condition;
              let is_edge_sel = cur_selected_edge
                .as_ref()
                .is_some_and(|(f, t)| f == &edge.from && t == &edge.to);
              let is_live = matches!(
                tgt_status,
                StudioNodeRunStatus::Running | StudioNodeRunStatus::PausedBefore
              ) || src_status == StudioNodeRunStatus::PausedAfter;

              let (stroke, marker_ref) = if is_edge_sel {
                ("#dc2626", "url(#arrow-selected)")
              } else if is_cond {
                ("#d97706", "url(#arrow-cond)")
              } else if src_status == StudioNodeRunStatus::Succeeded {
                ("#0d8a6a", "url(#arrow-ok)")
              } else if is_live {
                ("#2563eb", "url(#arrow-live)")
              } else {
                ("#94a3b8", "url(#arrow-idle)")
              };
              let stroke_w = if is_edge_sel {
                "2.8"
              } else if is_live || src_status == StudioNodeRunStatus::Succeeded {
                "2.2"
              } else {
                "1.7"
              };
              let dash = if is_cond { "6 4" } else { "" };
              let e_from = edge.from.clone();
              let e_to = edge.to.clone();
              let edge_label = edge.label.clone().unwrap_or_default();
              let short_label = if edge_label.chars().count() > 26 {
                format!("{}…", edge_label.chars().take(25).collect::<String>())
              } else {
                edge_label
              };

              rsx! {
                g {
                  key: "edge-{edge.from}-{edge.to}-{idx}",
                  class: if editable { "cursor-pointer" } else { "" },
                  "data-testid": "dag-edge-{edge.from}-{edge.to}",
                  onclick: move |ev| {
                    ev.stop_propagation();
                    if editable {
                      selected_edge.set(Some((e_from.clone(), e_to.clone())));
                    }
                  },
                  path {
                    d: "{d_attr}",
                    fill: "none",
                    stroke: "transparent",
                    stroke_width: "14",
                  }
                  path {
                    d: "{d_attr}",
                    fill: "none",
                    stroke: "{stroke}",
                    stroke_width: "{stroke_w}",
                    stroke_dasharray: "{dash}",
                    marker_end: "{marker_ref}",
                  }
                  if !short_label.is_empty() {
                    {
                      let pill_w = (short_label.chars().count() as f64 * 6.2 + 14.0).clamp(48.0, 114.0);
                      rsx! {
                        rect {
                          "data-testid": "dag-edge-pill-{edge.from}-{edge.to}",
                          x: "{mid_x - pill_w / 2.0:.1}",
                          y: "{mid_y - 16.0:.1}",
                          width: "{pill_w:.1}",
                          height: "18",
                          rx: "4",
                          fill: if is_cond { "#fef3c7" } else { "#f8fafc" },
                          stroke: "{stroke}",
                          stroke_width: "0.8",
                          opacity: "0.94",
                        }
                        text {
                          class: "pointer-events-none",
                          x: "{mid_x:.1}",
                          y: "{mid_y - 4.0:.1}",
                          text_anchor: "middle",
                          font_size: "10",
                          font_weight: "600",
                          fill: if is_cond { "#92400e" } else { "#334155" },
                          "{short_label}"
                        }
                      }
                    }
                  }
                }
              }
            }
          }
        }

        // HTML Node Cards Layer
        for (fallback_idx, node) in nodes.iter().enumerate() {
          {
            let &(nx, ny) = positions.get(&node.id).unwrap_or(&(WORLD_PAD, WORLD_PAD));
            let telemetry = telemetry_by_node.get(&node.id).cloned().unwrap_or_default();
            let has_bp = bp_set.contains(node.id.as_str());
            let is_sel = selected_node_id.as_deref() == Some(node.id.as_str());
            let step_idx = order_map.get(node.id.as_str()).copied().unwrap_or(fallback_idx);
            let style = node_type_style(&node.node_type);
            let ring = status_ring_class(telemetry.status, is_sel);
            let nid_click = node.id.clone();
            let nid_bp = node.id.clone();
            let nid_in = node.id.clone();
            let nid_out = node.id.clone();
            let is_connect_source = cur_connecting.as_deref() == Some(node.id.as_str());
            let is_connect_target = cur_connecting
              .as_deref()
              .is_some_and(|src| src != node.id.as_str());

            rsx! {
              div {
                key: "node-{node.id}",
                class: "absolute rounded-lg border bg-card p-2.5 transition-shadow flex flex-col justify-between cursor-pointer {ring}",
                style: "left: {nx}px; top: {ny}px; width: {NODE_W}px; min-height: {NODE_H}px;",
                "data-testid": "dag-node-{node.id}",
                "data-status": "{telemetry.status.label()}",
                onmousedown: move |ev| ev.stop_propagation(),
                onclick: move |ev| {
                  ev.stop_propagation();
                  let pending_src = connecting_from.peek().clone();
                  if editable
                    && let Some(src) = pending_src
                    && src != nid_click
                    && let Some(conn_cb) = on_connect_nodes
                  {
                    conn_cb.call((src, nid_click.clone()));
                    connecting_from.set(None);
                    return;
                  }
                  selected_edge.set(None);
                  on_select_node.call(nid_click.clone());
                },

                // Left input port
                button {
                  r#type: "button",
                  "data-testid": "port-in-{node.id}",
                  title: if is_connect_target { "点击完成连线到此节点" } else { "上游输入端口" },
                  class: if is_connect_target {
                    "absolute -left-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full border-2 border-primary bg-primary/20 hover:scale-125 transition-transform"
                  } else {
                    "absolute -left-1.5 top-1/2 -translate-y-1/2 w-3 h-3 rounded-full border-2 border-border bg-card"
                  },
                  onclick: move |ev| {
                    ev.stop_propagation();
                    let pending_src = connecting_from.peek().clone();
                    if editable
                      && let Some(src) = pending_src
                      && src != nid_in
                      && let Some(conn_cb) = on_connect_nodes
                    {
                      conn_cb.call((src, nid_in.clone()));
                      connecting_from.set(None);
                    }
                  },
                }

                // Header row: #step, type pill, breakpoint toggle
                div { class: "flex items-center justify-between gap-1.5",
                  div { class: "flex items-center gap-1.5 min-w-0",
                    span { class: "text-[10px] font-mono text-muted-foreground", "#{step_idx + 1}" }
                    span {
                      class: "rounded px-1.5 py-0.5 text-[10px] font-semibold truncate",
                      style: "color: {style.accent}; background-color: {style.tint};",
                      "{node.node_type}{style.label_suffix}"
                    }
                  }
                  button {
                    r#type: "button",
                    "data-testid": "bp-toggle-{node.id}",
                    "aria-pressed": has_bp.to_string(),
                    title: if has_bp { "已设执行前断点（点击移除）" } else { "点击设置执行前断点" },
                    class: if has_bp {
                      "w-4 h-4 rounded-full border border-rose-600 bg-rose-500 flex items-center justify-center shrink-0"
                    } else {
                      "w-4 h-4 rounded-full border border-border bg-muted/60 hover:border-rose-400 flex items-center justify-center shrink-0"
                    },
                    onclick: move |ev| {
                      ev.stop_propagation();
                      on_toggle_breakpoint.call(nid_bp.clone());
                    },
                    span {
                      class: if has_bp { "w-1.5 h-1.5 rounded-full bg-white" } else { "w-1.5 h-1.5 rounded-full bg-muted-foreground/40" }
                    }
                  }
                }

                // Node ID & chips
                div { class: "my-1.5 space-y-1",
                  div { class: "text-xs font-bold text-foreground truncate", title: "{node.id}", "{node.id}" }
                  if !node.sub_nodes.is_empty() {
                    div { class: "text-[10px] rounded bg-teal-500/10 text-teal-700 dark:text-teal-300 px-1.5 py-0.5 truncate",
                      "内嵌 {node.sub_nodes.len()} 个子节点"
                    }
                  }
                  if let Some(cond) = &node.run_if {
                    div {
                      class: "text-[10px] rounded bg-amber-500/10 text-amber-700 dark:text-amber-300 px-1.5 py-0.5 truncate",
                      title: "{cond}",
                      "run_if 条件守卫"
                    }
                  }
                }

                // Footer: status + cost / duration / IO pills
                div { class: "flex items-center justify-between gap-1 pt-1 border-t border-border/60 text-[10px]",
                  span { class: status_badge_class(telemetry.status), "{telemetry.status.label()}" }
                  div { class: "flex items-center gap-1 text-muted-foreground",
                    if let Some(cost) = telemetry.cost_usd {
                      if cost > 0.0 {
                        span { class: "rounded bg-emerald-500/15 text-emerald-700 dark:text-emerald-300 px-1 font-mono font-semibold",
                          "{format_usd_badge(cost)}"
                        }
                      }
                    }
                    if let Some(ms) = telemetry.duration_ms {
                      span { class: "font-mono", "{ms}ms" }
                    }
                    if telemetry.has_input || telemetry.has_output {
                      span { class: "rounded bg-primary/15 text-primary px-1 font-semibold", "I/O" }
                    }
                  }
                }

                // Right output port (click to start connecting in editable mode)
                button {
                  r#type: "button",
                  "data-testid": "port-out-{node.id}",
                  title: if editable { "点击从本节点发起连线到下游节点" } else { "下游输出端口" },
                  class: if is_connect_source {
                    "absolute -right-2 top-1/2 -translate-y-1/2 w-4 h-4 rounded-full border-2 border-primary bg-primary scale-125 transition-transform"
                  } else if editable {
                    "absolute -right-1.5 top-1/2 -translate-y-1/2 w-3.5 h-3.5 rounded-full border-2 border-primary/70 bg-card hover:bg-primary hover:scale-125 transition-transform"
                  } else {
                    "absolute -right-1.5 top-1/2 -translate-y-1/2 w-3 h-3 rounded-full border-2 border-border bg-card"
                  },
                  onclick: move |ev| {
                    ev.stop_propagation();
                    if editable && on_connect_nodes.is_some() {
                      if connecting_from.peek().as_deref() == Some(nid_out.as_str()) {
                        connecting_from.set(None);
                      } else {
                        connecting_from.set(Some(nid_out.clone()));
                      }
                    }
                  },
                }
              }
            }
          }
        }
      }

      // Bottom-left Zoom & Fit Controls
      div {
        class: "absolute bottom-2.5 left-2.5 z-20 flex items-center gap-1 rounded-md border border-border bg-card/95 backdrop-blur p-1 shadow-xs",
        onclick: move |ev| ev.stop_propagation(),
        Button {
          variant: ButtonVariant::Ghost,
          size: ButtonSize::Sm,
          class: "h-6 w-6 p-0 text-xs".to_string(),
          "data-testid": "canvas-zoom-out",
          onclick: move |_| zoom.set((zoom() - 0.12).clamp(0.35, 1.8)),
          "−"
        }
        span { class: "text-[11px] font-mono w-10 text-center text-muted-foreground", "{zoom_pct}%" }
        Button {
          variant: ButtonVariant::Ghost,
          size: ButtonSize::Sm,
          class: "h-6 w-6 p-0 text-xs".to_string(),
          "data-testid": "canvas-zoom-in",
          onclick: move |_| zoom.set((zoom() + 0.12).clamp(0.35, 1.8)),
          "+"
        }
        Button {
          variant: ButtonVariant::Ghost,
          size: ButtonSize::Sm,
          class: "h-6 px-2 text-[11px]".to_string(),
          "data-testid": "canvas-fit-view",
          onclick: move |_| {
            zoom.set(init_zoom);
            pan_x.set(6.0);
            pan_y.set(24.0);
          },
          "复位"
        }
      }

      // Bottom-right Compact SVG MiniMap
      div {
        class: "absolute bottom-2.5 right-2.5 z-20 rounded-md border border-border bg-card/90 backdrop-blur p-1.5 shadow-xs",
        onclick: move |ev| ev.stop_propagation(),
        svg {
          width: "136",
          height: "74",
          view_box: "0 0 {world_w} {world_h}",
          for node in &nodes {
            {
              let &(nx, ny) = positions.get(&node.id).unwrap_or(&(0.0, 0.0));
              let st = telemetry_by_node
                .get(&node.id)
                .map(|t| t.status)
                .unwrap_or(StudioNodeRunStatus::Idle);
              let fill = minimap_node_fill(st);
              let nid_jump = node.id.clone();
              rsx! {
                rect {
                  key: "mm-{node.id}",
                  x: "{nx}",
                  y: "{ny}",
                  width: "{NODE_W}",
                  height: "{NODE_H}",
                  rx: "16",
                  fill: "{fill}",
                  opacity: if selected_node_id.as_deref() == Some(node.id.as_str()) { "1" } else { "0.65" },
                  class: "cursor-pointer",
                  onclick: move |_| on_select_node.call(nid_jump.clone()),
                }
              }
            }
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::studio_yaml::{INSTANT_STEP_DEBUG_YAML, inspect_yaml_locally};

  #[test]
  fn dag_layout_positions_all_levels_without_overlap() {
    let insp = inspect_yaml_locally(INSTANT_STEP_DEBUG_YAML);
    let (pos, w, h) = compute_dag_layout(&insp.nodes, &insp.levels);
    assert_eq!(pos.len(), 4);
    assert!(w > NODE_W * 2.0);
    assert!(h > NODE_H);
    let x0 = pos["normalize_brief"].0;
    let x1 = pos["core_outline"].0;
    let x2 = pos["assemble_handout"].0;
    assert!(x0 < x1 && x1 < x2);
  }
}
