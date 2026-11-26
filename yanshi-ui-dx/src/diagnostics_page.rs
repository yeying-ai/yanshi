//! `/ui-dx/diagnostics` — Operator doctor view migrated to `dioxus-shadcn`.
//! Fetches `/v1/diagnostics` once on mount (explicit manual Refresh, no auto-poll).

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardDescription, CardHeader, CardTitle, Input, InputSize, Label, Table,
  TableBody, TableCell, TableHead, TableHeader, TableRow,
};

use crate::api::{DiagnosticsDirCheck, DiagnosticsReport, fetch_diagnostics, mask_token};

#[derive(Clone, Debug, PartialEq, Eq)]
struct DiagRow {
  component: String,
  status: String,
  detail: String,
}

fn collect_rows(report: &DiagnosticsReport) -> Vec<DiagRow> {
  let mut rows = Vec::new();
  if let Some(ref cfg) = report.config {
    let status = if cfg.error.is_some() || !cfg.missing_env_vars.is_empty() {
      "warning"
    } else {
      "ok"
    };
    let detail = if let Some(ref err) = cfg.error {
      err.clone()
    } else if !cfg.missing_env_vars.is_empty() {
      format!("Missing env vars: {}", cfg.missing_env_vars.join(", "))
    } else {
      format!(
        "{} models / {} providers",
        cfg.models.unwrap_or(0),
        cfg.providers.unwrap_or(0)
      )
    };
    rows.push(DiagRow {
      component: "Models config".to_string(),
      status: status.to_string(),
      detail,
    });
  }
  if let Some(ref sec) = report.security {
    let status = if sec.warning.is_some() {
      "warning"
    } else {
      "ok"
    };
    let detail = sec
      .warning
      .clone()
      .unwrap_or_else(|| format!("profile: {}", sec.profile.as_deref().unwrap_or("unknown")));
    rows.push(DiagRow {
      component: "Security profile".to_string(),
      status: status.to_string(),
      detail,
    });
  }
  if let Some(ref sb) = report.sandbox {
    let status = if sb.enforcing { "ok" } else { "warning" };
    let detail = format!(
      "backend: {} ({})",
      sb.backend.as_deref().unwrap_or("unknown"),
      sb.enforcement.as_deref().unwrap_or("unknown")
    );
    rows.push(DiagRow {
      component: "OS sandbox".to_string(),
      status: status.to_string(),
      detail,
    });
  }
  if let Some(ref disk) = report.disk {
    let checks: [(&str, Option<&DiagnosticsDirCheck>); 3] = [
      ("Run dir", disk.run_dir.as_ref()),
      ("Trace dir", disk.trace_dir.as_ref()),
      ("Marketplace cache", disk.marketplace_cache.as_ref()),
    ];
    for (label, maybe_dir) in checks {
      if let Some(dir) = maybe_dir {
        let status = if !dir.exists {
          "warning"
        } else if !dir.writable {
          "fail"
        } else {
          "ok"
        };
        let writable = if dir.writable { ", writable" } else { "" };
        rows.push(DiagRow {
          component: label.to_string(),
          status: status.to_string(),
          detail: format!("{} ({}{})", dir.path, dir.source, writable),
        });
      }
    }
  }
  if let Some(ref env) = report.environment {
    rows.push(DiagRow {
      component: "YANSHI_API_TOKEN".to_string(),
      status: if env.yanshi_api_token_set {
        "ok"
      } else {
        "warning"
      }
      .to_string(),
      detail: if env.yanshi_api_token_set {
        "set (value masked)".to_string()
      } else {
        "not set".to_string()
      },
    });
  }
  rows
}

fn status_badge_variant(status: &str) -> (BadgeVariant, &'static str) {
  match status {
    "ok" => (BadgeVariant::Default, "PASS"),
    "warning" => (BadgeVariant::Secondary, "WARN"),
    _ => (BadgeVariant::Destructive, "FAIL"),
  }
}

