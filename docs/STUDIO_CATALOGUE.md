# Content Studio Catalogue & Desktop App

Yanshi's Content Studio turns raw sources — a topic idea, web URLs, GitHub
repositories, academic PDFs, or audio/video recordings — into finished,
downloadable deliverables through a deterministic five-stage pipeline
([`RFC_CONTENT_STUDIO.md`](RFC_CONTENT_STUDIO.md)):

1. **Sources** — ingest web search results, pages (`http` with SSRF protection),
   codebases (`repo_snapshot`), PDFs (`pdf_extract`), arXiv source tarballs
   (`arxiv`), or recordings (`video_ingest` + `asr`).
2. **Planner** — optional content-level planning (`examples/applications/content-studio/`
   or interactive `type: agent` clarification via `ask_user`).
3. **Generators** — hand-authored, schema-validated DAG templates under
   [`examples/generators/`](../examples/generators/) with structured JSON
   outputs and chunk-level citations.
4. **Renderers** — embedded pure-Rust writers (`render_slides` + native OOXML
   `.pptx`, `render_table` `.xlsx`/`.csv`, `render_svg`, `render_mindmap`,
   `render_study`, `render_markdown`) plus OS-sandboxed media/typesetting tools
   (`typst`, `pandoc`, `ffmpeg`, `d2`, `lualatex`, `ContainerBackend` Python).
5. **Artifacts** — first-class `FlowValue::File` outputs stored on disk with
   SHA-256 manifests, downloadable via CLI `--output-dir`, `/v1/runs/{id}/artifacts`,
   Web Studio, or the `Yanshi Studio` desktop app.

---

## Built-in Generator Catalogue (19 Workflows)

Every template in [`examples/generators/`](../examples/generators/) validates
under `yanshi workflow validate --strict` and can be executed from the CLI, the
HTTP gateway, `/ui/studio`, `/ui/app`, or the macOS/Windows/Linux desktop app.

### 1. Keynote Slides, Video & Audio

| Template | Primary Inputs | Deliverables | Visual & Media Highlights |
| --- | --- | --- | --- |
| [`illustrated_slides.yml`](../examples/generators/illustrated_slides.yml) | `subject`, `audience`, `slide_count`, `slide_theme` (`dark`/`light`), `style_preset` | Interactive `.html` deck, native editable OOXML `.pptx`, `16:9` `.pdf` | Web grounding → slide visual routing across AI `illustration` (6 art presets), D2 `figure`, and native editable shapes/tables/code |
| [`tech_slides_video.yml`](../examples/generators/tech_slides_video.yml) | `topic`, `repo_url`, `pdf_source`, `presenter_mode` (`spotlight`/`pip`/`intro_outro`) | 1080p narrated `.mp4`, `.srt`/`.vtt` subtitles, `.pptx`, `16:9` `.pdf`, `.html` | Progressive step-by-step slide builds synced millisecond-accurately to expressive TTS, spotlight rings, optional PiP presenter video |
| [`slides.yml`](../examples/generators/slides.yml) | `url` / `text` / `chunks`, `focus`, `enable_illustrations`, `style_preset` | Native editable `.pptx`, `16:9` `.pdf`, `.html` | Source-grounded presentation with self-repairing vector `figure` diagrams and optional `illustration` visuals (works with zero image keys by default) |
| [`video_overview.yml`](../examples/generators/video_overview.yml) | `url` / `text`, `image_model`, `tts_model`, `video_model` (optional) | Narrated `.mp4` + `.srt` | Parallel scene illustration (`text_to_image` or optional `text_to_video` motion shots) + per-scene `tts` + `render_video` muxing |
| [`audio_overview.yml`](../examples/generators/audio_overview.yml) | `url` / `text`, `minutes`, `tts_model`, `host_voice`, `guest_voice` | Loudness-normalised `.mp3` | Two-speaker conversational script voiced in parallel and EBU R128 normalised via `render_audio` |

