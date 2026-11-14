//! Shared endpoint and auth resolution for Google Gemini Developer API
//! (`generativelanguage.googleapis.com`) and Google Cloud Vertex AI / Agent
//! Platform (`aiplatform.googleapis.com`).

use crate::{LLMError, Result};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};

pub(crate) const DEFAULT_GEMINI_BASE_URL: &str = "https://generativelanguage.googleapis.com";
pub(crate) const DEFAULT_VERTEX_BASE_URL: &str = "https://aiplatform.googleapis.com";
pub(crate) const DEFAULT_VERTEX_PROJECT_ID: &str = "340214391376";
pub(crate) const DEFAULT_VERTEX_LOCATION: &str = "global";

/// Resolved Google API mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum GoogleApiMode {
  /// Standard Gemini Developer API (`/v1beta/models/{model}:{action}`).
  GeminiApi,
  /// Google Cloud Vertex AI / Agent Platform
  /// (`/v1/projects/{project_id}/locations/{location}/publishers/google/models/{model}:{action}`).
  VertexAi {
    project_id: String,
    location: String,
  },
}

/// Shared URL & header builder for `GoogleProvider`, `GoogleMediaProvider`,
/// and `GoogleVeoClient`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GoogleEndpoint {
  pub(crate) base_url: String,
  pub(crate) mode: GoogleApiMode,
}

impl GoogleEndpoint {
  /// Resolve the Google endpoint mode from `api_key`, optional `base_url`,
  /// and standard GCP environment variables (`GCP_PROJECT_ID` /
  /// `GOOGLE_CLOUD_PROJECT`, `GCP_LOCATION` / `GOOGLE_CLOUD_LOCATION`).
  pub(crate) fn new(api_key: &str, base_url: Option<String>) -> Self {
    let env_project = env_first(&["GCP_PROJECT_ID", "GOOGLE_CLOUD_PROJECT"]);
    let env_location = env_first(&["GCP_LOCATION", "GOOGLE_CLOUD_LOCATION"]);
    Self::with_env(api_key, base_url, env_project, env_location)
  }

  pub(crate) fn with_env(
    api_key: &str,
    base_url: Option<String>,
    env_project: Option<String>,
    env_location: Option<String>,
  ) -> Self {
    let trimmed_base = base_url
      .as_deref()
      .map(|s| s.trim_end_matches('/'))
      .filter(|s| !s.is_empty());

    let is_explicit_vertex = trimmed_base
      .map(|u| u.contains("aiplatform.googleapis.com"))
      .unwrap_or(false);
    let is_default_gemini_or_none =
      trimmed_base.is_none() || trimmed_base == Some(DEFAULT_GEMINI_BASE_URL);

    if is_explicit_vertex || (is_default_gemini_or_none && api_key.starts_with("AQ.")) {
      let (host_base, url_project, url_location) = match trimmed_base {
        Some(u) if is_explicit_vertex => parse_vertex_base_url(u),
        _ => (DEFAULT_VERTEX_BASE_URL.to_string(), None, None),
      };
      let project_id = url_project
        .or(env_project)
        .unwrap_or_else(|| DEFAULT_VERTEX_PROJECT_ID.to_string());
      let location = url_location
        .or(env_location)
        .unwrap_or_else(|| DEFAULT_VERTEX_LOCATION.to_string());
      return Self {
        base_url: host_base,
        mode: GoogleApiMode::VertexAi {
          project_id,
          location,
        },
      };
    }

    Self {
      base_url: trimmed_base.unwrap_or(DEFAULT_GEMINI_BASE_URL).to_string(),
      mode: GoogleApiMode::GeminiApi,
    }
  }

  pub(crate) fn is_vertex(&self) -> bool {
    matches!(self.mode, GoogleApiMode::VertexAi { .. })
  }

