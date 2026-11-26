//! Node configuration form editor (`NodeConfigEditor`) and live execution I/O,
//! media preview & debug pause override panel (`NodeIoInspector`).

use std::collections::BTreeMap;

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Input, NativeSelect, NativeSelectOption, NativeSelectSize, Textarea,
};

use crate::deliverable_viewer::AuthorizedMedia;
use crate::studio_api::{
  DebugResumeOverrides, FlowDebugPause, NodeRuntimeTelemetry, extract_node_media_previews,
  parse_debug_override_object, unwrap_captured_flow_json,
};
use crate::studio_yaml::{InspectedNode, NodeUpdatePatch, node_palette};

#[derive(Clone, Debug, PartialEq, Eq)]
struct MappingRowDraft {
  key: String,
  expr: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum QuickTextField {
  Prompt,
  Template,
  InputTemplate,
}

impl QuickTextField {
  const fn param_key(self) -> &'static str {
    match self {
      Self::Prompt => "prompt",
      Self::Template => "template",
      Self::InputTemplate => "input_template",
    }
  }

  const fn label(self) -> &'static str {
    match self {
      Self::Prompt => "提示词模板 (parameters.prompt)",
      Self::Template => "Jinja 模板内容 (parameters.template)",
      Self::InputTemplate => "语音输入模板 (parameters.input_template)",
    }
  }
}

fn detect_quick_text_field(
  node_type: &str,
  params: &serde_json::Map<String, serde_json::Value>,
) -> Option<QuickTextField> {
  match node_type {
    "llm" | "agent" | "skill_agent" | "text_to_image" | "text_to_video" => {
      Some(QuickTextField::Prompt)
    }
    "template" => Some(QuickTextField::Template),
    "tts" if params.contains_key("input_template") || params.is_empty() => {
      Some(QuickTextField::InputTemplate)
    }
    _ => None,
  }
}

#[derive(Clone, Debug, PartialEq)]
struct NodeEditorDraft {
  id_draft: String,
  type_draft: String,
  dependencies_draft: Vec<String>,
  mapping_rows: Vec<MappingRowDraft>,
  run_if_draft: String,
  timeout_draft: String,
  retries_draft: String,
  quick_field: Option<QuickTextField>,
  quick_value: String,
  params_json_draft: String,
}

fn create_node_editor_draft(node: &InspectedNode) -> NodeEditorDraft {
  let params_obj = node.parameters.as_object().cloned().unwrap_or_default();
  let quick_field = detect_quick_text_field(&node.node_type, &params_obj);
  let quick_value = quick_field
    .and_then(|qf| params_obj.get(qf.param_key()))
    .and_then(serde_json::Value::as_str)
    .unwrap_or("")
    .to_string();

  let mut rest_params = params_obj;
  if let Some(qf) = quick_field {
    rest_params.remove(qf.param_key());
  }

  NodeEditorDraft {
    id_draft: node.id.clone(),
    type_draft: node.node_type.clone(),
    dependencies_draft: node.dependencies.clone(),
    mapping_rows: node
      .input_mapping
      .iter()
      .map(|(k, v)| MappingRowDraft {
        key: k.clone(),
        expr: v.clone(),
      })
      .collect(),
    run_if_draft: node.run_if.clone().unwrap_or_default(),
    timeout_draft: node.timeout_ms.map(|v| v.to_string()).unwrap_or_default(),
    retries_draft: node.max_retries.map(|v| v.to_string()).unwrap_or_default(),
    quick_field,
    quick_value,
    params_json_draft: serde_json::to_string_pretty(&serde_json::Value::Object(rest_params))
      .unwrap_or_else(|_| "{}".to_string()),
  }
}

