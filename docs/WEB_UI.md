# Yanshi Studio & Web UI (`yanshi-ui` + `src-tauri`)

`yanshi-ui/` is Yanshi's cross-platform visual workflow studio, one-click
deliverable runner, and single-run event console. A single React + Vite +
TypeScript codebase runs in two deployment modes:

1. **Embedded Gateway Web UI (`/ui/*`)** — compiled into `yanshi-server` via
   `include_str!` (`yanshi-server/src/ui.rs`), requiring zero Node.js runtime in
   production.
2. **Standalone Desktop App (`Yanshi Studio` via Tauri v2)** — packaged by
   [`yanshi-ui/src-tauri/`](../yanshi-ui/src-tauri/) with an in-process
   loopback engine (`local_engine.rs`) that executes workflows directly against
   `yanshi-config` + `yanshi-core` with **zero PostgreSQL dependency**.

---

## Four Surfaces

| Route | Mode | Purpose |
| --- | --- | --- |
| **`/ui/notebooks`** | **Notebook Workspace** | Multi-source research & deliverable workspace: ingest URLs, `.pdf` / `.docx` / `.xlsx` / `.pptx` / `.csv` / `.tsv` / `.json` / `.md` / `.html` documents, text notes, and ASR audio recordings into per-notebook BM25 indexes; rename notebooks inline (`PATCH`); scope active sources via checkboxes (`source_ids`); run multi-turn grounded Q&A (`POST /v1/notebooks/{id}/ask` with `history`) with verified chunk citations; launch 12 source-grounded Studio generators; and inspect or detach deliverables with interactive citation chips. |
| **`/ui/studio`** | **Visual DAG Studio & Step Debugger** | Interactive `@xyflow/react` graph editor with topological auto-layout, node palette (including `illustration`, `figure`, `render_slides`, `render_video`, `agent`, `map`/`while`), per-node breakpoints, step-by-step Input/Output inspection, inline slide/video/audio/HTML preview, and live HITL `ask_user` prompt cards. |
| **`/ui/app`** | **App Runner (Publish Mode)** | Schema-driven typed input forms generated automatically from a workflow's `inputs:` block over the 19-template [Content Studio catalogue](STUDIO_CATALOGUE.md). Runs independent stages concurrently, tracks live stage progress, surfaces `ask_user` clarification cards, and showcases downloadable `.pptx`, `.pdf`, `.mp4`, `.xlsx`, `.mp3`, and `.html` artifacts. |
| **`/ui/console`** | **Classic Run & Session Console** | Low-level run/session inspector: live SSE event timeline with `?after_seq=` reconnect, server-side `?filter=` history pre-filtering, Harness session approval cards, trace diffing, and tenant preference sync. |

### Positioning & Scope Discipline

Even with visual DAG editing and deliverable preview, `yanshi-ui` remains
focused on **authoring, running, and inspecting individual workflows and
deliverables** — intentionally **not** a multi-tenant fleet/billing BI
dashboard:

- **In scope**: visual DAG editing & YAML two-way sync, per-node breakpoints
  (`step` / `continue` / `cancel`), deliverable preview & artifact download,
  inline HITL `ask_user` cards (`GET /v1/runs/{id}/questions` +
  `POST /v1/runs/{id}/questions/{question_id}`), Harness interactive approvals,
  and single-run SSE trace inspection.
- **Out of scope**: cross-tenant billing charts, multi-day retry trend
  aggregations, and worker fleet utilization dashboards (scrape Prometheus
  `/metrics` with Grafana instead).

---

## 1. Embedded Web UI (`yanshi-server`)

Build the static bundle (writes both `dist/assets/*` and `dist/ui/assets/*`
with stable asset names `app.js` and `styles.css` consumed by
[`yanshi-server/src/ui.rs`](../yanshi-server/src/ui.rs)):

```bash
npm --prefix yanshi-ui install
npm --prefix yanshi-ui run build
cargo run -p yanshi-server
```

Then open:

- Studio debugger: `http://127.0.0.1:8080/ui/studio`
- App Runner: `http://127.0.0.1:8080/ui/app`
- Classic console: `http://127.0.0.1:8080/ui/console`

### Key Gateway Endpoints Consumed by Studio & Console

- `POST /v1/workflows:inspect` — parse YAML, validate schema, and return the
  topological graph structure + declared `inputs`.
- `POST /v1/runs` / `POST /v1/runs/{id}:cancel` / `POST /v1/runs/{id}:resume` —
  submit, cancel, or checkpoint-resume workflow runs (including debug
  breakpoint controls).
