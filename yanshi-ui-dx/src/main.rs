//! Phase 0 technical validation spike for migrating Yanshi Studio UI to
//! Dioxus 0.7 + `dioxus-shadcn` 0.7.0 (Crate Mode + Tailwind CSS v4).

mod api;

use std::rc::Rc;

use api::{
  BlobPreviewBundle, ModelConfigSummary, SseAbortHandle, SseFrame, create_blob_previews,
  fetch_models_config, resolve_api_base, resolve_embedded_token, revoke_blob_url, stream_run_sse,
  submit_smoke_run,
};
use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertStyle, AlertTitle, AlertVariant, Badge, BadgeSize, BadgeStyle,
  BadgeVariant, Button, ButtonSize, ButtonVariant, Card, CardContent, CardDescription, CardHeader,
  CardTitle, Combobox, ComboboxContent, ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList,
  Dialog, DialogClose, DialogContent, DialogDescription, DialogOverlay, DialogTitle, Input,
  InputSize, Label, LayoutOrientation, ResizableHandle, ResizablePanel, ResizablePanelGroup,
  ResizablePanelState, Separator, Sheet, SheetClose, SheetContent, SheetDescription, SheetFooter,
  SheetHeader, SheetOverlay, SheetSide, SheetTitle, Tabs, TabsContent, TabsList, TabsTrigger,
  TabsVariant, Textarea, Theme, ThemeController, resizable_resize_pair,
};

const APP_CSS: Asset = asset!("/assets/app.generated.css");

const KNOWN_MODELS: &[&str] = &[
  "gpt-4o",
  "gpt-4o-mini",
  "claude-3-7-sonnet",
  "deepseek-chat",
  "deepseek-reasoner",
  "deepseek-flash",
  "gemini-2.5-pro",
  "qwen-plus",
  "moonshot-v1-128k",
];

fn main() {
  dioxus::launch(App);
}

