//! First-class CLI client for Content Studio notebooks (`/v1/notebooks/*`).

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result, bail};
use base64::Engine as _;
use clap::{Args, Subcommand};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::commands::parse_input_flag_tokens;
use crate::server_client::{
  ServerClient, resolve_auth_token, resolve_server_url, resolve_tenant_id,
};

const DEFAULT_NOTEBOOK_SERVER_URL: &str = "http://127.0.0.1:8080";

#[derive(Args)]
pub struct NotebookArgs {
  #[command(flatten)]
  pub target: NotebookServerOpts,

  #[command(subcommand)]
  pub command: NotebookCommands,
}

#[derive(Args, Clone)]
pub struct NotebookServerOpts {
  /// Gateway or Desktop Studio base URL (defaults to YANSHI_SERVER_URL or http://127.0.0.1:8080)
  #[arg(long, global = true)]
  pub server: Option<String>,

  /// Bearer token for authenticated gateways (falls back to YANSHI_API_TOKEN)
  #[arg(long = "auth-token", visible_alias = "token", global = true)]
  pub auth_token: Option<String>,

  /// Tenant scope header (`X-Yanshi-Tenant`; falls back to YANSHI_TENANT or `default`)
  #[arg(long, global = true)]
  pub tenant: Option<String>,

  /// Output format: `text` (human-readable) or `json` (machine-readable JSON on stdout)
  #[arg(short = 'o', long = "output", default_value = "text", value_parser = ["text", "json"], global = true)]
  pub output: String,
}

