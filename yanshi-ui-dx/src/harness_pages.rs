//! Harness Session List, Submit Form (with free-solo `Combobox` model picker),
//! and Live SSE Session Detail (with approvals, `ask_user` interrupt, cancel,
//! resume, and expandable `SheetSize::Xl` <-> `SheetSize::Full` detail viewer).

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardDescription, CardHeader, CardTitle, Combobox, ComboboxContent,
  ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, Input, InputSize, Label, NativeSelect,
  NativeSelectOption, Separator, Sheet, SheetClose, SheetContent, SheetHeader, SheetOverlay,
  SheetSide, SheetSize, SheetTitle, Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
  Textarea, TextareaSize,
};

use crate::api::{
  HarnessEvent, HarnessSession, ModelConfigItem, PendingApproval, PendingInterrupt, SseAbortHandle,
  answer_harness_interrupt, cancel_harness_session, create_harness_session,
  decide_harness_approval, encode_uri_component, extract_token_delta, get_harness_events_history,
  get_harness_interrupt, get_harness_session, is_harness_terminal, is_local_desktop_engine,
  list_harness_approvals, list_harness_sessions, pick_desktop_directory, read_storage,
  resume_harness_session, stream_sse, write_storage,
};

const KEY_HARNESS_TENANT: &str = "yanshi.ui.harness.tenant";
const KEY_HARNESS_PROFILE: &str = "yanshi.ui.harness.profile";
const KEY_HARNESS_RUNTIME: &str = "yanshi.ui.harness.runtime";
const KEY_HARNESS_MODEL: &str = "yanshi.ui.harness.model";
const KEY_HARNESS_SKILL: &str = "yanshi.ui.harness.skill";
const KEY_HARNESS_WORKSPACE: &str = "yanshi.ui.harness.workspace";
const KEY_HARNESS_PROMPT: &str = "yanshi.ui.harness.prompt";

fn session_status_variant(status: &str) -> BadgeVariant {
  match status.to_ascii_lowercase().as_str() {
    "completed" => BadgeVariant::Default,
    "failed" | "cancelled" | "canceled" => BadgeVariant::Destructive,
    _ => BadgeVariant::Secondary,
  }
}

// ── 1. Harness Session List (/ui-dx/harness/sessions) ────────────────────────

