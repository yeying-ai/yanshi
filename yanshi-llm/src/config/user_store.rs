//! User-editable model & provider configuration store (`~/.yanshi/models.yml` + `~/.yanshi/.env`).
//!
//! Shared by both `yanshi-server` and the Tauri Desktop loopback engine so the
//! CLI, Web UI, and Desktop app read/write the exact same files with:
//! - Atomic `tmp + rename` writes for `models.yml` and `.env`;
//! - Line-preserving `.env` edits (preserving comments & unrelated secrets) + Unix `0o600` mode;
//! - Zero plaintext API keys ever written into `models.yml` or returned in API snapshots;
//! - Safe in-process hot-reload via `ModelRegistry::global()` and `set_runtime_env_override`
//!   (no `unsafe std::env::set_var` in multi-threaded runtimes).

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::config::model_config::{
  BUILTIN_VENDORS, RateLimitConfig, SUPPORTED_CUSTOM_PROTOCOLS, clear_runtime_env_override,
  set_runtime_env_override,
};
use crate::config::{
  GlobalDefaults, LLMConfig, ModelConfig, ProviderConfig, builtin_default_config,
};
use crate::model_types::InputType;
use crate::providers::create_provider;
use crate::registry::ModelRegistry;
use crate::thinking::ThinkingKind;
use crate::{LLMError, Result};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn next_tmp_path(target: &Path) -> PathBuf {
  let seq = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
  let pid = std::process::id();
  let file_name = target
    .file_name()
    .and_then(|n| n.to_str())
    .unwrap_or("config");
  target.with_file_name(format!(".{file_name}.tmp.{pid}.{seq}"))
}

fn write_file_atomic(path: &Path, content: &[u8], secret_permissions: bool) -> Result<()> {
  if let Some(parent) = path.parent() {
    std::fs::create_dir_all(parent).map_err(|err| LLMError::ConfigurationError {
      message: format!("failed to create config dir '{}': {err}", parent.display()),
    })?;
  }
  let tmp = next_tmp_path(path);
  std::fs::write(&tmp, content).map_err(|err| LLMError::ConfigurationError {
    message: format!("failed to write temp file '{}': {err}", tmp.display()),
  })?;
  #[cfg(unix)]
  if secret_permissions {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
  }
  #[cfg(not(unix))]
  let _ = secret_permissions;

  std::fs::rename(&tmp, path).map_err(|err| {
    let _ = std::fs::remove_file(&tmp);
    LLMError::ConfigurationError {
      message: format!("failed to replace '{}': {err}", path.display()),
    }
  })?;
  Ok(())
}

/// Default user config directory (`~/.yanshi`).
pub fn default_yanshi_config_dir() -> Result<PathBuf> {
  dirs::home_dir()
    .map(|home| home.join(".yanshi"))
    .ok_or_else(|| LLMError::ConfigurationError {
      message: "could not determine home directory for ~/.yanshi".to_string(),
    })
}

/// Mask an API key for safe display (e.g. `sk-••••9f2a`). Never exposes full key.
pub fn mask_api_key(raw: &str) -> String {
  let trimmed = raw.trim();
  if trimmed.is_empty() {
    return String::new();
  }
  let chars: Vec<char> = trimmed.chars().collect();
  if chars.len() <= 4 {
    return "••••".to_string();
  }
  let suffix: String = chars[chars.len() - 4..].iter().collect();
  if chars.len() >= 10 {
    let prefix: String = chars[..3].iter().collect();
    return format!("{prefix}••••{suffix}");
  }
  format!("••••{suffix}")
}

/// Validate a provider or model identifier name.
fn validate_identifier(raw: &str, label: &str) -> Result<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() || trimmed.len() > 128 {
    return Err(LLMError::ConfigurationError {
      message: format!("{label} must be 1..=128 characters"),
    });
  }
  if !trimmed
    .chars()
    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
  {
    return Err(LLMError::ConfigurationError {
      message: format!("{label} '{trimmed}' contains unsupported characters"),
    });
  }
  Ok(trimmed.to_string())
}

fn validate_env_var_name(raw: &str) -> Result<String> {
  let trimmed = raw.trim();
  if trimmed.is_empty() || trimmed.len() > 96 {
    return Err(LLMError::ConfigurationError {
      message: "api_key_env must be 1..=96 characters".to_string(),
    });
  }
  if !trimmed
    .chars()
    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
    || trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
  {
    return Err(LLMError::ConfigurationError {
      message: format!("invalid api_key_env '{trimmed}' (use UPPER_SNAKE_CASE)"),
    });
  }
  Ok(trimmed.to_string())
}

