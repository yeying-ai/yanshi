//! Process-local HITL `ask_user` question registry and handler for workflow
//! runs (`/v1/runs/:id/questions`) in the zero-Postgres Tauri Desktop engine.
//!
//! Mirrors `yanshi-server::run_questions` so inline `type: agent` workflow
//! nodes (such as `backend_project_planner`) pause cleanly, surface
//! `question.pending` progress events on SSE, and resume when answered in
//! Studio.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot;
use uuid::Uuid;

use yanshi_core::FlowCancellationToken;
use yanshi_core::run_context::{RunQuestionHandler, report_progress};

/// Default maximum time a desktop workflow agent waits on a single `ask_user`
/// prompt before failing the node cleanly.
pub const DEFAULT_RUN_QUESTION_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingRunQuestion {
  pub question_id: String,
  pub node_id: String,
  pub question: String,
  pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalQuestionResolveError {
  NotFound,
  WaiterGone,
}

type ParkedQuestionEntry = (PendingRunQuestion, oneshot::Sender<String>);

#[derive(Clone, Default)]
pub struct LocalRunQuestionRegistry {
  inner: Arc<Mutex<HashMap<Uuid, Vec<ParkedQuestionEntry>>>>,
}

impl std::fmt::Debug for LocalRunQuestionRegistry {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let active_runs = self
      .inner
      .lock()
      .unwrap_or_else(std::sync::PoisonError::into_inner)
      .len();
    f.debug_struct("LocalRunQuestionRegistry")
      .field("active_runs", &active_runs)
      .finish()
  }
}

impl LocalRunQuestionRegistry {
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
  ) -> Result<(), LocalQuestionResolveError> {
    let tx = {
      let mut guard = self
        .inner
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
      let Some(entries) = guard.get_mut(&run_id) else {
        return Err(LocalQuestionResolveError::NotFound);
      };
      let Some(pos) = entries
        .iter()
        .position(|(q, _)| q.question_id == question_id)
      else {
        return Err(LocalQuestionResolveError::NotFound);
      };
      let (_, tx) = entries.remove(pos);
      if entries.is_empty() {
        guard.remove(&run_id);
      }
      tx
    };
    tx.send(answer)
      .map_err(|_| LocalQuestionResolveError::WaiterGone)
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

pub struct LocalRunQuestionHandler {
  run_id: Uuid,
  registry: LocalRunQuestionRegistry,
  cancellation_token: FlowCancellationToken,
  timeout: Duration,
}

impl LocalRunQuestionHandler {
  pub fn new(
    run_id: Uuid,
    registry: LocalRunQuestionRegistry,
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

impl RunQuestionHandler for LocalRunQuestionHandler {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn registry_parks_lists_and_resolves_in_order() {
    let registry = LocalRunQuestionRegistry::new();
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
    let registry = LocalRunQuestionRegistry::new();
    let run_id = Uuid::new_v4();
    let token = FlowCancellationToken::new();
    let handler = LocalRunQuestionHandler::new(run_id, registry.clone(), token.clone());
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