#[derive(Subcommand)]
pub enum NotebookCommands {
  /// Create a new notebook
  Create {
    /// Notebook title
    title: String,
  },
  /// List notebooks in the active tenant
  List,
  /// Show notebook metadata and its ingested sources
  Show {
    /// Notebook UUID
    notebook_id: String,
  },
  /// Rename an existing notebook
  Rename {
    /// Notebook UUID
    notebook_id: String,
    /// New notebook title
    title: String,
  },
  /// Delete a notebook and all of its sources, chunks, and artifact links
  Delete {
    /// Notebook UUID
    notebook_id: String,
  },
  /// Export a portable JSON bundle (and optional Markdown research dossier) of a notebook
  Export {
    /// Notebook UUID
    notebook_id: String,
    /// Optional path to write the portable JSON bundle (prints to stdout when omitted)
    #[arg(
      short = 'f',
      long = "out",
      visible_alias = "bundle-out",
      value_name = "PATH"
    )]
    out: Option<PathBuf>,
    /// Optional path to write the cited Markdown research dossier (`markdown_brief`)
    #[arg(long, value_name = "PATH")]
    brief_out: Option<PathBuf>,
  },
  /// Import a portable notebook JSON bundle (`POST /v1/notebooks/import`)
  Import {
    /// Path to a `.json` bundle produced by `yanshi notebook export` or `/ui/notebooks`
    #[arg(value_name = "BUNDLE_JSON")]
    file: PathBuf,
    /// Optional title override for the imported notebook
    #[arg(long)]
    title: Option<String>,
  },
  /// Add a URL, pasted text, local document (.pdf/.docx/.xlsx/.pptx/.csv/.tsv/.json/.md/.txt/.html), or audio recording
  AddSource {
    /// Notebook UUID
    notebook_id: String,
    /// Remote web/document URL (`http://` or `https://`)
    #[arg(long, conflicts_with_all = ["text", "file", "recording"])]
    url: Option<String>,
    /// Inline text note
    #[arg(long, conflicts_with_all = ["url", "file", "recording"])]
    text: Option<String>,
    /// Local document file path (`.pdf`, `.docx`, `.xlsx`, `.pptx`, `.csv`, `.tsv`, `.json`, `.md`, `.txt`, `.html`)
    #[arg(long, value_name = "PATH", conflicts_with_all = ["url", "text", "recording"])]
    file: Option<PathBuf>,
    /// Local audio recording file path (`.mp3`, `.wav`, `.m4a`, `.ogg`, `.flac`, `.webm`, `.aac`)
    #[arg(long, value_name = "PATH", conflicts_with_all = ["url", "text", "file"])]
    recording: Option<PathBuf>,
    /// Optional display title override
    #[arg(long)]
    title: Option<String>,
    /// ASR model registry name when uploading `--recording` (falls back to server `YANSHI_ASR_MODEL`)
    #[arg(short = 'm', long)]
    model: Option<String>,
    /// Optional BCP-47 / ISO-639 language hint for `--recording`
    #[arg(long)]
    language: Option<String>,
    /// Poll until background ingestion reaches `ready` (or fails)
    #[arg(long)]
    wait: bool,
    /// Max seconds to wait when `--wait` is enabled
    #[arg(long, default_value_t = 120)]
    timeout_secs: u64,
  },
  /// Remove a single source (and its indexed chunks) from a notebook
  RemoveSource {
    /// Notebook UUID
    notebook_id: String,
    /// Source UUID
    source_id: String,
  },
  /// Inspect a single cited chunk by `chunk_id` (e.g. `<source_uuid>:0`)
  Chunk {
    /// Notebook UUID
    notebook_id: String,
    /// Chunk ID (`<source_uuid>:<ordinal>`)
    chunk_id: String,
  },
  /// List all ordered passages/chunks extracted from a single source (`GET /v1/notebooks/{id}/sources/{source_id}/chunks`)
  Chunks {
    /// Notebook UUID
    notebook_id: String,
    /// Source UUID
    source_id: String,
  },
  /// Run BM25 passage search across ready sources in a notebook
  Search {
    /// Notebook UUID
    notebook_id: String,
    /// Search query
    query: String,
    /// Max hits to return (1..=50)
    #[arg(long, default_value_t = 5)]
    top_k: usize,
    /// Restrict search to specific source UUIDs (repeatable)
    #[arg(long = "source-id", value_name = "UUID")]
    source_ids: Vec<String>,
  },
  /// Ask a source-grounded question and receive an answer with verified chunk citations
  Ask {
    /// Notebook UUID
    notebook_id: String,
    /// Question to answer from the notebook's sources
    question: String,
    /// Chat model override (falls back to server `YANSHI_GENERATE_MODEL`)
    #[arg(short = 'm', long)]
    model: Option<String>,
    /// Optional cap on the number of relevant passages passed to the model
    #[arg(long)]
    top_k: Option<usize>,
    /// Restrict grounding to specific source UUIDs (repeatable)
    #[arg(long = "source-id", value_name = "UUID")]
    source_ids: Vec<String>,
    /// Optional JSON file persisting multi-turn `{question, answer}` history across CLI calls
    #[arg(long, value_name = "PATH")]
    history_file: Option<PathBuf>,
  },
  /// Launch one of the 12 Studio deliverable generators against notebook sources
  Generate {
    /// Notebook UUID
    notebook_id: String,
    /// Deliverable kind (`report`, `mindmap`, `slides`, `data_table`, `flashcards`, `quiz`, `infographic`, `audio_overview`, `video_overview`, `illustrated_slides`, `tech_slides_video`, `debate_podcast_show`)
    kind: String,
    /// Generator LLM model override (falls back to server `YANSHI_GENERATE_MODEL`)
    #[arg(short = 'm', long)]
    model: Option<String>,
    /// Generator parameter as `--input KEY VALUE` or `--input KEY=VALUE` (repeatable)
    #[arg(short, long, num_args = 1..=2, value_names = ["KEY[=VALUE]", "VALUE"])]
    input: Vec<String>,
    /// Restrict generation to specific source UUIDs (repeatable)
    #[arg(long = "source-id", value_name = "UUID")]
    source_ids: Vec<String>,
    /// Optional USD spend cap for this generation run (`> 0`)
    #[arg(long, value_name = "USD")]
    cost_limit_usd: Option<f64>,
    /// Poll `/v1/runs/{run_id}` until the generator run finishes
    #[arg(long)]
    wait: bool,
    /// Download generated deliverable files into `<DIR>/<node_id>/<name>` and write `artifacts.json` (implies `--wait`)
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// Max seconds to wait when `--wait` or `--output-dir` is enabled
    #[arg(long, default_value_t = 600)]
    timeout_secs: u64,
  },
  /// Plan a tailored multi-deliverable Studio batch from a natural-language goal (`POST /v1/notebooks/{id}/plan`)
  Plan {
    /// Notebook UUID
    notebook_id: String,
    /// Natural-language content production goal (e.g. "Prepare flashcards and a quiz on tidal energetics")
    goal: String,
    /// Planner (and optional batch) LLM model override (falls back to server `YANSHI_GENERATE_MODEL`)
    #[arg(short = 'm', long)]
    model: Option<String>,
    /// Maximum deliverables to include in the plan (`1..=6`)
    #[arg(long, default_value_t = 4)]
    max_items: usize,
    /// Restrict planning (and `--execute`) to specific source UUIDs (repeatable)
    #[arg(long = "source-id", value_name = "UUID")]
    source_ids: Vec<String>,
    /// Immediately submit the planned deliverables to `POST /v1/notebooks/{id}/generate-batch`
    #[arg(long)]
    execute: bool,
    /// Optional aggregate USD spend cap when `--execute` is enabled (`> 0`)
    #[arg(long, value_name = "USD")]
    cost_limit_usd: Option<f64>,
    /// Poll all dispatched runs until completion (implies `--execute`)
    #[arg(long)]
    wait: bool,
    /// Download generated deliverables into `<DIR>/<kind>/<node_id>/<name>` (implies `--execute` and `--wait`)
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// Max seconds to wait when `--wait` or `--output-dir` is enabled
    #[arg(long, default_value_t = 600)]
    timeout_secs: u64,
  },
  /// Launch multiple Studio generators atomically (`POST /v1/notebooks/{id}/generate-batch`)
  GenerateBatch {
    /// Notebook UUID
    notebook_id: String,
    /// Comma-separated or repeated deliverable kinds (e.g. `--kinds report,flashcards,quiz`)
    #[arg(
      long = "kinds",
      visible_alias = "kind",
      value_delimiter = ',',
      required = true
    )]
    kinds: Vec<String>,
    /// Optional shared `focus` parameter applied to every selected generator that declares `focus`
    #[arg(long)]
    focus: Option<String>,
    /// Generator LLM model override (falls back to server `YANSHI_GENERATE_MODEL`)
    #[arg(short = 'm', long)]
    model: Option<String>,
    /// Restrict generation to specific source UUIDs (repeatable)
    #[arg(long = "source-id", value_name = "UUID")]
    source_ids: Vec<String>,
    /// Optional aggregate USD spend cap split evenly across the batch (`> 0`)
    #[arg(long, value_name = "USD")]
    cost_limit_usd: Option<f64>,
    /// Poll all dispatched runs until completion
    #[arg(long)]
    wait: bool,
    /// Download generated deliverables into `<DIR>/<kind>/<node_id>/<name>` and write `batch_artifacts.json` (implies `--wait`)
    #[arg(long, value_name = "DIR")]
    output_dir: Option<PathBuf>,
    /// Max seconds to wait when `--wait` or `--output-dir` is enabled
    #[arg(long, default_value_t = 600)]
    timeout_secs: u64,
  },
  /// List deliverables and artifacts attached to a notebook
  Artifacts {
    /// Notebook UUID
    notebook_id: String,
  },
  /// Detach an artifact from a notebook (`DELETE /v1/notebooks/{id}/artifacts/{artifact_id}`)
  RemoveArtifact {
    /// Notebook UUID
    notebook_id: String,
    /// Artifact UUID
    artifact_id: String,
  },
  /// Download all deliverables attached to a notebook as a single `.zip` bundle (with `BRIEFING.md`)
  DownloadArtifacts {
    /// Notebook UUID
    notebook_id: String,
    /// Destination `.zip` file path (defaults to `yanshi-notebook-<short_id>-artifacts.zip`)
    #[arg(short = 'f', long = "out", value_name = "ZIP_PATH")]
    out: Option<PathBuf>,
  },
}

