# Current Project Status

Last updated: 2026-11-21 (Content Studio track **Y0–Y10**, **Z**, **AA**, **AB**,
**AC**, **AD**, **AE**, **AF**, **AG**, **AH**, **AI**, and **AJ** closed, including single-machine multi-core parallel `WorkflowTool` / `Skill` / `AgentNode` / `AgentTool` execution, unified `concurrent` (`8`) DAG scheduling across CLI / Server / Desktop, shared `CostMeter` ancestor budget pool + zero-DB `yanshi workflow run-batch`, plus the daily productivity, keynote & multi-paradigm creative
studio suites — native pure-Rust OOXML `.pptx` writer, `16:9` widescreen Typst
`.pdf` export, art-directed `illustration` node, 19 Studio catalogue workflows
including `data_table`, `episodic_drama_studio`, `debate_podcast_show`, and
`worldbuilding_bible`, persistent multi-turn source-grounded Notebook Q&A
(`POST /v1/notebooks/{id}/ask` + `GET/DELETE /v1/notebooks/{id}/messages`) with
`source_ids` scoping, Notebook Studio planner (`POST /v1/notebooks/{id}/plan`)
& atomic multi-deliverable `generate-batch`, standalone Studio run attachment
(`POST /v1/notebooks/{id}/artifacts` / `yanshi notebook attach-run`), lossless
`raw_text` source rename/note edit (`PATCH`), failed source `:retry` & smart
`arxiv.org`/`openreview.net`/`github.com` & `.srt`/`.vtt` ingestion, per-source
passage reader (`GET /v1/notebooks/{id}/sources/{source_id}/chunks`), portable
Notebook bundle export/import with 100% `deliverables` restoration & one-click
`artifacts.zip` archive, pure-Rust `.docx`/`.xlsx`/`.pptx`/`.csv` + DeepSeek OCR `.png`/`.jpg`/`.webp`/`.gif` & scanned-PDF source ingestion,
interactive UI flashcards/quiz/sortable-table/audio-transcript/video-storyboard
viewers, first-class `yanshi notebook` CLI, 24-route 1:1 Tauri desktop
Notebook engine parity, and Pingora production gateway; see `RoadMap.md` and `TODOs.md`)

This is the current authoritative status entrypoint for Yanshi. Historical
evaluations, roadmap notes, and TODO queues may explain how the project arrived
here, but this document is the shortest maintained summary of what exists now
and what remains active.

## Summary

Yanshi is a multimodal content-production runtime: sources in (idea / URL /
files / recording), deliverables out (report, mind map, audio overview, slides,
flashcards, quiz, infographic, data table, video overview, domain engineering
diagrams). The contracts are in [`RFC_CONTENT_STUDIO.md`](RFC_CONTENT_STUDIO.md);
the evaluation behind the reset is
[`PROJECT_EVALUATION_2026-11-04.md`](PROJECT_EVALUATION_2026-11-04.md). It is
not a coding-agent framework.

Underneath it is a Rust workspace for deterministic DAG workflows, agent-native
runtime loops, Skills, MCP tools, RAG, memory, tracing, plugins, distributed
worker foundations, and a Web UI run console.

The current architecture is organized into five layers, with a narrow-waist
**contract kernel** (L0) extracted by the P-A track so the runtimes depend only on
shared contracts (enforced by `cargo xtask check-arch`; see
`docs/RFC_CRATE_ARCHITECTURE.md` and `docs/ARCHITECTURE.md`):

- L0 contract kernel: `yanshi-value` (`FlowValue`), `yanshi-graph`
  (the `Flow` IR), `yanshi-store-spi`, `yanshi-agent-spi`,
  `yanshi-async-util`, and `yanshi-tool` (the `Tool` contract, split out of
  `yanshi-tools` in T3.3).
- L1 execution core (the executor): `yanshi-core` runs the L0 `Flow` IR via
  the `FlowExt` trait (`flow.run()`).
