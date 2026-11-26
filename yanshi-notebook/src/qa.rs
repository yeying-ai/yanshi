//! Grounded Q&A (`/ask`) DTOs, JSON Schema, reply parsing, citation validation,
//! and multi-turn history normalization.

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

/// Maximum prior Q&A turns retained in multi-turn grounded `/ask`.
pub const MAX_ASK_HISTORY_TURNS: usize = 8;
/// Per-field character cap when normalizing caller-supplied conversation turns.
pub const MAX_ASK_TURN_CHARS: usize = 2_000;

/// Query parameters for `GET /v1/notebooks/{id}/search`.
#[derive(Debug, Deserialize)]
pub struct SearchQuery {
  pub q: String,
  #[serde(default)]
  pub top_k: Option<usize>,
  /// Optional comma-separated source UUIDs scoping search to a subset of the
  /// notebook's sources (`?source_ids=uuid1,uuid2`).
  #[serde(default)]
  pub source_ids: Option<String>,
}

/// Single BM25 hit returned by `GET /v1/notebooks/{id}/search`.
#[derive(Debug, Serialize)]
pub struct SearchHit {
  pub source_id: Uuid,
  pub chunk_id: String,
  pub content: String,
  pub score: f32,
}

/// Query parameters for `GET /v1/notebooks/{id}/messages`.
#[derive(Debug, Default, Deserialize)]
pub struct ListMessagesQuery {
  #[serde(default)]
  pub limit: Option<usize>,
}

/// One prior question/answer turn supplied for multi-turn grounded `/ask`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NotebookConversationTurn {
  #[serde(default)]
  pub question: String,
  #[serde(default)]
  pub answer: String,
}

/// Request payload for `POST /v1/notebooks/{id}/ask`.
#[derive(Debug, Deserialize)]
pub struct AskNotebookRequest {
  pub question: String,
  /// Optional prior Q&A turns for multi-turn follow-up grounding.
  #[serde(default)]
  pub history: Vec<NotebookConversationTurn>,
  /// Optional subset of notebook source IDs to ground the answer against.
  #[serde(default)]
  pub source_ids: Option<Vec<Uuid>>,
  /// Chat model for grounded synthesis; falls back to `YANSHI_GENERATE_MODEL`.
  #[serde(default)]
  pub model: Option<String>,
  /// Optional cap on how many relevant passages are passed to the model.
  #[serde(default)]
  pub top_k: Option<usize>,
}

/// A verified citation linking an answer quote back to its source chunk.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GroundedAnswerCitation {
  pub source_id: Uuid,
  pub chunk_id: String,
  pub quote: String,
}

/// Response payload for `POST /v1/notebooks/{id}/ask`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskNotebookResponse {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub message_id: Option<Uuid>,
  pub notebook_id: Uuid,
  pub question: String,
  pub answer: String,
  pub model: String,
  pub citations: Vec<GroundedAnswerCitation>,
  pub used_chunk_count: usize,
}

/// Strict JSON Schema requested from the LLM for grounded Q&A replies.
pub fn grounded_answer_schema() -> Value {
  json!({
    "type": "object",
    "additionalProperties": false,
    "required": ["answer", "citations"],
    "properties": {
      "answer": { "type": "string" },
      "citations": {
        "type": "array",
        "items": {
          "type": "object",
          "additionalProperties": false,
          "required": ["chunk_id", "quote"],
          "properties": {
            "chunk_id": { "type": "string" },
            "quote": { "type": "string" }
          }
        }
      }
    }
  })
}

/// Resolve the chat model from an explicit request field or `YANSHI_GENERATE_MODEL`.
pub fn resolve_notebook_model(explicit: Option<&str>) -> Result<String, String> {
  explicit
    .map(str::trim)
    .filter(|m| !m.is_empty())
    .map(str::to_string)
    .or_else(|| {
      std::env::var("YANSHI_GENERATE_MODEL")
        .ok()
        .map(|m| m.trim().to_string())
        .filter(|m| !m.is_empty())
    })
    .ok_or_else(|| "set `model`, or YANSHI_GENERATE_MODEL on the gateway".to_string())
}

/// Parse and validate a grounded Q&A JSON reply, filtering citations to known chunk IDs.
pub fn parse_grounded_answer_reply(
  raw_reply: &str,
  chunk_sources: &HashMap<String, Uuid>,
) -> Result<(String, Vec<GroundedAnswerCitation>), String> {
  #[derive(Deserialize)]
  struct RawCitation {
    #[serde(default)]
    chunk_id: String,
    #[serde(default)]
    quote: String,
  }
  #[derive(Deserialize)]
  struct RawAnswer {
    answer: String,
    #[serde(default)]
    citations: Vec<RawCitation>,
  }

  let value = extract_json_object(raw_reply)?;
  let parsed: RawAnswer =
    serde_json::from_value(value).map_err(|e| format!("invalid grounded answer JSON: {e}"))?;
  let answer = parsed.answer.trim().to_string();
  if answer.is_empty() {
    return Err("grounded answer was empty".to_string());
  }

  let mut seen: HashSet<(String, String)> = HashSet::new();
  let mut citations = Vec::new();
  for raw in parsed.citations {
    let cid = raw.chunk_id.trim();
    let quote = raw.quote.trim();
    if cid.is_empty() || quote.is_empty() {
      continue;
    }
    let Some(&source_id) = chunk_sources.get(cid) else {
      continue;
    };
    let key = (cid.to_string(), quote.to_string());
    if seen.insert(key) {
      citations.push(GroundedAnswerCitation {
        source_id,
        chunk_id: cid.to_string(),
        quote: quote.to_string(),
      });
    }
  }
  Ok((answer, citations))
}

