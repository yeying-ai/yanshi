# Yanshi 试用手册（Trial Guide）

**版本基线**：2026-11-21（Content Studio **Y0–Y10** 及 **Z–AJ** 全闭环 + 单机多 Agent 工作流真并行与 `workflow run-batch` 本地跑批 + 19 个内置生成器 + 多源笔记本工作台 + Tauri 桌面端 `Yanshi Studio`）  
**适用读者**：想在日常研发、论文研读、技术分享与多模态内容生产中直接使用 Yanshi 的开发者与创作者  
**配套文档**：当前全景状态见 [`CURRENT_STATUS.md`](CURRENT_STATUS.md)；19 个内置工作流清单见 [`STUDIO_CATALOGUE.md`](STUDIO_CATALOGUE.md)

---

## 0. 先读：当前各入口成熟度总览

经过 V / W / X / Y 四轮收口及 Z–AJ 增量演进，核心 CLI、Studio 生成器、服务端网关与桌面端均已打通端到端闭环：

| 状态 | 入口 / 路径 | 说明 |
|:---:|---|---|
| 🟢 | **Studio 生成器（19 套模板）** | `yanshi workflow run examples/generators/<name>.yml --output-dir out`，直出 `.pptx`、`.pdf`、`.mp4`、`.xlsx`、`.mp3`、`.svg`、`.html` |
| 🟢 | **桌面端 `Yanshi Studio.app`（macOS / Windows / Linux）** | 免 PostgreSQL 零配置本地引擎，支持可视化单步调试、一键运行、内置 76 模型价格表与交互式问答卡片 |
| 🟢 | **Web Studio（`/ui/notebooks` · `/ui/app` · `/ui/studio` · `/ui/console`）** | `yanshi-server` 内嵌前端，支持多源笔记本（含图片/扫描件 OCR）、模板运行、DAG 画布断点调试与 SSE 事件追踪 |
| 🟢 | **本地并发 DAG 与批量编排（`workflow run / run-batch / resume / validate / debug`）** | 默认 `concurrent(8)` 调度、`FlowValue::File` 零 base64 流转、跨子工作流共享 `cost_limit_usd` 预算池与不重复计费断点续跑 |
| 🟢 | **Skill + 并行工作流工具编排（`skill run / chat / validate`）** | 并发安全的 `[[workflows]]` 子工作流跨多核真并行执行（默认 `--max-concurrency 8`），内置双语 CJK BM25 知识检索 |
| 🟢 | **Harness 治理会话与动态工作流** | 支持非幂等/发布类（`Publish`）/超预算审批门禁、`DenyAndStop` 立即停机与 `ask_user` 中断恢复 |
| 🟢 | **诊断与复盘（`doctor` / `trace replay` / `trace tui`）** | 自动体检模型配置、价格表、沙箱等级与外部渲染二进制（`ffmpeg` / `pandoc` / `typst` / `d2`） |

---

## 1. 环境准备（5 分钟）

### 1.1 安装 CLI（或桌面端）

```bash
# 安装命令行工具 yanshi
cargo install --path yanshi-cli
yanshi --help

# （macOS 可选）一键构建并安装桌面端到 /Applications/Yanshi Studio.app
./scripts/install_desktop.sh --open
```

### 1.2 初始化模型与 API Key

```bash
yanshi config init
# 生成:
#   ~/.yanshi/models.yml   用户模型覆盖配置（自动继承内置多模态模型库）
#   ~/.yanshi/.env         API Key 模板
```

编辑 `~/.yanshi/.env`，填入至少一家厂商的 API Key：

```text
OPENAI_API_KEY / ANTHROPIC_API_KEY / GEMINI_API_KEY / MOONSHOT_API_KEY /
DASHSCOPE_API_KEY / STEP_API_KEY / GLM_API_KEY / DEEPSEEK_API_KEY / MINIMAX_API_KEY
```

> **提示**：配置单个 `DEEPSEEK_API_KEY` 即可同时使用 `deepseek-flash` 多模态视觉理解以及原生 `/v1/tools/{tts,asr,search,fetch,ocr}`（语音合成、语音识别、联网检索、网页抓取与 PDF/图片 OCR）。

运行体检确认配置与外部工具状态：

```bash
yanshi config validate
yanshi doctor --profile local
```

