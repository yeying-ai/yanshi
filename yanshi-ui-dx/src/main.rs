//! Yanshi Studio Dioxus 0.7 + `dioxus-shadcn 0.8` frontend.
//!
//! Routes served under `/ui/*` (with `/ui-dx/*` prefix compatibility):
//! - `/ui` or `/ui/studio` — Workflow Studio (Debug Mode, pure-Rust DAG canvas)
//! - `/ui/app` — Workflow Studio (Release Mode, one-click app runner)
//! - `/ui/notebooks` and `/ui/notebooks/:id` — Three-pane Notebook workspace
//! - `/ui/console` — Classic Event Run Console
//! - `/ui/runs/:id/compare` — Side-by-side Trace Comparison
//! - `/ui/runs/new` — Workflow Run submission + Rust `YamlCodeEditor` + live SSE
//! - `/ui/harness/sessions` / `new` / `:id` — Harness sessions, approvals & HITL
//! - `/ui/diagnostics` — System Diagnostics panel
//! - `/ui/spike` — Component & strict-CSP verification lab

mod api;
mod console_compare_pages;
mod dag_canvas;
mod deliverable_viewer;
mod diagnostics_page;
mod harness_pages;
mod model_config_drawer;
mod node_inspector;
mod notebook_api;
mod notebook_page;
mod run_create_page;
mod studio_api;
mod studio_page;
mod studio_yaml;
mod yaml_editor;

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardDescription, CardHeader, CardTitle, Combobox, ComboboxContent,
  ComboboxEmpty, ComboboxInput, ComboboxItem, ComboboxList, Dialog, DialogClose, DialogContent,
  DialogDescription, DialogOverlay, DialogTitle, DialogTrigger, LayoutOrientation, ResizableHandle,
  ResizablePanel, ResizablePanelGroup, ResizablePanelState, Separator, Theme, ThemeController,
  resizable_resize_pair,
};

use api::{
  BlobPreviewBundle, ModelConfigItem, ModelConfigSnapshot, create_blob_previews,
  current_location_path_and_search, fetch_models_config, push_browser_route, resolve_api_base,
  resolve_embedded_token, revoke_blob_url,
};
use console_compare_pages::{RunComparePage, RunConsolePage};
use diagnostics_page::DiagnosticsPage;
use harness_pages::{HarnessSessionDetailPage, HarnessSessionListPage, HarnessSubmitPage};
use model_config_drawer::ModelConfigDrawer;
use notebook_page::NotebookWorkspacePage;
use run_create_page::RunCreatePage;
use studio_page::{StudioPosture, WorkflowStudioPage};

const APP_CSS: Asset = asset!("/assets/app.generated.css");

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppRoute {
  Studio {
    mode: StudioPosture,
    preset: Option<String>,
    run_id: Option<String>,
  },
  Notebooks {
    notebook_id: Option<String>,
  },
  Console {
    run_id: Option<String>,
  },
  RunCompare {
    primary_run_id: String,
    against: Option<String>,
    tenant: String,
  },
  Diagnostics,
  RunCreate,
  HarnessList,
  HarnessNew,
  HarnessDetail {
    session_id: String,
    tenant: String,
  },
  SpikeLab,
}