- L2 capability adapters: `yanshi-nodes` (tool-tier + deterministic renderers
  including `render_slides`, pure-Rust `pptx_writer`, `render_video`,
  `render_audio`, `render_mindmap`, `render_study`, `render_table`,
  `render_svg`, `render_markdown`, `render_document`, plus `web_search`,
  `repo_snapshot`, `pdf_extract`, `video_ingest`), `yanshi-nodes-ai`
  (capability-backed node adapters: `llm`, `figure`, `illustration`, `asr`,
  `tts`, `text_to_image`, `image_to_image`, `image_edit`, `image_understand`,
  `text_to_video`, `mcp`, `rag`), `yanshi-llm`, `yanshi-tools` (builtin tool
  implementations + OS-sandbox backends), `yanshi-tools-ai` (TTS / ASR /
  image / video as `Tool`s), `yanshi-mcp`, `yanshi-rag`, `yanshi-memory`.
- L3 agent and orchestration: `yanshi-agents`, `yanshi-skills`,
  `yanshi-harness`, `yanshi-config` (shared config-first workflow assembly
  + diagnostics, consumed by both the CLI and server), `yanshi-cli`.
- L4 operations and productization: `yanshi-tracing`,
  `yanshi-server`, `yanshi-db`, `yanshi-worker`, `yanshi-ui`, plus the
  standalone Cloudflare Pingora TLS/HTTP2 reverse proxy in `crates/gateway`.

## Implemented Surfaces

- DAG workflow execution through `yanshi-core::Flow` (run via the `FlowExt` trait).
- Config-first workflow validation and execution through `yanshi-cli`.
- Agent-native runtimes through `AgentRuntime`, ReAct, Plan-Execute,
  reflection, memory, and supervisor patterns.
- Dynamic workflow: `yanshi_agents::dynamic::compile_plan_to_flow` compiles a
  declarative `WorkflowPlan` into a parallel `Flow` of tool calls, and
  `DynamicWorkflowAgent` makes the LLM planning call then compiles + executes.
  Exposed on the CLI as `yanshi workflow dynamic --goal ... --model ...`, where
  the LLM-authored plan runs against a restrictive built-in tool sandbox
  (`--allow-path` / `--allow-domain` grant access; shell is never registered),
  `--dry-run` prints the plan without executing, and `--approve` routes every tool
  call through the Harness approval pipeline.
- Harness governance shell (`yanshi-harness`): hooks, interactive approval,
  sandbox, audit, run limits, background tasks, and the `HarnessEvent` envelope.
- Skills through `SKILL.md` and `skill.toml`, including tiered
  `[[knowledge]]` retrieval (inline `files` tier or indexed `rag` tier)
  with pluggable chunking strategies — fixed-size, sentence, recursive,
  paragraph, heading, code-AST, and embedding-based `semantic` chunking
  (requires `OPENAI_API_KEY`).
- Tool abstraction through `Tool`, `ToolRegistry`, policy, permissions, and
  typed output parts.
- MCP client, server scaffolding, workflow nodes, CLI calls, and Skill tool
  integration.
- RAG search/index/eval foundations behind the `rag` feature.
- Trace persistence, replay, TUI, redaction, and OpenTelemetry mapping.
- Server run APIs, event history, SSE streaming, cancellation, and embedded Web
  UI run console.
- Subprocess plugin runtime, `plugin.toml`, workflow plugin nodes, plugin CLI,
  and marketplace schema support.
- Distributed scheduler foundation, gRPC worker protocol, worker runtime, and
  stitched worker trace events.
