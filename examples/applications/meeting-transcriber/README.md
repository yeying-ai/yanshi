# A4 — meeting-transcriber

**Status**: DONE — offline mock smoke in CI + live ASR/LLM/PDF verified (2026-11-18)
**Tracking entry**: [`EXAMPLES_TODOs.md` § A4](../../../EXAMPLES_TODOs.md#a4--meeting-transcriber)

## Business

Input: a meeting recording (`.mp3` / `.wav` / `.m4a` / `.ogg`), URL, or `data:audio/*;base64` URI.
Output (exported via `--output-dir`, gateway-safe with zero raw host `file` nodes):
- `transcribe/1.srt` — millisecond-timestamped SubRip cues from ASR.
- `transcript_md/1.md` — speaker-segmented transcript (`[<timestamp>] <speaker>: <utterance>`) + raw `.srt` reference.
- `summary_md/1.md` & `summary_html/1.html` — executive summary, locked decisions, and topic table.
- `action_items_md/1.md` — `"<owner> will <action> by <deadline>"` checklist + structured Markdown table.
- `action_items_xlsx/1.xlsx` & `action_items_csv/1.csv` — spreadsheet trackers (`id`, `priority`, `owner`, `action`, `deadline`, `evidence`).
- `minutes_pdf/1.pdf` — full compiled meeting dossier PDF (optional via `--input export_pdf false`).

## Architecture

Declarative gateway-safe DAG (`workflow.yml`):

```
audio_source →
  transcribe (type: asr, response_format: srt) →
  segment_and_summarize (type: llm, json_schema: speaker_turns + decisions + topics) →
  extract_action_items (type: llm, json_schema: owner / action / deadline / priority) →
  parallel Studio renderers:
    ├── render_markdown → transcript.md / summary.md / summary.html / action_items.md
    ├── render_table    → action_items.xlsx / action_items.csv
    └── render_document → minutes.pdf (optional run_if)
```

## ASR Provider Tradeoff Matrix

| Provider (`asr_model`) | Cost | Timestamp / Speaker Quality | Privacy & Setup |
| --- | --- | --- | --- |
| `gemini-2.5-flash-asr` *(default)* | Low | Accurate SubRip cue timestamps + multilingual punctuation | Cloud API (`GEMINI_API_KEY`) |
| `whisper-1` | ~$0.006 / min | Strong English & technical jargon; pass `vocabulary_hint` for project acronyms | Cloud API (`OPENAI_API_KEY`) |
| `step-asr` / local OpenAI-compatible Whisper (`whisper.cpp` / `faster-whisper-server`) | Free (local) | Full data locality; point custom `base_url` in `~/.yanshi/models.yml` | On-prem / zero external audio egress |
| `mock-asr` | Zero | Deterministic CI smoke (`"mock transcript"`) | Offline tests (`meeting_transcriber_cli_tests`) |

## Run

```bash
# Strict static validation (zero API keys needed)
yanshi workflow validate examples/applications/meeting-transcriber/workflow.yml --strict

# Live run on the bundled two-speaker standup recording (~24.5s MP3)
yanshi workflow run examples/applications/meeting-transcriber/workflow.yml \
  --model gemini-2.5-flash \
  --input vocabulary_hint "Postgres, SQLx, Yanshi, PDF" \
  --output-dir runs/meeting-transcriber

# Headless / without pandoc + typst PDF rendering
yanshi workflow run examples/applications/meeting-transcriber/workflow.yml \
  --model gemini-2.5-flash \
  --input export_pdf false \
  --output-dir runs/meeting-transcriber
```

## Tests

```bash
cargo test -p yanshi-cli --test meeting_transcriber_cli_tests
```

## Findings during dogfooding

1. **`F-A4-1` — `load_bytes_from_source` HTTP path bypasses `HttpTool` SSRF filter**: `ASRNode` delegates remote `http(s)://` URLs to `yanshi_nodes::common::utils::load_bytes_from_source`, which calls bare `reqwest::get(source)` rather than `HttpTool`. On multi-tenant gateways, untrusted remote URLs should go through `video_ingest` (which uses `HttpTool` + SSRF checks) or uploaded audio files.
2. **`F-A4-2` — CLI `--input` flag arity differs between `workflow run` and `skill run`**: `yanshi workflow run` expects two tokens (`--input <KEY> <VALUE>`), whereas `yanshi skill run` expects one (`--input KEY=VALUE`). Worth unifying with `value_parser` accepting either form in a future CLI polish pass.
3. **`F-A4-3` — `dependencies` vs soft `input_mapping` on optional `run_if` outputs**: Listing an optional `run_if` node (`minutes_pdf`) in `deliverable.dependencies` propagates `Skipped` to `deliverable` when `export_pdf=false`; referencing `minutes_pdf` only in `deliverable.input_mapping` acts as a soft ordering edge so `deliverable` still runs.
4. **`F-A4-4` — Domain jargon benefits from `ASRNode` `prompt` (`vocabulary_hint`)**: Without a vocabulary hint, synthesized `"Postgres pool config"` was transcribed as `"post pool config"`; wiring `vocabulary_hint` into `transcribe.input_mapping.prompt` gives Whisper/Gemini ASR domain-term priming.
