//! `weekly-digest` — A5 L1 scheduled digest pipeline.
//!
//! Executes a deterministic 4-node `yanshi_core::Flow`:
//! 1. `collect_and_search`: reads the `last_successful_run` watermark from
//!    `yanshi_memory::SqliteEntityFactStore`, loads Markdown/text notes modified
//!    after that watermark, chunks them via `ParagraphChunker`, and ranks relevant
//!    passages with `Bm25KnowledgeBackend`.
//! 2. `synthesize_digest`: asks `yanshi_llm::Yanshi` for a structured Markdown
//!    digest, renders an HTML email counterpart, and persists both to `outbox_dir`
//!    before network transmission so failed deliveries never lose synthesized copy.
//! 3. `deliver_email`: posts to SendGrid (`/v3/mail/send`) or Resend (`/emails`)
//!    with bounded exponential-backoff retries on `429` / `5xx` / transport errors
//!    (or skips network in `DryRun` mode).
//! 4. `commit_watermark`: runs strictly after `deliver_email` succeeds and advances
//!    `last_successful_run` in SQLite so failed runs automatically retry the same
//!    window on the next tick while succeeded runs never double-send.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::sync::Mutex;
use tracing::{info, warn};
use yanshi_core::async_node::{AsyncNode, AsyncNodeInputs, AsyncNodeResult};
use yanshi_core::error::YanshiError;
use yanshi_core::events::NoOpListener;
use yanshi_core::value::FlowValue;
use yanshi_core::{Flow, FlowExt, GraphNode, NodeType};
use yanshi_llm::Yanshi as LlmService;
use yanshi_memory::SqliteEntityFactStore;
use yanshi_memory::layer::{EntityFact, EntityFactStore};
use yanshi_rag::chunking::{ChunkingStrategy, ParagraphChunker};
use yanshi_rag::{Bm25KnowledgeBackend, KnowledgeBackend};

const CHUNK_SIZE: usize = 1200;
const CHUNK_OVERLAP: usize = 150;
const WATERMARK_FACT_ID: &str = "last_successful_run";

/// Configuration for one scheduled digest execution.
#[derive(Debug, Clone)]
pub struct DigestConfig {
  pub digest_id: String,
  pub corpus_dir: PathBuf,
  pub state_path: PathBuf,
  pub outbox_dir: PathBuf,
  pub query: String,
  pub top_k: usize,
  pub lookback_days: i64,
  pub model: String,
  pub subject_prefix: String,
  pub from_email: String,
  pub to_emails: Vec<String>,
  pub quiet_console: bool,
}

/// Outbound email transport backend.
#[derive(Debug, Clone)]
pub enum EmailTransport {
  /// Write rendered `.md` and `.html` to `outbox_dir` without calling an external API.
  DryRun,
  /// SendGrid v3 `/v3/mail/send` endpoint (`endpoint` defaults to `https://api.sendgrid.com`).
  SendGrid { endpoint: String, api_key: String },
  /// Resend `/emails` endpoint (`endpoint` defaults to `https://api.resend.com`).
  Resend { endpoint: String, api_key: String },
}

/// Retry-capable HTTPS email sender with injectable `reqwest::Client`.
#[derive(Clone)]
pub struct EmailDispatcher {
  client: reqwest::Client,
  transport: EmailTransport,
  max_retries: u32,
  retry_base_delay: Duration,
}

impl EmailDispatcher {
  /// Construct a dispatcher with a production HTTPS client.
  pub fn new(
    transport: EmailTransport,
    max_retries: u32,
    retry_base_delay: Duration,
  ) -> Result<Self> {
    let client = reqwest::Client::builder()
      .timeout(Duration::from_secs(30))
      .build()
      .context("failed to build reqwest HTTP client")?;
    Ok(Self::with_client(
      client,
      transport,
      max_retries,
      retry_base_delay,
    ))
  }

  /// Construct a dispatcher with a caller-supplied `reqwest::Client` (e.g.
  /// `.no_proxy()` in loopback integration tests).
  pub fn with_client(
    client: reqwest::Client,
    transport: EmailTransport,
    max_retries: u32,
    retry_base_delay: Duration,
  ) -> Self {
    Self {
      client,
      transport,
      max_retries,
      retry_base_delay,
    }
  }

