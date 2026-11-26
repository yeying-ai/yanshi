# CI Workflows

This document is the source of truth for what runs in GitHub Actions for
the Yanshi repository, what each job gates, and which planned jobs do
not yet exist. It is intentionally short: read the workflow YAML for the
ground truth, read this doc for the *map*.

Last audited: 2026-11-26.

## Existing workflows

| File | Trigger | Required for release gate | Purpose |
|------|---------|---------------------------|---------|
| `.github/workflows/quality.yml` | `push` to `main`/`master`, `pull_request`, `workflow_dispatch`, tag `v*` | Yes (`release-gate` job) | All 23 PR/release gates: `fmt`, `verify-edition`, `check-agent-sdk-doc`, `check-arch`, `clippy`, `clippy-lib-deny`, `redaction-lint`, `println-lint`, `check-changelog`, `ui-dx-check` (Dioxus 0.7 `fmt` + host unit tests + host/`wasm32` Clippy), `tauri-desktop-check` (`yanshi-desktop` fmt + clippy + loopback tests), `gateway-check` (`crates/gateway` Pingora fmt + clippy + tests), `cargo-audit`, `linux-sandbox-check`, `proto-validate`, `test` (incl. live Postgres), `doctest`, `rustdoc`, `features`, `examples` (incl. `--strict` across all 19 Studio generators), `rag-eval-smoke`, `agent-eval-smoke`, and `media-tools`. |
| `.github/workflows/release.yml` | `workflow_run` on `Quality` completion + `workflow_dispatch` | Triggered only when `Quality` passes on `v*` | Builds multi-arch `yanshi` CLI tarballs, pushes multi-arch `yanshi-server` GHCR image, and publishes GitHub Release artifacts. |
| `.github/workflows/docs.yml` | `push` to `main`, `pull_request`, `workflow_dispatch` | No (runs on every PR) | Builds the bilingual product landing page (`site/landing/`) + mdBook manual (`docs/`) via `python3 site/build.py` (failing on broken repo links via `site/repo_links.py`) and deploys `site/book` to GitHub Pages on `main`. |
| `.github/workflows/ui-e2e.yml` | Daily cron (10:30 UTC) + `workflow_dispatch` | No (nightly + manual) | Builds fresh Dioxus 0.7 WASM bundle (`yanshi-ui-dx/dist/public`), boots `yanshi-server` against PostgreSQL 18 + mock LLM, and runs all 4 Chromium Playwright specs (`studio`, `notebooks`, `runs-new`, `harness-sessions`). |
| `.github/workflows/bench.yml` | Weekly cron + `workflow_dispatch` | No | Runs Criterion benchmarks and checks against checked-in baselines via `cargo xtask bench-gate`. |
| `.github/workflows/llm-live.yml` | Daily cron (09:30 UTC) + `workflow_dispatch` | No — non-deterministic, costs money | Cross-provider live LLM & multimodal smoke against real vendor endpoints. |

### `quality.yml` jobs

| Job | What it gates |
|-----|---------------|
| `fmt` / `verify-edition` / `check-agent-sdk-doc` / `check-arch` | `cargo fmt --all --check`, Rust 2024 edition across workspace members, `docs/AGENT_SDK.md` symbol accuracy, and the eight L0–L4 crate dependency laws. |
| `clippy` / `clippy-lib-deny` / `redaction-lint` / `println-lint` | Workspace `-D warnings`, zero `unwrap()`/`expect()` in library crates, structured log redaction, and zero stray `println!`/`eprintln!` in library crates. |
| `check-changelog` / `cargo-audit` / `proto-validate` / `linux-sandbox-check` | Non-trivial PR `CHANGELOG.md` entry check, RustSec advisory scan (`Cargo.lock` + `yanshi-desktop/Cargo.lock`), gRPC proto check, and Linux Landlock/seccomp/cgroup v2 compilation. |
| `ui-dx-check` / `tauri-desktop-check` / `gateway-check` | `yanshi-ui-dx` `fmt --check` + host unit tests + host/`wasm32` `clippy -D warnings`, `yanshi-desktop` `fmt --check` + `clippy -D warnings` + local engine tests, and `crates/gateway` Pingora `fmt --check` + `clippy -D warnings` + unit tests. |
| `test` / `doctest` / `rustdoc` / `features` | Full workspace test suite (including PostgreSQL `yanshi-db` & `yanshi-server` tests), doc tests, `RUSTDOCFLAGS="-D warnings" cargo doc`, and curated feature matrix. |
| `examples` / `rag-eval-smoke` / `agent-eval-smoke` / `media-tools` | `cargo xtask examples-smoke`, `--strict` validation across all 19 `examples/generators/*.yml` and `examples/applications/*/workflow*.yml`, offline RAG & Agent eval baseline gates, and `ffmpeg`/`d2` media-tools tests. |
| `release-gate` | Aggregate `success` check across all 23 jobs above. PR- and release-blocking. |

