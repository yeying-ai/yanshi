# Yanshi Multimodal & Media Production Guide

Yanshi treats media and rendered documents as first-class citizens across both
its config-first DAG workflows and its Rust SDK. For the full per-provider model
table (context windows, tool calling, vision, image/audio/video generation, and
reasoning support), see [`LLM_PROVIDERS_MATRIX.md`](LLM_PROVIDERS_MATRIX.md) and
[`GRANULAR_MODEL_TYPES.md`](GRANULAR_MODEL_TYPES.md).

---

## 1. First-Class `FlowValue::File` Artifact Pipeline

Modality and renderer nodes emit `FlowValue::File { path, mime_type }` written
under `<run_dir>/artifacts/<node_id>/<n>.<ext>` rather than stuffing base64
blobs into the workflow state pool:

- **Constant-size state & checkpoints**: a 100 MB MP4 or 20-slide `.pptx` keeps
  state serialization under a few kilobytes, making `--checkpoint` /
  `workflow resume` fast and deterministic.
- **Direct downstream consumption**: `image_understand`, `image_edit`,
  `image_to_image`, `render_slides`, `render_video`, `render_audio`, and `file`
  (`copy` / `move` / `read_bytes` / `write`) read `FlowValue::File` inputs
  directly from disk; Tera templates expose the path plus `{{ f | mime }}` and
  `{{ f | file_size }}` filters.
- **Export & streaming download**:
  - CLI: `yanshi workflow run <file.yml> --output-dir ./out` copies every file
    artifact to `./out/<node_id>/<file>` alongside `artifacts.json` (size +
    SHA-256).
  - Gateway / Studio: `GET /v1/runs/{id}/artifacts` lists registered files and
    `GET /v1/runs/{id}/artifacts/{artifact_id}` streams them with tenant
    isolation.
- **Legacy compatibility**: modality nodes still accept `output_format: data_uri`
  when an external consumer explicitly requires inline `data:` strings.

---

## 2. Multimodal & Rendering Nodes in Config-First YAML

### Vision, Image Generation & Art-Directed Illustrations

| Node `type:` | Crate | Purpose |
| --- | --- | --- |
| `image_understand` | `yanshi-nodes-ai` | Visual Q&A and structured critique over local `FlowValue::File` images or URLs (`model`, `text_prompt`, `image_source`). |
| `text_to_image` | `yanshi-nodes-ai` | Direct prompt-to-image synthesis across Gemini / Imagen (Vertex AI & AI Studio), OpenAI (`gpt-image-1` / DALL-E), DashScope (`wanx` / `qwen-image`), and GLM (`cogview`), with automatic aspect-ratio normalization. |
| `image_to_image` / `image_edit` | `yanshi-nodes-ai` | Reference-guided restyling and mask/instruction editing over `FlowValue::File` inputs. |
| `illustration` | `yanshi-nodes-ai` | Art-directed conceptual illustration generator ([`illustration.rs`](../yanshi-nodes-ai/src/nodes/illustration.rs)): optional LLM prompt refinement (`refine_prompt: true`), dark/light canvas palette harmonization (`#0B1120` / `#F8FAFC`), strict anti-text/no-watermark guardrails, bounded concurrent batch generation over `SlideDeck`s, and 6 `style_preset`s (`editorial_tech`, `isometric_3d`, `flat_vector`, `blueprint_schematic`, `scientific_diagram`, `cinematic_keynote`). |
| `figure` | `yanshi-nodes-ai` | Validated vector diagram compiler (`language`: `d2`, `svg`, `typst`, `tikz`, or isolated container `python`) with automatic compile-error feedback repair (`max_attempts`) and domain parts-list verification (`domain: circuit`). |

Example YAML combining `figure` + `illustration` + native OOXML `render_slides`:

```yaml
name: "illustrated-brief"
cost_limit_usd: 0.50
nodes:
  - id: arch_svg
    type: figure
    parameters:
      language: d2
      description: "Client -> Pingora TLS Gateway -> Axum Server -> PostgreSQL"

  - id: hero_art
    type: illustration
    parameters:
      model: gemini-3.1-flash-image
      concept: "Deterministic DAG scheduler orchestrating parallel media pipelines"
      style_preset: editorial_tech
      theme: dark
      refine_prompt: true

  - id: critique
    type: image_understand
    dependencies: ["hero_art"]
    input_mapping:
      image_source: "{{ nodes.hero_art.outputs.output }}"
    parameters:
      model: gemini-2.5-flash
      text_prompt: "Describe the visual metaphor and color palette in two sentences."
```