  async fn send(
    &self,
    from: &str,
    to: &[String],
    subject: &str,
    markdown: &str,
    html: &str,
  ) -> Result<u32> {
    let (url, api_key, payload) = match &self.transport {
      EmailTransport::DryRun => return Ok(0),
      EmailTransport::SendGrid { endpoint, api_key } => {
        let to_objs: Vec<Value> = to.iter().map(|addr| json!({ "email": addr })).collect();
        let url = format!("{}/v3/mail/send", endpoint.trim_end_matches('/'));
        let body = json!({
          "personalizations": [{ "to": to_objs }],
          "from": { "email": from },
          "subject": subject,
          "content": [
            { "type": "text/plain", "value": markdown },
            { "type": "text/html", "value": html }
          ]
        });
        (url, api_key, body)
      }
      EmailTransport::Resend { endpoint, api_key } => {
        let url = format!("{}/emails", endpoint.trim_end_matches('/'));
        let body = json!({
          "from": from,
          "to": to,
          "subject": subject,
          "text": markdown,
          "html": html
        });
        (url, api_key, body)
      }
    };

    let mut attempt: u32 = 0;
    loop {
      attempt += 1;
      let send_res = self
        .client
        .post(&url)
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await;

      match send_res {
        Ok(resp) if resp.status().is_success() => return Ok(attempt),
        Ok(resp) => {
          let status = resp.status();
          let body_excerpt: String = resp
            .text()
            .await
            .unwrap_or_default()
            .chars()
            .take(240)
            .collect();
          let retryable = status.as_u16() == 429 || status.is_server_error();
          if retryable && attempt <= self.max_retries {
            let delay = self
              .retry_base_delay
              .saturating_mul(1u32 << (attempt - 1).min(5));
            warn!(
              attempt,
              max_retries = self.max_retries,
              status = %status,
              ?delay,
              "transient email API failure; retrying"
            );
            tokio::time::sleep(delay).await;
            continue;
          }
          anyhow::bail!(
            "email delivery failed after {attempt} attempt(s) with HTTP {status}: {body_excerpt}"
          );
        }
        Err(err) => {
          if attempt <= self.max_retries {
            let delay = self
              .retry_base_delay
              .saturating_mul(1u32 << (attempt - 1).min(5));
            warn!(
              attempt,
              max_retries = self.max_retries,
              error = %err,
              ?delay,
              "network error calling email API; retrying"
            );
            tokio::time::sleep(delay).await;
            continue;
          }
          return Err(err)
            .with_context(|| format!("email delivery transport error after {attempt} attempt(s)"));
        }
      }
    }
  }
}

/// Summary returned after running the 4-node digest flow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigestOutcome {
  pub new_docs: usize,
  pub matched_chunks: usize,
  pub delivered: bool,
  pub delivery_attempts: u32,
  pub watermark_before: Option<DateTime<Utc>>,
  pub watermark_after: Option<DateTime<Utc>>,
  pub markdown_path: Option<PathBuf>,
  pub html_path: Option<PathBuf>,
}

/// Read the persisted `last_successful_run` watermark for `digest_id`, if any.
pub async fn read_watermark(state_path: &Path, digest_id: &str) -> Result<Option<DateTime<Utc>>> {
  if !state_path.exists() {
    return Ok(None);
  }
  let store = SqliteEntityFactStore::open(state_path)
    .await
    .with_context(|| format!("open watermark store {}", state_path.display()))?;
  let entity_id = format!("digest:{digest_id}");
  let facts = store
    .get_facts(&entity_id, false)
    .await
    .with_context(|| format!("get_facts({entity_id})"))?;
  let Some(fact) = facts.into_iter().find(|f| f.fact_id == WATERMARK_FACT_ID) else {
    return Ok(None);
  };
  let Some(ts_str) = fact.value.get("timestamp").and_then(Value::as_str) else {
    return Ok(None);
  };
  let parsed = DateTime::parse_from_rfc3339(ts_str)
    .with_context(|| format!("invalid watermark timestamp '{ts_str}'"))?
    .with_timezone(&Utc);
  Ok(Some(parsed))
}

