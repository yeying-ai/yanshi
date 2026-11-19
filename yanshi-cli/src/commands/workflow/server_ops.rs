//! Server-backed `workflow` subcommands (P2.5).
//!
//! These delegate to the [`ServerClient`] when the caller passes
//! `--server <url>` (or sets `YANSHI_SERVER_URL`). They never reach
//! into the in-process executor — that path is in `run.rs` and stays
//! the default when no server URL is configured.

use anyhow::Result;

use crate::server_client::{ServerClient, resolve_auth_token, resolve_tenant_id};

/// Build a [`ServerClient`] from `(server_url, auth_token, tenant)` flags
/// after applying the env fallbacks. The caller has already decided to
/// enter server mode (i.e. `resolve_server_url` returned `Some`).
pub fn build_client(
  server_url: &str,
  auth_token: Option<&str>,
  tenant: Option<&str>,
) -> Result<ServerClient> {
  let token = resolve_auth_token(auth_token);
  let tenant_id = resolve_tenant_id(tenant);
  ServerClient::new(server_url.to_string(), token, tenant_id)
}

/// Render the server response — either as the legacy bare JSON
/// body (default) or wrapped in the canonical `CliJsonEnvelope`.
fn print_server_response(command: &str, format: &str, body: &serde_json::Value) -> Result<()> {
  if format == "json-envelope" {
    let envelope = crate::json_envelope::CliJsonEnvelope::ok(command, body);
    println!("{}", serde_json::to_string_pretty(&envelope)?);
  } else {
    println!("{}", serde_json::to_string_pretty(body)?);
  }
  Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn list(
  server_url: &str,
  auth_token: Option<&str>,
  tenant: Option<&str>,
  limit: Option<i64>,
  offset: Option<i64>,
  status: Option<&str>,
  format: &str,
) -> Result<()> {
  let client = build_client(server_url, auth_token, tenant)?;
  let body = client.list_runs(limit, offset, status).await?;
  print_server_response("workflow list", format, &body)
}

pub async fn cancel(
  server_url: &str,
  auth_token: Option<&str>,
  tenant: Option<&str>,
  run_id: &str,
  format: &str,
) -> Result<()> {
  let client = build_client(server_url, auth_token, tenant)?;
  let body = client.cancel_run(run_id).await?;
  print_server_response("workflow cancel", format, &body)
}

/// P10.11.4: reject `workflow run` flags that are local-only when
/// the operator is dispatching via `--server`. Today the
/// `POST /v1/runs` wire body only accepts `{ workflow, tenant_id }`
/// — every per-run knob the local executor consumes
/// (`--model` / `--execution-mode` / `--max-concurrency` /
/// `--run-dir` / `--watch` / `--output` / `--input` / `--dry-run` /
/// `--timeout` / `--max-retries`) would be silently dropped without
/// this guard, which is exactly the class of bug operators only
/// catch in prod.
///
/// Two categories:
/// - **Always-local**: filesystem-side concerns
///   (`--run-dir`, `--output <path>`) and the in-process flow
///   (`--watch`, `--dry-run`). The server has its own filesystem
///   and event stream, so these never make sense remotely.
/// - **Future API addition**: server-side execution knobs that the
///   wire format could accept but doesn't today (`--model`,
///   `--execution-mode`, `--max-concurrency`, `--input`,
///   `--timeout`, `--max-retries`). Each error names the gap and
///   points at the local-mode workaround.
///
/// Options forwarded to `POST /v1/runs` and post-completion artifact retrieval
/// when `yanshi workflow run --server` is invoked.
pub struct ServerRunOptions<'a> {
  pub inputs: &'a serde_json::Map<String, serde_json::Value>,
  pub model: Option<&'a str>,
  pub cost_limit_usd: Option<f64>,
  pub max_concurrency: Option<usize>,
  pub output_dir: Option<&'a std::path::Path>,
  pub poll_timeout: std::time::Duration,
}

