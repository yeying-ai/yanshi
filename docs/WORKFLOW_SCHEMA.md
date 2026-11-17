# Workflow Schema

本文档描述 `yanshi workflow validate` 当前执行的 CLI workflow schema 校验规则。

## 校验入口

```bash
yanshi workflow validate path/to/workflow.yml
yanshi workflow validate path/to/workflow.yml --format json
yanshi workflow validate path/to/workflow.yml --strict
```

- 默认模式下，未知参数作为 warning 输出，用于兼容已有 YAML。
- `--strict` 会把未知参数升级为 error，适合 CI 或发布前检查。
- `--format json` 输出 `workflow`、`valid`、`issues`、`warnings`，供脚本和 server 复用。
- `workflow run` 和 `workflow run --dry-run` 会在构建 graph 前执行同一套 schema validation。
- `workflow debug --validate` 会复用同一套 schema validation，并叠加依赖和结构分析。
- `run_if` 与 `while.parameters.condition` 使用统一表达式语言；参考
  `docs/EXPRESSION_LANGUAGE.md`。`--strict` 会编译这些表达式并报告列号。
- `timeout_ms` / `max_retries`（T3.1）是节点级通用字段，与 `run_if` 同级、
  不在 `parameters:` 内，对任意节点类型生效（`map`/`while` 除外，会被
  拒绝——它们执行的是嵌套子流程而非单个节点）。重试只针对
  网络/超时/限流类瞬时错误，非瞬时失败（校验错误、4xx 类应用层错误）不会
  被重试。这与 `mcp` 节点自己的 `parameters.timeout_ms`/`max_retries`
  （只控制其 MCP client 连接，见下表）是两套独立机制，字段名相同但含义
  不同、互不影响。详见 `docs/CONFIGURATION.md` 的 Node fields 表。

## 通用规则

- `nodes` 至少包含一个节点。
- 每个 node `id` 必须非空，并且在 workflow 内唯一。
- `dependencies` 必须引用已存在的 node id。
- `input_mapping` 支持 `{{ nodes.<id>.outputs.<field> }}` 形式，并校验 `<id>` 是否存在。
- 标记为 input-compatible 的 required 参数可以通过 `parameters` 或 `input_mapping` 满足。
- `mcp` 和 `rag` 节点需要对应 crate feature；未启用时会输出明确 feature gate 错误。

## 节点参数

| Node type | Required | Input-compatible required | Optional |
| --- | --- | --- | --- |
| `llm` | - | `prompt` | `model`, `system`, `temperature`, `max_tokens`, `json_schema` |
| `skill_agent` | - | `skill`, `message` | `model` |
| `agent` | - | `prompt` / `message`（内联模式）或 `skill` + `message`（技能模式） | `system`, `model`, `tools`（内联支持 `web_search`, `http`）, `interactive`, `max_steps`, `max_questions`, `json_schema` |
| `multi_agent` | `mode` | `message` | `model`, `agents`, `participants`, `judge`, `initial_agent`, `max_handoffs`, `schedule`, `stop_when`, `answer_from`, `rounds`, `judge_prompt` |
| `http` | - | `url` | `method`, `headers`, `body` |
| `web_search` | - | `query` | `provider`, `max_results`, `topic` |
| `repo_snapshot` | - | `source` | `git_ref`, `subpath`, `max_bytes`, `max_files`, `allowed_paths` |
| `pdf_extract` | - | `source` | `max_chars`, `max_pages`, `allowed_paths` |
| `video_ingest` | - | `source` | `prefer_subtitles`, `subtitle_langs`, `max_duration_seconds`, `allowed_paths` |
| `file` | `allowed_paths` | `operation`（`read` / `write` / `copy` / `move` / `read_bytes`）, `path` | `content`（`write` 时为字符串或 `File` 值；`copy` / `move` 时为来源 `File` 值或路径） |
| `shell` | `allowed_commands` | `command` | `allowed_paths` |
| `template` | `template` | - | `output_key`, `output_format`（以及模板引用的任意上下文字段） |
| `arxiv` | `url` | - | `fetch_source`, `simplify_latex` |
| `asr` | - | `model`, `audio_source` | `response_format`, `language`, `prompt` |
| `image_edit` | `model` | `prompt`, `image_source` | `output_format` |
| `image_to_image` | `model` | `prompt`, `source_image` | `output_format` |
| `image_understand` | `model` | `text_prompt`, `image_source` | - |
| `markmap` | - | - | `markdown`, `save_to_file` |
| `text_to_image` | - | `model`, `prompt` | `negative_prompt`, `size`, `output_format` |
| `text_to_video` | - | `model`, `prompt` | `image`, `aspect_ratio`, `resolution`, `duration_seconds`, `output_format` |
| `tts` | - | `model`, `voice`, `input_template` | `instructions`, `speed`, `response_format`, `output_format` |
| `illustration` | - | - | `model`, `prompt_model`, `concept`, `prompt`, `context`, `deck`, `items`, `arch_image`, `arch_title`, `arch_caption`, `style_preset`, `style`, `theme`（亦支持 `input_mapping.palette` 别名避免覆盖工作流级 `theme`）, `size`, `refine_prompt`, `max_images`, `max_concurrent`, `enabled`, `graceful_fallback`, `output_format` |
| `figure` | - | `language`（`svg` / `d2` / `tikz` / `typst` / `python`） | `model`, `description`, `source`, `max_attempts`, `png`, `png_width`, `d2_theme`, `domain`（`circuit` / `chemistry` / `block`）, `spec` |
| `render_slides` | - | `deck` | `width`, `progressive`, `section_cards`, `pip_safe_zone`, `takeaway_only_bar`, `export_pptx`（纯 Rust 原生 OOXML `.pptx`）, `export_pdf`（经沙箱 `typst` 直出 `16:9` 宽屏 PDF） |
| `render_video` | - | `scenes` | `width`, `height`, `burn_subtitles`, `transition`, `step_pause_ms`, `slide_pause_ms`, `pip_video`, `pip_image`, `pip_position`, `pip_scale` |
| `render_audio` | - | `clips` | `format`, `target_lufs` |
| `render_mindmap` | - | `tree` | `format` |
| `render_study` | `kind`（`flashcards` / `quiz`） | `items` | `title` |
| `render_table` | - | `rows` | `format`（`csv` / `xlsx`）, `columns` |
| `render_svg` | - | `svg` | `width`, `format`（`png` / `svg`） |
| `render_markdown` | - | `markdown` | `title`, `format`（`html` / `md`） |
| `render_document` | `format`（`docx` / `pptx` / `html` / `epub` / `pdf`） | `markdown` | `title`, `toc`, `theme` |
| `map` | `template` | - | `parallel` |
| `while` | `condition`, `max_iterations`, `do` | - | `continue_on_error`, `fail_on_exhausted` |
| `mcp` | `server_command`, `tool_name` | - | `tool_params`, `parameters.timeout_ms`/`parameters.max_retries` (MCP-client-only, see note above) |
| `rag` | `operation`, `collection` | - | `qdrant_url`, `embedding_model`, `query`, `documents`, `top_k`, `search_type`, `alpha`, `rerank`, `lambda`, `vector_size`, `distance` |

