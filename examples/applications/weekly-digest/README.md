# A5 — weekly-digest

**Status**: WIP — L1 pipeline + watermark/retry loopback test + live LLM run ✅ (2026-11-18; 4-week unattended soak pending)
**Tracking entry**: [`EXAMPLES_TODOs.md` § A5](../../../EXAMPLES_TODOs.md#a5--weekly-digest)

## Business

On a fixed weekly schedule, query a Markdown/briefing corpus (such as [`research-assistant`](../research-assistant/) briefings, a personal blog archive, or saved reading notes) for items modified after `last_successful_run`, rank relevant passages via BM25 RAG, synthesize a structured Markdown + HTML digest with an LLM, and ship it via SendGrid (`/v3/mail/send`) or Resend (`/emails`) with retry and atomic watermark persistence.

## Architecture: L1 4-Node `Flow` with Pre-Delivery Outbox & Atomic Watermark

Per the L1+L3 reflection rule (same rationale as A3 and A7: high pass-through on recipient addresses, endpoints, and timestamps; fixed pipeline with zero agent branching):

```
collect_and_search (SqliteEntityFactStore watermark + ParagraphChunker + Bm25KnowledgeBackend)
  └─▶ synthesize_digest (Yanshi::model -> .md + .html written to outbox_dir BEFORE network call)
        └─▶ deliver_email (SendGrid /v3/mail/send or Resend /emails, exponential backoff on 429/5xx)
              └─▶ commit_watermark (SqliteEntityFactStore records last_successful_run ONLY after 2xx)
```

Failure & dead-letter guarantees:
1. **Outbox before network**: Synthesized `.md` and `.html` are flushed to `--outbox-dir` before `deliver_email` runs, so a total provider outage never loses the LLM output.
2. **Watermark gated on `2xx`**: If `deliver_email` exhausts retries (`429` / `5xx` / connection drop), `commit_watermark` is skipped and `last_successful_run` stays untouched — the next scheduled run automatically catches up on the missed window.
3. **Zero-cost idempotent re-run**: When no corpus files have `mtime > last_successful_run`, `collect_and_search` short-circuits in `<10 ms` without calling the LLM or email API.

## Run

```bash
cd examples/applications/weekly-digest

# Local dry-run against A3 research-assistant briefings (writes .md + .html to outbox, advances watermark)
cargo run --release -- \
  --corpus-dir ../research-assistant/sample-briefings \
  --lookback-days 365 \
  --query "LLM agent reasoning" \
  --model gemini-2.5-flash \
  --outbox-dir /tmp/yanshi-weekly-digest

# Live SendGrid delivery (requires SENDGRID_API_KEY in env or ~/.yanshi/.env)
cargo run --release -- \
  --corpus-dir ~/.yanshi/briefings \
  --query "RAG sandbox distributed runtime" \
  --provider sendgrid \
  --from digest@yourdomain.com \
  --to eng-team@yourdomain.com
```

### Scheduling (`cron` / `systemd` timer)

```cron
# Every Monday at 09:00 local time
0 9 * * 1 /path/to/weekly-digest --corpus-dir /var/lib/yanshi/briefings --provider sendgrid --from digest@example.com --to team@example.com >> /var/log/yanshi-weekly-digest.log 2>&1
```

## Tests

```bash
cargo test --manifest-path examples/applications/weekly-digest/Cargo.toml
```

`tests/smoke.rs` spins up a local `TcpListener` SendGrid mock (using `reqwest::Client::builder().no_proxy()`) and verifies the full 4-run state machine: `503` retry exhaustion preserves `None` watermark → `202 Accepted` recovery delivers Bearer-authenticated payload and advances watermark → immediate re-run skips in `0` HTTP calls → new note triggers incremental digest.

## Findings during dogfooding

1. **`F-A5-1` — `FlowExt::run()` returns `Ok(state)` even when a node fails**: Because per-node errors live inside `HashMap<String, Result<..., YanshiError>>`, L1 callers must explicitly inspect `state.get(node_id)` for `Err` (unlike A3's `ok_or_else("did not produce output")` which masked the root error).
2. **`F-A5-2` — `EntityFactStore::record_fact(&mut self)` requires `&mut self` despite `SqlitePool` interior mutability**: Readers (`get_facts(&self)`) take `&self`, while writers (`record_fact(&mut self)`) require `&mut self`, preventing sharing a single `Arc<SqliteEntityFactStore>` across concurrent nodes without an outer `Mutex` or reopening per step.
3. **`F-A5-3` — Loopback HTTP tests on proxy-enabled hosts require `.no_proxy()`**: Exposing `EmailDispatcher::with_client(client, ...)` let `tests/smoke.rs` inject `reqwest::Client::builder().no_proxy().build()` cleanly while keeping system-proxy support in production `EmailDispatcher::new(...)`.