/// Reject `workflow run` flags that remain strictly local-only when
/// dispatching via `--server` (`--run-dir`, `--output`, `--checkpoint`,
/// `--watch`, `--dry-run`, `--max-retries`), and validate server-bound
/// `--max-concurrency` / `--cost-limit-usd` up front before opening a socket.
#[allow(clippy::too_many_arguments)]
pub fn reject_local_only_flags(
  max_concurrency: usize,
  cost_limit_usd: Option<f64>,
  run_dir: Option<&str>,
  watch: bool,
  output: Option<&str>,
  checkpoint: bool,
  dry_run: bool,
  max_retries: u32,
) -> Result<()> {
  if run_dir.is_some() {
    anyhow::bail!(
      "--run-dir is local-only (the server controls its own filesystem layout under its \
       configured YANSHI_RUN_DIR). Drop --run-dir when using --server."
    );
  }
  if let Some(path) = output {
    anyhow::bail!(
      "--output '{path}' is local-only (writes the final workflow output to a local file). \
       In server mode, capture the terminal run row via --format json-envelope or stream \
       events via `yanshi workflow logs <run_id> --follow`."
    );
  }
  if checkpoint {
    anyhow::bail!(
      "--checkpoint is local-only: the server checkpoints every run itself. Drop --checkpoint \
       when using --server and resume with POST /v1/runs/{{id}}:resume."
    );
  }
  if watch {
    anyhow::bail!(
      "--watch is local-only (polls the in-process run state). In server mode, stream the \
       event log via `yanshi workflow logs <run_id> --follow` after submitting the run."
    );
  }
  if dry_run {
    anyhow::bail!(
      "--dry-run is local-only (validates without execution). In server mode, validate the \
       workflow up-front via `yanshi workflow validate <file>` and then submit only \
       once validation passes."
    );
  }
  if max_retries != 0 {
    anyhow::bail!(
      "--max-retries {max_retries} is not yet wired to the server (POST /v1/runs body does \
       not accept a per-run retry budget today). Drop --max-retries or run locally."
    );
  }
  if max_concurrency == 0 {
    anyhow::bail!("--max-concurrency must be greater than zero");
  }
  if let Some(limit) = cost_limit_usd {
    yanshi_config::config::v2::validate_cost_limit(limit).map_err(anyhow::Error::msg)?;
  }
  Ok(())
}

/// `yanshi workflow logs <run_id>` — stream the server's
/// persisted event log for a run. Without `--follow`, fetches the
/// historical events as a single JSON array via
/// `GET /v1/runs/{id}/events/history`. With `--follow`, opens an
/// SSE connection at `GET /v1/runs/{id}/events` and keeps printing
/// events until the server closes (run reaches a terminal state)
/// or the user cancels.
///
/// `format` chooses the stdout shape:
/// - `"text"` (default): one line per event:
///   `[seq] kind ts payload_summary`.
/// - `"json"`: one [`serde_json::Value`] per line (JSONL).
/// - `"json-envelope"`: a single canonical `CliJsonEnvelope`
///   wrapping the events array. Only valid for the non-follow
///   path — envelope mode requires a single bounded output, so
///   combining it with `--follow` is rejected with a clear error.
///
/// `after_seq` lets a reconnecting consumer resume past a known
/// last-seen `seq`, avoiding duplicate prints.
#[allow(clippy::too_many_arguments)]
pub async fn logs(
  server_url: &str,
  auth_token: Option<&str>,
  tenant: Option<&str>,
  run_id: &str,
  follow: bool,
  after_seq: Option<i64>,
  format: &str,
) -> Result<()> {
  let client = build_client(server_url, auth_token, tenant)?;

  if follow && format == "json-envelope" {
    anyhow::bail!(
      "--format json-envelope is incompatible with --follow: an envelope wraps a bounded result, \
       but --follow produces an open-ended stream. Use --format json (JSONL) or --format text."
    );
  }

  if !follow {
    let body = client.list_events_history(run_id, after_seq).await?;
    return print_events_history(format, run_id, &body);
  }

  // Q3.5.3: follow mode wraps `stream_events_sse` in a reconnect
  // loop with bounded exponential backoff. Without this, any
  // mid-stream TCP blip (network reset, server rolling restart,
  // gateway 502) terminated the consumer with a hard error and the
  // operator had to manually rerun with `--after-seq <last_seen>`.
  // We track the highest observed `seq` across reconnects so events
  // already printed are not re-emitted.
  stream_events_with_reconnect(&client, run_id, after_seq, format).await
}

