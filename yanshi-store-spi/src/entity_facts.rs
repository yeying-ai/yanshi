//! Entity-facts layer contract (Z3, `F-A3-2` + `F-A5-2`): durable,
//! provenance-tracked structured facts. See `docs/MEMORY_LAYERING.md` §4.
//!
//! Extracted from `yanshi-memory` mirroring `PreferenceStore` (U2.6) and
//! `ProjectMemoryStore` (U2.5). All write methods take `&self` rather than
//! `&mut self` (`sqlx::SqlitePool` and `InMemoryEntityFactStore` both use
//! interior mutability), so callers can share a single `Arc<dyn EntityFactStore>`
//! across concurrent DAG nodes without an external `Mutex` (`F-A5-2`).

use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::MemoryError;

/// A structured fact about an entity (person, project, codebase, …).
///
/// Two facts about the same `(entity_id, attribute)` stay as separate rows
/// (different `fact_id`s). The runtime renders each fact with its own
/// citation so the agent can show its work when challenged.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityFact {
  /// Stable id of the entity (UUID, slug, or external system id).
  pub entity_id: String,
  /// Stable id of *this fact*. Multiple facts can share an entity_id.
  pub fact_id: String,
  /// Name of the attribute (e.g. `"birthday"`, `"primary_language"`).
  pub attribute: String,
  /// Value associated with the attribute. JSON so structured facts can
  /// nest (`{"start": "2026-01-01", "end": "2026-12-31"}`).
  pub value: Value,
  /// Where this fact came from — usually a `message_id` from the
  /// conversation that extracted it. Optional so external pipelines can
  /// seed facts without a conversation.
  pub source_message_id: Option<String>,
  /// Extractor confidence in `[0.0, 1.0]`. Higher is more confident.
  pub confidence: f32,
  /// When the fact was extracted.
  pub extracted_at: DateTime<Utc>,
  /// When the fact was invalidated. Invalidated facts are preserved for
  /// audit until `prune_invalidated` removes them.
  pub invalidated_at: Option<DateTime<Utc>>,
  /// Operator-supplied reason captured on invalidation.
  pub invalidation_reason: Option<String>,
}

impl EntityFact {
  /// Convenience constructor for newly-extracted facts (not yet invalidated).
  pub fn new(
    entity_id: impl Into<String>,
    fact_id: impl Into<String>,
    attribute: impl Into<String>,
    value: Value,
    confidence: f32,
  ) -> Self {
    Self {
      entity_id: entity_id.into(),
      fact_id: fact_id.into(),
      attribute: attribute.into(),
      value,
      source_message_id: None,
      confidence,
      extracted_at: Utc::now(),
      invalidated_at: None,
      invalidation_reason: None,
    }
  }

  /// Constructor for deterministic / directly-observed facts (`confidence = 1.0`),
  /// such as ingestion watermarks and deduplication ledgers (`F-A3-2`).
  pub fn observed(
    entity_id: impl Into<String>,
    fact_id: impl Into<String>,
    attribute: impl Into<String>,
    value: Value,
  ) -> Self {
    Self::new(entity_id, fact_id, attribute, value, 1.0)
  }

  pub fn with_source(mut self, message_id: impl Into<String>) -> Self {
    self.source_message_id = Some(message_id.into());
    self
  }

  pub fn is_invalidated(&self) -> bool {
    self.invalidated_at.is_some()
  }
}

/// Durable, provenance-tracked structured-fact store. See
/// `docs/MEMORY_LAYERING.md` §4.
#[async_trait]
pub trait EntityFactStore: Send + Sync {
  /// Insert a new fact. Replaces an existing `(entity_id, fact_id)` row
  /// if one exists — callers that want a strict "no-clobber" insert
  /// should check `get_facts` first.
  async fn record_fact(&self, fact: EntityFact) -> Result<(), MemoryError>;

  /// Fetch every fact for an entity. `include_invalidated` controls
  /// whether previously-invalidated rows are returned (default: hide
  /// them — the agent should not surface stale facts to the user).
  async fn get_facts(
    &self,
    entity_id: &str,
    include_invalidated: bool,
  ) -> Result<Vec<EntityFact>, MemoryError>;

  /// Mark a fact invalidated. Preserves the row for audit; `prune_invalidated`
  /// hard-deletes it later.
  async fn invalidate_fact(
    &self,
    entity_id: &str,
    fact_id: &str,
    reason: &str,
  ) -> Result<(), MemoryError>;

  /// Hard-delete invalidated rows whose `invalidated_at` is older than
  /// `older_than`. Returns the number of rows removed.
  async fn prune_invalidated(&self, older_than: Duration) -> Result<u64, MemoryError>;
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn observed_sets_unit_confidence_and_active_state() {
    let fact = EntityFact::observed(
      "arxiv:cs.AI",
      "2501.00001",
      "first_seen",
      json!({"ok": true}),
    );
    assert!((fact.confidence - 1.0).abs() < f32::EPSILON);
    assert!(!fact.is_invalidated());
    assert!(fact.source_message_id.is_none());
  }
}
