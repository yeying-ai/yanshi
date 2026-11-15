use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use url::Url;

use crate::{
  Tool, ToolError, ToolIdempotency, ToolMetadata, ToolOutput, builtin::HttpTool,
  sandbox::SandboxPolicy,
};

const DEFAULT_TAVILY_URL: &str = "https://api.tavily.com/search";
const DEFAULT_BOCHA_URL: &str = "https://api.bochaai.com/v1/web-search";
const DEFAULT_SERPER_URL: &str = "https://google.serper.dev/search";
const DEFAULT_BRAVE_URL: &str = "https://api.search.brave.com/res/v1/web/search";
const DEFAULT_DUCKDUCKGO_URL: &str = "https://html.duckduckgo.com/html/";
const DEFAULT_WIKIPEDIA_EN_URL: &str = "https://en.wikipedia.org/w/api.php";
const DEFAULT_WIKIPEDIA_ZH_URL: &str = "https://zh.wikipedia.org/w/api.php";
const DEFAULT_HN_ALGOLIA_URL: &str = "https://hn.algolia.com/api/v1/search";

/// One normalized search hit returned by [`WebSearchTool`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResultItem {
  pub title: String,
  pub url: String,
  pub snippet: String,
  pub source: String,
}

/// Normalized multi-provider search response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SearchResponse {
  pub provider: String,
  pub query: String,
  pub results: Vec<SearchResultItem>,
  pub markdown: String,
}

/// Optional credential and endpoint overrides for [`WebSearchTool`].
#[derive(Debug, Clone, Default)]
pub struct SearchConfig {
  pub tavily_api_key: Option<String>,
  pub tavily_url: Option<String>,
  pub bocha_api_key: Option<String>,
  pub bocha_url: Option<String>,
  pub serper_api_key: Option<String>,
  pub serper_url: Option<String>,
  pub brave_api_key: Option<String>,
  pub brave_url: Option<String>,
  pub searxng_url: Option<String>,
  pub duckduckgo_url: Option<String>,
  pub wikipedia_en_url: Option<String>,
  pub wikipedia_zh_url: Option<String>,
  pub hn_algolia_url: Option<String>,
}

impl SearchConfig {
  pub fn from_env() -> Self {
    let env_opt = |key: &str| -> Option<String> {
      std::env::var(key)
        .ok()
        .map(|val| val.trim().to_string())
        .filter(|val| !val.is_empty())
    };
    Self {
      tavily_api_key: env_opt("TAVILY_API_KEY"),
      tavily_url: env_opt("TAVILY_API_URL"),
      bocha_api_key: env_opt("BOCHA_API_KEY"),
      bocha_url: env_opt("BOCHA_API_URL"),
      serper_api_key: env_opt("SERPER_API_KEY"),
      serper_url: env_opt("SERPER_API_URL"),
      brave_api_key: env_opt("BRAVE_API_KEY"),
      brave_url: env_opt("BRAVE_API_URL"),
      searxng_url: env_opt("SEARXNG_URL"),
      duckduckgo_url: None,
      wikipedia_en_url: None,
      wikipedia_zh_url: None,
      hn_algolia_url: None,
    }
  }

  fn resolved_with_env(&self) -> Self {
    let env = Self::from_env();
    Self {
      tavily_api_key: self.tavily_api_key.clone().or(env.tavily_api_key),
      tavily_url: self.tavily_url.clone().or(env.tavily_url),
      bocha_api_key: self.bocha_api_key.clone().or(env.bocha_api_key),
      bocha_url: self.bocha_url.clone().or(env.bocha_url),
      serper_api_key: self.serper_api_key.clone().or(env.serper_api_key),
      serper_url: self.serper_url.clone().or(env.serper_url),
      brave_api_key: self.brave_api_key.clone().or(env.brave_api_key),
      brave_url: self.brave_url.clone().or(env.brave_url),
      searxng_url: self.searxng_url.clone().or(env.searxng_url),
      duckduckgo_url: self.duckduckgo_url.clone(),
      wikipedia_en_url: self.wikipedia_en_url.clone(),
      wikipedia_zh_url: self.wikipedia_zh_url.clone(),
      hn_algolia_url: self.hn_algolia_url.clone(),
    }
  }
}

/// Multi-provider web search tool backed by [`HttpTool`] SSRF and DNS-pinning guards.
pub struct WebSearchTool {
  http: HttpTool,
  config: Option<SearchConfig>,
}