#### Slide Visual Routing & Art-Direction Presets

Slide workflows (`illustrated_slides.yml`, `slides.yml`, `tech_slides_video.yml`)
classify each slide's visual layout automatically:

- **`illustration`** — conceptual, cover, analogy, and summary slides are
  rendered by [`IllustrationNode`](../yanshi-nodes-ai/src/nodes/illustration.rs)
  with optional LLM art-director prompt refinement (`refine_prompt: true`),
  canvas palette harmonization (`dark` `#0B1120` / `light` `#F8FAFC`), and strict
  anti-text guardrails across six `style_preset` modes:
  - `editorial_tech` *(default)* — editorial publication illustration with crisp geometric forms
  - `isometric_3d` — clean 3D isometric miniature with soft studio lighting
  - `flat_vector` — Swiss-grid geometric vector art with bold silhouettes
  - `blueprint_schematic` — precision architectural/engineering CAD schematic
  - `scientific_diagram` — textbook-grade layered cross-section visual
  - `cinematic_keynote` — dramatic wide-angle keynote hero visual
- **`arch_diagram`** — system topology and dataflow slides compile validated
  vector SVG diagrams via `figure` (`language: d2` / `svg` / `typst` / `tikz` / `python`)
  with automatic compile-error feedback repair.
- **`structured_panel`** — code walkthroughs, benchmarks, and checklists emit
  native editable PowerPoint shapes, `<a:tbl>` tables, `<p:cxnSp>` connectors,
  and monospace callouts directly via pure-Rust OOXML (`export_pptx: true`).

### 2. Daily Engineering & Research Workflows

| Template | Primary Inputs | Deliverables | Workflow Highlights |
| --- | --- | --- | --- |
| [`repo_architecture.yml`](../examples/generators/repo_architecture.yml) | `source` (GitHub URL or local dir), `git_ref`, `subpath`, `focus` | D2 system & module dependency `.svg`/`.png`, `.pdf`, `.html`, `.md` | `repo_snapshot` scans directory tree, manifests, and internal imports without executing untrusted repo code |
| [`backend_project_planner.yml`](../examples/generators/backend_project_planner.yml) | `idea`, `tech_preference`, `interactive` (`true`/`false`), `max_questions` | System arch, module dep, UML class/sequence, flowchart & ER `.svg`, PostgreSQL `.sql` DDL, mindmap, full spec `.pdf` | Multi-turn HITL `ask_user` clarification pauses the run in Studio/CLI to confirm scale and constraints before generating full specs |
| [`paper_reader.yml`](../examples/generators/paper_reader.yml) | `source` (arXiv URL, PDF URL, or local PDF), `target_lang`, `max_pages` | Bilingual digest `.pdf`/`.html`, extracted experiment tables `.xlsx`/`.csv`, method diagram `.svg`, mindmap | `pdf_extract` + structured experimental benchmark extraction into numeric Excel sheets |
| [`video_summarizer.yml`](../examples/generators/video_summarizer.yml) | `source` (video URL or local media file), `subtitle_langs`, `asr_model` | Timestamped summary `.pdf`/`.html`, `.srt` subtitles (native or ASR + optional bilingual polish), mindmap | `video_ingest` prefers native captions and falls back automatically to FFmpeg audio extraction + `asr` |

### 3. Sourced Reports, Diagrams & Study Kits

