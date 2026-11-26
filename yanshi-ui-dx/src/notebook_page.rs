//! Three-pane Notebook Workspace (`/ui-dx/notebooks` and `/ui-dx/notebooks/:id`)
//! built with `dioxus-shadcn` (`Resizable`, `Tabs`, `Combobox`, `Dialog`,
//! `Sheet`, `Checkbox`, `NativeSelect`, `Table`, `Card`).

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardDescription, CardHeader, CardTitle, Checkbox, Combobox, ComboboxContent,
  ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, Dialog, DialogClose, DialogContent,
  DialogDescription, DialogOverlay, DialogTitle, Input, Label, LayoutOrientation, NativeSelect,
  ResizableHandle, ResizablePanel, ResizablePanelGroup, ResizablePanelState, Separator, Tabs,
  TabsContent, TabsList, TabsTrigger, Textarea, resizable_resize_pair,
};

use crate::api::{
  ModelConfigItem, SseAbortHandle, clear_file_input, download_authenticated_resource,
  encode_uri_component, read_input_file_bytes, read_storage, sleep_ms, stream_sse,
  trigger_text_download, write_storage,
};
use crate::deliverable_viewer::{
  AuthorizedMedia, NotebookDeliverableCitationBar, NotebookDeliverableViewer,
  has_exclusive_deliverable_view,
};
use crate::notebook_api::{
  MAX_STUDIO_BATCH_ITEMS, Notebook, NotebookArtifact, NotebookAskResponse, NotebookChunk,
  NotebookDeliverable, NotebookPlanResponse, NotebookSearchHit, NotebookSource, ParamFieldKind,
  STUDIO_GENERATORS, StudioGeneratorSpec, TrackedGeneratorRun, ask_notebook,
  build_ask_history_payload, build_generator_params, build_initial_generator_drafts,
  clear_notebook_messages, create_notebook, create_notebook_source, delete_notebook,
  delete_notebook_artifact, delete_notebook_source, encode_base64, export_notebook_bundle,
  format_ask_result_as_note_text, generate_notebook, generate_notebook_batch, get_chunk,
  get_notebook_detail, get_run_status, get_source_chunks, import_notebook_bundle, is_run_terminal,
  list_notebook_artifacts, list_notebook_messages, list_notebooks, notebook_artifact_to_media_item,
  parse_cost_limit_usd_draft, patch_notebook_source, plan_notebook, rename_notebook,
  resolve_scoped_source_ids, retry_notebook_source, search_notebook, source_status_label,
};

const NB_TENANT_STORAGE_KEY: &str = "yanshi.ui.tenantId";
const NB_MODEL_STORAGE_KEY: &str = "yanshi.studio.modelOverride";