/// Q3.5.3: follow-mode driver — calls `stream_events_sse` in a loop,
/// resuming past the highest seq we've successfully printed. Transport
/// errors trigger an exponential-backoff sleep (1s → 30s cap) before
/// the next reconnect; HTTP-level errors (4xx/5xx from the server)
/// stay fatal because they typically mean misconfiguration, not a
/// transient hiccup.
///
/// Distinguishing a terminal close from a transient drop is the
/// trickiest bit: the SSE wire has no explicit "we're done" marker.
/// The server ends the stream after a terminal event kind
/// ([`is_terminal_run_event_kind`]), so "stream closed after a terminal
/// event" is a normal exit. Some failure paths (a workflow that fails
/// to build, a skill run that errors) end the run without one, so a
/// clean close that delivered nothing new asks for the run's status and
/// exits when it is terminal; any other close is a blip and reconnects.
async fn stream_events_with_reconnect(
  client: &crate::server_client::ServerClient,
  run_id: &str,
  initial_after_seq: Option<i64>,
  format: &str,
) -> Result<()> {
  use std::time::Duration;

  // Highest seq we've already handed to `print_single_event`. The
  // server's `?after_seq=N` filter is strictly `seq > N`, so passing
  // the last printed seq directly resumes without duplicates.
  let mut high_water_mark: Option<i64> = initial_after_seq;
  let mut saw_terminal_event = false;
  // Backoff state — bumped on each consecutive transport failure,
  // reset whenever at least one event flows through cleanly.
  const BACKOFF_FLOOR: Duration = Duration::from_secs(1);
  const BACKOFF_CEILING: Duration = Duration::from_secs(30);
  let mut next_backoff = BACKOFF_FLOOR;

  loop {
    let mut events_since_reconnect: u64 = 0;
    let stream_result = {
      let high_water_mark_ref = &mut high_water_mark;
      let events_since_reconnect_ref = &mut events_since_reconnect;
      let saw_terminal_event_ref = &mut saw_terminal_event;
      client
        .stream_events_sse(run_id, *high_water_mark_ref, |event| {
          if let Some(seq) = event.get("seq").and_then(|s| s.as_i64()) {
            *high_water_mark_ref = Some(match *high_water_mark_ref {
              Some(prev) => prev.max(seq),
              None => seq,
            });
          }
          if let Some(kind) = event.get("kind").and_then(|k| k.as_str())
            && is_terminal_run_event_kind(kind)
          {
            *saw_terminal_event_ref = true;
          }
          *events_since_reconnect_ref += 1;
          if let Err(err) = print_single_event(format, &event) {
            eprintln!("warning: failed to render event: {err}");
          }
        })
        .await
    };

    if events_since_reconnect > 0 {
      next_backoff = BACKOFF_FLOOR;
    }

    match stream_result {
      Ok(()) if saw_terminal_event => {
        // Clean close after a terminal event — the run is genuinely
        // done; exit without reconnecting.
        return Ok(());
      }
      Ok(()) if events_since_reconnect == 0 && run_is_terminal(client, run_id).await? => {
        return Ok(());
      }
      Ok(()) => {
        // Stream closed without a terminal marker — treat as a
        // transient drop and reconnect after the backoff window.
        eprintln!(
          "⚠  workflow logs stream closed mid-run after {events_since_reconnect} events; \
           reconnecting in {next_backoff:?}"
        );
        tokio::time::sleep(next_backoff).await;
        next_backoff = (next_backoff.saturating_mul(2)).min(BACKOFF_CEILING);
      }
      Err(err) => {
        eprintln!(
          "⚠  workflow logs stream dropped after {events_since_reconnect} events; \
           reconnecting in {next_backoff:?}. cause: {err}"
        );
        tokio::time::sleep(next_backoff).await;
        next_backoff = (next_backoff.saturating_mul(2)).min(BACKOFF_CEILING);
      }
    }
  }
}