`yanshi doctor` 会逐项报告：LLM 模型注册表、计费价格表（`PricingTable`）、OS 沙箱后端，以及可选外部渲染工具（`pandoc`、`typst`、`ffmpeg`、`ffprobe`、`d2`）。即使未安装外部工具，纯 Rust 内嵌渲染器（原生 OOXML `.pptx`、`.xlsx`/`.csv`、思维导图 `.svg`/`.png`、闪卡/测验 `.html`）依然开箱可用。

---

## 2. 场景一：一句话生成日常成品（Content Studio，10 分钟）

[`examples/generators/`](../examples/generators/) 提供了 19 套生产级模板（完整说明见 [`STUDIO_CATALOGUE.md`](STUDIO_CATALOGUE.md)）。推荐先试以下四条日常高频工作流：

### 2.1 精美插图技术演讲 PPT（`.pptx` + `16:9 .pdf` + 交互 `.html`）

自动联网检索资料 → 规划大纲 → 按页智能分流生成 AI 概念插图（6 种艺术预设，严禁嵌入乱码文字）、编译校验的 D2 架构图与原生可编辑 PowerPoint 形状/表格/代码框：

```bash
yanshi workflow run examples/generators/illustrated_slides.yml \
  --input subject "Rust 异步运行时 Tokio 调度器与工作窃取原理" \
  --input style_preset editorial_tech \
  --output-dir out/tokio-slides
```

### 2.2 论文双语精读与实验表格提取（arXiv / PDF → `.pdf` + `.xlsx` + 架构图）

```bash
yanshi workflow run examples/generators/paper_reader.yml \
  --input source "https://arxiv.org/abs/1706.03762" \
  --output-dir out/paper-digest
```

### 2.3 GitHub 仓库架构扫描与模块拓扑图（Repo → D2 `.svg` + `.pdf`）

```bash
yanshi workflow run examples/generators/repo_architecture.yml \
  --input source "https://github.com/tokio-rs/axum" \
  --output-dir out/axum-arch
```

### 2.4 交互式后端项目架构规划（多轮 `ask_user` 澄清 → 全套设计图 + PostgreSQL DDL + PDF）

```bash
yanshi workflow run examples/generators/backend_project_planner.yml \
  --input idea "设计一个支持高并发秒杀与多租户库存隔离的订单履约后端系统" \
  --output-dir out/order-spec
```

> **提示**：在桌面端 `Yanshi Studio` 或 Web `/ui/app`、`/ui/studio` 中运行该模板时，画布与运行面板会自动弹出交互式问答卡片，回答后工作流无缝继续执行。

---

## 3. 场景二：自定义 DAG 流水线与断点续跑（20 分钟）

### 3.1 编写声明式 YAML 工作流

```yaml
# my_pipeline.yml
name: "daily-brief"
cost_limit_usd: 0.25
inputs:
  topic:
    description: "简报主题"
    default: "Rust WebAssembly 组件模型"
nodes:
  - id: build_prompt
    type: template
    parameters:
      template: "请围绕以下主题撰写一份结构化技术要点简报：{{ topic }}"

  - id: draft
    type: llm
    dependencies: ["build_prompt"]
    input_mapping:
      prompt: "{{ nodes.build_prompt.outputs.output }}"

  - id: brief_html
    type: render_markdown
    dependencies: ["draft"]
    input_mapping:
      markdown: "{{ nodes.draft.outputs.output }}"
    parameters:
      title: "每日技术简报"
```

校验并运行：

```bash
yanshi workflow validate my_pipeline.yml --strict
yanshi workflow run my_pipeline.yml \
  --input topic "Rust 异步生态演进" \
  --output-dir out/brief
```

输入优先级规则明确固定为：**`input_mapping` > 工作流级 `inputs` > 节点静态 `parameters`**（见 [`CONFIGURATION.md`](CONFIGURATION.md)）。

### 3.2 成本上限与付费节点断点续跑

长耗时多模态流水线（例如 `tech_slides_video.yml` 演播级 1080p 课程视频）建议开启 `--checkpoint`：

```bash
yanshi workflow run examples/generators/tech_slides_video.yml \
  --checkpoint --timeout 15m \
  --input topic "Rust Traits 零成本抽象与动态分发" \
  --output-dir out/traits-video

# 若中途因网络波动中断，直接续跑未完成节点（已完成节点跳过且不重复计费）：
yanshi workflow resume <run_id> examples/generators/tech_slides_video.yml \
  --output-dir out/traits-video
```

### 3.3 单机多工作流并行批处理（`workflow run-batch`）