#[component]
pub fn NotebookWorkspacePage(
  initial_notebook_id: Option<String>,
  api_token: String,
  on_token_change: EventHandler<String>,
  models: Vec<ModelConfigItem>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut tenant_id = use_signal(|| read_storage(NB_TENANT_STORAGE_KEY, "default"));
  let mut model_name = use_signal(|| read_storage(NB_MODEL_STORAGE_KEY, ""));
  let mut model_query = use_signal(|| read_storage(NB_MODEL_STORAGE_KEY, ""));
  let mut model_combo_open = use_signal(|| false);
  let mut cost_limit_draft = use_signal(String::new);

  // Notebooks list & active selection
  let mut notebooks = use_signal(Vec::<Notebook>::new);
  let mut selected_notebook_id = use_signal(|| initial_notebook_id.clone().unwrap_or_default());
  let mut active_notebook = use_signal(|| None::<Notebook>);
  let mut sources = use_signal(Vec::<NotebookSource>::new);
  let mut artifacts = use_signal(Vec::<NotebookArtifact>::new);
  let mut ask_thread = use_signal(Vec::<NotebookAskResponse>::new);
  let mut loading_list = use_signal(|| false);

  // Banners
  let mut error_banner = use_signal(|| None::<String>);
  let mut notice_banner = use_signal(|| None::<String>);

  // Top bar notebook CRUD & import/export
  let mut new_nb_title = use_signal(String::new);
  let mut create_nb_busy = use_signal(|| false);
  let mut renaming_nb = use_signal(|| false);
  let mut rename_title_draft = use_signal(String::new);
  let mut rename_nb_busy = use_signal(|| false);
  let mut delete_dialog_open = use_signal(|| false);
  let mut delete_nb_busy = use_signal(|| false);
  let mut export_busy = use_signal(|| false);
  let mut import_busy = use_signal(|| false);
  let mut zip_busy = use_signal(|| false);

  // Pane sizes & collapse state
  let mut left_pct = use_signal(|| 28.0_f64);
  let mut center_pct = use_signal(|| 38.0_f64);
  let mut right_pct = use_signal(|| 34.0_f64);
  let mut left_collapsed = use_signal(|| false);
  let mut right_collapsed = use_signal(|| false);

  // Left pane: source ingestion & scoping & BM25 search
  let mut ingest_tab = use_signal(|| "text".to_string());
  let mut source_title_draft = use_signal(String::new);
  let mut source_text_draft = use_signal(String::new);
  let mut source_url_draft = use_signal(String::new);
  let mut recording_model_draft = use_signal(String::new);
  let mut recording_lang_draft = use_signal(|| "zh".to_string());
  let mut ingest_busy = use_signal(|| false);
  let mut excluded_source_ids = use_signal(HashSet::<String>::new);
  let mut deleting_source_id = use_signal(|| None::<String>);
  let mut retrying_source_id = use_signal(|| None::<String>);
  let mut browse_source_busy_id = use_signal(|| None::<String>);
  let mut search_query = use_signal(String::new);
  let mut search_busy = use_signal(|| false);
  let mut search_hits = use_signal(|| None::<Vec<NotebookSearchHit>>);

  // Chunk reader dialog
  let mut chunk_dialog_open = use_signal(|| false);
  let mut inspected_chunk = use_signal(|| None::<NotebookChunk>);
  let mut reader_chunks = use_signal(Vec::<NotebookChunk>::new);
  let mut reader_source_id = use_signal(String::new);

  // Edit source dialog
  let mut edit_source_open = use_signal(|| false);
  let mut editing_source = use_signal(|| None::<NotebookSource>);
  let mut edit_source_title = use_signal(String::new);
  let mut edit_source_text = use_signal(String::new);
  let mut initial_edit_source_text = use_signal(String::new);
  let mut edit_source_loading_text = use_signal(|| false);
  let mut edit_source_busy = use_signal(|| false);

  // Center pane: Grounded Ask
  let mut ask_draft = use_signal(String::new);
  let mut ask_busy = use_signal(|| false);
  let mut clear_ask_busy = use_signal(|| false);
  let mut save_ask_busy = use_signal(|| false);

  // Center pane: AI Studio Plan & 12 Generators & live runs
  let mut plan_goal_draft = use_signal(String::new);
  let mut plan_busy = use_signal(|| false);
  let mut studio_plan = use_signal(|| None::<NotebookPlanResponse>);
  let mut expanded_kind = use_signal(|| "report".to_string());
  let mut param_drafts = use_signal(build_initial_generator_drafts);
  let mut generate_busy_kind = use_signal(|| None::<String>);
  let mut batch_selected_kinds = use_signal(Vec::<String>::new);
  let mut batch_busy = use_signal(|| false);
  let mut tracked_runs = use_signal(Vec::<TrackedGeneratorRun>::new);
  let mut run_generation = use_signal(|| 0u64);

  // Right pane: Artifacts gallery
  let mut artifact_filter = use_signal(|| "all".to_string());
  let mut deleting_artifact_id = use_signal(|| None::<String>);

  // Sync URL initial_notebook_id prop changes
  let prop_nb_id = initial_notebook_id.clone();
  use_effect(use_reactive((&prop_nb_id,), move |(next_opt,)| {
    if let Some(id) = next_opt
      && !id.is_empty()
      && id != *selected_notebook_id.peek()
    {
      selected_notebook_id.set(id);
    }
  }));

  // Load notebooks list whenever tenant or api_token changes
  let list_tok = api_token.clone();
  let list_ten = tenant_id();
  use_effect(use_reactive((&list_tok, &list_ten), move |(tok, ten)| {
    loading_list.set(true);
    spawn(async move {
      match list_notebooks(&ten, &tok).await {
        Ok(list) => {
          let cur = selected_notebook_id.peek().clone();
          let next_sel = if !cur.is_empty() && list.iter().any(|n| n.id == cur) {
            cur
          } else {
            list.first().map(|n| n.id.clone()).unwrap_or_default()
          };
          notebooks.set(list);
          if next_sel != *selected_notebook_id.peek() {
            selected_notebook_id.set(next_sel);
          }
        }
        Err(err) => error_banner.set(Some(format!("读取笔记本列表失败：{err}"))),
      }
      loading_list.set(false);
    });
  }));

  // Load selected notebook detail + artifacts + messages
  let sel_id = selected_notebook_id();
  let det_tok = api_token.clone();
  let det_ten = tenant_id();
  use_effect(use_reactive(
    (&sel_id, &det_tok, &det_ten),
    move |(nb_id, tok, ten)| {
      let next_gen = run_generation.peek().wrapping_add(1);
      run_generation.set(next_gen);
      excluded_source_ids.set(HashSet::new());
      ask_thread.set(Vec::new());
      studio_plan.set(None);
      batch_selected_kinds.set(Vec::new());
      tracked_runs.set(Vec::new());
      search_hits.set(None);
      renaming_nb.set(false);
      inspected_chunk.set(None);
      reader_chunks.set(Vec::new());
      reader_source_id.set(String::new());

      if nb_id.is_empty() {
        active_notebook.set(None);
        sources.set(Vec::new());
        artifacts.set(Vec::new());
        return;
      }

      spawn(async move {
        match get_notebook_detail(&nb_id, &ten, &tok).await {
          Ok(detail) => {
            active_notebook.set(Some(detail.notebook));
            sources.set(detail.sources);
          }
          Err(err) => {
            active_notebook.set(None);
            sources.set(Vec::new());
            error_banner.set(Some(format!("读取笔记本详情失败：{err}")));
            return;
          }
        }
        if let Ok(arts) = list_notebook_artifacts(&nb_id, &ten, &tok).await {
          artifacts.set(arts);
        }
        if let Ok(msgs) = list_notebook_messages(&nb_id, &ten, &tok).await {
          ask_thread.set(msgs.into_iter().map(NotebookAskResponse::from).collect());
        }
      });
    },
  ));

  // Poll sources every 1.2s while any source is pending/ingesting
  let has_ingesting = sources()
    .iter()
    .any(|s| s.status == "pending" || s.status == "ingesting");
  let poll_nb_id = selected_notebook_id();
  let poll_tok = api_token.clone();
  let poll_ten = tenant_id();
  use_effect(use_reactive(
    (&has_ingesting, &poll_nb_id, &poll_tok, &poll_ten),
    move |(ingesting, nb_id, tok, ten)| {
      if !ingesting || nb_id.is_empty() {
        return;
      }
      let expected_gen = *run_generation.peek();
      spawn(async move {
        sleep_ms(1200).await;
        if *run_generation.peek() != expected_gen || *selected_notebook_id.peek() != nb_id {
          return;
        }
        if let Ok(detail) = get_notebook_detail(&nb_id, &ten, &tok).await {
          active_notebook.set(Some(detail.notebook));
          sources.set(detail.sources);
        }
      });
    },
  ));

  // Derived metadata
  let cur_sources = sources();
  let excluded_set = excluded_source_ids();
  let ready_count = cur_sources.iter().filter(|s| s.status == "ready").count();
  let selected_ready_count = cur_sources
    .iter()
    .filter(|s| s.status == "ready" && !excluded_set.contains(&s.id))
    .count();
  let scoped_source_ids = resolve_scoped_source_ids(&cur_sources, &excluded_set);
  let source_title_by_id: HashMap<String, String> = cur_sources
    .iter()
    .map(|s| (s.id.clone(), s.title.clone()))
    .collect();

  // Filtered model suggestions for Combobox
  let mq = model_query().trim().to_lowercase();
  let filtered_models: Vec<String> = models
    .iter()
    .map(|m| m.name.clone())
    .filter(|name| mq.is_empty() || name.to_lowercase().contains(&mq))
    .take(24)
    .collect();

  // Inspect chunk helper (opens reader dialog)
  let inspect_chunk_tok = api_token.clone();
  let inspect_chunk_ten = tenant_id();
  let inspect_chunk_nb = selected_notebook_id();
  let on_inspect_chunk = use_callback(move |raw_chunk_id: String| {
    let cid = raw_chunk_id.trim().to_string();
    if cid.is_empty() || inspect_chunk_nb.is_empty() {
      return;
    }
    let nb_id = inspect_chunk_nb.clone();
    let tok = inspect_chunk_tok.clone();
    let ten = inspect_chunk_ten.clone();
    error_banner.set(None);
    spawn(async move {
      match get_chunk(&nb_id, &cid, &ten, &tok).await {
        Ok(chunk) => {
          if *reader_source_id.peek() != chunk.source_id {
            reader_chunks.set(Vec::new());
            reader_source_id.set(String::new());
          }
          inspected_chunk.set(Some(chunk));
          chunk_dialog_open.set(true);
        }
        Err(err) => error_banner.set(Some(format!("查询分块 {cid} 失败：{err}"))),
      }
    });
  });

  // Watch dispatched generator run via SSE + terminal poll
  let watch_run = {
    let tok = api_token.clone();
    let ten = tenant_id();
    move |nb_id: String, run_id: String, kind: String, initial_status: String| {
      let mut list = tracked_runs();
      list.retain(|r| r.run_id != run_id);
      list.insert(
        0,
        TrackedGeneratorRun {
          run_id: run_id.clone(),
          kind: kind.clone(),
          status: if initial_status.is_empty() {
            "running".to_string()
          } else {
            initial_status
          },
          completed_nodes: 0,
          total_cost_usd: 0.0,
        },
      );
      list.truncate(8);
      tracked_runs.set(list);

      let expected_gen = *run_generation.peek();
      let tok = tok.clone();
      let ten = ten.clone();
      spawn(async move {
        let Ok(abort_handle) = SseAbortHandle::new() else {
          return;
        };
        let sse_path = format!("/v1/runs/{}/events", encode_uri_component(&run_id));
        let mut completed_nodes_set = HashSet::<String>::new();
        let mut total_cost = 0.0_f64;
        let rid_for_cb = run_id.clone();

        let _ = stream_sse(&sse_path, &tok, &ten, &abort_handle, move |frame| {
          if *run_generation.peek() != expected_gen {
            return;
          }
          let Ok(val) = serde_json::from_str::<serde_json::Value>(&frame.data) else {
            return;
          };
          let kind_str = val
            .get("kind")
            .or_else(|| val.get("event"))
            .and_then(|v| v.as_str())
            .unwrap_or(frame.event.as_str());
          let node_id = val
            .get("node_id")
            .or_else(|| val.get("payload").and_then(|p| p.get("node_id")))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
          if matches!(
            kind_str,
            "node_succeeded" | "node_completed" | "node.completed" | "node_skipped"
          ) && let Some(nid) = node_id
          {
            completed_nodes_set.insert(nid);
          }
          if let Some(cost) = val
            .get("cost_usd")
            .or_else(|| val.get("payload").and_then(|p| p.get("cost_usd")))
            .and_then(|v| v.as_f64())
            && cost.is_finite()
          {
            total_cost += cost;
          }
          let count = completed_nodes_set.len();
          let mut cur = tracked_runs();
          for item in &mut cur {
            if item.run_id == rid_for_cb {
              item.completed_nodes = count;
              item.total_cost_usd = total_cost;
            }
          }
          tracked_runs.set(cur);
        })
        .await;

        // Short poll GET /v1/runs/:id until terminal (runs.rs Y1.3 invariant).
        for _ in 0..80 {
          if *run_generation.peek() != expected_gen {
            return;
          }
          let Ok(summary) = get_run_status(&run_id, &ten, &tok).await else {
            break;
          };
          let mut cur = tracked_runs();
          for item in &mut cur {
            if item.run_id == run_id {
              item.status = summary.status.clone();
            }
          }
          tracked_runs.set(cur);

          if is_run_terminal(&summary.status) {
            if summary.status == "failed" {
              let msg = summary.error.unwrap_or_else(|| "未知错误".to_string());
              error_banner.set(Some(format!("生成任务 ({kind}) 失败：{msg}")));
            } else if summary.status == "succeeded" || summary.status == "completed" {
              notice_banner.set(Some(format!("「{kind}」成品已生成并自动挂载至右侧画廊。")));
            }
            break;
          }
          sleep_ms(120).await;
        }

        if *run_generation.peek() == expected_gen
          && let Ok(arts) = list_notebook_artifacts(&nb_id, &ten, &tok).await
        {
          artifacts.set(arts);
        }
      });
    }
  };

  // Filtered artifacts (newest first)
  let all_artifacts = artifacts();
  let mut artifact_kinds: Vec<String> = all_artifacts
    .iter()
    .map(|a| a.kind.clone())
    .collect::<HashSet<_>>()
    .into_iter()
    .collect();
  artifact_kinds.sort();
  let cur_art_filter = artifact_filter();
  let filtered_artifacts: Vec<NotebookArtifact> = all_artifacts
    .iter()
    .rev()
    .filter(|a| cur_art_filter == "all" || a.kind == cur_art_filter)
    .cloned()
    .collect();

  // Compute effective panel sizes considering collapsed panes
  let eff_left = if left_collapsed() { 0.0 } else { left_pct() };
  let eff_right = if right_collapsed() { 0.0 } else { right_pct() };
  let eff_center = (100.0 - eff_left - eff_right).max(20.0);

  let active_nb_title = active_notebook()
    .map(|n| n.title)
    .unwrap_or_else(|| "未选择笔记本".to_string());

  let chunk_dialog_title = if let Some(chunk) = inspected_chunk() {
    let src_title = source_title_by_id
      .get(&chunk.source_id)
      .cloned()
      .unwrap_or_else(|| chunk.source_id.clone());
    format!("原文分块阅读 · {src_title}")
  } else {
    "原文分块阅读".to_string()
  };

  let reader_idx = inspected_chunk().as_ref().and_then(|cur| {
    if reader_source_id() == cur.source_id {
      reader_chunks()
        .iter()
        .position(|c| c.chunk_id == cur.chunk_id)
    } else {
      None
    }
  });

  rsx! {
    div { class: "flex flex-col h-[calc(100vh-53px)] overflow-hidden", "data-testid": "notebook-workspace",
      // ── Top Notebook Toolbar ──────────────────────────────────────────────
      div { class: "border-b border-border bg-card/60 px-4 py-2 flex flex-wrap items-center justify-between gap-2 shrink-0",
        div { class: "flex flex-wrap items-center gap-2",
          div { class: "w-56",
            NativeSelect {
              value: selected_notebook_id(),
              class: "h-8 py-1 text-xs".to_string(),
              "data-testid": "notebook-selector",
              on_value_change: move |next_id: String| {
                selected_notebook_id.set(next_id.clone());
                if next_id.is_empty() {
                  on_navigate.call("/ui/notebooks".to_string());
                } else {
                  on_navigate.call(format!("/ui/notebooks/{}", encode_uri_component(&next_id)));
                }
              },
              if notebooks().is_empty() {
                option { value: "", "暂无笔记本（请先新建）" }
              }
              for nb in notebooks() {
                option { key: "{nb.id}", value: "{nb.id}", "{nb.title}" }
              }
            }
          }

          // Inline Create Notebook
          div { class: "flex items-center gap-1",
            div { class: "w-44",
              Input {
                value: new_nb_title(),
                placeholder: "新建笔记本标题…".to_string(),
                class: "h-8 text-xs".to_string(),
                "data-testid": "new-notebook-title-input",
                on_value_change: move |v: String| new_nb_title.set(v),
              }
            }
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              disabled: create_nb_busy(),
              "data-testid": "create-notebook-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                move |_| {
                  let title = new_nb_title().trim().to_string();
                  if title.is_empty() {
                    error_banner.set(Some("请输入笔记本标题后再创建。".to_string()));
                    return;
                  }
                  let tok = tok.clone();
                  let ten = ten.clone();
                  create_nb_busy.set(true);
                  error_banner.set(None);
                  spawn(async move {
                    match create_notebook(&title, &ten, &tok).await {
                      Ok(created) => {
                        new_nb_title.set(String::new());
                        let id = created.id.clone();
                        let t = created.title.clone();
                        let mut list = notebooks();
                        list.retain(|n| n.id != id);
                        list.insert(0, created);
                        notebooks.set(list);
                        selected_notebook_id.set(id.clone());
                        on_navigate.call(format!("/ui/notebooks/{}", encode_uri_component(&id)));
                        notice_banner.set(Some(format!("已创建笔记本「{t}」，请添加资料来源。")));
                      }
                      Err(err) => error_banner.set(Some(format!("创建笔记本失败：{err}"))),
                    }
                    create_nb_busy.set(false);
                  });
                }
              },
              if create_nb_busy() { "创建中…" } else { "+ 新建" }
            }
          }

          if active_notebook().is_some() {
            if renaming_nb() {
              div { class: "flex items-center gap-1",
                div { class: "w-40",
                  Input {
                    value: rename_title_draft(),
                    class: "h-8 text-xs".to_string(),
                    "data-testid": "rename-notebook-input",
                    on_value_change: move |v: String| rename_title_draft.set(v),
                  }
                }
                Button {
                  variant: ButtonVariant::Secondary,
                  size: ButtonSize::Sm,
                  disabled: rename_nb_busy(),
                  "data-testid": "rename-notebook-save-btn",
                  onclick: {
                    let tok = api_token.clone();
                    let ten = tenant_id();
                    let nb_id = selected_notebook_id();
                    move |_| {
                      let title = rename_title_draft().trim().to_string();
                      if title.is_empty() {
                        error_banner.set(Some("笔记本标题不能为空。".to_string()));
                        return;
                      }
                      let tok = tok.clone();
                      let ten = ten.clone();
                      let nb_id = nb_id.clone();
                      rename_nb_busy.set(true);
                      spawn(async move {
                        match rename_notebook(&nb_id, &title, &ten, &tok).await {
                          Ok(updated) => {
                            let new_title = updated.title.clone();
                            active_notebook.set(Some(updated.clone()));
                            let mut list = notebooks();
                            for item in &mut list {
                              if item.id == updated.id {
                                *item = updated.clone();
                              }
                            }
                            notebooks.set(list);
                            renaming_nb.set(false);
                            notice_banner.set(Some(format!("已将笔记本重命名为「{new_title}」。")));
                          }
                          Err(err) => error_banner.set(Some(format!("重命名失败：{err}"))),
                        }
                        rename_nb_busy.set(false);
                      });
                    }
                  },
                  "保存"
                }
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  onclick: move |_| renaming_nb.set(false),
                  "取消"
                }
              }
            } else {
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                "data-testid": "rename-notebook-btn",
                onclick: {
                  let cur_title = active_nb_title.clone();
                  move |_| {
                    rename_title_draft.set(cur_title.clone());
                    renaming_nb.set(true);
                  }
                },
                "重命名"
              }
            }

            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              disabled: export_busy(),
              "data-testid": "export-notebook-json-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                let nb_id = selected_notebook_id();
                move |_| {
                  let tok = tok.clone();
                  let ten = ten.clone();
                  let nb_id = nb_id.clone();
                  export_busy.set(true);
                  spawn(async move {
                    match export_notebook_bundle(&nb_id, &ten, &tok).await {
                      Ok(bundle) => {
                        let short_id: String = nb_id.chars().take(8).collect();
                        let fname = format!("yanshi-notebook-{short_id}.json");
                        match serde_json::to_string_pretty(&bundle) {
                          Ok(json_str) => {
                            if let Err(err) =
                              trigger_text_download(&json_str, "application/json;charset=utf-8", &fname)
                            {
                              error_banner.set(Some(err));
                            } else {
                              notice_banner.set(Some(format!("已导出可移植笔记本包「{fname}」。")));
                            }
                          }
                          Err(err) => error_banner.set(Some(err.to_string())),
                        }
                      }
                      Err(err) => error_banner.set(Some(format!("导出笔记本失败：{err}"))),
                    }
                    export_busy.set(false);
                  });
                }
              },
              "导出 JSON"
            }

            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              disabled: export_busy(),
              "data-testid": "export-notebook-brief-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                let nb_id = selected_notebook_id();
                move |_| {
                  let tok = tok.clone();
                  let ten = ten.clone();
                  let nb_id = nb_id.clone();
                  export_busy.set(true);
                  spawn(async move {
                    match export_notebook_bundle(&nb_id, &ten, &tok).await {
                      Ok(bundle) => {
                        let short_id: String = nb_id.chars().take(8).collect();
                        let fname = format!("yanshi-notebook-{short_id}-briefing.md");
                        let md = bundle
                          .get("markdown_brief")
                          .and_then(|v| v.as_str())
                          .unwrap_or("");
                        if let Err(err) =
                          trigger_text_download(md, "text/markdown;charset=utf-8", &fname)
                        {
                          error_banner.set(Some(err));
                        } else {
                          notice_banner.set(Some(format!("已导出研究简报「{fname}」。")));
                        }
                      }
                      Err(err) => error_banner.set(Some(format!("导出简报失败：{err}"))),
                    }
                    export_busy.set(false);
                  });
                }
              },
              "导出简报"
            }

            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              disabled: zip_busy(),
              "data-testid": "download-artifacts-zip-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                let nb_id = selected_notebook_id();
                move |_| {
                  let tok = tok.clone();
                  let ten = ten.clone();
                  let nb_id = nb_id.clone();
                  zip_busy.set(true);
                  spawn(async move {
                    let short_id: String = nb_id.chars().take(8).collect();
                    let fname = format!("yanshi-notebook-{short_id}-artifacts.zip");
                    let path = format!("/v1/notebooks/{}/artifacts.zip", encode_uri_component(&nb_id));
                    match download_authenticated_resource(&path, &tok, &ten, &fname).await {
                      Ok(()) => notice_banner.set(Some(format!("已打包下载全部成品与 BRIEFING.md（{fname}）。"))),
                      Err(err) => error_banner.set(Some(format!("打包下载成品失败：{err}"))),
                    }
                    zip_busy.set(false);
                  });
                }
              },
              if zip_busy() { "打包中…" } else { "打包 ZIP" }
            }

            Button {
              variant: ButtonVariant::Destructive,
              size: ButtonSize::Sm,
              "data-testid": "delete-notebook-btn",
              onclick: move |_| delete_dialog_open.set(true),
              "删除"
            }
          }

          // Portable JSON import
          label { class: "inline-flex items-center gap-1.5 text-xs text-muted-foreground cursor-pointer border border-dashed border-border rounded-md px-2 py-1 hover:bg-accent/40",
            input {
              id: "nb-import-json-input",
              r#type: "file",
              accept: ".json,application/json",
              class: "hidden",
              onchange: {
                let tok = api_token.clone();
                let ten = tenant_id();
                move |_| {
                  let tok = tok.clone();
                  let ten = ten.clone();
                  import_busy.set(true);
                  error_banner.set(None);
                  spawn(async move {
                    match read_input_file_bytes("nb-import-json-input").await {
                      Ok(Some((fname, bytes))) => {
                        match serde_json::from_slice::<serde_json::Value>(&bytes) {
                          Ok(parsed) => match import_notebook_bundle(&parsed, &ten, &tok).await {
                            Ok(imported) => {
                              let nb = imported.notebook.clone();
                              let id = nb.id.clone();
                              let mut list = notebooks();
                              list.retain(|n| n.id != id);
                              list.insert(0, nb);
                              notebooks.set(list);
                              selected_notebook_id.set(id.clone());
                              on_navigate.call(format!("/ui/notebooks/{}", encode_uri_component(&id)));
                              let deliv = imported
                                .imported_deliverables
                                .filter(|n| *n > 0)
                                .map(|n| format!("、{n} 份交付物已恢复"))
                                .unwrap_or_default();
                              notice_banner.set(Some(format!(
                                "已导入笔记本「{}」（{} 份来源、{} 个分块{}）。",
                                imported.notebook.title,
                                imported.imported_sources,
                                imported.imported_chunks,
                                deliv
                              )));
                            }
                            Err(err) => error_banner.set(Some(format!("导入笔记本失败：{err}"))),
                          },
                          Err(_) => error_banner.set(Some(format!("文件「{fname}」不是合法的 JSON 笔记本包。"))),
                        }
                        clear_file_input("nb-import-json-input");
                      }
                      Ok(None) => {}
                      Err(err) => error_banner.set(Some(err)),
                    }
                    import_busy.set(false);
                  });
                }
              },
            }
            if import_busy() { "导入中…" } else { "导入 JSON" }
          }
        }

        // Right toolbar: tenant, model Combobox, cost limit, pane toggles
        div { class: "flex flex-wrap items-center gap-2",
          div { class: "w-24",
            Input {
              value: tenant_id(),
              placeholder: "tenant".to_string(),
              class: "h-8 text-xs".to_string(),
              "data-testid": "nb-tenant-input",
              on_value_change: move |v: String| {
                write_storage(NB_TENANT_STORAGE_KEY, &v);
                tenant_id.set(v);
              },
            }
          }
          div { class: "w-32",
            Input {
              value: api_token.clone(),
              placeholder: "API Token".to_string(),
              r#type: "password",
              class: "h-8 text-xs".to_string(),
              "data-testid": "nb-token-input",
              on_value_change: move |v: String| on_token_change.call(v),
            }
          }
          div { class: "w-48 rounded-md border border-input bg-background", "data-testid": "nb-model-combobox",
            Combobox {
              value: if model_name().is_empty() { None } else { Some(model_name()) },
              open: Some(model_combo_open()),
              allow_custom_value: true,
              on_open_change: move |next| model_combo_open.set(next),
              on_value_change: move |v: String| {
                write_storage(NB_MODEL_STORAGE_KEY, &v);
                model_name.set(v.clone());
                model_query.set(v);
              },
              ComboboxInput {
                value: model_query(),
                placeholder: "模型覆盖（默认后端）".to_string(),
                class: "h-8 py-1 text-xs".to_string(),
                "data-testid": "nb-model-input",
                on_value_change: move |v: String| {
                  model_query.set(v.clone());
                  write_storage(NB_MODEL_STORAGE_KEY, &v);
                  model_name.set(v);
                  model_combo_open.set(true);
                },
                onfocus: move |_| model_combo_open.set(true),
              }
              ComboboxContent {
                ComboboxList {
                  for m in filtered_models {
                    ComboboxItem { key: "{m}", value: "{m}", "{m}" }
                  }
                  ComboboxEmpty { "按 Enter 直接使用自定义模型名" }
                }
              }
            }
          }
          div { class: "w-24",
            Input {
              value: cost_limit_draft(),
              placeholder: "预算 USD".to_string(),
              class: "h-8 text-xs".to_string(),
              "data-testid": "nb-cost-limit-input",
              on_value_change: move |v: String| cost_limit_draft.set(v),
            }
          }
          Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Sm,
            class: "h-8 px-2 text-xs".to_string(),
            "data-testid": "toggle-left-pane-btn",
            onclick: move |_| left_collapsed.set(!left_collapsed()),
            if left_collapsed() { "展开左栏" } else { "收起左栏" }
          }
          Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Sm,
            class: "h-8 px-2 text-xs".to_string(),
            "data-testid": "toggle-right-pane-btn",
            onclick: move |_| right_collapsed.set(!right_collapsed()),
            if right_collapsed() { "展开右栏" } else { "收起右栏" }
          }
          Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Sm,
            class: "h-8 px-2 text-xs".to_string(),
            "data-testid": "reset-panes-btn",
            onclick: move |_| {
              left_collapsed.set(false);
              right_collapsed.set(false);
              left_pct.set(28.0);
              center_pct.set(38.0);
              right_pct.set(34.0);
            },
            "重置分栏"
          }
        }
      }

      // ── Error & Notice Banners ────────────────────────────────────────────
      if let Some(err) = error_banner() {
        div { class: "px-4 pt-2 shrink-0", "data-testid": "notebook-error-banner",
          Alert { variant: AlertVariant::Destructive,
            AlertDescription {
              div { class: "flex items-center justify-between gap-2 text-xs",
                span { "{err}" }
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  class: "h-6 px-2 text-xs".to_string(),
                  onclick: move |_| error_banner.set(None),
                  "关闭"
                }
              }
            }
          }
        }
      }
      if let Some(notice) = notice_banner() {
        div { class: "px-4 pt-2 shrink-0", "data-testid": "notebook-notice-banner",
          Alert { variant: AlertVariant::Default,
            AlertDescription {
              div { class: "flex items-center justify-between gap-2 text-xs",
                span { "{notice}" }
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  class: "h-6 px-2 text-xs".to_string(),
                  onclick: move |_| notice_banner.set(None),
                  "关闭"
                }
              }
            }
          }
        }
      }

      // ── Main Three-Pane Resizable Workbench ───────────────────────────────
      div { class: "flex-1 min-h-0 p-3",
        if loading_list() && notebooks().is_empty() {
          div { class: "h-full flex items-center justify-center text-sm text-muted-foreground",
            "正在加载笔记本列表…"
          }
        } else {
          ResizablePanelGroup {
            orientation: LayoutOrientation::Horizontal,
            class: "h-full rounded-lg border border-border bg-background overflow-hidden".to_string(),
            "data-testid": "notebook-resizable-group",

            // ══════════════ LEFT PANE: Sources, Chunks & BM25 Search ══════════
            ResizablePanel {
              size: eff_left,
              min_size: 16.0,
              max_size: 50.0,
              collapsed: left_collapsed(),
              class: "flex flex-col h-full bg-card/30".to_string(),
              "data-testid": "notebook-left-pane",
              div { class: "flex-1 overflow-y-auto p-3 space-y-4",
                // Source ingestion card
                Card {
                  CardHeader { class: "p-3 pb-2".to_string(),
                    CardTitle { class: "text-sm".to_string(), "添加资料来源" }
                    CardDescription { class: "text-xs".to_string(),
                      "支持文本、网页/ArXiv/GitHub URL、文档图片 OCR 及录音转写"
                    }
                  }
                  CardContent { class: "p-3 pt-1 space-y-2.5".to_string(),
                    Tabs {
                      value: Some(ingest_tab()),
                      on_value_change: move |v: String| ingest_tab.set(v),
                      TabsList { class: "grid grid-cols-4 h-8".to_string(),
                        TabsTrigger { value: "text".to_string(), class: "text-xs px-1".to_string(), "文本" }
                        TabsTrigger { value: "url".to_string(), class: "text-xs px-1".to_string(), "URL" }
                        TabsTrigger { value: "file".to_string(), class: "text-xs px-1".to_string(), "文档" }
                        TabsTrigger { value: "recording".to_string(), class: "text-xs px-1".to_string(), "录音" }
                      }

                      div { class: "pt-2 space-y-2",
                        Input {
                          value: source_title_draft(),
                          placeholder: "资料标题（可选）".to_string(),
                          class: "h-8 text-xs".to_string(),
                          "data-testid": "source-title-input",
                          on_value_change: move |v: String| source_title_draft.set(v),
                        }

                        TabsContent { value: "text".to_string(), class: "mt-0 space-y-2".to_string(),
                          Textarea {
                            value: source_text_draft(),
                            placeholder: "粘贴研究正文、会议纪要或论文段落…".to_string(),
                            class: "min-h-24 text-xs".to_string(),
                            "data-testid": "source-text-input",
                            on_value_change: move |v: String| source_text_draft.set(v),
                          }
                        }

                        TabsContent { value: "url".to_string(), class: "mt-0 space-y-2".to_string(),
                          Input {
                            value: source_url_draft(),
                            placeholder: "https://arxiv.org/abs/... 或任意网页 URL".to_string(),
                            class: "h-8 text-xs".to_string(),
                            "data-testid": "source-url-input",
                            on_value_change: move |v: String| source_url_draft.set(v),
                          }
                        }

                        TabsContent { value: "file".to_string(), class: "mt-0 space-y-2".to_string(),
                          input {
                            id: "nb-upload-file-input",
                            r#type: "file",
                            accept: ".pdf,.png,.jpg,.jpeg,.webp,.gif,.docx,.xlsx,.pptx,.csv,.tsv,.json,.txt,.md,.html,.srt,.vtt",
                            class: "block w-full text-xs text-muted-foreground file:mr-2 file:rounded file:border-0 file:bg-secondary file:px-2.5 file:py-1 file:text-xs file:font-medium",
                            "data-testid": "source-file-input",
                          }
                        }

                        TabsContent { value: "recording".to_string(), class: "mt-0 space-y-2".to_string(),
                          input {
                            id: "nb-upload-rec-input",
                            r#type: "file",
                            accept: ".mp3,.wav,.m4a,.ogg,.webm",
                            class: "block w-full text-xs text-muted-foreground file:mr-2 file:rounded file:border-0 file:bg-secondary file:px-2.5 file:py-1 file:text-xs file:font-medium",
                            "data-testid": "source-recording-input",
                          }
                          div { class: "grid grid-cols-2 gap-1.5",
                            Input {
                              value: recording_model_draft(),
                              placeholder: "ASR 模型（可选）".to_string(),
                              class: "h-7 text-xs".to_string(),
                              on_value_change: move |v: String| recording_model_draft.set(v),
                            }
                            Input {
                              value: recording_lang_draft(),
                              placeholder: "语言（如 zh）".to_string(),
                              class: "h-7 text-xs".to_string(),
                              on_value_change: move |v: String| recording_lang_draft.set(v),
                            }
                          }
                        }

                        Button {
                          variant: ButtonVariant::Primary,
                          size: ButtonSize::Sm,
                          class: "w-full".to_string(),
                          disabled: ingest_busy() || selected_notebook_id().is_empty(),
                          "data-testid": "ingest-source-submit-btn",
                          onclick: {
                            let tok = api_token.clone();
                            let ten = tenant_id();
                            let nb_id = selected_notebook_id();
                            move |_| {
                              if nb_id.is_empty() {
                                error_banner.set(Some("请先创建或选择一个笔记本。".to_string()));
                                return;
                              }
                              let tab = ingest_tab();
                              let title_opt = source_title_draft().trim().to_string();
                              let text_val = source_text_draft();
                              let url_val = source_url_draft().trim().to_string();
                              let rec_model = recording_model_draft().trim().to_string();
                              let rec_lang = recording_lang_draft().trim().to_string();
                              let tok = tok.clone();
                              let ten = ten.clone();
                              let nb_id = nb_id.clone();

                              ingest_busy.set(true);
                              error_banner.set(None);
                              spawn(async move {
                                let payload_res: Result<serde_json::Value, String> = match tab.as_str() {
                                  "text" => {
                                    if text_val.trim().is_empty() {
                                      Err("粘贴的资料正文不能为空。".to_string())
                                    } else {
                                      let mut obj = serde_json::json!({
                                        "kind": "text",
                                        "text": text_val,
                                      });
                                      if !title_opt.is_empty()
                                        && let Some(m) = obj.as_object_mut()
                                      {
                                        m.insert("title".to_string(), serde_json::json!(title_opt));
                                      }
                                      Ok(obj)
                                    }
                                  }
                                  "url" => {
                                    if !url_val.starts_with("http://") && !url_val.starts_with("https://") {
                                      Err("来源 URL 必须以 http:// 或 https:// 开头。".to_string())
                                    } else {
                                      let mut obj = serde_json::json!({
                                        "kind": "url",
                                        "url": url_val,
                                      });
                                      if !title_opt.is_empty()
                                        && let Some(m) = obj.as_object_mut()
                                      {
                                        m.insert("title".to_string(), serde_json::json!(title_opt));
                                      }
                                      Ok(obj)
                                    }
                                  }
                                  "file" => match read_input_file_bytes("nb-upload-file-input").await {
                                    Ok(Some((fname, bytes))) => {
                                      let mut obj = serde_json::json!({
                                        "kind": "file",
                                        "filename": fname,
                                        "content_base64": encode_base64(&bytes),
                                      });
                                      if !title_opt.is_empty()
                                        && let Some(m) = obj.as_object_mut()
                                      {
                                        m.insert("title".to_string(), serde_json::json!(title_opt));
                                      }
                                      Ok(obj)
                                    }
                                    Ok(None) => Err("请先选择要上传的文档或图像文件。".to_string()),
                                    Err(err) => Err(err),
                                  },
                                  _ => match read_input_file_bytes("nb-upload-rec-input").await {
                                    Ok(Some((fname, bytes))) => {
                                      let mut obj = serde_json::json!({
                                        "kind": "recording",
                                        "filename": fname,
                                        "content_base64": encode_base64(&bytes),
                                      });
                                      if let Some(m) = obj.as_object_mut() {
                                        if !title_opt.is_empty() {
                                          m.insert("title".to_string(), serde_json::json!(title_opt));
                                        }
                                        if !rec_model.is_empty() {
                                          m.insert("model".to_string(), serde_json::json!(rec_model));
                                        }
                                        if !rec_lang.is_empty() {
                                          m.insert("language".to_string(), serde_json::json!(rec_lang));
                                        }
                                      }
                                      Ok(obj)
                                    }
                                    Ok(None) => Err("请先选择要上传的录音文件。".to_string()),
                                    Err(err) => Err(err),
                                  },
                                };

                                match payload_res {
                                  Ok(payload) => match create_notebook_source(&nb_id, &payload, &ten, &tok).await {
                                    Ok(created) => {
                                      let created_title = created.title.clone();
                                      let mut next_sources = sources();
                                      next_sources.push(created);
                                      sources.set(next_sources);
                                      source_title_draft.set(String::new());
                                      match tab.as_str() {
                                        "text" => source_text_draft.set(String::new()),
                                        "url" => source_url_draft.set(String::new()),
                                        "file" => clear_file_input("nb-upload-file-input"),
                                        _ => clear_file_input("nb-upload-rec-input"),
                                      }
                                      notice_banner.set(Some(format!(
                                        "来源「{created_title}」已提交后台分块与索引构建。"
                                      )));
                                    }
                                    Err(err) => error_banner.set(Some(format!("添加来源失败：{err}"))),
                                  },
                                  Err(err) => error_banner.set(Some(err)),
                                }
                                ingest_busy.set(false);
                              });
                            }
                          },
                          if ingest_busy() { "提交入库中…" } else { "添加资料来源" }
                        }
                      }
                    }
                  }
                }

                // Sources list with scope checkboxes
                div { class: "space-y-2",
                  div { class: "flex items-center justify-between gap-2",
                    span { class: "text-xs font-semibold text-foreground", "data-testid": "sources-scope-summary",
                      "资料来源 ({selected_ready_count}/{ready_count} 已勾选)"
                    }
                    if ready_count > 0 {
                      div { class: "flex items-center gap-1",
                        Button {
                          variant: ButtonVariant::Ghost,
                          size: ButtonSize::Sm,
                          class: "h-6 px-1.5 text-[11px]".to_string(),
                          onclick: move |_| excluded_source_ids.set(HashSet::new()),
                          "全选"
                        }
                        Button {
                          variant: ButtonVariant::Ghost,
                          size: ButtonSize::Sm,
                          class: "h-6 px-1.5 text-[11px]".to_string(),
                          onclick: move |_| {
                            let all_ids: HashSet<String> = sources()
                              .into_iter()
                              .filter(|s| s.status == "ready")
                              .map(|s| s.id)
                              .collect();
                            excluded_source_ids.set(all_ids);
                          },
                          "全不选"
                        }
                      }
                    }
                  }

                  if cur_sources.is_empty() {
                    p { class: "text-xs text-muted-foreground rounded border border-dashed border-border p-3 text-center",
                      "暂无资料来源，请在上方添加文本、URL、文档或录音。"
                    }
                  } else {
                    ul { class: "space-y-2", "data-testid": "notebook-sources-list",
                      for src in cur_sources {
                        {
                          let sid = src.id.clone();
                          let is_ready = src.status == "ready";
                          let is_failed = src.status == "failed";
                          let is_checked = is_ready && !excluded_set.contains(&sid);
                          let status_badge = match src.status.as_str() {
                            "ready" => BadgeVariant::Secondary,
                            "failed" => BadgeVariant::Destructive,
                            _ => BadgeVariant::Outline,
                          };
                          let src_for_browse = src.clone();
                          let src_for_edit = src.clone();
                          let src_for_retry = src.clone();
                          let src_for_delete = src.clone();
                          let sid_cb = sid.clone();
                          let tok_browse = api_token.clone();
                          let ten_browse = tenant_id();
                          let nb_browse = selected_notebook_id();
                          let tok_edit = api_token.clone();
                          let ten_edit = tenant_id();
                          let nb_edit = selected_notebook_id();
                          let tok_retry = api_token.clone();
                          let ten_retry = tenant_id();
                          let nb_retry = selected_notebook_id();
                          let tok_del = api_token.clone();
                          let ten_del = tenant_id();
                          let nb_del = selected_notebook_id();

                          rsx! {
                            li {
                              key: "{src.id}",
                              class: "rounded-md border border-border bg-background p-2.5 space-y-1.5",
                              "data-testid": "source-item-{src.id}",
                              div { class: "flex items-start gap-2",
                                div { class: "pt-0.5",
                                  Checkbox {
                                    checked: is_checked,
                                    disabled: !is_ready,
                                    "data-testid": "source-checkbox-{src.id}",
                                    on_checked_change: move |checked: bool| {
                                      let mut next = excluded_source_ids();
                                      if checked {
                                        next.remove(&sid_cb);
                                      } else {
                                        next.insert(sid_cb.clone());
                                      }
                                      excluded_source_ids.set(next);
                                    },
                                  }
                                }
                                div { class: "flex-1 min-w-0 space-y-1",
                                  div { class: "flex items-center justify-between gap-1",
                                    span { class: "text-xs font-medium truncate", title: "{src.title}", "{src.title}" }
                                    Badge { variant: status_badge, "{source_status_label(&src.status)}" }
                                  }
                                  div { class: "flex items-center gap-1.5 text-[11px] text-muted-foreground",
                                    span { "{src.kind}" }
                                    if let Some(uri) = src.uri.as_deref().filter(|u| !u.is_empty()) {
                                      span { class: "truncate", title: "{uri}", "· {uri}" }
                                    }
                                  }
                                  if let Some(err_txt) = src.error.as_deref().filter(|e| !e.is_empty()) {
                                    p { class: "text-[11px] text-destructive break-all", "{err_txt}" }
                                  }
                                }
                              }

                              div { class: "flex flex-wrap items-center gap-1 pl-6",
                                if is_ready {
                                  Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    disabled: browse_source_busy_id().as_deref() == Some(src.id.as_str()),
                                    "data-testid": "source-browse-chunks-{src.id}",
                                    onclick: move |_| {
                                      let s = src_for_browse.clone();
                                      let tok = tok_browse.clone();
                                      let ten = ten_browse.clone();
                                      let nb_id = nb_browse.clone();
                                      browse_source_busy_id.set(Some(s.id.clone()));
                                      spawn(async move {
                                        match get_source_chunks(&nb_id, &s.id, &ten, &tok).await {
                                          Ok(env) => {
                                            if let Some(first) = env.chunks.first().cloned() {
                                              reader_source_id.set(s.id.clone());
                                              reader_chunks.set(env.chunks);
                                              inspected_chunk.set(Some(first));
                                              chunk_dialog_open.set(true);
                                            } else {
                                              notice_banner.set(Some(format!("来源「{}」尚无就绪文本分块。", s.title)));
                                            }
                                          }
                                          Err(err) => error_banner.set(Some(format!("读取来源分块失败：{err}"))),
                                        }
                                        browse_source_busy_id.set(None);
                                      });
                                    },
                                    "阅读分块"
                                  }
                                }
                                Button {
                                  variant: ButtonVariant::Ghost,
                                  size: ButtonSize::Sm,
                                  class: "h-6 px-2 text-[11px]".to_string(),
                                  "data-testid": "source-edit-{src.id}",
                                  onclick: move |_| {
                                    let s = src_for_edit.clone();
                                    let tok = tok_edit.clone();
                                    let ten = ten_edit.clone();
                                    let nb_id = nb_edit.clone();
                                    edit_source_title.set(s.title.clone());
                                    edit_source_text.set(String::new());
                                    initial_edit_source_text.set(String::new());
                                    editing_source.set(Some(s.clone()));
                                    edit_source_open.set(true);

                                    if s.kind == "text" && s.status == "ready" {
                                      edit_source_loading_text.set(true);
                                      spawn(async move {
                                        if let Ok(env) = get_source_chunks(&nb_id, &s.id, &ten, &tok).await {
                                          let hydrated = env
                                            .raw_text
                                            .filter(|t| !t.is_empty())
                                            .unwrap_or_else(|| {
                                              env
                                                .chunks
                                                .iter()
                                                .map(|c| c.content.as_str())
                                                .collect::<Vec<_>>()
                                                .join("\n\n")
                                            });
                                          edit_source_text.set(hydrated.clone());
                                          initial_edit_source_text.set(hydrated);
                                        }
                                        edit_source_loading_text.set(false);
                                      });
                                    }
                                  },
                                  "编辑"
                                }
                                if is_failed {
                                  Button {
                                    variant: ButtonVariant::Outline,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    disabled: retrying_source_id().as_deref() == Some(src.id.as_str()),
                                    "data-testid": "source-retry-{src.id}",
                                    onclick: move |_| {
                                      let s = src_for_retry.clone();
                                      let tok = tok_retry.clone();
                                      let ten = ten_retry.clone();
                                      let nb_id = nb_retry.clone();
                                      retrying_source_id.set(Some(s.id.clone()));
                                      spawn(async move {
                                        match retry_notebook_source(&nb_id, &s.id, &ten, &tok).await {
                                          Ok(updated) => {
                                            let title = updated.title.clone();
                                            let mut list = sources();
                                            for item in &mut list {
                                              if item.id == updated.id {
                                                *item = updated.clone();
                                              }
                                            }
                                            sources.set(list);
                                            notice_banner.set(Some(format!("已重新提交「{title}」抓取与分块。")));
                                          }
                                          Err(err) => error_banner.set(Some(format!("重试入库失败：{err}"))),
                                        }
                                        retrying_source_id.set(None);
                                      });
                                    },
                                    "重试"
                                  }
                                }
                                Button {
                                  variant: ButtonVariant::Ghost,
                                  size: ButtonSize::Sm,
                                  class: "h-6 px-2 text-[11px] text-destructive".to_string(),
                                  disabled: deleting_source_id().as_deref() == Some(src.id.as_str()),
                                  "data-testid": "source-delete-{src.id}",
                                  onclick: move |_| {
                                    let s = src_for_delete.clone();
                                    let tok = tok_del.clone();
                                    let ten = ten_del.clone();
                                    let nb_id = nb_del.clone();
                                    deleting_source_id.set(Some(s.id.clone()));
                                    spawn(async move {
                                      match delete_notebook_source(&nb_id, &s.id, &ten, &tok).await {
                                        Ok(()) => {
                                          let mut list = sources();
                                          list.retain(|item| item.id != s.id);
                                          sources.set(list);
                                          let mut ex = excluded_source_ids();
                                          ex.remove(&s.id);
                                          excluded_source_ids.set(ex);
                                          notice_banner.set(Some(format!("已移除来源「{}」。", s.title)));
                                        }
                                        Err(err) => error_banner.set(Some(format!("移除来源失败：{err}"))),
                                      }
                                      deleting_source_id.set(None);
                                    });
                                  },
                                  "删除"
                                }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }

                Separator {}

                // BM25 Search box
                div { class: "space-y-2",
                  Label { "BM25 原文检索" }
                  div { class: "flex items-center gap-1.5",
                    Input {
                      value: search_query(),
                      placeholder: "检索笔记本原文关键词…".to_string(),
                      class: "h-8 text-xs".to_string(),
                      "data-testid": "notebook-search-input",
                      on_value_change: move |v: String| search_query.set(v),
                    }
                    Button {
                      variant: ButtonVariant::Secondary,
                      size: ButtonSize::Sm,
                      class: "shrink-0 whitespace-nowrap".to_string(),
                      disabled: search_busy() || selected_notebook_id().is_empty(),
                      "data-testid": "notebook-search-btn",
                      onclick: {
                        let tok = api_token.clone();
                        let ten = tenant_id();
                        let nb_id = selected_notebook_id();
                        let scope = scoped_source_ids.clone();
                        move |_| {
                          let q = search_query().trim().to_string();
                          if q.is_empty() {
                            search_hits.set(None);
                            return;
                          }
                          if matches!(&scope, Some(ids) if ids.is_empty()) {
                            error_banner.set(Some("请至少勾选一份「已就绪」资料来源后再检索。".to_string()));
                            return;
                          }
                          let tok = tok.clone();
                          let ten = ten.clone();
                          let nb_id = nb_id.clone();
                          let scope = scope.clone();
                          search_busy.set(true);
                          spawn(async move {
                            match search_notebook(&nb_id, &q, 6, scope.as_deref(), &ten, &tok).await {
                              Ok(hits) => search_hits.set(Some(hits)),
                              Err(err) => error_banner.set(Some(format!("BM25 检索失败：{err}"))),
                            }
                            search_busy.set(false);
                          });
                        }
                      },
                      if search_busy() { "检索中" } else { "检索" }
                    }
                  }

                  if let Some(hits) = search_hits() {
                    if hits.is_empty() {
                      p { class: "text-xs text-muted-foreground", "未命中相关原文分块。" }
                    } else {
                      div { class: "space-y-1.5", "data-testid": "notebook-search-results",
                        for hit in hits {
                          {
                            let cid = hit.chunk_id.clone();
                            let snippet_focus: String = hit
                              .content
                              .split_whitespace()
                              .collect::<Vec<_>>()
                              .join(" ")
                              .chars()
                              .take(140)
                              .collect();
                            let src_label = source_title_by_id
                              .get(&hit.source_id)
                              .cloned()
                              .unwrap_or_else(|| hit.source_id.clone());
                            rsx! {
                              div { key: "{hit.chunk_id}", class: "rounded border border-border bg-background p-2 space-y-1 text-xs",
                                div { class: "flex items-center justify-between gap-1",
                                  span { class: "font-medium truncate", "{src_label}" }
                                  Badge { variant: BadgeVariant::Outline, "{hit.score:.2}" }
                                }
                                p { class: "text-muted-foreground line-clamp-3 whitespace-pre-wrap", "{hit.content}" }
                                div { class: "flex items-center gap-1.5",
                                  Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    onclick: move |_| on_inspect_chunk.call(cid.clone()),
                                    "查看分块"
                                  }
                                  Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    onclick: move |_| {
                                      let kind = expanded_kind();
                                      let mut drafts = param_drafts();
                                      drafts
                                        .entry(kind.clone())
                                        .or_default()
                                        .insert("focus".to_string(), snippet_focus.clone());
                                      param_drafts.set(drafts);
                                      notice_banner.set(Some(format!("已将该段落设为「{kind}」生成器的聚焦主题。")));
                                    },
                                    "设为焦点"
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

            if !left_collapsed() {
              ResizableHandle {
                orientation: LayoutOrientation::Horizontal,
                value: left_pct(),
                min: 16.0,
                max: 50.0,
                step: 2.0,
                on_resize: move |delta: f64| {
                  let (next_left, next_center) = resizable_resize_pair(
                    ResizablePanelState::new(left_pct(), 16.0, 50.0),
                    ResizablePanelState::new(center_pct(), 24.0, 70.0),
                    delta,
                  );
                  left_pct.set(next_left.size);
                  center_pct.set(next_center.size);
                },
              }
            }

            // ══════════════ CENTER PANE: Grounded Ask + AI Studio ═════════════
            ResizablePanel {
              size: eff_center,
              min_size: 24.0,
              max_size: 80.0,
              class: "flex flex-col h-full bg-background".to_string(),
              "data-testid": "notebook-center-pane",
              div { class: "flex-1 overflow-y-auto p-3 space-y-4",
                // ── Grounded Ask Card (Phase 2B) ──
                Card {
                  CardHeader { class: "p-3 pb-2".to_string(),
                    div { class: "flex items-center justify-between gap-2",
                      div {
                        CardTitle { class: "text-sm".to_string(), "源接地多轮问答（Grounded Ask）" }
                        CardDescription { class: "text-xs".to_string(),
                          "自动基于已勾选资料分块作答并附带可点击原文引用，持久化保存至笔记本"
                        }
                      }
                      if !ask_thread().is_empty() {
                        Button {
                          variant: ButtonVariant::Ghost,
                          size: ButtonSize::Sm,
                          disabled: clear_ask_busy(),
                          "data-testid": "clear-ask-thread-btn",
                          onclick: {
                            let tok = api_token.clone();
                            let ten = tenant_id();
                            let nb_id = selected_notebook_id();
                            move |_| {
                              let tok = tok.clone();
                              let ten = ten.clone();
                              let nb_id = nb_id.clone();
                              clear_ask_busy.set(true);
                              spawn(async move {
                                match clear_notebook_messages(&nb_id, &ten, &tok).await {
                                  Ok(()) => {
                                    ask_thread.set(Vec::new());
                                    notice_banner.set(Some("已清空当前笔记本问答记录。".to_string()));
                                  }
                                  Err(err) => error_banner.set(Some(format!("清空对话失败：{err}"))),
                                }
                                clear_ask_busy.set(false);
                              });
                            }
                          },
                          "清空对话"
                        }
                      }
                    }
                  }
                  CardContent { class: "p-3 pt-1 space-y-3".to_string(),
                    if ask_thread().is_empty() {
                      p { class: "text-xs text-muted-foreground rounded border border-dashed border-border p-3 text-center",
                        "向当前笔记本资料发起提问，支持连续追问与原文分块溯源。"
                      }
                    } else {
                      div { class: "space-y-3 max-h-96 overflow-y-auto pr-1", "data-testid": "notebook-ask-thread",
                        for (turn_idx, turn) in ask_thread().into_iter().enumerate() {
                          {
                            let turn_for_save = turn.clone();
                            let answer_for_focus = turn.answer.clone();
                            let titles_for_save = source_title_by_id.clone();
                            let tok_save = api_token.clone();
                            let ten_save = tenant_id();
                            let nb_save = selected_notebook_id();
                            rsx! {
                              div {
                                key: "ask-turn-{turn_idx}",
                                class: "rounded-md border border-border bg-card/60 p-3 space-y-2",
                                "data-testid": "notebook-ask-result",
                                div { class: "flex flex-wrap items-center justify-between gap-2 text-xs",
                                  strong { class: "text-foreground", "#{turn_idx + 1} · Q: {turn.question}" }
                                  span { class: "text-[11px] text-muted-foreground",
                                    "{turn.model} · 检索 {turn.used_chunk_count} 段原文"
                                  }
                                }
                                p { class: "text-xs leading-relaxed whitespace-pre-wrap text-foreground", "{turn.answer}" }
                                if !turn.citations.is_empty() {
                                  NotebookDeliverableCitationBar {
                                    deliverable: NotebookDeliverable {
                                      kind: "ask".to_string(),
                                      title: String::new(),
                                      content: None,
                                      citations: turn.citations.clone(),
                                    },
                                    source_title_by_id: source_title_by_id.clone(),
                                    on_inspect_chunk: on_inspect_chunk,
                                  }
                                }
                                div { class: "flex items-center gap-2 pt-1",
                                  Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    onclick: move |_| {
                                      let compact: String = answer_for_focus
                                        .split_whitespace()
                                        .collect::<Vec<_>>()
                                        .join(" ")
                                        .chars()
                                        .take(140)
                                        .collect();
                                      let kind = expanded_kind();
                                      let mut drafts = param_drafts();
                                      drafts
                                        .entry(kind.clone())
                                        .or_default()
                                        .insert("focus".to_string(), compact);
                                      param_drafts.set(drafts);
                                      notice_banner.set(Some(format!("已将该回答设为「{kind}」生成器的聚焦主题。")));
                                    },
                                    "设为生成器焦点"
                                  }
                                  Button {
                                    variant: ButtonVariant::Ghost,
                                    size: ButtonSize::Sm,
                                    class: "h-6 px-2 text-[11px]".to_string(),
                                    disabled: save_ask_busy(),
                                    "data-testid": "save-ask-as-source-btn",
                                    onclick: move |_| {
                                      let res = turn_for_save.clone();
                                      let titles = titles_for_save.clone();
                                      let tok = tok_save.clone();
                                      let ten = ten_save.clone();
                                      let nb_id = nb_save.clone();
                                      save_ask_busy.set(true);
                                      spawn(async move {
                                        let short_q: String = res.question.trim().chars().take(40).collect();
                                        let payload = serde_json::json!({
                                          "kind": "text",
                                          "title": format!("问答摘录：{short_q}"),
                                          "text": format_ask_result_as_note_text(&res, &titles),
                                        });
                                        match create_notebook_source(&nb_id, &payload, &ten, &tok).await {
                                          Ok(created) => {
                                            let t = created.title.clone();
                                            let mut list = sources();
                                            list.push(created);
                                            sources.set(list);
                                            notice_banner.set(Some(format!("已将回答保存为笔记来源「{t}」。")));
                                          }
                                          Err(err) => error_banner.set(Some(format!("保存问答为笔记失败：{err}"))),
                                        }
                                        save_ask_busy.set(false);
                                      });
                                    },
                                    if save_ask_busy() { "保存中…" } else { "存为笔记来源" }
                                  }
                                }
                              }
                            }
                          }
                        }
                      }
                    }

                    // Ask input row
                    div { class: "flex items-end gap-2",
                      div { class: "flex-1",
                        Textarea {
                          value: ask_draft(),
                          placeholder: "输入问题，基于已勾选资料提问…".to_string(),
                          class: "min-h-16 text-xs".to_string(),
                          "data-testid": "notebook-ask-input",
                          on_value_change: move |v: String| ask_draft.set(v),
                        }
                      }
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        disabled: ask_busy() || selected_notebook_id().is_empty(),
                        "data-testid": "notebook-ask-submit-btn",
                        onclick: {
                          let tok = api_token.clone();
                          let ten = tenant_id();
                          let nb_id = selected_notebook_id();
                          let scope = scoped_source_ids.clone();
                          move |_| {
                            let question = ask_draft().trim().to_string();
                            if question.is_empty() {
                              error_banner.set(Some("请输入要向当前笔记本提问的问题。".to_string()));
                              return;
                            }
                            if selected_ready_count == 0 {
                              error_banner.set(Some("请至少勾选一份「已就绪」资料来源参与问答。".to_string()));
                              return;
                            }
                            let history = build_ask_history_payload(&ask_thread(), 8);
                            let mut body = serde_json::json!({ "question": question });
                            if let Some(m) = body.as_object_mut() {
                              if !history.is_empty() {
                                m.insert("history".to_string(), serde_json::json!(history));
                              }
                              let mdl = model_name().trim().to_string();
                              if !mdl.is_empty() {
                                m.insert("model".to_string(), serde_json::json!(mdl));
                              }
                              if let Some(ids) = scope.clone() {
                                m.insert("source_ids".to_string(), serde_json::json!(ids));
                              }
                            }
                            let tok = tok.clone();
                            let ten = ten.clone();
                            let nb_id = nb_id.clone();
                            ask_busy.set(true);
                            error_banner.set(None);
                            spawn(async move {
                              match ask_notebook(&nb_id, &body, &ten, &tok).await {
                                Ok(parsed) => {
                                  let mut thread = ask_thread();
                                  thread.push(parsed);
                                  ask_thread.set(thread);
                                  ask_draft.set(String::new());
                                }
                                Err(err) => error_banner.set(Some(format!("笔记本问答失败：{err}"))),
                              }
                              ask_busy.set(false);
                            });
                          }
                        },
                        if ask_busy() { "思考中…" } else { "提问" }
                      }
                    }
                  }
                }

                // ── AI Studio Planner Card (Phase 2C) ──
                Card {
                  CardHeader { class: "p-3 pb-2".to_string(),
                    CardTitle { class: "text-sm".to_string(), "AI Studio 智能规划（/plan）" }
                    CardDescription { class: "text-xs".to_string(),
                      "描述内容生产目标，自动推荐最佳多模态交付物组合并一键批量生成"
                    }
                  }
                  CardContent { class: "p-3 pt-1 space-y-2.5".to_string(),
                    div { class: "flex items-center gap-2",
                      div { class: "flex-1",
                        Input {
                          value: plan_goal_draft(),
                          placeholder: "例如：为期末复习生成闪卡、自测题与思维导图".to_string(),
                          class: "h-8 text-xs".to_string(),
                          "data-testid": "studio-plan-goal-input",
                          on_value_change: move |v: String| plan_goal_draft.set(v),
                        }
                      }
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        disabled: plan_busy() || selected_notebook_id().is_empty(),
                        "data-testid": "studio-plan-submit-btn",
                        onclick: {
                          let tok = api_token.clone();
                          let ten = tenant_id();
                          let nb_id = selected_notebook_id();
                          let scope = scoped_source_ids.clone();
                          move |_| {
                            let goal = plan_goal_draft().trim().to_string();
                            if goal.is_empty() {
                              error_banner.set(Some("请输入本次内容生产的规划目标。".to_string()));
                              return;
                            }
                            if selected_ready_count == 0 {
                              error_banner.set(Some("请至少勾选一份「已就绪」资料来源后再执行 AI 规划。".to_string()));
                              return;
                            }
                            let mut body = serde_json::json!({ "goal": goal, "max_items": 4 });
                            if let Some(m) = body.as_object_mut() {
                              let mdl = model_name().trim().to_string();
                              if !mdl.is_empty() {
                                m.insert("model".to_string(), serde_json::json!(mdl));
                              }
                              if let Some(ids) = scope.clone() {
                                m.insert("source_ids".to_string(), serde_json::json!(ids));
                              }
                            }
                            let tok = tok.clone();
                            let ten = ten.clone();
                            let nb_id = nb_id.clone();
                            plan_busy.set(true);
                            error_banner.set(None);
                            spawn(async move {
                              match plan_notebook(&nb_id, &body, &ten, &tok).await {
                                Ok(plan) => studio_plan.set(Some(plan)),
                                Err(err) => error_banner.set(Some(format!("AI Studio 规划失败：{err}"))),
                              }
                              plan_busy.set(false);
                            });
                          }
                        },
                        if plan_busy() { "规划中…" } else { "生成规划" }
                      }
                    }

                    if let Some(plan) = studio_plan() {
                      div { class: "rounded-md border border-border bg-muted/30 p-2.5 space-y-2 text-xs", "data-testid": "studio-plan-result",
                        div { class: "flex items-center justify-between gap-2",
                          span { class: "font-semibold", "推荐组合（{plan.items.len()} 项 · {plan.model}）" }
                          Button {
                            variant: ButtonVariant::Primary,
                            size: ButtonSize::Sm,
                            disabled: batch_busy() || plan.items.is_empty(),
                            "data-testid": "studio-plan-execute-btn",
                            onclick: {
                              let tok = api_token.clone();
                              let ten = tenant_id();
                              let nb_id = selected_notebook_id();
                              let scope = scoped_source_ids.clone();
                              let plan_items = plan.items.clone();
                              let watch = watch_run.clone();
                              move |_| {
                                let cost_opt = match parse_cost_limit_usd_draft(&cost_limit_draft()) {
                                  Ok(v) => v,
                                  Err(err) => {
                                    error_banner.set(Some(err));
                                    return;
                                  }
                                };
                                let items: Vec<serde_json::Value> = plan_items
                                  .iter()
                                  .map(|it| serde_json::json!({ "kind": it.kind, "params": it.params }))
                                  .collect();
                                let mut body = serde_json::json!({ "items": items });
                                if let Some(m) = body.as_object_mut() {
                                  let mdl = model_name().trim().to_string();
                                  if !mdl.is_empty() {
                                    m.insert("model".to_string(), serde_json::json!(mdl));
                                  }
                                  if let Some(ids) = scope.clone() {
                                    m.insert("source_ids".to_string(), serde_json::json!(ids));
                                  }
                                  if let Some(c) = cost_opt {
                                    m.insert("cost_limit_usd".to_string(), serde_json::json!(c));
                                  }
                                }
                                let tok = tok.clone();
                                let ten = ten.clone();
                                let nb_id = nb_id.clone();
                                let mut watch = watch.clone();
                                batch_busy.set(true);
                                spawn(async move {
                                  match generate_notebook_batch(&nb_id, &body, &ten, &tok).await {
                                    Ok(batch) => {
                                      let kinds: Vec<String> = batch.runs.iter().map(|r| r.kind.clone()).collect();
                                      for r in batch.runs {
                                        watch(
                                          nb_id.clone(),
                                          r.run_id,
                                          r.kind,
                                          r.status.unwrap_or_else(|| "running".to_string()),
                                        );
                                      }
                                      notice_banner.set(Some(format!(
                                        "已按规划批量提交 {} 项生成任务（{}）。",
                                        kinds.len(),
                                        kinds.join("、")
                                      )));
                                    }
                                    Err(err) => error_banner.set(Some(format!("批量执行规划失败：{err}"))),
                                  }
                                  batch_busy.set(false);
                                });
                              }
                            },
                            "一键批量执行"
                          }
                        }
                        if !plan.rationale.is_empty() {
                          p { class: "text-muted-foreground", "{plan.rationale}" }
                        }
                        ul { class: "space-y-1",
                          for item in &plan.items {
                            li { class: "flex items-center gap-2",
                              Badge { variant: BadgeVariant::Outline, "{item.kind}" }
                              if let Some(reason) = item.reason.as_deref() {
                                span { class: "text-muted-foreground", "{reason}" }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }

                // ── Live Generator Runs Queue ──
                if !tracked_runs().is_empty() {
                  div { class: "rounded-md border border-border bg-card/50 p-3 space-y-1.5", "data-testid": "tracked-generator-runs",
                    div { class: "text-xs font-semibold", "正在追踪的生成任务" }
                    for run in tracked_runs() {
                      div { key: "{run.run_id}", class: "flex flex-wrap items-center justify-between gap-2 text-xs rounded border border-border/60 bg-background px-2.5 py-1.5",
                        div { class: "flex items-center gap-2",
                          Badge { variant: BadgeVariant::Secondary, "{run.kind}" }
                          span { class: "font-mono text-[11px] text-muted-foreground", "{run.run_id}" }
                        }
                        div { class: "flex items-center gap-2 text-[11px]",
                          span { "节点 {run.completed_nodes}" }
                          if run.total_cost_usd > 0.0 {
                            span { "${run.total_cost_usd:.4}" }
                          }
                          Badge {
                            variant: if run.status == "failed" { BadgeVariant::Destructive } else { BadgeVariant::Outline },
                            "{run.status}"
                          }
                        }
                      }
                    }
                  }
                }

                // ── 12 Studio Generators Catalogue ──
                div { class: "space-y-2",
                  div { class: "flex flex-wrap items-center justify-between gap-2",
                    span { class: "text-xs font-semibold text-foreground",
                      "Studio 交付物生成器（{STUDIO_GENERATORS.len()} 种）"
                    }
                    if !batch_selected_kinds().is_empty() {
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        disabled: batch_busy(),
                        "data-testid": "studio-batch-launch-btn",
                        onclick: {
                          let tok = api_token.clone();
                          let ten = tenant_id();
                          let nb_id = selected_notebook_id();
                          let scope = scoped_source_ids.clone();
                          let watch = watch_run.clone();
                          move |_| {
                            if selected_ready_count == 0 {
                              error_banner.set(Some("请至少勾选一份「已就绪」资料来源参与批量生成。".to_string()));
                              return;
                            }
                            let cost_opt = match parse_cost_limit_usd_draft(&cost_limit_draft()) {
                              Ok(v) => v,
                              Err(err) => {
                                error_banner.set(Some(err));
                                return;
                              }
                            };
                            let drafts = param_drafts();
                            let mut items = Vec::new();
                            for kind in batch_selected_kinds() {
                              if let Some(spec) = STUDIO_GENERATORS.iter().find(|s| s.kind == kind) {
                                match build_generator_params(spec, drafts.get(spec.kind)) {
                                  Ok(params) => items.push(serde_json::json!({ "kind": spec.kind, "params": params })),
                                  Err(err) => {
                                    error_banner.set(Some(err));
                                    return;
                                  }
                                }
                              }
                            }
                            let mut body = serde_json::json!({ "items": items });
                            if let Some(m) = body.as_object_mut() {
                              let mdl = model_name().trim().to_string();
                              if !mdl.is_empty() {
                                m.insert("model".to_string(), serde_json::json!(mdl));
                              }
                              if let Some(ids) = scope.clone() {
                                m.insert("source_ids".to_string(), serde_json::json!(ids));
                              }
                              if let Some(c) = cost_opt {
                                m.insert("cost_limit_usd".to_string(), serde_json::json!(c));
                              }
                            }
                            let tok = tok.clone();
                            let ten = ten.clone();
                            let nb_id = nb_id.clone();
                            let mut watch = watch.clone();
                            batch_busy.set(true);
                            spawn(async move {
                              match generate_notebook_batch(&nb_id, &body, &ten, &tok).await {
                                Ok(batch) => {
                                  let kinds: Vec<String> = batch.runs.iter().map(|r| r.kind.clone()).collect();
                                  for r in batch.runs {
                                    watch(
                                      nb_id.clone(),
                                      r.run_id,
                                      r.kind,
                                      r.status.unwrap_or_else(|| "running".to_string()),
                                    );
                                  }
                                  notice_banner.set(Some(format!(
                                    "已批量提交 {} 项生成任务（{}）。",
                                    kinds.len(),
                                    kinds.join("、")
                                  )));
                                }
                                Err(err) => error_banner.set(Some(format!("批量生成失败：{err}"))),
                              }
                              batch_busy.set(false);
                            });
                          }
                        },
                        if batch_busy() {
                          "批量提交中…"
                        } else {
                          "批量生成 ({batch_selected_kinds().len()})"
                        }
                      }
                    }
                  }

                  div { class: "space-y-2", "data-testid": "studio-generators-list",
                    for spec in STUDIO_GENERATORS {
                      {
                        let kind = spec.kind;
                        let is_expanded = expanded_kind() == kind;
                        let is_batch_checked = batch_selected_kinds().iter().any(|k| k == kind);
                        let is_busy = generate_busy_kind().as_deref() == Some(kind);
                        let spec_ref: &'static StudioGeneratorSpec = spec;
                        let tok_gen = api_token.clone();
                        let ten_gen = tenant_id();
                        let nb_gen = selected_notebook_id();
                        let scope_gen = scoped_source_ids.clone();
                        let watch_gen = watch_run.clone();

                        rsx! {
                          div {
                            key: "{kind}",
                            class: if is_expanded {
                              "rounded-md border border-primary/60 bg-card p-3 space-y-2.5"
                            } else {
                              "rounded-md border border-border bg-card/40 p-2.5 space-y-2"
                            },
                            "data-testid": "generator-card-{kind}",
                            div { class: "flex items-start justify-between gap-2",
                              div { class: "flex items-start gap-2 min-w-0",
                                div { class: "pt-0.5",
                                  Checkbox {
                                    checked: is_batch_checked,
                                    "data-testid": "generator-batch-check-{kind}",
                                    on_checked_change: move |checked: bool| {
                                      let mut cur = batch_selected_kinds();
                                      if checked {
                                        if !cur.iter().any(|k| k == kind) {
                                          if cur.len() >= MAX_STUDIO_BATCH_ITEMS {
                                            error_banner.set(Some(format!(
                                              "单次批量生成最多选择 {MAX_STUDIO_BATCH_ITEMS} 种生成器。"
                                            )));
                                            return;
                                          }
                                          cur.push(kind.to_string());
                                        }
                                      } else {
                                        cur.retain(|k| k != kind);
                                      }
                                      batch_selected_kinds.set(cur);
                                    },
                                  }
                                }
                                button {
                                  r#type: "button",
                                  class: "text-left space-y-0.5 cursor-pointer",
                                  onclick: move |_| expanded_kind.set(kind.to_string()),
                                  div { class: "flex flex-wrap items-center gap-1.5",
                                    span { class: "text-xs font-semibold text-foreground", "{spec.title}" }
                                    Badge { variant: BadgeVariant::Outline, "{spec.deliverable_badge}" }
                                  }
                                  p { class: "text-[11px] text-muted-foreground", "{spec.summary}" }
                                }
                              }
                              Button {
                                variant: if is_expanded { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                                size: ButtonSize::Sm,
                                class: "h-7 px-2 text-xs shrink-0".to_string(),
                                onclick: move |_| expanded_kind.set(kind.to_string()),
                                if is_expanded { "配置中" } else { "展开" }
                              }
                            }

                            if is_expanded {
                              div { class: "pt-2 border-t border-border/60 space-y-2",
                                for field in spec.fields {
                                  {
                                    let fkey = field.key;
                                    let cur_val = param_drafts()
                                      .get(kind)
                                      .and_then(|m| m.get(fkey))
                                      .cloned()
                                      .unwrap_or_else(|| field.default_str.to_string());
                                    match field.kind {
                                      ParamFieldKind::Checkbox => {
                                        let checked = cur_val == "true";
                                        rsx! {
                                          label { key: "{kind}-{fkey}", class: "flex items-center gap-2 text-xs cursor-pointer",
                                            Checkbox {
                                              checked,
                                              on_checked_change: move |next: bool| {
                                                let mut drafts = param_drafts();
                                                drafts
                                                  .entry(kind.to_string())
                                                  .or_default()
                                                  .insert(fkey.to_string(), if next { "true" } else { "false" }.to_string());
                                                param_drafts.set(drafts);
                                              },
                                            }
                                            span { "{field.label}" }
                                          }
                                        }
                                      }
                                      ParamFieldKind::Number { min, max } => rsx! {
                                        div { key: "{kind}-{fkey}", class: "space-y-1",
                                          Label { class: "text-[11px]".to_string(), "{field.label}" }
                                          Input {
                                            r#type: "number",
                                            min: "{min}",
                                            max: "{max}",
                                            value: cur_val,
                                            class: "h-8 text-xs".to_string(),
                                            on_value_change: move |v: String| {
                                              let mut drafts = param_drafts();
                                              drafts
                                                .entry(kind.to_string())
                                                .or_default()
                                                .insert(fkey.to_string(), v);
                                              param_drafts.set(drafts);
                                            },
                                          }
                                        }
                                      },
                                      ParamFieldKind::Text => rsx! {
                                        div { key: "{kind}-{fkey}", class: "space-y-1",
                                          Label { class: "text-[11px]".to_string(), "{field.label}" }
                                          Input {
                                            value: cur_val,
                                            class: "h-8 text-xs".to_string(),
                                            "data-testid": "generator-{kind}-field-{fkey}",
                                            on_value_change: move |v: String| {
                                              let mut drafts = param_drafts();
                                              drafts
                                                .entry(kind.to_string())
                                                .or_default()
                                                .insert(fkey.to_string(), v);
                                              param_drafts.set(drafts);
                                            },
                                          }
                                        }
                                      },
                                    }
                                  }
                                }

                                div { class: "pt-1 flex justify-end",
                                  Button {
                                    variant: ButtonVariant::Primary,
                                    size: ButtonSize::Sm,
                                    disabled: is_busy || selected_notebook_id().is_empty(),
                                    "data-testid": "generator-launch-{kind}",
                                    onclick: move |_| {
                                      if ready_count == 0 {
                                        error_banner.set(Some("当前笔记本尚无「已就绪」资料分块。".to_string()));
                                        return;
                                      }
                                      if selected_ready_count == 0 {
                                        error_banner.set(Some("请至少勾选一份「已就绪」资料来源参与生成。".to_string()));
                                        return;
                                      }
                                      let params = match build_generator_params(spec_ref, param_drafts().get(kind)) {
                                        Ok(p) => p,
                                        Err(err) => {
                                          error_banner.set(Some(err));
                                          return;
                                        }
                                      };
                                      let cost_opt = match parse_cost_limit_usd_draft(&cost_limit_draft()) {
                                        Ok(v) => v,
                                        Err(err) => {
                                          error_banner.set(Some(err));
                                          return;
                                        }
                                      };
                                      let mut body = serde_json::json!({
                                        "kind": kind,
                                        "params": params,
                                      });
                                      if let Some(m) = body.as_object_mut() {
                                        let mdl = model_name().trim().to_string();
                                        if !mdl.is_empty() {
                                          m.insert("model".to_string(), serde_json::json!(mdl));
                                        }
                                        if let Some(ids) = scope_gen.clone() {
                                          m.insert("source_ids".to_string(), serde_json::json!(ids));
                                        }
                                        if let Some(c) = cost_opt {
                                          m.insert("cost_limit_usd".to_string(), serde_json::json!(c));
                                        }
                                      }
                                      let tok = tok_gen.clone();
                                      let ten = ten_gen.clone();
                                      let nb_id = nb_gen.clone();
                                      let mut watch = watch_gen.clone();
                                      generate_busy_kind.set(Some(kind.to_string()));
                                      error_banner.set(None);
                                      spawn(async move {
                                        match generate_notebook(&nb_id, &body, &ten, &tok).await {
                                          Ok(started) => {
                                            watch(
                                              nb_id.clone(),
                                              started.run_id,
                                              spec_ref.kind.to_string(),
                                              started.status.unwrap_or_else(|| "running".to_string()),
                                            );
                                          }
                                          Err(err) => error_banner.set(Some(format!("启动生成器失败：{err}"))),
                                        }
                                        generate_busy_kind.set(None);
                                      });
                                    },
                                    if is_busy { "启动中…" } else { "立即生成" }
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

            if !right_collapsed() {
              ResizableHandle {
                orientation: LayoutOrientation::Horizontal,
                value: eff_left + eff_center,
                min: 40.0,
                max: 84.0,
                step: 2.0,
                on_resize: move |delta: f64| {
                  let (next_center, next_right) = resizable_resize_pair(
                    ResizablePanelState::new(center_pct(), 24.0, 70.0),
                    ResizablePanelState::new(right_pct(), 18.0, 56.0),
                    delta,
                  );
                  center_pct.set(next_center.size);
                  right_pct.set(next_right.size);
                },
              }
            }

            // ══════════════ RIGHT PANE: Deliverables & Artifacts Gallery ══════
            ResizablePanel {
              size: eff_right,
              min_size: 18.0,
              max_size: 56.0,
              collapsed: right_collapsed(),
              class: "flex flex-col h-full bg-card/30".to_string(),
              "data-testid": "notebook-right-pane",
              div { class: "border-b border-border px-3 py-2 flex flex-wrap items-center justify-between gap-2 shrink-0",
                div { class: "flex items-center gap-2",
                  span { class: "text-xs font-semibold", "成品画廊" }
                  Badge { variant: BadgeVariant::Secondary, "{all_artifacts.len()}" }
                }
                div { class: "flex items-center gap-1.5",
                  div { class: "w-32",
                    NativeSelect {
                      value: artifact_filter(),
                      class: "h-8 py-1 text-xs".to_string(),
                      "data-testid": "artifact-filter-select",
                      on_value_change: move |v: String| artifact_filter.set(v),
                      option { value: "all", "全部类型" }
                      for k in artifact_kinds {
                        option { key: "{k}", value: "{k}", "{k}" }
                      }
                    }
                  }
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2 text-xs".to_string(),
                    disabled: selected_notebook_id().is_empty(),
                    onclick: {
                      let tok = api_token.clone();
                      let ten = tenant_id();
                      let nb_id = selected_notebook_id();
                      move |_| {
                        let tok = tok.clone();
                        let ten = ten.clone();
                        let nb_id = nb_id.clone();
                        spawn(async move {
                          match list_notebook_artifacts(&nb_id, &ten, &tok).await {
                            Ok(list) => artifacts.set(list),
                            Err(err) => error_banner.set(Some(format!("刷新画廊失败：{err}"))),
                          }
                        });
                      }
                    },
                    "刷新"
                  }
                }
              }

              div { class: "flex-1 overflow-y-auto p-3 space-y-3", "data-testid": "notebook-artifacts-gallery",
                if filtered_artifacts.is_empty() {
                  p { class: "text-xs text-muted-foreground rounded border border-dashed border-border p-4 text-center",
                    "暂无已挂载成品。在中栏选择任意 Studio 生成器或执行 AI 规划后，交付物将自动出现在此处。"
                  }
                } else {
                  for art in filtered_artifacts {
                    {
                      let media_item = notebook_artifact_to_media_item(&art);
                      let header_only = art
                        .deliverable
                        .as_ref()
                        .is_some_and(has_exclusive_deliverable_view);
                      let art_for_del = art.clone();
                      let tok_del = api_token.clone();
                      let ten_del = tenant_id();
                      let nb_del = selected_notebook_id();
                      rsx! {
                        div {
                          key: "{art.id}",
                          class: "rounded-lg border border-border bg-card p-3 space-y-2.5",
                          "data-testid": "artifact-card-{art.id}",
                          div { class: "flex items-center justify-between gap-2",
                            div { class: "flex items-center gap-1.5 min-w-0",
                              Badge { variant: BadgeVariant::Outline, "{art.kind}" }
                              span { class: "text-xs font-medium truncate", "{art.name}" }
                            }
                            Button {
                              variant: ButtonVariant::Ghost,
                              size: ButtonSize::Sm,
                              class: "h-6 px-2 text-[11px] text-destructive shrink-0".to_string(),
                              disabled: deleting_artifact_id().as_deref() == Some(art.id.as_str()),
                              "data-testid": "artifact-delete-{art.id}",
                              onclick: move |_| {
                                let a = art_for_del.clone();
                                let tok = tok_del.clone();
                                let ten = ten_del.clone();
                                let nb_id = nb_del.clone();
                                deleting_artifact_id.set(Some(a.id.clone()));
                                spawn(async move {
                                  match delete_notebook_artifact(&nb_id, &a.id, &ten, &tok).await {
                                    Ok(()) => {
                                      let mut list = artifacts();
                                      list.retain(|it| it.id != a.id);
                                      artifacts.set(list);
                                      notice_banner.set(Some(format!("已从笔记本移除成品「{}」。", a.name)));
                                    }
                                    Err(err) => error_banner.set(Some(format!("移除成品失败：{err}"))),
                                  }
                                  deleting_artifact_id.set(None);
                                });
                              },
                              "移除"
                            }
                          }

                          if let Some(deliv) = art.deliverable.clone() {
                            NotebookDeliverableCitationBar {
                              deliverable: deliv.clone(),
                              source_title_by_id: source_title_by_id.clone(),
                              on_inspect_chunk: on_inspect_chunk,
                            }
                            NotebookDeliverableViewer {
                              deliverable: deliv,
                              on_inspect_chunk: on_inspect_chunk,
                            }
                          }

                          AuthorizedMedia {
                            item: media_item,
                            api_token: api_token.clone(),
                            tenant_id: tenant_id(),
                            header_only,
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

      // ── Dialog 1: Chunk Reader & Provenance Inspector ─────────────────────
      Dialog {
        open: Some(chunk_dialog_open()),
        on_open_change: move |next| chunk_dialog_open.set(next),
        DialogOverlay {}
        DialogContent { class: "max-w-2xl max-h-[82vh] flex flex-col gap-3".to_string(), "data-testid": "chunk-reader-dialog",
          DialogTitle { "{chunk_dialog_title}" }
          DialogDescription {
            if let Some(chunk) = inspected_chunk() {
              "chunk_id: {chunk.chunk_id} · ordinal #{chunk.ordinal}"
            }
          }
          if let Some(idx) = reader_idx {
            div { class: "flex items-center justify-between gap-2 text-xs",
              span { class: "text-muted-foreground", "第 {idx + 1} / {reader_chunks().len()} 段" }
              div { class: "flex items-center gap-1.5",
                Button {
                  variant: ButtonVariant::Outline,
                  size: ButtonSize::Sm,
                  disabled: idx == 0,
                  "data-testid": "chunk-prev-btn",
                  onclick: move |_| {
                    if idx > 0
                      && let Some(prev) = reader_chunks().get(idx - 1).cloned()
                    {
                      inspected_chunk.set(Some(prev));
                    }
                  },
                  "← 上一段"
                }
                Button {
                  variant: ButtonVariant::Outline,
                  size: ButtonSize::Sm,
                  disabled: idx + 1 >= reader_chunks().len(),
                  "data-testid": "chunk-next-btn",
                  onclick: move |_| {
                    if let Some(next) = reader_chunks().get(idx + 1).cloned() {
                      inspected_chunk.set(Some(next));
                    }
                  },
                  "下一段 →"
                }
              }
            }
          }
          if let Some(chunk) = inspected_chunk() {
            pre { class: "flex-1 overflow-y-auto rounded-md border border-border bg-muted/40 p-3 text-xs font-mono whitespace-pre-wrap break-words leading-relaxed", "data-testid": "chunk-reader-content",
              "{chunk.content}"
            }
          }
          div { class: "flex justify-end",
            DialogClose { "关闭" }
          }
        }
      }

      // ── Dialog 2: Edit Source (Rename + Lossless raw_text Note Re-chunk) ──
      Dialog {
        open: Some(edit_source_open()),
        on_open_change: move |next| edit_source_open.set(next),
        DialogOverlay {}
        DialogContent { class: "max-w-xl space-y-3".to_string(), "data-testid": "edit-source-dialog",
          DialogTitle { "编辑资料来源" }
          DialogDescription {
            "修改标题；对于文本笔记来源，可无损编辑原文并即时重建分块与 BM25 索引。"
          }
          div { class: "space-y-2.5",
            div { class: "space-y-1",
              Label { "来源标题" }
              Input {
                value: edit_source_title(),
                class: "h-9 text-xs".to_string(),
                "data-testid": "edit-source-title-input",
                on_value_change: move |v: String| edit_source_title.set(v),
              }
            }
            if editing_source().as_ref().is_some_and(|s| s.kind == "text") {
              div { class: "space-y-1",
                Label { "文本笔记正文" }
                if edit_source_loading_text() {
                  p { class: "text-xs text-muted-foreground", "正在加载无损原文…" }
                } else {
                  Textarea {
                    value: edit_source_text(),
                    class: "min-h-48 text-xs font-mono".to_string(),
                    "data-testid": "edit-source-text-input",
                    on_value_change: move |v: String| edit_source_text.set(v),
                  }
                }
              }
            }
          }
          div { class: "flex justify-end gap-2",
            DialogClose { "取消" }
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              disabled: edit_source_busy(),
              "data-testid": "edit-source-save-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                let nb_id = selected_notebook_id();
                move |_| {
                  let Some(src) = editing_source() else {
                    return;
                  };
                  let title = edit_source_title().trim().to_string();
                  if title.is_empty() {
                    error_banner.set(Some("来源标题不能为空。".to_string()));
                    return;
                  }
                  let mut payload = serde_json::json!({ "title": title });
                  let mut text_changed = false;
                  if src.kind == "text" && !edit_source_loading_text() {
                    let next_text = edit_source_text();
                    if next_text.trim().is_empty() {
                      error_banner.set(Some("文本笔记正文不能为空。".to_string()));
                      return;
                    }
                    if next_text != initial_edit_source_text()
                      && let Some(m) = payload.as_object_mut()
                    {
                      m.insert("text".to_string(), serde_json::json!(next_text));
                      text_changed = true;
                    }
                  }
                  let tok = tok.clone();
                  let ten = ten.clone();
                  let nb_id = nb_id.clone();
                  edit_source_busy.set(true);
                  spawn(async move {
                    match patch_notebook_source(&nb_id, &src.id, &payload, &ten, &tok).await {
                      Ok(updated) => {
                        let utitle = updated.title.clone();
                        let mut list = sources();
                        for item in &mut list {
                          if item.id == updated.id {
                            *item = updated.clone();
                          }
                        }
                        sources.set(list);
                        if text_changed {
                          search_hits.set(None);
                        }
                        edit_source_open.set(false);
                        editing_source.set(None);
                        notice_banner.set(Some(if text_changed {
                          format!("已更新笔记「{utitle}」并即时重建分块与 BM25 索引。")
                        } else {
                          format!("已将来源重命名为「{utitle}」。")
                        }));
                      }
                      Err(err) => error_banner.set(Some(format!("更新来源失败：{err}"))),
                    }
                    edit_source_busy.set(false);
                  });
                }
              },
              if edit_source_busy() { "保存中…" } else { "保存更改" }
            }
          }
        }
      }

      // ── Dialog 3: Delete Notebook Confirmation ────────────────────────────
      Dialog {
        open: Some(delete_dialog_open()),
        on_open_change: move |next| delete_dialog_open.set(next),
        DialogOverlay {}
        DialogContent { class: "max-w-md space-y-3".to_string(), "data-testid": "delete-notebook-dialog",
          DialogTitle { "确认删除笔记本" }
          DialogDescription {
            "将永久删除笔记本「{active_nb_title}」及其关联资料分块与索引，此操作不可撤销。"
          }
          div { class: "flex justify-end gap-2",
            DialogClose { "取消" }
            Button {
              variant: ButtonVariant::Destructive,
              size: ButtonSize::Sm,
              disabled: delete_nb_busy(),
              "data-testid": "confirm-delete-notebook-btn",
              onclick: {
                let tok = api_token.clone();
                let ten = tenant_id();
                let nb_id = selected_notebook_id();
                let deleted_title = active_nb_title.clone();
                move |_| {
                  let tok = tok.clone();
                  let ten = ten.clone();
                  let nb_id = nb_id.clone();
                  let title = deleted_title.clone();
                  delete_nb_busy.set(true);
                  spawn(async move {
                    match delete_notebook(&nb_id, &ten, &tok).await {
                      Ok(()) => {
                        delete_dialog_open.set(false);
                        let mut list = notebooks();
                        list.retain(|n| n.id != nb_id);
                        let next_id = list.first().map(|n| n.id.clone()).unwrap_or_default();
                        notebooks.set(list);
                        selected_notebook_id.set(next_id.clone());
                        if next_id.is_empty() {
                          on_navigate.call("/ui/notebooks".to_string());
                        } else {
                          on_navigate.call(format!("/ui/notebooks/{}", encode_uri_component(&next_id)));
                        }
                        notice_banner.set(Some(format!("已删除笔记本「{title}」。")));
                      }
                      Err(err) => error_banner.set(Some(format!("删除笔记本失败：{err}"))),
                    }
                    delete_nb_busy.set(false);
                  });
                }
              },
              if delete_nb_busy() { "删除中…" } else { "确认删除" }
            }
          }
        }
      }
    }
  }
}
