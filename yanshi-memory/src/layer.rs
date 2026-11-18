//! Memory layer traits and shared types for P4.7.
//!
//! The four canonical memory layers are described in
//! `docs/MEMORY_LAYERING.md`. This module is the trait surface those
//! layers share.
//!
//! Why a dedicated module instead of bolting more methods onto
//! [`crate::MemoryStore`]: preferences and entity facts are not
//! `Message`-shaped, so every backend would have to stub the methods it
//! doesn't support. The separate traits here keep each backend honest:
//! a `SqlitePreferenceStore` implements `PreferenceStore` and nothing
//! else, an agent runtime that wants preferences asks for that trait
//! object explicitly, and a `MemoryStore` backend keeps doing what it's
//! always done.
//!
//! Stability: experimental at first land. See `docs/STABILITY.md`.

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{MemoryError, MemoryStore, Message};

/// Identifier for one of the four memory layers documented in
/// `docs/MEMORY_LAYERING.md`. Used by retention policies and by the
/// agent runtime when it decides which store to dispatch to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
  Session,
  Semantic,
  Preference,
  EntityFacts,
}

impl MemoryLayer {
  /// Stable string identifier used in CLI flags and skill manifests.
  pub const fn as_str(&self) -> &'static str {
    match self {
      Self::Session => "session",
      Self::Semantic => "semantic",
      Self::Preference => "preference",
      Self::EntityFacts => "entity_facts",
    }
  }
}

impl std::fmt::Display for MemoryLayer {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.as_str())
  }
}

/// Per-layer retention configuration. The CLI subcommand wiring is tracked
/// alongside the design doc; the type itself is what backends consult when
/// pruning.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RetentionPolicy {
  /// Hard cap on row age (by `updated_at` / `extracted_at`). `None` means
  /// keep indefinitely — the preference / entity-facts default.
  pub max_age: Option<Duration>,
  /// For Entity facts: how long to keep rows after `invalidated_at` is set
  /// before a `prune_invalidated` call drops them. Defaults to 2 years per
  /// the design doc.
  pub keep_invalidated_for: Option<Duration>,
}

impl RetentionPolicy {
  /// Default retention for the named layer. Matches the table in
  /// `docs/MEMORY_LAYERING.md`.
  pub fn default_for(layer: MemoryLayer) -> Self {
    match layer {
      MemoryLayer::Session => Self {
        max_age: None,
        keep_invalidated_for: None,
      },
      MemoryLayer::Semantic => Self {
        max_age: None,
        keep_invalidated_for: None,
      },
      MemoryLayer::Preference => Self {
        max_age: None,
        keep_invalidated_for: None,
      },
      MemoryLayer::EntityFacts => Self {
        max_age: None,
        // 2 years in seconds (365.25 days × 2)
        keep_invalidated_for: Some(Duration::from_secs(63_115_200)),
      },
    }
  }
}

// ── Preference layer ────────────────────────────────────────────────────────
//
// The contract (`PreferenceStore` trait + `PreferenceScope` +
// `PreferenceValue`) moved to `yanshi-store-spi::preference` (U2.6,
// mirroring the `ProjectMemoryStore` split in U2.5) and is re-exported
// below under its original `yanshi_memory::layer::*` paths so existing
// call sites keep compiling.
pub use yanshi_store_spi::{PreferenceScope, PreferenceStore, PreferenceValue};

// ── Entity facts layer ──────────────────────────────────────────────────────
//
// The contract (`EntityFactStore` trait + `EntityFact`) moved to
// `yanshi-store-spi::entity_facts` (Z3, `F-A3-2` + `F-A5-2`) with `&self`
// methods and `EntityFact::observed` (`confidence = 1.0`), re-exported here
// under its original `yanshi_memory::layer::*` paths so existing call sites
// keep compiling.
pub use yanshi_store_spi::{EntityFact, EntityFactStore};

// ── Semantic layer ──────────────────────────────────────────────────────────

/// Typed semantic search API on top of [`MemoryStore`].
///
/// `MemoryStore::search` is preserved for callers that already depend on
/// it; new code should reach for `search_semantic` since it returns the
/// per-row similarity scores. `session_id = None` is reserved for stores
/// that index across sessions — today's `SemanticMemory` requires a
/// session id, but the trait shape leaves the door open.
#[async_trait]
pub trait SemanticMemoryStore: MemoryStore {
  async fn search_semantic(
    &self,
    session_id: Option<&str>,
    query: &str,
    k: usize,
  ) -> Result<Vec<(Message, f32)>, MemoryError>;
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn memory_layer_as_str_matches_serde() {
    for layer in [
      MemoryLayer::Session,
      MemoryLayer::Semantic,
      MemoryLayer::Preference,
      MemoryLayer::EntityFacts,
    ] {
      let serialized = serde_json::to_string(&layer).unwrap();
      let trimmed = serialized.trim_matches('"');
      assert_eq!(trimmed, layer.as_str(), "serde / as_str must agree");
    }
  }

  #[test]
  fn entity_facts_default_keeps_invalidated_for_two_years() {
    let policy = RetentionPolicy::default_for(MemoryLayer::EntityFacts);
    let two_years = Duration::from_secs(63_115_200);
    assert_eq!(policy.keep_invalidated_for, Some(two_years));
    assert!(policy.max_age.is_none());
  }

  #[test]
  fn entity_fact_new_is_not_invalidated() {
    let fact = EntityFact::new("e1", "f1", "color", serde_json::json!("blue"), 0.9);
    assert!(!fact.is_invalidated());
    assert_eq!(fact.confidence, 0.9);
    assert!(fact.source_message_id.is_none());
  }
}
