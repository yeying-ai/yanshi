# Yanshi Studio Dioxus UI Playwright E2E

Browser smoke & lifecycle specs for `yanshi-ui-dx` (Dioxus 0.7 WASM) served under strict zero-eval CSP at `/ui/*`.

## Specs

- `studio.spec.ts` — `/` → `/ui/studio` & `/ui-dx/*` → `/ui/*` redirects, strict CSP verification (zero violations), DAG canvas rendering, preset switching, `/ui/app`, and `/ui/diagnostics`.
- `notebooks.spec.ts` — `/ui/notebooks` lifecycle: create notebook, rename inline, ingest pasted source, browse chunk reader, BM25 search, lossless `raw_text` source edit, delete notebook.
- `runs-new.spec.ts` — `/ui/runs/new` workflow submission, live run ID receipt, and API token non-persistence.
- `harness-sessions.spec.ts` — `/ui/harness/sessions/new` submission → detail view, and `/ui/harness/sessions` tenant-scoped list navigation.

## Local Usage

```bash
# 1. Build the Dioxus 0.7 WASM bundle
./yanshi-ui-dx/scripts/build.sh

# 2. Start either yanshi-server (Postgres) or zero-DB yanshi-studio --web
cargo run --manifest-path yanshi-desktop/Cargo.toml -- --web 127.0.0.1:18080

# 3. Install Playwright once and run the suite
cd yanshi-ui-dx/e2e
npm install
npx playwright install chromium
BASE_URL=http://127.0.0.1:18080 npm run e2e
```

CI runs this suite nightly and via `workflow_dispatch` in [`.github/workflows/ui-e2e.yml`](../../.github/workflows/ui-e2e.yml).