async fn write_watermark(
  state_path: &Path,
  digest_id: &str,
  watermark: DateTime<Utc>,
  new_docs: usize,
) -> Result<()> {
  if let Some(parent) = state_path.parent()
    && !parent.as_os_str().is_empty()
  {
    std::fs::create_dir_all(parent)
      .with_context(|| format!("create watermark parent dir {}", parent.display()))?;
  }
  let store = SqliteEntityFactStore::open(state_path)
    .await
    .with_context(|| format!("open watermark store {}", state_path.display()))?;
  let fact = EntityFact::observed(
    format!("digest:{digest_id}"),
    WATERMARK_FACT_ID.to_string(),
    "watermark".to_string(),
    json!({
      "timestamp": watermark.to_rfc3339(),
      "new_docs": new_docs,
      "recorded_at": Utc::now().to_rfc3339(),
    }),
  );
  store
    .record_fact(fact)
    .await
    .context("persist last_successful_run watermark")?;
  Ok(())
}

#[derive(Debug, Clone)]
struct CorpusPassages {
  watermark_before: Option<DateTime<Utc>>,
  candidate_watermark: DateTime<Utc>,
  new_docs: usize,
  passages: Vec<(String, String)>,
  markdown: String,
  html: String,
  subject: String,
  markdown_path: Option<PathBuf>,
  html_path: Option<PathBuf>,
  delivered: bool,
  delivery_attempts: u32,
}

struct CollectAndSearchNode {
  config: DigestConfig,
  bus: Arc<Mutex<CorpusPassages>>,
}

#[async_trait]
impl AsyncNode for CollectAndSearchNode {
  async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let watermark_before = read_watermark(&self.config.state_path, &self.config.digest_id)
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("read_watermark: {e:#}"),
      })?;
    let cutoff = watermark_before
      .unwrap_or_else(|| Utc::now() - ChronoDuration::days(self.config.lookback_days.max(1)));

    let docs = load_corpus_since(&self.config.corpus_dir, cutoff)
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("load_corpus_since: {e:#}"),
      })?;

    if docs.is_empty() {
      let mut guard = self.bus.lock().await;
      guard.watermark_before = watermark_before;
      guard.new_docs = 0;
      let mut out = HashMap::new();
      out.insert("new_docs".to_string(), FlowValue::Json(json!(0)));
      return Ok(out);
    }

    let latest_doc_ts = docs
      .iter()
      .map(|d| d.modified_at)
      .max()
      .unwrap_or_else(Utc::now);
    let candidate_watermark = latest_doc_ts.max(Utc::now());

    let chunker = ParagraphChunker::new(CHUNK_SIZE, CHUNK_OVERLAP);
    let mut indexed_chunks: Vec<(String, String)> = Vec::new();
    for doc in &docs {
      let pieces = chunker
        .chunk(&doc.content)
        .map_err(|e| YanshiError::AsyncExecutionError {
          message: format!("chunking '{}': {e}", doc.name),
        })?;
      for (idx, piece) in pieces.into_iter().enumerate() {
        let trimmed = piece.content.trim();
        if !trimmed.is_empty() {
          indexed_chunks.push((
            format!("{}#{idx}", doc.name),
            format!("[{}] {}", doc.name, trimmed),
          ));
        }
      }
    }

    let selected: Vec<(String, String)> =
      if self.config.query.trim().is_empty() || self.config.query.trim() == "*" {
        indexed_chunks
          .into_iter()
          .take(self.config.top_k.max(1))
          .collect()
      } else {
        let by_id: HashMap<String, String> = indexed_chunks.iter().cloned().collect();
        let backend = Bm25KnowledgeBackend::from_documents(indexed_chunks.clone())
          .with_name(format!("digest:{}", self.config.digest_id));
        let hits = backend
          .search(&self.config.query, self.config.top_k.max(1))
          .await
          .map_err(|e| YanshiError::AsyncExecutionError {
            message: format!("BM25 search failed: {e}"),
          })?;
        if hits.is_empty() {
          indexed_chunks
            .into_iter()
            .take(self.config.top_k.max(1))
            .collect()
        } else {
          hits
            .into_iter()
            .filter_map(|h| by_id.get(&h.id).map(|txt| (h.id, txt.clone())))
            .collect()
        }
      };

    let matched = selected.len();
    {
      let mut guard = self.bus.lock().await;
      guard.watermark_before = watermark_before;
      guard.candidate_watermark = candidate_watermark;
      guard.new_docs = docs.len();
      guard.passages = selected;
    }

    let mut out = HashMap::new();
    out.insert("new_docs".to_string(), FlowValue::Json(json!(docs.len())));
    out.insert(
      "matched_chunks".to_string(),
      FlowValue::Json(json!(matched)),
    );
    Ok(out)
  }
}