fn build_notebook_client(opts: &NotebookServerOpts) -> Result<ServerClient> {
  let url = resolve_server_url(opts.server.as_deref())
    .unwrap_or_else(|| DEFAULT_NOTEBOOK_SERVER_URL.to_string());
  let token = resolve_auth_token(opts.auth_token.as_deref());
  let tenant = resolve_tenant_id(opts.tenant.as_deref());
  ServerClient::new(url, token, tenant)
}

fn parse_typed_param_value(raw: &str) -> Value {
  let trimmed = raw.trim();
  if trimmed.eq_ignore_ascii_case("true") {
    return Value::Bool(true);
  }
  if trimmed.eq_ignore_ascii_case("false") {
    return Value::Bool(false);
  }
  if let Ok(int_val) = trimmed.parse::<i64>() {
    return Value::Number(int_val.into());
  }
  if let Ok(float_val) = trimmed.parse::<f64>()
    && let Some(num) = serde_json::Number::from_f64(float_val)
  {
    return Value::Number(num);
  }
  Value::String(raw.to_string())
}

fn sanitize_path_segment(raw: &str, fallback: &str) -> String {
  let candidate = Path::new(raw)
    .file_name()
    .and_then(|n| n.to_str())
    .map(str::trim)
    .filter(|s| !s.is_empty() && *s != "." && *s != "..")
    .unwrap_or(fallback);
  candidate.replace(['/', '\\'], "_")
}

