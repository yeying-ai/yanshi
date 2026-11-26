//! Interactive Notebook deliverable viewers (Flashcards, Quiz, Sortable Data
//! Table, Audio Transcript, Video Storyboard), deduplicated citation chips,
//! and authenticated `AuthorizedMedia` preview cards with full-screen `Sheet`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap};

use dioxus::prelude::*;
use dioxus_shadcn::{
  Badge, BadgeVariant, Button, ButtonSize, ButtonVariant, Input, Sheet, SheetClose, SheetContent,
  SheetDescription, SheetHeader, SheetOverlay, SheetSide, SheetSize, SheetTitle, Table, TableBody,
  TableCell, TableHead, TableHeader, TableRow,
};

use crate::api::{
  download_authenticated_resource, fetch_blob_url, fetch_text_resource, is_local_desktop_engine,
  open_desktop_artifact, resolve_embedded_token, revoke_blob_url,
};
use crate::notebook_api::{
  MediaPreviewItem, MediaPreviewKind, NotebookDeliverable, append_inline_query,
  dedupe_deliverable_citations, format_bytes,
};

// ── RFC-4180 CSV / TSV Parser & Natural Sort ─────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TableRowData {
  pub id: String,
  pub cells: Vec<String>,
  pub chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedDelimitedTable {
  pub columns: Vec<String>,
  pub rows: Vec<TableRowData>,
}

pub fn parse_delimited_table(raw: &str, delimiter: char) -> ParsedDelimitedTable {
  let chars: Vec<char> = raw.chars().collect();
  let mut rows: Vec<Vec<String>> = Vec::new();
  let mut current_row: Vec<String> = Vec::new();
  let mut cell = String::new();
  let mut in_quotes = false;
  let mut i = 0usize;

  while i < chars.len() {
    let ch = chars[i];
    if in_quotes {
      if ch == '"' {
        if i + 1 < chars.len() && chars[i + 1] == '"' {
          cell.push('"');
          i += 2;
          continue;
        }
        in_quotes = false;
      } else {
        cell.push(ch);
      }
      i += 1;
      continue;
    }

    if ch == '"' {
      in_quotes = true;
    } else if ch == delimiter {
      current_row.push(cell.trim().to_string());
      cell.clear();
    } else if ch == '\r' || ch == '\n' {
      if ch == '\r' && i + 1 < chars.len() && chars[i + 1] == '\n' {
        i += 1;
      }
      current_row.push(cell.trim().to_string());
      cell.clear();
      if current_row.iter().any(|c| !c.is_empty()) {
        rows.push(std::mem::take(&mut current_row));
      } else {
        current_row.clear();
      }
    } else {
      cell.push(ch);
    }
    i += 1;
  }

  if !cell.is_empty() || !current_row.is_empty() {
    current_row.push(cell.trim().to_string());
    if current_row.iter().any(|c| !c.is_empty()) {
      rows.push(current_row);
    }
  }

  if rows.is_empty() {
    return ParsedDelimitedTable {
      columns: Vec::new(),
      rows: Vec::new(),
    };
  }
  let columns = rows[0].clone();
  let body_rows = rows[1..]
    .iter()
    .enumerate()
    .map(|(idx, cells)| TableRowData {
      id: format!("row-{idx}"),
      cells: (0..columns.len())
        .map(|c_idx| cells.get(c_idx).cloned().unwrap_or_default())
        .collect(),
      chunk_id: None,
    })
    .collect();
  ParsedDelimitedTable {
    columns,
    rows: body_rows,
  }
}

pub fn natural_cmp(a: &str, b: &str) -> Ordering {
  let a_num = a.trim().replace(',', "").parse::<f64>();
  let b_num = b.trim().replace(',', "").parse::<f64>();
  if let (Ok(na), Ok(nb)) = (a_num, b_num)
    && let Some(ord) = na.partial_cmp(&nb)
  {
    return ord;
  }
  a.to_lowercase().cmp(&b.to_lowercase())
}

// ── Deliverable Extractors ───────────────────────────────────────────────────