impl WebSearchTool {
  pub fn new(policy: Arc<SandboxPolicy>) -> Result<Self, ToolError> {
    let http = HttpTool::new(policy)?.with_max_response_chars(usize::MAX);
    Ok(Self { http, config: None })
  }

  pub fn default_policy() -> Result<Self, ToolError> {
    Self::new(Arc::new(SandboxPolicy::default()))
  }

  /// Pin explicit credentials/endpoints (isolated from process env when set).
  pub fn with_config(mut self, config: SearchConfig) -> Self {
    self.config = Some(config);
    self
  }

  /// Bypass system HTTP proxies (used by local loopback tests).
  pub fn with_no_proxy(mut self) -> Self {
    self.http = self.http.with_no_proxy();
    self
  }

  fn active_config(&self) -> SearchConfig {
    match &self.config {
      Some(cfg) => cfg.clone(),
      None => SearchConfig::default().resolved_with_env(),
    }
  }

  pub async fn search(&self, params: &Value) -> Result<SearchResponse, ToolError> {
    let query = params["query"]
      .as_str()
      .map(str::trim)
      .filter(|q| !q.is_empty())
      .ok_or_else(|| ToolError::InvalidParams {
        message: "Missing or empty required parameter 'query'".to_string(),
      })?;

    let provider = params["provider"]
      .as_str()
      .map(|s| s.trim().to_ascii_lowercase())
      .filter(|s| !s.is_empty())
      .unwrap_or_else(|| "auto".to_string());

    let max_results = parse_max_results(&params["max_results"]).clamp(1, 20);
    let topic = params["topic"]
      .as_str()
      .map(str::trim)
      .filter(|s| !s.is_empty())
      .unwrap_or("general");

    let cfg = self.active_config();

    let (provider_used, mut results) = match provider.as_str() {
      "tavily" => (
        "tavily".to_string(),
        self.search_tavily(&cfg, query, max_results, topic).await?,
      ),
      "bocha" => (
        "bocha".to_string(),
        self.search_bocha(&cfg, query, max_results).await?,
      ),
      "serper" => (
        "serper".to_string(),
        self.search_serper(&cfg, query, max_results).await?,
      ),
      "brave" => (
        "brave".to_string(),
        self.search_brave(&cfg, query, max_results).await?,
      ),
      "searxng" => (
        "searxng".to_string(),
        self.search_searxng(&cfg, query, max_results).await?,
      ),
      "duckduckgo" => (
        "duckduckgo".to_string(),
        self.search_duckduckgo(&cfg, query, max_results).await?,
      ),
      "wikipedia" => (
        "wikipedia".to_string(),
        self.search_public_json(&cfg, query, max_results).await?,
      ),
      "auto" => self.search_auto(&cfg, query, max_results, topic).await?,
      other => {
        return Err(ToolError::InvalidParams {
          message: format!(
            "Unsupported search provider '{other}'. Expected one of: auto, tavily, bocha, serper, brave, searxng, duckduckgo, wikipedia"
          ),
        });
      }
    };

    results.truncate(max_results);
    let markdown = format_results_markdown(query, &provider_used, &results);
    Ok(SearchResponse {
      provider: provider_used,
      query: query.to_string(),
      results,
      markdown,
    })
  }