pub async fn dispatch(args: NotebookArgs) -> Result<()> {
  let client = build_notebook_client(&args.target)?;
  let json_mode = args.target.output == "json";

  match args.command {
    NotebookCommands::Create { title } => {
      let created = client.create_notebook(&title).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&created)?);
      } else {
        println!(
          "Created notebook {}  {}",
          created["id"].as_str().unwrap_or("?"),
          created["title"].as_str().unwrap_or(&title)
        );
      }
    }
    NotebookCommands::List => {
      let env = client.list_notebooks().await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&env)?);
      } else {
        let items = env["notebooks"].as_array().cloned().unwrap_or_default();
        if items.is_empty() {
          println!("No notebooks found.");
        } else {
          for nb in items {
            println!(
              "{}  {}  ({})",
              nb["id"].as_str().unwrap_or("?"),
              nb["title"].as_str().unwrap_or(""),
              nb["created_at"].as_str().unwrap_or("")
            );
          }
        }
      }
    }
    NotebookCommands::Show { notebook_id } => {
      let detail = client.get_notebook(&notebook_id).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&detail)?);
      } else {
        let nb = &detail["notebook"];
        println!(
          "Notebook: {} ({})",
          nb["title"].as_str().unwrap_or(""),
          nb["id"].as_str().unwrap_or(&notebook_id)
        );
        let sources = detail["sources"].as_array().cloned().unwrap_or_default();
        println!("Sources ({}):", sources.len());
        for src in sources {
          let err_suffix = src["error"]
            .as_str()
            .filter(|e| !e.is_empty())
            .map(|e| format!(" — error: {e}"))
            .unwrap_or_default();
          println!(
            "  - {}  [{}] ({}) {}{}",
            src["id"].as_str().unwrap_or("?"),
            src["status"].as_str().unwrap_or("?"),
            src["kind"].as_str().unwrap_or("?"),
            src["title"].as_str().unwrap_or(""),
            err_suffix
          );
        }
      }
    }
    NotebookCommands::Rename { notebook_id, title } => {
      let updated = client.rename_notebook(&notebook_id, &title).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&updated)?);
      } else {
        println!(
          "Renamed notebook {} -> {}",
          updated["id"].as_str().unwrap_or(&notebook_id),
          updated["title"].as_str().unwrap_or(&title)
        );
      }
    }
    NotebookCommands::Delete { notebook_id } => {
      client.delete_notebook(&notebook_id).await?;
      if json_mode {
        println!(
          "{}",
          serde_json::to_string_pretty(&json!({
            "deleted": true,
            "notebook_id": notebook_id,
          }))?
        );
      } else {
        println!("Deleted notebook {notebook_id}");
      }
    }
    NotebookCommands::Export {
      notebook_id,
      out,
      brief_out,
    } => {
      let bundle = client.export_notebook(&notebook_id).await?;
      if let Some(path) = &out {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
          std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(&bundle)?)
          .with_context(|| format!("failed to write bundle {}", path.display()))?;
      }
      if let Some(path) = &brief_out {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
          std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let brief = bundle["markdown_brief"].as_str().unwrap_or("");
        std::fs::write(path, brief.as_bytes())
          .with_context(|| format!("failed to write markdown brief {}", path.display()))?;
      }
      if json_mode {
        if out.is_some() || brief_out.is_some() {
          println!(
            "{}",
            serde_json::to_string_pretty(&json!({
              "notebook_id": notebook_id,
              "version": bundle["version"],
              "bundle_path": out.as_ref().map(|p| p.display().to_string()),
              "brief_path": brief_out.as_ref().map(|p| p.display().to_string()),
              "sources": bundle["sources"].as_array().map(Vec::len).unwrap_or(0),
              "deliverables": bundle["deliverables"].as_array().map(Vec::len).unwrap_or(0),
            }))?
          );
        } else {
          println!("{}", serde_json::to_string_pretty(&bundle)?);
        }
      } else if out.is_none() && brief_out.is_none() {
        println!("{}", bundle["markdown_brief"].as_str().unwrap_or(""));
      } else {
        if let Some(path) = &out {
          println!("Exported notebook bundle -> {}", path.display());
        }
        if let Some(path) = &brief_out {
          println!("Exported markdown brief -> {}", path.display());
        }
      }
    }
    NotebookCommands::Import { file, title } => {
      let raw = std::fs::read_to_string(&file)
        .with_context(|| format!("failed to read notebook bundle {}", file.display()))?;
      let parsed: Value = serde_json::from_str(&raw)
        .with_context(|| format!("{} is not valid JSON", file.display()))?;
      let body = match title
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
      {
        Some(override_title) => json!({
          "bundle": parsed,
          "title": override_title,
        }),
        None => parsed,
      };
      let imported = client.import_notebook(&body).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&imported)?);
      } else {
        println!(
          "Imported notebook {}  {} ({} sources, {} chunks)",
          imported["notebook"]["id"].as_str().unwrap_or("?"),
          imported["notebook"]["title"].as_str().unwrap_or(""),
          imported["imported_sources"].as_u64().unwrap_or(0),
          imported["imported_chunks"].as_u64().unwrap_or(0)
        );
      }
    }
    NotebookCommands::AddSource {
      notebook_id,
      url,
      text,
      file,
      recording,
      title,
      model,
      language,
      wait,
      timeout_secs,
    } => {
      let payload = build_add_source_payload(url, text, file, recording, title, model, language)?;
      let mut source = client.add_notebook_source(&notebook_id, &payload).await?;
      let source_id = source["id"]
        .as_str()
        .context("server response missing source id")?
        .to_string();

      if wait {
        let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
        loop {
          let detail = client.get_notebook(&notebook_id).await?;
          if let Some(found) = detail["sources"]
            .as_array()
            .and_then(|arr| arr.iter().find(|s| s["id"].as_str() == Some(&source_id)))
          {
            source = found.clone();
            match source["status"].as_str() {
              Some("ready") => break,
              Some("failed") => {
                let reason = source["error"]
                  .as_str()
                  .unwrap_or("unknown ingestion error");
                bail!("source {source_id} failed to ingest: {reason}");
              }
              _ => {}
            }
          }
          if Instant::now() >= deadline {
            bail!("timed out after {timeout_secs}s waiting for source {source_id} to ingest");
          }
          tokio::time::sleep(Duration::from_millis(150)).await;
        }
      }

      if json_mode {
        println!("{}", serde_json::to_string_pretty(&source)?);
      } else {
        println!(
          "Source {}  [{}]  {}",
          source_id,
          source["status"].as_str().unwrap_or("pending"),
          source["title"].as_str().unwrap_or("")
        );
      }
    }
    NotebookCommands::RemoveSource {
      notebook_id,
      source_id,
    } => {
      client
        .delete_notebook_source(&notebook_id, &source_id)
        .await?;
      if json_mode {
        println!(
          "{}",
          serde_json::to_string_pretty(&json!({
            "deleted": true,
            "notebook_id": notebook_id,
            "source_id": source_id,
          }))?
        );
      } else {
        println!("Removed source {source_id} from notebook {notebook_id}");
      }
    }
    NotebookCommands::Chunk {
      notebook_id,
      chunk_id,
    } => {
      let chunk = client.get_notebook_chunk(&notebook_id, &chunk_id).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&chunk)?);
      } else {
        println!(
          "[{}] (source {} · ordinal {})",
          chunk["chunk_id"].as_str().unwrap_or(&chunk_id),
          chunk["source_id"].as_str().unwrap_or("?"),
          chunk["ordinal"].as_i64().unwrap_or(0)
        );
        println!("{}", chunk["content"].as_str().unwrap_or(""));
      }
    }
    NotebookCommands::Chunks {
      notebook_id,
      source_id,
    } => {
      let doc = client.list_source_chunks(&notebook_id, &source_id).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&doc)?);
      } else {
        let title = doc["source"]["title"].as_str().unwrap_or(&source_id);
        let chunks = doc["chunks"].as_array().cloned().unwrap_or_default();
        println!(
          "Source '{title}' ({source_id}) — {} chunk(s):",
          chunks.len()
        );
        for ch in chunks {
          println!(
            "  #{} [{}] {}",
            ch["ordinal"].as_i64().unwrap_or(0),
            ch["chunk_id"].as_str().unwrap_or("?"),
            ch["content"].as_str().unwrap_or("")
          );
        }
      }
    }
    NotebookCommands::Search {
      notebook_id,
      query,
      top_k,
      source_ids,
    } => {
      let res = client
        .search_notebook(&notebook_id, &query, Some(top_k), &source_ids)
        .await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&res)?);
      } else {
        let hits = res["chunks"].as_array().cloned().unwrap_or_default();
        if hits.is_empty() {
          println!("No matching chunks.");
        } else {
          for (idx, hit) in hits.iter().enumerate() {
            println!(
              "{}. [{}] score={:.2} (source {})",
              idx + 1,
              hit["chunk_id"].as_str().unwrap_or("?"),
              hit["score"].as_f64().unwrap_or(0.0),
              hit["source_id"].as_str().unwrap_or("?")
            );
            println!("   {}", hit["content"].as_str().unwrap_or(""));
          }
        }
      }
    }
    NotebookCommands::Ask {
      notebook_id,
      question,
      model,
      top_k,
      source_ids,
      history_file,
    } => {
      let mut prior_turns: Vec<Value> = if let Some(path) = &history_file
        && path.exists()
      {
        let raw = std::fs::read_to_string(path)
          .with_context(|| format!("failed to read history file {}", path.display()))?;
        serde_json::from_str::<Vec<Value>>(&raw)
          .with_context(|| format!("history file {} must be a JSON array", path.display()))?
      } else {
        Vec::new()
      };

      let mut body = serde_json::Map::new();
      body.insert("question".to_string(), Value::String(question.clone()));
      if !prior_turns.is_empty() {
        body.insert("history".to_string(), Value::Array(prior_turns.clone()));
      }
      if let Some(m) = model.filter(|s| !s.trim().is_empty()) {
        body.insert("model".to_string(), Value::String(m));
      }
      if let Some(k) = top_k {
        body.insert("top_k".to_string(), Value::Number(k.into()));
      }
      if !source_ids.is_empty() {
        body.insert(
          "source_ids".to_string(),
          Value::Array(source_ids.into_iter().map(Value::String).collect()),
        );
      }
      let ans = client
        .ask_notebook(&notebook_id, &Value::Object(body))
        .await?;

      if let Some(path) = &history_file {
        prior_turns.push(json!({
          "question": question,
          "answer": ans["answer"].as_str().unwrap_or(""),
        }));
        let skip = prior_turns.len().saturating_sub(8);
        let bounded: Vec<Value> = prior_turns.into_iter().skip(skip).collect();
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
          std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(&bounded)?)
          .with_context(|| format!("failed to write history file {}", path.display()))?;
      }

      if json_mode {
        println!("{}", serde_json::to_string_pretty(&ans)?);
      } else {
        println!("{}\n", ans["answer"].as_str().unwrap_or(""));
        if let Some(citations) = ans["citations"].as_array()
          && !citations.is_empty()
        {
          println!("Citations ({}):", citations.len());
          for (idx, cit) in citations.iter().enumerate() {
            println!(
              "  [{}] {} — \"{}\"",
              idx + 1,
              cit["chunk_id"].as_str().unwrap_or("?"),
              cit["quote"].as_str().unwrap_or("")
            );
          }
        }
      }
    }
    NotebookCommands::Generate {
      notebook_id,
      kind,
      model,
      input,
      source_ids,
      cost_limit_usd,
      wait,
      output_dir,
      timeout_secs,
    } => {
      if let Some(limit) = cost_limit_usd {
        yanshi_config::config::v2::validate_cost_limit(limit).map_err(anyhow::Error::msg)?;
      }
      let pairs = parse_input_flag_tokens(&input)?;
      let mut params = serde_json::Map::new();
      for (k, v) in pairs {
        params.insert(k, parse_typed_param_value(&v));
      }
      let mut req_body = serde_json::Map::new();
      req_body.insert("kind".to_string(), Value::String(kind.clone()));
      req_body.insert("params".to_string(), Value::Object(params));
      if let Some(m) = model.filter(|s| !s.trim().is_empty()) {
        req_body.insert("model".to_string(), Value::String(m));
      }
      if !source_ids.is_empty() {
        req_body.insert(
          "source_ids".to_string(),
          Value::Array(source_ids.into_iter().map(Value::String).collect()),
        );
      }
      if let Some(limit) = cost_limit_usd
        && let Some(num) = serde_json::Number::from_f64(limit)
      {
        req_body.insert("cost_limit_usd".to_string(), Value::Number(num));
      }

      let gen_resp = client
        .generate_notebook(&notebook_id, &Value::Object(req_body))
        .await?;
      let run_id = gen_resp["run_id"]
        .as_str()
        .context("generate response missing run_id")?
        .to_string();

      let should_wait = wait || output_dir.is_some();
      if !should_wait {
        if json_mode {
          println!("{}", serde_json::to_string_pretty(&gen_resp)?);
        } else {
          println!("Submitted generator '{kind}' -> run_id={run_id}");
        }
        return Ok(());
      }

      let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
      let final_run = loop {
        let run_env = client.get_run(&run_id).await?;
        let run_obj = run_env.get("run").unwrap_or(&run_env);
        let status = run_obj["status"].as_str().unwrap_or("");
        match status {
          "succeeded" => break run_obj.clone(),
          "failed" | "cancelled" => {
            let err = run_obj["error"]
              .as_str()
              .unwrap_or("generator run did not succeed");
            bail!("generator run {run_id} ended with status '{status}': {err}");
          }
          _ => {}
        }
        if Instant::now() >= deadline {
          bail!("timed out after {timeout_secs}s waiting for generator run {run_id}");
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
      };

      let mut downloaded_manifest: Vec<Value> = Vec::new();
      if let Some(out_dir) = &output_dir {
        std::fs::create_dir_all(out_dir)
          .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;
        let art_env = client.list_run_artifacts(&run_id).await?;
        let art_list = art_env["artifacts"]
          .as_array()
          .cloned()
          .or_else(|| art_env.as_array().cloned())
          .unwrap_or_default();

        for art in art_list {
          let art_id = art["id"].as_str().context("artifact entry missing id")?;
          let node_id =
            sanitize_path_segment(art["node_id"].as_str().unwrap_or("artifact"), "artifact");
          let file_name = sanitize_path_segment(art["name"].as_str().unwrap_or(art_id), art_id);
          let bytes = client.download_run_artifact(&run_id, art_id).await?;
          let sha256 = format!("{:x}", Sha256::digest(&bytes));
          let node_dir = out_dir.join(&node_id);
          std::fs::create_dir_all(&node_dir)
            .with_context(|| format!("failed to create {}", node_dir.display()))?;
          let dest = node_dir.join(&file_name);
          std::fs::write(&dest, &bytes)
            .with_context(|| format!("failed to write {}", dest.display()))?;
          downloaded_manifest.push(json!({
            "id": art_id,
            "node_id": node_id,
            "name": file_name,
            "mime_type": art["mime_type"],
            "size_bytes": bytes.len(),
            "sha256": sha256,
            "path": dest.display().to_string(),
          }));
        }

        let manifest_path = out_dir.join("artifacts.json");
        std::fs::write(
          &manifest_path,
          serde_json::to_vec_pretty(&json!({
            "notebook_id": notebook_id,
            "run_id": run_id,
            "kind": kind,
            "artifacts": downloaded_manifest,
          }))?,
        )
        .with_context(|| format!("failed to write {}", manifest_path.display()))?;
      }

      if json_mode {
        println!(
          "{}",
          serde_json::to_string_pretty(&json!({
            "notebook_id": notebook_id,
            "run_id": run_id,
            "kind": kind,
            "status": final_run["status"],
            "artifacts": downloaded_manifest,
          }))?
        );
      } else {
        println!("Generator '{kind}' succeeded (run_id={run_id})");
        for item in &downloaded_manifest {
          println!(
            "  - {} ({} bytes)",
            item["path"].as_str().unwrap_or(""),
            item["size_bytes"].as_u64().unwrap_or(0)
          );
        }
      }
    }
    NotebookCommands::Plan {
      notebook_id,
      goal,
      model,
      max_items,
      source_ids,
      execute,
      cost_limit_usd,
      wait,
      output_dir,
      timeout_secs,
    } => {
      if let Some(limit) = cost_limit_usd {
        yanshi_config::config::v2::validate_cost_limit(limit).map_err(anyhow::Error::msg)?;
      }
      let clean_model = model.filter(|s| !s.trim().is_empty());
      let mut plan_body = serde_json::Map::new();
      plan_body.insert("goal".to_string(), Value::String(goal));
      plan_body.insert("max_items".to_string(), json!(max_items.clamp(1, 6)));
      if let Some(m) = &clean_model {
        plan_body.insert("model".to_string(), Value::String(m.clone()));
      }
      if !source_ids.is_empty() {
        plan_body.insert(
          "source_ids".to_string(),
          Value::Array(source_ids.iter().cloned().map(Value::String).collect()),
        );
      }

      let plan_resp = client
        .plan_notebook(&notebook_id, &Value::Object(plan_body))
        .await?;
      let should_execute = execute || wait || output_dir.is_some();
      if !should_execute {
        if json_mode {
          println!("{}", serde_json::to_string_pretty(&plan_resp)?);
        } else {
          println!(
            "Studio Plan ({}):\n  {}\n",
            plan_resp["model"].as_str().unwrap_or("default"),
            plan_resp["rationale"].as_str().unwrap_or("")
          );
          for (idx, item) in plan_resp["items"]
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .enumerate()
          {
            println!(
              "  {}. [{}] params={} {}",
              idx + 1,
              item["kind"].as_str().unwrap_or("?"),
              item["params"],
              item["reason"]
                .as_str()
                .map(|r| format!("— {r}"))
                .unwrap_or_default()
            );
          }
        }
        return Ok(());
      }

      let planned_items = plan_resp["items"]
        .as_array()
        .cloned()
        .filter(|arr| !arr.is_empty())
        .context("plan response contained no items to execute")?;
      execute_cli_batch(
        &client,
        &notebook_id,
        planned_items,
        clean_model,
        source_ids,
        cost_limit_usd,
        wait || output_dir.is_some(),
        output_dir,
        timeout_secs,
        Some(plan_resp),
        json_mode,
      )
      .await?;
    }
    NotebookCommands::GenerateBatch {
      notebook_id,
      kinds,
      focus,
      model,
      source_ids,
      cost_limit_usd,
      wait,
      output_dir,
      timeout_secs,
    } => {
      if let Some(limit) = cost_limit_usd {
        yanshi_config::config::v2::validate_cost_limit(limit).map_err(anyhow::Error::msg)?;
      }
      let clean_focus = focus
        .map(|f| f.trim().to_string())
        .filter(|f| !f.is_empty());
      let mut items: Vec<Value> = Vec::new();
      for raw_kind in kinds {
        let k = raw_kind.trim();
        if k.is_empty() {
          continue;
        }
        let mut params = serde_json::Map::new();
        if let Some(f) = &clean_focus {
          params.insert("focus".to_string(), Value::String(f.clone()));
        }
        items.push(json!({
          "kind": k,
          "params": params,
        }));
      }
      if items.is_empty() {
        bail!("--kinds must specify at least one generator kind");
      }
      execute_cli_batch(
        &client,
        &notebook_id,
        items,
        model.filter(|s| !s.trim().is_empty()),
        source_ids,
        cost_limit_usd,
        wait || output_dir.is_some(),
        output_dir,
        timeout_secs,
        None,
        json_mode,
      )
      .await?;
    }
    NotebookCommands::Artifacts { notebook_id } => {
      let env = client.list_notebook_artifacts(&notebook_id).await?;
      if json_mode {
        println!("{}", serde_json::to_string_pretty(&env)?);
      } else {
        let items = env["artifacts"].as_array().cloned().unwrap_or_default();
        if items.is_empty() {
          println!("No artifacts attached to notebook {notebook_id}.");
        } else {
          for art in items {
            let title = art["deliverable"]["title"]
              .as_str()
              .filter(|t| !t.is_empty())
              .map(|t| format!(" — {t}"))
              .unwrap_or_default();
            println!(
              "{}  [{}] {}/{} (run {}){}",
              art["id"].as_str().unwrap_or("?"),
              art["kind"].as_str().unwrap_or("?"),
              art["node_id"].as_str().unwrap_or("?"),
              art["name"].as_str().unwrap_or("?"),
              art["run_id"].as_str().unwrap_or("?"),
              title
            );
          }
        }
      }
    }
    NotebookCommands::RemoveArtifact {
      notebook_id,
      artifact_id,
    } => {
      client
        .delete_notebook_artifact(&notebook_id, &artifact_id)
        .await?;
      if json_mode {
        println!(
          "{}",
          serde_json::to_string_pretty(&json!({
            "deleted": true,
            "notebook_id": notebook_id,
            "artifact_id": artifact_id,
          }))?
        );
      } else {
        println!("Removed artifact {artifact_id} from notebook {notebook_id}");
      }
    }
    NotebookCommands::DownloadArtifacts { notebook_id, out } => {
      let zip_bytes = client.download_notebook_artifacts_zip(&notebook_id).await?;
      let short_id: String = notebook_id.chars().take(8).collect();
      let dest =
        out.unwrap_or_else(|| PathBuf::from(format!("yanshi-notebook-{short_id}-artifacts.zip")));
      if let Some(parent) = dest.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
          .with_context(|| format!("failed to create {}", parent.display()))?;
      }
      let sha256 = format!("{:x}", Sha256::digest(&zip_bytes));
      std::fs::write(&dest, &zip_bytes)
        .with_context(|| format!("failed to write {}", dest.display()))?;
      if json_mode {
        println!(
          "{}",
          serde_json::to_string_pretty(&json!({
            "notebook_id": notebook_id,
            "path": dest.display().to_string(),
            "size_bytes": zip_bytes.len(),
            "sha256": sha256,
          }))?
        );
      } else {
        println!(
          "Saved notebook archive -> {} ({} bytes)",
          dest.display(),
          zip_bytes.len()
        );
      }
    }
  }

  Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn execute_cli_batch(
  client: &ServerClient,
  notebook_id: &str,
  items: Vec<Value>,
  model: Option<String>,
  source_ids: Vec<String>,
  cost_limit_usd: Option<f64>,
  should_wait: bool,
  output_dir: Option<PathBuf>,
  timeout_secs: u64,
  plan_meta: Option<Value>,
  json_mode: bool,
) -> Result<()> {
  let mut req_body = serde_json::Map::new();
  req_body.insert("items".to_string(), Value::Array(items));
  if let Some(m) = model {
    req_body.insert("model".to_string(), Value::String(m));
  }
  if !source_ids.is_empty() {
    req_body.insert(
      "source_ids".to_string(),
      Value::Array(source_ids.into_iter().map(Value::String).collect()),
    );
  }
  if let Some(limit) = cost_limit_usd
    && let Some(num) = serde_json::Number::from_f64(limit)
  {
    req_body.insert("cost_limit_usd".to_string(), Value::Number(num));
  }

  let batch_resp = client
    .generate_notebook_batch(notebook_id, &Value::Object(req_body))
    .await?;
  let dispatched = batch_resp["runs"]
    .as_array()
    .cloned()
    .context("generate-batch response missing runs array")?;

  if !should_wait {
    if json_mode {
      let mut out = serde_json::Map::new();
      if let Some(plan) = plan_meta {
        out.insert("plan".to_string(), plan);
      }
      out.insert("batch".to_string(), batch_resp);
      println!(
        "{}",
        serde_json::to_string_pretty(&if out.contains_key("plan") {
          Value::Object(out)
        } else {
          out.remove("batch").unwrap_or(Value::Null)
        })?
      );
    } else {
      println!("Submitted {} generator run(s):", dispatched.len());
      for r in &dispatched {
        println!(
          "  - [{}] run_id={}",
          r["kind"].as_str().unwrap_or("?"),
          r["run_id"].as_str().unwrap_or("?")
        );
      }
    }
    return Ok(());
  }

  let deadline = Instant::now() + Duration::from_secs(timeout_secs.max(1));
  let mut finished_runs: Vec<Value> = Vec::with_capacity(dispatched.len());

  for entry in &dispatched {
    let run_id = entry["run_id"]
      .as_str()
      .context("dispatched run missing run_id")?
      .to_string();
    let kind = entry["kind"].as_str().unwrap_or("deliverable").to_string();

    let final_run = loop {
      let run_env = client.get_run(&run_id).await?;
      let run_obj = run_env.get("run").unwrap_or(&run_env);
      let status = run_obj["status"].as_str().unwrap_or("");
      match status {
        "succeeded" => break run_obj.clone(),
        "failed" | "cancelled" => {
          let err = run_obj["error"]
            .as_str()
            .unwrap_or("generator run did not succeed");
          bail!("batch generator '{kind}' ({run_id}) ended with status '{status}': {err}");
        }
        _ => {}
      }
      if Instant::now() >= deadline {
        bail!("timed out after {timeout_secs}s waiting for batch generator '{kind}' ({run_id})");
      }
      tokio::time::sleep(Duration::from_millis(200)).await;
    };

    let mut run_downloaded: Vec<Value> = Vec::new();
    if let Some(out_dir) = &output_dir {
      let safe_kind = sanitize_path_segment(&kind, "deliverable");
      let kind_root = out_dir.join(&safe_kind);
      std::fs::create_dir_all(&kind_root)
        .with_context(|| format!("failed to create {}", kind_root.display()))?;
      let art_env = client.list_run_artifacts(&run_id).await?;
      let art_list = art_env["artifacts"]
        .as_array()
        .cloned()
        .or_else(|| art_env.as_array().cloned())
        .unwrap_or_default();
      for art in art_list {
        let art_id = art["id"].as_str().context("artifact entry missing id")?;
        let node_id =
          sanitize_path_segment(art["node_id"].as_str().unwrap_or("artifact"), "artifact");
        let file_name = sanitize_path_segment(art["name"].as_str().unwrap_or(art_id), art_id);
        let bytes = client.download_run_artifact(&run_id, art_id).await?;
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        let node_dir = kind_root.join(&node_id);
        std::fs::create_dir_all(&node_dir)
          .with_context(|| format!("failed to create {}", node_dir.display()))?;
        let dest = node_dir.join(&file_name);
        std::fs::write(&dest, &bytes)
          .with_context(|| format!("failed to write {}", dest.display()))?;
        run_downloaded.push(json!({
          "id": art_id,
          "node_id": node_id,
          "name": file_name,
          "mime_type": art["mime_type"],
          "size_bytes": bytes.len(),
          "sha256": sha256,
          "path": dest.display().to_string(),
        }));
      }
    }

    finished_runs.push(json!({
      "run_id": run_id,
      "kind": kind,
      "status": final_run["status"],
      "artifacts": run_downloaded,
    }));
  }

  if let Some(out_dir) = &output_dir {
    std::fs::create_dir_all(out_dir)
      .with_context(|| format!("failed to create {}", out_dir.display()))?;
    let manifest_path = out_dir.join("batch_artifacts.json");
    std::fs::write(
      &manifest_path,
      serde_json::to_vec_pretty(&json!({
        "notebook_id": notebook_id,
        "runs": finished_runs,
      }))?,
    )
    .with_context(|| format!("failed to write {}", manifest_path.display()))?;
  }

  if json_mode {
    let mut payload = json!({
      "notebook_id": notebook_id,
      "runs": finished_runs,
    });
    if let Some(plan) = plan_meta {
      payload["plan"] = plan;
    }
    println!("{}", serde_json::to_string_pretty(&payload)?);
  } else {
    println!(
      "Batch completed ({} deliverable run(s)):",
      finished_runs.len()
    );
    for entry in &finished_runs {
      println!(
        "  - [{}] {} (run_id={})",
        entry["kind"].as_str().unwrap_or("?"),
        entry["status"].as_str().unwrap_or("?"),
        entry["run_id"].as_str().unwrap_or("?")
      );
    }
  }
  Ok(())
}