#[component]
pub fn HarnessSessionListPage(
  api_token: String,
  on_token_change: EventHandler<String>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut tenant_id = use_signal(|| read_storage(KEY_HARNESS_TENANT, "default"));
  let mut sessions = use_signal(Vec::<HarnessSession>::new);
  let mut busy = use_signal(|| false);
  let mut error = use_signal(|| None::<String>);

  let tok_effect = api_token.clone();
  use_effect(use_reactive(
    (&tenant_id(), &tok_effect),
    move |(ten, tok)| {
      busy.set(true);
      error.set(None);
      spawn(async move {
        match list_harness_sessions(&ten, &tok).await {
          Ok(list) => sessions.set(list),
          Err(err) => error.set(Some(err)),
        }
        busy.set(false);
      });
    },
  ));

  let tok_refresh = api_token.clone();

  rsx! {
    div { class: "max-w-6xl mx-auto p-5 space-y-4", "data-testid": "dx-harness-list-page",
      Card {
        CardHeader { class: "flex-row items-center justify-between gap-3".to_string(),
          div {
            CardTitle { "Harness Sessions" }
            CardDescription { "Autonomous agent sessions with tool approvals, HITL interrupts, and live SSE traces." }
          }
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            "data-testid": "harness-new-link",
            onclick: move |_| on_navigate.call("/ui-dx/harness/sessions/new".to_string()),
            "+ New session"
          }
        }
        CardContent { class: "space-y-4".to_string(),
          div { class: "flex flex-wrap items-end gap-3",
            div { class: "space-y-1 w-44",
              Label { "Tenant" }
              Input {
                size: InputSize::Sm,
                value: tenant_id(),
                placeholder: "default".to_string(),
                "data-testid": "harness-list-tenant",
                on_value_change: move |v: String| {
                  write_storage(KEY_HARNESS_TENANT, &v);
                  tenant_id.set(v);
                },
              }
            }
            div { class: "space-y-1 flex-1 min-w-[200px]",
              Label { "API token (not persisted)" }
              Input {
                size: InputSize::Sm,
                r#type: "password",
                autocomplete: "off",
                value: api_token,
                placeholder: "Bearer token".to_string(),
                "data-testid": "harness-list-token",
                on_value_change: move |v: String| on_token_change.call(v),
              }
            }
            Button {
              variant: ButtonVariant::Secondary,
              size: ButtonSize::Sm,
              loading: busy(),
              onclick: move |_| {
                let ten = tenant_id();
                let tok = tok_refresh.clone();
                busy.set(true);
                error.set(None);
                spawn(async move {
                  match list_harness_sessions(&ten, &tok).await {
                    Ok(list) => sessions.set(list),
                    Err(err) => error.set(Some(err)),
                  }
                  busy.set(false);
                });
              },
              if busy() { "Loading…" } else { "Refresh" }
            }
          }

          if let Some(err_msg) = error() {
            Alert { variant: AlertVariant::Destructive,
              AlertDescription { "{err_msg}" }
            }
          }

          Table { compact: true, striped: true,
            TableHeader {
              TableRow {
                TableHead { "Started" }
                TableHead { "Status" }
                TableHead { "Profile" }
                TableHead { "Runtime" }
                TableHead { "Model" }
                TableHead { "Prompt" }
                TableHead { "ID" }
              }
            }
            TableBody {
              if sessions().is_empty() {
                TableRow {
                  TableCell { class: "text-muted-foreground".to_string(),
                    "No sessions yet for tenant \"{tenant_id()}\"."
                  }
                  TableCell {}
                  TableCell {}
                  TableCell {}
                  TableCell {}
                  TableCell {}
                  TableCell {}
                }
              } else {
                for sess in sessions().iter() {
                  {
                    let sid = sess.id.clone();
                    let short_id: String = sid.chars().take(8).collect();
                    let ten_now = tenant_id();
                    let variant = session_status_variant(&sess.status);
                    let started = sess.started_at.clone().unwrap_or_else(|| "—".to_string());
                    rsx! {
                      TableRow {
                        key: "{sid}",
                        class: "cursor-pointer hover:bg-accent/50".to_string(),
                        TableCell { class: "font-mono text-xs".to_string(), "{started}" }
                        TableCell { Badge { variant, "{sess.status}" } }
                        TableCell { "{sess.profile}" }
                        TableCell { "{sess.runtime_kind}" }
                        TableCell { class: "font-mono text-xs".to_string(), "{sess.model}" }
                        TableCell { class: "max-w-[320px] truncate".to_string(), "{sess.user_input}" }
                        TableCell {
                          Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: "h-6 px-2 font-mono text-xs".to_string(),
                            "data-testid": "harness-list-row",
                            onclick: move |_| {
                              let t = ten_now.trim();
                              let suffix = if !t.is_empty() && t != "default" {
                                format!("?tenant={}", encode_uri_component(t))
                              } else {
                                String::new()
                              };
                              on_navigate.call(format!(
                                "/ui-dx/harness/sessions/{}{}",
                                encode_uri_component(&sid),
                                suffix
                              ));
                            },
                            "{short_id}…"
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

// ── 2. Harness Submit Form (/ui-dx/harness/sessions/new) ─────────────────────

#[component]
pub fn HarnessSubmitPage(
  api_token: String,
  on_token_change: EventHandler<String>,
  models: Vec<ModelConfigItem>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut tenant_id = use_signal(|| read_storage(KEY_HARNESS_TENANT, "default"));
  let mut profile = use_signal(|| read_storage(KEY_HARNESS_PROFILE, "local"));
  let mut runtime_kind = use_signal(|| read_storage(KEY_HARNESS_RUNTIME, "react"));
  let mut model = use_signal(|| read_storage(KEY_HARNESS_MODEL, ""));
  let mut model_open = use_signal(|| false);
  let mut skill_name = use_signal(|| read_storage(KEY_HARNESS_SKILL, ""));
  let mut workspace_root = use_signal(|| read_storage(KEY_HARNESS_WORKSPACE, "/data/workspace"));
  let mut user_input =
    use_signal(|| read_storage(KEY_HARNESS_PROMPT, "请用一句话总结当前工作区。"));
  let mut busy = use_signal(|| false);
  let mut picking_dir = use_signal(|| false);
  let mut error = use_signal(|| None::<String>);

  let can_pick = is_local_desktop_engine();
  let query_lower = model().trim().to_ascii_lowercase();
  let matching_models: Vec<ModelConfigItem> = models
    .iter()
    .filter(|m| {
      query_lower.is_empty()
        || m.name.to_ascii_lowercase().contains(&query_lower)
        || m.vendor.to_ascii_lowercase().contains(&query_lower)
    })
    .take(12)
    .cloned()
    .collect();

  let tok_pick = api_token.clone();
  let tok_submit = api_token.clone();

  rsx! {
    div { class: "max-w-5xl mx-auto p-5 space-y-4", "data-testid": "dx-harness-new-page",
      Card {
        CardHeader { class: "flex-row items-center justify-between gap-3".to_string(),
          div {
            CardTitle { "New Harness Session" }
            CardDescription { "Launch an autonomous agent loop with free-solo model selection and OS-sandboxed tools." }
          }
          Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Sm,
            onclick: move |_| on_navigate.call("/ui-dx/harness/sessions".to_string()),
            "← Sessions"
          }
        }
        CardContent { class: "space-y-4".to_string(),
          if let Some(err_msg) = error() {
            Alert { variant: AlertVariant::Destructive,
              AlertDescription { "{err_msg}" }
            }
          }

          div { class: "grid grid-cols-1 sm:grid-cols-4 gap-3",
            div { class: "space-y-1.5",
              Label { "Tenant" }
              Input {
                size: InputSize::Sm,
                value: tenant_id(),
                "data-testid": "harness-new-tenant",
                on_value_change: move |v: String| {
                  write_storage(KEY_HARNESS_TENANT, &v);
                  tenant_id.set(v);
                },
              }
            }
            div { class: "space-y-1.5",
              Label { "Profile" }
              NativeSelect {
                class: "h-8 text-xs w-full".to_string(),
                "data-testid": "harness-new-profile",
                on_value_change: move |v: String| {
                  write_storage(KEY_HARNESS_PROFILE, &v);
                  profile.set(v);
                },
                NativeSelectOption { value: "dev".to_string(), selected: profile() == "dev", "dev" }
                NativeSelectOption { value: "local".to_string(), selected: profile() == "local", "local" }
                NativeSelectOption { value: "production".to_string(), selected: profile() == "production", "production" }
              }
            }
            div { class: "space-y-1.5",
              Label { "Runtime" }
              NativeSelect {
                class: "h-8 text-xs w-full".to_string(),
                "data-testid": "harness-new-runtime",
                on_value_change: move |v: String| {
                  write_storage(KEY_HARNESS_RUNTIME, &v);
                  runtime_kind.set(v);
                },
                NativeSelectOption { value: "react".to_string(), selected: runtime_kind() == "react", "react" }
                NativeSelectOption { value: "plan_execute".to_string(), selected: runtime_kind() == "plan_execute", "plan_execute" }
                NativeSelectOption { value: "reflection".to_string(), selected: runtime_kind() == "reflection", "reflection" }
              }
            }
            div { class: "space-y-1.5",
              Label { "API token" }
              Input {
                size: InputSize::Sm,
                r#type: "password",
                autocomplete: "off",
                value: api_token,
                "data-testid": "harness-new-token",
                on_value_change: move |v: String| on_token_change.call(v),
              }
            }
          }

          div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
            div { class: "space-y-1.5",
              Label { "Model (optional · free-solo Combobox)" }
              Combobox {
                id: Some("harness-new-model-combo".to_string()),
                allow_custom_value: true,
                value: Some(model()),
                open: Some(model_open()),
                on_open_change: move |next: bool| model_open.set(next),
                on_value_change: move |chosen: String| {
                  write_storage(KEY_HARNESS_MODEL, &chosen);
                  model.set(chosen);
                },
                ComboboxInput {
                  class: "h-8 rounded-md border border-input bg-background px-2.5 text-xs".to_string(),
                  value: model(),
                  placeholder: "搜索或直接输入任意模型 ID 回车确认".to_string(),
                  "data-testid": "harness-new-model",
                  on_value_change: move |v: String| {
                    write_storage(KEY_HARNESS_MODEL, &v);
                    model.set(v);
                  },
                }
                ComboboxContent {
                  ComboboxList {
                    if matching_models.is_empty() {
                      ComboboxEmpty { "按 Enter 直接使用自定义模型名「{model()}」" }
                    } else {
                      for m in matching_models.iter() {
                        ComboboxItem { key: "{m.name}", value: m.name.clone(),
                          span { class: "font-mono text-xs", "{m.name}" }
                          span { class: "ml-2 text-[11px] text-muted-foreground", "({m.vendor})" }
                        }
                      }
                    }
                  }
                }
              }
            }

            div { class: "space-y-1.5",
              Label { "Skill (optional)" }
              Input {
                size: InputSize::Sm,
                value: skill_name(),
                placeholder: "leave blank for no skill".to_string(),
                "data-testid": "harness-new-skill",
                on_value_change: move |v: String| {
                  write_storage(KEY_HARNESS_SKILL, &v);
                  skill_name.set(v);
                },
              }
            }

            div { class: "space-y-1.5",
              Label { "Workspace root" }
              div { class: "flex items-center gap-1.5",
                Input {
                  size: InputSize::Sm,
                  value: workspace_root(),
                  placeholder: "/data/workspace".to_string(),
                  "data-testid": "harness-new-workspace",
                  on_value_change: move |v: String| {
                    write_storage(KEY_HARNESS_WORKSPACE, &v);
                    workspace_root.set(v);
                  },
                }
                if can_pick {
                  Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Sm,
                    loading: picking_dir(),
                    "data-testid": "harness-pick-workspace",
                    onclick: move |_| {
                      let tok = tok_pick.clone();
                      let init_dir = workspace_root();
                      picking_dir.set(true);
                      spawn(async move {
                        match pick_desktop_directory(&tok, &init_dir).await {
                          Ok(Some(path)) => {
                            write_storage(KEY_HARNESS_WORKSPACE, &path);
                            workspace_root.set(path);
                          }
                          Ok(None) => {}
                          Err(err) => error.set(Some(err)),
                        }
                        picking_dir.set(false);
                      });
                    },
                    "选择目录"
                  }
                }
              }
            }
          }

          div { class: "space-y-1.5",
            Label { "Prompt" }
            Textarea {
              size: TextareaSize::Lg,
              value: user_input(),
              placeholder: "Describe the task for the Harness agent…".to_string(),
              "data-testid": "harness-new-prompt",
              on_value_change: move |v: String| {
                write_storage(KEY_HARNESS_PROMPT, &v);
                user_input.set(v);
              },
            }
          }

          div { class: "flex items-center gap-3",
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              loading: busy(),
              "data-testid": "harness-new-submit",
              onclick: move |_| {
                let prompt = user_input().trim().to_string();
                let workspace = workspace_root().trim().to_string();
                if prompt.is_empty() {
                  error.set(Some("User prompt is required".to_string()));
                  return;
                }
                if workspace.is_empty() {
                  error.set(Some("Workspace root is required".to_string()));
                  return;
                }
                let ten = if tenant_id().trim().is_empty() {
                  "default".to_string()
                } else {
                  tenant_id().trim().to_string()
                };
                let mut body = serde_json::json!({
                  "user_input": prompt,
                  "workspace_root": workspace,
                  "tenant_id": ten,
                  "profile": profile(),
                  "runtime_kind": runtime_kind(),
                });
                if let Some(obj) = body.as_object_mut() {
                  if !model().trim().is_empty() {
                    obj.insert("model".to_string(), serde_json::json!(model().trim()));
                  }
                  if !skill_name().trim().is_empty() {
                    obj.insert("skill_name".to_string(), serde_json::json!(skill_name().trim()));
                  }
                }
                let tok = tok_submit.clone();
                busy.set(true);
                error.set(None);
                spawn(async move {
                  match create_harness_session(&body, &ten, &tok).await {
                    Ok(sid) => {
                      let suffix = if ten != "default" {
                        format!("?tenant={}", encode_uri_component(&ten))
                      } else {
                        String::new()
                      };
                      on_navigate.call(format!(
                        "/ui-dx/harness/sessions/{}{}",
                        encode_uri_component(&sid),
                        suffix
                      ));
                    }
                    Err(err) => error.set(Some(err)),
                  }
                  busy.set(false);
                });
              },
              if busy() { "Submitting…" } else { "Start session" }
            }
          }
        }
      }
    }
  }
}

// ── 3. Harness Session Detail (/ui-dx/harness/sessions/:id) ──────────────────

#[component]
pub fn HarnessSessionDetailPage(
  session_id: String,
  tenant: String,
  api_token: String,
  on_token_change: EventHandler<String>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut session = use_signal(|| None::<HarnessSession>);
  let mut events = use_signal(Vec::<HarnessEvent>::new);
  let mut live_typing = use_signal(String::new);
  let mut approvals = use_signal(Vec::<PendingApproval>::new);
  let mut pending_interrupt = use_signal(|| None::<PendingInterrupt>);
  let mut interrupt_answer = use_signal(String::new);
  let mut interrupt_busy = use_signal(|| false);
  let mut selected_seq = use_signal(|| None::<i64>);
  let mut stream_state = use_signal(|| "idle".to_string());
  let mut error = use_signal(|| None::<String>);
  let mut info = use_signal(|| None::<String>);
  let mut resume_prompt = use_signal(String::new);
  let mut resume_mode = use_signal(|| "rerun".to_string());
  let mut resume_busy = use_signal(|| false);

  // Expandable Sheet viewer for long answer / event payload
  let mut detail_sheet_open = use_signal(|| false);
  let mut detail_sheet_full = use_signal(|| false);
  let mut detail_sheet_title = use_signal(String::new);
  let mut detail_sheet_body = use_signal(String::new);

  let sid_eff = session_id.clone();
  let ten_eff = tenant.clone();
  let tok_eff = api_token.clone();
  use_effect(use_reactive(
    (&sid_eff, &ten_eff, &tok_eff),
    move |(sid, ten, tok)| {
      let sid_a = sid.clone();
      let ten_a = ten.clone();
      let tok_a = tok.clone();
      spawn(async move {
        if let Ok(s) = get_harness_session(&sid_a, &ten_a, &tok_a).await {
          session.set(Some(s));
        }
        if let Ok(hist) = get_harness_events_history(&sid_a, &ten_a, &tok_a).await {
          let mut sorted = hist;
          sorted.sort_by_key(|e| e.seq);
          events.set(sorted);
        }
        if let Ok(apps) = list_harness_approvals(&sid_a, &ten_a, &tok_a).await {
          approvals.set(apps);
        }
        if let Ok(intr) = get_harness_interrupt(&sid_a, &ten_a, &tok_a).await {
          pending_interrupt.set(intr);
        }
      });

      stream_state.set("streaming".to_string());
      spawn(async move {
        let Ok(abort_handle) = SseAbortHandle::new() else {
          stream_state.set("error".to_string());
          return;
        };
        let path = format!("/v1/harness/sessions/{}/events", encode_uri_component(&sid));
        let res = stream_sse(&path, &tok, &ten, &abort_handle, |frame| {
          if let Ok(ev) = serde_json::from_str::<HarnessEvent>(&frame.data) {
            if ev.kind == "token_delta" {
              if let Some(delta) = extract_token_delta(&ev.payload) {
                live_typing.write().push_str(&delta);
              }
              return;
            }
            if ev.kind == "step_started" {
              live_typing.set(String::new());
            }
            let mut list = events.write();
            if !list.iter().any(|existing| existing.seq == ev.seq) {
              list.push(ev);
              list.sort_by_key(|e| e.seq);
            }
          }
        })
        .await;
        match res {
          Ok(_) => stream_state.set("closed".to_string()),
          Err(_) => stream_state.set("error".to_string()),
        }
      });
    },
  ));

  let sess_opt = session();
  let terminal = sess_opt
    .as_ref()
    .is_some_and(|s| is_harness_terminal(&s.status));
  let selected_event = selected_seq().and_then(|seq| events().into_iter().find(|e| e.seq == seq));
  let short_id: String = session_id.chars().take(8).collect();

  let sid_refresh = session_id.clone();
  let ten_refresh = tenant.clone();
  let tok_refresh = api_token.clone();
  let sid_cancel = session_id.clone();
  let ten_cancel = tenant.clone();
  let tok_cancel = api_token.clone();
  let sid_resume = session_id.clone();
  let ten_resume = tenant.clone();
  let tok_resume = api_token.clone();
  let sid_intr = session_id.clone();
  let ten_intr = tenant.clone();
  let tok_intr = api_token.clone();

  rsx! {
    div { class: "max-w-6xl mx-auto p-5 space-y-4", "data-testid": "dx-harness-detail-page",
      div { class: "flex flex-wrap items-center justify-between gap-3",
        div { class: "flex items-center gap-2",
          h2 { class: "text-lg font-semibold", "Harness Session {short_id}…" }
          span { "data-testid": "harness-detail-stream-state",
            Badge { variant: BadgeVariant::Outline,
              "stream: {stream_state()}"
            }
          }
        }
        div { class: "flex items-center gap-2",
          Button {
            variant: ButtonVariant::Ghost,
            size: ButtonSize::Sm,
            onclick: move |_| on_navigate.call("/ui-dx/harness/sessions".to_string()),
            "← Sessions"
          }
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            onclick: move |_| {
              let sid = sid_refresh.clone();
              let ten = ten_refresh.clone();
              let tok = tok_refresh.clone();
              spawn(async move {
                if let Ok(s) = get_harness_session(&sid, &ten, &tok).await {
                  session.set(Some(s));
                }
                if let Ok(apps) = list_harness_approvals(&sid, &ten, &tok).await {
                  approvals.set(apps);
                }
                if let Ok(intr) = get_harness_interrupt(&sid, &ten, &tok).await {
                  pending_interrupt.set(intr);
                }
              });
            },
            "Refresh"
          }
          Button {
            variant: ButtonVariant::Destructive,
            size: ButtonSize::Sm,
            disabled: terminal,
            "data-testid": "harness-detail-cancel",
            onclick: move |_| {
              let sid = sid_cancel.clone();
              let ten = ten_cancel.clone();
              let tok = tok_cancel.clone();
              spawn(async move {
                match cancel_harness_session(&sid, &ten, &tok).await {
                  Ok(()) => {
                    info.set(Some("Cancel requested".to_string()));
                    if let Ok(s) = get_harness_session(&sid, &ten, &tok).await {
                      session.set(Some(s));
                    }
                  }
                  Err(err) => error.set(Some(err)),
                }
              });
            },
            if terminal { "Terminal" } else { "Cancel session" }
          }
        }
      }

      if let Some(err_msg) = error() {
        Alert { variant: AlertVariant::Destructive, AlertDescription { "{err_msg}" } }
      }
      if let Some(info_msg) = info() {
        Alert { variant: AlertVariant::Default, AlertDescription { "{info_msg}" } }
      }

      // Resume & Token controls
      Card {
        CardContent { class: "pt-4 flex flex-wrap items-end gap-3".to_string(),
          div { class: "space-y-1 flex-1 min-w-[240px]",
            Label { "Resume prompt (optional — empty replays original)" }
            Input {
              size: InputSize::Sm,
              value: resume_prompt(),
              disabled: !terminal || resume_busy(),
              placeholder: "Leave blank to rerun with the original prompt".to_string(),
              "data-testid": "harness-detail-resume-prompt",
              on_value_change: move |v: String| resume_prompt.set(v),
            }
          }
          div { class: "space-y-1 w-44",
            Label { "Mode" }
            NativeSelect {
              class: "h-8 text-xs w-full".to_string(),
              disabled: !terminal || resume_busy(),
              "data-testid": "harness-detail-resume-mode",
              on_value_change: move |v: String| resume_mode.set(v),
              NativeSelectOption { value: "rerun".to_string(), selected: resume_mode() == "rerun", "rerun (reset events)" }
              NativeSelectOption { value: "append".to_string(), selected: resume_mode() == "append", "append (continue seq)" }
            }
          }
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            disabled: !terminal || resume_busy(),
            loading: resume_busy(),
            "data-testid": "harness-detail-resume",
            onclick: move |_| {
              let sid = sid_resume.clone();
              let ten = ten_resume.clone();
              let tok = tok_resume.clone();
              let mode = resume_mode();
              let prompt = resume_prompt();
              resume_busy.set(true);
              spawn(async move {
                match resume_harness_session(&sid, &mode, Some(&prompt), &ten, &tok).await {
                  Ok(()) => {
                    if mode == "rerun" {
                      events.set(Vec::new());
                      selected_seq.set(None);
                      info.set(Some("Resume (rerun) — events reset, executor restarted.".to_string()));
                    } else {
                      info.set(Some("Resume (append) — prior events preserved.".to_string()));
                    }
                    resume_prompt.set(String::new());
                    if let Ok(s) = get_harness_session(&sid, &ten, &tok).await {
                      session.set(Some(s));
                    }
                  }
                  Err(err) => error.set(Some(err)),
                }
                resume_busy.set(false);
              });
            },
            "Resume ({resume_mode()})"
          }
          div { class: "space-y-1 w-48",
            Label { "API token" }
            Input {
              size: InputSize::Sm,
              r#type: "password",
              value: api_token.clone(),
              on_value_change: move |v: String| on_token_change.call(v),
            }
          }
        }
      }

      // Pending HITL Interrupt card
      if let Some(ref intr) = pending_interrupt() {
        div { "data-testid": "harness-interrupt-section",
          Card { class: "border-primary/50".to_string(),
            CardHeader {
              CardTitle { "❓ Pending Question (step #{intr.step_index})" }
              CardDescription { "{intr.question}" }
            }
            CardContent { class: "space-y-2".to_string(),
              Textarea {
                size: TextareaSize::Sm,
                value: interrupt_answer(),
                placeholder: "Type your answer…".to_string(),
                disabled: interrupt_busy(),
                "data-testid": "harness-interrupt-answer",
                on_value_change: move |v: String| interrupt_answer.set(v),
              }
              Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                loading: interrupt_busy(),
                disabled: interrupt_busy() || interrupt_answer().trim().is_empty(),
                "data-testid": "harness-interrupt-submit",
                onclick: move |_| {
                  let sid = sid_intr.clone();
                  let ten = ten_intr.clone();
                  let tok = tok_intr.clone();
                  let ans = interrupt_answer().trim().to_string();
                  interrupt_busy.set(true);
                  spawn(async move {
                    match answer_harness_interrupt(&sid, &ans, &ten, &tok).await {
                      Ok(()) => {
                        info.set(Some("Answer submitted — session resuming.".to_string()));
                        pending_interrupt.set(None);
                        interrupt_answer.set(String::new());
                      }
                      Err(err) => error.set(Some(err)),
                    }
                    interrupt_busy.set(false);
                  });
                },
                "Submit answer"
              }
            }
          }
        }
      }

      // Summary + Approvals
      div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
        Card {
          CardHeader { CardTitle { "Summary" } }
          CardContent { class: "space-y-2 text-xs".to_string(),
            if let Some(ref s) = sess_opt {
              div { class: "grid grid-cols-[100px_1fr] gap-y-1.5",
                span { class: "text-muted-foreground", "Status" }
                span { Badge { variant: session_status_variant(&s.status), "{s.status}" } }
                span { class: "text-muted-foreground", "Tenant" }
                span { "{s.tenant_id}" }
                span { class: "text-muted-foreground", "Profile" }
                span { "{s.profile}" }
                span { class: "text-muted-foreground", "Runtime" }
                span { "{s.runtime_kind}" }
                span { class: "text-muted-foreground", "Model" }
                span { class: "font-mono", "{s.model}" }
                span { class: "text-muted-foreground", "Workspace" }
                span { class: "font-mono break-all", "{s.workspace_root}" }
              }
              if let Some(ref ans) = s.final_answer {
                {
                  let ans_clone = ans.clone();
                  rsx! {
                    Separator {}
                    div { class: "flex items-center justify-between",
                      span { class: "font-semibold", "Final answer" }
                      Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Sm,
                        class: "h-6 text-xs".to_string(),
                        "data-testid": "harness-expand-answer",
                        onclick: move |_| {
                          detail_sheet_title.set("Final answer".to_string());
                          detail_sheet_body.set(ans_clone.clone());
                          detail_sheet_open.set(true);
                        },
                        "⤢ 放大"
                      }
                    }
                    pre { class: "rounded-md bg-muted/40 p-2.5 whitespace-pre-wrap max-h-40 overflow-auto font-mono text-xs",
                      "{ans}"
                    }
                  }
                }
              }
            } else {
              p { class: "text-muted-foreground", "Loading session…" }
            }
          }
        }

        div { "data-testid": "harness-approvals-section",
          Card {
            CardHeader { CardTitle { "Pending Approvals ({approvals().len()})" } }
            CardContent { class: "space-y-2".to_string(),
              if approvals().is_empty() {
                p { class: "text-xs text-muted-foreground", "No approvals waiting for this session." }
              } else {
                for app in approvals().iter() {
                  {
                    let req_id = app.id.clone();
                    let sid = session_id.clone();
                    let ten = tenant.clone();
                    let tok = api_token.clone();
                    let params_pretty = serde_json::to_string_pretty(&app.params_summary)
                      .unwrap_or_else(|_| app.params_summary.to_string());
                    rsx! {
                      div {
                        key: "{req_id}",
                        class: "rounded-md border border-border p-3 space-y-2",
                        "data-testid": "harness-approval-card",
                        div { class: "flex items-center justify-between gap-2",
                          strong { class: "text-sm font-mono", "{app.tool}" }
                          Badge { variant: BadgeVariant::Destructive, "{app.risk}" }
                        }
                        p { class: "text-xs text-muted-foreground", "{app.reason}" }
                        pre { class: "rounded bg-muted/40 p-2 text-[11px] font-mono overflow-auto max-h-24",
                          "{params_pretty}"
                        }
                        div { class: "flex items-center gap-2",
                          for (decision, label, variant) in [
                            ("allow", "Allow", ButtonVariant::Primary),
                            ("deny", "Deny", ButtonVariant::Secondary),
                            ("deny_and_stop", "Deny & Stop", ButtonVariant::Destructive),
                          ] {
                            {
                              let req_id = req_id.clone();
                              let sid = sid.clone();
                              let ten = ten.clone();
                              let tok = tok.clone();
                              rsx! {
                                Button {
                                  key: "{decision}",
                                  variant,
                                  size: ButtonSize::Sm,
                                  "data-testid": "harness-approval-{decision}",
                                  onclick: move |_| {
                                    let req_id = req_id.clone();
                                    let sid = sid.clone();
                                    let ten = ten.clone();
                                    let tok = tok.clone();
                                    spawn(async move {
                                      match decide_harness_approval(&sid, &req_id, decision, "once", &ten, &tok).await {
                                        Ok(()) => {
                                          info.set(Some(format!("Approval {req_id} → {decision}/once")));
                                          if let Ok(apps) = list_harness_approvals(&sid, &ten, &tok).await {
                                            approvals.set(apps);
                                          }
                                        }
                                        Err(err) => error.set(Some(err)),
                                      }
                                    });
                                  },
                                  "{label}"
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

      // Timeline + Event Payload
      div { class: "grid grid-cols-1 lg:grid-cols-2 gap-4",
        Card {
          CardHeader { CardTitle { "Timeline ({events().len()})" } }
          CardContent { class: "space-y-2".to_string(),
            if !live_typing().is_empty() {
              div { class: "rounded-md border border-primary/40 bg-primary/5 p-2 text-xs font-mono whitespace-pre-wrap",
                "data-testid": "harness-live-typing",
                "💭 {live_typing()}"
              }
            }
            if events().is_empty() {
              p { class: "text-xs text-muted-foreground", "No events yet." }
            } else {
              div { class: "space-y-1 max-h-80 overflow-y-auto", "data-testid": "harness-event-list",
                for ev in events().iter() {
                  {
                    let seq = ev.seq;
                    let selected = selected_seq() == Some(seq);
                    let cls = if selected {
                      "w-full text-left rounded-md px-2.5 py-1.5 border border-primary bg-accent/60 font-mono text-xs flex items-center justify-between gap-2"
                    } else {
                      "w-full text-left rounded-md px-2.5 py-1.5 border border-border/60 hover:bg-muted/60 font-mono text-xs flex items-center justify-between gap-2"
                    };
                    let ts = ev.ts.clone().unwrap_or_default();
                    rsx! {
                      button {
                        key: "{seq}",
                        r#type: "button",
                        class: cls,
                        onclick: move |_| selected_seq.set(Some(seq)),
                        span { "#{seq} · {ev.kind}" }
                        span { class: "text-[11px] text-muted-foreground", "{ts}" }
                      }
                    }
                  }
                }
              }
            }
          }
        }

        Card {
          CardHeader { class: "flex-row items-center justify-between gap-2".to_string(),
            CardTitle { "Event Payload" }
            if let Some(ref ev) = selected_event {
              {
                let title = format!("Event #{} · {}", ev.seq, ev.kind);
                let body = serde_json::to_string_pretty(&ev.payload)
                  .unwrap_or_else(|_| ev.payload.to_string());
                rsx! {
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    "data-testid": "harness-expand-payload",
                    onclick: move |_| {
                      detail_sheet_title.set(title.clone());
                      detail_sheet_body.set(body.clone());
                      detail_sheet_open.set(true);
                    },
                    "⤢ 放大"
                  }
                }
              }
            }
          }
          CardContent {
            if let Some(ref ev) = selected_event {
              {
                let body = serde_json::to_string_pretty(&ev.payload)
                  .unwrap_or_else(|_| ev.payload.to_string());
                rsx! {
                  pre { class: "rounded-md bg-muted/40 p-3 font-mono text-xs overflow-auto max-h-80 whitespace-pre-wrap",
                    "{body}"
                  }
                }
              }
            } else {
              p { class: "text-xs text-muted-foreground", "Select an event from the timeline." }
            }
          }
        }
      }

      Sheet {
        open: Some(detail_sheet_open()),
        on_open_change: move |next: bool| detail_sheet_open.set(next),
        SheetOverlay {}
        SheetContent {
          side: SheetSide::Right,
          size: if detail_sheet_full() { SheetSize::Full } else { SheetSize::Xl },
          class: "flex flex-col gap-3 overflow-hidden".to_string(),
          "data-testid": "harness-detail-drawer",
          SheetHeader { class: "flex-row items-center justify-between pr-8".to_string(),
            SheetTitle { "{detail_sheet_title()}" }
            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              onclick: move |_| detail_sheet_full.set(!detail_sheet_full()),
              if detail_sheet_full() { "⤡ 抽屉宽度" } else { "⤢ 全宽模式" }
            }
            SheetClose { "✕" }
          }
          pre { class: "flex-1 overflow-auto rounded-md border border-border bg-muted/30 p-3 font-mono text-xs whitespace-pre-wrap",
            "{detail_sheet_body()}"
          }
        }
      }
    }
  }
}
