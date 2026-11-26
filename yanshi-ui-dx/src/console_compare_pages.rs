//! Classic Event Run Console (`/ui-dx/console`) and Side-by-Side Trace
//! Comparison (`/ui-dx/runs/:id/compare?against=...`) pages.

use std::collections::{HashMap, HashSet};

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Card, CardContent, CardHeader, CardTitle, Input, NativeSelect, NativeSelectOption,
  NativeSelectSize,
};

use crate::api::{SseAbortHandle, encode_uri_component, read_storage, stream_sse, write_storage};
use crate::studio_api::{
  LaunchStudioRunRequest, RunRecord, StreamedEvent, cancel_run, compare_key, event_node_id,
  event_tone, get_run, get_run_events_history, is_terminal_run_record, launch_studio_run,
  list_runs, matches_event_filter, summarise_events,
};
use crate::yaml_editor::YamlCodeEditor;

const CONSOLE_STARTER_YAML: &str = "name: web-ui-console-smoke\nnodes:\n  - id: hello\n    type: template\n    parameters:\n      template: \"hello from the run console\"";

fn tone_badge_variant(tone: &str) -> BadgeVariant {
  match tone {
    "error" => BadgeVariant::Destructive,
    "ok" => BadgeVariant::Default,
    "agent" | "tool" => BadgeVariant::Secondary,
    _ => BadgeVariant::Outline,
  }
}

fn short_id(id: &str) -> String {
  id.chars().take(8).collect()
}

// ── Classic Event Run Console (`/ui-dx/console`) ─────────────────────────────

