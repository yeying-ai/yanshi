//! Interactive human-in-the-loop question registry and HTTP routes for
//! workflow runs (`/v1/runs/{id}/questions`).
//!
//! When an inline `type: agent` node calls `ask_user`, [`ServerRunQuestionHandler`]
//! parks a [`PendingRunQuestion`] + `oneshot::Sender<String>` on
//! [`RunQuestionRegistry`] and emits `WorkflowEvent::NodeProgress` with
//! `kind: "question.pending"`. Clients (such as Workflow Studio) list
//! pending questions via `GET /v1/runs/{id}/questions` and submit answers via
//! `POST /v1/runs/{id}/questions/{question_id}`.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::{
  Extension, Json,
  extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;
use uuid::Uuid;

use yanshi_core::FlowCancellationToken;
use yanshi_core::run_context::{RunQuestionHandler, report_progress};
use yanshi_db::RunRepo;

use crate::AppState;
use crate::error::{ApiError, JsonReq};
use crate::tenant::TenantId;

/// Default maximum time a workflow agent waits on a single `ask_user` prompt
/// before failing the node cleanly (prevents abandoned browser tabs from
/// holding run slots forever).
pub const DEFAULT_RUN_QUESTION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRunQuestion {
  pub question_id: String,
  pub node_id: String,
  pub question: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestionResolveError {
  NotFound,
  WaiterGone,
}

type ParkedQuestionEntry = (PendingRunQuestion, oneshot::Sender<String>);

#[derive(Clone, Default)]
pub struct RunQuestionRegistry {
  inner: Arc<Mutex<HashMap<Uuid, Vec<ParkedQuestionEntry>>>>,
}

impl std::fmt::Debug for RunQuestionRegistry {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let active_runs = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .len();
    f.debug_struct("RunQuestionRegistry")
      .field("active_runs", &active_runs)
      .finish()
  }
}

impl RunQuestionRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn park(&self, run_id: Uuid, question: PendingRunQuestion) -> oneshot::Receiver<String> {
    let (tx, rx) = oneshot::channel();
    let mut guard = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.entry(run_id).or_default().push((question, tx));
    rx
  }

  pub fn list(&self, run_id: Uuid) -> Vec<PendingRunQuestion> {
    let guard = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard
      .get(&run_id)
      .map(|entries| entries.iter().map(|(q, _)| q.clone()).collect())
      .unwrap_or_default()
  }

  pub fn answer(
    &self,
    run_id: Uuid,
    question_id: &str,
    answer: String,
  ) -> Result<(), QuestionResolveError> {
    let tx = {
      let mut guard = self
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
      let Some(entries) = guard.get_mut(&run_id) else {
        return Err(QuestionResolveError::NotFound);
      };
      let Some(pos) = entries
        .iter()
        .position(|(q, _)| q.question_id == question_id)
      else {
        return Err(QuestionResolveError::NotFound);
      };
      let (_, tx) = entries.remove(pos);
      if entries.is_empty() {
        guard.remove(&run_id);
      }
      tx
    };
    tx.send(answer)
      .map_err(|_| QuestionResolveError::WaiterGone)
  }

  pub fn remove(&self, run_id: Uuid, question_id: &str) {
    let mut guard = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(entries) = guard.get_mut(&run_id) {
      entries.retain(|(q, _)| q.question_id != question_id);
      if entries.is_empty() {
        guard.remove(&run_id);
      }
    }
  }

  pub fn clear_run(&self, run_id: Uuid) {
    let mut guard = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner);
    guard.remove(&run_id);
  }
}

pub struct ServerRunQuestionHandler {
  run_id: Uuid,
  registry: RunQuestionRegistry,
  cancellation_token: FlowCancellationToken,
  timeout: Duration,
}

impl ServerRunQuestionHandler {
  pub fn new(
    run_id: Uuid,
    registry: RunQuestionRegistry,
    cancellation_token: FlowCancellationToken,
  ) -> Self {
    Self {
      run_id,
      registry,
      cancellation_token,
      timeout: DEFAULT_RUN_QUESTION_TIMEOUT,
    }
  }

  pub fn with_timeout(mut self, timeout: Duration) -> Self {
    self.timeout = timeout;
    self
  }
}

