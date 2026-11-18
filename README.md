# Yanshi 偃师

[![Rust](https://img.shields.io/badge/rust-1.85%2B%20(2024)-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](Cargo.toml)
[![Website](https://img.shields.io/badge/website-yanshi.dev-0f6e66.svg)](https://yanshi.dev/en/)
[![Documentation](https://img.shields.io/badge/docs-manual-green.svg)](https://yanshi.dev/docs/)

**English** | **[简体中文](README_zh.md)** · **[Product Website](https://yanshi.dev/en/)** · **[Online Manual](https://yanshi.dev/docs/)** · **[18 Studio Workflows](docs/STUDIO_CATALOGUE.md)** · **[Quick Trial Guide](docs/TRIAL_GUIDE.md)**

> **A multimodal content-production runtime in Rust: sources in (an idea, a URL, a repository, academic PDFs, or audio/video recordings), finished deliverables out (editable `.pptx` slides, narrated `1080p` videos, `16:9` Typst `.pdf` decks, Excel tables, podcasts, infographics, and validated architecture/circuit diagrams) — built on deterministic DAG workflows, agent-native loops, Skills, MCP tools, RAG, tracing, and checkpointed recovery.**

**About the name.** In the *Liezi* (《列子·汤问》), the artificer Yanshi (偃师) presents King Mu of Zhou with a figure that sings, dances, and acts on its own — and, taken apart, turns out to be nothing but leather, wood, glue, and lacquer assembled by explicit structure, which "works again as before" once put back together. It is the oldest Chinese account of a built autonomous agent, and the principle behind this project: autonomous capabilities assembled from explicit, inspectable contracts (`Flow` graphs, `Tool` traits, `Skill` manifests) that can be traced, replayed, and recomposed.

**Direction (Content Studio Y0–Y10 closed).** Yanshi turns raw sources into finished, downloadable artifacts through a predictable five-stage pipeline — **Sources → Planner → Generators → Renderers → Artifacts** — paired with an 18-template built-in catalogue ([`docs/STUDIO_CATALOGUE.md`](docs/STUDIO_CATALOGUE.md)) and the cross-platform **`Yanshi Studio`** Web & Tauri v2 desktop app. LLMs plan the *content* (outlines, citations, slide visual routing, diagram source), while hand-authored, schema-validated DAG workflows and multi-agent supervisors execute deterministically:

- **First-class media & artifacts**: `FlowValue::File` carries typed files across nodes with SHA-256 manifests, CLI `--output-dir` export, and streaming gateway downloads (`/v1/runs/{id}/artifacts`).
- **Native editable slides & typesetting**: Pure-Rust OOXML `pptx_writer` emits genuinely editable PowerPoint shapes, `<a:tbl>` tables, `<p:cxnSp>` connectors, and speaker notes (not flat raster screenshots), alongside `16:9` widescreen Typst `.pdf` and interactive `.html` decks.
- **Sandboxed deterministic renderers**: Embedded pure-Rust writers (`render_slides`, `render_table`, `render_svg`, `render_mindmap`, `render_study`, `render_markdown`) work alongside OS-sandboxed external tools (`typst`, `pandoc`, `ffmpeg`, `d2`, `lualatex`, and `ContainerBackend`-isolated Python).
- **Explicit cost & fault governance**: Per-run `cost_limit_usd` caps combined LLM + image + TTS + ASR + video spend, while `--checkpoint` / `workflow resume` resumes interrupted pipelines without re-billing completed nodes.

For the full design rationale, see [`docs/RFC_CONTENT_STUDIO.md`](docs/RFC_CONTENT_STUDIO.md) and [`RoadMap.md`](RoadMap.md). For the maintained snapshot of implemented surfaces and stability tiers, see [`docs/CURRENT_STATUS.md`](docs/CURRENT_STATUS.md) and [`docs/LLM_PROVIDERS_MATRIX.md`](docs/LLM_PROVIDERS_MATRIX.md). *(Yanshi is a content-production and workflow/agent runtime, not a coding-agent IDE wrapper.)*

---

## 🎬 Content Studio Catalogue (18 Built-in Generators)

Every workflow under [`examples/generators/`](examples/generators/) passes `yanshi workflow validate --strict` and runs identically across the CLI, HTTP gateway (`/v1/runs`), Web Studio (`/ui/studio` & `/ui/app`), and `Yanshi Studio` desktop app.

### 1. Keynote Slides, Video & Audio

| Workflow | Inputs | Deliverables | Highlights |
| --- | --- | --- | --- |
| [`illustrated_slides.yml`](examples/generators/illustrated_slides.yml) | `subject`, `audience`, `slide_count`, `slide_theme`, `style_preset` | `.pptx`, `16:9 .pdf`, `.html` | Web grounding + 3-way slide visual routing (`illustration` 6 art presets, D2 `figure`, native OOXML tables/code) |
| [`tech_slides_video.yml`](examples/generators/tech_slides_video.yml) | `topic`, `repo_url`, `pdf_source`, `presenter_mode` | `1080p .mp4`, `.srt`/`.vtt`, `.pptx`, `.pdf` | Progressive step-by-step slide builds synced millisecond-accurately to TTS, spotlight highlights & PiP presenter mode |
| [`slides.yml`](examples/generators/slides.yml) | `url` / `text` / `chunks`, `focus`, `enable_illustrations` | `.pptx`, `16:9 .pdf`, `.html` | Source-grounded presentation with self-repairing vector `figure` diagrams (runs with zero image API keys by default) |
| [`video_overview.yml`](examples/generators/video_overview.yml) | `url` / `text`, `image_model`, `tts_model`, `video_model` | `.mp4`, `.srt` | Parallel scene illustration or `text_to_video` motion shots + per-scene `tts` + `render_video` subtitle muxing |
| [`audio_overview.yml`](examples/generators/audio_overview.yml) | `url` / `text`, `minutes`, `host_voice`, `guest_voice` | `.mp3` | Two-speaker podcast dialogue voiced in parallel and loudness-normalised to EBU R128 via `render_audio` |

### 2. Daily Engineering & Research Workflows

| Workflow | Inputs | Deliverables | Highlights |
| --- | --- | --- | --- |
| [`repo_architecture.yml`](examples/generators/repo_architecture.yml) | `source` (GitHub URL or local path), `git_ref`, `focus` | `2x .svg`/`.png`, `.pdf`, `.html`, `.md` | `repo_snapshot` inspects manifests, directory trees, and internal imports without executing untrusted repo code |
| [`backend_project_planner.yml`](examples/generators/backend_project_planner.yml) | `idea`, `tech_preference`, `interactive`, `max_questions` | `6x .svg`, `.sql` DDL, `.pdf` spec | Multi-turn HITL `ask_user` clarification pauses the run in Studio/CLI before compiling architecture, UML, ER & DDL |
| [`paper_reader.yml`](examples/generators/paper_reader.yml) | `source` (arXiv URL, PDF URL, or local PDF), `target_lang` | `.pdf`, `.xlsx`/`.csv`, `.svg`, `.html` | Bilingual research dossier + numeric experimental benchmark extraction to Excel + D2 method diagram & mindmap |
| [`video_summarizer.yml`](examples/generators/video_summarizer.yml) | `source` (video URL or local media), `subtitle_langs` | `.pdf`, `.html`, `.srt`, `.svg` | `video_ingest` prefers native captions and automatically falls back to FFmpeg audio extraction + `asr` transcription |

### 3. Sourced Reports, Diagrams & Study Kits

| Workflow | Inputs | Deliverables | Highlights |
| --- | --- | --- | --- |
| [`report.yml`](examples/generators/report.yml) | `url` / `text` / `chunks`, `focus`, `sections`, `pdf` | `.pdf`, `.html`, `.md` | Parallel `map` section drafting where every claim carries verbatim chunk-level footnote citations |
| [`infographic.yml`](examples/generators/infographic.yml) | `url` / `text` / `chunks`, `focus` | `.svg`, `.png` | Deterministic Tera SVG poster engine driven by schema-validated headline metrics and comparisons |
| [`mindmap.yml`](examples/generators/mindmap.yml) | `url` / `text` / `chunks`, `depth` | `.svg`, `.png`, `.html` | Hierarchical concept tree rendered to crisp vector SVG/PNG and interactive offline markmap HTML |
| [`circuit_diagram.yml`](examples/generators/circuit_diagram.yml) | `description`, `text` | `.svg`, `.png` | Extracts a typed parts manifest first, verifies component topology in `circuitikz`, then compiles via `lualatex` |
| [`flashcards.yml`](examples/generators/flashcards.yml) | `url` / `text` / `chunks`, `count` | `.html`, `.json` | Self-contained offline flip-card study deck rendered by embedded `render_study` |
| [`quiz.yml`](examples/generators/quiz.yml) | `url` / `text` / `chunks`, `count` | `.html`, `.json` | Self-grading multiple-choice assessment with per-option explanations and live score tally |

### 4. Multi-Agent Creative & Deliberative Studios

| Workflow | Inputs | Deliverables | Highlights |
| --- | --- | --- | --- |
| [`episodic_drama_studio.yml`](examples/generators/episodic_drama_studio.yml) | `premise`, `episode_count`, `existing_cast_sheet`, `existing_series_bible` | `1080p .mp4`, `.srt`, `.pdf` | `multi_agent (blackboard)` writers' room + `while` + `image_understand` cast-sheet critic + cross-episode `image_to_image` / I2V face lock & pinned character voices |
| [`debate_podcast_show.yml`](examples/generators/debate_podcast_show.yml) | `motion_topic`, `pro_position`, `con_position`, `turn_count` | `3-voice .mp3`, `.png`, `.pdf` | Genuine multi-round clash via `multi_agent (debate)` (`pro-advocate` vs `con-challenger` + `moderator` judge) normalized to EBU R128 MP3 |
| [`worldbuilding_bible.yml`](examples/generators/worldbuilding_bible.yml) | `seed_idea`, `medium_target`, `art_direction`, `max_art_rounds` | `.pdf`, `.html`, `2x .svg`, `.png` | `multi_agent (handoff)` specialist relay (`lore_architect` → `faction_smith` → `quest_designer`) + `while` VLM key-art polish + D2 faction & quest diagrams |

### Three-Way Slide Visual Routing

Slide generators (`illustrated_slides.yml`, `slides.yml`, `tech_slides_video.yml`) automatically route each slide to the right rendering engine:
1. **`illustration`** — conceptual, cover, analogy, and summary slides use [`IllustrationNode`](yanshi-nodes-ai/src/nodes/illustration.rs) with LLM art-director prompt refinement, canvas palette harmonization (`dark` `#0B1120` / `light` `#F8FAFC`), anti-garbled-text guardrails, and 6 style presets (`editorial_tech`, `isometric_3d`, `flat_vector`, `blueprint_schematic`, `scientific_diagram`, `cinematic_keynote`).
2. **`arch_diagram`** — system topology and dataflow slides compile verified vector SVG via `figure` (`d2` / `svg` / `typst` / `tikz` / sandboxed `python`) with automatic compile-error feedback repair.
3. **`structured_panel`** — code walkthroughs, benchmark matrices, and checklists emit native editable PowerPoint shapes, tables, connectors, and monospace callouts directly via pure-Rust OOXML.

---

## 🚀 Content Studio Quickstart (CLI · Web · Desktop)

```bash
# 1. Illustrated Keynote Deck -> native editable .pptx + 16:9 .pdf + interactive .html
yanshi workflow run examples/generators/illustrated_slides.yml \
  --input subject "Rust Async Runtime & Work-Stealing Scheduler" \
  --input style_preset editorial_tech \
  --output-dir out/slides

# 2. Academic Paper Reader -> bilingual .pdf + extracted .xlsx tables + D2 method diagram
yanshi workflow run examples/generators/paper_reader.yml \
  --input source "https://arxiv.org/abs/1706.03762" \
  --output-dir out/paper

# 3. GitHub Repository Architecture -> D2 system & module dependency SVGs + PDF report
yanshi workflow run examples/generators/repo_architecture.yml \
  --input source "https://github.com/tokio-rs/axum" \
  --output-dir out/axum-arch

# 4. Fault-tolerant narrated video run with per-node checkpoints (resumable on interruption)
yanshi workflow run examples/generators/tech_slides_video.yml \
  --checkpoint --timeout 15m \
  --input topic "Zero-Copy Serialization in Rust" \
  --output-dir out/video

# 5. Launch Web Studio (/ui/studio & /ui/app) or build & install Yanshi Studio.app (macOS)
cargo run -p yanshi-server
./scripts/install_desktop.sh --open
```

---

## 🏗️ Five-Layer Narrow-Waist Architecture

Yanshi's 25-crate Rust workspace + Web/Tauri UI is organized into five strict layers around an **L0 Contract Kernel**, enforced in CI by `cargo xtask check-arch` (eight dependency laws — runtimes never depend on each other, only on shared contracts; see [`docs/RFC_CRATE_ARCHITECTURE.md`](docs/RFC_CRATE_ARCHITECTURE.md)):

| Layer | Role | Crates |
| --- | --- | --- |
| **L0 Contract Kernel** | Narrow waist — shared types & traits only, zero runtime logic | `yanshi-value` (`FlowValue`), `yanshi-graph` (`Flow` IR, `AsyncNode`, `expr`, `YanshiError`), `yanshi-store-spi` (`MemoryStore`, `KnowledgeBackend`), `yanshi-agent-spi` (`AgentRuntime`, `Capability`, HITL interrupt/resume), `yanshi-async-util` (retry, timeout, `race_with_limits`), `yanshi-tool` (`Tool`, `ToolRegistry`, `ToolPolicy`, `SecurityProfile`, `SandboxBackend`) |
| **L1 Execution Core** | Deterministic DAG executor for the L0 `Flow` IR | `yanshi-core` (dependency-ready concurrent scheduler, `FlowExt::run()`, checkpoints, resume plans, `ResourceLimits`, health, events) |
| **L2 Capability Adapters** | Tool-tier & capability-backed nodes, 6 LLM providers, RAG, MCP | `yanshi-nodes` (deterministic Studio renderers & source ingestion), `yanshi-nodes-ai` (`llm`, `figure`, `illustration`, `asr`, `tts`, image/video, `mcp`, `rag`), `yanshi-llm` (OpenAI, Anthropic, Google, Moonshot, StepFun, Mock), `yanshi-tools` (builtin tools + OS sandboxes), `yanshi-tools-ai`, `yanshi-mcp`, `yanshi-rag`, `yanshi-memory` |
| **L3 Agent & Orchestration** | Agent loops, Skills, Harness governance, YAML config & CLI | `yanshi-agents` (ReAct, Plan-Execute, Reflection, Handoff/Blackboard/Debate supervisors, `dynamic` plan-to-Flow), `yanshi-skills`, `yanshi-harness` (hooks, approval, audit, background tasks), `yanshi-config`, `yanshi-cli` |
| **L4 Operations & Product** | Observability, HTTP gateway, distributed workers, Web & Desktop UI | `yanshi-tracing` (JSONL, replay TUI, W3C `traceparent`, OTLP/HTTP), `yanshi-server` (Axum API + SSE), `yanshi-db` (Postgres 9-repo store), `yanshi-worker` + `yanshi-worker-proto` (gRPC), `yanshi-ui` (React/Vite + Tauri v2), `crates/gateway` (Cloudflare Pingora TLS/HTTP2 proxy) |

---

## 🤖 Four Execution Paradigms

All workloads run on one unified capability substrate (`ToolRegistry`, Skills, MCP, RAG, Memory) across four points on the *binding-time* spectrum:

- **Static DAG (`yanshi-core::Flow`)** — plan authored up front in YAML or Rust; concurrent dependency-ready scheduling, `run_if`, `map`, `while`, retries, and checkpointed recovery.
- **Native Agent Loop (`yanshi-agents::ReActAgent`)** — plan decided step-by-step with native tool calling, reflection, verification-before-stop, token/cost guards, and HITL `ask_user` interrupts.
- **Dynamic Workflow (`yanshi_agents::dynamic`)** — an LLM authors a declarative `WorkflowPlan` once at runtime, which compiles into a validated `Flow` (`compile_plan_to_flow` / `yanshi workflow dynamic`) and executes in parallel against a restricted sandbox.
- **Harness Governance (`yanshi-harness`)** — wraps any runtime with pre/post hooks, interactive or remote approvals, OS-level sandboxing, audit logs, and background task limits.

The paradigms compose in both directions:
- **`AgentNode`** embeds an autonomous agent *inside* a DAG workflow step.
- **`WorkflowTool`** exposes an entire DAG workflow *as* a callable tool to an agent.

See [`docs/ARCHITECTURE.md` § Four Execution Paradigms](docs/ARCHITECTURE.md#four-execution-paradigms--mental-model) for the full mental model.

---

## 🎯 Two Ways to Build

### 📋 Configuration-First (Declarative YAML + CLI)

```yaml
# workflow_v2.yml
name: "Source-Grounded Brief"
cost_limit_usd: 0.50
nodes:
  - id: build_prompt
    type: template
    parameters:
      template: "Summarize core trade-offs of {{ topic }} in 3 bullet points."
      topic: "work-stealing schedulers"

  - id: draft_brief
    type: llm
    dependencies: ["build_prompt"]
    input_mapping:
      prompt: "{{ nodes.build_prompt.outputs.output }}"

  - id: render_md
    type: render_markdown
    dependencies: ["draft_brief"]
    run_if: "{{ nodes.draft_brief.outputs.output != '' }}"
    input_mapping:
      content: "{{ nodes.draft_brief.outputs.output }}"
```

```bash
yanshi workflow validate workflow_v2.yml --strict
yanshi workflow run workflow_v2.yml --output-dir out/brief
```

### 💻 Code-First (Rust Core SDK)

```rust
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use yanshi_core::{
  FlowExt, // brings `flow.run()` into scope for the L0 `Flow` IR
  flow::{Flow, GraphNode, NodeType},
  value::FlowValue,
};
use yanshi_nodes::nodes::template::TemplateNode;
use yanshi_nodes_ai::nodes::llm::LlmNode;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let template_node = GraphNode {
    id: "get_topic".to_string(),
    node_type: NodeType::Standard(Arc::new(TemplateNode::new(
      "get_topic",
      "Write a concise technical haiku about {{topic}}.",
    ))),
    dependencies: vec![],
    input_mapping: None,
    run_if: None,
    initial_inputs: HashMap::from([(
      "topic".to_string(),
      FlowValue::Json(json!("deterministic DAGs")),
    )]),
  };

  let llm_node = GraphNode {
    id: "generate_haiku".to_string(),
    node_type: NodeType::Standard(Arc::new(LlmNode::default())),
    dependencies: vec!["get_topic".to_string()],
    input_mapping: Some(HashMap::from([(
      "prompt".to_string(),
      ("get_topic".to_string(), "output".to_string()),
    )])),
    run_if: None,
    initial_inputs: HashMap::new(),
  };

  let flow = Flow::new(vec![template_node, llm_node]);
  let final_state = flow.run().await?;

  if let Some(Ok(outputs)) = final_state.get("generate_haiku")
    && let Some(FlowValue::Json(serde_json::Value::String(text))) = outputs.get("output")
  {
    println!("{text}");
  }
  Ok(())
}
```

---

## 🛡️ Production Reliability, Security & Observability

### 🔄 Retry & Status-Aware Backoff
Billed LLM and multimodal nodes classify vendor errors by HTTP status (`429`, `5xx`, and network timeouts retry with backoff; `4xx` validation/content rejections fail fast without double-billing):

```rust
use yanshi_core::{RetryPolicy, RetryStrategy, execute_with_retry};

let policy = RetryPolicy::builder()
  .max_attempts(3)
  .strategy(RetryStrategy::exponential_backoff(100, 5000, 2.0))
  .build();

let data = execute_with_retry(&policy, "fetch_source", || async {
  client.fetch().await
})
.await?;
```

### 💾 Advisory Resource Budgets & Timeout Control
`ResourceLimits` integrates with `FlowExecutionConfig::resource_limits` to emit structured `WorkflowEvent::ResourceWarning` alerts without unsafe state-pool eviction, paired with `TimeoutConfig` deadlines:

```rust
use yanshi_core::resource_limits::ResourceLimits;
use yanshi_core::timeout::{TimeoutConfig, with_timeout};

let limits = ResourceLimits::builder()
  .max_state_size(100 * 1024 * 1024) // 100 MiB state pool warning threshold
  .max_value_size(10 * 1024 * 1024)  // 10 MiB per-node value threshold
  .build();

let timeouts = TimeoutConfig::production();
let output = with_timeout(run_pipeline(), timeouts.workflow_execution_timeout).await?;
```

### 🔁 Checkpointed Recovery & Workflow Debugging
Inspect, validate, dry-run, checkpoint, and replay runs from the CLI:

```bash
# Strict schema validation, DAG structure visualization, and dry-run inspection
yanshi workflow validate workflow.yml --strict
yanshi workflow debug workflow.yml --visualize
yanshi workflow debug workflow.yml --dry-run --verbose

# Resume an interrupted checkpointed run while preserving prior spend & completed nodes
yanshi workflow resume <run_id> examples/generators/tech_slides_video.yml --output-dir out/video

# Replay execution traces interactively in the terminal TUI
yanshi trace replay <run_id>
```

### 🔒 Defense-in-Depth Sandbox & Security Profiles
- **`SecurityProfile` (`Dev` / `Local` / `Production`)**: Fail-closed defaults in `Production` reject unauthenticated non-loopback gateway binds, unsandboxed host-reaching nodes, and unsigned marketplace/plugin packages ([`docs/SECURITY_PROFILES.md`](docs/SECURITY_PROFILES.md)).
- **OS Sandbox & Container Isolation**: Built-in OS sandboxing defaults **on** (`sandbox-exec` on macOS; `seccomp` + `Landlock` + `cgroup v2` on Linux). LLM-authored Python (`code_exec` and Python `figure` scripts) runs inside a mandatory `ContainerBackend` with zero network access and hard memory/CPU/PID caps.
- **SSRF-Safe Networking**: `HttpTool` normalizes IPv4-mapped IPv6 addresses and pins DNS resolution per request to close DNS-rebinding TOCTOU windows.

---

## 📦 Installation & Quality Gates

### Install CLI or Desktop Studio
```bash
# Install the `yanshi` CLI binary from source
cargo install --path yanshi-cli
yanshi doctor

# Build & install macOS `Yanshi Studio.app` (bundles Web UI + local Tauri engine)
./scripts/install_desktop.sh --open
```

### Embed as Rust SDK Crates
```toml
[dependencies]
yanshi-core = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-nodes = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-nodes-ai = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-agents = { git = "https://github.com/yeying-ai/yanshi" }
```

### Workspace Verification Gates
```bash
cargo fmt --check
cargo clippy --workspace --all-features -- -D warnings
cargo xtask check-arch           # Enforces the 8 narrow-waist crate dependency laws
cargo xtask println-lint         # Ensures zero stray println!/eprintln! in library crates
cargo xtask redaction-lint       # Audits structured logs against raw prompt/secret leakage
cargo test --workspace --all-features
```

---

## 📚 Documentation

### Entry Points & Product Guides
- **[Product Website (`yanshi.dev`)](https://yanshi.dev/en/)** & **[Chinese Home (`yanshi.dev`)](https://yanshi.dev/)**: Interactive workbench, YAML workflow inspector, and visual routing showcase.
- **[Online Manual (`yanshi.dev/docs/`)](https://yanshi.dev/docs/)** & **[Docs Index](docs/README.md)**: Full mdBook documentation.
- **[Content Studio Catalogue & Desktop](docs/STUDIO_CATALOGUE.md)**: All 15 built-in generators, slide visual routing presets, Web Studio, and `Yanshi Studio` desktop app.
- **[Trial Guide (中文试用手册)](docs/TRIAL_GUIDE.md)** & **[Runnable Tutorials](docs/examples/runnable_tutorials_zh.md)**: Step-by-step hands-on walkthroughs from zero API keys to full Studio pipelines.
- **[Current Project Status](docs/CURRENT_STATUS.md)** & **[RoadMap](RoadMap.md)**: Authoritative snapshot of closed tracks (`Y0–Y10`) and future re-evaluation triggers.

### Architecture, Contracts & Operations
- **[Content Studio RFC](docs/RFC_CONTENT_STUDIO.md)**: Five-stage pipeline, `FlowValue::File` artifact contract, citation model, and renderer architecture.
- **[Architecture](docs/ARCHITECTURE.md)** & **[Crate Architecture RFC](docs/RFC_CRATE_ARCHITECTURE.md)**: Five-layer workspace layout, four execution paradigms, and L0 contract kernel.
- **[Configuration](docs/CONFIGURATION.md)** & **[Workflow Schema](docs/WORKFLOW_SCHEMA.md)**: Model config, `~/.yanshi/.env`, strict YAML validation, and artifact directories.
- **[LLM Providers Matrix](docs/LLM_PROVIDERS_MATRIX.md)**: Per-provider capabilities across OpenAI, Anthropic, Google, Moonshot, StepFun, and Mock.
- **[Agent Runtime](docs/AGENT_RUNTIME.md)** & **[Skills Guide](docs/SKILLS.md)**: ReAct / Plan-Execute loops, HITL interrupts, `SKILL.md` packaging, tiered RAG knowledge, and MCP integration.
- **[Security Profiles](docs/SECURITY_PROFILES.md)** & **[Deployment Guide](docs/DEPLOYMENT.md)**: OS sandbox, gateway policy, Docker/Helm multi-replica Postgres deployment, and Pingora TLS gateway.
- **[Stability & API Compatibility](docs/STABILITY.md)**: Stable, beta, experimental, and internal surface guarantees.

### Selected Runnable Examples ([Full Matrix](examples/README.md))
- [`examples/generators/`](examples/generators/): All 15 production Content Studio YAML generators.
- [`examples/applications/content-studio/`](examples/applications/content-studio/): Multi-deliverable Planner Skill orchestrating Studio workflows.
- [`yanshi-core/examples/fixed_dag_workflow.rs`](yanshi-core/examples/fixed_dag_workflow.rs) & [`checkpoint_recovery.rs`](yanshi-core/examples/checkpoint_recovery.rs): Deterministic DAG execution and checkpoint resume.
- [`yanshi-agents/examples/agent_native_react.rs`](yanshi-agents/examples/agent_native_react.rs) & [`hybrid_workflow_agent.rs`](yanshi-agents/examples/hybrid_workflow_agent.rs): Native ReAct loop and bidirectional `AgentNode` / `WorkflowTool` composition.
- [`yanshi-agents/examples/dynamic_workflow_plan.rs`](yanshi-agents/examples/dynamic_workflow_plan.rs): Runtime JSON `WorkflowPlan` compiled to a parallel `Flow`.

---

## 📄 License

Licensed under the **MIT License** (see [`Cargo.toml`](Cargo.toml)).