- Official offline-first ecosystem samples under `examples/ecosystem/`.
- Sandbox and code-execution hardening (the **S** track, closed 2026-07-24
  through 2026-07-27): file+script combination-chain and skill-script
  integrity fixes; a real Linux 6.12 VM (via Apple's `container` CLI) used to
  compile- and kernel-enforce the Landlock + cgroup v2 backend; OS sandbox now
  defaults **on** (`security.os_sandbox = true`); `code_exec` — a mandatory,
  strongly-isolated `ContainerBackend` tool for running LLM-generated Python,
  separate from the OS-sandbox tier, with zero network access, hardcoded
  resource limits, and automatic Harness approval escalation
  (`ToolIdempotency::NonIdempotent`). See `docs/RFC_LLM_CODE_EXECUTION.md`.
- Long-horizon tasks and retrieval hardening (the **L** track, closed
  2026-07-2x): replan-loop closure for stalled/failed plan steps, task-summary
  recovery so a resumed session doesn't re-derive prior progress from scratch,
  project-level memory persisted across sessions, RAG retrieval
  strengthening, and a delegation contract (`yanshi-agent-spi::delegation`
  + `aggregation`) for sub-agent hand-off with schema-validated answers and
  conflict-flagged result aggregation.

## LLM providers

The full per-provider capability matrix, `ProviderRequest` contract,
`ToolChoice` modes, `ModelCapabilities` flags, model families /
context windows, and rate-limit handling all live in
[`LLM_PROVIDERS_MATRIX.md`](LLM_PROVIDERS_MATRIX.md). That document is
the single source of truth for what each provider supports; entries
are verified by `yanshi-llm/tests/provider_consistency.rs` (offline)
and `provider_consistency_live.rs` (opt-in live).

## Stability

The v1 stability inventory lives in:

- [STABILITY.md](STABILITY.md)
- [API_COMPATIBILITY.md](API_COMPATIBILITY.md)

These documents define stable, beta, experimental, and internal surfaces for
Rust traits, manifests, trace schemas, server envelopes, and plugin/marketplace
contracts.

## Active Work

The short-term execution queue in `TODOs.md` (maintainer-local, gitignored) is
currently **empty** — every segment through **X** (2026-10-07 evaluation fixes),
**Y0–Y10** (the Content Studio reset specified by `RFC_CONTENT_STUDIO.md`),
**Z** (2026-11-18 Notebook lifecycle closure, modality SSRF/gateway hardening &
dogfooding refinements), **AA** (2026-11-19 `data_table` generator, persisted
Notebook `Deliverable` citations, skill-run admission parity &
`.docx`/`.csv`/`.tsv`/`.json` source ingestion), **AB** (2026-11-19 `source_ids`
scoping, `POST /v1/notebooks/{id}/ask` grounded Q&A, 1:1 Tauri desktop Notebook
engine parity & `yanshi notebook` CLI), **AC** (2026-11-19
`PATCH /v1/notebooks/{id}` rename, `DELETE /v1/notebooks/{id}/artifacts/{artifact_id}`,
multi-turn `/ask` `history`, pure-Rust `.xlsx`/`.pptx` OOXML source ingestion,
`ExplicitOrigins` CORS `PATCH` + `x-yanshi-tenant` fix, and `/ui/notebooks`
Playwright E2E), **AD** (2026-11-19 portable Notebook bundle export/import,
one-click `GET /v1/notebooks/{id}/artifacts.zip` with `BRIEFING.md`,
`generate.cost_limit_usd` enforcement, desktop/CLI/UI parity + live Chromium
Playwright E2E, and `yanshi-core` / `yanshi-cli` Cargo manifest dependency pruning),
**AE** (2026-11-19 `GET /v1/notebooks/{id}/sources/{source_id}/chunks` passage
reader, `POST /v1/notebooks/{id}/plan` Studio planner, `POST /v1/notebooks/{id}/generate-batch`
multi-deliverable batch generation across gateway/Tauri/CLI/UI, concurrent UI run
tracking + terminal-status gallery sync, and 7/7 live Chromium Playwright E2E closure),
**AF** (2026-11-19 19-generator contract & `export_pdf=false` fixes, inline SVG
CSP & `?inline=1` artifact parity, `cleanup.rs` Notebook deliverable retention
protection, persistent `notebook_messages` (`0016`), source `PATCH` rename/text
re-chunking & `:retry`, smart `arxiv.org`/`github.com` URL ingestion, interactive
UI flashcards/quiz/sortable-table viewers & custom-tenant preview auth, and CI
`docs.yml`/`quality.yml`/`ui-e2e.yml` hardening), and **AG** (2026-11-19 UTF-8
CJK slice-panic elimination, runtime media-source `allowed_path_roots` confinement
& `video_ingest` trusted-host gate, `:resume` model/budget/Notebook context
persistence (`0017`) & idempotent artifact registration, portable `deliverables`
restoration on `import`, lossless `raw_text` note editing, 24-route Studio→Notebook
`POST /v1/notebooks/{id}/artifacts` + `yanshi notebook attach-run`, `workflow run --server`
parity, interactive UI audio-transcript/video-storyboard viewers, and CI `dist/`
freshness + `gateway-check` gates)
has closed:

- **Y0–Y2** — `RFC_CONTENT_STUDIO.md`, first-class `FlowValue::File` artifacts
  with DB registration + `/v1/runs/{id}/artifacts` streaming download + CLI
  `--output-dir`, streaming token `usage`, modality `CostMeter` +
  `cost_limit_usd`, and DAG checkpoint/resume across CLI and server.
- **Y3** — embedded pure-Rust renderers (`render_table`, `render_svg`,
  `render_markdown`, `render_mindmap`, `render_study`, `render_slides` +
  native OOXML `pptx_writer`), sandboxed media/document renderers
  (`render_audio`, `render_video`, `render_document` over pandoc + typst),
  `figure` node (`svg` / `d2` / `tikz` / `typst` / `python` container with
  domain checks + compile-error feedback loop), and `illustration` node
  (art-directed prompt refinement, 6 visual style presets, dark/light palette
  harmonization, bounded concurrent `SlideDeck` / batch generation).
- **Y4–Y6** — 19 Studio catalogue workflows under `examples/generators/`
  (`illustrated_slides`, `tech_slides_video`, `episodic_drama_studio`,
  `debate_podcast_show`, `worldbuilding_bible`, `slides`, `video_overview`,
  `report`, `infographic`, `data_table`, `mindmap`, `circuit_diagram`,
  `audio_overview`, `flashcards`, `quiz`, `repo_architecture`,
  `backend_project_planner`, `paper_reader`, `video_summarizer`),
  `examples/applications/content-studio/`,
  persistent `notebooks` / `sources` / `source_chunks` / `notebook_artifacts` /
  `notebook_messages` (with persisted `deliverable JSONB`, interactive citation
  lookup, and `.pdf` / `.docx` / `.xlsx` / `.pptx` / `.csv` / `.tsv` / `.json` /
  `.html` / `.md` / ASR `recording` ingestion) and `POST /v1/notebooks/{id}/generate`, plus inline
  workflow `agent` HITL `ask_user` interrupt/resume and gateway-safe inline
  `multi_agent` (`blackboard` / `debate` / `handoff`) in gateway and Studio UI.
- **Y7–Y10 & Z** — cost-budget approvals, `ToolEffect::Publish`, gateway reach
  ceilings (including modality media sources and runtime `CreateRunRequest.inputs`),
  pinned-DNS SSRF filtering on remote modality URLs, Notebook/source `DELETE`
  lifecycle with immediate BM25 cache invalidation + `GET /v1/notebooks/{id}/chunks/{chunk_id}`
  citation lookup + focus-ranked multi-source chunk selection, `EntityFactStore`
  `&self` promotion to `yanshi-store-spi`, unified CLI `--input` syntax +
  `FlowStateExt`, streaming thinking & reply images, Anthropic adaptive thinking
  + native structured output, and live multimodal provider verification.

Tracks that this product does not need yet (distributed workers, remaining
multi-replica gaps, coding-session Harness providers, seccomp refinement, MCP
follow-ups) are listed in `RoadMap.md` § Deferred, each with the condition
that reopens it.

The previously closed **V (2026-08-05 production-readiness remediation)**
segment came from findings by six independent parallel sub-agents that each
read one architecture layer, cross-checked by the orchestrator running
`cargo test/clippy/fmt/tree` locally:

- **V0** — blocking correctness/security bugs (panics, unsafe defaults,
  terminal-state bugs, path traversal, an auth gap).
- **V1** — core execution/agent robustness (DAG terminal-state + resume
  semantics, cancellation races, reliability-stack wiring, LLM retry,
  memory runtime integration, log normalization).
- **V2** — structured output, token-level streaming, a generic HITL
  interrupt/resume protocol (`ask_user`), and agent-loop checkpointing.
- **V3** — fail-closed defaults on non-loopback binds, execution-side
  sandbox enforcement for DAG `shell` nodes, two SSRF gaps closed in
  `HttpTool`, per-tenant run admission control, supply-chain tightening
  (Ed25519 marketplace/plugin signatures, bounded MCP stdio reads), and an
  `expr` parser recursion-depth limit.
- **V4** — `AGENTS.md` regenerated from current sources, `yanshi-cli`'s
  2700-line `main.rs` split into per-domain `commands/*/cli.rs` modules,
  `cargo-audit` wired into CI (11 of 13 pre-existing CVEs resolved via
  targeted dependency bumps), and completeness follow-ups: an
  `input_mapping` whitespace-parsing bug, a Gemini `base_url` bug, DB-free
  `~/.yanshi/runs` retention, and (as a follow-up once the rest of V4
  closed) `chunk_strategy = "semantic"` now reachable from a skill's
  `[[knowledge]]` manifest entries, backed by `yanshi-rag`'s
  embedding-based `SemanticChunker`.

The ongoing documentation-convergence convention:

- keep this file as the current status entrypoint;
- keep `RoadMap.md` focused on future direction;
- keep `TODOs.md` focused on short-term execution;
- leave historical evaluations marked as historical references.

## Historical References

- [`PROJECT_EVALUATION_2026-11-04.md`](PROJECT_EVALUATION_2026-11-04.md):
  most recent evaluation — closure of the 10-07 findings (36 fixed / 13
  partial / 17 open), new findings, then re-weighted for the
  content-production goal (A− library / B− platform); drove the Y track.
- [`PROJECT_EVALUATION_2026-10-07.md`](PROJECT_EVALUATION_2026-10-07.md):
  evaluation that drove the X segment (gateway policy, cross-replica fixes).
- [`PROJECT_EVALUATION_2026-08-09.md`](PROJECT_EVALUATION_2026-08-09.md):
  evaluation that drove the W segment; the baseline of
  [`TRIAL_GUIDE.md`](TRIAL_GUIDE.md).
- [`PROJECT_EVALUATION_2026-08-05.md`](PROJECT_EVALUATION_2026-08-05.md):
  6 parallel layer sub-agents + orchestrator cross-check, B+/A- composite,
  production-readiness C+; drove the V segment above.
- [`PROJECT_EVALUATION_2026-05-19.md`](archive/PROJECT_EVALUATION_2026-05-19.md):
  prior module-by-module evaluation (A overall, v1.0.0-rc.1 candidate).
- [`PROJECT_EVALUATION_2026-05-14.md`](archive/PROJECT_EVALUATION_2026-05-14.md):
  prior evaluation that informed the P6/P7/P-H/M segment additions.
- [`PROJECT_EVALUATION_2026-05-01.md`](archive/PROJECT_EVALUATION_2026-05-01.md):
  historical module-by-module evaluation that informed the P0-P4 task queue.
- [`RoadMap.md`](../RoadMap.md): roadmap and future direction.
- `TODOs.md` (maintainer-local, gitignored): active execution queue and task completion record.