  async fn search_auto(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
    topic: &str,
  ) -> Result<(String, Vec<SearchResultItem>), ToolError> {
    let mut errors = Vec::new();

    if cfg.tavily_api_key.is_some() {
      match self.search_tavily(cfg, query, max_results, topic).await {
        Ok(items) if !items.is_empty() => return Ok(("tavily".to_string(), items)),
        Ok(_) => errors.push("tavily returned 0 results".to_string()),
        Err(err) => errors.push(format!("tavily: {err}")),
      }
    }
    if cfg.bocha_api_key.is_some() {
      match self.search_bocha(cfg, query, max_results).await {
        Ok(items) if !items.is_empty() => return Ok(("bocha".to_string(), items)),
        Ok(_) => errors.push("bocha returned 0 results".to_string()),
        Err(err) => errors.push(format!("bocha: {err}")),
      }
    }
    if cfg.serper_api_key.is_some() {
      match self.search_serper(cfg, query, max_results).await {
        Ok(items) if !items.is_empty() => return Ok(("serper".to_string(), items)),
        Ok(_) => errors.push("serper returned 0 results".to_string()),
        Err(err) => errors.push(format!("serper: {err}")),
      }
    }
    if cfg.brave_api_key.is_some() {
      match self.search_brave(cfg, query, max_results).await {
        Ok(items) if !items.is_empty() => return Ok(("brave".to_string(), items)),
        Ok(_) => errors.push("brave returned 0 results".to_string()),
        Err(err) => errors.push(format!("brave: {err}")),
      }
    }
    if cfg.searxng_url.is_some() {
      match self.search_searxng(cfg, query, max_results).await {
        Ok(items) if !items.is_empty() => return Ok(("searxng".to_string(), items)),
        Ok(_) => errors.push("searxng returned 0 results".to_string()),
        Err(err) => errors.push(format!("searxng: {err}")),
      }
    }

    match self.search_duckduckgo(cfg, query, max_results).await {
      Ok(items) if !items.is_empty() => return Ok(("duckduckgo".to_string(), items)),
      Ok(_) => errors.push("duckduckgo returned 0 results".to_string()),
      Err(err) => errors.push(format!("duckduckgo: {err}")),
    }

    match self.search_public_json(cfg, query, max_results).await {
      Ok(items) if !items.is_empty() => Ok(("wikipedia".to_string(), items)),
      Ok(_) => Ok(("wikipedia".to_string(), Vec::new())),
      Err(err) => {
        errors.push(format!("wikipedia: {err}"));
        Err(ToolError::HttpError {
          message: format!("All web_search providers failed: {}", errors.join("; ")),
        })
      }
    }
  }