| Template | Primary Inputs | Deliverables | Workflow Highlights |
| --- | --- | --- | --- |
| [`report.yml`](../examples/generators/report.yml) | `url` / `text` / `chunks`, `focus`, `sections`, `pdf` | Footnoted `.md`, self-contained `.html`, typeset `.pdf` | Parallel `map` section drafting with verbatim source quotes and chunk citations |
| [`data_table.yml`](../examples/generators/data_table.yml) | `url` / `text` / `chunks`, `focus`, `max_rows` | `.xlsx`, `.csv`, self-contained preview `.html` | Structured entity/metric extraction with per-row `chunk_id` & `quote` citations rendered via `render_table` + `render_markdown` |
| [`infographic.yml`](../examples/generators/infographic.yml) | `url` / `text` / `chunks`, `focus` | `.svg` and high-DPI `.png` | Deterministic Tera SVG poster layout driven by schema-validated headline metrics |
| [`mindmap.yml`](../examples/generators/mindmap.yml) | `url` / `text` / `chunks`, `depth` | Vector `.svg`, `.png`, interactive markmap `.html` | Hierarchical tree extraction rendered via embedded `render_mindmap` + `render_svg` |
| [`circuit_diagram.yml`](../examples/generators/circuit_diagram.yml) | `description`, `text` | Checked schematic `.svg` and `.png` | Extracts a typed parts list first, then verifies every component ID/kind in `circuitikz` before `lualatex` compile |
| [`flashcards.yml`](../examples/generators/flashcards.yml) | `url` / `text` / `chunks`, `count` | Interactive flip-card `.html` + JSON deck | Self-contained offline study page rendered by `render_study` (`kind: flashcards`) |
| [`quiz.yml`](../examples/generators/quiz.yml) | `url` / `text` / `chunks`, `count` | Self-grading multiple-choice `.html` + JSON | Instant explanation and score tally via `render_study` (`kind: quiz`) |

### 4. Multi-Agent Creative & Deliberative Studios

| Template | Primary Inputs | Deliverables | Multi-Paradigm Highlights |
| --- | --- | --- | --- |
| [`episodic_drama_studio.yml`](../examples/generators/episodic_drama_studio.yml) | `premise`, `episode_count`, `scenes_per_episode`, `existing_cast_sheet`, `existing_series_bible` | 1080p multi-episode `.mp4`, `.srt`, Master Cast Sheet `.png`, Series Bible `.pdf` | `multi_agent (blackboard)` writers' room (`showrunner` + `casting_director` + `episode_writer`) + `while` + `image_understand` VLM cast-sheet critic + cross-episode `image_to_image` / I2V face lock + character-pinned `tts` + cross-run continuation |
| [`debate_podcast_show.yml`](../examples/generators/debate_podcast_show.yml) | `motion_topic`, `pro_position`, `con_position`, `turn_count` | 3-voice loudness-normalised `.mp3`, cover `.png`, briefing `.pdf` | Genuine 2-round adversarial clash via `multi_agent (debate)` (`pro-advocate` vs `con-challenger` + `roundtable-moderator` judge) adapted into a 3-voice radio show |
| [`worldbuilding_bible.yml`](../examples/generators/worldbuilding_bible.yml) | `seed_idea`, `medium_target`, `art_direction`, `max_art_rounds` | Illustrated Bible `.pdf`/`.html`, key art & region plates `.png`, 2x D2 `.svg`, mindmap | `multi_agent (handoff)` specialist relay (`lore_architect` → `faction_smith` → `quest_designer`) + `while` + `image_understand` key-art aesthetic critic + D2 faction & quest diagrams |

In addition, [`examples/skills/creative-producer/skill.toml`](../examples/skills/creative-producer/skill.toml) exposes `episodic_drama`, `debate_podcast`, `worldbuilding_bible`, and `illustrated_slides` as callable **`WorkflowTool`s** so an autonomous executive producer agent can chain worldbuilding into episodic production on demand.

---

## Quickstart Commands

### CLI One-Liners