### `llm-live.yml` notes

- Each provider test self-skips with a log line when its API key secret is
  empty, so the suite is safe to flip on a single provider at a time
  without rewriting the matrix.
- The job has a 15-minute wall-clock timeout; individual tests carry their
  own 30s timeout in code.
- **Flake profile:** non-deterministic by design (provider behaviour
  shifts with model versions and load). Treat alert noise as informative,
  not blocking. Never gate releases on this workflow.

## Planned workflows (not yet created)

These exist as TODOs in `TODOs.md`; the file column below is the path the
tracking task expects the new workflow to live at.

| TODO ID | File | Purpose |
|---------|------|---------|
| P3.9    | `.github/workflows/feature-matrix.yml` | Exhaustive `cargo check` matrix across feature combinations (`--no-default-features` + each feature, `--all-features`). Marks broken combinations explicitly. |
| P3.10   | `.github/workflows/examples-smoke.yml` | Runs every example in `examples/ecosystem/` and `yanshi-*/examples/` with mock providers under a 5-minute cap; fails CI on any error or panic. |
| P4.1    | `.github/workflows/rag-eval-smoke.yml` | Runs `yanshi rag eval` against `eval_datasets/ci_offline/` and asserts schema (`recall@5`, `mrr`, `ndcg@10`, `latency_ms_p50/p95`). |
| P7.2    | `.github/workflows/bench.yml` | Runs `cargo bench` on a fixed runner, compares against `benches/baselines/<host>.json`, fails when median ≥ 1.25× baseline. Posts a PR summary. |

When any of these workflows is created, update this table accordingly and
add an entry to the "Existing workflows" table above.

## Flake-prone jobs and retry policy

| Job | Risk | Mitigation |
|-----|------|------------|
| `quality.examples` | The plugin-host demo spawns a subprocess; if `cargo build` produces an artifact that fails to spawn on the runner (rare), the step errors. | No automatic retry. Re-run the job on transient failure; investigate if it recurs. |
| `llm-live.live-smoke` | Provider 429s, model deprecations, slow nights. | Per-test 30s timeout in code, 15-minute wall-clock on the job. No retry — flake here is signal, not noise. |
| `quality.test` (per crate) | A small number of tests touch the filesystem (`yanshi-tools` sandbox matrix, `yanshi-skills` install fixtures) and can collide if the runner cache is dirty. | None today. If observed, isolate via `--test-threads=1` for the offending crate. |

Other jobs (`fmt`, `clippy`, `doctest`, `features`) are deterministic and
should not flake. Any flake there is a real bug.

## Conventions

- New gating jobs go into `quality.yml` so they participate in the
  aggregate `release-gate` check. Non-blocking jobs (cost / live API /
  perf) live in their own workflow file and are intentionally excluded
  from the gate.
- Job names use lower-case kebab-case (`feature-matrix`, `rag-eval-smoke`).
- Workflows write their purpose and gating intent in a comment block at
  the top of the file, like `llm-live.yml` does. New workflow files
  should follow that pattern so a reader doesn't need to cross-reference
  this doc.
