//! `/ui-dx/runs/new` — Run creation form with pure-Rust `YamlCodeEditor`,
//! `localStorage` draft persistence (never saving API token), and live
//! authenticated SSE trace playback right after submission.

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardDescription, CardHeader, CardTitle, Input, InputSize, Label, NativeSelect,
  NativeSelectOption, Textarea, TextareaSize,
};

use crate::api::{SseAbortHandle, SseFrame, read_storage, stream_sse, submit_run, write_storage};
use crate::yaml_editor::{YamlCodeEditor, validate_workflow_yaml};

const STARTER_WORKFLOW: &str = "name: my-new-run\nnodes:\n  - id: greet\n    type: template\n    parameters:\n      template: \"hello {{ name }}\"";
const STARTER_INPUTS: &str = "{\n  \"name\": \"world\"\n}";

const KEY_TENANT: &str = "yanshi.ui.newRun.tenant";
const KEY_PROFILE: &str = "yanshi.ui.newRun.profile";
const KEY_WORKFLOW: &str = "yanshi.ui.newRun.workflow";
const KEY_INPUTS: &str = "yanshi.ui.newRun.inputs";

#[component]
pub fn RunCreatePage(api_token: String, on_token_change: EventHandler<String>) -> Element {
  let mut tenant_id = use_signal(|| read_storage(KEY_TENANT, "default"));
  let mut profile = use_signal(|| {
    let saved = read_storage(KEY_PROFILE, "local");
    if matches!(saved.as_str(), "dev" | "local" | "production") {
      saved
    } else {
      "local".to_string()
    }
  });
  let mut workflow_yaml = use_signal(|| read_storage(KEY_WORKFLOW, STARTER_WORKFLOW));
  let mut inputs_json = use_signal(|| read_storage(KEY_INPUTS, STARTER_INPUTS));
  let mut busy = use_signal(|| false);
  let mut error = use_signal(|| None::<String>);
  let mut info = use_signal(|| None::<String>);
  let mut last_run_id = use_signal(|| None::<String>);
  let mut sse_frames = use_signal(Vec::<SseFrame>::new);
  let mut sse_status = use_signal(|| "idle".to_string());

  let yaml_check = validate_workflow_yaml(&workflow_yaml());
  let inputs_check: Result<Option<serde_json::Value>, String> = {
    let raw = inputs_json();
    let trimmed = raw.trim();
    if trimmed.is_empty() {
      Ok(None)
    } else {
      serde_json::from_str::<serde_json::Value>(trimmed)
        .map(Some)
        .map_err(|err| err.to_string())
    }
  };
  let inputs_ok = inputs_check.is_ok();
  let inputs_msg = match &inputs_check {
    Ok(_) => "valid JSON or empty".to_string(),
    Err(err) => err.clone(),
  };

  let tok_submit = api_token.clone();

  rsx! {
    div { class: "max-w-5xl mx-auto p-5 space-y-4", "data-testid": "dx-run-create-page",
      Card {
        CardHeader {
          CardTitle { "Submit a Workflow Run" }
          CardDescription {
            "Author or paste workflow YAML + JSON inputs. Drafts persist in localStorage; the API token is never persisted."
          }
        }
        CardContent { class: "space-y-4".to_string(),
          if let Some(err_msg) = error() {
            Alert { variant: AlertVariant::Destructive,
              AlertDescription { "{err_msg}" }
            }
          }
          if let Some(info_msg) = info() {
            Alert { variant: AlertVariant::Default,
              AlertDescription { "{info_msg}" }
            }
          }

          div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
            div { class: "space-y-1.5",
              Label { "Tenant" }
              Input {
                size: InputSize::Sm,
                value: tenant_id(),
                placeholder: "default".to_string(),
                "data-testid": "create-tenant",
                on_value_change: move |v: String| {
                  write_storage(KEY_TENANT, &v);
                  tenant_id.set(v);
                },
              }
            }
            div { class: "space-y-1.5",
              Label { "Profile" }
              NativeSelect {
                class: "h-8 text-xs w-full".to_string(),
                "data-testid": "create-profile",
                on_value_change: move |v: String| {
                  write_storage(KEY_PROFILE, &v);
                  profile.set(v);
                },
                NativeSelectOption { value: "dev".to_string(), selected: profile() == "dev", "dev" }
                NativeSelectOption { value: "local".to_string(), selected: profile() == "local", "local" }
                NativeSelectOption { value: "production".to_string(), selected: profile() == "production", "production" }
              }
            }
            div { class: "space-y-1.5",
              Label { "API token (not persisted)" }
              Input {
                size: InputSize::Sm,
                r#type: "password",
                autocomplete: "off",
                value: api_token,
                placeholder: "Bearer token".to_string(),
                "data-testid": "create-token",
                on_value_change: move |v: String| on_token_change.call(v),
              }
            }
          }

          div { class: "space-y-1.5",
            div { class: "flex items-center justify-between gap-2",
              Label { "Workflow YAML" }
              Badge {
                variant: if yaml_check.ok { BadgeVariant::Secondary } else { BadgeVariant::Destructive },
                "{yaml_check.message}"
              }
            }
            YamlCodeEditor {
              value: workflow_yaml(),
              test_id: "create-workflow".to_string(),
              aria_label: "Workflow YAML".to_string(),
              rows: 14,
              on_change: move |next: String| {
                write_storage(KEY_WORKFLOW, &next);
                workflow_yaml.set(next);
              },
            }
          }

          div { class: "space-y-1.5",
            div { class: "flex items-center justify-between gap-2",
              Label { "Inputs (JSON, optional)" }
              Badge {
                variant: if inputs_ok { BadgeVariant::Secondary } else { BadgeVariant::Destructive },
                "{inputs_msg}"
              }
            }
            Textarea {
              size: TextareaSize::Md,
              class: "font-mono text-xs".to_string(),
              value: inputs_json(),
              placeholder: "{ \"name\": \"world\" }".to_string(),
              "data-testid": "create-inputs",
              on_value_change: move |v: String| {
                write_storage(KEY_INPUTS, &v);
                inputs_json.set(v);
              },
            }
          }

          div { class: "flex flex-wrap items-center gap-3",
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              loading: busy(),
              disabled: busy() || !yaml_check.ok || !inputs_ok,
              "data-testid": "create-submit",
              onclick: move |_| {
                let parsed_inputs = match serde_json::from_str::<serde_json::Value>(inputs_json().trim()) {
                  Ok(val) => Some(val),
                  Err(_) if inputs_json().trim().is_empty() => None,
                  Err(err) => {
                    error.set(Some(format!("inputs JSON is invalid: {err}")));
                    return;
                  }
                };
                let tenant = tenant_id();
                let prof = profile();
                let yaml = workflow_yaml();
                let tok = tok_submit.clone();
                busy.set(true);
                error.set(None);
                info.set(None);
                sse_frames.set(Vec::new());
                sse_status.set("submitting".to_string());
                spawn(async move {
                  match submit_run(&tenant, &prof, &yaml, parsed_inputs, &tok).await {
                    Ok(run_id) => {
                      last_run_id.set(Some(run_id.clone()));
                      info.set(Some(format!("Submitted run {run_id} — streaming live SSE events…")));
                      sse_status.set("streaming".to_string());
                      if let Ok(abort_handle) = SseAbortHandle::new() {
                        let path = format!("/v1/runs/{run_id}/events");
                        let res = stream_sse(&path, &tok, &tenant, &abort_handle, |frame| {
                          sse_frames.write().push(frame);
                        })
                        .await;
                        match res {
                          Ok(n) => sse_status.set(format!("done ({n} frames)")),
                          Err(err) => sse_status.set(format!("stream error: {err}")),
                        }
                      }
                    }
                    Err(err) => {
                      error.set(Some(err));
                      sse_status.set("error".to_string());
                    }
                  }
                  busy.set(false);
                });
              },
              if busy() { "Submitting…" } else { "Submit run" }
            }
            if let Some(ref rid) = last_run_id() {
              span { "data-testid": "create-run-id",
                Badge { variant: BadgeVariant::Outline,
                  "run_id: {rid} · {sse_status()}"
                }
              }
            }
          }

          if !sse_frames().is_empty() {
            div { class: "rounded-md border border-border bg-muted/30 p-3 space-y-1.5 max-h-64 overflow-y-auto",
              "data-testid": "create-run-sse-log",
              for (idx, frame) in sse_frames().iter().enumerate() {
                div { key: "{idx}", class: "font-mono text-xs flex gap-2",
                  Badge { variant: BadgeVariant::Secondary, "{frame.event}" }
                  span { class: "break-all text-foreground", "{frame.data}" }
                }
              }
            }
          }
        }
      }
    }
  }
}
