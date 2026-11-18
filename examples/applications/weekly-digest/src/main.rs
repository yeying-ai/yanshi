use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result};
use tracing_subscriber::EnvFilter;
use weekly_digest::{DigestConfig, EmailDispatcher, EmailTransport, run_weekly_digest};

fn load_yanshi_dotenv() {
  if let Some(home) = std::env::home_dir() {
    let _ = dotenvy::from_path(home.join(".yanshi").join(".env"));
  }
}

fn default_state_path() -> PathBuf {
  std::env::home_dir()
    .map(|h| h.join(".yanshi/state/weekly-digest.db"))
    .unwrap_or_else(|| PathBuf::from("weekly-digest.db"))
}

fn print_help() {
  println!(
    "weekly-digest — A5 Yanshi app (BM25 RAG + LLM weekly digest + SendGrid/Resend delivery)\n\
     \n\
     USAGE:\n  \
       weekly-digest --corpus-dir <dir> [OPTIONS]\n\
     \n\
     FLAGS:\n  \
       --corpus-dir <dir>       Folder of Markdown/text notes (e.g. ../research-assistant/sample-briefings)\n  \
       --digest-id <id>         Watermark namespace in SQLite (default: default)\n  \
       --query <q>              BM25 retrieval focus query, or '*' for chronological (default: *)\n  \
       --top-k <N>              Max chunks fed to LLM (default: 12)\n  \
       --lookback-days <N>      Initial lookback window when no watermark exists (default: 7)\n  \
       --model <name>           LLM model registry name (default: gemini-2.5-flash)\n  \
       --provider <mode>        dry-run | sendgrid | resend (default: dry-run)\n  \
       --endpoint <url>         Override email API base URL (default: provider official URL)\n  \
       --from <email>           Sender email address (default: digest@example.org)\n  \
       --to <email[,email...]>  Comma-separated recipient list (default: team@example.org)\n  \
       --max-retries <N>        Retries on 429/5xx/network errors (default: 3)\n  \
       --state <path>           SQLite watermark DB (default: ~/.yanshi/state/weekly-digest.db)\n  \
       --outbox-dir <dir>       Archive/dead-letter output folder (default: /tmp/yanshi-weekly-digest)\n  \
       -h, --help               Show this help\n\
     \n\
     ENV:\n  \
       SENDGRID_API_KEY         Required when --provider sendgrid\n  \
       RESEND_API_KEY           Required when --provider resend\n"
  );
}

#[tokio::main]
async fn main() -> Result<()> {
  load_yanshi_dotenv();
  tracing_subscriber::fmt()
    .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
    .init();

  let mut corpus_dir: Option<PathBuf> = None;
  let mut digest_id = "default".to_string();
  let mut query = "*".to_string();
  let mut top_k: usize = 12;
  let mut lookback_days: i64 = 7;
  let mut model = "gemini-2.5-flash".to_string();
  let mut provider = "dry-run".to_string();
  let mut endpoint: Option<String> = None;
  let mut from_email = "digest@example.org".to_string();
  let mut to_emails = vec!["team@example.org".to_string()];
  let mut max_retries: u32 = 3;
  let mut state_path: Option<PathBuf> = None;
  let mut outbox_dir = PathBuf::from("/tmp/yanshi-weekly-digest");

  let mut args = std::env::args().skip(1);
  while let Some(flag) = args.next() {
    match flag.as_str() {
      "--corpus-dir" => {
        corpus_dir = Some(args.next().context("--corpus-dir expects a path")?.into());
      }
      "--digest-id" => digest_id = args.next().context("--digest-id expects an id")?,
      "--query" => query = args.next().context("--query expects a string")?,
      "--top-k" => {
        top_k = args
          .next()
          .context("--top-k expects an integer")?
          .parse()
          .context("--top-k must be an integer")?;
      }
      "--lookback-days" => {
        lookback_days = args
          .next()
          .context("--lookback-days expects an integer")?
          .parse()
          .context("--lookback-days must be an integer")?;
      }
      "--model" => model = args.next().context("--model expects a model name")?,
      "--provider" => {
        provider = args
          .next()
          .context("--provider expects dry-run|sendgrid|resend")?
      }
      "--endpoint" => endpoint = Some(args.next().context("--endpoint expects a URL")?),
      "--from" => from_email = args.next().context("--from expects an email address")?,
      "--to" => {
        let raw = args.next().context("--to expects email address(es)")?;
        to_emails = raw
          .split(',')
          .map(str::trim)
          .filter(|s| !s.is_empty())
          .map(ToString::to_string)
          .collect();
        if to_emails.is_empty() {
          anyhow::bail!("--to must specify at least one recipient email");
        }
      }
      "--max-retries" => {
        max_retries = args
          .next()
          .context("--max-retries expects an integer")?
          .parse()
          .context("--max-retries must be an integer")?;
      }
      "--state" => state_path = Some(args.next().context("--state expects a path")?.into()),
      "--outbox-dir" => {
        outbox_dir = args.next().context("--outbox-dir expects a path")?.into();
      }
      "-h" | "--help" => {
        print_help();
        return Ok(());
      }
      other => anyhow::bail!("unknown flag `{other}` — pass --help for usage"),
    }
  }

  let corpus_dir = corpus_dir.context("--corpus-dir <dir> is required")?;
  let transport = match provider.as_str() {
    "dry-run" => EmailTransport::DryRun,
    "sendgrid" => {
      let api_key = std::env::var("SENDGRID_API_KEY")
        .context("SENDGRID_API_KEY env var is required for --provider sendgrid")?;
      EmailTransport::SendGrid {
        endpoint: endpoint.unwrap_or_else(|| "https://api.sendgrid.com".to_string()),
        api_key,
      }
    }
    "resend" => {
      let api_key = std::env::var("RESEND_API_KEY")
        .context("RESEND_API_KEY env var is required for --provider resend")?;
      EmailTransport::Resend {
        endpoint: endpoint.unwrap_or_else(|| "https://api.resend.com".to_string()),
        api_key,
      }
    }
    other => anyhow::bail!("unsupported --provider `{other}` (expected dry-run, sendgrid, resend)"),
  };

  let config = DigestConfig {
    digest_id,
    corpus_dir,
    state_path: state_path.unwrap_or_else(default_state_path),
    outbox_dir,
    query,
    top_k,
    lookback_days,
    model,
    subject_prefix: "[Yanshi Weekly Digest]".to_string(),
    from_email,
    to_emails,
    quiet_console: false,
  };
  let dispatcher = EmailDispatcher::new(transport, max_retries, Duration::from_millis(1000))?;
  let outcome = run_weekly_digest(config, dispatcher).await?;

  println!("\n=== Weekly Digest Outcome ===");
  println!("{}", serde_json::to_string_pretty(&outcome)?);
  Ok(())
}