无需部署 PostgreSQL 或分布式 Worker，即可在单机上并发/并行运行多套工作流（或 `--manifest` 参数矩阵），并共享同一个根 `CostMeter` 预算池、按 `<output-dir>/<job>/` 隔离产物与输出 `batch_summary.json`：

```bash
yanshi workflow run-batch \
  examples/generators/report.yml \
  examples/generators/mindmap.yml \
  examples/generators/flashcards.yml \
  --input source="https://arxiv.org/abs/1706.03762" \
  --max-parallel 3 \
  --cost-limit-usd 1.20 \
  --output-dir out/paper-bundle
```

---

## 4. 场景三：可视化 Studio 与桌面端调试（10 分钟）

除了命令行，推荐打开可视化工作台体验**单步断点调试**与**一键应用运行**：

1. **桌面端（零依赖推荐）**：直接启动 `/Applications/Yanshi Studio.app`（或先运行 `./yanshi-ui-dx/scripts/build.sh`，再执行 `cd yanshi-desktop && cargo tauri dev`）。
2. **Web 端（网关模式）**：构建 `./yanshi-ui-dx/scripts/build.sh` 后运行 `cargo run -p yanshi-server`，浏览器打开：
   - `http://127.0.0.1:8080/ui/app` —— **App Runner**：表单化输入，一键并发生成并在线预览幻灯片、1080p 视频与 PDF。
   - `http://127.0.0.1:8080/ui/studio` —— **Studio 调试台**：在纯 Rust SVG+HTML 拓扑画布上拖拽添加节点（含 `illustration`、`figure`、`render_slides` 等）、勾选「每步暂停观测」或点击节点断点，在右侧实时检视求值后的 Input / Output 与内联媒体产物。

---

## 5. 场景四：Skill 编排与 Harness 治理会话（20 分钟）

### 5.1 用 Planner Skill 一次并行调度多个生成器

[`examples/applications/content-studio`](../examples/applications/content-studio) 将多个生成器封装为并发安全的 `WorkflowTool`，Agent 在单轮发出多个工具调用时会跨多核线程真并行执行（子工作流内亦默认以 `--max-concurrency 8` 并发调度）：

```bash
yanshi skill run examples/applications/content-studio \
  --input url=https://example.org/article \
  --input want=report,mindmap,flashcards \
  --max-concurrency 8 \
  --output-dir out/bundle
```

### 5.2 受治理的 Agent 会话（Harness Mode）

```bash
yanshi harness run "调研 ./docs 目录结构并总结模块边界" \
  --skill examples/ecosystem/skills/research-assistant \
  --approve cli \
  --profile local \
  --max-steps 12 \
  --cost-limit-usd 0.50
```

- 非幂等工具、`Publish` 发布动作以及超出成本阈值的操作均会触发交互式审批（支持单次放行、按 SHA-256 参数摘要缓存或 `q` `DenyAndStop` 立即终止）。
- `skill.toml` 的 `[[knowledge]]` 支持 `files` 内联与 `rag` 索引（内置中英文/日文双字切分 BM25 与 MMR，亦可选 `chunk_strategy = "semantic"`）。

---

## 6. 诊断与 Trace 复盘

每次执行都会在 `~/.yanshi/traces`（或 `YANSHI_TRACE_DIR`）留下包含 `cost_usd` 与工具策略决策的结构化轨迹：

```bash
yanshi workflow debug examples/generators/illustrated_slides.yml --visualize --analyze
yanshi trace replay <run_id>
yanshi trace tui <run_id> --filter tool --details
```

---

## 7. 常用命令速查

```bash
# 环境与模型体检
yanshi doctor --profile local
yanshi config validate
yanshi llm models

# 运行、批量并行与续跑工作流（默认 concurrent(8)，结果导出到 --output-dir）
yanshi workflow validate <yml> --strict
yanshi workflow run <yml> --input KEY VALUE --checkpoint --output-dir out
yanshi workflow run-batch a.yml b.yml --max-parallel 2 --cost-limit-usd 1.00 --output-dir out/batch
yanshi workflow resume <run_id> <yml> --output-dir out

# Skill 与多成品编排
yanshi skill validate <dir> && yanshi skill list-tools <dir>
yanshi skill run examples/applications/content-studio --input url=<url> --input want=report,slides --output-dir out

# 动态工作流（先 dry-run 审阅计划）
yanshi workflow dynamic --goal "..." -m <model> --allow-path ./docs --dry-run

# 桌面端打包与安装（macOS）
./scripts/install_desktop.sh --open
```