#[derive(Clone, Debug, PartialEq, Eq)]
struct FlashcardItem {
  front: String,
  back: String,
  chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct QuizQuestionItem {
  question: String,
  options: Vec<String>,
  answer: usize,
  explanation: Option<String>,
  chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct AudioScriptTurn {
  speaker: String,
  speaker_label: Option<String>,
  text: String,
  chunk_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct VideoStoryboardScene {
  label: Option<String>,
  narration: String,
  visual: Option<String>,
  motion: bool,
  chunk_id: Option<String>,
}

fn non_empty_str(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
  obj
    .get(key)
    .and_then(|v| v.as_str())
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(ToString::to_string)
}

fn extract_flashcards(deliverable: &NotebookDeliverable) -> Vec<FlashcardItem> {
  let Some(items) = deliverable
    .content
    .as_ref()
    .and_then(|c| c.get("items"))
    .and_then(|v| v.as_array())
  else {
    return Vec::new();
  };
  let mut out = Vec::new();
  for raw in items {
    let Some(obj) = raw.as_object() else {
      continue;
    };
    let (Some(front), Some(back)) = (non_empty_str(obj, "front"), non_empty_str(obj, "back"))
    else {
      continue;
    };
    out.push(FlashcardItem {
      front,
      back,
      chunk_id: non_empty_str(obj, "chunk_id"),
    });
  }
  out
}

fn extract_quiz_questions(deliverable: &NotebookDeliverable) -> Vec<QuizQuestionItem> {
  let Some(items) = deliverable
    .content
    .as_ref()
    .and_then(|c| c.get("items"))
    .and_then(|v| v.as_array())
  else {
    return Vec::new();
  };
  let mut out = Vec::new();
  for raw in items {
    let Some(obj) = raw.as_object() else {
      continue;
    };
    let Some(question) = non_empty_str(obj, "question") else {
      continue;
    };
    let options: Vec<String> = obj
      .get("options")
      .and_then(|v| v.as_array())
      .map(|arr| {
        arr
          .iter()
          .filter_map(|v| v.as_str().map(|s| s.trim().to_string()))
          .collect()
      })
      .unwrap_or_default();
    let Some(ans_i64) = obj.get("answer").and_then(|v| v.as_i64()) else {
      continue;
    };
    let Ok(answer) = usize::try_from(ans_i64) else {
      continue;
    };
    if options.len() < 2 || answer >= options.len() {
      continue;
    }
    out.push(QuizQuestionItem {
      question,
      options,
      answer,
      explanation: non_empty_str(obj, "explanation"),
      chunk_id: non_empty_str(obj, "chunk_id"),
    });
  }
  out
}

fn extract_deliverable_table(deliverable: &NotebookDeliverable) -> Option<ParsedDelimitedTable> {
  let obj = deliverable.content.as_ref()?.as_object()?;
  let columns: Vec<String> = obj
    .get("columns")?
    .as_array()?
    .iter()
    .filter_map(|v| v.as_str().map(ToString::to_string))
    .collect();
  let raw_rows = obj.get("rows")?.as_array()?;
  if columns.is_empty() || raw_rows.is_empty() {
    return None;
  }
  let mut rows = Vec::new();
  for (idx, entry) in raw_rows.iter().enumerate() {
    let Some(row_obj) = entry.as_object() else {
      continue;
    };
    let Some(raw_cells) = row_obj.get("cells").and_then(|v| v.as_array()) else {
      continue;
    };
    if raw_cells.is_empty() {
      continue;
    }
    let cells = (0..columns.len())
      .map(|c_idx| match raw_cells.get(c_idx) {
        None | Some(serde_json::Value::Null) => String::new(),
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
      })
      .collect();
    rows.push(TableRowData {
      id: format!("dt-{idx}"),
      cells,
      chunk_id: non_empty_str(row_obj, "chunk_id"),
    });
  }
  if rows.is_empty() {
    None
  } else {
    Some(ParsedDelimitedTable { columns, rows })
  }
}

fn extract_audio_script(deliverable: &NotebookDeliverable) -> (Vec<AudioScriptTurn>, Option<f64>) {
  let Some(obj) = deliverable.content.as_ref().and_then(|c| c.as_object()) else {
    return (Vec::new(), None);
  };
  let raw_script = obj
    .get("script")
    .or_else(|| obj.get("turns"))
    .and_then(|v| v.as_array());
  let mut turns = Vec::new();
  if let Some(entries) = raw_script {
    for entry in entries {
      let Some(item) = entry.as_object() else {
        continue;
      };
      let Some(text) = non_empty_str(item, "text") else {
        continue;
      };
      turns.push(AudioScriptTurn {
        speaker: non_empty_str(item, "speaker").unwrap_or_else(|| "host".to_string()),
        speaker_label: non_empty_str(item, "speaker_label"),
        text,
        chunk_id: non_empty_str(item, "chunk_id"),
      });
    }
  }
  let lufs = obj
    .get("loudness_lufs")
    .and_then(|v| v.as_f64())
    .filter(|n| n.is_finite());
  (turns, lufs)
}

fn extract_video_scenes(
  deliverable: &NotebookDeliverable,
) -> (Vec<VideoStoryboardScene>, Option<f64>) {
  let Some(obj) = deliverable.content.as_ref().and_then(|c| c.as_object()) else {
    return (Vec::new(), None);
  };
  let mut scenes = Vec::new();

  if let Some(raw_scenes) = obj.get("scenes").and_then(|v| v.as_array()) {
    for entry in raw_scenes {
      let Some(item) = entry.as_object() else {
        continue;
      };
      let narration = non_empty_str(item, "narration").unwrap_or_default();
      let visual = non_empty_str(item, "visual");
      if narration.is_empty() && visual.is_none() {
        continue;
      }
      scenes.push(VideoStoryboardScene {
        label: None,
        narration,
        visual,
        motion: item
          .get("motion")
          .and_then(|v| v.as_bool())
          .unwrap_or(false),
        chunk_id: non_empty_str(item, "chunk_id"),
      });
    }
  }

  // Fallback 1: `tech_slides_video` stores `content.slides[]`.
  if scenes.is_empty()
    && let Some(raw_slides) = obj.get("slides").and_then(|v| v.as_array())
  {
    for (idx, entry) in raw_slides.iter().enumerate() {
      let Some(slide) = entry.as_object() else {
        continue;
      };
      let title = non_empty_str(slide, "title").unwrap_or_default();
      let step_lines: Vec<String> = slide
        .get("steps")
        .and_then(|v| v.as_array())
        .map(|arr| {
          arr
            .iter()
            .filter_map(|s| {
              let so = s.as_object()?;
              non_empty_str(so, "spoken").or_else(|| non_empty_str(so, "caption"))
            })
            .collect()
        })
        .unwrap_or_default();
      let narration = if !step_lines.is_empty() {
        step_lines.join(" ")
      } else {
        non_empty_str(slide, "notes")
          .or_else(|| non_empty_str(slide, "narration"))
          .unwrap_or_default()
      };
      let visual_tail =
        non_empty_str(slide, "illustration_prompt").or_else(|| non_empty_str(slide, "takeaway"));
      let visual_parts: Vec<String> = [(!title.is_empty()).then_some(title), visual_tail]
        .into_iter()
        .flatten()
        .collect();
      let visual = (!visual_parts.is_empty()).then(|| visual_parts.join(" · "));
      if narration.is_empty() && visual.is_none() {
        continue;
      }
      scenes.push(VideoStoryboardScene {
        label: Some(format!("幻灯片 {}", idx + 1)),
        narration,
        visual,
        motion: false,
        chunk_id: non_empty_str(slide, "chunk_id"),
      });
    }
  }

  // Fallback 2: `episodic_drama_studio` stores `content.episodes[].scenes[]`.
  if scenes.is_empty()
    && let Some(episodes) = obj.get("episodes").and_then(|v| v.as_array())
  {
    for (ep_idx, ep_entry) in episodes.iter().enumerate() {
      let Some(ep) = ep_entry.as_object() else {
        continue;
      };
      let ep_num = ep
        .get("episode_number")
        .and_then(|v| v.as_u64())
        .unwrap_or((ep_idx + 1) as u64);
      let Some(ep_scenes) = ep.get("scenes").and_then(|v| v.as_array()) else {
        continue;
      };
      for (sc_idx, sc_entry) in ep_scenes.iter().enumerate() {
        let Some(sc) = sc_entry.as_object() else {
          continue;
        };
        let sc_num = sc
          .get("scene_number")
          .and_then(|v| v.as_u64())
          .unwrap_or((sc_idx + 1) as u64);
        let speaker = non_empty_str(sc, "speaker_name");
        let dialogue = non_empty_str(sc, "dialogue");
        let narration = match (speaker, dialogue) {
          (Some(sp), Some(dl)) => format!("{sp}：{dl}"),
          (_, Some(dl)) => dl,
          _ => non_empty_str(sc, "narration").unwrap_or_default(),
        };
        let loc = non_empty_str(sc, "location").map(|l| format!("场景：{l}"));
        let shot = non_empty_str(sc, "shot_prompt_en");
        let visual_parts: Vec<String> = [loc, shot].into_iter().flatten().collect();
        let visual = (!visual_parts.is_empty()).then(|| visual_parts.join(" · "));
        if narration.is_empty() && visual.is_none() {
          continue;
        }
        let motion = sc
          .get("climax_motion")
          .and_then(|v| v.as_bool())
          .unwrap_or(false)
          || sc.get("motion").and_then(|v| v.as_bool()).unwrap_or(false);
        scenes.push(VideoStoryboardScene {
          label: Some(format!("第 {ep_num} 集 · 场 {sc_num}")),
          narration,
          visual,
          motion,
          chunk_id: non_empty_str(sc, "chunk_id"),
        });
      }
    }
  }

  let seconds = obj
    .get("seconds")
    .and_then(|v| v.as_f64())
    .filter(|n| n.is_finite());
  (scenes, seconds)
}

fn short_chunk_label(chunk_id: &str) -> &str {
  chunk_id.rsplit(':').next().unwrap_or(chunk_id)
}

pub fn has_exclusive_deliverable_view(deliverable: &NotebookDeliverable) -> bool {
  deliverable.kind == "data_table" && extract_deliverable_table(deliverable).is_some()
}

// ── Citation Bar Component ───────────────────────────────────────────────────

#[component]
pub fn NotebookDeliverableCitationBar(
  deliverable: NotebookDeliverable,
  source_title_by_id: HashMap<String, String>,
  on_inspect_chunk: EventHandler<String>,
) -> Element {
  let citations = dedupe_deliverable_citations(&deliverable.citations);
  let title = deliverable.title.trim().to_string();
  if title.is_empty() && citations.is_empty() {
    return rsx! {};
  }

  rsx! {
    div { class: "space-y-2", "data-testid": "deliverable-citation-bar",
      if !title.is_empty() {
        h4 { class: "text-sm font-semibold text-foreground", "{title}" }
      }
      if !citations.is_empty() {
        ul { class: "flex flex-wrap gap-1.5", role: "list", "aria-label": "交付物原文引用",
          for (index, cit) in citations.into_iter().enumerate() {
            {
              let source_label = source_title_by_id
                .get(&cit.source_id)
                .cloned()
                .unwrap_or_else(|| "资料引用".to_string());
              let snippet: String = if cit.quote.chars().count() > 56 {
                format!("{}…", cit.quote.chars().take(56).collect::<String>())
              } else {
                cit.quote.clone()
              };
              let chunk_id = cit.chunk_id.clone();
              let has_chunk = !chunk_id.is_empty();
              let badge_text = format!("[{}] {source_label}", index + 1);
              let test_id = format!("citation-chip-{chunk_id}");
              rsx! {
                li { key: "{chunk_id}-{index}",
                  if has_chunk {
                    button {
                      r#type: "button",
                      class: "inline-flex items-center gap-1.5 rounded-md border border-border bg-muted/50 hover:bg-accent px-2 py-1 text-left text-xs transition-colors cursor-pointer",
                      "data-testid": "{test_id}",
                      onclick: move |_| on_inspect_chunk.call(chunk_id.clone()),
                      span { class: "font-medium text-primary shrink-0", "{badge_text}" }
                      span { class: "text-muted-foreground line-clamp-1", "“{snippet}”" }
                    }
                  } else {
                    span { class: "inline-flex items-center gap-1.5 rounded-md border border-border/60 bg-muted/30 px-2 py-1 text-xs",
                      span { class: "font-medium text-foreground shrink-0", "{badge_text}" }
                      span { class: "text-muted-foreground", "“{snippet}”" }
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
}

// ── Interactive Sortable Table ───────────────────────────────────────────────

#[component]
pub fn InteractiveSortableTable(
  caption: String,
  columns: Vec<String>,
  rows: Vec<TableRowData>,
  #[props(default)] on_inspect_chunk: Option<EventHandler<String>>,
  #[props(default = "interactive-data-table".to_string())] test_id: String,
) -> Element {
  let mut filter_query = use_signal(String::new);
  let mut sort_col = use_signal(|| None::<usize>);
  let mut sort_asc = use_signal(|| true);

  if columns.is_empty() {
    return rsx! {};
  }

  let has_citations = on_inspect_chunk.is_some() && rows.iter().any(|r| r.chunk_id.is_some());
  let q = filter_query().trim().to_lowercase();
  let mut visible_rows: Vec<TableRowData> = rows
    .iter()
    .filter(|r| {
      if q.is_empty() {
        return true;
      }
      r.cells.iter().any(|c| c.to_lowercase().contains(&q))
    })
    .cloned()
    .collect();

  if let Some(col_idx) = sort_col() {
    let asc = sort_asc();
    visible_rows.sort_by(|a, b| {
      let va = a.cells.get(col_idx).map(String::as_str).unwrap_or("");
      let vb = b.cells.get(col_idx).map(String::as_str).unwrap_or("");
      let ord = natural_cmp(va, vb);
      if asc { ord } else { ord.reverse() }
    });
  }

  let total_rows = rows.len();
  let shown_rows = visible_rows.len();

  rsx! {
    div { class: "space-y-2 rounded-md border border-border p-3 bg-card/50", "data-testid": "{test_id}",
      div { class: "flex flex-wrap items-center justify-between gap-2",
        span { class: "text-xs font-medium text-muted-foreground",
          "{caption} · 显示 {shown_rows} / {total_rows} 行"
        }
        div { class: "w-48",
          Input {
            value: filter_query(),
            placeholder: "筛选表格关键字…".to_string(),
            class: "h-7 text-xs".to_string(),
            "data-testid": "table-filter-input",
            on_value_change: move |v: String| filter_query.set(v),
          }
        }
      }

      div { class: "overflow-x-auto max-h-80 rounded border border-border/60",
        Table {
          TableHeader {
            TableRow {
              for (idx, col_name) in columns.iter().enumerate() {
                {
                  let active = sort_col() == Some(idx);
                  let indicator = if active {
                    if sort_asc() { " ↑" } else { " ↓" }
                  } else {
                    ""
                  };
                  rsx! {
                    TableHead { key: "col-{idx}",
                      button {
                        r#type: "button",
                        class: "inline-flex items-center gap-1 font-semibold hover:text-primary cursor-pointer text-xs",
                        "data-testid": "table-sort-col-{idx}",
                        onclick: move |_| {
                          if sort_col() == Some(idx) {
                            sort_asc.set(!sort_asc());
                          } else {
                            sort_col.set(Some(idx));
                            sort_asc.set(true);
                          }
                        },
                        "{col_name}{indicator}"
                      }
                    }
                  }
                }
              }
              if has_citations {
                TableHead { "溯源" }
              }
            }
          }
          TableBody {
            for row in visible_rows {
              TableRow { key: "{row.id}",
                for (c_idx, val) in row.cells.iter().enumerate() {
                  TableCell { key: "{row.id}-{c_idx}", class: "text-xs".to_string(), "{val}" }
                }
                if has_citations {
                  TableCell { class: "text-xs".to_string(),
                    if let Some(cid) = row.chunk_id.clone() {
                      Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: "h-6 px-2 text-xs".to_string(),
                        onclick: move |_| {
                          if let Some(handler) = on_inspect_chunk {
                            handler.call(cid.clone());
                          }
                        },
                        "查看"
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
  }
}

// ── NotebookDeliverableViewer (5 Interactive Modes) ──────────────────────────

#[component]
pub fn NotebookDeliverableViewer(
  deliverable: NotebookDeliverable,
  on_inspect_chunk: EventHandler<String>,
) -> Element {
  let mut card_order = use_signal(Vec::<usize>::new);
  let mut card_cursor = use_signal(|| 0usize);
  let mut card_flipped = use_signal(|| false);
  let mut shuffle_seed = use_signal(|| 1u64);
  let mut selected_answers = use_signal(BTreeMap::<usize, usize>::new);

  match deliverable.kind.as_str() {
    "flashcards" => {
      let cards = extract_flashcards(&deliverable);
      if cards.is_empty() {
        return rsx! {};
      }
      let len = cards.len();
      let indices: Vec<usize> = if card_order().len() == len {
        card_order()
      } else {
        (0..len).collect()
      };
      let safe_cursor = card_cursor().min(len - 1);
      let active_card = &cards[indices[safe_cursor]];
      let active_chunk = active_card.chunk_id.clone();

      rsx! {
        div { class: "space-y-2.5 rounded-md border border-border p-3 bg-card/50", "data-testid": "interactive-flashcards",
          div { class: "flex flex-wrap items-center justify-between gap-2",
            span { class: "text-xs font-medium text-muted-foreground", "data-testid": "flashcard-progress",
              "互动闪卡 · 第 {safe_cursor + 1} / {len} 张"
            }
            div { class: "flex items-center gap-1.5",
              Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                disabled: safe_cursor == 0,
                "data-testid": "flashcard-prev-btn",
                onclick: move |_| {
                  card_cursor.set(safe_cursor.saturating_sub(1));
                  card_flipped.set(false);
                },
                "← 上一张"
              }
              Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                disabled: safe_cursor + 1 >= len,
                "data-testid": "flashcard-next-btn",
                onclick: move |_| {
                  card_cursor.set((safe_cursor + 1).min(len - 1));
                  card_flipped.set(false);
                },
                "下一张 →"
              }
              if len > 1 {
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  "data-testid": "flashcard-shuffle-btn",
                  onclick: move |_| {
                    let mut next: Vec<usize> = (0..len).collect();
                    let mut state = shuffle_seed().wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                    shuffle_seed.set(state);
                    for i in (1..len).rev() {
                      state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                      let j = (state as usize) % (i + 1);
                      next.swap(i, j);
                    }
                    card_order.set(next);
                    card_cursor.set(0);
                    card_flipped.set(false);
                  },
                  "随机洗牌"
                }
              }
            }
          }

          button {
            r#type: "button",
            class: if card_flipped() {
              "w-full min-h-32 rounded-lg border-2 border-primary/60 bg-primary/5 p-4 text-left transition-colors cursor-pointer space-y-2"
            } else {
              "w-full min-h-32 rounded-lg border border-border bg-background hover:bg-accent/40 p-4 text-left transition-colors cursor-pointer space-y-2"
            },
            "data-testid": "flashcard-flip-btn",
            "aria-pressed": if card_flipped() { "true" } else { "false" },
            onclick: move |_| card_flipped.set(!card_flipped()),
            div { class: "text-[11px] font-medium text-muted-foreground",
              if card_flipped() { "背面 · 答案解析（点击翻回）" } else { "正面 · 核心问题（点击翻转）" }
            }
            div { class: "text-sm font-medium leading-relaxed text-foreground whitespace-pre-wrap",
              if card_flipped() { "{active_card.back}" } else { "{active_card.front}" }
            }
          }

          if let Some(cid) = active_chunk {
            {
              let short_cid = short_chunk_label(&cid).to_string();
              rsx! {
                div {
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| on_inspect_chunk.call(cid.clone()),
                    "查看原文分块 [{short_cid}]"
                  }
                }
              }
            }
          }
        }
      }
    }

    "quiz" => {
      let questions = extract_quiz_questions(&deliverable);
      if questions.is_empty() {
        return rsx! {};
      }
      let ans_map = selected_answers();
      let answered_count = ans_map.len();
      let correct_count = ans_map
        .iter()
        .filter(|(q_idx, choice)| questions.get(**q_idx).is_some_and(|q| q.answer == **choice))
        .count();

      rsx! {
        div { class: "space-y-3 rounded-md border border-border p-3 bg-card/50", "data-testid": "interactive-quiz",
          div { class: "flex flex-wrap items-center justify-between gap-2",
            span { class: "text-xs font-medium text-muted-foreground", "data-testid": "quiz-score-banner",
              "自测得分：{correct_count} / {questions.len()}（已答 {answered_count} / {questions.len()}）"
            }
            if answered_count > 0 {
              Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Sm,
                "data-testid": "quiz-reset-btn",
                onclick: move |_| selected_answers.set(BTreeMap::new()),
                "重置测验"
              }
            }
          }

          div { class: "space-y-3",
            for (q_idx, q) in questions.into_iter().enumerate() {
              {
                let chosen = ans_map.get(&q_idx).copied();
                let is_answered = chosen.is_some();
                let explanation = q.explanation.clone();
                let chunk_id = q.chunk_id.clone();
                rsx! {
                  fieldset { key: "quiz-{q_idx}", class: "rounded-md border border-border/80 p-3 space-y-2 bg-background",
                    legend { class: "text-xs font-semibold text-foreground px-1",
                      "{q_idx + 1}. {q.question}"
                    }
                    div { class: "space-y-1.5",
                      for (opt_idx, opt_text) in q.options.into_iter().enumerate() {
                        {
                          let is_selected = chosen == Some(opt_idx);
                          let is_right = opt_idx == q.answer;
                          let style = if is_answered {
                            if is_right {
                              "border-emerald-500/70 bg-emerald-500/10 text-foreground"
                            } else if is_selected {
                              "border-destructive/70 bg-destructive/10 text-foreground"
                            } else {
                              "border-border/50 text-muted-foreground"
                            }
                          } else {
                            "border-border hover:bg-accent/50 text-foreground"
                          };
                          rsx! {
                            button {
                              key: "q-{q_idx}-opt-{opt_idx}",
                              r#type: "button",
                              class: "w-full flex items-center gap-2 rounded border px-2.5 py-1.5 text-left text-xs transition-colors cursor-pointer {style}",
                              "data-testid": "quiz-q{q_idx}-opt{opt_idx}",
                              onclick: move |_| {
                                let mut next = selected_answers();
                                next.insert(q_idx, opt_idx);
                                selected_answers.set(next);
                              },
                              span { class: "font-mono text-[11px] opacity-75",
                                if is_selected { "●" } else { "○" }
                              }
                              span { "{opt_text}" }
                            }
                          }
                        }
                      }
                    }
                    if is_answered && (explanation.is_some() || chunk_id.is_some()) {
                      div { class: "pt-1 border-t border-border/60 text-xs text-muted-foreground space-y-1",
                        if let Some(exp) = explanation {
                          p { "{exp}" }
                        }
                        if let Some(cid) = chunk_id {
                          {
                            let short_cid = short_chunk_label(&cid).to_string();
                            rsx! {
                              Button {
                                variant: ButtonVariant::Ghost,
                                size: ButtonSize::Sm,
                                class: "h-6 px-2 text-xs".to_string(),
                                onclick: move |_| on_inspect_chunk.call(cid.clone()),
                                "溯源分块 [{short_cid}]"
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
          }
        }
      }
    }

    "data_table" => {
      let Some(table) = extract_deliverable_table(&deliverable) else {
        return rsx! {};
      };
      let caption = if deliverable.title.trim().is_empty() {
        "结构化提取数据表".to_string()
      } else {
        deliverable.title.clone()
      };
      rsx! {
        InteractiveSortableTable {
          caption,
          columns: table.columns,
          rows: table.rows,
          on_inspect_chunk: Some(on_inspect_chunk),
        }
      }
    }

    "audio_overview" | "debate_podcast_show" => {
      let (turns, lufs) = extract_audio_script(&deliverable);
      if turns.is_empty() {
        return rsx! {};
      }
      let lufs_note = lufs.map(|v| format!(" · {v:.1} LUFS")).unwrap_or_default();
      rsx! {
        div { class: "space-y-2 rounded-md border border-border p-3 bg-card/50", "data-testid": "interactive-audio-transcript",
          div { class: "text-xs font-medium text-muted-foreground",
            "播客逐字稿 · 共 {turns.len()} 轮对谈{lufs_note}"
          }
          ol { class: "space-y-2 max-h-72 overflow-y-auto pr-1",
            for (idx, turn) in turns.into_iter().enumerate() {
              {
                let role_lower = turn.speaker.to_ascii_lowercase();
                let label = turn.speaker_label.unwrap_or_else(|| match role_lower.as_str() {
                  "host" => "主持人".to_string(),
                  "moderator" => "圆桌主持".to_string(),
                  "pro" => "正方辩手".to_string(),
                  "con" => "反方辩手".to_string(),
                  "guest" => "特邀嘉宾".to_string(),
                  _ => turn.speaker.clone(),
                });
                let variant = match role_lower.as_str() {
                  "pro" => BadgeVariant::Default,
                  "con" => BadgeVariant::Destructive,
                  "guest" => BadgeVariant::Secondary,
                  _ => BadgeVariant::Outline,
                };
                let cid = turn.chunk_id.clone();
                rsx! {
                  li { key: "turn-{idx}", class: "rounded border border-border/70 bg-background p-2.5 space-y-1",
                    div { class: "flex items-center justify-between gap-2",
                      Badge { variant, "{label}" }
                      if let Some(chunk_id) = cid {
                        {
                          let short_cid = short_chunk_label(&chunk_id).to_string();
                          rsx! {
                            Button {
                              variant: ButtonVariant::Ghost,
                              size: ButtonSize::Sm,
                              class: "h-6 px-2 text-xs".to_string(),
                              onclick: move |_| on_inspect_chunk.call(chunk_id.clone()),
                              "[{short_cid}] 溯源"
                            }
                          }
                        }
                      }
                    }
                    p { class: "text-xs leading-relaxed text-foreground whitespace-pre-wrap", "{turn.text}" }
                  }
                }
              }
            }
          }
        }
      }
    }

    "video_overview" | "tech_slides_video" => {
      let (scenes, seconds) = extract_video_scenes(&deliverable);
      if scenes.is_empty() {
        return rsx! {};
      }
      let sec_note = seconds.map(|v| format!(" · {v:.1}s")).unwrap_or_default();
      rsx! {
        div { class: "space-y-2 rounded-md border border-border p-3 bg-card/50", "data-testid": "interactive-video-storyboard",
          div { class: "text-xs font-medium text-muted-foreground",
            "视频分镜脚本 · 共 {scenes.len()} 个镜头{sec_note}"
          }
          ol { class: "space-y-2 max-h-72 overflow-y-auto pr-1",
            for (idx, scene) in scenes.into_iter().enumerate() {
              {
                let label = scene.label.unwrap_or_else(|| format!("镜头 {}", idx + 1));
                let cid = scene.chunk_id.clone();
                rsx! {
                  li { key: "scene-{idx}", class: "rounded border border-border/70 bg-background p-2.5 space-y-1",
                    div { class: "flex items-center justify-between gap-2",
                      div { class: "flex items-center gap-1.5",
                        Badge { variant: BadgeVariant::Outline, "{label}" }
                        if scene.motion {
                          Badge { variant: BadgeVariant::Secondary, "动态运镜" }
                        }
                      }
                      if let Some(chunk_id) = cid {
                        {
                          let short_cid = short_chunk_label(&chunk_id).to_string();
                          rsx! {
                            Button {
                              variant: ButtonVariant::Ghost,
                              size: ButtonSize::Sm,
                              class: "h-6 px-2 text-xs".to_string(),
                              onclick: move |_| on_inspect_chunk.call(chunk_id.clone()),
                              "[{short_cid}] 溯源"
                            }
                          }
                        }
                      }
                    }
                    if !scene.narration.is_empty() {
                      p { class: "text-xs leading-relaxed text-foreground", "{scene.narration}" }
                    }
                    if let Some(vis) = scene.visual {
                      p { class: "text-[11px] text-muted-foreground",
                        strong { "画面提示：" }
                        "{vis}"
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

    _ => rsx! {},
  }
}

// ── AuthorizedMedia Component ────────────────────────────────────────────────

fn requires_header_media_fetch(api_token: &str, tenant_id: &str) -> bool {
  if !api_token.trim().is_empty() || !resolve_embedded_token().is_empty() {
    return true;
  }
  let trimmed = tenant_id.trim();
  !trimmed.is_empty() && trimmed != "default"
}

#[component]
pub fn AuthorizedMedia(
  item: MediaPreviewItem,
  api_token: String,
  #[props(default)] tenant_id: String,
  #[props(default)] header_only: bool,
  #[props(default)] in_drawer: bool,
) -> Element {
  let mut blob_url = use_signal(|| None::<String>);
  let mut text_preview = use_signal(|| None::<String>);
  let mut load_error = use_signal(|| None::<String>);
  let mut loading = use_signal(|| false);
  let mut native_busy = use_signal(|| None::<&'static str>);
  let mut preview_drawer_open = use_signal(|| false);

  let needs_header = requires_header_media_fetch(&api_token, &tenant_id);
  let wants_text = item.kind.wants_text_body();
  let inline_path = append_inline_query(&item.api_path);
  let desktop_native = is_local_desktop_engine();

  // Revoke any previously allocated Blob ObjectURL when unmounting.
  use_drop(move || {
    if let Some(url) = blob_url.peek().as_deref() {
      revoke_blob_url(url);
    }
  });

  let fetch_key = format!(
    "{}|{}|{}|{}|{}",
    inline_path,
    api_token,
    tenant_id,
    header_only,
    item.mime_type.as_deref().unwrap_or("")
  );
  let mime_for_effect = item.mime_type.clone();
  let inline_for_effect = inline_path.clone();
  let token_for_effect = api_token.clone();
  let tenant_for_effect = tenant_id.clone();

  use_effect(use_reactive((&fetch_key,), move |_| {
    let prev_url = blob_url.peek().clone();
    if let Some(old_url) = prev_url {
      revoke_blob_url(&old_url);
      blob_url.set(None);
    }
    text_preview.set(None);
    load_error.set(None);

    if header_only || (!needs_header && !wants_text) {
      loading.set(false);
      return;
    }

    let path = inline_for_effect.clone();
    let tok = token_for_effect.clone();
    let ten = tenant_for_effect.clone();
    let mime = mime_for_effect.clone();
    loading.set(true);

    spawn(async move {
      let outcome = if wants_text {
        fetch_text_resource(&path, &tok, &ten, 65_536)
          .await
          .map(|txt| {
            text_preview.set(Some(txt));
          })
      } else {
        fetch_blob_url(&path, &tok, &ten, mime.as_deref())
          .await
          .map(|url| {
            blob_url.set(Some(url));
          })
      };
      if let Err(err) = outcome {
        load_error.set(Some(err));
      }
      loading.set(false);
    });
  }));

  let effective_url = if needs_header {
    blob_url()
  } else {
    Some(item.direct_url.clone())
  };
  let size_label = format_bytes(item.size_bytes);
  let api_path_dl = item.api_path.clone();
  let label_dl = item.label.clone();
  let tok_dl = api_token.clone();
  let ten_dl = tenant_id.clone();

  let api_path_open = item.api_path.clone();
  let tok_open = api_token.clone();
  let ten_open = tenant_id.clone();

  let api_path_reveal = item.api_path.clone();
  let tok_reveal = api_token.clone();
  let ten_reveal = tenant_id.clone();

  let parsed_table = if item.kind == MediaPreviewKind::Table {
    text_preview().map(|txt| {
      let is_tsv = item.label.to_ascii_lowercase().ends_with(".tsv")
        || item
          .mime_type
          .as_deref()
          .unwrap_or("")
          .to_ascii_lowercase()
          .contains("tab-separated");
      parse_delimited_table(&txt, if is_tsv { '\t' } else { ',' })
    })
  } else {
    None
  };

  rsx! {
    article { class: "rounded-md border border-border bg-background overflow-hidden", "data-testid": "authorized-media-card",
      header { class: "flex flex-wrap items-center justify-between gap-2 px-3 py-2 border-b border-border/70 bg-muted/30",
        div { class: "flex items-center gap-2 min-w-0",
          Badge { variant: BadgeVariant::Secondary, "{item.kind.badge_label()}" }
          strong { class: "text-xs font-semibold truncate", title: "{item.label}", "{item.label}" }
          small { class: "text-[11px] text-muted-foreground shrink-0",
            "#{item.node_id}"
            if !size_label.is_empty() {
              " · {size_label}"
            }
          }
        }
        div { class: "flex items-center gap-1 flex-wrap",
          if !header_only && !in_drawer && item.kind != MediaPreviewKind::Document {
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              "data-testid": "media-expand-preview",
              onclick: move |_| preview_drawer_open.set(true),
              "⤢ 放大"
            }
          }
          if desktop_native {
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              disabled: native_busy().is_some(),
              onclick: move |_| {
                let path = api_path_open.clone();
                let tok = tok_open.clone();
                let ten = ten_open.clone();
                native_busy.set(Some("open"));
                spawn(async move {
                  if let Err(err) = open_desktop_artifact(&path, "open", &tok, &ten).await {
                    load_error.set(Some(err));
                  }
                  native_busy.set(None);
                });
              },
              if native_busy() == Some("open") { "打开中…" } else { "本地打开" }
            }
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              disabled: native_busy().is_some(),
              onclick: move |_| {
                let path = api_path_reveal.clone();
                let tok = tok_reveal.clone();
                let ten = ten_reveal.clone();
                native_busy.set(Some("reveal"));
                spawn(async move {
                  if let Err(err) = open_desktop_artifact(&path, "reveal", &tok, &ten).await {
                    load_error.set(Some(err));
                  }
                  native_busy.set(None);
                });
              },
              if native_busy() == Some("reveal") { "定位中…" } else { "在文件夹中显示" }
            }
          }
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            class: "h-7 px-2 text-xs".to_string(),
            onclick: move |_| {
              let path = api_path_dl.clone();
              let name = label_dl.clone();
              let tok = tok_dl.clone();
              let ten = ten_dl.clone();
              spawn(async move {
                if let Err(err) = download_authenticated_resource(&path, &tok, &ten, &name).await {
                  load_error.set(Some(err));
                }
              });
            },
            "下载 / 打开"
          }
        }
      }

      if !header_only {
        div { class: "p-3",
          if loading() {
            p { class: "text-xs text-muted-foreground", "正在加载预览…" }
          }
          if let Some(err) = load_error() {
            p { class: "text-xs text-destructive", "预览加载失败：{err}" }
          }
          if !loading() && load_error().is_none() {
            match item.kind {
              MediaPreviewKind::Image => rsx! {
                if let Some(src) = effective_url.clone() {
                  img {
                    class: "max-w-full max-h-96 mx-auto rounded border border-border/60 object-contain",
                    src: "{src}",
                    alt: "{item.label}",
                  }
                }
              },
              MediaPreviewKind::Video => rsx! {
                if let Some(src) = effective_url.clone() {
                  video {
                    class: "w-full max-h-96 rounded bg-black",
                    src: "{src}",
                    controls: true,
                    preload: "metadata",
                  }
                }
              },
              MediaPreviewKind::Audio => rsx! {
                if let Some(src) = effective_url.clone() {
                  audio {
                    class: "w-full",
                    src: "{src}",
                    controls: true,
                    preload: "metadata",
                  }
                }
              },
              MediaPreviewKind::Html => rsx! {
                if let Some(src) = effective_url.clone() {
                  iframe {
                    class: if in_drawer { "w-full h-[78vh] rounded border border-border" } else { "w-full h-72 rounded border border-border" },
                    src: "{src}",
                    title: "{item.label}",
                    "sandbox": "allow-scripts",
                  }
                }
              },
              MediaPreviewKind::Pdf => rsx! {
                if let Some(src) = effective_url.clone() {
                  iframe {
                    class: if in_drawer { "w-full h-[78vh] rounded border border-border" } else { "w-full h-80 rounded border border-border" },
                    src: "{src}",
                    title: "{item.label}",
                  }
                }
              },
              MediaPreviewKind::Table => rsx! {
                if let Some(tbl) = parsed_table {
                  InteractiveSortableTable {
                    caption: item.label.clone(),
                    columns: tbl.columns,
                    rows: tbl.rows,
                    test_id: "media-csv-table".to_string(),
                  }
                }
              },
              MediaPreviewKind::Subtitle | MediaPreviewKind::Markdown => rsx! {
                if let Some(txt) = text_preview() {
                  pre { class: "text-xs font-mono whitespace-pre-wrap break-words max-h-80 overflow-y-auto rounded bg-muted/40 p-2.5",
                    "{txt}"
                  }
                }
              },
              MediaPreviewKind::Document => rsx! {
                p { class: "text-xs text-muted-foreground",
                  "已生成可编辑套件产物（{item.label}），点击右上角按钮下载或本地打开。"
                }
              },
            }
          }
        }
      }

      if !in_drawer && preview_drawer_open() {
        Sheet {
          open: Some(preview_drawer_open()),
          on_open_change: move |next| preview_drawer_open.set(next),
          SheetOverlay {}
          SheetContent {
            side: SheetSide::Right,
            size: SheetSize::Full,
            class: "flex flex-col gap-3 p-4 overflow-y-auto".to_string(),
            "data-testid": "media-preview-drawer",
            SheetHeader {
              SheetTitle { "{item.label}" }
              SheetDescription {
                "{item.kind.badge_label()} · #{item.node_id}"
                if !size_label.is_empty() {
                  " · {size_label}"
                }
              }
            }
            SheetClose { "关闭" }
            AuthorizedMedia {
              item: item.clone(),
              api_token: api_token.clone(),
              tenant_id: tenant_id.clone(),
              in_drawer: true,
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

  #[test]
  fn rfc4180_csv_and_tsv_parsing_handles_quotes_and_crlf() {
    let csv = "Name,Score,Note\r\n\"Alpha, Inc.\",98.5,\"Said \"\"hello\"\"\"\r\nBeta,12,ok\n";
    let parsed = parse_delimited_table(csv, ',');
    assert_eq!(parsed.columns, vec!["Name", "Score", "Note"]);
    assert_eq!(parsed.rows.len(), 2);
    assert_eq!(parsed.rows[0].cells[0], "Alpha, Inc.");
    assert_eq!(parsed.rows[0].cells[2], "Said \"hello\"");
    assert_eq!(parsed.rows[1].cells[1], "12");

    let tsv = "Key\tVal\nA\t10\nB\t2\n";
    let parsed_tsv = parse_delimited_table(tsv, '\t');
    assert_eq!(parsed_tsv.rows[1].cells[1], "2");
  }

  #[test]
  fn natural_cmp_orders_numbers_numerically() {
    assert_eq!(natural_cmp("2", "10"), Ordering::Less);
    assert_eq!(natural_cmp("1,200", "900"), Ordering::Greater);
    assert_eq!(natural_cmp("alpha", "Beta"), Ordering::Less);
  }

  #[test]
  fn video_storyboard_extracts_scenes_and_slide_fallbacks() {
    let slide_deliverable = NotebookDeliverable {
      kind: "tech_slides_video".to_string(),
      title: "Deep Dive".to_string(),
      content: Some(serde_json::json!({
        "seconds": 42.5,
        "slides": [
          {
            "title": "Architecture",
            "steps": [{ "spoken": "Welcome to Yanshi." }],
            "takeaway": "Five layers",
            "chunk_id": "s1:0"
          }
        ]
      })),
      citations: vec![],
    };
    let (scenes, secs) = extract_video_scenes(&slide_deliverable);
    assert_eq!(secs, Some(42.5));
    assert_eq!(scenes.len(), 1);
    assert_eq!(scenes[0].label.as_deref(), Some("幻灯片 1"));
    assert_eq!(scenes[0].narration, "Welcome to Yanshi.");
    assert_eq!(scenes[0].chunk_id.as_deref(), Some("s1:0"));
  }
}