/// Terminal event kinds the server ends a run's SSE stream after.
/// Mirrors `yanshi_server::events_stream::is_terminal_run_event_kind`;
/// change both together.
fn is_terminal_run_event_kind(kind: &str) -> bool {
  matches!(
    kind,
    "workflow.completed"
      | "workflow.failed"
      | "workflow.cancelled"
      | "skill_run_completed"
      | "run_completed"
      | "run.cancelled"
  )
}

async fn run_is_terminal(
  client: &crate::server_client::ServerClient,
  run_id: &str,
) -> Result<bool> {
  let run = client.get_run(run_id).await?;
  Ok(matches!(
    run["status"].as_str(),
    Some("succeeded" | "failed" | "cancelled")
  ))
}

fn print_events_history(format: &str, run_id: &str, body: &serde_json::Value) -> Result<()> {
  match format {
    "json-envelope" => {
      let envelope = crate::json_envelope::CliJsonEnvelope::ok("workflow logs", body);
      println!("{}", serde_json::to_string_pretty(&envelope)?);
    }
    "json" => {
      // JSONL: one event per line. Bare `Value` rendering keeps
      // downstream `jq -c` filters happy.
      let events = body.as_array().ok_or_else(|| {
        anyhow::anyhow!("server response for {run_id} events was not a JSON array (got: {body:?})")
      })?;
      for event in events {
        println!("{}", serde_json::to_string(event)?);
      }
    }
    _ => {
      // text: human-readable summary, one line per event.
      let events = body.as_array().ok_or_else(|| {
        anyhow::anyhow!("server response for {run_id} events was not a JSON array (got: {body:?})")
      })?;
      if events.is_empty() {
        eprintln!("(no events for run {run_id})");
      }
      for event in events {
        println!("{}", format_event_text(event));
      }
    }
  }
  Ok(())
}

fn print_single_event(format: &str, event: &serde_json::Value) -> Result<()> {
  match format {
    "json" => println!("{}", serde_json::to_string(event)?),
    _ => println!("{}", format_event_text(event)),
  }
  Ok(())
}

/// Format a single `StreamedEvent` JSON value as a one-line human
/// summary. Shape matches `yanshi_server::events_stream::
/// StreamedEvent`: `{ run_id, seq, kind, payload, ts }`.
/// Missing fields render as `?` rather than panicking — the
/// command's job is to surface what the server sent, not validate
/// it.
fn format_event_text(event: &serde_json::Value) -> String {
  let seq = event
    .get("seq")
    .and_then(|s| s.as_i64())
    .map(|s| s.to_string())
    .unwrap_or_else(|| "?".into());
  let kind = event.get("kind").and_then(|k| k.as_str()).unwrap_or("?");
  let ts = event.get("ts").and_then(|t| t.as_str()).unwrap_or("?");
  // Render payload as compact JSON so the line stays scannable.
  // Truncate to 240 chars to keep `tail -f` legible — full payload
  // is available via `--format json`.
  let payload = event
    .get("payload")
    .map(|p| serde_json::to_string(p).unwrap_or_else(|_| "{}".into()))
    .unwrap_or_else(|| "{}".into());
  const MAX_PAYLOAD_CHARS: usize = 240;
  let payload_display = if payload.chars().count() > MAX_PAYLOAD_CHARS {
    let mut truncated: String = payload.chars().take(MAX_PAYLOAD_CHARS).collect();
    truncated.push('…');
    truncated
  } else {
    payload
  };
  format!("[{seq}] {kind:<32} {ts} {payload_display}")
}

