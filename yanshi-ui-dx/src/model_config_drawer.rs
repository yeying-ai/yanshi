//! Global `ModelConfigDrawer` migrated to Dioxus 0.7 + `dioxus-shadcn`.
//! Consumes `Sheet` with `SheetSize::Xl` <-> `SheetSize::Full` expandable
//! presets, `Tabs`, `Input`, `Checkbox`, `NativeSelect`, `Button`, `Badge`,
//! and `Alert`.

use dioxus::prelude::*;
use dioxus_shadcn::{
  Alert, AlertDescription, AlertVariant, Badge, BadgeVariant, Button, ButtonSize, ButtonVariant,
  Checkbox, Input, InputSize, Label, NativeSelect, NativeSelectOption, Separator, Sheet,
  SheetClose, SheetContent, SheetDescription, SheetHeader, SheetOverlay, SheetSide, SheetSize,
  SheetTitle, Tabs, TabsContent, TabsList, TabsTrigger,
};

use crate::api::{
  ModelConfigItem, ModelConfigSnapshot, ProviderConfigItem, TestProviderOutcome, delete_model_item,
  delete_provider, fetch_models_config, put_model_item, put_provider, test_provider,
};

struct ProviderPreset {
  label: &'static str,
  name: &'static str,
  protocol: &'static str,
  base_url: &'static str,
  env_var: &'static str,
}

const CUSTOM_PROVIDER_PRESETS: &[ProviderPreset] = &[
  ProviderPreset {
    label: "OpenRouter",
    name: "openrouter",
    protocol: "openai",
    base_url: "https://openrouter.ai/api/v1",
    env_var: "OPENROUTER_API_KEY",
  },
  ProviderPreset {
    label: "Ollama (本地)",
    name: "ollama",
    protocol: "openai",
    base_url: "http://127.0.0.1:11434/v1",
    env_var: "OLLAMA_API_KEY",
  },
  ProviderPreset {
    label: "SiliconFlow",
    name: "siliconflow",
    protocol: "openai",
    base_url: "https://api.siliconflow.cn/v1",
    env_var: "SILICONFLOW_API_KEY",
  },
  ProviderPreset {
    label: "OneAPI / 中转",
    name: "oneapi",
    protocol: "openai",
    base_url: "https://your-relay.example.com/v1",
    env_var: "ONEAPI_API_KEY",
  },
];

const MODEL_TYPES: &[&str] = &[
  "chat",
  "embedding",
  "text_to_image",
  "image_to_image",
  "image_edit",
  "text_to_video",
  "tts",
  "asr",
];

const INPUT_MODALITIES: &[&str] = &["text", "image", "audio", "video", "document"];