/// Extract a JSON object from raw model output, tolerating `<think>` blocks and code fences.
pub fn extract_json_object(reply: &str) -> Result<Value, String> {
  let trimmed = reply.trim();
  if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
    return Ok(v);
  }
  let without_think = if let Some(start) = trimmed.find("<think>")
    && let Some(end) = trimmed[start..].find("</think>")
  {
    format!(
      "{}{}",
      &trimmed[..start],
      &trimmed[start + end + "</think>".len()..]
    )
  } else {
    trimmed.to_string()
  };
  let s = without_think.trim();
  if let Ok(v) = serde_json::from_str::<Value>(s) {
    return Ok(v);
  }
  if let Some(fence_start) = s.find("```") {
    let after_ticks = &s[fence_start + 3..];
    let body_start = after_ticks.find('\n').map_or(0, |nl| nl + 1);
    let body = &after_ticks[body_start..];
    if let Some(fence_end) = body.find("```")
      && let Ok(v) = serde_json::from_str::<Value>(body[..fence_end].trim())
    {
      return Ok(v);
    }
  }
  if let Some(open) = s.find('{')
    && let Some(close) = s.rfind('}')
    && close > open
    && let Ok(v) = serde_json::from_str::<Value>(&s[open..=close])
  {
    return Ok(v);
  }
  Err("model reply did not contain valid JSON".to_string())
}

/// Trim whitespace and truncate a string to at most `max_chars` Unicode scalar values.
pub fn truncate_chars(s: &str, max_chars: usize) -> String {
  s.trim().chars().take(max_chars).collect()
}

/// Clean, truncate, and cap multi-turn `/ask` conversation history.
pub fn normalize_ask_history(raw: &[NotebookConversationTurn]) -> Vec<NotebookConversationTurn> {
  let cleaned: Vec<NotebookConversationTurn> = raw
    .iter()
    .filter_map(|turn| {
      let question = truncate_chars(&turn.question, MAX_ASK_TURN_CHARS);
      let answer = truncate_chars(&turn.answer, MAX_ASK_TURN_CHARS);
      if question.is_empty() && answer.is_empty() {
        None
      } else {
        Some(NotebookConversationTurn { question, answer })
      }
    })
    .collect();
  let skip = cleaned.len().saturating_sub(MAX_ASK_HISTORY_TURNS);
  cleaned.into_iter().skip(skip).collect()
}

/// Build a BM25 retrieval hint combining recent conversation turns with the
/// active follow-up question so pronoun/coreference follow-ups still match
/// subjects introduced in prior turns.
pub fn build_ask_retrieval_hint(question: &str, history: &[NotebookConversationTurn]) -> String {
  let recent_skip = history.len().saturating_sub(2);
  let mut parts: Vec<String> = Vec::with_capacity(5);
  for turn in history.iter().skip(recent_skip) {
    if !turn.question.is_empty() {
      parts.push(truncate_chars(&turn.question, 240));
    }
    if !turn.answer.is_empty() {
      parts.push(truncate_chars(&turn.answer, 240));
    }
  }
  parts.push(question.trim().to_string());
  parts.join(" ")
}

/// Format normalized conversation turns as a grounded prompt prefix block.
pub fn format_ask_history_block(history: &[NotebookConversationTurn]) -> String {
  if history.is_empty() {
    return String::new();
  }
  let turns = history
    .iter()
    .enumerate()
    .map(|(idx, turn)| {
      format!(
        "Turn {}:\nUser: {}\nAssistant: {}",
        idx + 1,
        turn.question,
        turn.answer
      )
    })
    .collect::<Vec<_>>()
    .join("\n\n");
  format!(
    "Prior conversation context (use ONLY to resolve follow-up references; every factual claim MUST still be grounded in Source passages below):\n{turns}\n\n"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn parse_grounded_answer_reply_filters_unknown_chunks_and_deduplicates() {
    let src_id = Uuid::new_v4();
    let mut map = HashMap::new();
    map.insert(format!("{src_id}:0"), src_id);

    let raw = format!(
      "<think>reasoning</think>\n```json\n{{\"answer\":\"Verified\",\"citations\":[{{\"chunk_id\":\"{src_id}:0\",\"quote\":\"verbatim\"}},{{\"chunk_id\":\"{src_id}:0\",\"quote\":\"verbatim\"}},{{\"chunk_id\":\"hallucinated:9\",\"quote\":\"nope\"}}]}}\n```"
    );
    let (answer, citations) = parse_grounded_answer_reply(&raw, &map).expect("parsed");
    assert_eq!(answer, "Verified");
    assert_eq!(citations.len(), 1);
    assert_eq!(citations[0].source_id, src_id);
    assert_eq!(citations[0].quote, "verbatim");
  }
}