- `GET /v1/runs/{id}/events` & `GET /v1/runs/{id}/events/history?filter=<expr>` —
  live SSE stream with `?after_seq=` resume and server-side event filtering.
- `GET /v1/runs/{id}/artifacts` & `GET /v1/runs/{id}/artifacts/{artifact_id}` —
  list and stream generated `FlowValue::File` deliverables.
- `GET /v1/runs/{id}/questions` & `POST /v1/runs/{id}/questions/{question_id}` —
  list and answer inline workflow `type: agent` HITL `ask_user` prompts.
- `/v1/notebooks` (`POST`/`GET`/`PATCH`/`DELETE`), `/v1/notebooks/{id}/sources` (`POST`/`DELETE`),
  `/v1/notebooks/{id}/chunks/{chunk_id}`, `/v1/notebooks/{id}/search?q=&source_ids=`,
  `/v1/notebooks/{id}/ask` (with multi-turn `history`), `/v1/notebooks/{id}/generate`, &
  `/v1/notebooks/{id}/artifacts` (`GET`/`DELETE`) — multi-source notebook ingestion,
  `source_ids` scoping, multi-turn cited Q&A, and Studio deliverable generation
  (served identically by both `yanshi-server` and the Tauri desktop
  `local_notebooks.rs` engine).
- `GET /v1/preferences` & `PUT /v1/preferences` — durable tenant-scoped UI
  preferences.

---

## 2. Desktop App (`Yanshi Studio` — Tauri v2)

[`yanshi-ui/src-tauri/`](../yanshi-ui/src-tauri/) embeds a local Axum loopback
server (`local_engine.rs`) inside the native desktop process so users can run
and debug workflows offline without installing PostgreSQL:

- **Automatic Credential & Pricing Bootstrap**: loads `~/.yanshi/.env` and
  `~/.yanshi/models.yml`, and automatically materializes the bundled 44-model
  production pricing catalog (`config/pricing.prod.yml`) into
  `YANSHI_PRICING_TABLE` when no custom pricing table is configured.
- **Full HITL Question Parity**: `local_questions.rs` (`LocalRunQuestionHandler`)
  parks `ask_user` calls on `oneshot` channels, emits `question.pending` /
  `question.answered` SSE progress events, and serves `/v1/runs/{id}/questions`
  so interactive workflows like `backend_project_planner.yml` work identically
  offline.
- **Switchable Gateway Target**: the header "Gateway Connection" dialog lets
  you switch between the built-in local engine and any remote `yanshi-server`
  cluster at runtime.

### Build & Install on macOS

```bash
# Development mode with hot reload
cd yanshi-ui && cargo tauri dev

# Build release Yanshi Studio.app and install into /Applications
./scripts/install_desktop.sh --open

# Re-stage an existing release build into /Applications without recompiling
./scripts/install_desktop.sh --skip-build
```

---

## 3. Durable Preferences Contract

Selected non-sensitive UI preferences sync to `/v1/preferences` while keeping
`localStorage` as an instant first-paint cache:

- **Synced**: active tenant ID (`yanshi.ui.tenantId` → `ui.run-console.tenant`)
  and mapped form/filter preferences (`<= 16 KiB` JSON values, keys matching
  `^[a-zA-Z0-9_.\-:]{1,128}$`).
- **Never synced (local-only by design)**: `yanshi.ui.apiToken` (security),
  large workflow YAML drafts (`yanshi.ui.workflowDraft`), free-form user prompt
  inputs, and machine-specific host paths (`workspace_root`). In addition,
  [`yanshi-server/src/preferences.rs`](../yanshi-server/src/preferences.rs)
  enforces server-side token-shape rejection against accidental secret uploads.

---

## 4. Verification & Testing

```bash
# TypeScript typecheck + Vitest unit suite (125 tests)
npm --prefix yanshi-ui run typecheck
npm --prefix yanshi-ui test

# Embedded gateway UI route tests
cargo test -p yanshi-server ui::tests

# Tauri local engine integration tests (20-node Studio inspect, 1:1 notebooks + /ask, PPTX artifact, HITL questions, pricing)
cargo test --manifest-path yanshi-ui/src-tauri/Cargo.toml -- --test-threads=1
```

### Playwright E2E

Browser E2E specs in [`yanshi-ui/e2e/`](../yanshi-ui/e2e/README.md) run against
a live `yanshi-server` + PostgreSQL (`npm --prefix yanshi-ui run e2e` locally,
and nightly / via `workflow_dispatch` in [`.github/workflows/ui-e2e.yml`](../.github/workflows/ui-e2e.yml)).