`file` 节点的 `copy` / `move` / `read_bytes` 输出 `FlowValue::File`，`read` / `write` 保持字符串输出；`write` 的 `content` 若是 `File` 值则复制字节。Tera 模板里 `File` 值渲染为路径，`{{ x | mime }}` 与 `{{ x | file_size }}` 给出 MIME 与字节数。

媒体节点（`tts` / `text_to_image` / `image_to_image` / `image_edit` / `text_to_video`）的 `output_format` 取 `file`（默认）或 `data_uri`。`file` 把字节写到运行目录下的 `artifacts/<node_id>/<n>.<ext>`，输出 `FlowValue::File { path, mime_type }`，状态池里只有路径；`data_uri` 保留旧的 base64 `data:` 字符串（`text_to_video` 则是厂商的临时 URL）。`file` 模式下 `text_to_video` 会经厂商下载视频落盘，默认上限 512 MiB。

`http` 输出 `status`（真实的 2xx 状态码）和 `body`；非 2xx 响应会让节点失败，错误信息为 `HTTP <code>: <body 摘要>`。5xx 和 429 可重试，其余状态码不可重试。

## 参数类型

- `String`: YAML string。
- `Number`: integer 或 float。
- `Integer`: YAML integer。
- `Bool`: YAML boolean。
- `Object`: YAML map。
- `Sequence`: YAML list。
- `SequenceOfStrings`: 所有元素均为 string 的 YAML list。
- `Any`: 不限制类型。

## 嵌套节点

- `map.parameters.template` 必须是 workflow node definition 列表。
- `while.parameters.do` 必须是 workflow node definition 列表。
- 嵌套节点复用普通节点的 required 参数、类型和 unknown parameter 校验规则。

### `while` 循环可观测性（D11 / W2.4）

- 输出恒定携带 `iterations_used`（实际执行的迭代数）与 `exhausted`
  （`bool`：因 `condition` 转 false 正常退出为 `false`，因耗尽
  `max_iterations` 退出为 `true`）。
- `parameters.continue_on_error`（`Bool`，默认 `false`）：子流程某个 exit
  节点执行失败时，默认将其上浮为 While 节点级错误（整个 While 节点失败）；
  设为 `true` 恢复旧行为——记录警告日志后继续下一轮迭代。
- `parameters.fail_on_exhausted`（`Bool`，默认 `false`）：耗尽
  `max_iterations` 默认只记一条 `tracing::warn!`（`event =
  "while_loop_exhausted"`）并正常返回（`exhausted: true`）；设为 `true`
  则改为让 While 节点失败。
- 每轮迭代开始时额外记一条 `tracing::info!`（`event =
  "while_loop_iteration_started"`），带 `iteration`/`max_iterations`
  字段，便于观测循环进度。

## 条件表达式

`run_if` 和 `while.parameters.condition` 支持布尔、比较、算术和函数调用:

```yaml
run_if: "len(nodes.search.outputs.items) > 0 && nodes.classify.outputs.score > 0.7"

parameters:
  condition: "{{ count < 3 }}"
```

支持的路径包括 `nodes.X.outputs.Y`、`inputs.Z`、数组索引
`nodes.X.outputs.items.0`，以及 while loop 输入的简写形式 `count`。