```bash
# 1. Illustrated Keynote Deck (.pptx + 16:9 .pdf + interactive .html)
yanshi workflow run examples/generators/illustrated_slides.yml \
  --input subject "Rust Async Runtime & Work-Stealing Scheduler" \
  --input style_preset editorial_tech \
  --output-dir out/slides

# 2. Academic Paper Reader (arXiv/PDF -> .pdf dossier + .xlsx tables + D2 diagram)
yanshi workflow run examples/generators/paper_reader.yml \
  --input source "https://arxiv.org/abs/1706.03762" \
  --output-dir out/paper

# 3. GitHub Repository Architecture Analysis
yanshi workflow run examples/generators/repo_architecture.yml \
  --input source "https://github.com/tokio-rs/axum" \
  --output-dir out/axum-arch

# 4. Multi-Deliverable Planner Skill (one source -> report + mindmap + audio)
yanshi skill run examples/applications/content-studio \
  --input url=https://example.org/article \
  --input want=report,mindmap \
  --output-dir out/studio

# 5. Notebook CLI (ingest rich Office/tabular sources -> multi-turn Q&A -> deliverables & portable bundle)
yanshi notebook create "Oceanography Study"
yanshi notebook add-source <notebook_id> --file metrics.xlsx --wait
yanshi notebook ask <notebook_id> "What is the peak range of Bay of Fundy?" --history-file chat.json
yanshi notebook generate <notebook_id> data_table --cost-limit-usd 2.50 --output-dir out/table
yanshi notebook download-artifacts <notebook_id> --out out/oceanography.zip
yanshi notebook export <notebook_id> --out out/oceanography.json --brief-out out/BRIEFING.md
```

Cap total LLM + media spend on any run with `cost_limit_usd` in YAML or `--checkpoint`
to resume interrupted multi-stage pipelines without re-billing completed nodes:

```bash
yanshi workflow run examples/generators/tech_slides_video.yml \
  --checkpoint --timeout 15m \
  --input topic "Zero-Copy Networking in Rust" \
  --output-dir out/lecture

# If interrupted at a late stage, resume from the last completed node:
yanshi workflow resume <run_id> examples/generators/tech_slides_video.yml \
  --output-dir out/lecture
```

### Web Studio (`/ui/notebooks`, `/ui/studio` & `/ui/app`)

Start `yanshi-server` and open:

- **`http://127.0.0.1:8080/ui/notebooks`** — **Notebook Workspace**: ingest URLs,
  `.pdf`/`.docx`/`.csv`/`.tsv`/`.json`/`.md`/`.html` files, notes, and audio
  recordings into per-notebook BM25 indexes, scope active sources via checkboxes,
  ask grounded questions with verified chunk citations, and launch 12
  source-grounded deliverables with interactive citation chips.
- **`http://127.0.0.1:8080/ui/app`** — **App Runner**: pick any of the 19
  templates, fill in the generated input form, answer inline `ask_user`
  clarification cards if prompted, and preview/download finished `.pptx`,
  `.pdf`, `.mp4`, `.xlsx`, and `.html` deliverables.
- **`http://127.0.0.1:8080/ui/studio`** — **Visual DAG Studio**: inspect and
  edit the workflow graph on an interactive canvas, set per-node breakpoints,
  step through evaluated inputs/outputs, and preview rendered slides or media
  inline.

### Desktop App (`Yanshi Studio` via Tauri v2)

`yanshi-ui/src-tauri` packages the full Studio and App Runner as a standalone
desktop app with an in-process local execution engine (`local_engine.rs`) —
**no PostgreSQL or external server process required**. It automatically loads
`~/.yanshi/.env`, seeds the bundled 44-model pricing table
(`config/pricing.prod.yml`), and supports interactive `ask_user` HITL prompts
offline.

On macOS, build and install `Yanshi Studio.app` directly into `/Applications`:

```bash
# Build release bundle and stage into /Applications/Yanshi Studio.app
./scripts/install_desktop.sh --open

# Re-install an already built bundle without recompiling
./scripts/install_desktop.sh --skip-build
```

Check external rendering binaries (`ffmpeg`, `ffprobe`, `pandoc`, `typst`, `d2`)
at any time with:

```bash
yanshi doctor --profile local
```