struct SynthesizeDigestNode {
  config: DigestConfig,
  bus: Arc<Mutex<CorpusPassages>>,
}

#[async_trait]
impl AsyncNode for SynthesizeDigestNode {
  async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let (new_docs, passages) = {
      let guard = self.bus.lock().await;
      (guard.new_docs, guard.passages.clone())
    };
    if new_docs == 0 || passages.is_empty() {
      return Ok(HashMap::new());
    }

    let joined_passages = passages
      .iter()
      .map(|(_, text)| text.as_str())
      .collect::<Vec<_>>()
      .join("\n\n---\n\n");

    let prompt = format!(
      "You are curating a concise weekly engineering & research digest.\n\
       Focus query: {}\n\
       Synthesize the {} passage(s) below into well-structured Markdown with:\n\
       1. `# Weekly Digest` title + 2-sentence executive overview\n\
       2. `## Highlights` (bulleted takeaways citing `[source#chunk]`)\n\
       3. `## Actionable Signals` (what to read or follow up on this week)\n\n\
       Passages:\n{}",
      if self.config.query.trim().is_empty() {
        "key updates and takeaways"
      } else {
        self.config.query.trim()
      },
      passages.len(),
      joined_passages
    );

    let markdown = LlmService::model(&self.config.model)
      .prompt(&prompt)
      .temperature(0.3)
      .execute()
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("LLM digest synthesis failed: {e}"),
      })?;

    let date_tag = Utc::now().format("%Y-%m-%d").to_string();
    let subject = format!(
      "{} — {} ({} new source(s))",
      self.config.subject_prefix, date_tag, new_docs
    );
    let html = markdown_to_simple_email_html(&subject, &markdown);

    tokio::fs::create_dir_all(&self.config.outbox_dir)
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!(
          "create outbox dir '{}': {e}",
          self.config.outbox_dir.display()
        ),
      })?;
    let stem = format!(
      "{}-{}",
      self.config.digest_id,
      Utc::now().format("%Y%m%dT%H%M%SZ")
    );
    let md_path = self.config.outbox_dir.join(format!("{stem}.md"));
    let html_path = self.config.outbox_dir.join(format!("{stem}.html"));

    tokio::fs::write(&md_path, &markdown)
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("write '{}': {e}", md_path.display()),
      })?;
    tokio::fs::write(&html_path, &html)
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("write '{}': {e}", html_path.display()),
      })?;

    {
      let mut guard = self.bus.lock().await;
      guard.subject = subject;
      guard.markdown = markdown;
      guard.html = html;
      guard.markdown_path = Some(md_path);
      guard.html_path = Some(html_path);
    }

    let mut out = HashMap::new();
    out.insert("synthesized".to_string(), FlowValue::Json(json!(true)));
    Ok(out)
  }
}

struct DeliverEmailNode {
  config: DigestConfig,
  dispatcher: EmailDispatcher,
  bus: Arc<Mutex<CorpusPassages>>,
}

#[async_trait]
impl AsyncNode for DeliverEmailNode {
  async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let (new_docs, subject, markdown, html) = {
      let guard = self.bus.lock().await;
      (
        guard.new_docs,
        guard.subject.clone(),
        guard.markdown.clone(),
        guard.html.clone(),
      )
    };
    if new_docs == 0 || markdown.is_empty() {
      return Ok(HashMap::new());
    }

    let attempts = self
      .dispatcher
      .send(
        &self.config.from_email,
        &self.config.to_emails,
        &subject,
        &markdown,
        &html,
      )
      .await
      .map_err(|e| YanshiError::AsyncExecutionError {
        message: format!("{e:#}"),
      })?;

    {
      let mut guard = self.bus.lock().await;
      guard.delivered = true;
      guard.delivery_attempts = attempts;
    }

    let mut out = HashMap::new();
    out.insert("attempts".to_string(), FlowValue::Json(json!(attempts)));
    Ok(out)
  }
}