#[component]
pub fn ModelConfigDrawer(
  open: bool,
  on_open_change: EventHandler<bool>,
  api_token: String,
  on_models_updated: EventHandler<ModelConfigSnapshot>,
) -> Element {
  let mut expanded = use_signal(|| false);
  let mut active_tab = use_signal(|| "providers".to_string());
  let mut snapshot = use_signal(|| None::<ModelConfigSnapshot>);
  let mut loading = use_signal(|| false);
  let mut error = use_signal(|| None::<String>);
  let mut notice = use_signal(|| None::<String>);

  // Provider editor state
  let mut selected_provider = use_signal(|| "openai".to_string());
  let mut creating_custom_provider = use_signal(|| false);
  let mut provider_name_draft = use_signal(String::new);
  let mut protocol_draft = use_signal(|| "openai".to_string());
  let mut base_url_draft = use_signal(String::new);
  let mut api_key_env_draft = use_signal(String::new);
  let mut api_key_draft = use_signal(String::new);
  let mut show_api_key = use_signal(|| false);
  let mut timeout_draft = use_signal(String::new);
  let mut rpm_draft = use_signal(String::new);
  let mut save_busy = use_signal(|| false);
  let mut test_busy = use_signal(|| false);
  let mut test_outcome = use_signal(|| None::<TestProviderOutcome>);

  // Model editor state
  let mut model_search = use_signal(String::new);
  let mut model_vendor_filter = use_signal(|| "all".to_string());
  let mut editing_model_name = use_signal(|| None::<String>);
  let mut creating_model = use_signal(|| false);
  let mut model_name_draft = use_signal(String::new);
  let mut model_vendor_draft = use_signal(|| "openai".to_string());
  let mut model_id_draft = use_signal(String::new);
  let mut model_type_draft = use_signal(|| "chat".to_string());
  let mut model_accepts_draft = use_signal(|| vec!["text".to_string()]);
  let mut model_base_url_draft = use_signal(String::new);
  let mut model_temp_draft = use_signal(String::new);
  let mut model_max_tokens_draft = use_signal(String::new);
  let mut model_tools_draft = use_signal(|| true);
  let mut model_thinking_draft = use_signal(|| false);

  let mut hydrate_provider = move |item: &ProviderConfigItem| {
    creating_custom_provider.set(false);
    selected_provider.set(item.name.clone());
    provider_name_draft.set(item.name.clone());
    let proto = if item.protocol == "anthropic" || item.protocol == "google" {
      item.protocol.clone()
    } else {
      "openai".to_string()
    };
    protocol_draft.set(proto);
    base_url_draft.set(
      item
        .base_url
        .clone()
        .or_else(|| item.default_base_url.clone())
        .unwrap_or_default(),
    );
    api_key_env_draft.set(item.api_key_env.clone());
    api_key_draft.set(String::new());
    show_api_key.set(false);
    timeout_draft.set(
      item
        .timeout_seconds
        .map(|v| v.to_string())
        .unwrap_or_default(),
    );
    rpm_draft.set(
      item
        .requests_per_minute
        .map(|v| v.to_string())
        .unwrap_or_default(),
    );
    test_outcome.set(None);
  };

  let mut hydrate_model = move |item: &ModelConfigItem| {
    creating_model.set(false);
    editing_model_name.set(Some(item.name.clone()));
    model_name_draft.set(item.name.clone());
    model_vendor_draft.set(item.vendor.clone());
    model_id_draft.set(item.model_id.clone());
    model_type_draft.set(if item.model_type.is_empty() {
      "chat".to_string()
    } else {
      item.model_type.clone()
    });
    model_accepts_draft.set(if item.accepts.is_empty() {
      vec!["text".to_string()]
    } else {
      item.accepts.clone()
    });
    model_base_url_draft.set(item.base_url.clone().unwrap_or_default());
    model_temp_draft.set(item.temperature.map(|v| v.to_string()).unwrap_or_default());
    model_max_tokens_draft.set(item.max_tokens.map(|v| v.to_string()).unwrap_or_default());
    model_tools_draft.set(item.supports_tools);
    model_thinking_draft.set(item.supports_thinking);
  };

  let token_for_load = api_token.clone();
  use_effect(use_reactive(
    (&open, &token_for_load),
    move |(is_open, tok)| {
      if !is_open {
        return;
      }
      notice.set(None);
      error.set(None);
      loading.set(true);
      spawn(async move {
        match fetch_models_config(&tok, "").await {
          Ok(data) => {
            let sel = selected_provider();
            if !creating_custom_provider()
              && let Some(found) = data
                .providers
                .iter()
                .find(|p| p.name == sel)
                .or_else(|| data.providers.first())
            {
              hydrate_provider(found);
            }
            on_models_updated.call(data.clone());
            snapshot.set(Some(data));
          }
          Err(err) => {
            error.set(Some(err));
          }
        }
        loading.set(false);
      });
    },
  ));

  let snap = snapshot();
  let providers = snap
    .as_ref()
    .map(|s| s.providers.clone())
    .unwrap_or_default();
  let all_models = snap.as_ref().map(|s| s.models.clone()).unwrap_or_default();
  let ready_count = providers.iter().filter(|p| p.api_key_set).count();
  let active_provider = providers
    .iter()
    .find(|p| p.name == selected_provider())
    .cloned();

  let q_lower = model_search().trim().to_ascii_lowercase();
  let vendor_filter = model_vendor_filter();
  let filtered_models: Vec<ModelConfigItem> = all_models
    .iter()
    .filter(|m| {
      if vendor_filter != "all" && m.vendor != vendor_filter {
        return false;
      }
      if q_lower.is_empty() {
        return true;
      }
      m.name.to_ascii_lowercase().contains(&q_lower)
        || m.vendor.to_ascii_lowercase().contains(&q_lower)
        || m.model_id.to_ascii_lowercase().contains(&q_lower)
        || m.model_type.to_ascii_lowercase().contains(&q_lower)
    })
    .cloned()
    .collect();

  let current_editing_model = editing_model_name()
    .as_ref()
    .and_then(|name| all_models.iter().find(|m| &m.name == name))
    .cloned();

  let tok_save_provider = api_token.clone();
  let tok_clear_key = api_token.clone();
  let tok_test_provider = api_token.clone();
  let tok_delete_provider = api_token.clone();
  let tok_save_model = api_token.clone();
  let tok_delete_model = api_token.clone();

  let active_provider_is_custom = active_provider.as_ref().is_some_and(|p| p.is_custom);
  let active_provider_for_save = active_provider.clone();
  let active_provider_for_clear = active_provider.clone();
  let active_provider_for_test = active_provider.clone();
  let active_provider_for_delete = active_provider.clone();

  let sheet_size = if expanded() {
    SheetSize::Full
  } else {
    SheetSize::Xl
  };

  rsx! {
    Sheet {
      open: Some(open),
      on_open_change: move |next: bool| on_open_change.call(next),
      SheetOverlay {}
      SheetContent {
        side: SheetSide::Right,
        size: sheet_size,
        class: "p-0 flex flex-col gap-0 overflow-hidden".to_string(),
        "data-testid": "model-config-drawer",
        SheetHeader { class: "px-5 py-4 border-b border-border bg-card/80 flex-row items-start justify-between gap-3".to_string(),
          div { class: "space-y-1 min-w-0",
            div { class: "flex items-center gap-2 flex-wrap",
              SheetTitle { "模型与厂商配置" }
              if let Some(ref s) = snap {
                Badge { variant: BadgeVariant::Secondary,
                  "{ready_count}/{s.providers.len()} 厂商就绪 · {s.models.len()} 模型"
                }
              }
            }
            SheetDescription {
              if let Some(ref s) = snap {
                "持久化于 {s.models_path} 与 {s.env_path}（与 CLI 互通，保存后热生效）"
              } else {
                "配置各厂商 Base URL、API Key、自定义中转站与模型参数"
              }
            }
          }
          div { class: "flex items-center gap-2 shrink-0 pr-6",
            Button {
              variant: ButtonVariant::Outline,
              size: ButtonSize::Sm,
              "data-testid": "model-drawer-expand-btn",
              onclick: move |_| expanded.set(!expanded()),
              if expanded() { "⤡ 抽屉宽度" } else { "⤢ 全宽模式" }
            }
          }
          SheetClose { class: "top-4 right-4".to_string(), "✕" }
        }

        div { class: "flex-1 overflow-y-auto p-5 space-y-4",
          if let Some(err_msg) = error() {
            Alert { variant: AlertVariant::Destructive,
              AlertDescription {
                div { class: "flex items-center justify-between gap-2",
                  span { "{err_msg}" }
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| error.set(None),
                    "关闭"
                  }
                }
              }
            }
          }
          if let Some(ok_msg) = notice() {
            Alert { variant: AlertVariant::Default,
              AlertDescription {
                div { class: "flex items-center justify-between gap-2",
                  span { "{ok_msg}" }
                  Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    onclick: move |_| notice.set(None),
                    "知道了"
                  }
                }
              }
            }
          }

          Tabs {
            value: Some(active_tab()),
            on_value_change: move |v: String| active_tab.set(v),
            TabsList {
              TabsTrigger { value: "providers".to_string(),
                span { "data-testid": "model-config-tab-providers", "厂商与密钥" }
              }
              TabsTrigger { value: "models".to_string(),
                span { "data-testid": "model-config-tab-models", "模型目录" }
              }
            }

            TabsContent { value: "providers".to_string(), class: "mt-4".to_string(),
              if loading() && snap.is_none() {
                p { class: "text-sm text-muted-foreground", "正在读取当前厂商与模型注册表…" }
              } else {
                div { class: "grid grid-cols-1 md:grid-cols-[240px_minmax(0,1fr)] gap-4",
                  aside { class: "space-y-2 border-r border-border pr-3",
                    Button {
                      variant: ButtonVariant::Secondary,
                      size: ButtonSize::Sm,
                      class: "w-full justify-start".to_string(),
                      "data-testid": "add-custom-provider-btn",
                      onclick: move |_| {
                        creating_custom_provider.set(true);
                        provider_name_draft.set(String::new());
                        protocol_draft.set("openai".to_string());
                        base_url_draft.set(String::new());
                        api_key_env_draft.set(String::new());
                        api_key_draft.set(String::new());
                        timeout_draft.set("180".to_string());
                        rpm_draft.set(String::new());
                        test_outcome.set(None);
                      },
                      "+ 新增自定义厂商 / 中转"
                    }
                    div { class: "space-y-1 max-h-[560px] overflow-y-auto",
                      for p in providers.iter() {
                        {
                          let p_clone = p.clone();
                          let is_sel = !creating_custom_provider() && p.name == selected_provider();
                          let btn_cls = if is_sel {
                            "w-full text-left rounded-md px-2.5 py-2 border border-primary/50 bg-accent/60 transition-colors"
                          } else {
                            "w-full text-left rounded-md px-2.5 py-2 border border-transparent hover:bg-muted/60 transition-colors"
                          };
                          let dot_cls = if p.api_key_set {
                            "inline-block size-2 rounded-full bg-emerald-500 shrink-0"
                          } else {
                            "inline-block size-2 rounded-full bg-muted-foreground/40 shrink-0"
                          };
                          let mask_text = if p.api_key_set {
                            p.api_key_masked.clone().unwrap_or_else(|| "已配 Key".to_string())
                          } else {
                            "未配 Key".to_string()
                          };
                          rsx! {
                            button {
                              key: "{p.name}",
                              r#type: "button",
                              class: btn_cls,
                              "data-testid": "provider-item-{p.name}",
                              onclick: move |_| hydrate_provider(&p_clone),
                              div { class: "flex items-center gap-1.5 text-sm font-medium",
                                span { class: dot_cls }
                                span { class: "truncate", "{p.name}" }
                                if p.is_custom {
                                  Badge { variant: BadgeVariant::Secondary, "自定义" }
                                } else if p.is_overridden {
                                  Badge { variant: BadgeVariant::Outline, "已覆写" }
                                }
                              }
                              div { class: "text-[11px] text-muted-foreground mt-0.5 truncate",
                                "{p.model_count} 模型 · {mask_text}"
                              }
                            }
                          }
                        }
                      }
                    }
                  }

                  section { class: "space-y-4 min-w-0",
                    if creating_custom_provider() {
                      div { class: "flex flex-wrap items-center gap-1.5",
                        span { class: "text-xs text-muted-foreground", "快捷模板：" }
                        for preset in CUSTOM_PROVIDER_PRESETS {
                          Button {
                            key: "{preset.name}",
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            onclick: move |_| {
                              provider_name_draft.set(preset.name.to_string());
                              protocol_draft.set(preset.protocol.to_string());
                              base_url_draft.set(preset.base_url.to_string());
                              api_key_env_draft.set(preset.env_var.to_string());
                            },
                            "{preset.label}"
                          }
                        }
                      }
                    }

                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                      div { class: "space-y-1.5",
                        Label { "厂商标识 (vendor)" }
                        Input {
                          size: InputSize::Sm,
                          value: provider_name_draft(),
                          placeholder: "例如 openrouter / ollama / my-relay".to_string(),
                          disabled: !creating_custom_provider(),
                          "data-testid": "provider-name-input",
                          on_value_change: move |v: String| provider_name_draft.set(v),
                        }
                      }
                      if creating_custom_provider() || active_provider_is_custom {
                        div { class: "space-y-1.5",
                          Label { "兼容协议 (protocol)" }
                          NativeSelect {
                            class: "h-8 text-xs w-full".to_string(),
                            "data-testid": "provider-protocol-select",
                            on_value_change: move |v: String| protocol_draft.set(v),
                            NativeSelectOption { value: "openai".to_string(), selected: protocol_draft() == "openai", "OpenAI 兼容 (/chat/completions)" }
                            NativeSelectOption { value: "anthropic".to_string(), selected: protocol_draft() == "anthropic", "Anthropic 兼容 (/v1/messages)" }
                            NativeSelectOption { value: "google".to_string(), selected: protocol_draft() == "google", "Google Gemini 兼容" }
                          }
                        }
                      }
                    }

                    div { class: "space-y-1.5",
                      div { class: "flex items-center justify-between",
                        Label { "Base URL" }
                        if let Some(default_url) = active_provider.as_ref().and_then(|p| p.default_base_url.clone()) {
                          Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: "h-6 text-xs".to_string(),
                            onclick: move |_| base_url_draft.set(default_url.clone()),
                            "恢复官方默认"
                          }
                        }
                      }
                      Input {
                        size: InputSize::Sm,
                        value: base_url_draft(),
                        placeholder: "https://api.example.com/v1".to_string(),
                        "data-testid": "provider-base-url-input",
                        on_value_change: move |v: String| base_url_draft.set(v),
                      }
                    }

                    div { class: "space-y-1.5",
                      div { class: "flex items-center justify-between",
                        Label { "API Key（写入 ~/.yanshi/.env，权限 0600）" }
                        div { class: "flex items-center gap-2",
                          Button {
                            variant: ButtonVariant::Ghost,
                            size: ButtonSize::Sm,
                            class: "h-6 text-xs".to_string(),
                            onclick: move |_| show_api_key.set(!show_api_key()),
                            if show_api_key() { "隐藏" } else { "显示" }
                          }
                          if active_provider.as_ref().is_some_and(|p| p.api_key_set) && !creating_custom_provider() {
                            Button {
                              variant: ButtonVariant::Ghost,
                              size: ButtonSize::Sm,
                              class: "h-6 text-xs text-destructive".to_string(),
                              disabled: save_busy(),
                              "data-testid": "provider-clear-key-btn",
                              onclick: move |_| {
                                let target = selected_provider().trim().to_string();
                                let tok = tok_clear_key.clone();
                                let is_custom = active_provider_for_clear.as_ref().is_some_and(|p| p.is_custom);
                                save_busy.set(true);
                                error.set(None);
                                notice.set(None);
                                spawn(async move {
                                  let mut payload = serde_json::json!({
                                    "base_url": base_url_draft().trim(),
                                    "api_key": "",
                                  });
                                  if is_custom && let Some(obj) = payload.as_object_mut() {
                                    obj.insert("protocol".to_string(), serde_json::json!(protocol_draft()));
                                  }
                                  match put_provider(&target, &payload, &tok).await {
                                    Ok(updated) => {
                                      if let Some(saved) = updated.providers.iter().find(|p| p.name == target) {
                                        hydrate_provider(saved);
                                      }
                                      on_models_updated.call(updated.clone());
                                      snapshot.set(Some(updated));
                                      notice.set(Some(format!("已清除厂商 {target} 的 API Key 并热重载生效。")));
                                    }
                                    Err(err) => error.set(Some(err)),
                                  }
                                  save_busy.set(false);
                                });
                              },
                              "清空已存 Key"
                            }
                          }
                        }
                      }
                      Input {
                        size: InputSize::Sm,
                        r#type: if show_api_key() { "text" } else { "password" },
                        autocomplete: "off",
                        value: api_key_draft(),
                        placeholder: if active_provider.as_ref().is_some_and(|p| p.api_key_set) && !creating_custom_provider() {
                          format!(
                            "当前已配置 ({}) — 留空保持不变，输入新 Key 覆盖",
                            active_provider
                              .as_ref()
                              .and_then(|p| p.api_key_masked.clone())
                              .unwrap_or_else(|| "已脱敏".to_string())
                          )
                        } else {
                          "输入 API Key（本地 Ollama 可填 ollama）".to_string()
                        },
                        "data-testid": "provider-api-key-input",
                        on_value_change: move |v: String| api_key_draft.set(v),
                      }
                    }

                    div { class: "grid grid-cols-1 sm:grid-cols-3 gap-3",
                      div { class: "space-y-1.5",
                        Label { "环境变量名 (api_key_env)" }
                        Input {
                          size: InputSize::Sm,
                          value: api_key_env_draft(),
                          placeholder: "OPENROUTER_API_KEY".to_string(),
                          on_value_change: move |v: String| api_key_env_draft.set(v),
                        }
                      }
                      div { class: "space-y-1.5",
                        Label { "超时秒数" }
                        Input {
                          size: InputSize::Sm,
                          r#type: "number",
                          value: timeout_draft(),
                          placeholder: "180".to_string(),
                          on_value_change: move |v: String| timeout_draft.set(v),
                        }
                      }
                      div { class: "space-y-1.5",
                        Label { "限流 RPM" }
                        Input {
                          size: InputSize::Sm,
                          r#type: "number",
                          value: rpm_draft(),
                          placeholder: "500".to_string(),
                          on_value_change: move |v: String| rpm_draft.set(v),
                        }
                      }
                    }

                    if let Some(outcome) = test_outcome() {
                      Alert {
                        variant: if outcome.ok { AlertVariant::Default } else { AlertVariant::Destructive },
                        AlertDescription {
                          span { "data-testid": "provider-test-result",
                            if outcome.ok {
                              "✓ 连接校验通过（协议 {outcome.protocol} · 延迟 {outcome.latency_ms} ms）"
                            } else {
                              "✕ 连接校验失败（{outcome.latency_ms} ms）：{outcome.error.clone().unwrap_or_else(|| \"未知错误\".to_string())}"
                            }
                          }
                        }
                      }
                    }

                    Separator {}

                    div { class: "flex flex-wrap items-center gap-2",
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        loading: save_busy(),
                        "data-testid": "provider-save-btn",
                        onclick: move |_| {
                          let target = if creating_custom_provider() {
                            provider_name_draft().trim().to_string()
                          } else {
                            selected_provider().trim().to_string()
                          };
                          if target.is_empty() {
                            error.set(Some("请填写厂商标识名（如 openrouter、ollama、my-relay）".to_string()));
                            return;
                          }
                          let tok = tok_save_provider.clone();
                          let is_custom = creating_custom_provider()
                            || active_provider_for_save.as_ref().is_some_and(|p| p.is_custom);
                          save_busy.set(true);
                          error.set(None);
                          notice.set(None);
                          spawn(async move {
                            let mut payload = serde_json::json!({
                              "base_url": base_url_draft().trim(),
                            });
                            if let Some(obj) = payload.as_object_mut() {
                              if !api_key_env_draft().trim().is_empty() {
                                obj.insert("api_key_env".to_string(), serde_json::json!(api_key_env_draft().trim()));
                              }
                              if is_custom {
                                obj.insert("protocol".to_string(), serde_json::json!(protocol_draft()));
                              }
                              if !api_key_draft().trim().is_empty() {
                                obj.insert("api_key".to_string(), serde_json::json!(api_key_draft().trim()));
                              }
                              if let Ok(sec) = timeout_draft().trim().parse::<u64>() && sec > 0 {
                                obj.insert("timeout_seconds".to_string(), serde_json::json!(sec));
                              }
                              if let Ok(rpm) = rpm_draft().trim().parse::<u32>() && rpm > 0 {
                                obj.insert("requests_per_minute".to_string(), serde_json::json!(rpm));
                              }
                            }
                            match put_provider(&target, &payload, &tok).await {
                              Ok(updated) => {
                                if let Some(saved) = updated.providers.iter().find(|p| p.name == target) {
                                  hydrate_provider(saved);
                                }
                                on_models_updated.call(updated.clone());
                                snapshot.set(Some(updated));
                                notice.set(Some(format!("已保存厂商 {target} 配置到 ~/.yanshi/ 并立即热生效。")));
                              }
                              Err(err) => error.set(Some(err)),
                            }
                            save_busy.set(false);
                          });
                        },
                        "💾 保存并热生效"
                      }

                      Button {
                        variant: ButtonVariant::Secondary,
                        size: ButtonSize::Sm,
                        loading: test_busy(),
                        "data-testid": "provider-test-btn",
                        onclick: move |_| {
                          let target = if creating_custom_provider() {
                            provider_name_draft().trim().to_string()
                          } else {
                            selected_provider().trim().to_string()
                          };
                          if target.is_empty() {
                            error.set(Some("请先填写厂商标识名".to_string()));
                            return;
                          }
                          let tok = tok_test_provider.clone();
                          let is_custom = creating_custom_provider()
                            || active_provider_for_test.as_ref().is_some_and(|p| p.is_custom);
                          test_busy.set(true);
                          error.set(None);
                          test_outcome.set(None);
                          spawn(async move {
                            let mut payload = serde_json::json!({});
                            if let Some(obj) = payload.as_object_mut() {
                              if is_custom {
                                obj.insert("protocol".to_string(), serde_json::json!(protocol_draft()));
                              }
                              if !base_url_draft().trim().is_empty() {
                                obj.insert("base_url".to_string(), serde_json::json!(base_url_draft().trim()));
                              }
                              if !api_key_draft().trim().is_empty() {
                                obj.insert("api_key".to_string(), serde_json::json!(api_key_draft().trim()));
                              }
                            }
                            match test_provider(&target, &payload, &tok).await {
                              Ok(res) => test_outcome.set(Some(res)),
                              Err(err) => error.set(Some(err)),
                            }
                            test_busy.set(false);
                          });
                        },
                        "🔌 测试连接"
                      }

                      if !creating_custom_provider()
                        && let Some(ref p_del) = active_provider_for_delete
                        && (p_del.is_custom || p_del.is_overridden)
                      {
                        {
                          let del_name = p_del.name.clone();
                          let del_custom = p_del.is_custom;
                          let tok = tok_delete_provider.clone();
                          rsx! {
                            Button {
                              variant: ButtonVariant::Destructive,
                              size: ButtonSize::Sm,
                              disabled: save_busy(),
                              "data-testid": "provider-delete-btn",
                              onclick: move |_| {
                                let name = del_name.clone();
                                let tok = tok.clone();
                                save_busy.set(true);
                                error.set(None);
                                notice.set(None);
                                spawn(async move {
                                  match delete_provider(&name, del_custom, &tok).await {
                                    Ok(updated) => {
                                      if let Some(next) = updated
                                        .providers
                                        .iter()
                                        .find(|p| p.name == name)
                                        .or_else(|| updated.providers.first())
                                      {
                                        hydrate_provider(next);
                                      }
                                      on_models_updated.call(updated.clone());
                                      snapshot.set(Some(updated));
                                      notice.set(Some(if del_custom {
                                        format!("已删除自定义厂商 {name}。")
                                      } else {
                                        format!("已将 {name} 恢复为内置默认地址与参数。")
                                      }));
                                    }
                                    Err(err) => error.set(Some(err)),
                                  }
                                  save_busy.set(false);
                                });
                              },
                              if del_custom { "删除自定义厂商" } else { "恢复内置默认" }
                            }
                          }
                        }
                      }
                    }
                  }
                }
              }
            }

            TabsContent { value: "models".to_string(), class: "mt-4".to_string(),
              div { class: "grid grid-cols-1 md:grid-cols-[260px_minmax(0,1fr)] gap-4",
                aside { class: "space-y-2 border-r border-border pr-3",
                  Button {
                    variant: ButtonVariant::Secondary,
                    size: ButtonSize::Sm,
                    class: "w-full justify-start".to_string(),
                    "data-testid": "add-custom-model-btn",
                    onclick: move |_| {
                      creating_model.set(true);
                      editing_model_name.set(None);
                      model_name_draft.set(String::new());
                      model_vendor_draft.set(selected_provider());
                      model_id_draft.set(String::new());
                      model_type_draft.set("chat".to_string());
                      model_accepts_draft.set(vec!["text".to_string()]);
                      model_base_url_draft.set(String::new());
                      model_temp_draft.set(String::new());
                      model_max_tokens_draft.set(String::new());
                      model_tools_draft.set(true);
                      model_thinking_draft.set(false);
                    },
                    "+ 新增自定义模型"
                  }
                  Input {
                    size: InputSize::Sm,
                    value: model_search(),
                    placeholder: "搜索模型名 / ID / 厂商…".to_string(),
                    on_value_change: move |v: String| model_search.set(v),
                  }
                  NativeSelect {
                    class: "h-8 text-xs w-full".to_string(),
                    on_value_change: move |v: String| model_vendor_filter.set(v),
                    NativeSelectOption { value: "all".to_string(), selected: model_vendor_filter() == "all",
                      "全部厂商 ({all_models.len()})"
                    }
                    for p in providers.iter() {
                      NativeSelectOption {
                        key: "{p.name}",
                        value: p.name.clone(),
                        selected: model_vendor_filter() == p.name,
                        "{p.name} ({p.model_count})"
                      }
                    }
                  }
                  div { class: "space-y-1 max-h-[480px] overflow-y-auto",
                    for m in filtered_models.iter() {
                      {
                        let m_clone = m.clone();
                        let is_sel = !creating_model() && editing_model_name().as_deref() == Some(m.name.as_str());
                        let cls = if is_sel {
                          "w-full text-left rounded-md px-2.5 py-2 border border-primary/50 bg-accent/60"
                        } else {
                          "w-full text-left rounded-md px-2.5 py-2 border border-transparent hover:bg-muted/60"
                        };
                        rsx! {
                          button {
                            key: "{m.name}",
                            r#type: "button",
                            class: cls,
                            "data-testid": "model-item-{m.name}",
                            onclick: move |_| hydrate_model(&m_clone),
                            div { class: "flex items-center gap-1.5 text-sm font-medium",
                              span { class: "truncate", "{m.name}" }
                              if m.is_custom {
                                Badge { variant: BadgeVariant::Secondary, "自定义" }
                              } else if m.is_overridden {
                                Badge { variant: BadgeVariant::Outline, "已覆写" }
                              }
                            }
                            div { class: "text-[11px] text-muted-foreground mt-0.5 truncate",
                              "{m.vendor} · {m.model_type}"
                            }
                          }
                        }
                      }
                    }
                  }
                }

                section { class: "space-y-4 min-w-0",
                  if !creating_model() && editing_model_name().is_none() {
                    p { class: "text-sm text-muted-foreground",
                      "点击左侧任意模型查看或覆写其上游 Model ID / 独立 Base URL / 推理参数，或点击「+ 新增自定义模型」为自定义厂商添加模型。"
                    }
                  } else {
                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                      div { class: "space-y-1.5",
                        Label { "模型别名 (name)" }
                        Input {
                          size: InputSize::Sm,
                          value: model_name_draft(),
                          disabled: !creating_model(),
                          placeholder: "例如 my-qwen3-coder 或 llama3.3".to_string(),
                          "data-testid": "model-name-input",
                          on_value_change: move |v: String| model_name_draft.set(v),
                        }
                      }
                      div { class: "space-y-1.5",
                        Label { "所属厂商 (vendor)" }
                        NativeSelect {
                          class: "h-8 text-xs w-full".to_string(),
                          "data-testid": "model-vendor-select",
                          on_value_change: move |v: String| model_vendor_draft.set(v),
                          for p in providers.iter() {
                            NativeSelectOption {
                              key: "{p.name}",
                              value: p.name.clone(),
                              selected: model_vendor_draft() == p.name,
                              "{p.name}"
                            }
                          }
                        }
                      }
                    }

                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                      div { class: "space-y-1.5",
                        Label { "上游实际 Model ID（可选）" }
                        Input {
                          size: InputSize::Sm,
                          value: model_id_draft(),
                          placeholder: "留空则与模型别名一致（如 openai/gpt-4o）".to_string(),
                          "data-testid": "model-id-input",
                          on_value_change: move |v: String| model_id_draft.set(v),
                        }
                      }
                      div { class: "space-y-1.5",
                        Label { "模型类型" }
                        NativeSelect {
                          class: "h-8 text-xs w-full".to_string(),
                          on_value_change: move |v: String| model_type_draft.set(v),
                          for t in MODEL_TYPES {
                            NativeSelectOption {
                              key: "{t}",
                              value: (*t).to_string(),
                              selected: model_type_draft() == *t,
                              "{t}"
                            }
                          }
                        }
                      }
                    }

                    div { class: "space-y-1.5",
                      Label { "单模型独立 Base URL（可选，留空继承厂商地址）" }
                      Input {
                        size: InputSize::Sm,
                        value: model_base_url_draft(),
                        placeholder: "https://special-endpoint.example.com/v1".to_string(),
                        on_value_change: move |v: String| model_base_url_draft.set(v),
                      }
                    }

                    div { class: "grid grid-cols-1 sm:grid-cols-2 gap-3",
                      div { class: "space-y-1.5",
                        Label { "默认 Temperature" }
                        Input {
                          size: InputSize::Sm,
                          r#type: "number",
                          value: model_temp_draft(),
                          placeholder: "0.7".to_string(),
                          on_value_change: move |v: String| model_temp_draft.set(v),
                        }
                      }
                      div { class: "space-y-1.5",
                        Label { "默认 Max Tokens" }
                        Input {
                          size: InputSize::Sm,
                          r#type: "number",
                          value: model_max_tokens_draft(),
                          placeholder: "8192".to_string(),
                          on_value_change: move |v: String| model_max_tokens_draft.set(v),
                        }
                      }
                    }

                    div { class: "flex flex-wrap items-center gap-3 text-xs",
                      span { class: "text-muted-foreground", "支持输入模态：" }
                      for modality in INPUT_MODALITIES {
                        {
                          let mod_str = (*modality).to_string();
                          let is_checked = model_accepts_draft().contains(&mod_str);
                          rsx! {
                            label { key: "{modality}", class: "inline-flex items-center gap-1.5 cursor-pointer",
                              Checkbox {
                                checked: is_checked,
                                on_checked_change: move |checked: bool| {
                                  let mut next = model_accepts_draft();
                                  if checked {
                                    if !next.contains(&mod_str) {
                                      next.push(mod_str.clone());
                                    }
                                  } else {
                                    next.retain(|x| x != &mod_str);
                                  }
                                  model_accepts_draft.set(next);
                                },
                              }
                              span { "{modality}" }
                            }
                          }
                        }
                      }
                    }

                    div { class: "flex flex-wrap items-center gap-4 text-xs",
                      label { class: "inline-flex items-center gap-1.5 cursor-pointer",
                        Checkbox {
                          checked: model_tools_draft(),
                          on_checked_change: move |v: bool| model_tools_draft.set(v),
                        }
                        span { "支持原生 Tool Calling" }
                      }
                      label { class: "inline-flex items-center gap-1.5 cursor-pointer",
                        Checkbox {
                          checked: model_thinking_draft(),
                          on_checked_change: move |v: bool| model_thinking_draft.set(v),
                        }
                        span { "支持深度思考 (Thinking)" }
                      }
                    }

                    Separator {}

                    div { class: "flex flex-wrap items-center gap-2",
                      Button {
                        variant: ButtonVariant::Primary,
                        size: ButtonSize::Sm,
                        loading: save_busy(),
                        "data-testid": "model-save-btn",
                        onclick: move |_| {
                          let name = if creating_model() {
                            model_name_draft().trim().to_string()
                          } else {
                            editing_model_name().unwrap_or_else(&*model_name_draft).trim().to_string()
                          };
                          if name.is_empty() {
                            error.set(Some("请填写模型别名（例如 my-gpt-4o 或 deepseek-r1）".to_string()));
                            return;
                          }
                          let tok = tok_save_model.clone();
                          save_busy.set(true);
                          error.set(None);
                          notice.set(None);
                          spawn(async move {
                            let mut payload = serde_json::json!({
                              "vendor": model_vendor_draft().trim(),
                              "model_type": model_type_draft(),
                              "accepts": model_accepts_draft(),
                              "base_url": model_base_url_draft().trim(),
                              "supports_tools": model_tools_draft(),
                              "native_tool_calling": model_tools_draft(),
                              "supports_thinking": model_thinking_draft(),
                            });
                            if let Some(obj) = payload.as_object_mut() {
                              if !model_id_draft().trim().is_empty() {
                                obj.insert("model_id".to_string(), serde_json::json!(model_id_draft().trim()));
                              }
                              if let Ok(t) = model_temp_draft().trim().parse::<f64>() {
                                obj.insert("temperature".to_string(), serde_json::json!(t));
                              }
                              if let Ok(m) = model_max_tokens_draft().trim().parse::<u32>() && m > 0 {
                                obj.insert("max_tokens".to_string(), serde_json::json!(m));
                              }
                            }
                            match put_model_item(&name, &payload, &tok).await {
                              Ok(updated) => {
                                if let Some(saved) = updated.models.iter().find(|m| m.name == name) {
                                  hydrate_model(saved);
                                }
                                on_models_updated.call(updated.clone());
                                snapshot.set(Some(updated));
                                notice.set(Some(format!("已保存模型 {name} 到 ~/.yanshi/models.yml 并热生效。")));
                              }
                              Err(err) => error.set(Some(err)),
                            }
                            save_busy.set(false);
                          });
                        },
                        "💾 保存模型并热生效"
                      }

                      if let Some(ref cur) = current_editing_model
                        && (cur.is_custom || cur.is_overridden)
                      {
                        {
                          let del_name = cur.name.clone();
                          let del_custom = cur.is_custom;
                          let tok = tok_delete_model.clone();
                          rsx! {
                            Button {
                              variant: ButtonVariant::Destructive,
                              size: ButtonSize::Sm,
                              disabled: save_busy(),
                              "data-testid": "model-delete-btn",
                              onclick: move |_| {
                                let name = del_name.clone();
                                let tok = tok.clone();
                                save_busy.set(true);
                                error.set(None);
                                notice.set(None);
                                spawn(async move {
                                  match delete_model_item(&name, &tok).await {
                                    Ok(updated) => {
                                      if del_custom {
                                        editing_model_name.set(None);
                                      } else if let Some(m) = updated.models.iter().find(|m| m.name == name) {
                                        hydrate_model(m);
                                      }
                                      on_models_updated.call(updated.clone());
                                      snapshot.set(Some(updated));
                                      notice.set(Some(if del_custom {
                                        format!("已删除自定义模型 {name}。")
                                      } else {
                                        format!("已恢复模型 {name} 为内置默认配置。")
                                      }));
                                    }
                                    Err(err) => error.set(Some(err)),
                                  }
                                  save_busy.set(false);
                                });
                              },
                              if del_custom { "删除自定义模型" } else { "恢复内置默认" }
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
}