#[component]
pub fn DiagnosticsPage(api_token: String, on_token_change: EventHandler<String>) -> Element {
  let mut report = use_signal(|| None::<DiagnosticsReport>);
  let mut loading = use_signal(|| false);
  let mut error = use_signal(|| None::<String>);
  let mut initialized = use_signal(|| false);

  let tok_init = api_token.clone();
  use_effect(move || {
    if initialized() {
      return;
    }
    initialized.set(true);
    let tok = tok_init.clone();
    loading.set(true);
    spawn(async move {
      match fetch_diagnostics(&tok).await {
        Ok(data) => {
          report.set(Some(data));
          error.set(None);
        }
        Err(err) => {
          error.set(Some(err));
          report.set(None);
        }
      }
      loading.set(false);
    });
  });

  let tok_refresh = api_token.clone();
  let rows = report().as_ref().map(collect_rows).unwrap_or_default();
  let token_display = mask_token(&api_token);

  rsx! {
    div { class: "max-w-5xl mx-auto p-5 space-y-4", "data-testid": "dx-diagnostics-page",
      Card {
        CardHeader {
          CardTitle { "System Diagnostics" }
          CardDescription {
            "In-process model registry, security profile, OS sandbox, and storage health check (manual refresh)."
          }
        }
        CardContent { class: "space-y-4".to_string(),
          div { class: "flex flex-wrap items-end gap-3",
            div { class: "space-y-1 flex-1 min-w-[220px]",
              Label { "API token (last 4 shown): {token_display}" }
              Input {
                size: InputSize::Sm,
                r#type: "password",
                autocomplete: "off",
                value: api_token,
                placeholder: "paste bearer token".to_string(),
                "data-testid": "diag-token-input",
                on_value_change: move |v: String| on_token_change.call(v),
              }
            }
            Button {
              variant: ButtonVariant::Primary,
              size: ButtonSize::Sm,
              loading: loading(),
              "data-testid": "diag-refresh-btn",
              onclick: move |_| {
                let tok = tok_refresh.clone();
                loading.set(true);
                error.set(None);
                spawn(async move {
                  match fetch_diagnostics(&tok).await {
                    Ok(data) => report.set(Some(data)),
                    Err(err) => {
                      error.set(Some(err));
                      report.set(None);
                    }
                  }
                  loading.set(false);
                });
              },
              if loading() { "Refreshing…" } else { "Refresh" }
            }
          }

          if let Some(err_msg) = error() {
            Alert { variant: AlertVariant::Destructive,
              AlertDescription { "Error: {err_msg}" }
            }
          }

          if let Some(ref rep) = report() {
            {
              let (variant, label) = status_badge_variant(&rep.status);
              rsx! {
                div { class: "flex flex-wrap items-center gap-2 rounded-md border border-border bg-muted/40 px-3 py-2 text-sm",
                  "data-testid": "diag-overall-banner",
                  span { class: "font-medium", "Overall:" }
                  Badge { variant, "{label}" }
                  if let Some(ref ver) = rep.version {
                    span { class: "text-xs text-muted-foreground font-mono", "version {ver}" }
                  }
                  if let Some(ref prof) = rep.profile {
                    span { class: "text-xs text-muted-foreground font-mono", "profile {prof}" }
                  }
                }
              }
            }
          }

          Table { compact: true, striped: true,
            TableHeader {
              TableRow {
                TableHead { "Component" }
                TableHead { "Status" }
                TableHead { "Detail" }
              }
            }
            TableBody {
              if rows.is_empty() && error().is_none() {
                TableRow {
                  TableCell { class: "text-muted-foreground".to_string(),
                    if loading() { "Loading…" } else { "No data yet" }
                  }
                  TableCell {}
                  TableCell {}
                }
              } else {
                for row in rows.iter() {
                  {
                    let (variant, label) = status_badge_variant(&row.status);
                    rsx! {
                      TableRow { key: "{row.component}",
                        TableCell { class: "font-medium".to_string(),
                          span { "data-testid": "diag-row", "{row.component}" }
                        }
                        TableCell { Badge { variant, "{label}" } }
                        TableCell { class: "font-mono text-xs".to_string(), "{row.detail}" }
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
