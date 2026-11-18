//! Integration tests for `weekly-digest` (A5):
//! 1. Transient HTTP 503 retries exhaust without advancing the SQLite watermark,
//!    while preserving the rendered digest in `outbox_dir`.
//! 2. Subsequent recovery (`202 Accepted` on `/v3/mail/send`) delivers the SendGrid
//!    payload with Bearer auth and advances `last_successful_run`.
//! 3. Immediate re-run with no newer corpus documents skips LLM and HTTP calls.
//! 4. Adding a newer Markdown document delivers only the incremental update.

use std::sync::{
  Arc,
  atomic::{AtomicU16, AtomicUsize, Ordering},
};
use std::time::Duration;

use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use weekly_digest::{
  DigestConfig, EmailDispatcher, EmailTransport, read_watermark, run_weekly_digest,
};

const MOCK_MODELS_YML: &str = "\
models:
  mock-model:
    vendor: mock
    type: text
    model_id: mock-model
providers:
  mock:
    api_key_env: MOCK_API_KEY
";

struct MockMailServer {
  base_url: String,
  status_code: Arc<AtomicU16>,
  requests: Arc<AtomicUsize>,
  last_body: Arc<Mutex<Option<Value>>>,
  last_auth: Arc<Mutex<Option<String>>>,
}

async fn spawn_mock_sendgrid(initial_status: u16) -> MockMailServer {
  let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
  let addr = listener.local_addr().expect("addr");
  let status_code = Arc::new(AtomicU16::new(initial_status));
  let requests = Arc::new(AtomicUsize::new(0));
  let last_body = Arc::new(Mutex::new(None));
  let last_auth = Arc::new(Mutex::new(None));

  let status_clone = Arc::clone(&status_code);
  let req_clone = Arc::clone(&requests);
  let body_clone = Arc::clone(&last_body);
  let auth_clone = Arc::clone(&last_auth);

  tokio::spawn(async move {
    while let Ok((mut socket, _)) = listener.accept().await {
      req_clone.fetch_add(1, Ordering::SeqCst);
      let mut buf = Vec::with_capacity(8192);
      let mut chunk = [0u8; 4096];
      let mut header_end = None;
      let mut content_len = 0usize;

      while let Ok(n) = socket.read(&mut chunk).await {
        if n == 0 {
          break;
        }
        buf.extend_from_slice(&chunk[..n]);
        if header_end.is_none()
          && let Some(pos) = buf.windows(4).position(|w| w == b"\r\n\r\n")
        {
          header_end = Some(pos + 4);
          let headers = String::from_utf8_lossy(&buf[..pos]);
          for line in headers.lines() {
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
              content_len = rest.trim().parse().unwrap_or(0);
            }
            if lower.starts_with("authorization:") {
              let raw_val = line.split_once(':').map(|(_, v)| v.trim().to_string());
              *auth_clone.lock().await = raw_val;
            }
          }
        }
        if let Some(h_end) = header_end
          && buf.len() >= h_end + content_len
        {
          if let Ok(parsed) = serde_json::from_slice::<Value>(&buf[h_end..h_end + content_len]) {
            *body_clone.lock().await = Some(parsed);
          }
          break;
        }
      }

      let code = status_clone.load(Ordering::SeqCst);
      let reason = if code == 202 {
        "Accepted"
      } else {
        "Service Unavailable"
      };
      let resp = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{{}}"
      );
      let _ = socket.write_all(resp.as_bytes()).await;
    }
  });

  MockMailServer {
    base_url: format!("http://{addr}"),
    status_code,
    requests,
    last_body,
    last_auth,
  }
}

