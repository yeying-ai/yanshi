//! Workflow Studio dual-posture workbench (`/ui-dx/studio` Debug Mode &
//! `/ui-dx/app` Release Mode) powered by pure-Rust YAML/DAG engine and
//! `dioxus-shadcn 0.8`.

use std::collections::HashMap;

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Checkbox, Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList,
  Dialog, DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogTitle, Input,
  LayoutOrientation, NativeSelect, NativeSelectOption, NativeSelectSize, Progress,
  ResizableHandle, ResizablePanel, ResizablePanelGroup, ResizablePanelState, Separator, Sheet,
  SheetClose, SheetContent, SheetDescription, SheetHeader, SheetOverlay, SheetSide, SheetSize,
  SheetTitle, Textarea, resizable_resize_pair,
};

use crate::api::{
  ModelConfigItem, PendingApproval, SseAbortHandle, clear_file_input,
  download_authenticated_resource, encode_uri_component, read_input_file_bytes, read_storage,
  sleep_ms, stream_sse, trigger_text_download, write_storage,
};
use crate::dag_canvas::WorkflowGraphCanvas;
use crate::deliverable_viewer::AuthorizedMedia;
use crate::node_inspector::NodeIoInspector;
use crate::notebook_api::{Notebook, encode_base64, list_notebooks};
use crate::studio_api::{
  ArtifactSummary, DebugResumeOverrides, FlowDebugPause, LaunchStudioRunRequest,
  PendingRunQuestion, ResumePlan, RunRecord, StreamedEvent, answer_run_question,
  attach_run_to_notebook, cancel_run, collect_run_deliverables, decide_run_approval,
  extract_final_text_summary, fetch_preferences, format_spend_usd, get_debug_status, get_resume_plan,
  get_run, get_run_artifacts, get_run_events_history, inspect_workflow_server,
  is_terminal_run_record, launch_studio_run, list_run_approvals, list_run_questions, list_runs,
  put_preferences, reduce_workflow_events, resume_run, send_debug_command,
};
use crate::studio_yaml::{
  InspectedInputKind, NodeUpdatePatch, PRESET_CATEGORIES, PersonalWorkflowTemplate, WorkflowPreset,
  add_node_to_yaml, build_personal_templates_preference_patch, build_yaml_download_filename,
  built_in_workflow_presets, coerce_inputs_payload, connect_nodes_in_yaml, default_input_string,
  delete_node_from_yaml, disconnect_edge_in_yaml, inspect_yaml_locally,
  remove_personal_template_patch, restore_personal_templates_from_preferences, update_node_in_yaml,
};
use crate::yaml_editor::YamlCodeEditor;

const STORAGE_PRESET_KEY: &str = "yanshi.ui.studioPreset";
const STORAGE_TENANT_KEY: &str = "yanshi.ui.tenant";
const STORAGE_MODEL_OVERRIDE_KEY: &str = "yanshi.ui.studioModelOverride";
const STORAGE_PERSONAL_TPL_KEY: &str = "yanshi.ui.studioPersonalTemplates";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioPosture {
  Debug,
  Release,
}

impl StudioPosture {
  pub const fn as_str(self) -> &'static str {
    match self {
      Self::Debug => "debug",
      Self::Release => "release",
    }
  }
}

fn short_id(id: &str) -> String {
  id.chars().take(8).collect()
}

fn guess_data_uri_mime(filename: &str) -> &'static str {
  let lower = filename.to_ascii_lowercase();
  if lower.ends_with(".pdf") {
    "application/pdf"
  } else if lower.ends_with(".mp4") {
    "video/mp4"
  } else if lower.ends_with(".webm") {
    "video/webm"
  } else if lower.ends_with(".mp3") {
    "audio/mpeg"
  } else if lower.ends_with(".wav") {
    "audio/wav"
  } else if lower.ends_with(".m4a") {
    "audio/mp4"
  } else if lower.ends_with(".srt") || lower.ends_with(".vtt") {
    "text/plain"
  } else {
    "application/octet-stream"
  }
}