#[component]
fn App() -> Element {
  let mut theme = use_signal(|| Theme::System);
  let mut api_token = use_signal(resolve_embedded_token);
  let mut tenant_id = use_signal(|| String::from("default"));
  let api_base = use_hook(resolve_api_base);

  // Resizable 3-pane workbench state (left / center / right percentages)
  let mut left_size = use_signal(|| 24.0_f64);
  let mut center_size = use_signal(|| 50.0_f64);
  let mut right_size = use_signal(|| 26.0_f64);
  let mut left_collapsed = use_signal(|| false);

  // Expandable wide Sheet (simulating ModelConfigDrawer) + Dialog overlay probes
  let mut sheet_open = use_signal(|| false);
  let mut sheet_expanded = use_signal(|| false);
  let mut dialog_open = use_signal(|| false);
  let mut custom_provider_name = use_signal(|| String::from("openai-compatible"));
  let mut custom_base_url = use_signal(|| String::from("https://api.openai.com/v1"));
  let mut notes_draft = use_signal(String::new);

  // Combobox model selector probe
  let mut model_query = use_signal(String::new);
  let mut selected_model = use_signal(|| String::from("deepseek-chat"));

  // Probe 1: GET /v1/models/config
  let mut models_loading = use_signal(|| false);
  let mut models_error = use_signal(|| Option::<String>::None);
  let mut models_snapshot = use_signal(|| Option::<ModelConfigSummary>::None);

  // Probe 2: POST /v1/runs + Auth SSE streaming
  let mut sse_running = use_signal(|| false);
  let mut sse_run_id = use_signal(|| Option::<String>::None);
  let mut sse_error = use_signal(|| Option::<String>::None);
  let mut sse_frames = use_signal(Vec::<SseFrame>::new);
  let mut sse_abort = use_signal(|| Option::<Rc<SseAbortHandle>>::None);

  // Probe 3: Blob ObjectURL preview (SVG + iframe) under strict CSP
  let mut blob_bundle = use_signal(|| Option::<BlobPreviewBundle>::None);
  let mut blob_error = use_signal(|| Option::<String>::None);

  use_drop(move || {
    if let Some(bundle) = blob_bundle.peek().as_ref() {
      revoke_blob_url(&bundle.svg_url);
      revoke_blob_url(&bundle.html_url);
    }
  });

  let filtered_models: Vec<&'static str> = {
    let q = model_query().trim().to_ascii_lowercase();
    KNOWN_MODELS
      .iter()
      .copied()
      .filter(|m| q.is_empty() || m.to_ascii_lowercase().contains(&q))
      .collect()
  };

  let sheet_width_class = if sheet_expanded() {
    "flex flex-col w-[96vw] sm:max-w-[1320px] p-0"
  } else {
    "flex flex-col w-[88vw] sm:max-w-[780px] p-0"
  };

  let active_theme_label: String = match theme() {
    Theme::System => String::from("System"),
    Theme::Light => String::from("Light"),
    Theme::Dark => String::from("Dark"),
    Theme::Preset(p) => p,
  };

  rsx! {
    document::Stylesheet { href: APP_CSS }
    ThemeController {
      theme: theme(),
      on_theme_change: move |stored| theme.set(stored),
    }

    div {
      class: "flex h-screen flex-col bg-background text-foreground",
      "data-testid": "dx-phase0-root",

      // Top navigation bar
      header { class: "flex h-14 shrink-0 items-center justify-between gap-3 border-b border-border bg-card px-4",
        div { class: "flex items-center gap-2.5",
          span { class: "text-sm font-semibold tracking-tight", "Yanshi Studio · Dioxus 0.7 Spike" }
          Badge { variant: BadgeVariant::Info, style: BadgeStyle::Soft, size: BadgeSize::Sm,
            "dioxus-shadcn 0.7.0"
          }
          Badge { variant: BadgeVariant::Outline, size: BadgeSize::Sm,
            if api_base.is_empty() { "Host: Same-Origin Web" } else { "Host: Tauri Loopback" }
          }
        }

        div { class: "flex items-center gap-2",
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            "data-testid": "dx-theme-cycle-btn",
            onclick: move |_| {
              let next = match theme() {
                Theme::System => Theme::Dark,
                Theme::Dark => Theme::Light,
                _ => Theme::System,
              };
              theme.set(next);
            },
            "Theme: {active_theme_label}"
          }
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            "data-testid": "dx-open-dialog-btn",
            onclick: move |_| dialog_open.set(true),
            "Dialog Probe"
          }
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            "data-testid": "dx-open-sheet-btn",
            onclick: move |_| sheet_open.set(true),
            "Model Config Sheet"
          }
        }
      }

      // 3-Pane Resizable Workbench
      div { class: "min-h-0 flex-1 overflow-hidden",
        ResizablePanelGroup {
          orientation: LayoutOrientation::Horizontal,
          class: "h-full w-full",

          // Left pane: connection & spike controls
          ResizablePanel {
            size: left_size(),
            min_size: 16.0,
            max_size: 40.0,
            collapsed: left_collapsed(),
            class: "flex flex-col border-r border-border bg-muted/20",
            div { class: "flex items-center justify-between border-b border-border px-3 py-2",
              span { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground",
                "Auth & Context"
              }
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Xs,
                onclick: move |_| left_collapsed.set(true),
                "Collapse"
              }
            }
            div { class: "flex flex-1 flex-col gap-3 overflow-y-auto p-3 text-xs",
              div { class: "space-y-1",
                Label { "API Base" }
                div { class: "rounded border border-border bg-background px-2.5 py-1.5 font-mono text-[11px] text-muted-foreground",
                  if api_base.is_empty() { "(same-origin)" } else { "{api_base}" }
                }
              }
              div { class: "space-y-1",
                Label { "Bearer Token (optional)" }
                Input {
                  size: InputSize::Sm,
                  value: api_token(),
                  placeholder: "Uses __YANSHI_API_TOKEN__ if blank",
                  on_value_change: move |v| api_token.set(v),
                }
              }
              div { class: "space-y-1",
                Label { "Tenant ID" }
                Input {
                  size: InputSize::Sm,
                  value: tenant_id(),
                  placeholder: "default",
                  on_value_change: move |v| tenant_id.set(v),
                }
              }
              Separator {}
              div { class: "space-y-1.5",
                p { class: "font-medium text-foreground", "Pane Split Telemetry" }
                p { class: "font-mono text-[11px] text-muted-foreground",
                  "L: {left_size():.1}% · C: {center_size():.1}% · R: {right_size():.1}%"
                }
                Button {
                  variant: ButtonVariant::Secondary,
                  size: ButtonSize::Xs,
                  onclick: move |_| {
                    left_collapsed.set(false);
                    left_size.set(24.0);
                    center_size.set(50.0);
                    right_size.set(26.0);
                  },
                  "Reset 24 / 50 / 26"
                }
              }
            }
          }

          if !left_collapsed() {
            ResizableHandle {
              orientation: LayoutOrientation::Horizontal,
              value: left_size(),
              min: 16.0,
              max: 40.0,
              on_resize: move |delta| {
                let (next_left, next_center) = resizable_resize_pair(
                  ResizablePanelState::new(left_size(), 16.0, 40.0),
                  ResizablePanelState::new(center_size(), 30.0, 72.0),
                  delta,
                );
                left_size.set(next_left.size);
                center_size.set(next_center.size);
              },
            }
          }

          // Center pane: tabbed probes
          ResizablePanel {
            size: if left_collapsed() { center_size() + left_size() } else { center_size() },
            min_size: 30.0,
            max_size: 84.0,
            class: "flex flex-col overflow-hidden bg-background",
            div { class: "flex items-center justify-between border-b border-border px-4 py-2",
              div { class: "flex items-center gap-2",
                if left_collapsed() {
                  Button {
                    variant: ButtonVariant::Outline,
                    size: ButtonSize::Xs,
                    onclick: move |_| left_collapsed.set(false),
                    "Expand Left"
                  }
                }
                span { class: "text-sm font-medium", "Phase 0 Gate Probes" }
              }
              span { class: "text-xs text-muted-foreground",
                "Strict CSP: script-src 'self' 'wasm-unsafe-eval'"
              }
            }

            div { class: "flex-1 overflow-y-auto p-4",
              Tabs { default_value: "api", variant: TabsVariant::Underline,
                TabsList {
                  TabsTrigger { value: "api", "1. Auth JSON API" }
                  TabsTrigger { value: "sse", "2. Auth SSE Stream" }
                  TabsTrigger { value: "blob", "3. Blob Media & CSP" }
                  TabsTrigger { value: "components", "4. Components Stress" }
                }

                // Tab 1: GET /v1/models/config
                TabsContent { value: "api", class: "pt-4 space-y-4",
                  Card {
                    CardHeader {
                      CardTitle { "GET /v1/models/config Probe" }
                      CardDescription {
                        "Verifies authenticated JSON fetch with Bearer + X-Tenant-Id headers from Dioxus WASM."
                      }
                    }
                    CardContent { class: "space-y-3",
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        loading: models_loading(),
                        "data-testid": "dx-probe-models-btn",
                        onclick: move |_| {
                          models_loading.set(true);
                          models_error.set(None);
                          let tok = api_token();
                          let ten = tenant_id();
                          spawn(async move {
                            match fetch_models_config(&tok, &ten).await {
                              Ok(summary) => models_snapshot.set(Some(summary)),
                              Err(err) => models_error.set(Some(err)),
                            }
                            models_loading.set(false);
                          });
                        },
                        "Fetch /v1/models/config"
                      }

                      if let Some(err) = models_error() {
                        Alert { variant: AlertVariant::Destructive, style: AlertStyle::Soft,
                          AlertTitle { "API Request Failed" }
                          AlertDescription { "{err}" }
                        }
                      }

                      if let Some(snap) = models_snapshot() {
                        Alert { variant: AlertVariant::Success, style: AlertStyle::Soft,
                          AlertTitle {
                            "Loaded {snap.providers.len()} providers · {snap.models.len()} models"
                          }
                          AlertDescription { "Config dir: {snap.config_dir}" }
                        }
                        div { class: "grid gap-2 sm:grid-cols-2",
                          for provider in snap.providers.iter() {
                            div {
                              key: "{provider.name}",
                              class: "rounded-md border border-border bg-muted/30 p-2.5 text-xs",
                              div { class: "flex items-center justify-between gap-2",
                                strong { "{provider.name}" }
                                Badge {
                                  variant: if provider.has_api_key { BadgeVariant::Success } else { BadgeVariant::Secondary },
                                  style: BadgeStyle::Soft,
                                  size: BadgeSize::Xs,
                                  if provider.has_api_key { "Key Set" } else { "No Key" }
                                }
                              }
                              if let Some(url) = &provider.base_url {
                                p { class: "mt-1 truncate font-mono text-[11px] text-muted-foreground", "{url}" }
                              }
                            }
                          }
                        }
                      }
                    }
                  }
                }

                // Tab 2: POST /v1/runs + SSE streaming
                TabsContent { value: "sse", class: "pt-4 space-y-4",
                  Card {
                    CardHeader {
                      CardTitle { "Authenticated SSE Streaming Probe" }
                      CardDescription {
                        "Submits a zero-cost 2-node template workflow (`POST /v1/runs`) and streams `GET /v1/runs/:id/events` via ReadableStreamDefaultReader."
                      }
                    }
                    CardContent { class: "space-y-3",
                      div { class: "flex flex-wrap items-center gap-2",
                        Button {
                          variant: ButtonVariant::Primary,
                          size: ButtonSize::Sm,
                          loading: sse_running(),
                          "data-testid": "dx-probe-sse-btn",
                          onclick: move |_| {
                            sse_running.set(true);
                            sse_error.set(None);
                            sse_frames.set(Vec::new());
                            sse_run_id.set(None);
                            let tok = api_token();
                            let ten = tenant_id();
                            spawn(async move {
                              let run_id = match submit_smoke_run(&tok, &ten).await {
                                Ok(id) => id,
                                Err(err) => {
                                  sse_error.set(Some(err));
                                  sse_running.set(false);
                                  return;
                                }
                              };
                              sse_run_id.set(Some(run_id.clone()));
                              let handle = match SseAbortHandle::new() {
                                Ok(h) => Rc::new(h),
                                Err(err) => {
                                  sse_error.set(Some(err));
                                  sse_running.set(false);
                                  return;
                                }
                              };
                              sse_abort.set(Some(Rc::clone(&handle)));
                              let stream_res = stream_run_sse(&run_id, &tok, &ten, &handle, |frame| {
                                sse_frames.write().push(frame);
                              })
                              .await;
                              if let Err(err) = stream_res {
                                sse_error.set(Some(err));
                              }
                              sse_abort.set(None);
                              sse_running.set(false);
                            });
                          },
                          "Run & Stream SSE"
                        }
                        if sse_running() {
                          Button {
                            variant: ButtonVariant::Destructive,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                              if let Some(handle) = sse_abort.peek().as_ref() {
                                handle.abort();
                              }
                            },
                            "Abort Stream"
                          }
                        }
                        if let Some(id) = sse_run_id() {
                          Badge { variant: BadgeVariant::Info, style: BadgeStyle::Outline, size: BadgeSize::Sm,
                            "run: {id}"
                          }
                        }
                      }

                      if let Some(err) = sse_error() {
                        Alert { variant: AlertVariant::Destructive, style: AlertStyle::Soft,
                          AlertTitle { "SSE Stream Status" }
                          AlertDescription { "{err}" }
                        }
                      }

                      div {
                        class: "max-h-72 overflow-y-auto rounded-md border border-border bg-muted/30 p-3 font-mono text-xs space-y-1.5",
                        "data-testid": "dx-sse-log",
                        if sse_frames().is_empty() {
                          span { class: "text-muted-foreground", "No SSE frames received yet." }
                        } else {
                          for (idx, frame) in sse_frames().iter().enumerate() {
                            div { key: "{idx}", class: "rounded bg-background px-2 py-1 border border-border/60",
                              span { class: "font-semibold text-primary", "#{idx + 1} [{frame.event}] " }
                              span { class: "break-all text-muted-foreground", "{frame.data}" }
                            }
                          }
                        }
                      }
                    }
                  }
                }

                // Tab 3: Blob ObjectURL preview
                TabsContent { value: "blob", class: "pt-4 space-y-4",
                  Card {
                    CardHeader {
                      CardTitle { "Blob ObjectURL & Strict CSP Probe" }
                      CardDescription {
                        "Verifies `img-src blob:` and `frame-src blob:` preview lifecycle (AuthorizedMedia parity) with explicit `URL.revokeObjectURL` cleanup."
                      }
                    }
                    CardContent { class: "space-y-3",
                      div { class: "flex items-center gap-2",
                        Button {
                          variant: ButtonVariant::Primary,
                          size: ButtonSize::Sm,
                          "data-testid": "dx-create-blob-btn",
                          onclick: move |_| {
                            if let Some(prev) = blob_bundle.write().take() {
                              revoke_blob_url(&prev.svg_url);
                              revoke_blob_url(&prev.html_url);
                            }
                            match create_blob_previews() {
                              Ok(bundle) => {
                                blob_error.set(None);
                                blob_bundle.set(Some(bundle));
                              }
                              Err(err) => blob_error.set(Some(err)),
                            }
                          },
                          "Generate Blob Previews"
                        }
                        if blob_bundle().is_some() {
                          Button {
                            variant: ButtonVariant::Outline,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                              if let Some(prev) = blob_bundle.write().take() {
                                revoke_blob_url(&prev.svg_url);
                                revoke_blob_url(&prev.html_url);
                              }
                            },
                            "Revoke URLs"
                          }
                        }
                      }

                      if let Some(err) = blob_error() {
                        Alert { variant: AlertVariant::Destructive, style: AlertStyle::Soft,
                          AlertTitle { "Blob Creation Failed" }
                          AlertDescription { "{err}" }
                        }
                      }

                      if let Some(bundle) = blob_bundle() {
                        div { class: "grid gap-3 md:grid-cols-2",
                          div { class: "space-y-1.5",
                            p { class: "text-xs font-medium", "Image (`img-src blob:`)" }
                            img {
                              src: "{bundle.svg_url}",
                              alt: "Blob SVG Preview",
                              class: "w-full rounded-md border border-border",
                              "data-testid": "dx-blob-img",
                            }
                          }
                          div { class: "space-y-1.5",
                            p { class: "text-xs font-medium", "Document (`frame-src blob:`)" }
                            iframe {
                              src: "{bundle.html_url}",
                              title: "Blob HTML Preview",
                              class: "h-36 w-full rounded-md border border-border bg-white",
                              "data-testid": "dx-blob-iframe",
                            }
                          }
                        }
                      }
                    }
                  }
                }

                // Tab 4: Component gap verification
                TabsContent { value: "components", class: "pt-4 space-y-4",
                  Card {
                    CardHeader {
                      CardTitle { "Combobox & Form Ergonomics Probe" }
                      CardDescription {
                        "Tests `Combobox` keyboard/filter behavior and documents P0 upstream gaps for dioxus-ui."
                      }
                    }
                    CardContent { class: "space-y-4",
                      div { class: "max-w-md space-y-1.5",
                        Label { "Model Combobox (selected: {selected_model()})" }
                        Combobox {
                          value: Some(selected_model()),
                          on_value_change: move |v: String| {
                            selected_model.set(v.clone());
                            model_query.set(v);
                          },
                          div { class: "rounded-md border border-input bg-background",
                            ComboboxInput {
                              value: model_query(),
                              placeholder: "Filter models (e.g. deepseek, gpt-4o)...",
                              oninput: move |ev: FormEvent| model_query.set(ev.value()),
                            }
                          }
                          ComboboxContent {
                            ComboboxList {
                              if filtered_models.is_empty() {
                                ComboboxEmpty { "No matching preset model (free-solo gap)" }
                              } else {
                                for model in filtered_models.iter() {
                                  ComboboxItem { key: "{model}", value: (*model).to_string(), "{model}" }
                                }
                              }
                            }
                          }
                        }
                      }

                      div { class: "max-w-md space-y-1.5",
                        Label { "Prompt Draft (`Textarea` — note missing onkeydown/IME prop)" }
                        Textarea {
                          value: notes_draft(),
                          placeholder: "Type notes here...",
                          on_value_change: move |v| notes_draft.set(v),
                        }
                      }
                    }
                  }
                }
              }
            }
          }

          ResizableHandle {
            orientation: LayoutOrientation::Horizontal,
            value: center_size(),
            min: 30.0,
            max: 72.0,
            on_resize: move |delta| {
              let (next_center, next_right) = resizable_resize_pair(
                ResizablePanelState::new(center_size(), 30.0, 72.0),
                ResizablePanelState::new(right_size(), 16.0, 42.0),
                delta,
              );
              center_size.set(next_center.size);
              right_size.set(next_right.size);
            },
          }

          // Right pane: spike checklist
          ResizablePanel {
            size: right_size(),
            min_size: 16.0,
            max_size: 42.0,
            class: "flex flex-col border-l border-border bg-muted/20",
            div { class: "border-b border-border px-3 py-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground",
              "Upstream Feedback Queue"
            }
            div { class: "flex-1 space-y-2.5 overflow-y-auto p-3 text-xs",
              Alert { variant: AlertVariant::Info, style: AlertStyle::Soft,
                AlertTitle { "Sheet Size Override" }
                AlertDescription {
                  "Verify whether `sm:max-w-[780px]` cleanly overrides `SheetSide::Right`'s default `sm:max-w-sm` via `merge_classes`."
                }
              }
              Alert { variant: AlertVariant::Warning, style: AlertStyle::Soft,
                AlertTitle { "Observed P0 Gaps" }
                AlertDescription {
                  "1) Sheet lacks native size presets & top-layer `<dialog>`. 2) Resizable lacks snap-collapse & dblclick reset. 3) Input/Textarea lack onkeydown/IME hooks. 4) Combobox lacks free-solo custom value."
                }
              }
            }
          }
        }
      }

      // Expandable Model Config Sheet Probe
      Sheet {
        open: Some(sheet_open()),
        on_open_change: move |next| sheet_open.set(next),
        SheetOverlay {}
        SheetContent {
          side: SheetSide::Right,
          class: sheet_width_class,
          "data-testid": "dx-model-sheet",
          SheetHeader { class: "border-b border-border px-6 py-4 pr-12",
            div { class: "flex items-center justify-between gap-2",
              div {
                SheetTitle { "Model & Provider Configuration (Sheet Probe)" }
                SheetDescription {
                  "Testing wide drawer + one-click full-focus expansion with `dioxus-shadcn::Sheet`."
                }
              }
              Button {
                variant: ButtonVariant::Outline,
                size: ButtonSize::Xs,
                "data-testid": "dx-sheet-expand-btn",
                onclick: move |_| sheet_expanded.set(!sheet_expanded()),
                if sheet_expanded() { "Restore Drawer" } else { "Full Focus" }
              }
            }
          }
          div { class: "flex-1 space-y-4 overflow-y-auto p-6",
            div { class: "space-y-1.5",
              Label { "Provider Name" }
              Input {
                value: custom_provider_name(),
                on_value_change: move |v| custom_provider_name.set(v),
              }
            }
            div { class: "space-y-1.5",
              Label { "Base URL" }
              Input {
                value: custom_base_url(),
                on_value_change: move |v| custom_base_url.set(v),
              }
            }
            Alert { variant: AlertVariant::Default, style: AlertStyle::Soft,
              AlertTitle { "Active Width Mode" }
              AlertDescription {
                if sheet_expanded() {
                  "Expanded full-focus mode (`w-[96vw] sm:max-w-[1320px]`)."
                } else {
                  "Standard wide drawer mode (`w-[88vw] sm:max-w-[780px]`)."
                }
              }
            }
          }
          SheetFooter { class: "border-t border-border px-6 py-4",
            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              onclick: move |_| sheet_open.set(false),
              "Cancel"
            }
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              onclick: move |_| sheet_open.set(false),
              "Save (Mock)"
            }
          }
          SheetClose { "✕" }
        }
      }

      // Modal Dialog Probe
      Dialog {
        open: Some(dialog_open()),
        on_open_change: move |next| dialog_open.set(next),
        DialogOverlay {}
        DialogContent {
          "data-testid": "dx-modal-dialog",
          DialogTitle { "CSP & Focus Trap Dialog Probe" }
          DialogDescription {
            "Press Escape or click outside to verify modal focus management works with zero JS eval."
          }
          div { class: "flex justify-end gap-2 pt-2",
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              onclick: move |_| dialog_open.set(false),
              "Close Dialog"
            }
          }
          DialogClose { "✕" }
        }
      }
    }
  }
}