  async fn search_tavily(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
    topic: &str,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let key = cfg
      .tavily_api_key
      .as_deref()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "TAVILY_API_KEY is not configured".to_string(),
      })?;
    let endpoint = cfg.tavily_url.as_deref().unwrap_or(DEFAULT_TAVILY_URL);
    let payload = json!({
      "api_key": key,
      "query": query,
      "max_results": max_results,
      "topic": if topic == "news" { "news" } else { "general" },
      "include_answer": true
    });
    let resp = self
      .send_json_post(endpoint, &payload, &[("Content-Type", "application/json")])
      .await?;

    let mut items = Vec::new();
    if let Some(answer) = resp["answer"]
      .as_str()
      .map(str::trim)
      .filter(|s| !s.is_empty())
    {
      items.push(SearchResultItem {
        title: format!("Tavily Summary: {query}"),
        url: "https://tavily.com".to_string(),
        snippet: answer.to_string(),
        source: "tavily".to_string(),
      });
    }
    if let Some(arr) = resp["results"].as_array() {
      for entry in arr {
        let url = entry["url"].as_str().unwrap_or_default().trim();
        if url.is_empty() {
          continue;
        }
        items.push(SearchResultItem {
          title: entry["title"].as_str().unwrap_or(url).trim().to_string(),
          url: url.to_string(),
          snippet: clean_text(entry["content"].as_str().unwrap_or_default()),
          source: "tavily".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn search_bocha(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let key = cfg
      .bocha_api_key
      .as_deref()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "BOCHA_API_KEY is not configured".to_string(),
      })?;
    let endpoint = cfg.bocha_url.as_deref().unwrap_or(DEFAULT_BOCHA_URL);
    let auth = format!("Bearer {key}");
    let payload = json!({
      "query": query,
      "count": max_results,
      "summary": true
    });
    let resp = self
      .send_json_post(
        endpoint,
        &payload,
        &[
          ("Content-Type", "application/json"),
          ("Authorization", auth.as_str()),
        ],
      )
      .await?;

    let pages = resp["data"]["webPages"]["value"]
      .as_array()
      .or_else(|| resp["webPages"]["value"].as_array());
    let mut items = Vec::new();
    if let Some(arr) = pages {
      for entry in arr {
        let url = entry["url"].as_str().unwrap_or_default().trim();
        if url.is_empty() {
          continue;
        }
        let snippet_raw = entry["summary"]
          .as_str()
          .filter(|s| !s.trim().is_empty())
          .or_else(|| entry["snippet"].as_str())
          .unwrap_or_default();
        items.push(SearchResultItem {
          title: entry["name"].as_str().unwrap_or(url).trim().to_string(),
          url: url.to_string(),
          snippet: clean_text(snippet_raw),
          source: "bocha".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn search_serper(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let key = cfg
      .serper_api_key
      .as_deref()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "SERPER_API_KEY is not configured".to_string(),
      })?;
    let endpoint = cfg.serper_url.as_deref().unwrap_or(DEFAULT_SERPER_URL);
    let payload = json!({
      "q": query,
      "num": max_results
    });
    let resp = self
      .send_json_post(
        endpoint,
        &payload,
        &[("Content-Type", "application/json"), ("X-API-KEY", key)],
      )
      .await?;

    let mut items = Vec::new();
    if let Some(arr) = resp["organic"].as_array() {
      for entry in arr {
        let url = entry["link"].as_str().unwrap_or_default().trim();
        if url.is_empty() {
          continue;
        }
        items.push(SearchResultItem {
          title: entry["title"].as_str().unwrap_or(url).trim().to_string(),
          url: url.to_string(),
          snippet: clean_text(entry["snippet"].as_str().unwrap_or_default()),
          source: "serper".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn search_brave(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let key = cfg
      .brave_api_key
      .as_deref()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "BRAVE_API_KEY is not configured".to_string(),
      })?;
    let base = cfg.brave_url.as_deref().unwrap_or(DEFAULT_BRAVE_URL);
    let mut url = Url::parse(base).map_err(|err| ToolError::InvalidParams {
      message: format!("Invalid BRAVE_API_URL '{base}': {err}"),
    })?;
    url
      .query_pairs_mut()
      .append_pair("q", query)
      .append_pair("count", &max_results.to_string());

    let resp = self
      .send_json_get(
        url.as_str(),
        &[
          ("Accept", "application/json"),
          ("X-Subscription-Token", key),
        ],
      )
      .await?;

    let mut items = Vec::new();
    if let Some(arr) = resp["web"]["results"].as_array() {
      for entry in arr {
        let hit_url = entry["url"].as_str().unwrap_or_default().trim();
        if hit_url.is_empty() {
          continue;
        }
        items.push(SearchResultItem {
          title: entry["title"]
            .as_str()
            .unwrap_or(hit_url)
            .trim()
            .to_string(),
          url: hit_url.to_string(),
          snippet: clean_text(entry["description"].as_str().unwrap_or_default()),
          source: "brave".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn search_searxng(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let base = cfg
      .searxng_url
      .as_deref()
      .ok_or_else(|| ToolError::InvalidParams {
        message: "SEARXNG_URL is not configured".to_string(),
      })?;
    let search_base = format!("{}/search", base.trim_end_matches('/'));
    let mut url = Url::parse(&search_base).map_err(|err| ToolError::InvalidParams {
      message: format!("Invalid SEARXNG_URL '{base}': {err}"),
    })?;
    url
      .query_pairs_mut()
      .append_pair("q", query)
      .append_pair("format", "json");

    let resp = self
      .send_json_get(url.as_str(), &[("Accept", "application/json")])
      .await?;

    let mut items = Vec::new();
    if let Some(arr) = resp["results"].as_array() {
      for entry in arr.iter().take(max_results) {
        let hit_url = entry["url"].as_str().unwrap_or_default().trim();
        if hit_url.is_empty() {
          continue;
        }
        items.push(SearchResultItem {
          title: entry["title"]
            .as_str()
            .unwrap_or(hit_url)
            .trim()
            .to_string(),
          url: hit_url.to_string(),
          snippet: clean_text(entry["content"].as_str().unwrap_or_default()),
          source: "searxng".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn search_duckduckgo(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let endpoint = cfg
      .duckduckgo_url
      .as_deref()
      .unwrap_or(DEFAULT_DUCKDUCKGO_URL);
    let encoded_body = url::form_urlencoded::Serializer::new(String::new())
      .append_pair("q", query)
      .finish();
    let response = self
      .http
      .send(&json!({
        "url": endpoint,
        "method": "POST",
        "headers": {
          "Content-Type": "application/x-www-form-urlencoded",
          "Accept": "text/html"
        },
        "body": encoded_body
      }))
      .await?;

    if response.status.as_u16() != 200 {
      return Err(ToolError::HttpError {
        message: format!("DuckDuckGo returned HTTP {}", response.status),
      });
    }

    let items = parse_duckduckgo_html(&response.body, max_results);
    if items.is_empty() {
      return Err(ToolError::HttpError {
        message: "DuckDuckGo HTML response contained no organic results".to_string(),
      });
    }
    Ok(items)
  }

  /// Zero-key JSON fallback using public MediaWiki Search API (+ Hacker News Algolia for tech topics).
  async fn search_public_json(
    &self,
    cfg: &SearchConfig,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let has_cjk = query.chars().any(is_cjk);
    let mut items = Vec::new();
    let mut last_err = None;

    let wiki_order: [(&str, &str); 2] = if has_cjk {
      [
        (
          "zh",
          cfg
            .wikipedia_zh_url
            .as_deref()
            .unwrap_or(DEFAULT_WIKIPEDIA_ZH_URL),
        ),
        (
          "en",
          cfg
            .wikipedia_en_url
            .as_deref()
            .unwrap_or(DEFAULT_WIKIPEDIA_EN_URL),
        ),
      ]
    } else {
      [
        (
          "en",
          cfg
            .wikipedia_en_url
            .as_deref()
            .unwrap_or(DEFAULT_WIKIPEDIA_EN_URL),
        ),
        (
          "zh",
          cfg
            .wikipedia_zh_url
            .as_deref()
            .unwrap_or(DEFAULT_WIKIPEDIA_ZH_URL),
        ),
      ]
    };

    for (idx, (lang, base_url)) in wiki_order.iter().enumerate() {
      if idx > 0 && items.len() >= max_results {
        break;
      }
      let remaining = max_results.saturating_sub(items.len()).max(1);
      match self.query_mediawiki(lang, base_url, query, remaining).await {
        Ok(mut hits) => items.append(&mut hits),
        Err(err) => last_err = Some(err),
      }
      if !has_cjk && !items.is_empty() {
        break;
      }
    }

    if items.len() < max_results {
      let hn_base = cfg
        .hn_algolia_url
        .as_deref()
        .unwrap_or(DEFAULT_HN_ALGOLIA_URL);
      let remaining = max_results.saturating_sub(items.len());
      if let Ok(mut hn_hits) = self.query_hn_algolia(hn_base, query, remaining).await {
        items.append(&mut hn_hits);
      }
    }

    if items.is_empty()
      && let Some(err) = last_err
    {
      return Err(err);
    }
    Ok(items)
  }

  async fn query_mediawiki(
    &self,
    lang: &str,
    base_url: &str,
    query: &str,
    limit: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let mut url = Url::parse(base_url).map_err(|err| ToolError::InvalidParams {
      message: format!("Invalid Wikipedia URL '{base_url}': {err}"),
    })?;
    url
      .query_pairs_mut()
      .append_pair("action", "query")
      .append_pair("list", "search")
      .append_pair("srsearch", query)
      .append_pair("srlimit", &limit.to_string())
      .append_pair("utf8", "1")
      .append_pair("format", "json");

    let resp = self
      .send_json_get(url.as_str(), &[("Accept", "application/json")])
      .await?;

    let mut items = Vec::new();
    if let Some(arr) = resp["query"]["search"].as_array() {
      for entry in arr {
        let title = entry["title"].as_str().unwrap_or_default().trim();
        if title.is_empty() {
          continue;
        }
        let pageid = entry["pageid"].as_u64();
        let page_url = match pageid {
          Some(id) => format!("https://{lang}.wikipedia.org/?curid={id}"),
          None => format!(
            "https://{lang}.wikipedia.org/wiki/{}",
            title.replace(' ', "_")
          ),
        };
        items.push(SearchResultItem {
          title: title.to_string(),
          url: page_url,
          snippet: clean_html_fragment(entry["snippet"].as_str().unwrap_or_default()),
          source: format!("wikipedia-{lang}"),
        });
      }
    }
    Ok(items)
  }

  async fn query_hn_algolia(
    &self,
    base_url: &str,
    query: &str,
    limit: usize,
  ) -> Result<Vec<SearchResultItem>, ToolError> {
    let mut url = Url::parse(base_url).map_err(|err| ToolError::InvalidParams {
      message: format!("Invalid HN Algolia URL '{base_url}': {err}"),
    })?;
    url
      .query_pairs_mut()
      .append_pair("query", query)
      .append_pair("tags", "story")
      .append_pair("hitsPerPage", &limit.to_string());

    let resp = self
      .send_json_get(url.as_str(), &[("Accept", "application/json")])
      .await?;

    let mut items = Vec::new();
    if let Some(arr) = resp["hits"].as_array() {
      for entry in arr {
        let title = entry["title"].as_str().unwrap_or_default().trim();
        if title.is_empty() {
          continue;
        }
        let object_id = entry["objectID"].as_str().unwrap_or_default();
        let hit_url = entry["url"]
          .as_str()
          .filter(|s| !s.trim().is_empty())
          .map(str::to_string)
          .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={object_id}"));
        let points = entry["points"].as_u64().unwrap_or(0);
        let comments = entry["num_comments"].as_u64().unwrap_or(0);
        let story_text = clean_html_fragment(entry["story_text"].as_str().unwrap_or_default());
        let snippet = if story_text.is_empty() {
          format!("Hacker News discussion ({points} points, {comments} comments)")
        } else {
          story_text
        };
        items.push(SearchResultItem {
          title: title.to_string(),
          url: hit_url,
          snippet,
          source: "hackernews".to_string(),
        });
      }
    }
    Ok(items)
  }

  async fn send_json_post(
    &self,
    url: &str,
    payload: &Value,
    headers: &[(&str, &str)],
  ) -> Result<Value, ToolError> {
    let mut header_map = serde_json::Map::new();
    for (k, v) in headers {
      header_map.insert((*k).to_string(), Value::String((*v).to_string()));
    }
    let body_str = serde_json::to_string(payload).map_err(|err| ToolError::ExecutionFailed {
      message: format!("Failed to serialize search request JSON: {err}"),
    })?;
    let response = self
      .http
      .send(&json!({
        "url": url,
        "method": "POST",
        "headers": header_map,
        "body": body_str
      }))
      .await?;
    if !response.status.is_success() {
      let excerpt: String = response.body.chars().take(512).collect();
      return Err(ToolError::HttpError {
        message: format!(
          "Search endpoint '{url}' returned HTTP {}: {excerpt}",
          response.status
        ),
      });
    }
    serde_json::from_str(&response.body).map_err(|err| ToolError::HttpError {
      message: format!("Search endpoint '{url}' returned invalid JSON: {err}"),
    })
  }

  async fn send_json_get(&self, url: &str, headers: &[(&str, &str)]) -> Result<Value, ToolError> {
    let mut header_map = serde_json::Map::new();
    for (k, v) in headers {
      header_map.insert((*k).to_string(), Value::String((*v).to_string()));
    }
    let response = self
      .http
      .send(&json!({
        "url": url,
        "method": "GET",
        "headers": header_map
      }))
      .await?;
    if !response.status.is_success() {
      let excerpt: String = response.body.chars().take(512).collect();
      return Err(ToolError::HttpError {
        message: format!(
          "Search endpoint '{url}' returned HTTP {}: {excerpt}",
          response.status
        ),
      });
    }
    serde_json::from_str(&response.body).map_err(|err| ToolError::HttpError {
      message: format!("Search endpoint '{url}' returned invalid JSON: {err}"),
    })
  }
}

#[async_trait]
impl Tool for WebSearchTool {
  fn name(&self) -> &str {
    "web_search"
  }

  fn description(&self) -> &str {
    "Search the web for up-to-date facts, documentation, benchmarks, and references. \
     Returns structured search hits (title, URL, snippet) and a numbered Markdown summary."
  }

  fn parameters_schema(&self) -> Value {
    json!({
      "type": "object",
      "properties": {
        "query": {
          "type": "string",
          "description": "Search query string"
        },
        "provider": {
          "type": "string",
          "enum": ["auto", "tavily", "bocha", "serper", "brave", "searxng", "duckduckgo", "wikipedia"],
          "description": "Search backend (default: auto)"
        },
        "max_results": {
          "type": "integer",
          "minimum": 1,
          "maximum": 20,
          "description": "Maximum number of search results to return (default: 5)"
        },
        "topic": {
          "type": "string",
          "enum": ["general", "news"],
          "description": "Search topic category (default: general)"
        }
      },
      "required": ["query"]
    })
  }

  fn metadata(&self) -> ToolMetadata {
    ToolMetadata::builtin_named(self.name()).with_idempotency(ToolIdempotency::Idempotent)
  }

  async fn execute(&self, params: Value) -> Result<ToolOutput, ToolError> {
    let response = self.search(&params).await?;
    let serialized =
      serde_json::to_string_pretty(&response).map_err(|err| ToolError::ExecutionFailed {
        message: format!("Failed to serialize search response: {err}"),
      })?;
    Ok(ToolOutput::success(serialized))
  }
}

fn parse_max_results(value: &Value) -> usize {
  if let Some(n) = value.as_u64() {
    return usize::try_from(n).unwrap_or(5);
  }
  if let Some(s) = value.as_str()
    && let Ok(n) = s.trim().parse::<usize>()
  {
    return n;
  }
  5
}

fn is_cjk(ch: char) -> bool {
  matches!(
    ch as u32,
    0x4E00..=0x9FFF | 0x3400..=0x4DBF | 0x3040..=0x30FF | 0xAC00..=0xD7AF
  )
}

fn clean_text(raw: &str) -> String {
  raw.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clean_html_fragment(raw: &str) -> String {
  let mut stripped = String::with_capacity(raw.len());
  let mut in_tag = false;
  for ch in raw.chars() {
    match ch {
      '<' => in_tag = true,
      '>' => in_tag = false,
      _ if !in_tag => stripped.push(ch),
      _ => {}
    }
  }
  let decoded = stripped
    .replace("&amp;", "&")
    .replace("&lt;", "<")
    .replace("&gt;", ">")
    .replace("&quot;", "\"")
    .replace("&#x27;", "'")
    .replace("&#39;", "'")
    .replace("&nbsp;", " ");
  clean_text(&decoded)
}

fn parse_duckduckgo_html(html: &str, max_results: usize) -> Vec<SearchResultItem> {
  let mut results = Vec::new();
  let marker = "result__a";
  let mut rest = html;

  while let Some(idx) = rest.find(marker) {
    if results.len() >= max_results {
      break;
    }
    let after_marker = &rest[idx + marker.len()..];
    // Look backwards in a small window before `result__a` or forward before `>` for `href="..."`
    let window_start = idx.saturating_sub(256);
    let tag_head = &rest[window_start..idx + marker.len()];
    let Some(close_angle) = after_marker.find('>') else {
      break;
    };
    let tag_tail = &after_marker[..close_angle];
    let href = extract_href(tag_tail).or_else(|| extract_href(tag_head));
    let body_after_open = &after_marker[close_angle + 1..];
    let Some(end_anchor) = body_after_open.find("</a>") else {
      break;
    };
    let title = clean_html_fragment(&body_after_open[..end_anchor]);
    rest = &body_after_open[end_anchor + 4..];

    let Some(raw_url) = href else {
      continue;
    };
    let resolved_url = unwrap_ddg_redirect(&raw_url);
    if resolved_url.is_empty() || title.is_empty() {
      continue;
    }

    // Extract optional `result__snippet` before the next `result__a`.
    let next_hit = rest.find(marker).unwrap_or(rest.len());
    let snippet_region = &rest[..next_hit];
    let snippet = extract_ddg_snippet(snippet_region);

    results.push(SearchResultItem {
      title,
      url: resolved_url,
      snippet,
      source: "duckduckgo".to_string(),
    });
  }

  results
}

fn extract_href(fragment: &str) -> Option<String> {
  let pos = fragment.find("href=\"")?;
  let after = &fragment[pos + 6..];
  let end = after.find('"')?;
  Some(after[..end].replace("&amp;", "&"))
}

fn unwrap_ddg_redirect(raw_url: &str) -> String {
  let candidate = if let Some(rest) = raw_url.strip_prefix("//") {
    format!("https://{rest}")
  } else {
    raw_url.to_string()
  };
  if let Ok(parsed) = Url::parse(&candidate) {
    for (k, v) in parsed.query_pairs() {
      if k == "uddg" && !v.is_empty() {
        return v.into_owned();
      }
    }
    return parsed.to_string();
  }
  candidate
}

fn extract_ddg_snippet(region: &str) -> String {
  let Some(pos) = region.find("result__snippet") else {
    return String::new();
  };
  let after = &region[pos..];
  let Some(open_end) = after.find('>') else {
    return String::new();
  };
  let content = &after[open_end + 1..];
  let close = content
    .find("</a>")
    .or_else(|| content.find("</div>"))
    .unwrap_or(content.len());
  clean_html_fragment(&content[..close])
}

fn format_results_markdown(query: &str, provider: &str, results: &[SearchResultItem]) -> String {
  if results.is_empty() {
    return format!("### Web Search Results for \"{query}\" ({provider})\n\n_No results found._");
  }
  let mut lines = Vec::with_capacity(results.len() * 3 + 2);
  lines.push(format!(
    "### Web Search Results for \"{query}\" ({provider})\n"
  ));
  for (idx, item) in results.iter().enumerate() {
    let rank = idx + 1;
    if item.snippet.is_empty() {
      lines.push(format!("{rank}. **[{}]({})**", item.title, item.url));
    } else {
      lines.push(format!(
        "{rank}. **[{}]({})** — {}",
        item.title, item.url, item.snippet
      ));
    }
  }
  lines.join("\n")
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio::io::{AsyncReadExt, AsyncWriteExt};
  use tokio::net::TcpListener;

  async fn spawn_json_server(status: &str, body: &str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    let response = format!(
      "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
      body.len()
    );
    let handle = tokio::spawn(async move {
      if let Ok((mut socket, _)) = listener.accept().await {
        let mut buf = [0u8; 4096];
        let _ = socket.read(&mut buf).await;
        let _ = socket.write_all(response.as_bytes()).await;
      }
    });
    (format!("http://{addr}"), handle)
  }

  fn loopback_policy() -> Arc<SandboxPolicy> {
    Arc::new(SandboxPolicy {
      allow_loopback_network_access: true,
      ..SandboxPolicy::default()
    })
  }

  #[tokio::test]
  async fn tavily_provider_parses_answer_and_results() {
    let body = r#"{
      "answer": "Pingora is Cloudflare's Rust proxy framework.",
      "results": [
        {
          "title": "Cloudflare Pingora GitHub",
          "url": "https://github.com/cloudflare/pingora",
          "content": "Pingora is a library for building fast, reliable network systems in Rust."
        }
      ]
    }"#;
    let (url, server) = spawn_json_server("200 OK", body).await;

    let tool = WebSearchTool::new(loopback_policy())
      .expect("build tool")
      .with_no_proxy()
      .with_config(SearchConfig {
        tavily_api_key: Some("tvly-test".to_string()),
        tavily_url: Some(url),
        ..SearchConfig::default()
      });

    let resp = tool
      .search(&json!({ "query": "Rust Pingora", "provider": "auto", "max_results": 5 }))
      .await
      .expect("search succeeds");
    server.await.expect("server task");

    assert_eq!(resp.provider, "tavily");
    assert_eq!(resp.results.len(), 2);
    assert_eq!(resp.results[1].title, "Cloudflare Pingora GitHub");
    assert!(
      resp
        .markdown
        .contains("https://github.com/cloudflare/pingora")
    );
  }

  #[tokio::test]
  async fn auto_mode_falls_back_when_ddg_fails_to_wikipedia_json() {
    let (ddg_url, ddg_server) = spawn_json_server("202 Accepted", "{}").await;
    let wiki_body = r#"{
      "query": {
        "search": [
          {
            "title": "Rust (programming language)",
            "pageid": 29414838,
            "snippet": "<span class=\"searchmatch\">Rust</span> emphasizes memory safety &amp; concurrency."
          }
        ]
      }
    }"#;
    let (wiki_url, wiki_server) = spawn_json_server("200 OK", wiki_body).await;
    let hn_body = r#"{"hits": []}"#;
    let (hn_url, hn_server) = spawn_json_server("200 OK", hn_body).await;

    let tool = WebSearchTool::new(loopback_policy())
      .expect("build tool")
      .with_no_proxy()
      .with_config(SearchConfig {
        duckduckgo_url: Some(ddg_url),
        wikipedia_en_url: Some(wiki_url),
        hn_algolia_url: Some(hn_url),
        ..SearchConfig::default()
      });

    let resp = tool
      .search(&json!({ "query": "Rust language", "max_results": 3 }))
      .await
      .expect("auto fallback succeeds");
    ddg_server.await.expect("ddg");
    wiki_server.await.expect("wiki");
    hn_server.await.expect("hn");

    assert_eq!(resp.provider, "wikipedia");
    assert_eq!(resp.results.len(), 1);
    assert_eq!(
      resp.results[0].snippet,
      "Rust emphasizes memory safety & concurrency."
    );
  }

  #[test]
  fn parses_duckduckgo_lite_html_and_unwraps_uddg_redirect() {
    let html = r##"
      <div class="result">
        <a rel="nofollow" class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Ftokio.rs%2F&amp;rut=abc">Tokio - Async Rust</a>
        <a class="result__snippet" href="#">Tokio is an asynchronous <b>runtime</b> for Rust.</a>
      </div>
    "##;
    let hits = parse_duckduckgo_html(html, 5);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].title, "Tokio - Async Rust");
    assert_eq!(hits[0].url, "https://tokio.rs/");
    assert_eq!(
      hits[0].snippet,
      "Tokio is an asynchronous runtime for Rust."
    );
  }
}