fn sanitize_remote_segment(raw: &str, fallback: &str) -> String {
  let candidate = std::path::Path::new(raw)
    .file_name()
    .and_then(|n| n.to_str())
    .map(str::trim)
    .filter(|s| !s.is_empty() && *s != "." && *s != "..")
    .unwrap_or(fallback);
  candidate.replace(['/', '\\'], "_")
}

async fn download_server_run_artifacts(
  client: &ServerClient,
  run_id: &str,
  out_dir: &std::path::Path,
) -> Result<usize> {
  use anyhow::Context as _;
  use sha2::{Digest as _, Sha256};

  std::fs::create_dir_all(out_dir)
    .with_context(|| format!("failed to create output directory {}", out_dir.display()))?;
  let art_env = client.list_run_artifacts(run_id).await?;
  let art_list = art_env["artifacts"]
    .as_array()
    .cloned()
    .or_else(|| art_env.as_array().cloned())
    .unwrap_or_default();

  let mut manifest_entries = Vec::with_capacity(art_list.len());
  for art in art_list {
    let art_id = art["id"].as_str().context("artifact entry missing id")?;
    let node_id =
      sanitize_remote_segment(art["node_id"].as_str().unwrap_or("artifact"), "artifact");
    let file_name = sanitize_remote_segment(art["name"].as_str().unwrap_or(art_id), art_id);
    let bytes = client.download_run_artifact(run_id, art_id).await?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let node_dir = out_dir.join(&node_id);
    std::fs::create_dir_all(&node_dir)
      .with_context(|| format!("failed to create {}", node_dir.display()))?;
    let dest = node_dir.join(&file_name);
    std::fs::write(&dest, &bytes).with_context(|| format!("failed to write {}", dest.display()))?;
    manifest_entries.push(serde_json::json!({
      "id": art_id,
      "node_id": node_id,
      "file": format!("{node_id}/{file_name}"),
      "mime_type": art["mime_type"],
      "size_bytes": bytes.len(),
      "sha256": sha256,
    }));
  }

  let manifest_path = out_dir.join(super::artifacts::MANIFEST_FILE);
  std::fs::write(
    &manifest_path,
    serde_json::to_vec_pretty(&serde_json::json!({
      "run_id": run_id,
      "artifacts": manifest_entries,
    }))?,
  )
  .with_context(|| format!("failed to write {}", manifest_path.display()))?;
  Ok(manifest_entries.len())
}