fn parse_route(path: &str, search: &str) -> AppRoute {
  let trimmed = path.trim_end_matches('/');
  let rel = trimmed
    .strip_prefix("/ui-dx")
    .or_else(|| trimmed.strip_prefix("/ui"))
    .unwrap_or(trimmed)
    .trim_start_matches('/');
  match rel {
    "" | "studio" => AppRoute::Studio {
      mode: StudioPosture::Debug,
      preset: extract_query_param(search, "preset").filter(|s| !s.trim().is_empty()),
      run_id: extract_query_param(search, "run_id")
        .or_else(|| extract_query_param(search, "run"))
        .filter(|s| !s.trim().is_empty()),
    },
    "app" => AppRoute::Studio {
      mode: StudioPosture::Release,
      preset: extract_query_param(search, "preset").filter(|s| !s.trim().is_empty()),
      run_id: extract_query_param(search, "run_id")
        .or_else(|| extract_query_param(search, "run"))
        .filter(|s| !s.trim().is_empty()),
    },
    "notebooks" => AppRoute::Notebooks { notebook_id: None },
    "console" => AppRoute::Console {
      run_id: extract_query_param(search, "run").filter(|s| !s.trim().is_empty()),
    },
    "diagnostics" => AppRoute::Diagnostics,
    "runs/new" => AppRoute::RunCreate,
    "harness/sessions" => AppRoute::HarnessList,
    "harness/sessions/new" => AppRoute::HarnessNew,
    "spike" => AppRoute::SpikeLab,
    other => {
      if let Some(nb_id) = other.strip_prefix("notebooks/") {
        AppRoute::Notebooks {
          notebook_id: (!nb_id.trim().is_empty()).then(|| nb_id.to_string()),
        }
      } else if let Some(rest) = other.strip_prefix("runs/")
        && let Some(prim) = rest.strip_suffix("/compare")
        && !prim.is_empty()
        && !prim.contains('/')
      {
        AppRoute::RunCompare {
          primary_run_id: prim.to_string(),
          against: extract_query_param(search, "against").filter(|s| !s.trim().is_empty()),
          tenant: extract_query_param(search, "tenant")
            .filter(|t| !t.trim().is_empty())
            .unwrap_or_else(|| "default".to_string()),
        }
      } else if let Some(sid) = other.strip_prefix("harness/sessions/") {
        let tenant = extract_query_param(search, "tenant")
          .filter(|t| !t.trim().is_empty())
          .unwrap_or_else(|| "default".to_string());
        AppRoute::HarnessDetail {
          session_id: sid.to_string(),
          tenant,
        }
      } else {
        AppRoute::Studio {
          mode: StudioPosture::Debug,
          preset: None,
          run_id: None,
        }
      }
    }
  }
}

fn extract_query_param(search: &str, key: &str) -> Option<String> {
  let stripped = search.strip_prefix('?').unwrap_or(search);
  for pair in stripped.split('&') {
    if let Some((k, v)) = pair.split_once('=')
      && k == key
    {
      return Some(v.to_string());
    }
  }
  None
}

fn main() {
  dioxus::launch(App);
}