fn default_env_var_for_provider(provider_name: &str) -> String {
  let upper: String = provider_name
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() {
        c.to_ascii_uppercase()
      } else {
        '_'
      }
    })
    .collect();
  format!("{upper}_API_KEY")
}

fn validate_base_url(raw: &str) -> Result<String> {
  let trimmed = raw.trim().trim_end_matches('/');
  if trimmed.is_empty() {
    return Err(LLMError::ConfigurationError {
      message: "base_url cannot be empty".to_string(),
    });
  }
  if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
    return Err(LLMError::ConfigurationError {
      message: format!("base_url '{trimmed}' must start with http:// or https://"),
    });
  }
  Ok(trimmed.to_string())
}

fn parse_dotenv_line(line: &str) -> Option<(String, String)> {
  let trimmed = line.trim();
  if trimmed.is_empty() || trimmed.starts_with('#') {
    return None;
  }
  let body = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
  let (raw_k, raw_v) = body.split_once('=')?;
  let key = raw_k.trim();
  if key.is_empty() {
    return None;
  }
  let mut val = raw_v.trim().to_string();
  if (val.starts_with('"') && val.ends_with('"') && val.len() >= 2)
    || (val.starts_with('\'') && val.ends_with('\'') && val.len() >= 2)
  {
    val = val[1..val.len() - 1].to_string();
  }
  Some((key.to_string(), val))
}

/// Parse key/value pairs from a `.env` file.
pub fn read_dotenv_map(path: &Path) -> HashMap<String, String> {
  let Ok(text) = std::fs::read_to_string(path) else {
    return HashMap::new();
  };
  let mut out = HashMap::new();
  for line in text.lines() {
    if let Some((k, v)) = parse_dotenv_line(line) {
      out.insert(k, v);
    }
  }
  out
}

fn format_dotenv_value(val: &str) -> String {
  if val
    .chars()
    .any(|c| c.is_whitespace() || c == '#' || c == '"' || c == '\'')
  {
    let escaped = val.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
  } else {
    val.to_string()
  }
}

/// Line-preserving `.env` update: replaces existing `KEY=...` (or uncomments
/// `# KEY=...`), appends when missing, or removes the key when `value` is `None`.
pub fn upsert_dotenv_var(path: &Path, key: &str, value: Option<&str>) -> Result<()> {
  let env_key = validate_env_var_name(key)?;
  let existing = std::fs::read_to_string(path).unwrap_or_default();
  let mut lines: Vec<String> = Vec::new();
  let mut replaced = false;

  for line in existing.lines() {
    let trimmed = line.trim();
    let is_active_match = parse_dotenv_line(line).is_some_and(|(k, _)| k == env_key);
    let is_commented_match = !is_active_match
      && trimmed.starts_with('#')
      && parse_dotenv_line(trimmed.trim_start_matches('#').trim())
        .is_some_and(|(k, _)| k == env_key);

    if is_active_match {
      if let Some(new_val) = value
        && !replaced
      {
        lines.push(format!("{env_key}={}", format_dotenv_value(new_val)));
        replaced = true;
      }
      continue;
    }

    if is_commented_match && value.is_some() && !replaced {
      if let Some(new_val) = value {
        lines.push(format!("{env_key}={}", format_dotenv_value(new_val)));
        replaced = true;
      }
      continue;
    }

    lines.push(line.to_string());
  }

  if let Some(new_val) = value
    && !replaced
  {
    lines.push(format!("{env_key}={}", format_dotenv_value(new_val)));
  }

  let mut out = lines.join("\n");
  if !out.is_empty() && !out.ends_with('\n') {
    out.push('\n');
  }
  write_file_atomic(path, out.as_bytes(), true)
}

/// Load only the user's overlay file (`<config_dir>/models.yml`, or empty if absent).
pub fn load_user_models_file(config_dir: &Path) -> Result<LLMConfig> {
  let yml_path = config_dir.join("models.yml");
  let yaml_path = config_dir.join("models.yaml");
  let target = if yml_path.exists() {
    Some(yml_path)
  } else if yaml_path.exists() {
    Some(yaml_path)
  } else {
    None
  };
  let Some(path) = target else {
    return Ok(LLMConfig::default());
  };
  let content = std::fs::read_to_string(&path).map_err(|err| LLMError::ConfigurationError {
    message: format!("failed to read '{}': {err}", path.display()),
  })?;
  if content.trim().is_empty() {
    return Ok(LLMConfig::default());
  }
  LLMConfig::from_yaml(&content)
}