struct CommitWatermarkNode {
  config: DigestConfig,
  bus: Arc<Mutex<CorpusPassages>>,
}

#[async_trait]
impl AsyncNode for CommitWatermarkNode {
  async fn execute(&self, _inputs: &AsyncNodeInputs) -> AsyncNodeResult {
    let (delivered, candidate_watermark, new_docs) = {
      let guard = self.bus.lock().await;
      (guard.delivered, guard.candidate_watermark, guard.new_docs)
    };
    if !delivered || new_docs == 0 {
      return Ok(HashMap::new());
    }
    write_watermark(
      &self.config.state_path,
      &self.config.digest_id,
      candidate_watermark,
      new_docs,
    )
    .await
    .map_err(|e| YanshiError::AsyncExecutionError {
      message: format!("write_watermark: {e:#}"),
    })?;
    let mut out = HashMap::new();
    out.insert(
      "watermark".to_string(),
      FlowValue::Json(json!(candidate_watermark.to_rfc3339())),
    );
    Ok(out)
  }
}

#[derive(Debug)]
struct CorpusDoc {
  name: String,
  content: String,
  modified_at: DateTime<Utc>,
}

async fn load_corpus_since(dir: &Path, cutoff: DateTime<Utc>) -> Result<Vec<CorpusDoc>> {
  if !dir.exists() {
    anyhow::bail!("corpus directory '{}' does not exist", dir.display());
  }
  let mut entries = tokio::fs::read_dir(dir)
    .await
    .with_context(|| format!("read corpus dir '{}'", dir.display()))?;
  let mut docs = Vec::new();
  while let Some(entry) = entries.next_entry().await? {
    let path = entry.path();
    if !path.is_file() {
      continue;
    }
    let ext = path
      .extension()
      .and_then(|e| e.to_str())
      .map(str::to_ascii_lowercase)
      .unwrap_or_default();
    if !matches!(ext.as_str(), "md" | "markdown" | "txt") {
      continue;
    }
    let meta = entry.metadata().await?;
    let modified_sys = meta.modified().context("file modified timestamp")?;
    let modified_at: DateTime<Utc> = modified_sys.into();
    if modified_at <= cutoff {
      continue;
    }
    let content = tokio::fs::read_to_string(&path)
      .await
      .with_context(|| format!("read '{}'", path.display()))?;
    if content.trim().is_empty() {
      continue;
    }
    let name = path
      .file_name()
      .map(|n| n.to_string_lossy().into_owned())
      .unwrap_or_else(|| "note.md".to_string());
    docs.push(CorpusDoc {
      name,
      content,
      modified_at,
    });
  }
  docs.sort_by(|a, b| {
    a.modified_at
      .cmp(&b.modified_at)
      .then_with(|| a.name.cmp(&b.name))
  });
  Ok(docs)
}

fn escape_html(raw: &str) -> String {
  raw
    .replace('&', "&amp;")
    .replace('<', "&lt;")
    .replace('>', "&gt;")
    .replace('"', "&quot;")
}

/// Minimal dependency-free Markdown-to-HTML wrapper suitable for email clients.
pub fn markdown_to_simple_email_html(subject: &str, markdown: &str) -> String {
  let mut body = String::new();
  let mut in_list = false;
  for line in markdown.lines() {
    let trimmed = line.trim();
    if trimmed.is_empty() {
      if in_list {
        body.push_str("</ul>\n");
        in_list = false;
      }
      continue;
    }
    if let Some(h1) = trimmed.strip_prefix("# ") {
      if in_list {
        body.push_str("</ul>\n");
        in_list = false;
      }
      body.push_str(&format!("<h1>{}</h1>\n", escape_html(h1)));
    } else if let Some(h2) = trimmed.strip_prefix("## ") {
      if in_list {
        body.push_str("</ul>\n");
        in_list = false;
      }
      body.push_str(&format!("<h2>{}</h2>\n", escape_html(h2)));
    } else if let Some(h3) = trimmed.strip_prefix("### ") {
      if in_list {
        body.push_str("</ul>\n");
        in_list = false;
      }
      body.push_str(&format!("<h3>{}</h3>\n", escape_html(h3)));
    } else if let Some(item) = trimmed
      .strip_prefix("- ")
      .or_else(|| trimmed.strip_prefix("* "))
    {
      if !in_list {
        body.push_str("<ul>\n");
        in_list = true;
      }
      body.push_str(&format!("  <li>{}</li>\n", escape_html(item)));
    } else {
      if in_list {
        body.push_str("</ul>\n");
        in_list = false;
      }
      body.push_str(&format!("<p>{}</p>\n", escape_html(trimmed)));
    }
  }
  if in_list {
    body.push_str("</ul>\n");
  }
  format!(
    "<!doctype html>\n<html lang=\"en\">\n<head>\n  <meta charset=\"utf-8\">\n  <title>{}</title>\n</head>\n<body style=\"font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;max-width:680px;margin:24px auto;line-height:1.6;color:#18212f;\">\n{}\n</body>\n</html>\n",
    escape_html(subject),
    body
  )
}