fn build_node_patch_from_draft(
  original: &InspectedNode,
  draft: &NodeEditorDraft,
) -> Result<NodeUpdatePatch, String> {
  let trimmed_id = draft.id_draft.trim();
  if trimmed_id.is_empty() {
    return Err("节点 ID 不能为空".to_string());
  }

  let mut parsed_params = serde_json::Map::new();
  let raw_json = draft.params_json_draft.trim();
  if !raw_json.is_empty() {
    let val: serde_json::Value = serde_json::from_str(raw_json)
      .map_err(|err| format!("parameters JSON 解析失败：{err}"))?;
    match val {
      serde_json::Value::Object(map) => parsed_params = map,
      _ => {
        return Err("parameters JSON 必须是对象（如 {\"temperature\": 0.3}）".to_string());
      }
    }
  }
  if let Some(qf) = draft.quick_field {
    parsed_params.insert(
      qf.param_key().to_string(),
      serde_json::Value::String(draft.quick_value.clone()),
    );
  }

  let timeout_ms = if draft.timeout_draft.trim().is_empty() {
    None
  } else {
    let parsed = draft
      .timeout_draft
      .trim()
      .parse::<u64>()
      .ok()
      .filter(|&v| v > 0)
      .ok_or_else(|| "超时时间 timeout_ms 必须是正整数（毫秒）".to_string())?;
    Some(parsed)
  };

  let max_retries = if draft.retries_draft.trim().is_empty() {
    None
  } else {
    let parsed = draft
      .retries_draft
      .trim()
      .parse::<u32>()
      .map_err(|_| "最大重试次数 max_retries 必须是非负整数".to_string())?;
    Some(parsed)
  };

  let mut input_mapping = BTreeMap::new();
  for row in &draft.mapping_rows {
    let k = row.key.trim();
    if !k.is_empty() {
      input_mapping.insert(k.to_string(), row.expr.clone());
    }
  }

  let run_if_val = {
    let t = draft.run_if_draft.trim();
    if t.is_empty() {
      None
    } else {
      Some(t.to_string())
    }
  };

  Ok(NodeUpdatePatch {
    new_id: (trimmed_id != original.id).then(|| trimmed_id.to_string()),
    node_type: Some(if draft.type_draft.trim().is_empty() {
      original.node_type.clone()
    } else {
      draft.type_draft.trim().to_string()
    }),
    dependencies: Some(draft.dependencies_draft.clone()),
    input_mapping: Some(input_mapping),
    run_if: Some(run_if_val),
    timeout_ms: Some(timeout_ms),
    max_retries: Some(max_retries),
    parameters: Some(parsed_params),
  })
}