/// Atomically persist the user's overlay config into `<config_dir>/models.yml`.
pub fn save_user_models_file(config_dir: &Path, user_cfg: &LLMConfig) -> Result<()> {
  let path = config_dir.join("models.yml");
  let yaml = serde_yaml::to_string(user_cfg).map_err(|err| LLMError::ConfigurationError {
    message: format!("failed to serialize models.yml: {err}"),
  })?;
  let body = format!(
    "# Yanshi user model & provider overrides (layered on built-in defaults)\n# API keys live in ~/.yanshi/.env — never commit secrets.\n\n{yaml}"
  );
  write_file_atomic(&path, body.as_bytes(), false)
}

/// Sync all entries from `<config_dir>/.env` into thread-safe in-process
/// runtime overrides so edits take effect immediately without `unsafe set_var`.
pub fn sync_dotenv_into_runtime_overrides(config_dir: &Path) {
  let dotenv_map = read_dotenv_map(&config_dir.join(".env"));
  for (key, value) in dotenv_map {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
      set_runtime_env_override(key, Some(trimmed.to_string()));
    }
  }
}

/// Reload the global `ModelRegistry` from `config_dir` (merging built-in
/// defaults + `<config_dir>/models.yml` + `<config_dir>/.env`).
pub async fn reload_global_registry_from_dir(config_dir: &Path) -> Result<()> {
  sync_dotenv_into_runtime_overrides(config_dir);
  let merged = load_merged_config_for_dir(config_dir)?;
  let yaml = serde_yaml::to_string(&merged).map_err(|err| LLMError::ConfigurationError {
    message: format!("failed to serialize merged LLMConfig: {err}"),
  })?;
  ModelRegistry::global().load_config_from_yaml(&yaml).await
}