impl RunQuestionHandler for ServerRunQuestionHandler {
  fn ask_user<'a>(
    &'a self,
    node_id: &'a str,
    question: &'a str,
  ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
    Box::pin(async move {
      let question_id = Uuid::new_v4().to_string();
      let created_at = Utc::now();
      let pending = PendingRunQuestion {
        question_id: question_id.clone(),
        node_id: node_id.to_string(),
        question: question.to_string(),
        created_at,
      };
      let rx = self.registry.park(self.run_id, pending);
      report_progress(
        "question.pending",
        json!({
          "question_id": question_id,
          "node_id": node_id,
          "question": question,
          "created_at": created_at,
        }),
      );

      let outcome = tokio::select! {
        res = rx => match res {
          Ok(answer) => Ok(answer),
          Err(_) => Err("question channel closed before an answer arrived".to_string()),
        },
        () = self.cancellation_token.cancelled() => {
          self.registry.remove(self.run_id, &question_id);
          Err("cancelled".to_string())
        },
        () = tokio::time::sleep(self.timeout) => {
          self.registry.remove(self.run_id, &question_id);
          Err(format!(
            "timed out after {}s waiting for user answer",
            self.timeout.as_secs()
          ))
        },
      };

      if let Ok(ref answer) = outcome {
        report_progress(
          "question.answered",
          json!({
            "question_id": question_id,
            "node_id": node_id,
            "answer_chars": answer.chars().count(),
          }),
        );
      }
      outcome
    })
  }
}

#[derive(Debug, Serialize)]
pub struct PendingRunQuestionsResponse {
  pub pending: Vec<PendingRunQuestion>,
}

#[derive(Debug, Deserialize)]
pub struct AnswerRunQuestionRequest {
  pub answer: String,
}

#[derive(Debug, Serialize)]
pub struct AnswerRunQuestionResponse {
  pub run_id: Uuid,
  pub question_id: String,
  pub resolved: bool,
}

/// `GET /v1/runs/{id}/questions` — list pending `ask_user` questions for an
/// active workflow run, oldest first.
pub async fn list_run_questions(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path(run_id): Path<Uuid>,
) -> Result<Json<PendingRunQuestionsResponse>, ApiError> {
  let run = state
    .repos
    .runs
    .get(run_id)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("run {} not found", run_id)))?;
  if run.tenant_id != tenant.as_str() {
    return Err(ApiError::NotFound(format!("run {} not found", run_id)));
  }

  let pending = state.question_registry.list(run_id);
  Ok(Json(PendingRunQuestionsResponse { pending }))
}

/// `POST /v1/runs/{id}/questions/{question_id}` — submit the user's answer
/// to a pending `ask_user` prompt and resume the paused workflow node.
pub async fn answer_run_question(
  State(state): State<AppState>,
  Extension(tenant): Extension<TenantId>,
  Path((run_id, question_id)): Path<(Uuid, String)>,
  JsonReq(body): JsonReq<AnswerRunQuestionRequest>,
) -> Result<Json<AnswerRunQuestionResponse>, ApiError> {
  let run = state
    .repos
    .runs
    .get(run_id)
    .await?
    .ok_or_else(|| ApiError::NotFound(format!("run {} not found", run_id)))?;
  if run.tenant_id != tenant.as_str() {
    return Err(ApiError::NotFound(format!("run {} not found", run_id)));
  }
  let trimmed = body.answer.trim();
  if trimmed.is_empty() {
    return Err(ApiError::BadRequest("answer must not be empty".to_string()));
  }

  match state
    .question_registry
    .answer(run_id, &question_id, trimmed.to_string())
  {
    Ok(()) => Ok(Json(AnswerRunQuestionResponse {
      run_id,
      question_id,
      resolved: true,
    })),
    Err(QuestionResolveError::WaiterGone) => Err(ApiError::BadRequest(format!(
      "question {} cannot be answered: node future already dropped",
      question_id
    ))),
    Err(QuestionResolveError::NotFound) => Err(ApiError::NotFound(format!(
      "no pending question {} for run {}",
      question_id, run_id
    ))),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn registry_parks_lists_and_resolves_in_order() {
    let registry = RunQuestionRegistry::new();
    let run_id = Uuid::new_v4();
    let rx = registry.park(
      run_id,
      PendingRunQuestion {
        question_id: "q-1".to_string(),
        node_id: "clarify".to_string(),
        question: "What database engine?".to_string(),
        created_at: Utc::now(),
      },
    );
    assert_eq!(registry.list(run_id).len(), 1);
    registry
      .answer(run_id, "q-1", "PostgreSQL 17".to_string())
      .expect("resolve");
    assert!(registry.list(run_id).is_empty());
    assert_eq!(rx.await.expect("answer"), "PostgreSQL 17");
  }

  #[tokio::test]
  async fn handler_aborts_cleanly_when_run_is_cancelled() {
    let registry = RunQuestionRegistry::new();
    let run_id = Uuid::new_v4();
    let token = FlowCancellationToken::new();
    let handler = ServerRunQuestionHandler::new(run_id, registry.clone(), token.clone());
    let canceller = tokio::spawn(async move {
      tokio::time::sleep(Duration::from_millis(25)).await;
      token.cancel();
    });
    let err = handler
      .ask_user("clarify", "Target QPS?")
      .await
      .expect_err("must cancel");
    let _ = canceller.await;
    assert_eq!(err, "cancelled");
    assert!(registry.list(run_id).is_empty());
  }
}