#[tokio::test]
async fn retry_failure_preserves_watermark_and_recovery_delivers_incrementally() {
  let dir = TempDir::new().expect("tempdir");
  let corpus_dir = dir.path().join("corpus");
  let outbox_dir = dir.path().join("outbox");
  let state_path = dir.path().join("state/weekly-digest.db");
  let models_yml = dir.path().join("models.yml");
  std::fs::create_dir_all(&corpus_dir).expect("corpus dir");
  std::fs::write(&models_yml, MOCK_MODELS_YML).expect("models.yml");

  std::fs::write(
    corpus_dir.join("2026-11-18-rag-notes.md"),
    "# Hybrid RAG Evaluation\n\nBM25 combined with dense embeddings improved Recall@5 by 14% on multi-hop technical queries.",
  )
  .expect("seed note");

  // Queue three mock LLM replies: Run 1 (503 failure), Run 2 (202 recovery), Run 4 (new note).
  let replies = json!([
    "# Weekly Digest\n\n## Highlights\n- [2026-11-18-rag-notes.md#0] Hybrid RAG improved Recall@5 by 14%.",
    "# Weekly Digest\n\n## Highlights\n- [2026-11-18-rag-notes.md#0] Hybrid RAG improved Recall@5 by 14%.",
    "# Weekly Digest\n\n## Highlights\n- [2026-11-18-sandbox-notes.md#0] Landlock isolation verified."
  ]);
  // SAFETY: single test in this integration test binary, set before Yanshi::init.
  unsafe {
    std::env::set_var("YANSHI_MOCK_RESPONSES", replies.to_string());
  }
  yanshi_llm::Yanshi::init_with_config(models_yml.to_str().expect("utf8"))
    .await
    .expect("init mock");

  let server = spawn_mock_sendgrid(503).await;
  let http_client = reqwest::Client::builder()
    .no_proxy()
    .build()
    .expect("no_proxy test client");

  let make_config = || DigestConfig {
    digest_id: "eng-weekly".to_string(),
    corpus_dir: corpus_dir.clone(),
    state_path: state_path.clone(),
    outbox_dir: outbox_dir.clone(),
    query: "RAG Recall Landlock".to_string(),
    top_k: 8,
    lookback_days: 7,
    model: "mock-model".to_string(),
    subject_prefix: "[Eng Digest]".to_string(),
    from_email: "bot@yanshi.dev".to_string(),
    to_emails: vec!["alice@yanshi.dev".to_string()],
    quiet_console: true,
  };

  let make_dispatcher = || {
    EmailDispatcher::with_client(
      http_client.clone(),
      EmailTransport::SendGrid {
        endpoint: server.base_url.clone(),
        api_key: "sg-secret-token".to_string(),
      },
      2, // 1 initial + 2 retries = 3 attempts
      Duration::from_millis(15),
    )
  };

  // 1. Server returns 503: retries 3 times, fails, keeps watermark None, saves dead-letter copy.
  let err = run_weekly_digest(make_config(), make_dispatcher())
    .await
    .expect_err("503 should fail run");
  assert!(
    err.to_string().contains("503"),
    "expected HTTP 503 error, got: {err:#}"
  );
  assert_eq!(server.requests.load(Ordering::SeqCst), 3);
  assert_eq!(
    read_watermark(&state_path, "eng-weekly")
      .await
      .expect("read watermark"),
    None,
    "failed delivery must NOT advance watermark"
  );
  let outbox_entries: Vec<_> = std::fs::read_dir(&outbox_dir)
    .expect("outbox exists")
    .collect();
  assert_eq!(
    outbox_entries.len(),
    2,
    "pre-delivery .md and .html preserved in outbox"
  );

  // 2. Server recovers (202 Accepted): delivers same note and advances watermark.
  server.status_code.store(202, Ordering::SeqCst);
  let ok_run = run_weekly_digest(make_config(), make_dispatcher())
    .await
    .expect("202 recovery succeeds");
  assert_eq!(ok_run.new_docs, 1);
  assert!(ok_run.delivered);
  assert_eq!(ok_run.delivery_attempts, 1);
  assert!(ok_run.watermark_after.is_some());
  assert_eq!(server.requests.load(Ordering::SeqCst), 4);
  assert_eq!(
    server.last_auth.lock().await.as_deref(),
    Some("Bearer sg-secret-token")
  );
  let sent_json = server.last_body.lock().await.clone().expect("sent json");
  assert_eq!(
    sent_json["personalizations"][0]["to"][0]["email"],
    "alice@yanshi.dev"
  );

  // 3. Immediate re-run with no newer files: zero docs, zero HTTP requests.
  let noop_run = run_weekly_digest(make_config(), make_dispatcher())
    .await
    .expect("noop run succeeds");
  assert_eq!(noop_run.new_docs, 0);
  assert!(!noop_run.delivered);
  assert_eq!(server.requests.load(Ordering::SeqCst), 4);

  // 4. Sleep briefly so filesystem mtime > watermark, write a new note, verify incremental send.
  tokio::time::sleep(Duration::from_millis(25)).await;
  std::fs::write(
    corpus_dir.join("2026-11-18-sandbox-notes.md"),
    "# Linux Sandbox Hardening\n\nLandlock and seccomp filters now cover legacy x86_64 filesystem write syscalls.",
  )
  .expect("second note");

  let inc_run = run_weekly_digest(make_config(), make_dispatcher())
    .await
    .expect("incremental run succeeds");
  assert_eq!(
    inc_run.new_docs, 1,
    "only the newly added note is processed"
  );
  assert!(inc_run.delivered);
  assert_eq!(server.requests.load(Ordering::SeqCst), 5);
  assert!(inc_run.watermark_after > ok_run.watermark_after);
}
