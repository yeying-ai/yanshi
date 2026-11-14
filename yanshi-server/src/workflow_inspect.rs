//! Static workflow inspection route (`POST /v1/workflows/inspect`).
//!
//! Validates a YAML/JSON workflow definition and returns its declared
//! `inputs:`, topological DAG `nodes`/`edges`/`levels`, and schema
//! diagnostics without creating a run row or executing any node.

use axum::{Json, extract::State};
use serde::Deserialize;
use yanshi_config::{
  config::v2::FlowDefinitionV2,
  inspect::{InspectWorkflowResponse, inspect_flow_definition, inspect_workflow},
};

use crate::{
  AppState,
  error::{ApiError, JsonReq},
};

/// Request body for `POST /v1/workflows/inspect`.
#[derive(Debug, Deserialize)]
pub struct InspectWorkflowRequest {
  pub workflow: String,
}

/// `POST /v1/workflows/inspect` — statically inspect a workflow YAML/JSON string.
pub async fn post_inspect_workflow(
  State(state): State<AppState>,
  JsonReq(req): JsonReq<InspectWorkflowRequest>,
) -> Result<Json<InspectWorkflowResponse>, ApiError> {
  if req.workflow.trim().is_empty() {
    return Err(ApiError::BadRequest(
      "`workflow` must be a non-empty YAML or JSON string".into(),
    ));
  }

  let mut response = match serde_yaml::from_str::<FlowDefinitionV2>(&req.workflow) {
    Ok(flow_def) => {
      let mut res = inspect_flow_definition(&flow_def);
      if let Err(policy_err) = state.gateway_policy.check_workflow(&flow_def) {
        res
          .diagnostics
          .warnings
          .push(format!("gateway policy: {policy_err}"));
      }
      res
    }
    Err(_) => inspect_workflow(&req.workflow),
  };

  response.diagnostics.valid = response.valid;
  Ok(Json(response))
}