### Audio, Video & Keynote Renderers

| Node `type:` | Crate | Purpose |
| --- | --- | --- |
| `tts` | `yanshi-nodes-ai` | Expressive speech synthesis across OpenAI, Gemini TTS, DashScope (`qwen3-tts-flash` / `cosyvoice`), MiniMax, and StepFun, with cross-vendor voice normalization and vocal style `instructions`. |
| `asr` & `video_ingest` | `yanshi-nodes-ai` / `yanshi-nodes` | Speech-to-text transcription (`whisper-1`, `gemini-2.5-flash-asr`, `glm-asr`) and video caption/audio ingestion with `.srt` output. |
| `text_to_video` | `yanshi-nodes-ai` | Generative video clips (Google Veo, DashScope Wan, Kling 4.0, MiniMax Hailuo) downloaded to `FlowValue::File` with size caps. |
| `render_slides` | `yanshi-nodes` | 1920×1080 vector slide engine: interactive `.html`, pure-Rust native editable OOXML `.pptx` (`export_pptx: true`), and sandboxed `16:9` Typst `.pdf` (`export_pdf: true`). |
| `render_video` | `yanshi-nodes` | Sandboxed FFmpeg 1080p video compositor: progressive step-synced narration, smooth transitions, burned-in ASS or soft `.srt`/`.vtt` subtitles, and rounded PiP presenter overlays. |
| `render_audio` | `yanshi-nodes` | Multi-clip concatenation and two-pass EBU R128 loudness normalization (`.mp3`). |

---

## 3. Cost Accounting & Retry Discipline

Paid multimodal runs are governed by hierarchical `CostMeter` tracking
([`yanshi-llm/src/pricing.rs`](../yanshi-llm/src/pricing.rs)):

- **Granular unit pricing**: `PricingTable` tracks token rates alongside
  `per_image`, `per_output_second` (TTS / video), and `per_1k_input_chars`.
- **Hard budget ceiling**: set `cost_limit_usd` at the top of a workflow YAML or
  in `POST /v1/runs`. Once cumulative spend reaches the ceiling, unstarted nodes
  abort with `YanshiError::CostLimitExceeded` before calling external vendors,
  emitting `cost.limit_exceeded`.
- **Smart retry classification**: modality calls retry transient `429`, `5xx`,
  and network timeouts with backoff, and **never** retry `4xx` content-policy or
  schema rejections so rejected prompts are never double-billed.

---

## 4. Rust SDK (`yanshi-llm`) Multimodal Chat & Streaming

When embedding `yanshi-llm` directly in Rust, use `MultimodalMessage` to combine
text, image URLs, or local bytes, and stream responses including reasoning
(`chunk.thinking`) and inline generated images (`chunk.media`):

```rust
use yanshi_llm::{MultimodalMessage, Yanshi};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  Yanshi::ensure_init().await?;

  let system = MultimodalMessage::system()
    .add_text("You are a systems architecture reviewer. Be concise and precise.")
    .build();

  let user = MultimodalMessage::user()
    .add_text("Compare the dataflow bottlenecks in this diagram:")
    .add_image_url_with_detail("https://example.com/architecture.png", "high")
    .build();

  let mut stream = Yanshi::model("gpt-4o")
    .multimodal_messages(vec![system, user])
    .temperature(0.3)
    .execute_streaming()
    .await?;

  while let Some(chunk) = stream.next_chunk().await? {
    if !chunk.content.is_empty() {
      print!("{}", chunk.content);
    }
    if chunk.is_final {
      break;
    }
  }
  println!();
  Ok(())
}
```

### Related Runnable Examples

- [`examples/generators/`](../examples/generators/) — 15 production Content
  Studio YAML workflows (`illustrated_slides.yml`, `tech_slides_video.yml`,
  `video_overview.yml`, `audio_overview.yml`, `paper_reader.yml`,
  `video_summarizer.yml`; see [`STUDIO_CATALOGUE.md`](STUDIO_CATALOGUE.md)).
- [`yanshi-llm/examples/rust_traits_studio.rs`](../yanshi-llm/examples/rust_traits_studio.rs) —
  Code-first multimodal pipeline combining Chat, Text-to-Image, VLM review, Veo
  I2V, and TTS.
- [`yanshi-config/examples/rust_traits_ppt_studio.rs`](../yanshi-config/examples/rust_traits_ppt_studio.rs) —
  Deterministic 1080p slide deck + progressive MP4 + native `.pptx` & `.pdf`
  export.