/// Run the 4-stage weekly digest `Flow` end-to-end.
pub async fn run_weekly_digest(
  config: DigestConfig,
  dispatcher: EmailDispatcher,
) -> Result<DigestOutcome> {
  LlmService::ensure_init()
    .await
    .context("failed to initialize Yanshi LLM registry")?;

  let bus = Arc::new(Mutex::new(CorpusPassages {
    watermark_before: None,
    candidate_watermark: Utc::now(),
    new_docs: 0,
    passages: Vec::new(),
    markdown: String::new(),
    html: String::new(),
    subject: String::new(),
    markdown_path: None,
    html_path: None,
    delivered: false,
    delivery_attempts: 0,
  }));

  let nodes = vec![
    GraphNode {
      id: "collect_and_search".to_string(),
      node_type: NodeType::Standard(Arc::new(CollectAndSearchNode {
        config: config.clone(),
        bus: Arc::clone(&bus),
      })),
      dependencies: vec![],
      input_mapping: None,
      run_if: None,
      initial_inputs: HashMap::new(),
    },
    GraphNode {
      id: "synthesize_digest".to_string(),
      node_type: NodeType::Standard(Arc::new(SynthesizeDigestNode {
        config: config.clone(),
        bus: Arc::clone(&bus),
      })),
      dependencies: vec!["collect_and_search".to_string()],
      input_mapping: None,
      run_if: None,
      initial_inputs: HashMap::new(),
    },
    GraphNode {
      id: "deliver_email".to_string(),
      node_type: NodeType::Standard(Arc::new(DeliverEmailNode {
        config: config.clone(),
        dispatcher,
        bus: Arc::clone(&bus),
      })),
      dependencies: vec!["synthesize_digest".to_string()],
      input_mapping: None,
      run_if: None,
      initial_inputs: HashMap::new(),
    },
    GraphNode {
      id: "commit_watermark".to_string(),
      node_type: NodeType::Standard(Arc::new(CommitWatermarkNode {
        config: config.clone(),
        bus: Arc::clone(&bus),
      })),
      dependencies: vec!["deliver_email".to_string()],
      input_mapping: None,
      run_if: None,
      initial_inputs: HashMap::new(),
    },
  ];

  let flow = if config.quiet_console {
    Flow::new(nodes).with_event_listener(Arc::new(NoOpListener))
  } else {
    Flow::new(nodes).with_event_listener(Arc::new(yanshi_core::ConsoleListener))
  };

  let state = flow.run().await.context("Flow execution aborted")?;
  for node_id in [
    "collect_and_search",
    "synthesize_digest",
    "deliver_email",
    "commit_watermark",
  ] {
    if let Some(Err(err)) = state.get(node_id) {
      anyhow::bail!("node '{node_id}' failed: {err}");
    }
  }

  let snapshot = bus.lock().await.clone();
  let watermark_after = read_watermark(&config.state_path, &config.digest_id).await?;
  info!(
    digest_id = %config.digest_id,
    new_docs = snapshot.new_docs,
    matched_chunks = snapshot.passages.len(),
    delivered = snapshot.delivered,
    "weekly digest run completed"
  );

  Ok(DigestOutcome {
    new_docs: snapshot.new_docs,
    matched_chunks: snapshot.passages.len(),
    delivered: snapshot.delivered,
    delivery_attempts: snapshot.delivery_attempts,
    watermark_before: snapshot.watermark_before,
    watermark_after,
    markdown_path: snapshot.markdown_path,
    html_path: snapshot.html_path,
  })
}