#[component]
pub fn RunConsolePage(
  #[props(default)] initial_run_id: Option<String>,
  api_token: String,
  on_token_change: EventHandler<String>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut run_id_input = use_signal(|| initial_run_id.clone().unwrap_or_default());
  let mut tenant_id = use_signal(|| {
    let t = read_storage("yanshi.ui.tenant", "default");
    if t.trim().is_empty() {
      "default".to_string()
    } else {
      t
    }
  });
  let mut workflow_draft = use_signal(|| CONSOLE_STARTER_YAML.to_string());
  let mut runs = use_signal(Vec::<RunRecord>::new);
  let mut active_run = use_signal(|| None::<RunRecord>);
  let mut events = use_signal(Vec::<StreamedEvent>::new);
  let mut selected_seq = use_signal(|| None::<i64>);
  let mut filter_expr = use_signal(String::new);
  let mut stream_state = use_signal(|| "idle");
  let mut error_msg = use_signal(|| None::<String>);
  let mut busy = use_signal(|| false);
  let mut gen_counter = use_signal(|| 0_u64);

  let mut connect_run = move |rid: String, tok: String, ten: String| {
    if rid.trim().is_empty() {
      return;
    }
    let my_gen = gen_counter.peek().wrapping_add(1);
    gen_counter.set(my_gen);
    run_id_input.set(rid.clone());
    stream_state.set("loading");
    error_msg.set(None);
    events.set(Vec::new());
    selected_seq.set(None);

    spawn(async move {
      let rec = match get_run(&rid, &ten, &tok).await {
        Ok(r) => r,
        Err(err) => {
          if *gen_counter.peek() == my_gen {
            stream_state.set("error");
            error_msg.set(Some(err));
          }
          return;
        }
      };
      if *gen_counter.peek() != my_gen {
        return;
      }
      let done = is_terminal_run_record(&rec);
      active_run.set(Some(rec));

      let mut after_seq = -1_i64;
      if let Ok(hist) = get_run_events_history(&rid, None, &ten, &tok).await
        && *gen_counter.peek() == my_gen
      {
        after_seq = hist.last().map(|e| e.seq).unwrap_or(-1);
        selected_seq.set(hist.last().map(|e| e.seq));
        events.set(hist);
      }
      if done {
        stream_state.set("closed");
        return;
      }

      stream_state.set("streaming");
      let path = format!(
        "/v1/runs/{}/events?after_seq={after_seq}",
        encode_uri_component(&rid)
      );
      if let Ok(handle) = SseAbortHandle::new() {
        let _ = stream_sse(&path, &tok, &ten, &handle, |frame| {
          if *gen_counter.peek() != my_gen {
            handle.abort();
            return;
          }
          if let Ok(ev) = serde_json::from_str::<StreamedEvent>(&frame.data) {
            let mut cur = events.peek().clone();
            if !cur.iter().any(|x| x.seq == ev.seq) {
              if selected_seq.peek().is_none() {
                selected_seq.set(Some(ev.seq));
              }
              cur.push(ev);
              cur.sort_by_key(|x| x.seq);
              events.set(cur);
            }
          }
        })
        .await;
      }
      if *gen_counter.peek() == my_gen {
        stream_state.set("closed");
        if let Ok(latest) = get_run(&rid, &ten, &tok).await {
          active_run.set(Some(latest));
        }
      }
    });
  };

  let tok_load = api_token.clone();
  let ten_load = tenant_id();
  let init_rid = initial_run_id.clone().unwrap_or_default();
  use_effect(use_reactive((&tok_load, &ten_load), move |(tok, ten)| {
    write_storage("yanshi.ui.tenant", &ten);
    let init_r = init_rid.clone();
    spawn(async move {
      if let Ok(list) = list_runs(&ten, 20, &tok).await {
        let first_id = list.first().map(|r| r.id.clone());
        runs.set(list);
        if !init_r.is_empty() {
          connect_run(init_r, tok, ten);
        } else if run_id_input.peek().is_empty()
          && let Some(fid) = first_id
        {
          connect_run(fid, tok, ten);
        }
      }
    });
  }));

  let all_events = events();
  let cur_filter = filter_expr();
  let filtered_events: Vec<StreamedEvent> = all_events
    .iter()
    .filter(|e| matches_event_filter(e, &cur_filter))
    .cloned()
    .collect();
  let selected_event = selected_seq()
    .and_then(|seq| all_events.iter().find(|e| e.seq == seq))
    .or_else(|| all_events.last())
    .cloned();

  // Derive observed node summary list from events
  let mut node_map: HashMap<String, (String, &'static str, i64)> = HashMap::new();
  let mut node_order: Vec<String> = Vec::new();
  for ev in &all_events {
    let key = event_node_id(ev).unwrap_or_else(|| ev.kind.clone());
    if !node_map.contains_key(&key) {
      node_order.push(key.clone());
    }
    node_map.insert(key, (ev.kind.clone(), event_tone(&ev.kind), ev.seq));
  }
  let node_summaries: Vec<(String, String, &'static str, i64)> = node_order
    .into_iter()
    .filter_map(|k| {
      node_map
        .remove(&k)
        .map(|(st, tone, seq)| (k, st, tone, seq))
    })
    .collect();

  let tok_conn = api_token.clone();
  let tok_cancel = api_token.clone();
  let tok_sub = api_token.clone();
  let tok_pick = api_token.clone();
  let cur_run_id = run_id_input();
  let cur_active = active_run();

  rsx! {
    div { class: "p-4 space-y-4 max-w-[1600px] mx-auto", "data-testid": "run-console-page",
      // Header bar
      div { class: "flex flex-wrap items-center justify-between gap-3 border-b border-border pb-3",
        div {
          h1 { class: "text-base font-bold tracking-tight", "Classic Run Console" }
          p { class: "text-xs text-muted-foreground", "实时 SSE 事件流、表达式过滤与原始 Payload 观测台" }
        }
        div { class: "flex flex-wrap items-center gap-2",
          Input {
            class: "h-8 w-56 text-xs font-mono".to_string(),
            placeholder: "Run ID".to_string(),
            value: cur_run_id.clone(),
            on_value_change: move |v: String| run_id_input.set(v),
          }
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            onclick: move |_| {
              let rid = run_id_input.peek().clone();
              let ten = tenant_id.peek().clone();
              connect_run(rid, tok_conn.clone(), ten);
            },
            "Connect"
          }
          Button {
            variant: ButtonVariant::Destructive,
            size: ButtonSize::Sm,
            disabled: cur_active.as_ref().is_none_or(is_terminal_run_record) || busy(),
            onclick: move |_| {
              let Some(run) = active_run.peek().clone() else { return; };
              let ten = run.tenant_id.unwrap_or_else(|| tenant_id.peek().clone());
              let tok = tok_cancel.clone();
              busy.set(true);
              spawn(async move {
                match cancel_run(&run.id, &ten, &tok).await {
                  Ok(rec) => {
                    let next_gen = gen_counter.peek().wrapping_add(1);
                    gen_counter.set(next_gen);
                    active_run.set(Some(rec));
                    stream_state.set("closed");
                  }
                  Err(err) => error_msg.set(Some(err)),
                }
                busy.set(false);
              });
            },
            "Cancel"
          }
          if !cur_run_id.trim().is_empty() {
            {
              let other = runs()
                .iter()
                .find(|r| r.id != cur_run_id.trim())
                .map(|r| r.id.clone());
              let href = match other {
                Some(o) => format!(
                  "/ui/runs/{}/compare?against={}",
                  encode_uri_component(cur_run_id.trim()),
                  encode_uri_component(&o)
                ),
                None => format!("/ui/runs/{}/compare", encode_uri_component(cur_run_id.trim())),
              };
              rsx! {
                Button {
                  variant: ButtonVariant::Outline,
                  size: ButtonSize::Sm,
                  onclick: move |_| on_navigate.call(href.clone()),
                  "⇄ Compare"
                }
              }
            }
          }
        }
      }

      // Status strip
      div { class: "grid grid-cols-2 sm:grid-cols-4 gap-2",
        Card { CardContent { class: "p-3".to_string(), span { class: "text-[11px] text-muted-foreground block", "Stream" } strong { class: "text-sm font-mono", "{stream_state()}" } } }
        Card { CardContent { class: "p-3".to_string(), span { class: "text-[11px] text-muted-foreground block", "Status" } strong { class: "text-sm font-mono", "{cur_active.as_ref().map(|r| r.status.as_str()).unwrap_or(\"none\")}" } } }
        Card { CardContent { class: "p-3".to_string(), span { class: "text-[11px] text-muted-foreground block", "Tenant" } strong { class: "text-sm font-mono", "{tenant_id()}" } } }
        Card { CardContent { class: "p-3".to_string(), span { class: "text-[11px] text-muted-foreground block", "Events" } strong { class: "text-sm font-mono", "{all_events.len()}" } } }
      }

      if let Some(err) = error_msg() {
        Alert { variant: AlertVariant::Destructive, AlertDescription { "{err}" } }
      }

      // Three-column main workspace
      div { class: "grid grid-cols-1 lg:grid-cols-12 gap-4",
        // Runs & Quick Submit
        Card { class: "lg:col-span-4".to_string(),
          CardHeader { class: "pb-2".to_string(), CardTitle { class: "text-sm".to_string(), "Runs & Quick Submit" } }
          CardContent { class: "space-y-3".to_string(),
            div { class: "space-y-1 max-h-48 overflow-y-auto",
              for run in runs() {
                {
                  let rid = run.id.clone();
                  let tok = tok_pick.clone();
                  let is_sel = cur_run_id == run.id;
                  rsx! {
                    Button {
                      key: "crun-{run.id}",
                      variant: if is_sel { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                      size: ButtonSize::Sm,
                      class: "w-full justify-between text-xs font-mono".to_string(),
                      onclick: move |_| {
                        let ten = tenant_id.peek().clone();
                        connect_run(rid.clone(), tok.clone(), ten);
                      },
                      span { "{short_id(&run.id)}" }
                      Badge { variant: BadgeVariant::Outline, "{run.status}" }
                    }
                  }
                }
              }
            }
            div { class: "space-y-2 pt-2 border-t border-border",
              div { class: "grid grid-cols-2 gap-2",
                Input {
                  class: "h-8 text-xs".to_string(),
                  placeholder: "Tenant".to_string(),
                  value: tenant_id(),
                  on_value_change: move |v: String| tenant_id.set(v),
                }
                Input {
                  r#type: "password".to_string(),
                  class: "h-8 text-xs".to_string(),
                  placeholder: "Bearer token".to_string(),
                  value: api_token.clone(),
                  on_value_change: move |v: String| on_token_change.call(v),
                }
              }
              YamlCodeEditor {
                aria_label: "Console Workflow YAML".to_string(),
                value: workflow_draft(),
                rows: 8,
                on_change: move |v: String| workflow_draft.set(v),
              }
              Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                class: "w-full".to_string(),
                disabled: busy(),
                onclick: move |_| {
                  let yml = workflow_draft.peek().clone();
                  let ten = tenant_id.peek().clone();
                  let tok = tok_sub.clone();
                  busy.set(true);
                  error_msg.set(None);
                  spawn(async move {
                    let req = LaunchStudioRunRequest {
                      tenant_id: ten.clone(),
                      workflow_yaml: yml,
                      inputs: serde_json::Map::new(),
                      model: None,
                      cost_limit_usd: None,
                      run_mode: "release".to_string(),
                      step_every_node: false,
                      breakpoints: Vec::new(),
                    };
                    match launch_studio_run(&req, &tok).await {
                      Ok(new_id) => {
                        if let Ok(list) = list_runs(&ten, 20, &tok).await {
                          runs.set(list);
                        }
                        connect_run(new_id, tok, ten);
                      }
                      Err(err) => error_msg.set(Some(err)),
                    }
                    busy.set(false);
                  });
                },
                if busy() { "Submitting…" } else { "Submit run" }
              }
            }
          }
        }

        // Observed DAG nodes + Selected Payload
        Card { class: "lg:col-span-4".to_string(),
          CardHeader { class: "pb-2".to_string(), CardTitle { class: "text-sm".to_string(), "Observed Nodes ({node_summaries.len()})" } }
          CardContent { class: "space-y-3".to_string(),
            if node_summaries.is_empty() {
              p { class: "text-xs text-muted-foreground", "Waiting for events…" }
            } else {
              div { class: "grid grid-cols-2 gap-2",
                for (name, status, tone, seq) in node_summaries {
                  button {
                    key: "obs-{name}",
                    r#type: "button",
                    class: "rounded-md border border-border bg-background hover:bg-accent/60 p-2 text-left",
                    onclick: move |_| selected_seq.set(Some(seq)),
                    div { class: "text-xs font-mono font-semibold truncate", "{name}" }
                    Badge { variant: tone_badge_variant(tone), class: "mt-1 text-[10px]".to_string(), "{status}" }
                  }
                }
              }
            }
            div { class: "pt-2 border-t border-border space-y-1",
              span { class: "text-xs font-semibold block",
                "Event Payload ({selected_event.as_ref().map(|e| e.kind.as_str()).unwrap_or(\"none\")})"
              }
              pre { class: "rounded-md border border-border bg-muted/40 p-2.5 text-[11px] font-mono whitespace-pre-wrap break-words max-h-72 overflow-y-auto",
                if let Some(ev) = &selected_event {
                  "{serde_json::to_string_pretty(&ev.payload).unwrap_or_default()}"
                } else {
                  "Select an event."
                }
              }
            }
          }
        }

        // Filterable Timeline
        Card { class: "lg:col-span-4".to_string(),
          CardHeader { class: "pb-2".to_string(),
            CardTitle { class: "text-sm".to_string(), "Event Timeline ({filtered_events.len()}/{all_events.len()})" }
          }
          CardContent { class: "space-y-2".to_string(),
            Input {
              class: "h-8 text-xs font-mono".to_string(),
              "data-testid": "console-event-filter",
              placeholder: "kind=node.completed AND seq>1".to_string(),
              value: filter_expr(),
              on_value_change: move |v: String| filter_expr.set(v),
            }
            div { class: "space-y-1 max-h-[460px] overflow-y-auto",
              for ev in filtered_events {
                {
                  let is_sel = selected_event.as_ref().is_some_and(|s| s.seq == ev.seq);
                  let seq = ev.seq;
                  rsx! {
                    Button {
                      key: "ev-{ev.seq}",
                      variant: if is_sel { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                      size: ButtonSize::Sm,
                      class: "w-full justify-between text-xs font-mono h-7 px-2".to_string(),
                      onclick: move |_| selected_seq.set(Some(seq)),
                      span { class: "truncate", "#{ev.seq} {ev.kind}" }
                      Badge { variant: tone_badge_variant(event_tone(&ev.kind)), class: "text-[10px]".to_string(),
                        "{event_node_id(&ev).unwrap_or_else(|| \"flow\".to_string())}"
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

// ── Trace Comparison View (`/ui-dx/runs/:id/compare?against=...`) ────────────

#[component]
pub fn RunComparePage(
  primary_run_id: String,
  #[props(default)] against: Option<String>,
  #[props(default = "default".to_string())] tenant: String,
  api_token: String,
  on_token_change: EventHandler<String>,
  on_navigate: EventHandler<String>,
) -> Element {
  let mut other_run_id = use_signal(|| against.clone().unwrap_or_default());
  let mut recent_runs = use_signal(Vec::<RunRecord>::new);
  let mut primary_events = use_signal(Vec::<StreamedEvent>::new);
  let mut other_events = use_signal(Vec::<StreamedEvent>::new);
  let mut loading = use_signal(|| false);
  let mut error_msg = use_signal(|| None::<String>);

  use_effect(use_reactive((&against,), move |(ag,)| {
    other_run_id.set(ag.unwrap_or_default());
  }));

  let tok_list = api_token.clone();
  let ten_list = tenant.clone();
  use_effect(use_reactive((&tok_list, &ten_list), move |(tok, ten)| {
    spawn(async move {
      if let Ok(list) = list_runs(&ten, 25, &tok).await {
        recent_runs.set(list);
      }
    });
  }));

  let cur_other = other_run_id();
  let tok_ev = api_token.clone();
  let ten_ev = tenant.clone();
  let prim_ev = primary_run_id.clone();
  use_effect(use_reactive(
    (&prim_ev, &cur_other, &tok_ev, &ten_ev),
    move |(p_id, o_id, tok, ten)| {
      if p_id.is_empty() {
        return;
      }
      loading.set(true);
      error_msg.set(None);
      spawn(async move {
        let p_res = get_run_events_history(&p_id, Some(1000), &ten, &tok).await;
        let o_res = if !o_id.is_empty() && o_id != p_id {
          get_run_events_history(&o_id, Some(1000), &ten, &tok).await
        } else {
          Ok(Vec::new())
        };
        match (p_res, o_res) {
          (Ok(pe), Ok(oe)) => {
            primary_events.set(pe);
            other_events.set(oe);
          }
          (Err(err), _) | (_, Err(err)) => error_msg.set(Some(err)),
        }
        loading.set(false);
      });
    },
  ));

  let p_evs = primary_events();
  let o_evs = other_events();
  let p_keys: HashSet<String> = p_evs.iter().map(compare_key).collect();
  let o_keys: HashSet<String> = o_evs.iter().map(compare_key).collect();
  let p_sum = summarise_events(&p_evs);
  let o_sum = summarise_events(&o_evs);
  let candidates: Vec<RunRecord> = recent_runs()
    .into_iter()
    .filter(|r| r.id != primary_run_id)
    .collect();

  let prim_for_select = primary_run_id.clone();
  let prim_for_go = primary_run_id.clone();
  let prim_for_swap = primary_run_id.clone();

  rsx! {
    div { class: "p-4 space-y-4 max-w-[1500px] mx-auto", "data-testid": "run-compare-page",
      div { class: "flex flex-wrap items-center justify-between gap-2 border-b border-border pb-3",
        div {
          h1 { class: "text-base font-bold", "Trace Comparison" }
          p { class: "text-xs text-muted-foreground", "对比两次运行的事件时间线与步骤差异" }
        }
        div { class: "flex flex-wrap items-center gap-2",
          if !candidates.is_empty() {
            NativeSelect {
              size: NativeSelectSize::Sm,
              class: "w-60".to_string(),
              on_value_change: move |picked: String| {
                other_run_id.set(picked.clone());
                if !picked.is_empty() {
                  on_navigate.call(format!(
                    "/ui/runs/{}/compare?against={}",
                    encode_uri_component(&prim_for_select),
                    encode_uri_component(&picked)
                  ));
                }
              },
              NativeSelectOption { value: String::new(), selected: cur_other.is_empty(), "-- 选择历史运行 --" }
              for r in candidates {
                NativeSelectOption {
                  key: "cmp-{r.id}",
                  value: r.id.clone(),
                  selected: cur_other == r.id,
                  "{short_id(&r.id)} · {r.status}"
                }
              }
            }
          }
          Input {
            class: "h-8 w-44 text-xs font-mono".to_string(),
            placeholder: "Compare against run id".to_string(),
            value: cur_other.clone(),
            on_value_change: move |v: String| other_run_id.set(v.trim().to_string()),
          }
          Input {
            r#type: "password".to_string(),
            class: "h-8 w-36 text-xs".to_string(),
            placeholder: "Token (optional)".to_string(),
            value: api_token,
            on_value_change: move |v: String| on_token_change.call(v),
          }
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            disabled: cur_other.is_empty() || cur_other == primary_run_id,
            onclick: move |_| {
              let o = other_run_id.peek().clone();
              on_navigate.call(format!(
                "/ui/runs/{}/compare?against={}",
                encode_uri_component(&prim_for_go),
                encode_uri_component(&o)
              ));
            },
            "Compare"
          }
          if !cur_other.is_empty() && cur_other != primary_run_id {
            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              onclick: move |_| {
                let o = other_run_id.peek().clone();
                on_navigate.call(format!(
                  "/ui/runs/{}/compare?against={}",
                  encode_uri_component(&o),
                  encode_uri_component(&prim_for_swap)
                ));
              },
              "⇄ Swap"
            }
          }
        }
      }

      if loading() {
        p { class: "text-xs text-muted-foreground", "Loading run timelines…" }
      }
      if let Some(err) = error_msg() {
        Alert { variant: AlertVariant::Destructive, AlertDescription { "{err}" } }
      }

      // Summary cards
      div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
        Card {
          CardHeader { class: "pb-2".to_string(), CardTitle { class: "text-sm font-mono".to_string(), "Primary · {primary_run_id}" } }
          CardContent { class: "grid grid-cols-3 gap-2 text-xs".to_string(),
            div { span { class: "text-muted-foreground block", "Events" } strong { "{p_sum.event_count}" } }
            div { span { class: "text-muted-foreground block", "Nodes" } strong { "{p_sum.node_count}" } }
            div { span { class: "text-muted-foreground block", "Tool calls" } strong { "{p_sum.tool_call_count}" } }
          }
        }
        Card {
          CardHeader { class: "pb-2".to_string(),
            CardTitle { class: "text-sm font-mono".to_string(),
              if cur_other.is_empty() { "Compared · —" } else { "Compared · {cur_other}" }
            }
          }
          CardContent { class: "grid grid-cols-3 gap-2 text-xs".to_string(),
            div { span { class: "text-muted-foreground block", "Events" } strong { "{o_sum.event_count}" } }
            div { span { class: "text-muted-foreground block", "Nodes" } strong { "{o_sum.node_count}" } }
            div { span { class: "text-muted-foreground block", "Tool calls" } strong { "{o_sum.tool_call_count}" } }
          }
        }
      }

      // Side-by-side event columns
      div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
        Card {
          CardHeader { class: "pb-2".to_string(), CardTitle { class: "text-xs font-mono".to_string(), "Primary Timeline" } }
          CardContent { class: "space-y-1 max-h-[520px] overflow-y-auto".to_string(),
            if p_evs.is_empty() {
              p { class: "text-xs text-muted-foreground", "No events." }
            } else {
              for ev in p_evs {
                {
                  let matched = o_keys.contains(&compare_key(&ev));
                  rsx! {
                    div {
                      key: "pe-{ev.seq}",
                      class: if matched {
                        "flex items-center justify-between gap-2 rounded border border-border px-2 py-1 text-xs font-mono"
                      } else {
                        "flex items-center justify-between gap-2 rounded border border-amber-500/50 bg-amber-500/10 px-2 py-1 text-xs font-mono"
                      },
                      span { "#{ev.seq} {ev.kind}" }
                      if !matched && !cur_other.is_empty() {
                        Badge { variant: BadgeVariant::Secondary, class: "text-[10px]".to_string(), "only here" }
                      }
                    }
                  }
                }
              }
            }
          }
        }
        Card {
          CardHeader { class: "pb-2".to_string(), CardTitle { class: "text-xs font-mono".to_string(), "Compared Timeline" } }
          CardContent { class: "space-y-1 max-h-[520px] overflow-y-auto".to_string(),
            if o_evs.is_empty() {
              p { class: "text-xs text-muted-foreground", "No events." }
            } else {
              for ev in o_evs {
                {
                  let matched = p_keys.contains(&compare_key(&ev));
                  rsx! {
                    div {
                      key: "oe-{ev.seq}",
                      class: if matched {
                        "flex items-center justify-between gap-2 rounded border border-border px-2 py-1 text-xs font-mono"
                      } else {
                        "flex items-center justify-between gap-2 rounded border border-amber-500/50 bg-amber-500/10 px-2 py-1 text-xs font-mono"
                      },
                      span { "#{ev.seq} {ev.kind}" }
                      if !matched {
                        Badge { variant: BadgeVariant::Secondary, class: "text-[10px]".to_string(), "only here" }
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