fn build_add_source_payload(
  url: Option<String>,
  text: Option<String>,
  file: Option<PathBuf>,
  recording: Option<PathBuf>,
  title: Option<String>,
  model: Option<String>,
  language: Option<String>,
) -> Result<Value> {
  let clean_title = title
    .map(|t| t.trim().to_string())
    .filter(|t| !t.is_empty());

  if let Some(u) = url {
    let mut map = serde_json::Map::from_iter([
      ("kind".to_string(), Value::String("url".to_string())),
      ("url".to_string(), Value::String(u)),
    ]);
    if let Some(t) = clean_title {
      map.insert("title".to_string(), Value::String(t));
    }
    return Ok(Value::Object(map));
  }

  if let Some(body) = text {
    let mut map = serde_json::Map::from_iter([
      ("kind".to_string(), Value::String("text".to_string())),
      ("text".to_string(), Value::String(body)),
    ]);
    if let Some(t) = clean_title {
      map.insert("title".to_string(), Value::String(t));
    }
    return Ok(Value::Object(map));
  }

  if let Some(path) = file {
    let bytes = std::fs::read(&path)
      .with_context(|| format!("failed to read source file {}", path.display()))?;
    let filename = path
      .file_name()
      .and_then(|n| n.to_str())
      .context("source file path has no valid UTF-8 filename")?
      .to_string();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mut map = serde_json::Map::from_iter([
      ("kind".to_string(), Value::String("file".to_string())),
      ("filename".to_string(), Value::String(filename)),
      ("content_base64".to_string(), Value::String(encoded)),
    ]);
    if let Some(t) = clean_title {
      map.insert("title".to_string(), Value::String(t));
    }
    return Ok(Value::Object(map));
  }

  if let Some(path) = recording {
    let bytes = std::fs::read(&path)
      .with_context(|| format!("failed to read recording file {}", path.display()))?;
    let filename = path
      .file_name()
      .and_then(|n| n.to_str())
      .context("recording file path has no valid UTF-8 filename")?
      .to_string();
    let encoded = base64::engine::general_purpose::STANDARD.encode(&bytes);
    let mut map = serde_json::Map::from_iter([
      ("kind".to_string(), Value::String("recording".to_string())),
      ("filename".to_string(), Value::String(filename)),
      ("content_base64".to_string(), Value::String(encoded)),
    ]);
    if let Some(t) = clean_title {
      map.insert("title".to_string(), Value::String(t));
    }
    if let Some(m) = model
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
    {
      map.insert("model".to_string(), Value::String(m));
    }
    if let Some(lang) = language
      .map(|s| s.trim().to_string())
      .filter(|s| !s.is_empty())
    {
      map.insert("language".to_string(), Value::String(lang));
    }
    return Ok(Value::Object(map));
  }

  bail!("specify exactly one of `--url`, `--text`, `--file`, or `--recording`")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn typed_param_coerces_booleans_numbers_and_strings() {
    assert_eq!(parse_typed_param_value("true"), Value::Bool(true));
    assert_eq!(parse_typed_param_value("FALSE"), Value::Bool(false));
    assert_eq!(parse_typed_param_value("12"), json!(12));
    assert_eq!(parse_typed_param_value("0.25"), json!(0.25));
    assert_eq!(
      parse_typed_param_value("key tidal metrics"),
      Value::String("key tidal metrics".to_string())
    );
  }

  #[test]
  fn add_source_payload_encodes_files_and_rejects_missing_kind() {
    let err = build_add_source_payload(None, None, None, None, None, None, None)
      .expect_err("must require one source flag");
    assert!(err.to_string().contains("--url"));

    let dir = tempfile::tempdir().expect("tempdir");
    let csv_path = dir.path().join("metrics.csv");
    std::fs::write(&csv_path, b"col,val\na,1\n").expect("write csv");
    let payload = build_add_source_payload(
      None,
      None,
      Some(csv_path),
      None,
      Some("Metrics Table".to_string()),
      None,
      None,
    )
    .expect("file payload");
    assert_eq!(payload["kind"], "file");
    assert_eq!(payload["filename"], "metrics.csv");
    assert_eq!(payload["title"], "Metrics Table");
    let decoded = base64::engine::general_purpose::STANDARD
      .decode(payload["content_base64"].as_str().expect("b64"))
      .expect("decode");
    assert_eq!(decoded, b"col,val\na,1\n");
  }
}
