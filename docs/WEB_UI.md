# Yanshi Studio & Web UI (`yanshi-ui-dx` + `yanshi-desktop`)

`yanshi-ui-dx/` is Yanshi's cross-platform visual workflow studio, one-click
deliverable runner, Notebook workspace, and run/Harness console — built purely
in Rust with **Dioxus 0.7 + `dioxus-shadcn 0.8`** (WebAssembly) and served under
a strict zero-eval Content Security Policy (`'wasm-unsafe-eval'`). It runs in
two deployment modes:

1. **Gateway Web UI (`/ui/*`)** — static WASM bundle built by
   `./yanshi-ui-dx/scripts/build.sh` into `yanshi-ui-dx/dist/public` (or
   `/usr/share/yanshi/ui` in Docker images) and served by `yanshi-server`
   (`yanshi-server/src/ui.rs`), requiring zero Node.js runtime or build chain in
   production.
2. **Standalone Desktop App (`Yanshi Studio` via Tauri v2)** — packaged by
   [`yanshi-desktop/`](../yanshi-desktop/) with an in-process loopback engine
   (`local_engine.rs`) that executes workflows directly against `yanshi-config` +
   `yanshi-core` with **zero PostgreSQL dependency**.

---

## Surfaces

| Route | Mode | Purpose |
| --- | --- | --- |
| **`/ui/notebooks`** | **Notebook Workspace** | Multi-source research & deliverable workspace: ingest URLs (incl. automatic `arxiv.org` PDF & `github.com` repo snapshots), `.pdf` / `.docx` / `.xlsx` / `.pptx` / `.csv` / `.tsv` / `.json` / `.md` / `.html` documents, editable text notes (`PATCH` re-chunking + `:retry`), and ASR audio recordings into per-notebook BM25 indexes; browse ordered source passages (`GET /v1/notebooks/{id}/sources/{source_id}/chunks`); rename notebooks inline (`PATCH`); scope active sources via checkboxes (`source_ids`); run persistent multi-turn grounded Q&A (`POST /v1/notebooks/{id}/ask` + `GET/DELETE /v1/notebooks/{id}/messages`) with verified chunk citations; run AI Studio content planning (`POST /v1/notebooks/{id}/plan`) or multi-select batch generation (`POST /v1/notebooks/{id}/generate-batch`) across 12 Studio generators with concurrent live tracking; and interact inline with flashcards, self-grading quizzes, sortable/filterable CSV/data tables, PDFs, or `.zip`-export deliverables. |
| **`/ui/studio`** | **Visual DAG Studio & Step Debugger** | Interactive pure-Rust SVG+HTML layered DAG graph editor with topological auto-layout, 21 built-in presets, node palette (including `illustration`, `figure`, `render_slides`, `render_video`, `agent`, `map`/`while`), per-node breakpoints, step-by-step Input/Output inspection, inline slide/video/audio/HTML preview, checkpoint `resume-plan`, and live HITL `ask_user` prompt cards. |
| **`/ui/app`** | **App Runner (Publish Mode)** | Schema-driven typed input forms generated automatically from a workflow's `inputs:` block over the 19-template [Content Studio catalogue](STUDIO_CATALOGUE.md). Runs independent stages concurrently, tracks live stage progress, surfaces `ask_user` clarification cards, and showcases downloadable `.pptx`, `.pdf`, `.mp4`, `.xlsx`, `.mp3`, and `.html` artifacts. |
| **`/ui/console`** | **Classic Run & Trace Compare Console** | Low-level run/session inspector: live SSE event timeline with `?after_seq=` reconnect, server-side `?filter=` history pre-filtering, trace diffing (`/ui/runs/:id/compare`), and tenant preference sync. |
| **`/ui/runs/new`** | **YAML Run Creator** | Direct YAML workflow submission with syntax highlighting, live validation, and SSE log stream. |
| **`/ui/harness/sessions`** | **Harness Governance UI** | Create, list, inspect, approve, answer interrupts, cancel, and resume Harness agent sessions. |
| **`/ui/diagnostics`** | **Runtime Diagnostics** | Live `/v1/diagnostics` health, provider readiness, and external tool availability panel. |

### Positioning & Scope Discipline

Even with visual DAG editing and deliverable preview, `yanshi-ui-dx` remains
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

## 1. Gateway Web UI (`yanshi-server`)

Build the Dioxus 0.7 WASM bundle (writes `yanshi-ui-dx/dist/public/`, served by
[`yanshi-server/src/ui.rs`](../yanshi-server/src/ui.rs); override location at
runtime via `YANSHI_UI_DIR`):

```bash
./yanshi-ui-dx/scripts/build.sh
cargo run -p yanshi-server
```

Then open:

- Studio debugger: `http://127.0.0.1:8080/ui/studio`
- App Runner: `http://127.0.0.1:8080/ui/app`
- Notebooks: `http://127.0.0.1:8080/ui/notebooks`
- Classic console: `http://127.0.0.1:8080/ui/console`

Legacy `/ui-dx/*` bookmarks automatically redirect (`307`) to `/ui/*` while
preserving query strings.

---

## 2. Desktop App (`Yanshi Studio` — Tauri v2)

[`yanshi-desktop/`](../yanshi-desktop/) embeds a local Axum loopback server
(`local_engine.rs`) inside the native desktop process so users can run and debug
workflows offline without installing PostgreSQL:

- **Automatic Credential & Pricing Bootstrap**: loads `~/.yanshi/.env` and
  `~/.yanshi/models.yml`, and automatically materializes the bundled 76-model
  production pricing catalog (`config/pricing.prod.yml`) into
  `YANSHI_PRICING_TABLE` when no custom pricing table is configured.
- **Strict-CSP Loopback Auth**: injects the per-session bearer token via a
  zero-inline-script `<meta name="yanshi-api-token">` tag so both Tauri
  `WKWebView` and `--web` browser mode pass strict CSP.
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
# Development mode (zero-Postgres desktop shell)
./yanshi-ui-dx/scripts/build.sh
cd yanshi-desktop && cargo tauri dev

# Zero-Postgres local web mode in browser
cargo run --manifest-path yanshi-desktop/Cargo.toml -- --web 127.0.0.1:18080

# Build release Yanshi Studio.app and install into /Applications
./scripts/install_desktop.sh --open

# Re-stage an existing release build into /Applications without recompiling
./scripts/install_desktop.sh --skip-build
```

---

## 3. Verification & Testing

```bash
# Dioxus UI formatting, unit tests, and host + wasm32 Clippy
cargo fmt --manifest-path yanshi-ui-dx/Cargo.toml -- --check
cargo test --manifest-path yanshi-ui-dx/Cargo.toml
cargo clippy --manifest-path yanshi-ui-dx/Cargo.toml --all-targets -- -D warnings
cargo clippy --manifest-path yanshi-ui-dx/Cargo.toml --target wasm32-unknown-unknown -- -D warnings

# Gateway UI route tests
cargo test -p yanshi-server ui::tests

# Tauri desktop local engine integration tests
cargo test --manifest-path yanshi-desktop/Cargo.toml --locked -- --test-threads=1
```

### Playwright E2E

Browser E2E specs in [`yanshi-ui-dx/e2e/`](../yanshi-ui-dx/e2e/README.md) run
against a live `yanshi-server` + PostgreSQL (`BASE_URL=http://127.0.0.1:8080 npm --prefix yanshi-ui-dx/e2e run e2e`
locally, and nightly / via `workflow_dispatch` in
[`.github/workflows/ui-e2e.yml`](../.github/workflows/ui-e2e.yml)).