#[component]
pub fn NodeConfigEditor(
  node: InspectedNode,
  all_nodes: Vec<InspectedNode>,
  editable: bool,
  on_apply_patch: EventHandler<(String, NodeUpdatePatch)>,
  #[props(default)] on_delete_node: Option<EventHandler<String>>,
) -> Element {
  let node_sig = serde_json::to_string(&node).unwrap_or_default();
  let mut draft = use_signal(|| create_node_editor_draft(&node));
  let mut local_error = use_signal(|| None::<String>);
  let mut saved_toast = use_signal(|| false);

  let node_for_effect = node.clone();
  use_effect(use_reactive((&node_sig,), move |_| {
    draft.set(create_node_editor_draft(&node_for_effect));
    local_error.set(None);
    saved_toast.set(false);
  }));

  let other_nodes: Vec<InspectedNode> = all_nodes
    .into_iter()
    .filter(|c| c.id != node.id)
    .collect();

  let cur_draft = draft();
  let mut type_options: Vec<(String, String)> = node_palette()
    .into_iter()
    .map(|p| (p.node_type.to_string(), p.label.to_string()))
    .collect();
  if !type_options.iter().any(|(v, _)| v == &cur_draft.type_draft) {
    type_options.insert(
      0,
      (
        cur_draft.type_draft.clone(),
        format!("{} (自定义)", cur_draft.type_draft),
      ),
    );
  }

  let node_for_apply = node.clone();
  let node_id_del = node.id.clone();
  let other_first_id = other_nodes.first().map(|n| n.id.clone());

  rsx! {
    div { class: "space-y-3", "data-testid": "node-config-editor",
      if !editable {
        Alert { variant: AlertVariant::Default,
          AlertDescription { "工作流运行中已锁定拓扑结构编辑，运行结束或终止后可随时修改同步至 YAML。" }
        }
      }
      if let Some(err) = local_error() {
        Alert { variant: AlertVariant::Destructive,
          AlertDescription { "{err}" }
        }
      }

      // ID + Type row
      div { class: "grid grid-cols-1 sm:grid-cols-2 gap-2.5",
        div { class: "space-y-1",
          label { class: "text-xs font-medium text-foreground block",
            "节点 ID "
            span { class: "text-[11px] font-normal text-muted-foreground", "（级联更新引用）" }
          }
          Input {
            class: "h-8 text-xs font-mono".to_string(),
            disabled: !editable,
            value: cur_draft.id_draft.clone(),
            "data-testid": "node-config-id-input",
            on_value_change: move |v: String| {
              let mut next = draft.peek().clone();
              next.id_draft = v;
              draft.set(next);
              saved_toast.set(false);
            },
          }
        }
        div { class: "space-y-1",
          label { class: "text-xs font-medium text-foreground block", "节点类型 (type)" }
          NativeSelect {
            size: NativeSelectSize::Sm,
            disabled: !editable,
            "data-testid": "node-config-type-select",
            on_value_change: move |v: String| {
              let mut next = draft.peek().clone();
              next.quick_field = detect_quick_text_field(&v, &serde_json::Map::new());
              next.type_draft = v;
              draft.set(next);
              saved_toast.set(false);
            },
            for (val, lbl) in type_options {
              NativeSelectOption {
                key: "{val}",
                value: val.clone(),
                selected: cur_draft.type_draft == val,
                "{lbl}"
              }
            }
          }
        }
      }

      // Dependencies pills
      div { class: "space-y-1.5",
        div { class: "flex items-center justify-between text-xs",
          span { class: "font-medium", "上游依赖节点 (dependencies)" }
          span { class: "text-[11px] text-muted-foreground", "也可在画布点端口连线" }
        }
        if other_nodes.is_empty() {
          p { class: "text-xs text-muted-foreground", "当前画布仅有 1 个节点，添加更多节点后可配置依赖。" }
        } else {
          div { class: "flex flex-wrap gap-1.5",
            for cand in &other_nodes {
              {
                let cid = cand.id.clone();
                let checked = cur_draft.dependencies_draft.iter().any(|d| d == &cid);
                rsx! {
                  Button {
                    key: "dep-{cid}",
                    variant: if checked { ButtonVariant::Secondary } else { ButtonVariant::Outline },
                    size: ButtonSize::Sm,
                    class: "h-6 px-2 text-[11px] font-mono".to_string(),
                    disabled: !editable,
                    onclick: move |_| {
                      let mut next = draft.peek().clone();
                      if let Some(pos) = next.dependencies_draft.iter().position(|d| d == &cid) {
                        next.dependencies_draft.remove(pos);
                      } else {
                        next.dependencies_draft.push(cid.clone());
                      }
                      draft.set(next);
                      saved_toast.set(false);
                    },
                    if checked { "✓ {cand.id}" } else { "+ {cand.id}" }
                  }
                }
              }
            }
          }
        }
      }

      // Dynamic input_mapping rows
      div { class: "space-y-1.5",
        div { class: "flex items-center justify-between text-xs",
          span { class: "font-medium", "输入字段映射 (input_mapping)" }
          if editable {
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "h-6 px-2 text-[11px]".to_string(),
              onclick: move |_| {
                let mut next = draft.peek().clone();
                let first_up = next
                  .dependencies_draft
                  .first()
                  .cloned()
                  .or_else(|| other_first_id.clone());
                let expr = match first_up {
                  Some(up) => format!("{{{{ nodes.{up}.outputs.output }}}}"),
                  None => "{{ inputs.topic }}".to_string(),
                };
                let key = format!("arg_{}", next.mapping_rows.len() + 1);
                next.mapping_rows.push(MappingRowDraft { key, expr });
                draft.set(next);
                saved_toast.set(false);
              },
              "+ 新增映射项"
            }
          }
        }
        if cur_draft.mapping_rows.is_empty() {
          p { class: "text-xs text-muted-foreground", "暂无显式字段映射（节点直接读取全局 inputs 或静态参数）。" }
        } else {
          div { class: "space-y-1.5",
            for (idx, row) in cur_draft.mapping_rows.iter().enumerate() {
              div { key: "map-{idx}", class: "flex items-center gap-1.5",
                Input {
                  class: "h-7 text-xs font-mono w-28 shrink-0".to_string(),
                  placeholder: "字段名".to_string(),
                  disabled: !editable,
                  value: row.key.clone(),
                  on_value_change: move |v: String| {
                    let mut next = draft.peek().clone();
                    if let Some(r) = next.mapping_rows.get_mut(idx) {
                      r.key = v;
                    }
                    draft.set(next);
                    saved_toast.set(false);
                  },
                }
                span { class: "text-xs text-muted-foreground", "←" }
                Input {
                  class: "h-7 text-xs font-mono flex-1".to_string(),
                  placeholder: "{{ nodes.prev.outputs.output }}".to_string(),
                  disabled: !editable,
                  value: row.expr.clone(),
                  on_value_change: move |v: String| {
                    let mut next = draft.peek().clone();
                    if let Some(r) = next.mapping_rows.get_mut(idx) {
                      r.expr = v;
                    }
                    draft.set(next);
                    saved_toast.set(false);
                  },
                }
                if editable {
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "h-7 w-7 p-0 text-xs text-destructive".to_string(),
                    onclick: move |_| {
                      let mut next = draft.peek().clone();
                      if idx < next.mapping_rows.len() {
                        next.mapping_rows.remove(idx);
                      }
                      draft.set(next);
                      saved_toast.set(false);
                    },
                    "×"
                  }
                }
              }
            }
          }
        }
      }

      // Quick prompt / template editor
      if let Some(qf) = cur_draft.quick_field {
        div { class: "space-y-1",
          label { class: "text-xs font-medium text-foreground block", "{qf.label()}" }
          Textarea {
            class: "min-h-[96px] text-xs font-mono".to_string(),
            disabled: !editable,
            value: cur_draft.quick_value.clone(),
            "data-testid": "node-config-quick-textarea",
            on_value_change: move |v: String| {
              let mut next = draft.peek().clone();
              next.quick_value = v;
              draft.set(next);
              saved_toast.set(false);
            },
          }
        }
      }

      // Additional parameters JSON
      div { class: "space-y-1",
        label { class: "text-xs font-medium text-foreground block",
          if cur_draft.quick_field.is_some() {
            "附加静态参数 JSON (parameters)"
          } else {
            "静态参数 JSON (parameters)"
          }
        }
        Textarea {
          class: "min-h-[84px] text-xs font-mono".to_string(),
          disabled: !editable,
          value: cur_draft.params_json_draft.clone(),
          on_value_change: move |v: String| {
            let mut next = draft.peek().clone();
            next.params_json_draft = v;
            draft.set(next);
            saved_toast.set(false);
          },
        }
      }

      // run_if + timeout + retries
      div { class: "space-y-2",
        div { class: "space-y-1",
          label { class: "text-xs font-medium text-foreground block", "条件守卫 (run_if)" }
          Input {
            class: "h-8 text-xs font-mono".to_string(),
            placeholder: "例如 {{ nodes.check.outputs.passed }}".to_string(),
            disabled: !editable,
            value: cur_draft.run_if_draft.clone(),
            on_value_change: move |v: String| {
              let mut next = draft.peek().clone();
              next.run_if_draft = v;
              draft.set(next);
              saved_toast.set(false);
            },
          }
        }
        div { class: "grid grid-cols-2 gap-2",
          div { class: "space-y-1",
            label { class: "text-xs font-medium text-foreground block", "超时毫秒 (timeout_ms)" }
            Input {
              class: "h-8 text-xs font-mono".to_string(),
              placeholder: "默认无额外限制".to_string(),
              disabled: !editable,
              value: cur_draft.timeout_draft.clone(),
              on_value_change: move |v: String| {
                let mut next = draft.peek().clone();
                next.timeout_draft = v;
                draft.set(next);
                saved_toast.set(false);
              },
            }
          }
          div { class: "space-y-1",
            label { class: "text-xs font-medium text-foreground block", "最大重试 (max_retries)" }
            Input {
              class: "h-8 text-xs font-mono".to_string(),
              placeholder: "0".to_string(),
              disabled: !editable,
              value: cur_draft.retries_draft.clone(),
              on_value_change: move |v: String| {
                let mut next = draft.peek().clone();
                next.retries_draft = v;
                draft.set(next);
                saved_toast.set(false);
              },
            }
          }
        }
      }

      if !node.sub_nodes.is_empty() {
        {
          let sub_summary = node
            .sub_nodes
            .iter()
            .map(|s| format!("{} ({})", s.id, s.node_type))
            .collect::<Vec<_>>()
            .join(" → ");
          rsx! {
            div { class: "rounded-md border border-border bg-muted/40 p-2 text-xs text-muted-foreground",
              div { class: "font-semibold text-foreground", "内嵌子流水线 ({node.sub_nodes.len()} 个子节点自动保留)" }
              p { class: "mt-0.5 font-mono text-[11px]", "{sub_summary}" }
            }
          }
        }
      }

      if editable {
        div { class: "flex flex-wrap items-center gap-2 pt-2 border-t border-border",
          Button {
            variant: ButtonVariant::Primary,
            size: ButtonSize::Sm,
            "data-testid": "apply-node-config-btn",
            onclick: move |_| {
              let current = draft.peek().clone();
              match build_node_patch_from_draft(&node_for_apply, &current) {
                Ok(patch) => {
                  local_error.set(None);
                  on_apply_patch.call((node_for_apply.id.clone(), patch));
                  saved_toast.set(true);
                }
                Err(err) => local_error.set(Some(err)),
              }
            },
            "💾 同步到 YAML"
          }
          if saved_toast() {
            span { class: "text-xs text-emerald-600 dark:text-emerald-400 font-medium",
              "✓ 已同步至 YAML 与画布"
            }
          }
          if let Some(del_cb) = on_delete_node {
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "ml-auto text-destructive hover:text-destructive".to_string(),
              "data-testid": "delete-node-btn",
              onclick: move |_| del_cb.call(node_id_del.clone()),
              "🗑 删除节点"
            }
          }
        }
      }
    }
  }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InspectorTab {
  Io,
  Media,
  Config,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BeforeOverrideKind {
  Input,
  MockOutput,
}

#[component]
pub fn NodeIoInspector(
  run_id: String,
  api_token: String,
  tenant_id: String,
  node: Option<InspectedNode>,
  #[props(default)] all_nodes: Vec<InspectedNode>,
  #[props(default)] editable: bool,
  #[props(default)] telemetry: Option<NodeRuntimeTelemetry>,
  #[props(default)] captured_input: Option<serde_json::Value>,
  #[props(default)] captured_output: Option<serde_json::Value>,
  #[props(default)] paused_at: Option<FlowDebugPause>,
  #[props(default)] has_breakpoint: bool,
  on_toggle_breakpoint: EventHandler<String>,
  on_step_debug: EventHandler<Option<DebugResumeOverrides>>,
  on_continue_debug: EventHandler<Option<DebugResumeOverrides>>,
  #[props(default)] debug_action_busy: bool,
  #[props(default)] on_apply_node_patch: Option<EventHandler<(String, NodeUpdatePatch)>>,
  #[props(default)] on_delete_node: Option<EventHandler<String>>,
  #[props(default)] on_expand_drawer: Option<EventHandler<()>>,
  #[props(default)] on_collapse_pane: Option<EventHandler<()>>,
  #[props(default)] in_drawer: bool,
) -> Element {
  let mut active_tab = use_signal(|| {
    if run_id.is_empty() && on_apply_node_patch.is_some() {
      InspectorTab::Config
    } else {
      InspectorTab::Io
    }
  });
  let mut override_open = use_signal(|| false);
  let mut before_kind = use_signal(|| BeforeOverrideKind::Input);
  let mut override_json = use_signal(|| "{\n  \"output\": \"\"\n}".to_string());
  let mut override_error = use_signal(|| None::<String>);

  let pause_key = paused_at
    .as_ref()
    .map(|p| format!("{}:{}", p.node_id, p.phase))
    .unwrap_or_default();
  let run_id_for_eff = run_id.clone();
  use_effect(use_reactive((&run_id_for_eff, &pause_key), move |(rid, pk)| {
    if !rid.is_empty() || !pk.is_empty() {
      active_tab.set(InspectorTab::Io);
    }
  }));

  let node_id_eff = node.as_ref().map(|n| n.id.clone()).unwrap_or_default();
  let cap_in_eff = captured_input.clone();
  let cap_out_eff = captured_output.clone();
  let paused_eff = paused_at.clone();
  use_effect(use_reactive((&node_id_eff, &pause_key), move |(nid, _)| {
    override_open.set(false);
    override_error.set(None);
    if let Some(p) = &paused_eff
      && p.node_id == nid
    {
      if p.phase == "before_node" {
        before_kind.set(BeforeOverrideKind::Input);
        let unwrapped = unwrap_captured_flow_json(cap_in_eff.as_ref());
        override_json.set(
          serde_json::to_string_pretty(&serde_json::Value::Object(unwrapped))
            .unwrap_or_else(|_| "{}".to_string()),
        );
      } else {
        let unwrapped = unwrap_captured_flow_json(cap_out_eff.as_ref());
        if unwrapped.is_empty() {
          override_json.set("{\n  \"output\": \"\"\n}".to_string());
        } else {
          override_json.set(
            serde_json::to_string_pretty(&serde_json::Value::Object(unwrapped))
              .unwrap_or_else(|_| "{\n  \"output\": \"\"\n}".to_string()),
          );
        }
      }
    }
  }));

  let Some(node) = node else {
    return rsx! {
      section { class: "h-full flex flex-col bg-card", "aria-label": "节点配置与观测台",
        div { class: "flex items-center justify-between px-3 py-2 border-b border-border",
          strong { class: "text-xs font-semibold", "节点配置与观测台" }
          if let Some(col_cb) = on_collapse_pane {
            Button {
              variant: ButtonVariant::Ghost,
              size: ButtonSize::Sm,
              class: "h-6 px-2 text-xs".to_string(),
              onclick: move |_| col_cb.call(()),
              "收起"
            }
          }
        }
        div { class: "p-5 text-xs text-muted-foreground space-y-2",
          h3 { class: "text-sm font-semibold text-foreground", "节点配置与 I/O 观测台" }
          p {
            "点击拓扑画布中的任意节点即可表单化编辑参数与依赖连线（实时同步左侧 YAML）；启动「单步调试」后在此实时核查输入、输出与多媒体产物。"
          }
        }
      }
    };
  };

  let media_previews = extract_node_media_previews(&run_id, &node.id, captured_output.as_ref());
  let is_paused_here = paused_at
    .as_ref()
    .is_some_and(|p| p.node_id == node.id);
  let cur_tab = if active_tab() == InspectorTab::Media && media_previews.is_empty() {
    InspectorTab::Io
  } else {
    active_tab()
  };

  let nid_bp = node.id.clone();
  let pause_for_override = paused_at.clone();
  let cap_in_for_btn = captured_input.clone();

  rsx! {
    section { class: "h-full flex flex-col bg-card overflow-hidden", "aria-label": "节点 {node.id} 观测详情",
      // Header
      header { class: "flex items-start justify-between gap-2 px-3 py-2.5 border-b border-border bg-muted/20",
        div { class: "min-w-0 space-y-0.5",
          div { class: "flex items-center gap-1.5 flex-wrap",
            Badge { variant: BadgeVariant::Secondary, class: "text-[10px]".to_string(), "{node.node_type}" }
            if let Some(ms) = telemetry.as_ref().and_then(|t| t.duration_ms) {
              Badge { variant: BadgeVariant::Outline, class: "text-[10px] font-mono".to_string(), "{ms} ms" }
            }
            if let Some(cost) = telemetry.as_ref().and_then(|t| t.cost_usd).filter(|c| *c > 0.0) {
              Badge { variant: BadgeVariant::Outline, class: "text-[10px] font-mono".to_string(),
                if cost < 0.1 { "${cost:.4}" } else { "${cost:.3}" }
              }
            }
          }
          h2 { class: "text-sm font-bold font-mono truncate", "{node.id}" }
        }
        div { class: "flex items-center gap-1 shrink-0 flex-wrap justify-end",
          if !in_drawer {
            if let Some(exp_cb) = on_expand_drawer {
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                "data-testid": "inspector-expand-drawer-btn",
                onclick: move |_| exp_cb.call(()),
                "⤢ 放大"
              }
            }
            if let Some(col_cb) = on_collapse_pane {
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                "data-testid": "inspector-collapse-pane-btn",
                onclick: move |_| col_cb.call(()),
                "收起"
              }
            }
          }
          Button {
            variant: if has_breakpoint { ButtonVariant::Destructive } else { ButtonVariant::Outline },
            size: ButtonSize::Sm,
            class: "h-7 px-2 text-xs".to_string(),
            onclick: move |_| on_toggle_breakpoint.call(nid_bp.clone()),
            if has_breakpoint { "● 已设断点" } else { "○ 设断点" }
          }
        }
      }

      // Active Debug Pause Callout + Override Panel
      if is_paused_here {
        if let Some(pause) = paused_at.clone() {
          div {
            class: "m-3 mb-0 rounded-md border border-amber-500/50 bg-amber-500/10 p-3 space-y-2.5",
            role: "status",
            "data-testid": "inspector-pause-callout",
            div { class: "space-y-1",
              strong { class: "text-xs text-amber-800 dark:text-amber-300 block",
                if pause.phase == "before_node" {
                  "⏸ 已在执行前断点挂起：{node.id}"
                } else {
                  "⏸ 本步执行完毕并暂停：{node.id}"
                }
              }
              p { class: "text-[11px] text-muted-foreground leading-relaxed",
                if pause.phase == "before_node" {
                  "调度器已完成上游表达式求值，可核查下方输入直接放行，或修改输入 / Mock 输出跳过真实外部调用。"
                } else {
                  "请观测下方实际输入与输出；如需修正结果供下游消费，可展开数据注入覆盖本步输出。"
                }
                if !pause.next_nodes.is_empty() {
                  " 下一就绪节点：{pause.next_nodes.join(\"、\")}。"
                }
              }
            }

            div { class: "flex flex-wrap items-center gap-1.5",
              Button {
                variant: ButtonVariant::Primary,
                size: ButtonSize::Sm,
                class: "h-7 px-2.5 text-xs".to_string(),
                disabled: debug_action_busy,
                onclick: move |_| on_step_debug.call(None),
                "▶ 单步下一步"
              }
              Button {
                variant: ButtonVariant::Secondary,
                size: ButtonSize::Sm,
                class: "h-7 px-2.5 text-xs".to_string(),
                disabled: debug_action_busy,
                onclick: move |_| on_continue_debug.call(None),
                "⏭ 继续跑完"
              }
              Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::Sm,
                class: "h-7 px-2 text-xs".to_string(),
                "data-testid": "toggle-debug-override-btn",
                onclick: move |_| override_open.set(!override_open()),
                if override_open() {
                  "收起注入"
                } else if pause.phase == "before_node" {
                  "🧪 改输入 / Mock 跳过"
                } else {
                  "🧪 覆盖输出"
                }
              }
            }

            if override_open() {
              div {
                class: "rounded border border-border bg-background p-2.5 space-y-2",
                "data-testid": "debug-override-panel",
                if pause.phase == "before_node" {
                  div { class: "flex flex-wrap gap-1.5",
                    Button {
                      variant: if before_kind() == BeforeOverrideKind::Input { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                      size: ButtonSize::Sm,
                      class: "h-6 px-2 text-[11px]".to_string(),
                      onclick: move |_| {
                        before_kind.set(BeforeOverrideKind::Input);
                        override_error.set(None);
                        let unwrapped = unwrap_captured_flow_json(cap_in_for_btn.as_ref());
                        override_json.set(
                          serde_json::to_string_pretty(&serde_json::Value::Object(unwrapped))
                            .unwrap_or_else(|_| "{}".to_string()),
                        );
                      },
                      "✏ 修改输入 (input_override)"
                    }
                    Button {
                      variant: if before_kind() == BeforeOverrideKind::MockOutput { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
                      size: ButtonSize::Sm,
                      class: "h-6 px-2 text-[11px]".to_string(),
                      onclick: move |_| {
                        before_kind.set(BeforeOverrideKind::MockOutput);
                        override_error.set(None);
                        override_json.set("{\n  \"output\": \"mocked_value\"\n}".to_string());
                      },
                      "⚡ Mock 跳过 (output_override)"
                    }
                  }
                } else {
                  p { class: "text-[11px] text-muted-foreground",
                    "将替换 #{node.id} 在状态池与 Checkpoint 中的输出，下游节点将读取下方 JSON："
                  }
                }

                Textarea {
                  class: "min-h-[92px] text-xs font-mono".to_string(),
                  value: override_json(),
                  "data-testid": "debug-override-textarea",
                  on_value_change: move |v: String| override_json.set(v),
                }

                if let Some(err) = override_error() {
                  p { class: "text-xs text-destructive", role: "alert", "{err}" }
                }

                div { class: "flex flex-wrap gap-1.5",
                  {
                    let p_step = pause_for_override.clone();
                    let p_cont = pause_for_override.clone();
                    rsx! {
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2.5 text-xs".to_string(),
                        "data-testid": "apply-override-step-btn",
                        disabled: debug_action_busy,
                        onclick: move |_| {
                          if let Some(p) = &p_step {
                            match parse_debug_override_object(&override_json()) {
                              Ok(map) => {
                                override_error.set(None);
                                let ov = if p.phase == "before_node" && before_kind() == BeforeOverrideKind::Input {
                                  DebugResumeOverrides { input_override: Some(map), output_override: None }
                                } else {
                                  DebugResumeOverrides { input_override: None, output_override: Some(map) }
                                };
                                on_step_debug.call(Some(ov));
                              }
                              Err(err) => override_error.set(Some(err)),
                            }
                          }
                        },
                        if pause.phase == "before_node" && before_kind() == BeforeOverrideKind::MockOutput {
                          "⚡ Mock 跳过并单步"
                        } else {
                          "▶ 注入并单步"
                        }
                      }
                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        class: "h-7 px-2.5 text-xs".to_string(),
                        "data-testid": "apply-override-continue-btn",
                        disabled: debug_action_busy,
                        onclick: move |_| {
                          if let Some(p) = &p_cont {
                            match parse_debug_override_object(&override_json()) {
                              Ok(map) => {
                                override_error.set(None);
                                let ov = if p.phase == "before_node" && before_kind() == BeforeOverrideKind::Input {
                                  DebugResumeOverrides { input_override: Some(map), output_override: None }
                                } else {
                                  DebugResumeOverrides { input_override: None, output_override: Some(map) }
                                };
                                on_continue_debug.call(Some(ov));
                              }
                              Err(err) => override_error.set(Some(err)),
                            }
                          }
                        },
                        "⏭ 注入并继续跑完"
                      }
                    }
                  }
                }
              }
            }
          }
        }
      }

      // Node Error / Skip Reason
      if let Some(err) = telemetry.as_ref().and_then(|t| t.error.clone()) {
        div { class: "px-3 pt-2",
          Alert { variant: AlertVariant::Destructive,
            AlertDescription { "节点执行报错：{err}" }
          }
        }
      }
      if let Some(reason) = telemetry.as_ref().and_then(|t| t.skip_reason.clone()) {
        div { class: "px-3 pt-2",
          Alert { variant: AlertVariant::Default,
            AlertDescription { "条件跳过原因：{reason}" }
          }
        }
      }

      // 3-Tab Switcher
      div { class: "flex items-center gap-1 px-3 py-2 border-b border-border", role: "tablist",
        Button {
          variant: if cur_tab == InspectorTab::Io { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
          size: ButtonSize::Sm,
          class: "h-7 px-2.5 text-xs".to_string(),
          "data-testid": "inspector-tab-io",
          onclick: move |_| active_tab.set(InspectorTab::Io),
          "运行 I/O 观测"
        }
        Button {
          variant: if cur_tab == InspectorTab::Media { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
          size: ButtonSize::Sm,
          class: "h-7 px-2.5 text-xs".to_string(),
          "data-testid": "inspector-tab-media",
          onclick: move |_| active_tab.set(InspectorTab::Media),
          "媒体预览"
          if !media_previews.is_empty() {
            Badge { variant: BadgeVariant::Secondary, class: "ml-1 text-[10px]".to_string(), "{media_previews.len()}" }
          }
        }
        Button {
          variant: if cur_tab == InspectorTab::Config { ButtonVariant::Secondary } else { ButtonVariant::Ghost },
          size: ButtonSize::Sm,
          class: "h-7 px-2.5 text-xs".to_string(),
          "data-testid": "inspector-tab-config",
          onclick: move |_| active_tab.set(InspectorTab::Config),
          "节点配置"
        }
      }

      // Tab Content Body
      div { class: "flex-1 overflow-y-auto p-3 space-y-3",
        match cur_tab {
          InspectorTab::Io => rsx! {
            div { class: "space-y-3",
              div { class: "space-y-1.5",
                div { class: "flex items-center justify-between text-xs",
                  span { class: "font-semibold", "实际输入 Input（表达式求值后）" }
                  span { class: "text-[11px] text-muted-foreground",
                    if captured_input.is_some() { "已捕获" } else { "等待节点启动" }
                  }
                }
                if let Some(inp) = &captured_input {
                  pre {
                    class: "rounded-md border border-border bg-muted/40 p-2.5 text-[11px] font-mono whitespace-pre-wrap break-words max-h-60 overflow-y-auto",
                    "data-testid": "inspector-node-input",
                    "{serde_json::to_string_pretty(inp).unwrap_or_default()}"
                  }
                } else if !node.input_mapping.is_empty() {
                  pre { class: "rounded-md border border-border bg-muted/20 p-2 text-[11px] font-mono text-muted-foreground whitespace-pre-wrap",
                    "{serde_json::to_string_pretty(&node.input_mapping).unwrap_or_default()}"
                  }
                } else {
                  p { class: "text-xs text-muted-foreground", "尚未执行到该节点（直接读取全局 inputs 与静态 parameters）。" }
                }
              }

              div { class: "space-y-1.5",
                div { class: "flex items-center justify-between text-xs",
                  span { class: "font-semibold", "实际输出 Output（节点返回载荷）" }
                  span { class: "text-[11px] text-muted-foreground",
                    if captured_output.is_some() {
                      if media_previews.is_empty() {
                        "已捕获"
                      } else {
                        "已捕获 · 含 {media_previews.len()} 项媒体产物"
                      }
                    } else {
                      "等待节点完成"
                    }
                  }
                }
                if let Some(out) = &captured_output {
                  pre {
                    class: "rounded-md border border-border bg-muted/40 p-2.5 text-[11px] font-mono whitespace-pre-wrap break-words max-h-72 overflow-y-auto",
                    "data-testid": "inspector-node-output",
                    "{serde_json::to_string_pretty(out).unwrap_or_default()}"
                  }
                } else {
                  p { class: "text-xs text-muted-foreground", "节点完成后将在此展示完整输出 JSON。" }
                }
              }

              if !media_previews.is_empty() {
                div { class: "space-y-2",
                  div { class: "flex items-center justify-between text-xs",
                    span { class: "font-semibold", "即时产物快览" }
                    Button {
                      variant: ButtonVariant::Ghost,
                      size: ButtonSize::Sm,
                      class: "h-6 px-2 text-xs".to_string(),
                      onclick: move |_| active_tab.set(InspectorTab::Media),
                      "查看全部 {media_previews.len()} 项 →"
                    }
                  }
                  for item in media_previews.iter().take(2).cloned() {
                    AuthorizedMedia {
                      key: "{item.key}",
                      item,
                      api_token: api_token.clone(),
                      tenant_id: tenant_id.clone(),
                    }
                  }
                }
              }
            }
          },
          InspectorTab::Media => rsx! {
            div { class: "space-y-2.5",
              if media_previews.is_empty() {
                p { class: "text-xs text-muted-foreground", "当前节点暂无图像、音频、视频或交互式 HTML 产物。" }
              } else {
                for item in media_previews {
                  AuthorizedMedia {
                    key: "{item.key}",
                    item,
                    api_token: api_token.clone(),
                    tenant_id: tenant_id.clone(),
                  }
                }
              }
            }
          },
          InspectorTab::Config => rsx! {
            if let Some(patch_cb) = on_apply_node_patch {
              NodeConfigEditor {
                node: node.clone(),
                all_nodes,
                editable,
                on_apply_patch: patch_cb,
                on_delete_node,
              }
            } else {
              pre { class: "rounded-md border border-border bg-muted/40 p-2.5 text-[11px] font-mono whitespace-pre-wrap",
                "{serde_json::to_string_pretty(&node.parameters).unwrap_or_default()}"
              }
            }
          },
        }
      }
    }
  }
}
