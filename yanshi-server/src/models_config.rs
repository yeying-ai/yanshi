//! `/v1/models/*` — operator-facing model & provider configuration routes
//! backed by `yanshi_llm::config::user_store` (`~/.yanshi/models.yml` + `~/.yanshi/.env`).
//!
//! Security boundary:
//! - `GET /v1/models/config` never returns plaintext API keys (only masked last-4
//!   hints and source metadata) and inherits the standard `/v1/*` bearer auth middleware.
//! - Mutating (`PUT`/`DELETE`) and live connectivity (`POST .../test`) routes
//!   are enabled in `Dev`/`Local` profiles; under `SecurityProfile::Production`
//!   they fail closed with `403 Forbidden` unless the operator explicitly sets
//!   `YANSHI_ALLOW_MODEL_CONFIG_API=1`.

use axum::{
  Json,
  extract::{Path as RoutePath, Query, State},
};
use serde::Deserialize;
use yanshi_llm::config::user_store::{
  ModelConfigSnapshot, TestProviderRequest, TestProviderResult, UpsertModelPatch,
  UpsertProviderPatch, default_yanshi_config_dir, delete_model_config, delete_provider_config,
  get_model_config_snapshot, test_provider_connection, upsert_model_config, upsert_provider_config,
};
use yanshi_tools::SecurityProfile;

use crate::AppState;
use crate::error::ApiError;

pub const ALLOW_MODEL_CONFIG_API_ENV: &str = "YANSHI_ALLOW_MODEL_CONFIG_API";

#[derive(Debug, Default, Deserialize)]
pub struct DeleteProviderQuery {
  #[serde(default)]
  pub clear_api_key: Option<bool>,
}

fn ensure_writes_allowed(state: &AppState) -> Result<(), ApiError> {
  if state.security.profile != SecurityProfile::Production {
    return Ok(());
  }
  let opted_in = std::env::var(ALLOW_MODEL_CONFIG_API_ENV)
    .ok()
    .map(|v| v.trim().to_ascii_lowercase())
    .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes"));
  if opted_in {
    Ok(())
  } else {
    tracing::warn!(
      env_var = ALLOW_MODEL_CONFIG_API_ENV,
      "rejected model config write in production profile"
    );
    Err(ApiError::Forbidden)
  }
}

pub async fn get_models_config() -> Result<Json<ModelConfigSnapshot>, ApiError> {
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let snapshot = tokio::task::spawn_blocking(move || get_model_config_snapshot(&dir))
    .await
    .map_err(|err| ApiError::Internal(format!("models config task panicked: {err}")))?
    .map_err(|err| ApiError::Internal(err.to_string()))?;
  Ok(Json(snapshot))
}

pub async fn put_model_provider(
  State(state): State<AppState>,
  RoutePath(name): RoutePath<String>,
  Json(patch): Json<UpsertProviderPatch>,
) -> Result<Json<ModelConfigSnapshot>, ApiError> {
  ensure_writes_allowed(&state)?;
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let snapshot = upsert_provider_config(&dir, &name, patch, true)
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()))?;
  Ok(Json(snapshot))
}

pub async fn delete_model_provider(
  State(state): State<AppState>,
  RoutePath(name): RoutePath<String>,
  Query(query): Query<DeleteProviderQuery>,
) -> Result<Json<ModelConfigSnapshot>, ApiError> {
  ensure_writes_allowed(&state)?;
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let snapshot = delete_provider_config(&dir, &name, query.clear_api_key.unwrap_or(false), true)
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()))?;
  Ok(Json(snapshot))
}

pub async fn put_model_item(
  State(state): State<AppState>,
  RoutePath(name): RoutePath<String>,
  Json(patch): Json<UpsertModelPatch>,
) -> Result<Json<ModelConfigSnapshot>, ApiError> {
  ensure_writes_allowed(&state)?;
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let snapshot = upsert_model_config(&dir, &name, patch, true)
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()))?;
  Ok(Json(snapshot))
}

pub async fn delete_model_item(
  State(state): State<AppState>,
  RoutePath(name): RoutePath<String>,
) -> Result<Json<ModelConfigSnapshot>, ApiError> {
  ensure_writes_allowed(&state)?;
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let snapshot = delete_model_config(&dir, &name, true)
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()))?;
  Ok(Json(snapshot))
}

pub async fn test_model_provider(
  State(state): State<AppState>,
  RoutePath(name): RoutePath<String>,
  Json(req): Json<TestProviderRequest>,
) -> Result<Json<TestProviderResult>, ApiError> {
  ensure_writes_allowed(&state)?;
  let dir = default_yanshi_config_dir().map_err(|err| ApiError::Internal(err.to_string()))?;
  let result = test_provider_connection(&dir, &name, req)
    .await
    .map_err(|err| ApiError::BadRequest(err.to_string()))?;
  Ok(Json(result))
}