fn load_merged_config_for_dir(config_dir: &Path) -> Result<LLMConfig> {
  let mut merged = builtin_default_config()?;
  let user_cfg = load_user_models_file(config_dir)?;
  merged.merge_overlay(user_cfg);
  Ok(merged)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderConfigView {
  pub name: String,
  pub protocol: String,
  pub base_url: Option<String>,
  pub default_base_url: Option<String>,
  pub api_key_env: String,
  pub api_key_set: bool,
  pub api_key_masked: Option<String>,
  /// `"dotenv"`, `"env"`, or `"unset"`.
  pub api_key_source: String,
  pub timeout_seconds: Option<u64>,
  pub requests_per_minute: Option<u32>,
  pub is_custom: bool,
  pub is_overridden: bool,
  pub model_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigView {
  pub name: String,
  pub vendor: String,
  pub model_id: String,
  pub model_type: String,
  pub accepts: Vec<String>,
  pub base_url: Option<String>,
  pub effective_base_url: Option<String>,
  pub temperature: Option<f32>,
  pub max_tokens: Option<u32>,
  pub supports_streaming: bool,
  pub supports_tools: bool,
  pub native_tool_calling: bool,
  pub supports_thinking: bool,
  pub thinking_kind: Option<ThinkingKind>,
  pub is_custom: bool,
  pub is_overridden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfigSnapshot {
  pub config_dir: String,
  pub models_path: String,
  pub env_path: String,
  pub providers: Vec<ProviderConfigView>,
  pub models: Vec<ModelConfigView>,
  pub defaults: GlobalDefaults,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct UpsertProviderPatch {
  #[serde(default)]
  pub protocol: Option<String>,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub api_key_env: Option<String>,
  /// `None` = keep existing key; `Some("")` = clear key; `Some("sk-...")` = update key in `.env`.
  #[serde(default)]
  pub api_key: Option<String>,
  #[serde(default)]
  pub timeout_seconds: Option<u64>,
  #[serde(default)]
  pub requests_per_minute: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpsertModelPatch {
  pub vendor: String,
  #[serde(default)]
  pub model_id: Option<String>,
  #[serde(default)]
  pub model_type: Option<String>,
  #[serde(default)]
  pub accepts: Option<Vec<InputType>>,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub temperature: Option<f32>,
  #[serde(default)]
  pub max_tokens: Option<u32>,
  #[serde(default)]
  pub supports_streaming: Option<bool>,
  #[serde(default)]
  pub supports_tools: Option<bool>,
  #[serde(default)]
  pub native_tool_calling: Option<bool>,
  #[serde(default)]
  pub supports_thinking: Option<bool>,
  #[serde(default)]
  pub thinking_kind: Option<ThinkingKind>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TestProviderRequest {
  #[serde(default)]
  pub protocol: Option<String>,
  #[serde(default)]
  pub base_url: Option<String>,
  #[serde(default)]
  pub api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProviderResult {
  pub ok: bool,
  pub provider: String,
  pub protocol: String,
  pub base_url: Option<String>,
  pub latency_ms: u64,
  pub error: Option<String>,
}

fn resolve_key_status(
  config: &LLMConfig,
  dotenv_map: &HashMap<String, String>,
  provider_name: &str,
  api_key_env: &str,
) -> (bool, Option<String>, String) {
  if let Some(dotenv_val) = dotenv_map
    .get(api_key_env)
    .map(|v| v.trim())
    .filter(|v| !v.is_empty())
  {
    return (true, Some(mask_api_key(dotenv_val)), "dotenv".to_string());
  }
  if let Ok(resolved) = config.get_api_key(provider_name) {
    let trimmed = resolved.trim();
    if !trimmed.is_empty() && !provider_name.eq_ignore_ascii_case("mock") {
      return (true, Some(mask_api_key(trimmed)), "env".to_string());
    }
  }
  (false, None, "unset".to_string())
}

/// Build the full read-only configuration snapshot (never exposes raw API keys).
pub fn get_model_config_snapshot(config_dir: &Path) -> Result<ModelConfigSnapshot> {
  let builtin = builtin_default_config()?;
  let user_cfg = load_user_models_file(config_dir)?;
  let mut merged = builtin.clone();
  merged.merge_overlay(user_cfg.clone());
  let dotenv_map = read_dotenv_map(&config_dir.join(".env"));

  let mut model_counts: HashMap<String, usize> = HashMap::new();
  for mc in merged.models.values() {
    *model_counts.entry(mc.vendor.clone()).or_insert(0) += 1;
  }

  let mut provider_names: BTreeSet<String> = BTreeSet::new();
  for name in builtin.providers.keys() {
    provider_names.insert(name.clone());
  }
  for name in user_cfg.providers.keys() {
    provider_names.insert(name.clone());
  }

  let mut providers = Vec::with_capacity(provider_names.len());
  for name in provider_names {
    let is_custom = !builtin.providers.contains_key(&name);
    let is_overridden = !is_custom && user_cfg.providers.contains_key(&name);
    let Some(effective) = merged.providers.get(&name) else {
      continue;
    };
    let default_base_url = builtin
      .providers
      .get(&name)
      .and_then(|p| p.base_url.clone());
    let effective_base_url = merged.get_provider_base_url(&name);
    let protocol = merged.effective_provider_protocol(&name).to_string();
    let (api_key_set, api_key_masked, api_key_source) =
      resolve_key_status(&merged, &dotenv_map, &name, &effective.api_key_env);

    providers.push(ProviderConfigView {
      name: name.clone(),
      protocol,
      base_url: effective_base_url,
      default_base_url,
      api_key_env: effective.api_key_env.clone(),
      api_key_set,
      api_key_masked,
      api_key_source,
      timeout_seconds: effective.timeout_seconds,
      requests_per_minute: effective.rate_limit.as_ref().map(|r| r.requests_per_minute),
      is_custom,
      is_overridden,
      model_count: model_counts.get(&name).copied().unwrap_or(0),
    });
  }

  let mut model_names: Vec<String> = merged.models.keys().cloned().collect();
  model_names.sort();
  let mut models = Vec::with_capacity(model_names.len());
  for name in model_names {
    let Some(mc) = merged.models.get(&name) else {
      continue;
    };
    let is_custom = !builtin.models.contains_key(&name);
    let is_overridden = !is_custom && user_cfg.models.contains_key(&name);
    let mut accepts: Vec<String> = mc
      .accepts()
      .iter()
      .map(|i| i.as_str().to_string())
      .collect();
    accepts.sort();
    let caps = mc.get_capabilities();
    let effective_base_url = mc
      .base_url
      .as_deref()
      .map(str::trim)
      .filter(|u| !u.is_empty())
      .map(ToOwned::to_owned)
      .or_else(|| merged.get_provider_base_url(&mc.vendor));

    models.push(ModelConfigView {
      name: name.clone(),
      vendor: mc.vendor.clone(),
      model_id: mc.model_id.clone().unwrap_or_else(|| name.clone()),
      model_type: mc.model_type().to_string(),
      accepts,
      base_url: mc.base_url.clone(),
      effective_base_url,
      temperature: mc.temperature,
      max_tokens: mc.max_tokens,
      supports_streaming: caps.supports_streaming,
      supports_tools: caps.supports_tools,
      native_tool_calling: caps.native_tool_calling,
      supports_thinking: mc.supports_thinking(),
      thinking_kind: mc.thinking_kind(),
      is_custom,
      is_overridden,
    });
  }

  Ok(ModelConfigSnapshot {
    config_dir: config_dir.display().to_string(),
    models_path: config_dir.join("models.yml").display().to_string(),
    env_path: config_dir.join(".env").display().to_string(),
    providers,
    models,
    defaults: merged.defaults,
  })
}

/// Upsert a built-in provider override or custom provider, persist to disk, and
/// optionally reload the global `ModelRegistry`.
pub async fn upsert_provider_config(
  config_dir: &Path,
  raw_name: &str,
  patch: UpsertProviderPatch,
  reload_global: bool,
) -> Result<ModelConfigSnapshot> {
  let name = validate_identifier(raw_name, "provider name")?;
  let builtin = builtin_default_config()?;
  let is_builtin = builtin.providers.contains_key(&name)
    || BUILTIN_VENDORS.contains(&name.to_ascii_lowercase().as_str());

  let mut user_cfg = load_user_models_file(config_dir)?;
  let base_provider = user_cfg
    .providers
    .get(&name)
    .or_else(|| builtin.providers.get(&name))
    .cloned();

  let api_key_env = match patch.api_key_env.as_deref().map(str::trim) {
    Some(custom_env) if !custom_env.is_empty() => validate_env_var_name(custom_env)?,
    _ => base_provider
      .as_ref()
      .map(|p| p.api_key_env.clone())
      .unwrap_or_else(|| default_env_var_for_provider(&name)),
  };

  let protocol = if is_builtin {
    None
  } else {
    let raw_proto = patch
      .protocol
      .as_deref()
      .or_else(|| base_provider.as_ref().and_then(|p| p.protocol.as_deref()))
      .unwrap_or("openai")
      .trim()
      .to_ascii_lowercase();
    if !SUPPORTED_CUSTOM_PROTOCOLS.contains(&raw_proto.as_str()) {
      return Err(LLMError::ConfigurationError {
        message: format!(
          "unsupported custom provider protocol '{raw_proto}'; expected one of: {}",
          SUPPORTED_CUSTOM_PROTOCOLS.join(", ")
        ),
      });
    }
    Some(raw_proto)
  };

  let base_url = match patch.base_url.as_deref() {
    Some(raw_url) if raw_url.trim().is_empty() => builtin
      .providers
      .get(&name)
      .and_then(|p| p.base_url.clone()),
    Some(raw_url) => Some(validate_base_url(raw_url)?),
    None => base_provider.as_ref().and_then(|p| p.base_url.clone()),
  };
  if !is_builtin && base_url.is_none() {
    return Err(LLMError::ConfigurationError {
      message: format!("custom provider '{name}' requires a valid http(s):// base_url"),
    });
  }

  let timeout_seconds = patch
    .timeout_seconds
    .or_else(|| base_provider.as_ref().and_then(|p| p.timeout_seconds));

  let rate_limit = match patch.requests_per_minute {
    Some(0) => {
      return Err(LLMError::ConfigurationError {
        message: "requests_per_minute must be greater than 0".to_string(),
      });
    }
    Some(rpm) => Some(RateLimitConfig {
      requests_per_minute: rpm,
      tokens_per_minute: base_provider
        .as_ref()
        .and_then(|p| p.rate_limit.as_ref())
        .and_then(|r| r.tokens_per_minute),
    }),
    None => base_provider.as_ref().and_then(|p| p.rate_limit.clone()),
  };

  user_cfg.providers.insert(
    name.clone(),
    ProviderConfig {
      api_key_env: api_key_env.clone(),
      protocol,
      base_url,
      timeout_seconds,
      rate_limit,
    },
  );

  // Validate merged configuration before persisting.
  let mut test_merged = builtin;
  test_merged.merge_overlay(user_cfg.clone());
  test_merged.validate()?;

  // Persist API key into `~/.yanshi/.env` (never into `models.yml`).
  let env_path = config_dir.join(".env");
  if let Some(raw_key) = patch.api_key {
    let trimmed = raw_key.trim();
    if trimmed.is_empty() {
      upsert_dotenv_var(&env_path, &api_key_env, None)?;
      if reload_global {
        set_runtime_env_override(&api_key_env, None);
      }
    } else {
      upsert_dotenv_var(&env_path, &api_key_env, Some(trimmed))?;
      if reload_global {
        set_runtime_env_override(&api_key_env, Some(trimmed.to_string()));
      }
    }
  }

  save_user_models_file(config_dir, &user_cfg)?;
  if reload_global {
    reload_global_registry_from_dir(config_dir).await?;
  }
  get_model_config_snapshot(config_dir)
}

/// Reset a built-in provider override back to defaults, or delete a custom
/// provider (and any custom models bound to it).
pub async fn delete_provider_config(
  config_dir: &Path,
  raw_name: &str,
  clear_api_key: bool,
  reload_global: bool,
) -> Result<ModelConfigSnapshot> {
  let name = validate_identifier(raw_name, "provider name")?;
  let builtin = builtin_default_config()?;
  let is_builtin = builtin.providers.contains_key(&name);

  let mut user_cfg = load_user_models_file(config_dir)?;
  let env_key = user_cfg
    .providers
    .get(&name)
    .or_else(|| builtin.providers.get(&name))
    .map(|p| p.api_key_env.clone())
    .unwrap_or_else(|| default_env_var_for_provider(&name));

  user_cfg.providers.remove(&name);
  if !is_builtin {
    user_cfg.models.retain(|_, m| m.vendor != name);
  }

  if clear_api_key {
    upsert_dotenv_var(&config_dir.join(".env"), &env_key, None)?;
    if reload_global {
      set_runtime_env_override(&env_key, None);
    }
  } else if reload_global {
    clear_runtime_env_override(&env_key);
  }

  save_user_models_file(config_dir, &user_cfg)?;
  if reload_global {
    reload_global_registry_from_dir(config_dir).await?;
  }
  get_model_config_snapshot(config_dir)
}

/// Add or override a model entry in `<config_dir>/models.yml` and optionally
/// hot-reload `ModelRegistry::global()`.
pub async fn upsert_model_config(
  config_dir: &Path,
  raw_name: &str,
  patch: UpsertModelPatch,
  reload_global: bool,
) -> Result<ModelConfigSnapshot> {
  let name = validate_identifier(raw_name, "model name")?;
  let vendor = validate_identifier(&patch.vendor, "vendor")?;

  let builtin = builtin_default_config()?;
  let mut user_cfg = load_user_models_file(config_dir)?;
  let existing = user_cfg
    .models
    .get(&name)
    .or_else(|| builtin.models.get(&name))
    .cloned();

  let base_url = match patch.base_url.as_deref() {
    Some(raw) if raw.trim().is_empty() => None,
    Some(raw) => Some(validate_base_url(raw)?),
    None => existing.as_ref().and_then(|m| m.base_url.clone()),
  };

  let model_cfg = ModelConfig {
    vendor,
    r#type: patch
      .model_type
      .filter(|s| !s.trim().is_empty())
      .or_else(|| existing.as_ref().and_then(|m| m.r#type.clone()))
      .or_else(|| Some("chat".to_string())),
    accepts: patch
      .accepts
      .filter(|list| !list.is_empty())
      .or_else(|| existing.as_ref().and_then(|m| m.accepts.clone())),
    outputs: existing.as_ref().and_then(|m| m.outputs.clone()),
    capabilities: None,
    model_id: patch
      .model_id
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
      .or_else(|| existing.as_ref().and_then(|m| m.model_id.clone())),
    base_url,
    temperature: patch
      .temperature
      .or_else(|| existing.as_ref().and_then(|m| m.temperature)),
    top_p: existing.as_ref().and_then(|m| m.top_p),
    max_tokens: patch
      .max_tokens
      .or_else(|| existing.as_ref().and_then(|m| m.max_tokens)),
    frequency_penalty: existing.as_ref().and_then(|m| m.frequency_penalty),
    stop: existing.as_ref().and_then(|m| m.stop.clone()),
    n: existing.as_ref().and_then(|m| m.n),
    supports_streaming: patch
      .supports_streaming
      .or_else(|| existing.as_ref().and_then(|m| m.supports_streaming)),
    supports_tools: patch
      .supports_tools
      .or_else(|| existing.as_ref().and_then(|m| m.supports_tools)),
    native_tool_calling: patch
      .native_tool_calling
      .or_else(|| existing.as_ref().and_then(|m| m.native_tool_calling)),
    supports_multimodal: existing.as_ref().and_then(|m| m.supports_multimodal),
    supports_thinking: patch
      .supports_thinking
      .or_else(|| existing.as_ref().and_then(|m| m.supports_thinking)),
    thinking_kind: patch
      .thinking_kind
      .or_else(|| existing.as_ref().and_then(|m| m.thinking_kind)),
    response_format: existing.as_ref().and_then(|m| m.response_format.clone()),
    additional_params: existing.map(|m| m.additional_params).unwrap_or_default(),
  };

  user_cfg.models.insert(name, model_cfg);

  let mut test_merged = builtin;
  test_merged.merge_overlay(user_cfg.clone());
  test_merged.validate()?;

  save_user_models_file(config_dir, &user_cfg)?;
  if reload_global {
    reload_global_registry_from_dir(config_dir).await?;
  }
  get_model_config_snapshot(config_dir)
}

/// Remove a custom model or reset a built-in model override back to defaults.
pub async fn delete_model_config(
  config_dir: &Path,
  raw_name: &str,
  reload_global: bool,
) -> Result<ModelConfigSnapshot> {
  let name = validate_identifier(raw_name, "model name")?;
  let mut user_cfg = load_user_models_file(config_dir)?;
  user_cfg.models.remove(&name);
  save_user_models_file(config_dir, &user_cfg)?;
  if reload_global {
    reload_global_registry_from_dir(config_dir).await?;
  }
  get_model_config_snapshot(config_dir)
}

/// Run a live lightweight connectivity/authentication test for `provider_name`
/// (accepting optional unsaved `base_url` / `api_key` / `protocol` overrides from
/// the form so users can test before saving).
pub async fn test_provider_connection(
  config_dir: &Path,
  raw_name: &str,
  req: TestProviderRequest,
) -> Result<TestProviderResult> {
  let name = validate_identifier(raw_name, "provider name")?;
  let merged = load_merged_config_for_dir(config_dir)?;
  let dotenv_map = read_dotenv_map(&config_dir.join(".env"));

  let protocol = req
    .protocol
    .as_deref()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_ascii_lowercase)
    .unwrap_or_else(|| merged.effective_provider_protocol(&name).to_string());

  let base_url = match req.base_url.as_deref().map(str::trim) {
    Some(u) if !u.is_empty() => Some(validate_base_url(u)?),
    _ => merged.get_provider_base_url(&name),
  };

  let api_key = if let Some(explicit) = req
    .api_key
    .as_deref()
    .map(str::trim)
    .filter(|k| !k.is_empty())
  {
    explicit.to_string()
  } else if let Some(env_name) = merged.get_provider(&name).map(|p| p.api_key_env.as_str())
    && let Some(from_dotenv) = dotenv_map
      .get(env_name)
      .map(String::as_str)
      .map(str::trim)
      .filter(|k| !k.is_empty())
  {
    from_dotenv.to_string()
  } else if let Ok(from_env) = merged.get_api_key(&name) {
    from_env
  } else {
    return Ok(TestProviderResult {
      ok: false,
      provider: name,
      protocol,
      base_url,
      latency_ms: 0,
      error: Some("未配置 API Key，请先填写或保存 API Key 后再测试".to_string()),
    });
  };

  let provider = match create_provider(&protocol, &api_key, base_url.clone()) {
    Ok(p) => p,
    Err(err) => {
      return Ok(TestProviderResult {
        ok: false,
        provider: name,
        protocol,
        base_url,
        latency_ms: 0,
        error: Some(err.to_string()),
      });
    }
  };

  let started = Instant::now();
  let outcome = tokio::time::timeout(Duration::from_secs(15), provider.validate_config()).await;
  let latency_ms = started.elapsed().as_millis() as u64;

  match outcome {
    Ok(Ok(())) => Ok(TestProviderResult {
      ok: true,
      provider: name,
      protocol,
      base_url,
      latency_ms,
      error: None,
    }),
    Ok(Err(err)) => Ok(TestProviderResult {
      ok: false,
      provider: name,
      protocol,
      base_url,
      latency_ms,
      error: Some(err.to_string()),
    }),
    Err(_) => Ok(TestProviderResult {
      ok: false,
      provider: name,
      protocol,
      base_url,
      latency_ms,
      error: Some("连接校验超时（15s），请检查 Base URL 与网络代理设置".to_string()),
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn dotenv_upsert_preserves_comments_and_other_keys_and_clears_cleanly() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let env_path = tmp.path().join(".env");
    std::fs::write(
      &env_path,
      "# Yanshi keys\n# OPENAI_API_KEY=old-comment\nHTTP_PROXY=http://127.0.0.1:7890\n",
    )
    .expect("seed .env");

    upsert_dotenv_var(&env_path, "OPENAI_API_KEY", Some("sk-test-1234567890")).expect("insert");
    let after_insert = std::fs::read_to_string(&env_path).expect("read");
    assert!(after_insert.contains("# Yanshi keys"));
    assert!(after_insert.contains("HTTP_PROXY=http://127.0.0.1:7890"));
    assert!(after_insert.contains("OPENAI_API_KEY=sk-test-1234567890"));
    assert!(!after_insert.contains("old-comment"));

    upsert_dotenv_var(&env_path, "OPENAI_API_KEY", None).expect("clear");
    let after_clear = std::fs::read_to_string(&env_path).expect("read");
    assert!(!after_clear.contains("OPENAI_API_KEY="));
    assert!(after_clear.contains("HTTP_PROXY=http://127.0.0.1:7890"));
  }

  #[test]
  fn mask_api_key_never_reveals_middle_characters() {
    assert_eq!(mask_api_key("sk-proj-secret-abcdef1234"), "sk-••••1234");
    assert_eq!(mask_api_key("short"), "••••hort");
    assert_eq!(mask_api_key("abc"), "••••");
  }

  #[tokio::test]
  async fn custom_openai_compatible_provider_and_model_round_trip_without_exposing_raw_key() {
    let tmp = tempfile::tempdir().expect("tempdir");

    let snap = upsert_provider_config(
      tmp.path(),
      "my-relay",
      UpsertProviderPatch {
        protocol: Some("openai".to_string()),
        base_url: Some("https://relay.example.com/v1/".to_string()),
        api_key_env: Some("MY_RELAY_API_KEY".to_string()),
        api_key: Some("sk-relay-super-secret-9988".to_string()),
        timeout_seconds: Some(120),
        requests_per_minute: Some(300),
      },
      false,
    )
    .await
    .expect("upsert custom provider");

    let relay = snap
      .providers
      .iter()
      .find(|p| p.name == "my-relay")
      .expect("custom provider present");
    assert!(relay.is_custom);
    assert_eq!(relay.protocol, "openai");
    assert_eq!(
      relay.base_url.as_deref(),
      Some("https://relay.example.com/v1")
    );
    assert!(relay.api_key_set);
    assert_eq!(relay.api_key_masked.as_deref(), Some("sk-••••9988"));
    assert_eq!(relay.api_key_source, "dotenv");

    // Raw API key must NEVER appear in models.yml!
    let models_yml = std::fs::read_to_string(tmp.path().join("models.yml")).expect("models.yml");
    assert!(!models_yml.contains("sk-relay-super-secret-9988"));

    // Add a custom model under `my-relay`
    let snap2 = upsert_model_config(
      tmp.path(),
      "relay-gpt-4o",
      UpsertModelPatch {
        vendor: "my-relay".to_string(),
        model_id: Some("openai/gpt-4o".to_string()),
        model_type: Some("chat".to_string()),
        accepts: Some(vec![InputType::Text, InputType::Image]),
        base_url: None,
        temperature: Some(0.3),
        max_tokens: Some(8192),
        supports_streaming: Some(true),
        supports_tools: Some(true),
        native_tool_calling: Some(true),
        supports_thinking: Some(false),
        thinking_kind: None,
      },
      false,
    )
    .await
    .expect("upsert custom model");

    let custom_model = snap2
      .models
      .iter()
      .find(|m| m.name == "relay-gpt-4o")
      .expect("custom model present");
    assert!(custom_model.is_custom);
    assert_eq!(custom_model.vendor, "my-relay");
    assert_eq!(custom_model.model_id, "openai/gpt-4o");
    assert_eq!(
      custom_model.effective_base_url.as_deref(),
      Some("https://relay.example.com/v1")
    );

    // Delete custom provider cleans up its models too.
    let snap3 = delete_provider_config(tmp.path(), "my-relay", true, false)
      .await
      .expect("delete custom provider");
    assert!(!snap3.providers.iter().any(|p| p.name == "my-relay"));
    assert!(!snap3.models.iter().any(|m| m.name == "relay-gpt-4o"));
  }

  #[tokio::test]
  async fn registry_routes_custom_protocol_and_per_model_base_url() {
    set_runtime_env_override("UNIT_CUSTOM_RELAY_KEY", Some("sk-unit-relay".to_string()));
    let yaml = r#"
providers:
  unit-relay:
    protocol: openai
    api_key_env: UNIT_CUSTOM_RELAY_KEY
    base_url: http://127.0.0.1:19991/v1
models:
  relay-default:
    vendor: unit-relay
    type: chat
  relay-override:
    vendor: unit-relay
    type: chat
    base_url: http://127.0.0.1:19992/v1
"#;
    let reg = ModelRegistry::new();
    reg
      .load_config_from_yaml(yaml)
      .await
      .expect("custom protocol loads");

    let p_default = reg
      .get_provider_for_model("relay-default", "unit-relay")
      .expect("default provider");
    assert_eq!(p_default.base_url(), "http://127.0.0.1:19991/v1");

    let p_override = reg
      .get_provider_for_model("relay-override", "unit-relay")
      .expect("per-model provider override");
    assert_eq!(p_override.base_url(), "http://127.0.0.1:19992/v1");

    clear_runtime_env_override("UNIT_CUSTOM_RELAY_KEY");
  }
}