  /// Build standard headers carrying `x-goog-api-key` (never in the URL)
  /// and W3C trace context.
  pub(crate) fn build_headers(&self, api_key: &str) -> Result<HeaderMap> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
      "x-goog-api-key",
      HeaderValue::from_str(api_key).map_err(|err| LLMError::ConfigurationError {
        message: format!("Google API key contains non-ASCII bytes: {err}"),
      })?,
    );
    crate::trace_context::inject_into_headers(&mut headers);
    Ok(headers)
  }

  /// Build `{endpoint}/models/{model}:{action}{?query}` for either Gemini API
  /// or Vertex AI.
  pub(crate) fn model_action_url(&self, model: &str, action: &str, query: Option<&str>) -> String {
    self.model_action_url_with_location(model, action, query, None)
  }

  /// Same as [`Self::model_action_url`] with an optional Vertex AI
  /// `location` override (used by Veo when `us-central1` is needed).
  pub(crate) fn model_action_url_with_location(
    &self,
    model: &str,
    action: &str,
    query: Option<&str>,
    location_override: Option<&str>,
  ) -> String {
    let clean_model = strip_model_prefix(model);
    let q = match query {
      Some(q) if !q.is_empty() => format!("?{q}"),
      _ => String::new(),
    };
    match &self.mode {
      GoogleApiMode::GeminiApi => {
        format!(
          "{}/v1beta/models/{}:{}{}",
          self.base_url, clean_model, action, q
        )
      }
      GoogleApiMode::VertexAi {
        project_id,
        location,
      } => {
        let loc = location_override.unwrap_or(location);
        format!(
          "{}/v1/projects/{}/locations/{}/publishers/google/models/{}:{}{}",
          self.base_url, project_id, loc, clean_model, action, q
        )
      }
    }
  }
}

pub(crate) fn strip_model_prefix(model: &str) -> &str {
  model.strip_prefix("models/").unwrap_or(model)
}

fn env_first(keys: &[&str]) -> Option<String> {
  keys
    .iter()
    .find_map(|k| std::env::var(k).ok().filter(|v| !v.trim().is_empty()))
}

/// Extract `(scheme_and_host, optional_project, optional_location)` from a
/// Vertex AI URL such as
/// `https://aiplatform.googleapis.com/v1/projects/123/locations/global`.
fn parse_vertex_base_url(url: &str) -> (String, Option<String>, Option<String>) {
  let host = if let Some(idx) = url.find("/v1") {
    url[..idx].to_string()
  } else {
    url.to_string()
  };
  let parts: Vec<&str> = url.split('/').collect();
  let project = parts
    .windows(2)
    .find(|w| w[0] == "projects")
    .map(|w| w[1].to_string())
    .filter(|s| !s.is_empty());
  let location = parts
    .windows(2)
    .find(|w| w[0] == "locations")
    .map(|w| w[1].to_string())
    .filter(|s| !s.is_empty());
  (host, project, location)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn standard_key_uses_gemini_api_mode() {
    let ep = GoogleEndpoint::with_env("AIzaSyTestKey", None, None, None);
    assert!(!ep.is_vertex());
    assert_eq!(ep.base_url, DEFAULT_GEMINI_BASE_URL);
    assert_eq!(
      ep.model_action_url("models/gemini-3.1-flash-image", "generateContent", None),
      "https://generativelanguage.googleapis.com/v1beta/models/gemini-3.1-flash-image:generateContent"
    );
  }

  #[test]
  fn aq_key_auto_routes_to_vertex_ai_even_with_default_gemini_base_url() {
    let ep = GoogleEndpoint::with_env(
      "AQ.Ab8RN6TestKey",
      Some(DEFAULT_GEMINI_BASE_URL.to_string()),
      Some("999888777".to_string()),
      Some("global".to_string()),
    );
    assert!(ep.is_vertex());
    assert_eq!(
      ep.model_action_url("gemini-4-hydrogen", "generateContent", None),
      "https://aiplatform.googleapis.com/v1/projects/999888777/locations/global/publishers/google/models/gemini-4-hydrogen:generateContent"
    );
  }

  #[test]
  fn explicit_vertex_url_parses_project_and_location() {
    let ep = GoogleEndpoint::with_env(
      "AIzaOrToken",
      Some(
        "https://aiplatform.googleapis.com/v1/projects/111222/locations/us-central1".to_string(),
      ),
      None,
      None,
    );
    assert!(ep.is_vertex());
    assert_eq!(
      ep.model_action_url("veo-3.1-fast-generate-001", "predictLongRunning", None),
      "https://aiplatform.googleapis.com/v1/projects/111222/locations/us-central1/publishers/google/models/veo-3.1-fast-generate-001:predictLongRunning"
    );
  }

  #[test]
  fn stream_query_is_appended_cleanly() {
    let ep = GoogleEndpoint::with_env("AQ.Test", None, None, None);
    assert_eq!(
      ep.model_action_url(
        "gemini-4-hydrogen",
        "streamGenerateContent",
        Some("alt=sse")
      ),
      "https://aiplatform.googleapis.com/v1/projects/340214391376/locations/global/publishers/google/models/gemini-4-hydrogen:streamGenerateContent?alt=sse"
    );
  }
}