#[component]
pub fn WorkflowStudioPage(
  initial_mode: StudioPosture,
  #[props(default)] initial_preset: Option<String>,
  #[props(default)] initial_run_id: Option<String>,
  api_token: String,
  on_token_change: EventHandler<String>,
  #[props(default)] models: Vec<ModelConfigItem>,
  on_navigate: EventHandler<String>,
) -> Element {
  let built_ins = use_hook(built_in_workflow_presets);
  let default_preset_id = {
    let from_param = initial_preset
      .as_deref()
      .and_then(|p| built_ins.iter().find(|item| item.id == p))
      .map(|item| item.id.clone());
    from_param.unwrap_or_else(|| {
      let saved = read_storage(STORAGE_PRESET_KEY, "");
      if !saved.is_empty() && built_ins.iter().any(|item| item.id == saved) {
        saved
      } else {
        built_ins
          .first()
          .map(|p| p.id.clone())
          .unwrap_or_else(|| "instant_step_debug_demo".to_string())
      }
    })
  };
  let initial_yaml = built_ins
    .iter()
    .find(|p| p.id == default_preset_id)
    .or_else(|| built_ins.first())
    .map(|p| p.yaml.clone())
    .unwrap_or_default();

  let mut mode = use_signal(move || initial_mode);
  let mut token_sig = use_signal(|| api_token.clone());
  let mut selected_preset_id = use_signal(move || default_preset_id);
  let initial_yaml_for_sig = initial_yaml.clone();
  let mut yaml_draft = use_signal(move || initial_yaml_for_sig);
  let mut inspected = use_signal(move || inspect_yaml_locally(&initial_yaml));
  let mut inspect_source = use_signal(|| "local");
  let mut input_drafts = use_signal(HashMap::<String, String>::new);

  let mut tenant_id = use_signal(|| {
    let t = read_storage(STORAGE_TENANT_KEY, "default");
    if t.trim().is_empty() {
      "default".to_string()
    } else {
      t
    }
  });
  let mut model_override = use_signal(|| read_storage(STORAGE_MODEL_OVERRIDE_KEY, ""));
  let mut model_combo_open = use_signal(|| false);
  let mut cost_limit_draft = use_signal(String::new);

  let mut preset_category = use_signal(|| "全部".to_string());
  let mut personal_templates = use_signal(|| {
    let raw = read_storage(STORAGE_PERSONAL_TPL_KEY, "");
    serde_json::from_str::<Vec<PersonalWorkflowTemplate>>(&raw).unwrap_or_default()
  });
  let mut save_tpl_open = use_signal(|| false);
  let mut save_tpl_title = use_signal(String::new);

  // Layout state
  let mut left_size = use_signal(|| 24.0_f64);
  let mut center_size = use_signal(|| 48.0_f64);
  let mut right_size = use_signal(|| 28.0_f64);
  let mut left_collapsed = use_signal(|| false);
  let mut right_collapsed = use_signal(|| false);
  let mut release_form_collapsed = use_signal(|| false);
  let mut show_yaml_editor = use_signal(|| true);

  // Drawers / Dialogs
  let mut config_drawer_open = use_signal(|| false);
  let mut inspector_drawer_open = use_signal(|| false);
  let mut yaml_drawer_open = use_signal(|| false);
  let mut conn_dialog_open = use_signal(|| false);

  // Debug & Run state
  let mut step_every_node = use_signal(|| true);
  let mut breakpoints = use_signal(Vec::<String>::new);
  let mut selected_node_id = use_signal(|| None::<String>);

  let mut run_id = use_signal(|| initial_run_id.clone().unwrap_or_default());
  let mut active_run = use_signal(|| None::<RunRecord>);
  let mut recent_runs = use_signal(Vec::<RunRecord>::new);
  let mut events = use_signal(Vec::<StreamedEvent>::new);
  let mut artifacts = use_signal(Vec::<ArtifactSummary>::new);
  let mut server_debug_paused = use_signal(|| None::<FlowDebugPause>);

  let mut stream_state = use_signal(|| "idle");
  let mut run_generation = use_signal(|| 0_u64);
  let mut submit_busy = use_signal(|| false);
  let mut debug_action_busy = use_signal(|| false);
  let mut zip_export_busy = use_signal(|| false);
  let mut banner_error = use_signal(|| None::<String>);
  let mut banner_notice = use_signal(|| None::<String>);

  // Resume plan modal
  let mut resume_modal_open = use_signal(|| false);
  let mut resume_plan_loading = use_signal(|| false);
  let mut resume_plan_data = use_signal(|| None::<ResumePlan>);
  let mut resume_plan_error = use_signal(|| None::<String>);
  let mut resume_action_busy = use_signal(|| false);

  // Approvals & HITL Questions
  let mut pending_approvals = use_signal(Vec::<PendingApproval>::new);
  let mut approval_busy_id = use_signal(|| None::<String>);
  let mut pending_questions = use_signal(Vec::<PendingRunQuestion>::new);
  let mut question_drafts = use_signal(HashMap::<String, String>::new);
  let mut question_busy_id = use_signal(|| None::<String>);

  // Attach run to notebook
  let mut attach_picker_open = use_signal(|| false);
  let mut attach_notebooks = use_signal(Vec::<Notebook>::new);
  let mut attach_loading = use_signal(|| false);
  let mut selected_attach_nb = use_signal(String::new);
  let mut attach_busy = use_signal(|| false);
  let mut attached_receipt = use_signal(|| None::<(String, String, usize)>);
  let mut last_auto_focused_pause = use_signal(String::new);

  // Sync mode & api_token when parent updates props
  use_effect(use_reactive((&initial_mode,), move |(m,)| {
    mode.set(m);
  }));
  let tok_sync = api_token.clone();
  use_effect(use_reactive((&tok_sync,), move |(t,)| {
    token_sig.set(t);
  }));

  // Helper to attach & stream a run by ID
  let mut watch_run = move |target_run_id: String, tok: String, ten: String| {
    if target_run_id.trim().is_empty() {
      return;
    }
    let next_gen = run_generation.peek().wrapping_add(1);
    run_generation.set(next_gen);
    run_id.set(target_run_id.clone());
    stream_state.set("loading");
    events.set(Vec::new());
    artifacts.set(Vec::new());
    server_debug_paused.set(None);
    pending_approvals.set(Vec::new());
    pending_questions.set(Vec::new());
    banner_error.set(None);

    spawn(async move {
      let record = match get_run(&target_run_id, &ten, &tok).await {
        Ok(r) => r,
        Err(err) => {
          if *run_generation.peek() == next_gen {
            stream_state.set("error");
            banner_error.set(Some(format!("查询运行记录失败：{err}")));
          }
          return;
        }
      };
      if *run_generation.peek() != next_gen {
        return;
      }
      if !record.workflow.trim().is_empty() {
        yaml_draft.set(record.workflow.clone());
      }
      let already_terminal = is_terminal_run_record(&record);
      active_run.set(Some(record));

      // Best-effort debug snapshot check
      if let Ok(dbg) = get_debug_status(&target_run_id, &ten, &tok).await
        && *run_generation.peek() == next_gen
        && dbg.active
        && let Some(snap) = dbg.snapshot
      {
        breakpoints.set(snap.breakpoints);
        step_every_node.set(snap.step_every_node);
        server_debug_paused.set(snap.paused_at);
      }

      // Fetch historical events first
      let mut after_seq: i64 = -1;
      if let Ok(hist) = get_run_events_history(&target_run_id, None, &ten, &tok).await
        && *run_generation.peek() == next_gen
      {
        after_seq = hist.last().map(|e| e.seq).unwrap_or(-1);
        events.set(hist);
      }

      if already_terminal {
        stream_state.set("closed");
        if let Ok(arts) = get_run_artifacts(&target_run_id, &ten, &tok).await
          && *run_generation.peek() == next_gen
        {
          artifacts.set(arts);
        }
        return;
      }

      // Spawn background polling for run-level approvals and HITL questions
      {
        let poll_rid = target_run_id.clone();
        let poll_tok = tok.clone();
        let poll_ten = ten.clone();
        spawn(async move {
          while *run_generation.peek() == next_gen
            && matches!(*stream_state.peek(), "loading" | "streaming")
          {
            if let Ok(apps) = list_run_approvals(&poll_rid, &poll_ten, &poll_tok).await
              && *run_generation.peek() == next_gen
            {
              pending_approvals.set(apps);
            }
            if let Ok(qs) = list_run_questions(&poll_rid, &poll_ten, &poll_tok).await
              && *run_generation.peek() == next_gen
            {
              pending_questions.set(qs);
            }
            sleep_ms(1800).await;
          }
          if *run_generation.peek() == next_gen {
            pending_approvals.set(Vec::new());
            pending_questions.set(Vec::new());
          }
        });
      }

      stream_state.set("streaming");
      let sse_path = format!(
        "/v1/runs/{}/events?after_seq={after_seq}",
        encode_uri_component(&target_run_id)
      );
      if let Ok(abort_handle) = SseAbortHandle::new() {
        let _ = stream_sse(&sse_path, &tok, &ten, &abort_handle, |frame| {
          if *run_generation.peek() != next_gen {
            abort_handle.abort();
            return;
          }
          if let Ok(ev) = serde_json::from_str::<StreamedEvent>(&frame.data) {
            if ev.kind == "node.debug.paused" {
              server_debug_paused.set(None);
            }
            let mut cur = events.peek().clone();
            if !cur.iter().any(|item| item.seq == ev.seq) {
              cur.push(ev);
              cur.sort_by_key(|item| item.seq);
              events.set(cur);
            }
          }
        })
        .await;
      }

      if *run_generation.peek() != next_gen {
        return;
      }
      stream_state.set("closed");
      server_debug_paused.set(None);
      if let Ok(final_run) = get_run(&target_run_id, &ten, &tok).await
        && *run_generation.peek() == next_gen
      {
        active_run.set(Some(final_run));
      }
      if let Ok(arts) = get_run_artifacts(&target_run_id, &ten, &tok).await
        && *run_generation.peek() == next_gen
      {
        artifacts.set(arts);
      }
      if let Ok(runs) = list_runs(&ten, 10, &tok).await
        && *run_generation.peek() == next_gen
      {
        recent_runs.set(runs);
      }
    });
  };

  // Auto-attach initial_run_id if provided in route query (skip redundant re-fetch on Debug<->Release switch)
  let init_rid = initial_run_id.clone().unwrap_or_default();
  let tok_init = api_token.clone();
  use_effect(use_reactive((&init_rid,), move |(rid,)| {
    if !rid.is_empty() && (*run_id.peek() != rid || *stream_state.peek() == "idle") {
      let ten = tenant_id.peek().clone();
      watch_run(rid, tok_init.clone(), ten);
    }
  }));

  // Hydrate recent runs and personal templates on mount / tenant / token change
  let tok_hyd = api_token.clone();
  let ten_hyd = tenant_id();
  use_effect(use_reactive((&tok_hyd, &ten_hyd), move |(tok, ten)| {
    write_storage(STORAGE_TENANT_KEY, &ten);
    spawn(async move {
      if let Ok(runs) = list_runs(&ten, 10, &tok).await {
        recent_runs.set(runs);
      }
      if let Ok(prefs) = fetch_preferences(&ten, &tok).await {
        let restored = restore_personal_templates_from_preferences(prefs.as_ref());
        if !restored.is_empty() {
          if let Ok(raw) = serde_json::to_string(&restored) {
            write_storage(STORAGE_PERSONAL_TPL_KEY, &raw);
          }
          personal_templates.set(restored);
        }
      }
    });
  }));

  // Debounced inspect: immediate local inspect + server `/v1/workflows/inspect`
  let yaml_for_inspect = yaml_draft();
  let tok_insp = api_token.clone();
  let ten_insp = tenant_id();
  let mut inspect_gen = use_signal(|| 0_u64);
  use_effect(use_reactive(
    (&yaml_for_inspect, &tok_insp, &ten_insp),
    move |(yml, tok, ten)| {
      let local = inspect_yaml_locally(&yml);
      inspected.set(local);
      inspect_source.set("local");
      let my_gen = inspect_gen.peek().wrapping_add(1);
      inspect_gen.set(my_gen);
      spawn(async move {
        sleep_ms(260).await;
        if *inspect_gen.peek() != my_gen {
          return;
        }
        if let Ok(server_res) = inspect_workflow_server(&yml, &tok, &ten).await
          && *inspect_gen.peek() == my_gen
        {
          inspected.set(server_res);
          inspect_source.set("server");
        }
      });
    },
  ));

  // Sync input defaults & default selected node whenever inspected topology updates
  let insp_now = inspected();
  let insp_node_sig: String = insp_now
    .nodes
    .iter()
    .map(|n| n.id.as_str())
    .collect::<Vec<_>>()
    .join(",");
  let insp_inputs_sig: String = insp_now
    .inputs
    .iter()
    .map(|i| i.name.as_str())
    .collect::<Vec<_>>()
    .join(",");
  use_effect(use_reactive(
    (&insp_node_sig, &insp_inputs_sig),
    move |_| {
      let cur_insp = inspected.peek().clone();
      let mut drafts = input_drafts.peek().clone();
      let mut changed = false;
      for spec in &cur_insp.inputs {
        if !drafts.contains_key(&spec.name) {
          drafts.insert(spec.name.clone(), default_input_string(spec));
          changed = true;
        }
      }
      if changed {
        input_drafts.set(drafts);
      }
      let cur_sel = selected_node_id.peek().clone();
      let valid_sel = cur_sel
        .as_deref()
        .is_some_and(|id| cur_insp.nodes.iter().any(|n| n.id == id));
      if !valid_sel {
        let fallback = cur_insp
          .execution_order
          .first()
          .cloned()
          .or_else(|| cur_insp.nodes.first().map(|n| n.id.clone()));
        selected_node_id.set(fallback);
      }
    },
  ));

  // Derive reduced telemetry & auto-focus paused node
  let all_events = events();
  let reduced = reduce_workflow_events(&all_events);
  let effective_paused = reduced.paused_at.clone().or(server_debug_paused());

  let pause_sig = effective_paused
    .as_ref()
    .map(|p| format!("{}:{}:{}:{}", run_id(), p.node_id, p.phase, all_events.len()))
    .unwrap_or_default();
  let pause_target_id = effective_paused.as_ref().map(|p| p.node_id.clone());
  use_effect(use_reactive((&pause_sig,), move |(sig,)| {
    if !sig.is_empty()
      && *last_auto_focused_pause.peek() != sig
      && let Some(nid) = pause_target_id.clone()
    {
      last_auto_focused_pause.set(sig);
      selected_node_id.set(Some(nid));
    }
  }));

  // Build combined preset list (personal + built-in)
  let all_presets: Vec<WorkflowPreset> = {
    let mut list: Vec<WorkflowPreset> = personal_templates()
      .into_iter()
      .map(|tpl| WorkflowPreset {
        id: format!("personal:{}", tpl.id),
        title: tpl.title,
        subtitle: format!("个人收藏工作流 · 更新于 {}", tpl.updated_at),
        badge: "个人收藏".to_string(),
        category: "个人收藏".to_string(),
        yaml: tpl.yaml,
        personal_id: Some(tpl.id),
      })
      .collect();
    list.extend(built_ins.iter().cloned());
    list
  };
  let cur_cat = preset_category();
  let visible_presets: Vec<WorkflowPreset> = if cur_cat == "全部" {
    all_presets.clone()
  } else {
    all_presets
      .iter()
      .filter(|p| p.category == cur_cat)
      .cloned()
      .collect()
  };

  let cur_run_id = run_id();
  let cur_active_run = active_run();
  let is_run_active = !cur_run_id.is_empty()
    && matches!(stream_state(), "loading" | "streaming")
    && cur_active_run
      .as_ref()
      .is_none_or(|r| !is_terminal_run_record(r));

  let selected_node = selected_node_id()
    .as_deref()
    .and_then(|sid| insp_now.nodes.iter().find(|n| n.id == sid).cloned());

  let deliverables = collect_run_deliverables(&cur_run_id, &artifacts(), &reduced.node_outputs);
  let final_text_summary =
    extract_final_text_summary(&insp_now.execution_order, &reduced.node_outputs);

  let completed_count = insp_now
    .nodes
    .iter()
    .filter(|n| {
      matches!(
        reduced.telemetry_by_node.get(&n.id).map(|t| t.status),
        Some(
          crate::studio_api::StudioNodeRunStatus::Succeeded
            | crate::studio_api::StudioNodeRunStatus::Skipped
            | crate::studio_api::StudioNodeRunStatus::PausedAfter
        )
      )
    })
    .count();
  let progress_pct = if insp_now.nodes.is_empty() {
    0.0
  } else {
    ((completed_count as f64 / insp_now.nodes.len() as f64) * 100.0).min(100.0)
  };

  let effective_cost_limit_usd = {
    let user_cap = cost_limit_draft()
      .trim()
      .parse::<f64>()
      .ok()
      .filter(|v| v.is_finite() && *v > 0.0);
    let yaml_cap = insp_now
      .cost_limit_usd
      .filter(|v| v.is_finite() && *v > 0.0);
    match (user_cap, yaml_cap) {
      (Some(a), Some(b)) => Some(a.min(b)),
      (a, b) => a.or(b),
    }
  };

  let can_resume_run = !cur_run_id.is_empty()
    && !is_run_active
    && (cur_active_run
      .as_ref()
      .is_some_and(|r| r.status == "failed" || r.status == "cancelled")
      || reduced.workflow_error.is_some());

  // Filtered model suggestions for free-solo Combobox
  let model_query_lower = model_override().to_ascii_lowercase();
  let filtered_models: Vec<String> = models
    .iter()
    .map(|m| m.name.clone())
    .filter(|m| model_query_lower.is_empty() || m.to_ascii_lowercase().contains(&model_query_lower))
    .take(24)
    .collect();

  // Action closures
  let tok_launch = api_token.clone();
  let mut do_launch_run = move |target_posture: StudioPosture| {
    let yml = yaml_draft.peek().clone();
    if yml.trim().is_empty() {
      banner_error.set(Some("工作流 YAML 不能为空".to_string()));
      return;
    }
    let cur_insp = inspected.peek().clone();
    let drafts = input_drafts.peek().clone();
    let inputs = match coerce_inputs_payload(&cur_insp.inputs, &drafts) {
      Ok(map) => map,
      Err(err) => {
        banner_error.set(Some(err));
        return;
      }
    };
    let cost_raw = cost_limit_draft.peek().trim().to_string();
    let parsed_cost_limit = if cost_raw.is_empty() {
      None
    } else {
      match cost_raw.parse::<f64>().ok().filter(|v| v.is_finite() && *v > 0.0) {
        Some(v) => Some(v),
        None => {
          banner_error.set(Some(
            "单次运行预算上限 (cost_limit_usd) 必须为大于 0 的数字".to_string(),
          ));
          return;
        }
      }
    };

    let ten = tenant_id.peek().clone();
    let mdl = model_override.peek().clone();
    let step_all = *step_every_node.peek();
    let bps = breakpoints.peek().clone();
    let tok = tok_launch.clone();

    submit_busy.set(true);
    banner_error.set(None);
    server_debug_paused.set(None);
    spawn(async move {
      let req = LaunchStudioRunRequest {
        tenant_id: ten.clone(),
        workflow_yaml: yml,
        inputs,
        model: (!mdl.trim().is_empty()).then_some(mdl),
        cost_limit_usd: parsed_cost_limit,
        run_mode: target_posture.as_str().to_string(),
        step_every_node: target_posture == StudioPosture::Debug && step_all,
        breakpoints: if target_posture == StudioPosture::Debug {
          bps
        } else {
          Vec::new()
        },
      };
      match launch_studio_run(&req, &tok).await {
        Ok(new_run_id) => {
          submit_busy.set(false);
          watch_run(new_run_id, tok, ten);
        }
        Err(err) => {
          submit_busy.set(false);
          banner_error.set(Some(format!("启动运行失败：{err}")));
        }
      }
    });
  };

  let mut do_debug_action = move |action: &'static str, overrides: Option<DebugResumeOverrides>| {
    let rid = run_id.peek().clone();
    if rid.is_empty() {
      return;
    }
    let ten = tenant_id.peek().clone();
    let tok = token_sig.peek().clone();
    debug_action_busy.set(true);
    banner_error.set(None);
    spawn(async move {
      let mut body = serde_json::Map::new();
      body.insert("action".to_string(), serde_json::json!(action));
      if let Some(ov) = overrides {
        if let Some(inp) = ov.input_override {
          body.insert(
            "input_override".to_string(),
            serde_json::Value::Object(inp),
          );
        }
        if let Some(out) = ov.output_override {
          body.insert(
            "output_override".to_string(),
            serde_json::Value::Object(out),
          );
        }
      }
      match send_debug_command(&rid, &serde_json::Value::Object(body), &ten, &tok).await {
        Ok(resp) => {
          server_debug_paused.set(resp.snapshot.and_then(|s| s.paused_at));
        }
        Err(err) => banner_error.set(Some(format!("调试指令失败：{err}"))),
      }
      debug_action_busy.set(false);
    });
  };

  let mut do_toggle_breakpoint = move |node_id: String| {
    let mut next = breakpoints.peek().clone();
    if let Some(pos) = next.iter().position(|b| b == &node_id) {
      next.remove(pos);
    } else {
      next.push(node_id);
    }
    breakpoints.set(next.clone());
    let rid = run_id.peek().clone();
    if !rid.is_empty() && matches!(*stream_state.peek(), "loading" | "streaming") {
      let ten = tenant_id.peek().clone();
      let tok = token_sig.peek().clone();
      let step_all = *step_every_node.peek();
      spawn(async move {
        let payload = serde_json::json!({
          "action": "set_breakpoints",
          "breakpoints": next,
          "step_every_node": step_all,
        });
        let _ = send_debug_command(&rid, &payload, &ten, &tok).await;
      });
    }
  };

  let tok_cancel = api_token.clone();
  let mut do_cancel_run = move || {
    let rid = run_id.peek().clone();
    if rid.is_empty() {
      return;
    }
    let ten = tenant_id.peek().clone();
    let tok = tok_cancel.clone();
    debug_action_busy.set(true);
    spawn(async move {
      match cancel_run(&rid, &ten, &tok).await {
        Ok(rec) => {
          let next_gen = run_generation.peek().wrapping_add(1);
          run_generation.set(next_gen);
          active_run.set(Some(rec));
          server_debug_paused.set(None);
          stream_state.set("closed");
        }
        Err(err) => banner_error.set(Some(format!("终止运行失败：{err}"))),
      }
      debug_action_busy.set(false);
    });
  };

  let tok_resume_open = api_token.clone();
  let mut do_open_resume_plan = move || {
    let rid = run_id.peek().clone();
    if rid.is_empty() {
      return;
    }
    let ten = tenant_id.peek().clone();
    let tok = tok_resume_open.clone();
    resume_modal_open.set(true);
    resume_plan_loading.set(true);
    resume_plan_error.set(None);
    resume_plan_data.set(None);
    spawn(async move {
      match get_resume_plan(&rid, &ten, &tok).await {
        Ok(plan) => resume_plan_data.set(Some(plan)),
        Err(err) => resume_plan_error.set(Some(format!("读取续跑计划失败：{err}"))),
      }
      resume_plan_loading.set(false);
    });
  };

  let tok_zip = api_token.clone();
  let mut do_export_zip = move || {
    let rid = run_id.peek().clone();
    if rid.is_empty() || *zip_export_busy.peek() {
      return;
    }
    let ten = tenant_id.peek().clone();
    let tok = tok_zip.clone();
    zip_export_busy.set(true);
    banner_error.set(None);
    spawn(async move {
      let path = format!("/v1/runs/{}/artifacts.zip", encode_uri_component(&rid));
      let fname = format!("yanshi-run-{}-artifacts.zip", short_id(&rid));
      if let Err(err) = download_authenticated_resource(&path, &tok, &ten, &fname).await {
        banner_error.set(Some(format!("打包导出失败：{err}")));
      }
      zip_export_busy.set(false);
    });
  };

  let tok_attach_open = api_token.clone();
  let mut do_open_attach = move || {
    let ten = tenant_id.peek().clone();
    let tok = tok_attach_open.clone();
    attach_picker_open.set(true);
    attached_receipt.set(None);
    attach_loading.set(true);
    spawn(async move {
      match list_notebooks(&ten, &tok).await {
        Ok(nbs) => {
          if let Some(first) = nbs.first()
            && !nbs.iter().any(|n| n.id == *selected_attach_nb.peek())
          {
            selected_attach_nb.set(first.id.clone());
          }
          attach_notebooks.set(nbs);
        }
        Err(err) => banner_error.set(Some(format!("加载笔记本列表失败：{err}"))),
      }
      attach_loading.set(false);
    });
  };

  // Canvas & Inspector YAML mutation callbacks
  let do_add_node = move |(node_type, after_id): (String, Option<String>)| {
    let yml = yaml_draft.peek().clone();
    match add_node_to_yaml(&yml, &node_type, after_id.as_deref(), None) {
      Ok(res) => {
        banner_error.set(None);
        selected_preset_id.set("custom".to_string());
        yaml_draft.set(res.yaml);
        if let Some(cid) = res.created_node_id {
          selected_node_id.set(Some(cid));
        }
      }
      Err(err) => banner_error.set(Some(err)),
    }
  };

  let do_connect_nodes = move |(src, tgt): (String, String)| {
    let yml = yaml_draft.peek().clone();
    match connect_nodes_in_yaml(&yml, &src, &tgt) {
      Ok(res) => {
        banner_error.set(None);
        selected_preset_id.set("custom".to_string());
        yaml_draft.set(res.yaml);
        selected_node_id.set(Some(tgt));
      }
      Err(err) => banner_error.set(Some(err)),
    }
  };

  let do_disconnect_edge = move |(src, tgt): (String, String)| {
    let yml = yaml_draft.peek().clone();
    match disconnect_edge_in_yaml(&yml, &src, &tgt) {
      Ok(res) => {
        banner_error.set(None);
        selected_preset_id.set("custom".to_string());
        yaml_draft.set(res.yaml);
      }
      Err(err) => banner_error.set(Some(err)),
    }
  };

  let do_delete_node = move |nid: String| {
    let yml = yaml_draft.peek().clone();
    match delete_node_from_yaml(&yml, &nid) {
      Ok(res) => {
        banner_error.set(None);
        selected_preset_id.set("custom".to_string());
        yaml_draft.set(res.yaml);
        let next_bps: Vec<String> = breakpoints
          .peek()
          .iter()
          .filter(|b| *b != &nid)
          .cloned()
          .collect();
        breakpoints.set(next_bps);
        if selected_node_id.peek().as_deref() == Some(nid.as_str()) {
          selected_node_id.set(None);
        }
      }
      Err(err) => banner_error.set(Some(err)),
    }
  };

  let do_apply_node_patch = move |(nid, patch): (String, NodeUpdatePatch)| {
    let yml = yaml_draft.peek().clone();
    match update_node_in_yaml(&yml, &nid, &patch) {
      Ok(res) => {
        banner_error.set(None);
        selected_preset_id.set("custom".to_string());
        yaml_draft.set(res.yaml);
        if let Some(renamed) = res.renamed_to {
          selected_node_id.set(Some(renamed.clone()));
          let next_bps: Vec<String> = breakpoints
            .peek()
            .iter()
            .map(|b| if b == &nid { renamed.clone() } else { b.clone() })
            .collect();
          breakpoints.set(next_bps);
        }
      }
      Err(err) => banner_error.set(Some(err)),
    }
  };

  let mut apply_selected_preset = move |preset_id: String, preset_yaml: String| {
    selected_preset_id.set(preset_id.clone());
    write_storage(STORAGE_PRESET_KEY, &preset_id);
    yaml_draft.set(preset_yaml);
    input_drafts.set(HashMap::new());
    breakpoints.set(Vec::new());
  };

  // Shared typed inputs renderer
  let render_inputs_form = |specs: &[crate::studio_yaml::InspectedInput], drafts: &HashMap<String, String>| {
    if specs.is_empty() {
      return rsx! {
        p { class: "text-xs text-muted-foreground", "该工作流未声明外部 inputs 参数，可直接启动运行。" }
      };
    }
    rsx! {
      div { class: "space-y-2.5",
        for spec in specs.iter().cloned() {
          {
            let name = spec.name.clone();
            let val = drafts
              .get(&name)
              .cloned()
              .unwrap_or_else(|| default_input_string(&spec));
            let is_long = spec.kind == InspectedInputKind::String
              && (name.contains("source")
                || name.contains("notes")
                || name.contains("prompt")
                || val.len() > 72);
            let supports_file = spec.kind == InspectedInputKind::String
              && (name == "source"
                || name.ends_with("_source")
                || name.contains("pdf")
                || name.contains("video")
                || name.contains("audio"));
            let is_data_uri = val.starts_with("data:");
            let file_input_id = format!("studio-file-input-{name}");

            if spec.kind == InspectedInputKind::Boolean {
              let checked = val == "true";
              let name_chk = name.clone();
              rsx! {
                div { key: "inp-{name}", class: "flex items-start gap-2 rounded-md border border-border/70 bg-background p-2",
                  Checkbox {
                    checked,
                    on_checked_change: move |next| {
                      let mut d = input_drafts.peek().clone();
                      d.insert(name_chk.clone(), if next { "true" } else { "false" }.to_string());
                      input_drafts.set(d);
                    },
                  }
                  div { class: "text-xs leading-tight",
                    strong { class: "font-mono block", "{spec.name}" }
                    if let Some(desc) = &spec.description {
                      span { class: "text-[11px] text-muted-foreground", "{desc}" }
                    }
                  }
                }
              }
            } else {
              let name_chg = name.clone();
              let name_clr = name.clone();
              let name_file = name.clone();
              let fid_change = file_input_id.clone();
              let default_str = default_input_string(&spec);
              rsx! {
                div { key: "inp-{name}", class: "space-y-1",
                  div { class: "flex items-center justify-between gap-2 text-xs",
                    span { class: "font-semibold font-mono",
                      "{spec.name}"
                      if spec.required {
                        span { class: "text-destructive ml-0.5", "*" }
                      }
                    }
                    div { class: "flex items-center gap-1.5 text-[11px] text-muted-foreground",
                      if supports_file {
                        label {
                          class: "cursor-pointer rounded border border-border bg-muted/40 hover:bg-accent px-1.5 py-0.5 text-[10px] text-foreground",
                          r#for: "{file_input_id}",
                          "📎 本地文件"
                        }
                        input {
                          id: "{file_input_id}",
                          r#type: "file",
                          class: "hidden",
                          accept: ".pdf,.mp4,.mkv,.webm,.mov,.mp3,.wav,.m4a,.srt,.vtt",
                          onchange: move |_| {
                            let fid = fid_change.clone();
                            let key = name_file.clone();
                            spawn(async move {
                              if let Ok(Some((fname, bytes))) = read_input_file_bytes(&fid).await {
                                clear_file_input(&fid);
                                if bytes.len() > 24 * 1024 * 1024 {
                                  banner_error.set(Some(format!("文件「{fname}」超过 24MB 内联上限，请改传 URL")));
                                  return;
                                }
                                let mime = guess_data_uri_mime(&fname);
                                let b64 = encode_base64(&bytes);
                                let uri = format!("data:{mime};base64,{b64}");
                                let mut d = input_drafts.peek().clone();
                                d.insert(key, uri);
                                input_drafts.set(d);
                                banner_notice.set(Some(format!("已载入本地文件「{fname}」（{:.1} KB）", bytes.len() as f64 / 1024.0)));
                              }
                            });
                          },
                        }
                      }
                      span { "{spec.kind.as_str()}" }
                    }
                  }
                  if let Some(desc) = &spec.description {
                    p { class: "text-[11px] text-muted-foreground leading-snug", "{desc}" }
                  }
                  if is_data_uri {
                    {
                      let kb = val.len() / 1024;
                      rsx! {
                        div { class: "flex items-center justify-between gap-2 rounded-md border border-border bg-muted/40 px-2.5 py-1.5 text-xs",
                          span { "已载入内联文件（{kb} KB Data URI）" }
                          Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: "h-6 px-2 text-[11px]".to_string(),
                            onclick: move |_| {
                              let mut d = input_drafts.peek().clone();
                              d.insert(name_clr.clone(), default_str.clone());
                              input_drafts.set(d);
                            },
                            "清除"
                          }
                        }
                      }
                    }
                  } else if spec.kind == InspectedInputKind::Json || is_long {
                    Textarea {
                      class: "min-h-[60px] text-xs font-mono".to_string(),
                      value: val,
                      on_value_change: move |v: String| {
                        let mut d = input_drafts.peek().clone();
                        d.insert(name_chg.clone(), v);
                        input_drafts.set(d);
                      },
                    }
                  } else {
                    Input {
                      class: "h-8 text-xs".to_string(),
                      value: val,
                      on_value_change: move |v: String| {
                        let mut d = input_drafts.peek().clone();
                        d.insert(name_chg.clone(), v);
                        input_drafts.set(d);
                      },
                    }
                  }
                }
              }
            }
          }
        }
      }
    }
  };

  let cur_mode = mode();
  let cur_drafts = input_drafts();
  let tok_save_tpl = api_token.clone();
  let tok_del_tpl = api_token.clone();
  let tok_attach_confirm = api_token.clone();
  let tok_resume_confirm = api_token.clone();
  let tok_step_toggle = api_token.clone();
  let tok_approval = api_token.clone();
  let tok_question = api_token.clone();
  let tok_recent_click = api_token.clone();

  rsx! {
    div {
      class: "h-[calc(100vh-53px)] flex flex-col overflow-hidden bg-background",
      "data-testid": "workflow-studio-page",

      // ── Studio Sub-Header Bar ───────────────────────────────────────
      div { class: "shrink-0 border-b border-border bg-card px-3 py-2 flex flex-wrap items-center justify-between gap-2",
        div { class: "flex items-center gap-2 flex-wrap",
          Badge { variant: BadgeVariant::Secondary, class: "font-mono text-xs".to_string(),
            "{insp_now.name.as_deref().unwrap_or(\"未命名工作流\")} · {insp_now.nodes.len()} 节点"
          }
          Badge { variant: BadgeVariant::Outline, class: "text-[11px]".to_string(),
            if inspect_source() == "server" { "校验源：Rust 引擎" } else { "校验源：本地解析" }
          }
        }

        // Posture Switcher (Debug / Release)
        div { class: "inline-flex items-center rounded-md border border-border p-0.5 bg-muted/40 gap-1",
          Button {
            variant: if cur_mode == StudioPosture::Debug { ButtonVariant::Primary } else { ButtonVariant::Ghost },
            size: ButtonSize::Sm,
            class: "h-7 px-2.5 text-xs".to_string(),
            "data-testid": "mode-switch-debug",
            onclick: move |_| {
              mode.set(StudioPosture::Debug);
              let rid = run_id.peek().clone();
              let suffix = if rid.is_empty() { String::new() } else { format!("?run={}", encode_uri_component(&rid)) };
              on_navigate.call(format!("/ui-dx/studio{suffix}"));
            },
            "🔍 调试状态"
          }
          Button {
            variant: if cur_mode == StudioPosture::Release { ButtonVariant::Primary } else { ButtonVariant::Ghost },
            size: ButtonSize::Sm,
            class: "h-7 px-2.5 text-xs".to_string(),
            "data-testid": "mode-switch-release",
            onclick: move |_| {
              mode.set(StudioPosture::Release);
              let rid = run_id.peek().clone();
              let suffix = if rid.is_empty() { String::new() } else { format!("?run={}", encode_uri_component(&rid)) };
              on_navigate.call(format!("/ui-dx/app{suffix}"));
            },
            "🚀 发布状态"
          }
        }

        div { class: "flex items-center gap-1.5 flex-wrap",
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            class: "h-7 px-2 text-xs".to_string(),
            "data-testid": "studio-conn-btn",
            onclick: move |_| conn_dialog_open.set(true),
            "租户：{tenant_id()}"
          }
          if !cur_run_id.is_empty() || !recent_runs().is_empty() {
            {
              let primary = if !cur_run_id.is_empty() {
                cur_run_id.clone()
              } else {
                recent_runs().first().map(|r| r.id.clone()).unwrap_or_default()
              };
              let against = recent_runs()
                .iter()
                .find(|r| r.id != primary)
                .map(|r| r.id.clone());
              let href = match against {
                Some(other) => format!(
                  "/ui-dx/runs/{}/compare?against={}",
                  encode_uri_component(&primary),
                  encode_uri_component(&other)
                ),
                None => format!("/ui-dx/runs/{}/compare", encode_uri_component(&primary)),
              };
              rsx! {
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  class: "h-7 px-2 text-xs".to_string(),
                  onclick: move |_| on_navigate.call(href.clone()),
                  "⇄ 运行对比"
                }
              }
            }
          }
        }
      }

      // ── Alerts, Approvals, HITL Questions & Attach-to-Notebook Bar ──
      div { class: "shrink-0 px-3 space-y-1.5 empty:hidden pt-1.5",
        if let Some(err) = banner_error() {
          Alert { variant: AlertVariant::Destructive, class: "py-2".to_string(),
            AlertDescription { class: "flex items-center justify-between gap-2 text-xs".to_string(),
              span { "{err}" }
              Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, class: "h-6 px-2 text-xs".to_string(), onclick: move |_| banner_error.set(None), "关闭" }
            }
          }
        }
        if let Some(notice) = banner_notice() {
          Alert { variant: AlertVariant::Default, class: "py-2".to_string(),
            AlertDescription { class: "flex items-center justify-between gap-2 text-xs".to_string(),
              span { "{notice}" }
              Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, class: "h-6 px-2 text-xs".to_string(), onclick: move |_| banner_notice.set(None), "关闭" }
            }
          }
        }
        if !insp_now.diagnostics.valid && !insp_now.diagnostics.issues.is_empty() {
          Alert { variant: AlertVariant::Destructive, class: "py-2".to_string(),
            AlertDescription { class: "text-xs".to_string(),
              "工作流校验问题：{insp_now.diagnostics.issues.join(\"；\")}"
            }
          }
        }
        if let Some(exceeded) = &reduced.cost_limit_exceeded {
          div { "data-testid": "cost-limit-alert",
            Alert { variant: AlertVariant::Destructive, class: "py-2".to_string(),
              AlertDescription { class: "text-xs".to_string(),
                "💰 预算熔断已触发：在节点 #{exceeded.node_id} 处累计花费 {format_spend_usd(exceeded.spent_usd)} 已超过预算上限 {format_spend_usd(exceeded.limit_usd)}"
              }
            }
          }
        }

        // Attach run to notebook bar
        if attach_picker_open() && !cur_run_id.is_empty() {
          div {
            class: "rounded-md border border-primary/40 bg-primary/5 px-3 py-2 flex flex-wrap items-center justify-between gap-2 text-xs",
            "data-testid": "attach-notebook-bar",
            div { class: "flex items-center gap-2 flex-wrap",
              span { class: "font-medium", "将本次运行成果存入笔记本：" }
              if attach_loading() {
                span { class: "text-muted-foreground", "正在读取笔记本列表…" }
              } else if attach_notebooks().is_empty() {
                span { class: "text-muted-foreground", "当前租户暂无笔记本，请先在 Notebooks 中创建。" }
              } else {
                NativeSelect {
                  size: NativeSelectSize::Sm,
                  class: "w-56".to_string(),
                  "data-testid": "studio-attach-notebook-select",
                  on_value_change: move |v: String| selected_attach_nb.set(v),
                  for nb in attach_notebooks() {
                    NativeSelectOption {
                      key: "{nb.id}",
                      value: nb.id.clone(),
                      selected: selected_attach_nb() == nb.id,
                      "{nb.title}"
                    }
                  }
                }
              }
            }
            div { class: "flex items-center gap-1.5",
              if !attach_notebooks().is_empty() {
                Button {
                  variant: ButtonVariant::Primary,
                  size: ButtonSize::Sm,
                  class: "h-7 px-2.5 text-xs".to_string(),
                  "data-testid": "confirm-attach-notebook-btn",
                  disabled: attach_busy() || selected_attach_nb().is_empty(),
                  onclick: move |_| {
                    let nb_id = selected_attach_nb.peek().clone();
                    let rid = run_id.peek().clone();
                    let ten = tenant_id.peek().clone();
                    let tok = tok_attach_confirm.clone();
                    let nb_title = attach_notebooks
                      .peek()
                      .iter()
                      .find(|n| n.id == nb_id)
                      .map(|n| n.title.clone())
                      .unwrap_or_else(|| short_id(&nb_id));
                    attach_busy.set(true);
                    spawn(async move {
                      match attach_run_to_notebook(&nb_id, &rid, &ten, &tok).await {
                        Ok(res) => {
                          attached_receipt.set(Some((nb_id, nb_title, res.attached_count)));
                          attach_picker_open.set(false);
                        }
                        Err(err) => banner_error.set(Some(format!("存入笔记本失败：{err}"))),
                      }
                      attach_busy.set(false);
                    });
                  },
                  if attach_busy() { "存入中…" } else { "确认存入" }
                }
              }
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                onclick: move |_| attach_picker_open.set(false),
                "取消"
              }
            }
          }
        }

        if let Some((nb_id, nb_title, count)) = attached_receipt() {
          div {
            class: "rounded-md border border-emerald-500/40 bg-emerald-500/10 px-3 py-2 flex items-center justify-between gap-2 text-xs",
            "data-testid": "attached-notebook-receipt",
            span { "✓ 已将 {count} 项成品存入笔记本「{nb_title}」" }
            div { class: "flex items-center gap-1.5",
              Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                onclick: move |_| on_navigate.call(format!("/ui-dx/notebooks/{}", encode_uri_component(&nb_id))),
                "前往笔记本查看 →"
              }
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                onclick: move |_| attached_receipt.set(None),
                "关闭"
              }
            }
          }
        }

        // Inline Run-Level Approvals
        if !pending_approvals().is_empty() {
          div { class: "space-y-1.5", "data-testid": "run-approvals-banner",
            for req in pending_approvals() {
              {
                let req_id_1 = req.id.clone();
                let req_id_2 = req.id.clone();
                let req_id_3 = req.id.clone();
                let busy = approval_busy_id().as_deref() == Some(req.id.as_str());
                let tok1 = tok_approval.clone();
                let tok2 = tok_approval.clone();
                let tok3 = tok_approval.clone();
                rsx! {
                  div { key: "app-{req.id}", class: "rounded-md border border-amber-500/50 bg-amber-500/10 p-2.5 flex flex-wrap items-center justify-between gap-2 text-xs",
                    div { class: "space-y-0.5 min-w-0",
                      div { class: "flex items-center gap-2",
                        Badge { variant: BadgeVariant::Destructive, "审批拦截 · {req.risk}" }
                        strong { "工具：{req.tool}" }
                        span { class: "text-muted-foreground", "Step #{req.step_index}" }
                      }
                      p { class: "text-muted-foreground", "{req.reason}" }
                    }
                    div { class: "flex items-center gap-1.5 shrink-0",
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2 text-xs".to_string(),
                        disabled: busy,
                        onclick: move |_| {
                          let rid = run_id.peek().clone();
                          let ten = tenant_id.peek().clone();
                          let req_id = req_id_1.clone();
                          let tok = tok1.clone();
                          approval_busy_id.set(Some(req_id.clone()));
                          spawn(async move {
                            match decide_run_approval(&rid, &req_id, "allow", "once", &ten, &tok).await {
                              Ok(()) => {
                                let next: Vec<_> = pending_approvals.peek().iter().filter(|a| a.id != req_id).cloned().collect();
                                pending_approvals.set(next);
                              }
                              Err(err) => banner_error.set(Some(format!("提交审批失败：{err}"))),
                            }
                            approval_busy_id.set(None);
                          });
                        },
                        "✓ 允许一次"
                      }
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2 text-xs".to_string(),
                        disabled: busy,
                        onclick: move |_| {
                          let rid = run_id.peek().clone();
                          let ten = tenant_id.peek().clone();
                          let req_id = req_id_2.clone();
                          let tok = tok2.clone();
                          approval_busy_id.set(Some(req_id.clone()));
                          spawn(async move {
                            match decide_run_approval(&rid, &req_id, "allow", "session", &ten, &tok).await {
                              Ok(()) => {
                                let next: Vec<_> = pending_approvals.peek().iter().filter(|a| a.id != req_id).cloned().collect();
                                pending_approvals.set(next);
                              }
                              Err(err) => banner_error.set(Some(format!("提交审批失败：{err}"))),
                            }
                            approval_busy_id.set(None);
                          });
                        },
                        "✓ 本轮始终允许"
                      }
                      Button {
                        variant: ButtonVariant::Destructive,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2 text-xs".to_string(),
                        disabled: busy,
                        onclick: move |_| {
                          let rid = run_id.peek().clone();
                          let ten = tenant_id.peek().clone();
                          let req_id = req_id_3.clone();
                          let tok = tok3.clone();
                          approval_busy_id.set(Some(req_id.clone()));
                          spawn(async move {
                            match decide_run_approval(&rid, &req_id, "deny_and_stop", "once", &ten, &tok).await {
                              Ok(()) => {
                                let next: Vec<_> = pending_approvals.peek().iter().filter(|a| a.id != req_id).cloned().collect();
                                pending_approvals.set(next);
                              }
                              Err(err) => banner_error.set(Some(format!("提交审批失败：{err}"))),
                            }
                            approval_busy_id.set(None);
                          });
                        },
                        "✕ 拒绝并中止"
                      }
                    }
                  }
                }
              }
            }
          }
        }

        // Inline HITL Run Questions (`ask_user`)
        if !pending_questions().is_empty() {
          div { class: "space-y-1.5", "data-testid": "run-questions-banner",
            for q in pending_questions() {
              {
                let qid_val = q.question_id.clone();
                let qid_send = q.question_id.clone();
                let qid_def = q.question_id.clone();
                let draft_txt = question_drafts().get(&q.question_id).cloned().unwrap_or_default();
                let busy = question_busy_id().as_deref() == Some(q.question_id.as_str());
                let tok_s = tok_question.clone();
                let tok_d = tok_question.clone();
                rsx! {
                  div { key: "q-{q.question_id}", class: "rounded-md border border-sky-500/40 bg-sky-500/10 p-2.5 space-y-2 text-xs",
                    div { class: "flex items-center gap-2",
                      Badge { variant: BadgeVariant::Secondary, "💬 Agent 需求澄清" }
                      strong { "节点：{q.node_id}" }
                      span { class: "text-muted-foreground", "回答后工作流将自动继续" }
                    }
                    p { class: "text-foreground whitespace-pre-wrap", "{q.question}" }
                    div { class: "flex flex-col sm:flex-row gap-2",
                      Input {
                        class: "h-8 text-xs flex-1".to_string(),
                        placeholder: "输入你的业务约束或技术偏好…".to_string(),
                        value: draft_txt.clone(),
                        disabled: busy,
                        on_value_change: move |v: String| {
                          let mut m = question_drafts.peek().clone();
                          m.insert(qid_val.clone(), v);
                          question_drafts.set(m);
                        },
                      }
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        class: "h-8 px-3 text-xs shrink-0".to_string(),
                        disabled: busy || draft_txt.trim().is_empty(),
                        onclick: move |_| {
                          let rid = run_id.peek().clone();
                          let ten = tenant_id.peek().clone();
                          let qid = qid_send.clone();
                          let ans = question_drafts.peek().get(&qid).cloned().unwrap_or_default();
                          let tok = tok_s.clone();
                          question_busy_id.set(Some(qid.clone()));
                          spawn(async move {
                            match answer_run_question(&rid, &qid, ans.trim(), &ten, &tok).await {
                              Ok(()) => {
                                let next: Vec<_> = pending_questions.peek().iter().filter(|item| item.question_id != qid).cloned().collect();
                                pending_questions.set(next);
                              }
                              Err(err) => banner_error.set(Some(format!("提交问答失败：{err}"))),
                            }
                            question_busy_id.set(None);
                          });
                        },
                        "发送回答并继续 →"
                      }
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        class: "h-8 px-2.5 text-xs shrink-0".to_string(),
                        disabled: busy,
                        onclick: move |_| {
                          let rid = run_id.peek().clone();
                          let ten = tenant_id.peek().clone();
                          let qid = qid_def.clone();
                          let tok = tok_d.clone();
                          question_busy_id.set(Some(qid.clone()));
                          spawn(async move {
                            let default_ans = "其余细节请直接采用生产级主流工程最佳实践默认方案，无需继续追问，直接输出完整规格方案。";
                            match answer_run_question(&rid, &qid, default_ans, &ten, &tok).await {
                              Ok(()) => {
                                let next: Vec<_> = pending_questions.peek().iter().filter(|item| item.question_id != qid).cloned().collect();
                                pending_questions.set(next);
                              }
                              Err(err) => banner_error.set(Some(format!("提交问答失败：{err}"))),
                            }
                            question_busy_id.set(None);
                          });
                        },
                        "按最佳实践默认继续"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // ── Posture 1: DEBUG MODE (Three-Pane ResizablePanelGroup) ──────
      if cur_mode == StudioPosture::Debug {
        div { class: "flex-1 min-h-0 p-2",
          ResizablePanelGroup {
            orientation: LayoutOrientation::Horizontal,
            class: "h-full rounded-lg border border-border bg-card overflow-hidden".to_string(),

            // LEFT PANE: Presets + Inputs + YAML Editor + Recent Runs
            ResizablePanel {
              size: if left_collapsed() { 0.0 } else { left_size() },
              min_size: 16.0,
              max_size: 44.0,
              collapsed: left_collapsed(),
              class: "h-full flex flex-col bg-card overflow-hidden".to_string(),
              div { class: "shrink-0 flex items-center justify-between px-3 py-2 border-b border-border bg-muted/20",
                strong { class: "text-xs font-semibold", "工作流与输入配置" }
                div { class: "flex items-center gap-1",
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "h-6 px-2 text-xs".to_string(),
                    "data-testid": "studio-expand-config-drawer",
                    onclick: move |_| config_drawer_open.set(true),
                    "⤢ 放大"
                  }
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "h-6 px-2 text-xs".to_string(),
                    "data-testid": "studio-collapse-left-btn",
                    onclick: move |_| left_collapsed.set(true),
                    "收起"
                  }
                }
              }

              div { class: "flex-1 overflow-y-auto p-3 space-y-4",
                // Preset Library Section
                div { class: "space-y-2",
                  div { class: "flex items-center justify-between gap-1 flex-wrap",
                    span { class: "text-xs font-semibold", "模板库 ({all_presets.len()})" }
                    div { class: "flex items-center gap-1",
                      label {
                        class: "cursor-pointer inline-flex items-center h-6 px-1.5 rounded border border-border bg-background hover:bg-accent text-[11px]",
                        r#for: "studio-import-yaml-file",
                        "📂 打开"
                      }
                      input {
                        id: "studio-import-yaml-file",
                        r#type: "file",
                        class: "hidden",
                        accept: ".yml,.yaml",
                        onchange: move |_| {
                          spawn(async move {
                            if let Ok(Some((fname, bytes))) = read_input_file_bytes("studio-import-yaml-file").await {
                              clear_file_input("studio-import-yaml-file");
                              let txt = String::from_utf8_lossy(&bytes).to_string();
                              selected_preset_id.set("custom".to_string());
                              yaml_draft.set(txt);
                              input_drafts.set(HashMap::new());
                              banner_notice.set(Some(format!("已加载本地工作流文件：{fname}")));
                            }
                          });
                        },
                      }
                      Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        class: "h-6 px-1.5 text-[11px]".to_string(),
                        "data-testid": "export-yaml-btn",
                        onclick: move |_| {
                          let fname = build_yaml_download_filename(inspected.peek().name.as_deref());
                          if let Err(err) = trigger_text_download(&yaml_draft.peek(), "text/yaml;charset=utf-8", &fname) {
                            banner_error.set(Some(err));
                          }
                        },
                        "💾 导出"
                      }
                      Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        class: "h-6 px-1.5 text-[11px]".to_string(),
                        "data-testid": "bookmark-template-btn",
                        onclick: move |_| {
                          let cur_name = inspected
                            .peek()
                            .name
                            .clone()
                            .unwrap_or_else(|| "我的自定义工作流".to_string());
                          save_tpl_title.set(cur_name);
                          save_tpl_open.set(!save_tpl_open());
                        },
                        "★ 收藏"
                      }
                    }
                  }

                  if save_tpl_open() {
                    div { class: "flex items-center gap-1.5 rounded border border-border bg-muted/30 p-1.5",
                      Input {
                        class: "h-7 text-xs flex-1".to_string(),
                        placeholder: "输入收藏模板名称…".to_string(),
                        value: save_tpl_title(),
                        on_value_change: move |v: String| save_tpl_title.set(v),
                      }
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2 text-xs".to_string(),
                        onclick: move |_| {
                          let cur_list = personal_templates.peek().clone();
                          let cur_sel = selected_preset_id.peek().clone();
                          let existing_id = cur_sel.strip_prefix("personal:");
                          let title = save_tpl_title.peek().clone();
                          let yml = yaml_draft.peek().clone();
                          match build_personal_templates_preference_patch(
                            &cur_list,
                            existing_id,
                            &title,
                            &yml,
                            "刚刚",
                          ) {
                            Ok((next_list, patch)) => {
                              if let Ok(raw) = serde_json::to_string(&next_list) {
                                write_storage(STORAGE_PERSONAL_TPL_KEY, &raw);
                              }
                              if let Some(first) = next_list.first() {
                                selected_preset_id.set(format!("personal:{}", first.id));
                                banner_notice.set(Some(format!("已保存个人模板「{}」", first.title)));
                              }
                              personal_templates.set(next_list);
                              save_tpl_open.set(false);
                              let ten = tenant_id.peek().clone();
                              let tok = tok_save_tpl.clone();
                              spawn(async move {
                                let _ = put_preferences(&patch, &ten, &tok).await;
                              });
                            }
                            Err(err) => banner_error.set(Some(err)),
                          }
                        },
                        "保存"
                      }
                    }
                  }

                  // Category pills
                  div { class: "flex flex-wrap gap-1",
                    for cat in PRESET_CATEGORIES {
                      {
                        let active = cur_cat == *cat;
                        let cat_s = (*cat).to_string();
                        rsx! {
                          Button {
                            key: "cat-{cat}",
                            variant: if active { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                            size: ButtonSize::Sm,
                            class: "h-6 px-2 text-[11px]".to_string(),
                            onclick: move |_| preset_category.set(cat_s.clone()),
                            "{cat}"
                          }
                        }
                      }
                    }
                  }

                  // Preset cards list
                  div { class: "space-y-1.5 max-h-48 overflow-y-auto pr-0.5",
                    if visible_presets.is_empty() {
                      p { class: "text-xs text-muted-foreground", "当前分类暂无模板（点「★ 收藏」保存当前工作流）。" }
                    } else {
                      for preset in &visible_presets {
                        {
                          let active = selected_preset_id() == preset.id;
                          let pid = preset.id.clone();
                          let pyml = preset.yaml.clone();
                          let personal_del = preset.personal_id.clone();
                          let tok_d = tok_del_tpl.clone();
                          rsx! {
                            div {
                              key: "preset-{preset.id}",
                              class: if active {
                                "flex items-start gap-1 rounded-md border border-primary bg-primary/5 p-2"
                              } else {
                                "flex items-start gap-1 rounded-md border border-border bg-background hover:bg-accent/50 p-2"
                              },
                              button {
                                r#type: "button",
                                class: "flex-1 text-left min-w-0",
                                "data-testid": "preset-card-{preset.id}",
                                onclick: move |_| apply_selected_preset(pid.clone(), pyml.clone()),
                                div { class: "flex items-center justify-between gap-1",
                                  strong { class: "text-xs font-semibold truncate", "{preset.title}" }
                                  Badge { variant: BadgeVariant::Outline, class: "text-[10px] shrink-0".to_string(), "{preset.badge}" }
                                }
                                p { class: "text-[11px] text-muted-foreground line-clamp-2 mt-0.5", "{preset.subtitle}" }
                              }
                              if let Some(personal_id) = personal_del {
                                Button {
                                  variant: ButtonVariant::Ghost,
                                  size: ButtonSize::Sm,
                                  class: "h-6 w-6 p-0 text-xs text-destructive shrink-0".to_string(),
                                  onclick: move |_| {
                                    let (next_list, patch) = remove_personal_template_patch(&personal_templates.peek(), &personal_id);
                                    if let Ok(raw) = serde_json::to_string(&next_list) {
                                      write_storage(STORAGE_PERSONAL_TPL_KEY, &raw);
                                    }
                                    personal_templates.set(next_list);
                                    let ten = tenant_id.peek().clone();
                                    let tok = tok_d.clone();
                                    spawn(async move {
                                      let _ = put_preferences(&patch, &ten, &tok).await;
                                    });
                                  },
                                  "✕"
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

                // Parameters section
                div { class: "space-y-2.5",
                  span { class: "text-xs font-semibold block", "调试输入参数 ({insp_now.inputs.len()})" }
                  div { class: "space-y-1",
                    label { class: "text-xs font-medium block", "全局推理模型覆盖 (model)" }
                    Combobox {
                      id: Some("studio-model-override".to_string()),
                      allow_custom_value: true,
                      value: Some(model_override()),
                      open: Some(model_combo_open()),
                      on_open_change: move |o| model_combo_open.set(o),
                      on_value_change: move |v: String| {
                        write_storage(STORAGE_MODEL_OVERRIDE_KEY, &v);
                        model_override.set(v);
                      },
                      ComboboxInput {
                        class: "h-8 w-full rounded-md border border-input bg-background px-2.5 py-1 text-xs".to_string(),
                        value: model_override(),
                        placeholder: "例如 deepseek-chat / gpt-4o-mini（留空按节点）".to_string(),
                        on_value_change: move |v: String| {
                          write_storage(STORAGE_MODEL_OVERRIDE_KEY, &v);
                          model_override.set(v);
                        },
                      }
                      ComboboxContent {
                        ComboboxList {
                          if filtered_models.is_empty() {
                            ComboboxEmpty { "按 Enter 使用自定义模型名" }
                          } else {
                            for m in filtered_models.clone() {
                              ComboboxItem { key: "{m}", value: m.clone(), "{m}" }
                            }
                          }
                        }
                      }
                    }
                  }
                  div { class: "space-y-1",
                    label { class: "text-xs font-medium block", "单次运行预算上限 USD (cost_limit_usd)" }
                    Input {
                      class: "h-8 text-xs font-mono".to_string(),
                      "data-testid": "studio-cost-limit-input",
                      placeholder: "例如 1.50（留空不设额外上限）".to_string(),
                      value: cost_limit_draft(),
                      on_value_change: move |v: String| cost_limit_draft.set(v),
                    }
                  }
                  {render_inputs_form(&insp_now.inputs, &cur_drafts)}
                }

                Separator {}

                // Inline YAML Editor
                div { class: "space-y-2",
                  div { class: "flex items-center justify-between",
                    span { class: "text-xs font-semibold", "YAML 编排源码" }
                    div { class: "flex items-center gap-1",
                      Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: "h-6 px-2 text-[11px]".to_string(),
                        onclick: move |_| yaml_drawer_open.set(true),
                        "⤢ 全屏"
                      }
                      Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: "h-6 px-2 text-[11px]".to_string(),
                        onclick: move |_| show_yaml_editor.set(!show_yaml_editor()),
                        if show_yaml_editor() { "收起" } else { "展开" }
                      }
                    }
                  }
                  if show_yaml_editor() {
                    YamlCodeEditor {
                      test_id: "studio-yaml-editor".to_string(),
                      aria_label: "YAML 编排源码".to_string(),
                      value: yaml_draft(),
                      rows: 12,
                      on_change: move |next: String| {
                        selected_preset_id.set("custom".to_string());
                        yaml_draft.set(next);
                      },
                    }
                  }
                }

                // Recent Runs
                if !recent_runs().is_empty() {
                  Separator {}
                  div { class: "space-y-1.5",
                    span { class: "text-xs font-semibold block", "最近运行记录" }
                    for r in recent_runs().into_iter().take(6) {
                      {
                        let rid_click = r.id.clone();
                        let rid_cmp = r.id.clone();
                        let is_cur = cur_run_id == r.id;
                        let tok_r = tok_recent_click.clone();
                        let cur_r_for_cmp = cur_run_id.clone();
                        rsx! {
                          div { key: "rr-{r.id}", class: "flex items-center gap-1",
                            Button {
                              variant: if is_cur { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                              size: ButtonSize::Sm,
                              class: "h-7 flex-1 justify-between px-2 text-[11px] font-mono".to_string(),
                              onclick: move |_| {
                                let ten = tenant_id.peek().clone();
                                watch_run(rid_click.clone(), tok_r.clone(), ten);
                              },
                              span { "{short_id(&r.id)}" }
                              Badge { variant: BadgeVariant::Outline, class: "text-[10px]".to_string(), "{r.status}" }
                            }
                            Button {
                              variant: ButtonVariant::Outline,
                              size: ButtonSize::Sm,
                              class: "h-7 px-2 text-[11px]".to_string(),
                              onclick: move |_| {
                                let href = if !cur_r_for_cmp.is_empty() && cur_r_for_cmp != rid_cmp {
                                  format!(
                                    "/ui-dx/runs/{}/compare?against={}",
                                    encode_uri_component(&cur_r_for_cmp),
                                    encode_uri_component(&rid_cmp)
                                  )
                                } else {
                                  format!("/ui-dx/runs/{}/compare", encode_uri_component(&rid_cmp))
                                };
                                on_navigate.call(href);
                              },
                              "对比"
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
                value: left_size(),
                min: 16.0,
                max: 44.0,
                on_resize: move |delta: f64| {
                  let l = ResizablePanelState::new(left_size(), 16.0, 44.0);
                  let c = ResizablePanelState::new(center_size(), 28.0, 72.0);
                  let (nl, nc) = resizable_resize_pair(l, c, delta);
                  left_size.set(nl.size);
                  center_size.set(nc.size);
                },
              }
            }

            // CENTER PANE: Debug Transport Deck + Pure-Rust DAG Canvas
            ResizablePanel {
              size: center_size(),
              min_size: 28.0,
              max_size: 84.0,
              class: "h-full flex flex-col overflow-hidden".to_string(),
              // Transport Deck
              div { class: "shrink-0 border-b border-border bg-muted/30 px-2.5 py-2 flex flex-wrap items-center justify-between gap-2",
                div { class: "flex flex-wrap items-center gap-1.5",
                  if left_collapsed() {
                    Button {
                      variant: ButtonVariant::Ghost,
                      size: ButtonSize::Sm,
                      class: "h-7 px-2 text-xs".to_string(),
                      "data-testid": "studio-reopen-left-btn",
                      onclick: move |_| left_collapsed.set(false),
                      "☰ 展开左栏"
                    }
                  }
                  if right_collapsed() {
                    Button {
                      variant: ButtonVariant::Ghost,
                      size: ButtonSize::Sm,
                      class: "h-7 px-2 text-xs".to_string(),
                      "data-testid": "studio-reopen-right-btn",
                      onclick: move |_| right_collapsed.set(false),
                      "◧ 展开观测"
                    }
                  }

                  Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2.5 text-xs".to_string(),
                    "data-testid": "launch-debug-run",
                    disabled: submit_busy() || !insp_now.diagnostics.valid,
                    onclick: move |_| do_launch_run(StudioPosture::Debug),
                    if submit_busy() { "启动中…" } else { "🔍 启动单步调试" }
                  }

                  div { class: "flex items-center gap-1.5 px-1 text-xs text-muted-foreground",
                    Checkbox {
                      checked: step_every_node(),
                      on_checked_change: move |next: bool| {
                        step_every_node.set(next);
                        let rid = run_id.peek().clone();
                        if !rid.is_empty() && matches!(*stream_state.peek(), "loading" | "streaming") {
                          let ten = tenant_id.peek().clone();
                          let tok = tok_step_toggle.clone();
                          let bps = breakpoints.peek().clone();
                          spawn(async move {
                            let payload = serde_json::json!({
                              "action": "set_breakpoints",
                              "breakpoints": bps,
                              "step_every_node": next,
                            });
                            let _ = send_debug_command(&rid, &payload, &ten, &tok).await;
                          });
                        }
                      },
                    }
                    span { "每步暂停" }
                  }

                  Button {
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2 text-xs".to_string(),
                    "data-testid": "debug-step-next",
                    disabled: effective_paused.is_none() || debug_action_busy(),
                    onclick: move |_| do_debug_action("step", None),
                    "▶ 单步下一步"
                  }
                  Button {
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2 text-xs".to_string(),
                    "data-testid": "debug-continue-run",
                    disabled: effective_paused.is_none() || debug_action_busy(),
                    onclick: move |_| do_debug_action("continue", None),
                    "⏭ 继续跑完"
                  }
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "h-7 px-2 text-xs text-destructive".to_string(),
                    disabled: !is_run_active || debug_action_busy(),
                    onclick: move |_| do_cancel_run(),
                    "⏹ 终止"
                  }
                  if can_resume_run {
                    Button {
                      variant: ButtonVariant::Secondary,
                      size: ButtonSize::Sm,
                      class: "h-7 px-2 text-xs".to_string(),
                      "data-testid": "open-resume-plan-btn",
                      onclick: move |_| do_open_resume_plan(),
                      "↻ 断点续跑"
                    }
                  }
                  if !cur_run_id.is_empty() && (!artifacts().is_empty() || !deliverables.is_empty()) {
                    Button {
                      variant: ButtonVariant::Outline,
                      size: ButtonSize::Sm,
                      class: "h-7 px-2 text-xs".to_string(),
                      "data-testid": "download-artifacts-zip-debug",
                      disabled: zip_export_busy(),
                      onclick: move |_| do_export_zip(),
                      if zip_export_busy() { "打包中…" } else { "📦 .zip" }
                    }
                    if cur_active_run.as_ref().is_some_and(|r| r.status == "succeeded") {
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2 text-xs".to_string(),
                        "data-testid": "open-attach-notebook-btn",
                        onclick: move |_| do_open_attach(),
                        "📥 存入笔记本"
                      }
                    }
                  }
                }

                // Live status & spend pills
                div { class: "flex items-center gap-1.5 flex-wrap",
                  if reduced.total_cost_usd > 0.0 || effective_cost_limit_usd.is_some() {
                    span { "data-testid": "studio-live-spend-pill",
                      Badge {
                        variant: if reduced.cost_limit_exceeded.is_some() { BadgeVariant::Destructive } else { BadgeVariant::Outline },
                        class: "font-mono text-[11px]".to_string(),
                        "💰 {format_spend_usd(reduced.total_cost_usd)}"
                        if let Some(lim) = effective_cost_limit_usd {
                          " / {format_spend_usd(lim)}"
                        }
                      }
                    }
                  }
                  if let Some(pause) = &effective_paused {
                    Badge { variant: BadgeVariant::Default, class: "text-[11px] bg-amber-500 text-white".to_string(),
                      "⏸ 已暂停于 #{pause.node_id}"
                    }
                  } else if is_run_active {
                    Badge { variant: BadgeVariant::Default, class: "text-[11px]".to_string(),
                      "● 运行中 · {completed_count}/{insp_now.nodes.len()}"
                    }
                  } else if let Some(run) = &cur_active_run {
                    Badge { variant: BadgeVariant::Secondary, class: "text-[11px]".to_string(),
                      "{run.status} · {completed_count}/{insp_now.nodes.len()}"
                    }
                  } else {
                    Badge { variant: BadgeVariant::Outline, class: "text-[11px]".to_string(),
                      "就绪 · {insp_now.levels.len()} 层拓扑"
                    }
                  }
                }
              }

              // Pure-Rust DAG Canvas
              div { class: "flex-1 min-h-0",
                WorkflowGraphCanvas {
                  nodes: insp_now.nodes.clone(),
                  edges: insp_now.edges.clone(),
                  levels: insp_now.levels.clone(),
                  execution_order: insp_now.execution_order.clone(),
                  telemetry_by_node: reduced.telemetry_by_node.clone(),
                  breakpoints: breakpoints(),
                  selected_node_id: selected_node_id(),
                  editable: !is_run_active,
                  on_select_node: move |nid: String| selected_node_id.set(Some(nid)),
                  on_toggle_breakpoint: move |nid: String| do_toggle_breakpoint(nid),
                  on_add_node: Some(EventHandler::new(do_add_node)),
                  on_connect_nodes: Some(EventHandler::new(do_connect_nodes)),
                  on_disconnect_edge: Some(EventHandler::new(do_disconnect_edge)),
                  on_delete_node: Some(EventHandler::new(do_delete_node)),
                }
              }
            }

            if !right_collapsed() {
              ResizableHandle {
                orientation: LayoutOrientation::Horizontal,
                value: right_size(),
                min: 20.0,
                max: 48.0,
                on_resize: move |delta: f64| {
                  let c = ResizablePanelState::new(center_size(), 28.0, 72.0);
                  let r = ResizablePanelState::new(right_size(), 20.0, 48.0);
                  let (nc, nr) = resizable_resize_pair(c, r, delta);
                  center_size.set(nc.size);
                  right_size.set(nr.size);
                },
              }
            }

            // RIGHT PANE: Node Config + Live I/O Inspector
            ResizablePanel {
              size: if right_collapsed() { 0.0 } else { right_size() },
              min_size: 20.0,
              max_size: 48.0,
              collapsed: right_collapsed(),
              class: "h-full overflow-hidden".to_string(),
              NodeIoInspector {
                run_id: cur_run_id.clone(),
                api_token: api_token.clone(),
                tenant_id: tenant_id(),
                node: selected_node.clone(),
                all_nodes: insp_now.nodes.clone(),
                editable: !is_run_active,
                telemetry: selected_node
                  .as_ref()
                  .and_then(|n| reduced.telemetry_by_node.get(&n.id).cloned()),
                captured_input: selected_node
                  .as_ref()
                  .and_then(|n| reduced.node_inputs.get(&n.id).cloned()),
                captured_output: selected_node
                  .as_ref()
                  .and_then(|n| reduced.node_outputs.get(&n.id).cloned()),
                paused_at: effective_paused.clone(),
                has_breakpoint: selected_node
                  .as_ref()
                  .is_some_and(|n| breakpoints().iter().any(|b| b == &n.id)),
                on_toggle_breakpoint: move |nid: String| do_toggle_breakpoint(nid),
                on_step_debug: move |ov| do_debug_action("step", ov),
                on_continue_debug: move |ov| do_debug_action("continue", ov),
                debug_action_busy: debug_action_busy(),
                on_apply_node_patch: Some(EventHandler::new(do_apply_node_patch)),
                on_delete_node: if insp_now.nodes.len() > 1 {
                  Some(EventHandler::new(do_delete_node))
                } else {
                  None
                },
                on_expand_drawer: Some(EventHandler::new(move |_| inspector_drawer_open.set(true))),
                on_collapse_pane: Some(EventHandler::new(move |_| right_collapsed.set(true))),
              }
            }
          }
        }
      } else {
        // ── Posture 2: RELEASE MODE (One-Click App Runner) ────────────
        div { class: "flex-1 min-h-0 p-3 grid grid-cols-1 lg:grid-cols-12 gap-3 overflow-y-auto",
          if !release_form_collapsed() {
            section { class: "lg:col-span-4 rounded-lg border border-border bg-card flex flex-col overflow-hidden",
              div { class: "p-3 border-b border-border flex items-start justify-between gap-2 bg-muted/20",
                div { class: "min-w-0",
                  span { class: "text-[10px] font-semibold uppercase tracking-wider text-primary", "RELEASE APP · 一键全速运行" }
                  h2 { class: "text-sm font-bold truncate", "{insp_now.name.as_deref().unwrap_or(\"工作流应用\")}" }
                }
                Button {
                  variant: ButtonVariant::Ghost,
                  size: ButtonSize::Sm,
                  class: "h-7 px-2 text-xs shrink-0".to_string(),
                  "data-testid": "release-collapse-form-btn",
                  onclick: move |_| release_form_collapsed.set(true),
                  "收起"
                }
              }

              div { class: "flex-1 overflow-y-auto p-3 space-y-3",
                div { class: "space-y-1",
                  label { class: "text-xs font-medium block", "选择应用工作流" }
                  {
                    let presets_for_rel = all_presets.clone();
                    rsx! {
                      NativeSelect {
                        size: NativeSelectSize::Sm,
                        "data-testid": "release-preset-select",
                        on_value_change: move |v: String| {
                          if let Some(found) = presets_for_rel.iter().find(|p| p.id == v) {
                            apply_selected_preset(found.id.clone(), found.yaml.clone());
                          }
                        },
                        for p in &all_presets {
                          NativeSelectOption {
                            key: "rel-{p.id}",
                            value: p.id.clone(),
                            selected: selected_preset_id() == p.id,
                            "[{p.category}] {p.title}"
                          }
                        }
                        if selected_preset_id() == "custom" {
                          NativeSelectOption { value: "custom".to_string(), selected: true, "自定义导入 YAML" }
                        }
                      }
                    }
                  }
                }
                div { class: "space-y-1",
                  label { class: "text-xs font-medium block", "全局推理模型覆盖 (model)" }
                  Input {
                    class: "h-8 text-xs".to_string(),
                    placeholder: "例如 deepseek-chat / gpt-4o-mini（留空按节点）".to_string(),
                    value: model_override(),
                    on_value_change: move |v: String| {
                      write_storage(STORAGE_MODEL_OVERRIDE_KEY, &v);
                      model_override.set(v);
                    },
                  }
                }
                div { class: "space-y-1",
                  label { class: "text-xs font-medium block", "单次运行预算上限 USD (cost_limit_usd)" }
                  Input {
                    class: "h-8 text-xs font-mono".to_string(),
                    placeholder: "例如 1.50（留空不设额外上限）".to_string(),
                    value: cost_limit_draft(),
                    on_value_change: move |v: String| cost_limit_draft.set(v),
                  }
                }
                {render_inputs_form(&insp_now.inputs, &cur_drafts)}
              }

              div { class: "p-3 border-t border-border bg-muted/20 flex flex-wrap gap-2",
                Button {
                  variant: ButtonVariant::Primary,
                  class: "flex-1".to_string(),
                  "data-testid": "launch-release-run",
                  disabled: submit_busy() || !insp_now.diagnostics.valid,
                  onclick: move |_| do_launch_run(StudioPosture::Release),
                  if submit_busy() { "正在提交并发任务…" } else { "🚀 一键运行整个流程" }
                }
                if can_resume_run {
                  Button {
                    variant: ButtonVariant::Secondary,
                    "data-testid": "open-resume-plan-release-btn",
                    onclick: move |_| do_open_resume_plan(),
                    "↻ 从断点续跑"
                  }
                }
                if is_run_active {
                  Button {
                    variant: ButtonVariant::Destructive,
                    onclick: move |_| do_cancel_run(),
                    "终止"
                  }
                }
              }
            }
          }

          section {
            class: if release_form_collapsed() {
              "lg:col-span-12 rounded-lg border border-border bg-card flex flex-col overflow-hidden"
            } else {
              "lg:col-span-8 rounded-lg border border-border bg-card flex flex-col overflow-hidden"
            },
            // Progress Panel
            div { class: "p-4 border-b border-border space-y-3 bg-muted/10",
              div { class: "flex flex-wrap items-center justify-between gap-2",
                div {
                  strong { class: "text-sm block",
                    if is_run_active {
                      "正在全速并发执行流水线…"
                    } else if cur_active_run.as_ref().is_some_and(|r| r.status == "succeeded")
                      || (completed_count > 0 && completed_count == insp_now.nodes.len())
                    {
                      "✓ 全流程执行完成"
                    } else if reduced.workflow_error.is_some()
                      || cur_active_run.as_ref().is_some_and(|r| r.status == "failed")
                    {
                      "✕ 执行过程中发生异常"
                    } else {
                      "等待启动"
                    }
                  }
                  span { class: "text-xs text-muted-foreground",
                    "已完成 {completed_count} / {insp_now.nodes.len()} 个阶段"
                    if let Some(ms) = reduced.workflow_duration_ms {
                      {
                        let secs = format!("{:.2}", ms as f64 / 1000.0);
                        rsx! { " · 总耗时 {secs}s" }
                      }
                    }
                  }
                }
                div { class: "flex items-center gap-2 flex-wrap",
                  if release_form_collapsed() {
                    Button {
                      variant: ButtonVariant::Outline,
                      size: ButtonSize::Sm,
                      "data-testid": "release-reopen-form-btn",
                      onclick: move |_| release_form_collapsed.set(false),
                      "☰ 展开参数表单"
                    }
                  }
                  if reduced.total_cost_usd > 0.0 || effective_cost_limit_usd.is_some() {
                    Badge { variant: BadgeVariant::Outline, class: "font-mono text-xs".to_string(),
                      "💰 {format_spend_usd(reduced.total_cost_usd)}"
                      if let Some(lim) = effective_cost_limit_usd {
                        " / {format_spend_usd(lim)}"
                      }
                    }
                  }
                  span { class: "text-sm font-bold font-mono", "{progress_pct.round() as i32}%" }
                  if !cur_run_id.is_empty() {
                    Button {
                      variant: ButtonVariant::Ghost,
                      size: ButtonSize::Sm,
                      onclick: move |_| {
                        let rid = run_id.peek().clone();
                        mode.set(StudioPosture::Debug);
                        on_navigate.call(format!("/ui-dx/studio?run={}", encode_uri_component(&rid)));
                      },
                      "切至调试视图核查 I/O →"
                    }
                  }
                }
              }

              Progress { value: progress_pct as f32, max: 100.0 }

              ol { class: "flex flex-wrap gap-1.5",
                for (idx, nid) in insp_now.execution_order.iter().enumerate() {
                  {
                    let st = reduced
                      .telemetry_by_node
                      .get(nid)
                      .map(|t| t.status)
                      .unwrap_or(crate::studio_api::StudioNodeRunStatus::Idle);
                    let variant = match st {
                      crate::studio_api::StudioNodeRunStatus::Succeeded => BadgeVariant::Default,
                      crate::studio_api::StudioNodeRunStatus::Failed => BadgeVariant::Destructive,
                      crate::studio_api::StudioNodeRunStatus::Running => BadgeVariant::Secondary,
                      _ => BadgeVariant::Outline,
                    };
                    rsx! {
                      li { key: "stage-{nid}",
                        Badge { variant, class: "font-mono text-[11px]".to_string(),
                          "{idx + 1}. {nid}"
                        }
                      }
                    }
                  }
                }
              }
            }

            if let Some(err) = &reduced.workflow_error {
              div { class: "p-3 pb-0",
                Alert { variant: AlertVariant::Destructive,
                  AlertDescription { "执行失败详情：{err}" }
                }
              }
            }

            // Deliverables Showcase
            div { class: "flex-1 overflow-y-auto p-4 space-y-3",
              div { class: "flex flex-wrap items-center justify-between gap-2",
                strong { class: "text-xs font-semibold",
                  "最终成果展台 ({deliverables.len() + usize::from(final_text_summary.is_some())})"
                }
                div { class: "flex items-center gap-1.5",
                  if !cur_run_id.is_empty() && (!artifacts().is_empty() || !deliverables.is_empty()) {
                    Button {
                      variant: ButtonVariant::Secondary,
                      size: ButtonSize::Sm,
                      "data-testid": "download-artifacts-zip-release",
                      disabled: zip_export_busy(),
                      onclick: move |_| do_export_zip(),
                      if zip_export_busy() { "正在打包…" } else { "📦 一键打包下载 (.zip)" }
                    }
                    if cur_active_run.as_ref().is_some_and(|r| r.status == "succeeded") {
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        "data-testid": "open-attach-notebook-btn-release",
                        onclick: move |_| do_open_attach(),
                        "📥 存入笔记本"
                      }
                    }
                  }
                  if !cur_run_id.is_empty() {
                    span { class: "text-[11px] font-mono text-muted-foreground", "Run {short_id(&cur_run_id)}" }
                  }
                }
              }

              if deliverables.is_empty() && final_text_summary.is_none() {
                div { class: "rounded-lg border border-dashed border-border p-8 text-center text-xs text-muted-foreground",
                  "点击左侧「🚀 一键运行整个流程」，系统将并发调度全部无依赖阶段，并在此自动呈现合成好的幻灯片、视频、PDF 报告或文本讲义。"
                }
              } else {
                div { class: "space-y-3",
                  for item in deliverables {
                    AuthorizedMedia {
                      key: "{item.key}",
                      item,
                      api_token: api_token.clone(),
                      tenant_id: tenant_id(),
                    }
                  }
                  if let Some((nid, txt)) = final_text_summary {
                    article { class: "rounded-md border border-border bg-background overflow-hidden",
                      header { class: "px-3 py-2 border-b border-border bg-muted/30 text-xs font-semibold",
                        "最终讲义 / 文本输出（#{nid}）"
                      }
                      pre {
                        class: "p-3 text-xs font-mono whitespace-pre-wrap break-words max-h-96 overflow-y-auto",
                        "data-testid": "release-final-text",
                        "{txt}"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // ── Expanded Sheets & Dialogs ───────────────────────────────────
      // 1. Config Drawer (Left Sheet)
      if config_drawer_open() {
        Sheet {
          open: Some(config_drawer_open()),
          on_open_change: move |o| config_drawer_open.set(o),
          SheetOverlay {}
          SheetContent {
            side: SheetSide::Left,
            size: SheetSize::Lg,
            class: "flex flex-col gap-3 p-4 overflow-y-auto".to_string(),
            "data-testid": "studio-config-drawer",
            SheetHeader {
              SheetTitle { "工作流模板与输入参数配置" }
              SheetDescription { "{all_presets.len()} 套模板 · {insp_now.inputs.len()} 参数" }
            }
            SheetClose { "关闭" }
            {render_inputs_form(&insp_now.inputs, &cur_drafts)}
          }
        }
      }

      // 2. Inspector Drawer (Right Sheet)
      if inspector_drawer_open() {
        Sheet {
          open: Some(inspector_drawer_open()),
          on_open_change: move |o| inspector_drawer_open.set(o),
          SheetOverlay {}
          SheetContent {
            side: SheetSide::Right,
            size: SheetSize::Xl,
            class: "flex flex-col p-0 overflow-hidden".to_string(),
            "data-testid": "studio-inspector-drawer",
            NodeIoInspector {
              run_id: cur_run_id.clone(),
              api_token: api_token.clone(),
              tenant_id: tenant_id(),
              node: selected_node.clone(),
              all_nodes: insp_now.nodes.clone(),
              editable: !is_run_active,
              telemetry: selected_node
                .as_ref()
                .and_then(|n| reduced.telemetry_by_node.get(&n.id).cloned()),
              captured_input: selected_node
                .as_ref()
                .and_then(|n| reduced.node_inputs.get(&n.id).cloned()),
              captured_output: selected_node
                .as_ref()
                .and_then(|n| reduced.node_outputs.get(&n.id).cloned()),
              paused_at: effective_paused.clone(),
              has_breakpoint: selected_node
                .as_ref()
                .is_some_and(|n| breakpoints().iter().any(|b| b == &n.id)),
              on_toggle_breakpoint: move |nid: String| do_toggle_breakpoint(nid),
              on_step_debug: move |ov| do_debug_action("step", ov),
              on_continue_debug: move |ov| do_debug_action("continue", ov),
              debug_action_busy: debug_action_busy(),
              on_apply_node_patch: Some(EventHandler::new(do_apply_node_patch)),
              on_delete_node: if insp_now.nodes.len() > 1 {
                Some(EventHandler::new(do_delete_node))
              } else {
                None
              },
              in_drawer: true,
            }
          }
        }
      }

      // 3. Full-Screen YAML Editor Sheet
      if yaml_drawer_open() {
        Sheet {
          open: Some(yaml_drawer_open()),
          on_open_change: move |o| yaml_drawer_open.set(o),
          SheetOverlay {}
          SheetContent {
            side: SheetSide::Right,
            size: SheetSize::Full,
            class: "flex flex-col gap-3 p-4 overflow-y-auto".to_string(),
            "data-testid": "studio-yaml-drawer",
            SheetHeader {
              SheetTitle { "YAML 编排源码编辑器" }
              SheetDescription {
                if insp_now.diagnostics.valid {
                  "✓ 拓扑有效 · {insp_now.nodes.len()} 节点 / {insp_now.edges.len()} 边"
                } else {
                  "⚠ {insp_now.diagnostics.issues.first().cloned().unwrap_or_else(|| \"语法或依赖有误\".to_string())}"
                }
              }
            }
            SheetClose { "关闭" }
            YamlCodeEditor {
              aria_label: "放大编辑工作流 YAML".to_string(),
              value: yaml_draft(),
              rows: 28,
              on_change: move |next: String| {
                selected_preset_id.set("custom".to_string());
                yaml_draft.set(next);
              },
            }
          }
        }
      }

      // 4. Connection / Tenant / Token Dialog
      Dialog {
        open: Some(conn_dialog_open()),
        on_open_change: move |o| conn_dialog_open.set(o),
        DialogOverlay {}
        DialogContent { "data-testid": "studio-conn-dialog",
          DialogTitle { "网关与租户凭据配置" }
          DialogDescription { "设置当前工作台的租户 Tenant ID 与会话 Bearer Token。" }
          div { class: "space-y-3 py-2",
            div { class: "space-y-1",
              label { class: "text-xs font-medium block", "租户 Tenant ID" }
              Input {
                class: "h-8 text-xs".to_string(),
                value: tenant_id(),
                on_value_change: move |v: String| tenant_id.set(v),
              }
            }
            div { class: "space-y-1",
              label { class: "text-xs font-medium block", "Bearer Token（仅保存在当前会话）" }
              Input {
                r#type: "password".to_string(),
                class: "h-8 text-xs".to_string(),
                placeholder: "可选鉴权令牌".to_string(),
                value: api_token.clone(),
                on_value_change: move |v: String| on_token_change.call(v),
              }
            }
          }
          div { class: "flex justify-end",
            DialogClose { "完成" }
          }
        }
      }

      // 5. Checkpoint Resume Plan Dialog
      Dialog {
        open: Some(resume_modal_open()),
        on_open_change: move |o| resume_modal_open.set(o),
        DialogOverlay {}
        DialogContent { class: "max-w-2xl".to_string(), "data-testid": "studio-resume-dialog",
          DialogTitle { "↻ 断点续跑计划预览 (Checkpoint Resume Plan)" }
          DialogDescription { "Run {short_id(&cur_run_id)} · 复用已完成节点缓存并安全重放未决步骤" }
          div { class: "space-y-3 py-2 text-xs",
            if resume_plan_loading() {
              p { class: "text-muted-foreground", "正在加载已落盘的 Checkpoint 状态与幂等性分析…" }
            } else if let Some(err) = resume_plan_error() {
              Alert { variant: AlertVariant::Destructive, AlertDescription { "{err}" } }
            } else if let Some(plan) = resume_plan_data() {
              div { class: "grid grid-cols-3 gap-2",
                div { class: "rounded border border-border p-2",
                  span { class: "text-[11px] text-muted-foreground block", "最近已完成节点" }
                  strong { class: "font-mono", "#{plan.last_completed_node}" }
                }
                div { class: "rounded border border-border p-2",
                  span { class: "text-[11px] text-muted-foreground block", "断点保存时间" }
                  strong { "{plan.created_at}" }
                }
                div { class: "rounded border border-border p-2",
                  span { class: "text-[11px] text-muted-foreground block", "未决工具调用" }
                  strong { "共 {plan.summary.total}（跳过 {plan.summary.to_skip} · 重放 {plan.summary.to_replay} · 人工 {plan.summary.requires_manual}）" }
                }
              }
              if !plan.tool_calls.is_empty() {
                ul { class: "space-y-1 max-h-40 overflow-y-auto",
                  for tc in &plan.tool_calls {
                    li { key: "{tc.node_id}-{tc.tool_call_id}", class: "rounded border border-border p-1.5 flex items-center justify-between gap-2",
                      span { class: "font-mono", "#{tc.node_id} · {tc.tool}" }
                      Badge { variant: BadgeVariant::Outline, "{tc.decision}" }
                    }
                  }
                }
              }
              if plan.summary.requires_manual > 0 {
                Alert { variant: AlertVariant::Destructive,
                  AlertDescription { "存在 {plan.summary.requires_manual} 项非幂等外部副作用调用需人工确认，自动续跑将被拦截。" }
                }
              }
            }
          }
          div { class: "flex justify-end gap-2",
            DialogClose { "取消" }
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              "data-testid": "confirm-resume-run-btn",
              disabled: resume_plan_loading()
                || resume_plan_data().is_none()
                || resume_action_busy()
                || resume_plan_data().as_ref().is_some_and(|p| p.summary.requires_manual > 0),
              onclick: move |_| {
                let rid = run_id.peek().clone();
                let ten = tenant_id.peek().clone();
                let tok = tok_resume_confirm.clone();
                let last_node = resume_plan_data
                  .peek()
                  .as_ref()
                  .map(|p| p.last_completed_node.clone())
                  .unwrap_or_else(|| "latest".to_string());
                resume_action_busy.set(true);
                spawn(async move {
                  match resume_run(&rid, &ten, &tok).await {
                    Ok(rec) => {
                      active_run.set(Some(rec));
                      resume_modal_open.set(false);
                      banner_notice.set(Some(format!("已从断点 #{last_node} 恢复运行")));
                      watch_run(rid, tok, ten);
                    }
                    Err(err) => resume_plan_error.set(Some(format!("续跑启动失败：{err}"))),
                  }
                  resume_action_busy.set(false);
                });
              },
              if resume_action_busy() { "正在恢复…" } else { "▶ 确认从断点续跑" }
            }
          }
        }
      }
    }
  }
}
