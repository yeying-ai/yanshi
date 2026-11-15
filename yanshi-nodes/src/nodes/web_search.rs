use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};
use yanshi_core::{
  async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult},
  error::YanshiError,
  value::FlowValue,
};
use yanshi_tools::SandboxPolicy;
use yanshi_tools::builtin::{SearchConfig, WebSearchTool};

/// Workflow node that executes a web search via [`WebSearchTool`].
///
/// Outputs:
/// - `results`: JSON array of `{ title, url, snippet, source }`
/// - `markdown`: Numbered Markdown summary ready to pass to downstream `llm` nodes
/// - `output`: Alias for `markdown`
/// - `provider_used`: Name of the search backend that served the query
#[derive(Clone)]
pub struct WebSearchNode {
  policy: Arc<SandboxPolicy>,
  config: Option<SearchConfig>,
  no_proxy: bool,
}

impl std::fmt::Debug for WebSearchNode {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("WebSearchNode").finish_non_exhaustive()
  }
}

impl WebSearchNode {
  pub fn new(policy: Arc<SandboxPolicy>) -> Self {
    Self {
      policy,
      config: None,
      no_proxy: false,
    }
  }

  pub fn with_config(mut self, config: SearchConfig) -> Self {
    self.config = Some(config);
    self
  }

  pub fn with_no_proxy(mut self) -> Self {
    self.no_proxy = true;
    self
  }
}

impl Default for WebSearchNode {
  fn default() -> Self {
    Self::new(Arc::new(SandboxPolicy::default()))
  }
}

#[async_trait]
impl AsyncNode for WebSearchNode {
  async fn execute(&self, inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let query = inputs
      .get("query")
      .and_then(|v| match v {
        FlowValue::Json(Value::String(s)) => Some(s.trim()),
        _ => None,
      })
      .filter(|s| !s.is_empty())
      .ok_or_else(|| YanshiError::NodeInputError {
        message: "Required non-empty string input 'query' is missing".to_string(),
      })?;

    let provider = read_optional_string(inputs, "provider")?.unwrap_or("auto");
    let topic = read_optional_string(inputs, "topic")?.unwrap_or("general");
    let max_results = read_optional_u64(inputs, "max_results")?.unwrap_or(5);

    let mut tool =
      WebSearchTool::new(self.policy.clone()).map_err(|err| YanshiError::AsyncExecutionError {
        message: format!("WebSearchNode failed to build client: {err}"),
      })?;
    if let Some(cfg) = &self.config {
      tool = tool.with_config(cfg.clone());
    }
    if self.no_proxy {
      tool = tool.with_no_proxy();
    }

    let response = tool
      .search(&json!({
        "query": query,
        "provider": provider,
        "max_results": max_results,
        "topic": topic,
      }))
      .await
      .map_err(|err| YanshiError::AsyncExecutionError {
        message: err.to_string(),
      })?;

    let results_json =
      serde_json::to_value(&response.results).map_err(|err| YanshiError::NodeExecutionFailed {
        message: format!("Failed to serialize search results: {err}"),
      })?;

    let mut outputs = HashMap::new();
    outputs.insert("results".to_string(), FlowValue::Json(results_json));
    outputs.insert(
      "markdown".to_string(),
      FlowValue::Json(json!(response.markdown)),
    );
    outputs.insert(
      "output".to_string(),
      FlowValue::Json(json!(response.markdown)),
    );
    outputs.insert(
      "provider_used".to_string(),
      FlowValue::Json(json!(response.provider)),
    );
    Ok(outputs)
  }
}

fn read_optional_string<'a>(
  inputs: &'a AsyncNodeInputs,
  key: &str,
) -> Result<Option<&'a str>, YanshiError> {
  match inputs.get(key) {
    None | Some(FlowValue::Json(Value::Null)) => Ok(None),
    Some(FlowValue::Json(Value::String(s))) => {
      let trimmed = s.trim();
      if trimmed.is_empty() {
        Ok(None)
      } else {
        Ok(Some(trimmed))
      }
    }
    _ => Err(YanshiError::NodeInputError {
      message: format!("Input '{key}' has wrong type, expected string"),
    }),
  }
}

fn read_optional_u64(inputs: &AsyncNodeInputs, key: &str) -> Result<Option<u64>, YanshiError> {
  match inputs.get(key) {
    None | Some(FlowValue::Json(Value::Null)) => Ok(None),
    Some(FlowValue::Json(Value::Number(n))) => {
      n.as_u64()
        .map(Some)
        .ok_or_else(|| YanshiError::NodeInputError {
          message: format!("Input '{key}' must be a positive integer"),
        })
    }
    Some(FlowValue::Json(Value::String(s))) => {
      let trimmed = s.trim();
      if trimmed.is_empty() {
        return Ok(None);
      }
      trimmed
        .parse::<u64>()
        .map(Some)
        .map_err(|_| YanshiError::NodeInputError {
          message: format!("Input '{key}' must be an integer, got '{trimmed}'"),
        })
    }
    _ => Err(YanshiError::NodeInputError {
      message: format!("Input '{key}' has wrong type, expected integer"),
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  #[tokio::test]
  async fn executes_search_and_populates_markdown_and_results() {
    let body = r#"{
      "organic": [
        {
          "title": "Yanshi DAG + Agent Runtime",
          "link": "https://yanshi.yeying.dev/docs",
          "snippet": "Deterministic DAG workflows with embedded ReAct agents."
        }
      ]
    }"#;
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let resp = format!(
      "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    );
    let server = tokio::spawn(async move {
      if let Ok((mut stream, _)) = listener.accept().await {
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf).await;
        let _ = stream.write_all(resp.as_bytes()).await;
      }
    });

    let policy = Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    });
    let node = WebSearchNode::new(policy)
      .with_no_proxy()
      .with_config(SearchConfig {
        serper_api_key: Some("serper-test".to_string()),
        serper_url: Some(format!("http://{addr}")),
        ..SearchConfig::default()
      });

    let mut inputs = AsyncNodeInputs::new();
    inputs.insert(
      "query".to_string(),
      FlowValue::Json(json!("Yanshi workflow engine")),
    );
    inputs.insert("max_results".to_string(), FlowValue::Json(json!("3")));

    let out = node.execute(&inputs).await.expect("node executes");
    server.await.expect("server");

    assert_eq!(out["provider_used"], FlowValue::Json(json!("serper")));
    let FlowValue::Json(Value::String(md)) = &out["markdown"] else {
      panic!("expected string markdown output");
    };
    assert!(md.contains("Yanshi DAG + Agent Runtime"));
    let FlowValue::Json(Value::Array(results)) = &out["results"] else {
      panic!("expected array results output");
    };
    assert_eq!(results.len(), 1);
  }
}