/// Submit a workflow body via the server and poll until terminal.
/// Returns `Err` when the run ends in `failed` or `cancelled` so CI / shell
/// callers observe a non-zero exit code.
pub async fn run_via_server(
  server_url: &str,
  auth_token: Option<&str>,
  tenant: Option<&str>,
  workflow_text: &str,
  format: &str,
  opts: ServerRunOptions<'_>,
) -> Result<()> {
  let is_envelope = format == "json-envelope";
  let client = build_client(server_url, auth_token, tenant)?;
  let submission = client
    .submit_run_with_options(
      workflow_text,
      Some(opts.inputs),
      opts.model,
      opts.cost_limit_usd,
      opts.max_concurrency,
    )
    .await?;
  let run_id = submission["run_id"]
    .as_str()
    .ok_or_else(|| anyhow::anyhow!("server response missing run_id: {submission}"))?
    .to_string();

  let progress_line = format!(
    "📋 Submitted run {run_id}; status: {}",
    submission["status"]
  );
  if is_envelope {
    eprintln!("{progress_line}");
  } else {
    println!("{progress_line}");
  }

  const POLL_INTERVAL_MS: u64 = 250;
  let deadline =
    std::time::Instant::now() + opts.poll_timeout.max(std::time::Duration::from_millis(1));
  loop {
    let row = client.get_run(&run_id).await?;
    let status = row["status"].as_str().unwrap_or("unknown");
    if matches!(status, "succeeded" | "failed" | "cancelled") {
      if status == "succeeded"
        && let Some(out_dir) = opts.output_dir
      {
        let count = download_server_run_artifacts(&client, &run_id, out_dir).await?;
        let dl_msg = format!(
          "📤 Downloaded {count} artifact(s) to {} (manifest: {})",
          out_dir.display(),
          super::artifacts::MANIFEST_FILE
        );
        if is_envelope {
          eprintln!("{dl_msg}");
        } else {
          println!("{dl_msg}");
        }
      }
      if is_envelope {
        let errors: Vec<String> = if status == "succeeded" {
          Vec::new()
        } else {
          vec![format!("run {run_id} ended with status '{status}'")]
        };
        let envelope =
          crate::json_envelope::CliJsonEnvelope::with_errors("workflow run", &row, errors);
        println!("{}", serde_json::to_string_pretty(&envelope)?);
      } else {
        println!("{}", serde_json::to_string_pretty(&row)?);
      }
      if status != "succeeded" {
        let err_detail = row["error"]
          .as_str()
          .filter(|e| !e.trim().is_empty())
          .map(|e| format!(": {e}"))
          .unwrap_or_default();
        anyhow::bail!("run {run_id} ended with status '{status}'{err_detail}");
      }
      return Ok(());
    }
    if std::time::Instant::now() >= deadline {
      anyhow::bail!(
        "run {run_id} did not reach a terminal status within {:?}; last status: {status}",
        opts.poll_timeout
      );
    }
    tokio::time::sleep(std::time::Duration::from_millis(POLL_INTERVAL_MS)).await;
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use serde_json::json;

  #[test]
  fn format_event_text_renders_all_fields() {
    let event = json!({
      "run_id": "00000000-0000-0000-0000-000000000001",
      "seq": 12,
      "kind": "step_started",
      "payload": { "node_id": "render" },
      "ts": "2026-05-20T10:00:00Z",
    });
    let line = format_event_text(&event);
    assert!(line.contains("[12]"));
    assert!(line.contains("step_started"));
    assert!(line.contains("2026-05-20T10:00:00Z"));
    assert!(line.contains(r#"{"node_id":"render"}"#));
  }

  #[test]
  fn format_event_text_handles_missing_fields_gracefully() {
    let event = json!({});
    let line = format_event_text(&event);
    // Missing seq → "?"; missing kind → "?"; missing ts → "?";
    // missing payload → "{}". Must never panic on partial input.
    assert!(line.contains("[?]"));
    assert!(line.contains('?'));
  }

  #[test]
  fn format_event_text_truncates_long_payloads() {
    let big_string: String = std::iter::repeat_n('x', 1000).collect();
    let event = json!({
      "seq": 1,
      "kind": "noisy",
      "ts": "now",
      "payload": { "blob": big_string },
    });
    let line = format_event_text(&event);
    // Truncation appends a `…` glyph; cap is 240 chars of payload.
    assert!(line.contains('…'), "long payloads must be truncated");
    // The wrapping format `[seq] kind ts payload` makes the line
    // somewhat longer than 240; ensure we did NOT include the full
    // 1000-char blob.
    assert!(
      !line.contains(&"x".repeat(1000)),
      "full blob must not appear in the truncated line"
    );
  }

  // ── P10.11.4 reject_local_only_flags tests ─────────────────────────
  //
  // Each test isolates one flag so a regression that loosens the
  // guard for one knob doesn't silently land. The defaults passed
  // to the validator must mirror the clap definitions in main.rs.

  /// Test seam over [`reject_local_only_flags`]: every test fills
  /// in the defaults from the clap definitions in `main.rs`, then
  /// overrides exactly one flag, then asserts the per-flag message.
  /// The `#[allow]` keeps clippy from flagging the long arg list —
  /// the wrapped function is the one that needs it; the helper
  /// inherits the shape.
  #[allow(clippy::too_many_arguments)]
  fn run_validator(
    max_concurrency: usize,
    cost_limit_usd: Option<f64>,
    run_dir: Option<&str>,
    watch: bool,
    output: Option<&str>,
    checkpoint: bool,
    dry_run: bool,
    max_retries: u32,
  ) -> Result<()> {
    reject_local_only_flags(
      max_concurrency,
      cost_limit_usd,
      run_dir,
      watch,
      output,
      checkpoint,
      dry_run,
      max_retries,
    )
  }

  #[test]
  fn workflow_run_server_baseline_passes() {
    run_validator(4, None, None, false, None, false, false, 0).expect("defaults must pass");
    run_validator(8, Some(1.5), None, false, None, false, false, 0)
      .expect("wired concurrency + cost_limit_usd must pass");
  }

  #[test]
  fn workflow_run_server_rejects_run_dir_as_local_only() {
    let err = run_validator(4, None, Some("/tmp/x"), false, None, false, false, 0)
      .expect_err("--run-dir must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("--run-dir is local-only"), "{msg}");
    assert!(
      msg.contains("YANSHI_RUN_DIR"),
      "should name the server-side env var: {msg}"
    );
  }

  #[test]
  fn workflow_run_server_rejects_output_path_with_alternative_hint() {
    let err = run_validator(4, None, None, false, Some("out.json"), false, false, 0)
      .expect_err("--output must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("--output 'out.json' is local-only"), "{msg}");
    assert!(
      msg.contains("json-envelope") || msg.contains("workflow logs"),
      "{msg}"
    );
  }

  #[test]
  fn workflow_run_server_rejects_watch_with_logs_alternative() {
    let err = run_validator(4, None, None, true, None, false, false, 0)
      .expect_err("--watch must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("--watch is local-only"), "{msg}");
    assert!(msg.contains("workflow logs"), "{msg}");
    assert!(msg.contains("--follow"), "{msg}");
  }

  #[test]
  fn workflow_run_server_rejects_dry_run_with_validate_alternative() {
    let err = run_validator(4, None, None, false, None, false, true, 0)
      .expect_err("--dry-run must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("--dry-run is local-only"), "{msg}");
    assert!(msg.contains("workflow validate"), "{msg}");
  }

  #[test]
  fn workflow_run_server_rejects_max_retries_when_nonzero() {
    let err = run_validator(4, None, None, false, None, false, false, 3)
      .expect_err("--max-retries 3 must be rejected");
    let msg = err.to_string();
    assert!(msg.contains("--max-retries 3"), "{msg}");
  }

  #[test]
  fn workflow_run_server_validates_zero_concurrency_and_bad_cost_limit() {
    let zero = run_validator(0, None, None, false, None, false, false, 0)
      .expect_err("--max-concurrency 0 must fail");
    assert!(zero.to_string().contains("--max-concurrency"));

    let bad_cost = run_validator(4, Some(0.0), None, false, None, false, false, 0)
      .expect_err("--cost-limit-usd 0 must fail");
    assert!(bad_cost.to_string().contains("cost_limit_usd"));
  }

  /// P10.11.1: --follow + --format json-envelope is rejected with
  /// a clear error. The contract: an envelope is bounded; a follow
  /// stream is unbounded — they're mutually exclusive.
  #[tokio::test]
  async fn logs_rejects_follow_with_json_envelope_format() {
    // No need for a real server — the validation runs before any
    // HTTP call. Using a junk URL exercises the early-bail path.
    let err = logs(
      "http://127.0.0.1:1",
      None,
      None,
      "any-run-id",
      true,            // follow
      None,            // after_seq
      "json-envelope", // incompatible
    )
    .await
    .expect_err("must reject the combination");
    let msg = err.to_string();
    assert!(
      msg.contains("incompatible with --follow"),
      "error must explain the incompatibility, got: {msg}"
    );
  }
}