#[component]
fn App() -> Element {
  let mut theme = use_signal(|| Theme::System);
  let mut route = use_signal(|| {
    let (path, search) = current_location_path_and_search();
    parse_route(&path, &search)
  });
  let mut api_token = use_signal(resolve_embedded_token);
  let mut model_drawer_open = use_signal(|| false);
  let mut cached_models = use_signal(Vec::<ModelConfigItem>::new);
  let mut ready_provider_badge = use_signal(|| None::<String>);

  let api_base = resolve_api_base();

  // Background preload of models for Combobox suggestions across views.
  let tok_preload = api_token();
  use_effect(use_reactive((&tok_preload,), move |(tok,)| {
    spawn(async move {
      if let Ok(snap) = fetch_models_config(&tok, "").await {
        let ready = snap.providers.iter().filter(|p| p.api_key_set).count();
        ready_provider_badge.set(Some(format!("{ready}/{}", snap.providers.len())));
        cached_models.set(snap.models);
      }
    });
  }));

  let mut navigate = move |target: String| {
    push_browser_route(&target);
    let (path, search) = match target.split_once('?') {
      Some((p, s)) => (p, format!("?{s}")),
      None => (target.as_str(), String::new()),
    };
    route.set(parse_route(path, &search));
  };

  let cur_route = route();

  rsx! {
    document::Stylesheet { href: APP_CSS }
    ThemeController {
      theme: theme(),
      on_theme_change: move |stored| theme.set(stored),
    }

    div { class: "min-h-screen bg-background text-foreground flex flex-col",
      // Top navigation shell
      header { class: "sticky top-0 z-30 border-b border-border bg-card/90 backdrop-blur px-4 py-2 flex flex-wrap items-center justify-between gap-2",
        div { class: "flex items-center gap-3 flex-wrap",
          div { class: "flex items-center gap-2",
            span { class: "text-sm font-semibold tracking-tight", "Yanshi Studio" }
            Badge { variant: BadgeVariant::Secondary, "Dioxus" }
            if !api_base.is_empty() {
              Badge { variant: BadgeVariant::Outline, "Desktop Engine" }
            }
          }

          nav { class: "flex items-center gap-1 flex-wrap", "aria-label": "主导航",
            Button {
              variant: if matches!(cur_route, AppRoute::Studio { mode: StudioPosture::Debug, .. }) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-studio",
              onclick: move |_| navigate("/ui/studio".to_string()),
              "Studio"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::Studio { mode: StudioPosture::Release, .. }) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-app",
              onclick: move |_| navigate("/ui/app".to_string()),
              "App"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::Notebooks { .. }) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-notebooks",
              onclick: move |_| navigate("/ui/notebooks".to_string()),
              "Notebooks"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::Console { .. } | AppRoute::RunCompare { .. }) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-console",
              onclick: move |_| navigate("/ui/console".to_string()),
              "Console"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::RunCreate) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-runs-new",
              onclick: move |_| navigate("/ui/runs/new".to_string()),
              "New Run"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::HarnessList | AppRoute::HarnessNew | AppRoute::HarnessDetail { .. }) {
                ButtonVariant::Secondary
              } else {
                ButtonVariant::Ghost
              },
              size: ButtonSize::Sm,
              "data-testid": "nav-harness",
              onclick: move |_| navigate("/ui/harness/sessions".to_string()),
              "Harness"
            }
            Button {
              variant: if matches!(cur_route, AppRoute::Diagnostics) { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              "data-testid": "nav-diagnostics",
              onclick: move |_| navigate("/ui/diagnostics".to_string()),
              "Diagnostics"
            }
          }
        }

        div { class: "flex items-center gap-2",
          Button {
            variant: ButtonVariant::Outline,
            size: ButtonSize::Sm,
            "data-testid": "open-model-config-btn",
            onclick: move |_| model_drawer_open.set(true),
            "⚙ 模型配置"
            if let Some(badge) = ready_provider_badge() {
              Badge { variant: BadgeVariant::Secondary, class: "ml-1.5".to_string(), "{badge}" }
            }
          }

          div { class: "inline-flex items-center rounded-md border border-border p-0.5 gap-0.5",
            Button {
              variant: if theme() == Theme::Light { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              "data-testid": "theme-light-btn",
              onclick: move |_| theme.set(Theme::Light),
              "Light"
            }
            Button {
              variant: if theme() == Theme::Dark { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              "data-testid": "theme-dark-btn",
              onclick: move |_| theme.set(Theme::Dark),
              "Dark"
            }
            Button {
              variant: if theme() == Theme::System { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
              size: ButtonSize::Sm,
              class: "h-7 px-2 text-xs".to_string(),
              "data-testid": "theme-system-btn",
              onclick: move |_| theme.set(Theme::System),
              "System"
            }
          }
        }
      }

      // Main page outlet
      main { class: "flex-1",
        match cur_route {
          AppRoute::Studio { mode, preset, run_id } => rsx! {
            WorkflowStudioPage {
              initial_mode: mode,
              initial_preset: preset,
              initial_run_id: run_id,
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              models: cached_models(),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::Notebooks { notebook_id } => rsx! {
            NotebookWorkspacePage {
              initial_notebook_id: notebook_id,
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              models: cached_models(),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::Console { run_id } => rsx! {
            RunConsolePage {
              initial_run_id: run_id,
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::RunCompare { primary_run_id, against, tenant } => rsx! {
            RunComparePage {
              primary_run_id,
              against,
              tenant,
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::Diagnostics => rsx! {
            DiagnosticsPage {
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
            }
          },
          AppRoute::RunCreate => rsx! {
            RunCreatePage {
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
            }
          },
          AppRoute::HarnessList => rsx! {
            HarnessSessionListPage {
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::HarnessNew => rsx! {
            HarnessSubmitPage {
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              models: cached_models(),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::HarnessDetail { session_id, tenant } => rsx! {
            HarnessSessionDetailPage {
              session_id,
              tenant,
              api_token: api_token(),
              on_token_change: move |v: String| api_token.set(v),
              on_navigate: move |target: String| navigate(target),
            }
          },
          AppRoute::SpikeLab => rsx! {
            SpikeLabPage {}
          },
        }
      }

      // Global ModelConfigDrawer mounted once across all pages
      ModelConfigDrawer {
        open: model_drawer_open(),
        on_open_change: move |next: bool| model_drawer_open.set(next),
        api_token: api_token(),
        on_models_updated: move |snap: ModelConfigSnapshot| {
          let ready = snap.providers.iter().filter(|p| p.api_key_set).count();
          ready_provider_badge.set(Some(format!("{ready}/{}", snap.providers.len())));
          cached_models.set(snap.models);
        },
      }
    }
  }
}

/// Interactive verification lab for Resizable 3-pane layout, immediate-Escape
/// Dialog, free-solo Combobox, and CSP-safe Blob SVG/HTML previews.
#[component]
fn SpikeLabPage() -> Element {
  let mut left_size = use_signal(|| 28.0_f64);
  let mut right_size = use_signal(|| 72.0_f64);
  let mut dialog_open = use_signal(|| false);
  let mut blob_bundle = use_signal(|| None::<BlobPreviewBundle>);
  let mut blob_error = use_signal(|| None::<String>);
  let mut combo_query = use_signal(String::new);
  let mut combo_value = use_signal(|| "deepseek-chat".to_string());
  let mut combo_open = use_signal(|| false);

  let preset_models = [
    "deepseek-chat",
    "deepseek-reasoner",
    "gpt-4.1-mini",
    "claude-sonnet-4-5",
  ];
  let q_lower = combo_query().to_ascii_lowercase();
  let filtered: Vec<&str> = preset_models
    .iter()
    .copied()
    .filter(|m| q_lower.is_empty() || m.to_ascii_lowercase().contains(&q_lower))
    .collect();

  rsx! {
    div { class: "h-[calc(100vh-57px)] p-4", "data-testid": "dx-spike-lab",
      ResizablePanelGroup {
        orientation: LayoutOrientation::Horizontal,
        class: "h-full rounded-lg border border-border bg-card overflow-hidden".to_string(),
        ResizablePanel { size: left_size(), min_size: 18.0, max_size: 45.0,
          div { class: "p-4 space-y-4",
            h3 { class: "text-sm font-semibold", "Component Upgrades Lab" }
            Separator {}
            div { class: "space-y-2",
              p { class: "text-xs text-muted-foreground", "1. Dialog Immediate-Escape & Blur Test" }
              Dialog {
                open: Some(dialog_open()),
                on_open_change: move |next: bool| dialog_open.set(next),
                DialogTrigger {
                  class: "inline-flex h-8 items-center justify-center rounded-md border border-input bg-background px-3 text-xs font-medium hover:bg-accent".to_string(),
                  "data-testid": "open-dialog-btn",
                  "Open Dialog"
                }
                DialogOverlay {}
                DialogContent { "data-testid": "spike-dialog-content",
                  DialogTitle { "Immediate Escape Dialog" }
                  DialogDescription {
                    "Press Escape immediately after opening (0ms delay) or after clicking outside."
                  }
                  DialogClose { "✕" }
                }
              }
            }
            Separator {}
            div { class: "space-y-2",
              p { class: "text-xs text-muted-foreground", "2. Free-Solo Combobox (type custom + Enter)" }
              Combobox {
                id: Some("spike-model-combo".to_string()),
                allow_custom_value: true,
                value: Some(combo_value()),
                open: Some(combo_open()),
                on_open_change: move |next: bool| combo_open.set(next),
                on_value_change: move |v: String| {
                  combo_value.set(v.clone());
                  combo_query.set(v);
                },
                ComboboxInput {
                  class: "h-8 rounded-md border border-input bg-background px-2.5 text-xs".to_string(),
                  value: combo_query(),
                  placeholder: "Search or type custom model…".to_string(),
                  "data-testid": "spike-combo-input",
                  on_value_change: move |v: String| combo_query.set(v),
                }
                ComboboxContent {
                  ComboboxList {
                    if filtered.is_empty() {
                      ComboboxEmpty { "Press Enter to commit custom model" }
                    } else {
                      for m in filtered {
                        ComboboxItem { key: "{m}", value: m.to_string(), "{m}" }
                      }
                    }
                  }
                }
              }
              span { "data-testid": "spike-combo-selected",
                Badge { variant: BadgeVariant::Secondary,
                  "Selected: {combo_value()}"
                }
              }
            }
          }
        }
        ResizableHandle {
          orientation: LayoutOrientation::Horizontal,
          value: left_size(),
          min: 18.0,
          max: 45.0,
          on_resize: move |delta: f64| {
            let left = ResizablePanelState::new(left_size(), 18.0, 45.0);
            let right = ResizablePanelState::new(right_size(), 55.0, 82.0);
            let (next_l, next_r) = resizable_resize_pair(left, right, delta);
            left_size.set(next_l.size);
            right_size.set(next_r.size);
          },
        }
        ResizablePanel { size: right_size(), min_size: 55.0, max_size: 82.0,
          div { class: "p-4 space-y-4 overflow-y-auto h-full",
            Card {
              CardHeader {
                CardTitle { "Blob Media & Strict CSP Probe" }
                CardDescription { "Verifies img-src blob: & frame-src blob: under zero-eval CSP and WKWebView." }
              }
              CardContent { class: "space-y-3".to_string(),
                div { class: "flex items-center gap-2",
                  Button {
                    variant: ButtonVariant::Primary,
                    size: ButtonSize::Sm,
                    "data-testid": "probe-blob-btn",
                    onclick: move |_| {
                      if let Some(old) = blob_bundle.take() {
                        revoke_blob_url(&old.svg_url);
                        revoke_blob_url(&old.html_url);
                      }
                      match create_blob_previews() {
                        Ok(bundle) => {
                          blob_error.set(None);
                          blob_bundle.set(Some(bundle));
                        }
                        Err(err) => blob_error.set(Some(err)),
                      }
                    },
                    "Render Blob SVG + HTML"
                  }
                }
                if let Some(err) = blob_error() {
                  Alert { variant: AlertVariant::Destructive, AlertDescription { "{err}" } }
                }
                if let Some(bundle) = blob_bundle() {
                  div { class: "grid grid-cols-1 lg:grid-cols-2 gap-3",
                    img {
                      src: "{bundle.svg_url}",
                      alt: "Blob SVG preview",
                      class: "w-full rounded-md border border-border",
                      "data-testid": "blob-svg-img",
                    }
                    iframe {
                      src: "{bundle.html_url}",
                      title: "Blob HTML preview",
                      "sandbox": "allow-same-origin",
                      class: "w-full h-36 rounded-md border border-border bg-white",
                      "data-testid": "blob-html-iframe",
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_route_resolves_all_canonical_and_legacy_dx_paths() {
    for prefix in ["/ui", "/ui-dx"] {
      assert_eq!(
        parse_route(prefix, ""),
        AppRoute::Studio {
          mode: StudioPosture::Debug,
          preset: None,
          run_id: None,
        }
      );
      assert_eq!(
        parse_route(
          &format!("{prefix}/studio"),
          "?preset=instant_step_debug_demo&run=r-9"
        ),
        AppRoute::Studio {
          mode: StudioPosture::Debug,
          preset: Some("instant_step_debug_demo".to_string()),
          run_id: Some("r-9".to_string()),
        }
      );
      assert_eq!(
        parse_route(&format!("{prefix}/app"), ""),
        AppRoute::Studio {
          mode: StudioPosture::Release,
          preset: None,
          run_id: None,
        }
      );
      assert_eq!(
        parse_route(&format!("{prefix}/console"), "?run=r-1"),
        AppRoute::Console {
          run_id: Some("r-1".to_string()),
        }
      );
      assert_eq!(
        parse_route(
          &format!("{prefix}/runs/r-1/compare"),
          "?against=r-2&tenant=acme"
        ),
        AppRoute::RunCompare {
          primary_run_id: "r-1".to_string(),
          against: Some("r-2".to_string()),
          tenant: "acme".to_string(),
        }
      );
      assert_eq!(
        parse_route(&format!("{prefix}/notebooks/nb-42"), ""),
        AppRoute::Notebooks {
          notebook_id: Some("nb-42".to_string())
        }
      );
      assert_eq!(
        parse_route(&format!("{prefix}/diagnostics"), ""),
        AppRoute::Diagnostics
      );
      assert_eq!(
        parse_route(&format!("{prefix}/runs/new"), ""),
        AppRoute::RunCreate
      );
      assert_eq!(
        parse_route(
          &format!("{prefix}/harness/sessions/sess-42"),
          "?tenant=acme"
        ),
        AppRoute::HarnessDetail {
          session_id: "sess-42".to_string(),
          tenant: "acme".to_string()
        }
      );
      assert_eq!(
        parse_route(&format!("{prefix}/spike"), ""),
        AppRoute::SpikeLab
      );
    }
  }
}
