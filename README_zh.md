# Yanshi 偃师

[![Rust](https://img.shields.io/badge/rust-1.85%2B%20(2024)-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](Cargo.toml)
[![Website](https://img.shields.io/badge/website-yanshi.dev-0f6e66.svg)](https://yanshi.dev/)
[![Documentation](https://img.shields.io/badge/docs-manual-green.svg)](https://yanshi.dev/docs/)

**[English](README.md)** | **简体中文** · **[产品官网](https://yanshi.dev/)** · **[在线文档手册](https://yanshi.dev/docs/)** · **[18 套 Studio 模板图谱](docs/STUDIO_CATALOGUE.md)** · **[10 分钟试用手册](docs/TRIAL_GUIDE.md)**

> **基于 Rust 构建的多模态内容生产与工作流/智能体运行时：原始素材输入（一句话想法、网页 URL、GitHub 代码库、学术 PDF 或音视频录音），成品交付物直接输出（原生可编辑 OOXML `.pptx` 演示文稿、`1080p` 逐页配音讲解视频、`16:9` Typst `.pdf`、Excel 实验对比表、双人深度播客、矢量信息图与自校验工程架构图/电路图）——底层统一承载于确定性 DAG 工作流、原生智能体循环、Skills、MCP 工具、RAG、全链路追踪与断点续跑机制之上。**

**名字由来。** 《列子·汤问》记载能工巧匠**偃师**向周穆王献上能歌善舞的机关人；将其拆解开来，不过是革、木、胶、漆等构件按精密结构拼接而成，重新合拢后又“活动如初”。这是中国古籍中关于自主智能体最早的工程记载，也是本项目的核心理念：**以显式、可拆解、可审计的结构契约（`Flow` 有向无环图、`Tool` 契约、`Skill` 能力包）组装智能体与多模态流水线，让每一次运行都可追踪、可重放、可重组。**

**产品定位（Content Studio Y0–Y10 已全面收官）。** Yanshi 将多模态内容生产收敛为可预测、可测试的五阶段确定性流水线 —— **Sources（素材摄取）→ Planner（内容规划）→ Generators（结构化生成）→ Renderers（确定性渲染）→ Artifacts（交付产物）**，内置 **18 套开箱即用生成器模板**（[`docs/STUDIO_CATALOGUE.md`](docs/STUDIO_CATALOGUE.md)）以及跨平台 **`Yanshi Studio`** Web 与 Tauri v2 桌面端应用。LLM 与多智能体监督器负责规划与博弈打磨*内容本身*（大纲、角色设定锁、多轮辩论、图表源码），由经过严格 Schema 校验的确定性 DAG 模板驱动执行：

- **一等公民媒体与交付物契约**：`FlowValue::File` 在节点间类型安全地传递多媒体与文档产物，自动生成 SHA-256 校验清单，支持 CLI `--output-dir` 落盘导出与网关 `/v1/runs/{id}/artifacts` 流式下载。
- **纯 Rust 原生可编辑幻灯片与出版级排版**：内置纯 Rust OOXML `pptx_writer`，直接输出 PowerPoint / Keynote / WPS 原生可编辑形状、`<a:tbl>` 表格、`<p:cxnSp>` 连接线与演讲者备注（绝非整页截图贴图），同时输出 `16:9` 宽屏 Typst `.pdf` 与离线交互 `.html`。
- **嵌入式渲染器 + 系统沙箱隔离**：内置纯 Rust 渲染器（`render_slides`、`render_table`、`render_svg`、`render_mindmap`、`render_study`、`render_markdown`）与受 OS 级沙箱严格隔离的外部排版/媒体工具（`typst`、`pandoc`、`ffmpeg`、`d2`、`lualatex` 及容器隔离 Python）协同工作。
- **全模态成本预算与付费节点断点续跑**：通过 `cost_limit_usd` 统一计量并硬性约束 LLM + 生图 + TTS + ASR + 视频总开销；配合 `--checkpoint` / `workflow resume` 跳过已完成节点续跑，绝不重复计费。

完整设计见 [`docs/RFC_CONTENT_STUDIO.md`](docs/RFC_CONTENT_STUDIO.md) 与 [`RoadMap.md`](RoadMap.md)；当前已实现能力边界与供应商能力矩阵见 [`docs/CURRENT_STATUS.md`](docs/CURRENT_STATUS.md) 与 [`docs/LLM_PROVIDERS_MATRIX.md`](docs/LLM_PROVIDERS_MATRIX.md)。*（注：Yanshi 是多模态内容生产与工作流/智能体运行时，而非代码补全类 IDE 编程助手框架。）*

---

## 🎬 Content Studio 内置生成器模板（18 套开箱即用）

[`examples/generators/`](examples/generators/) 下的全部 18 套工作流均通过 `yanshi workflow validate --strict` 严格校验，可在 CLI、HTTP 网关（`/v1/runs`）、Web Studio（`/ui/studio` 与 `/ui/app`）以及 `Yanshi Studio` 桌面端一致运行。也可在 [产品官网 `yanshi.dev`](https://yanshi.dev/#catalogue) 点击任意模板卡片直接在线检视完整 YAML 源码与节点拓扑。

### 1. 主题演讲 PPT、配音视频与双人播客

| 模板文件 | 核心输入 | 交付产物 | 核心亮点 |
| --- | --- | --- | --- |
| [`illustrated_slides.yml`](examples/generators/illustrated_slides.yml) | `subject`, `audience`, `slide_count`, `slide_theme`, `style_preset` | `.pptx`, `16:9 .pdf`, `.html` | 联网检索考证 + 三路智能视觉路由（`illustration` 6 种概念插画预设、D2 矢量架构图、原生 OOXML 表格/代码框） |
| [`tech_slides_video.yml`](examples/generators/tech_slides_video.yml) | `topic`, `repo_url`, `pdf_source`, `presenter_mode` | `1080p .mp4`, `.srt`/`.vtt`, `.pptx`, `.pdf` | 渐进式分步动画与毫秒级情感 TTS 配音对齐，支持聚光灯高亮框与画中画（PiP）主讲人合成 |
| [`slides.yml`](examples/generators/slides.yml) | `url` / `text` / `chunks`, `focus`, `enable_illustrations` | `.pptx`, `16:9 .pdf`, `.html` | 基于源文档事实依据生成技术胶片，内置编译错误自动闭环修复的 `figure` 矢量图（默认无需图像 API Key） |
| [`video_overview.yml`](examples/generators/video_overview.yml) | `url` / `text`, `image_model`, `tts_model`, `video_model` | `.mp4`, `.srt` | 多幕分镜并行插画或 `text_to_video` 动态镜头生成 + 逐幕配音与字幕音视频混流 |
| [`audio_overview.yml`](examples/generators/audio_overview.yml) | `url` / `text`, `minutes`, `host_voice`, `guest_voice` | `.mp3` | 双人对话式深度播客脚本，双声线并行合成并经 `render_audio` 完成 EBU R128 广播级响度标准化 |

### 2. 日常研发与学术精读工作流

| 模板文件 | 核心输入 | 交付产物 | 核心亮点 |
| --- | --- | --- | --- |
| [`repo_architecture.yml`](examples/generators/repo_architecture.yml) | `source`（GitHub URL 或本地路径）, `git_ref`, `focus` | `2x .svg`/`.png`, `.pdf`, `.html`, `.md` | `repo_snapshot` 静态扫描目录树、依赖清单与模块引用拓扑，零不可信代码执行风险 |
| [`backend_project_planner.yml`](examples/generators/backend_project_planner.yml) | `idea`, `tech_preference`, `interactive`, `max_questions` | `6x .svg`, `.sql` DDL, `.pdf` 设计书 | 支持多轮 HITL `ask_user` 交互式需求澄清，确认规模约束后一次性输出架构图、UML、ER 图、PostgreSQL DDL 与设计规范书 |
| [`paper_reader.yml`](examples/generators/paper_reader.yml) | `source`（arXiv 链接、PDF URL 或本地 PDF）, `target_lang` | `.pdf`, `.xlsx`/`.csv`, `.svg`, `.html` | 双语精读报告 + 实验基准数据提取导出为数值化 Excel 表格 + D2 方法流程图与思维导图 |
| [`video_summarizer.yml`](examples/generators/video_summarizer.yml) | `source`（视频 URL 或本地音视频）, `subtitle_langs` | `.pdf`, `.html`, `.srt`, `.svg` | `video_ingest` 优先提取原生字幕，无字幕时自动回退至 FFmpeg 抽轨 + `asr` 语音转写并生成带时间戳纪要 |

### 3. 溯源深度报告、工程制图与学习套件

| 模板文件 | 核心输入 | 交付产物 | 核心亮点 |
| --- | --- | --- | --- |
| [`report.yml`](examples/generators/report.yml) | `url` / `text` / `chunks`, `focus`, `sections`, `pdf` | `.pdf`, `.html`, `.md` | 按大纲分节 `map` 并行撰写，每处论据绑定原文分块逐字引文脚注并经 Typst 排版输出 PDF |
| [`infographic.yml`](examples/generators/infographic.yml) | `url` / `text` / `chunks`, `focus` | `.svg`, `.png` | 提炼核心指标与对比洞察，经由确定性 Tera SVG 海报引擎渲染高清矢量图与高分 PNG |
| [`mindmap.yml`](examples/generators/mindmap.yml) | `url` / `text` / `chunks`, `depth` | `.svg`, `.png`, `.html` | 层次化概念树抽取，同时生成高清矢量 SVG/PNG 与可折叠离线交互 markmap HTML |
| [`circuit_diagram.yml`](examples/generators/circuit_diagram.yml) | `description`, `text` | `.svg`, `.png` | 先抽取类型化元器件清单校验连接拓扑，再调用沙箱内 `circuitikz` + `lualatex` 编译精准电路原理图 |
| [`flashcards.yml`](examples/generators/flashcards.yml) | `url` / `text` / `chunks`, `count` | `.html`, `.json` | 将技术文档或教材转化为单文件离线可翻转记忆闪卡与结构化 JSON 题集 |
| [`quiz.yml`](examples/generators/quiz.yml) | `url` / `text` / `chunks`, `count` | `.html`, `.json` | 生成自评分单选测验页，提供即时选项解析反馈与得分统计 |

### 4. 多智能体创意制片与思辨工坊

| 模板文件 | 核心输入 | 交付产物 | 核心亮点 |
| --- | --- | --- | --- |
| [`episodic_drama_studio.yml`](examples/generators/episodic_drama_studio.yml) | `premise`, `episode_count`, `existing_cast_sheet`, `existing_series_bible` | `1080p .mp4`, `.srt`, `.pdf` | `multi_agent (blackboard)` 编剧室 + `while` + `image_understand` 定妆母图审片 + 跨集 `image_to_image` / I2V 锁脸与角色固定声纹，支持跨次无限续拍 |
| [`debate_podcast_show.yml`](examples/generators/debate_podcast_show.yml) | `motion_topic`, `pro_position`, `con_position`, `turn_count` | `三声线 .mp3`, `.png`, `.pdf` | `multi_agent (debate)` 正反独立人格多轮真实攻防 + 主持人终审边界裁决，合成为广播级响度标准化播客与思辨特刊 |
| [`worldbuilding_bible.yml`](examples/generators/worldbuilding_bible.yml) | `seed_idea`, `medium_target`, `art_direction`, `max_art_rounds` | `.pdf`, `.html`, `2x .svg`, `.png` | `multi_agent (handoff)` 三棒专家接力移交 + `while` VLM 主视觉打磨闭环 + D2 阵营制衡图与分支抉择状态图 |

### 三路幻灯片智能视觉路由（Slide Visual Routing）

幻灯片工作流（`illustrated_slides.yml`、`slides.yml`、`tech_slides_video.yml`）会自动识别每一页的表达意图并分发至最合适的渲染管线：
1. **`illustration`（AI 艺术指导概念插画）** — 封面、核心隐喻、场景类比与总结页交由 [`IllustrationNode`](yanshi-nodes-ai/src/nodes/illustration.rs) 渲染，内置 LLM 艺术总监提示词精炼、深浅色画布色调融合（`dark` `#0B1120` / `light` `#F8FAFC`）、防乱码文字护栏，并提供 6 种预设风格（`editorial_tech`、`isometric_3d`、`flat_vector`、`blueprint_schematic`、`scientific_diagram`、`cinematic_keynote`）。
2. **`arch_diagram`（自修复确定性架构图）** — 系统拓扑、状态机与数据流页通过 `figure` 节点调用 `d2` / `svg` / `typst` / `tikz` / 沙箱 `python` 编译精准矢量图，编译报错时自动回传诊断信息闭环修复。
3. **`structured_panel`（原生可编辑结构化面板）** — 代码走读、基准对比矩阵与检查清单直接通过纯 Rust OOXML 写出 PowerPoint 原生可编辑形状、表格、连接线与等宽代码标注框。

---

## 🚀 Content Studio 快速上手（CLI · Web · 桌面端）

```bash
# 1. 生成精美插图技术演讲 PPT（原生可编辑 .pptx + 16:9 .pdf + 交互 .html）
yanshi workflow run examples/generators/illustrated_slides.yml \
  --input subject "Rust 异步运行时与工作窃取调度器" \
  --input style_preset editorial_tech \
  --output-dir out/slides

# 2. 学术论文双语精读（arXiv / 本地 PDF -> 双语 .pdf + 实验数据 .xlsx + D2 方法图）
yanshi workflow run examples/generators/paper_reader.yml \
  --input source "https://arxiv.org/abs/1706.03762" \
  --output-dir out/paper

# 3. GitHub 仓库架构扫描（静态分析导入拓扑 -> D2 系统/模块依赖 SVG + PDF 报告）
yanshi workflow run examples/generators/repo_architecture.yml \
  --input source "https://github.com/tokio-rs/axum" \
  --output-dir out/axum-arch

# 4. 开启逐节点 Checkpoint 断点保护运行长耗时配音视频流水线（中断后可免重复计费续跑）
yanshi workflow run examples/generators/tech_slides_video.yml \
  --checkpoint --timeout 15m \
  --input topic "Rust 零拷贝序列化设计" \
  --output-dir out/video

# 5. 启动 Web Studio (/ui/studio 与 /ui/app) 或一键构建安装 macOS 桌面端 Yanshi Studio.app
cargo run -p yanshi-server
./scripts/install_desktop.sh --open
```

---

## 🏗️ 五层窄腰契约内核架构

Yanshi 包含 25 个 Rust Crate 与嵌入式 Web/Tauri 桌面端，围绕 **L0 窄腰契约内核（Contract Kernel）** 划分为严格的五层架构，并由 `cargo xtask check-arch` 在编译期与 CI 中强制守护八条依赖铁律（各运行时互不依赖，仅依赖共享契约；详见 [`docs/RFC_CRATE_ARCHITECTURE.md`](docs/RFC_CRATE_ARCHITECTURE.md)）：

| 层级 | 职责定位 | 包含 Crates |
| --- | --- | --- |
| **L0 契约内核** | 窄腰抽象层 —— 仅包含共享数据类型与 Trait，零运行时逻辑 | `yanshi-value`（`FlowValue`）、`yanshi-graph`（`Flow` IR、`AsyncNode`、`expr`、`YanshiError`）、`yanshi-store-spi`（`MemoryStore`、`KnowledgeBackend`）、`yanshi-agent-spi`（`AgentRuntime`、`Capability`、HITL 中断/恢复）、`yanshi-async-util`（重试、超时、`race_with_limits`）、`yanshi-tool`（`Tool`、`ToolRegistry`、`ToolPolicy`、`SecurityProfile`、`SandboxBackend`） |
| **L1 执行核心** | 驱动 L0 `Flow` IR 的确定性 DAG 执行引擎 | `yanshi-core`（依赖就绪并发调度器、`FlowExt::run()`、Checkpoint 状态快照、恢复计划、`ResourceLimits`、健康检查与事件流） |
| **L2 能力适配层** | 工具层与 AI 能力节点、6 大模型供应商、RAG、MCP、沙箱实现 | `yanshi-nodes`（确定性 Studio 渲染器与素材摄取节点）、`yanshi-nodes-ai`（`llm`、`figure`、`illustration`、`asr`、`tts`、图像/视频、`mcp`、`rag`）、`yanshi-llm`（OpenAI、Anthropic、Google、Moonshot、StepFun、Mock）、`yanshi-tools`（内置工具 + OS 沙箱后端）、`yanshi-tools-ai`、`yanshi-mcp`、`yanshi-rag`、`yanshi-memory` |
| **L3 智能体与编排** | 智能体运行时、Skills 能力包、Harness 治理壳、YAML 装配与 CLI | `yanshi-agents`（ReAct、Plan-Execute、Reflection、Handoff/Blackboard/Debate 多智能体监督器、`dynamic` 动态工作流编译）、`yanshi-skills`、`yanshi-harness`（钩子、审批、审计、后台任务）、`yanshi-config`、`yanshi-cli` |
| **L4 运维与产品化** | 可观测性、HTTP 网关、分布式 Worker、Web 与原生桌面端 | `yanshi-tracing`（JSONL 追踪、TUI 回放、W3C `traceparent`、OTLP/HTTP 导出）、`yanshi-server`（Axum API + SSE）、`yanshi-db`（Postgres 9 表存储）、`yanshi-worker` + `yanshi-worker-proto`（gRPC 分布式节点）、`yanshi-ui`（React/Vite + Tauri v2）、`crates/gateway`（Cloudflare Pingora TLS/HTTP2 反向代理） |

---

## 🤖 四种执行范式与双向递归组合

所有任务共享同一套底层能力底座（`ToolRegistry`、Skills、MCP、RAG、Memory），按**规划绑定时机（Binding-Time）**从完全静态到逐步自主覆盖四种范式：

- **静态确定性 DAG（`yanshi-core::Flow`）** — 预先通过 YAML 或 Rust 代码声明拓扑，支持依赖就绪并发调度、`run_if` 条件分支、`map` 并行扇出、`while` 循环、重试与断点恢复。
- **原生智能体循环（`yanshi-agents::ReActAgent`）** — 每一步动态执行观察、规划、工具调用、反思与结束前校验（verification-before-stop），内置 Token/成本护栏与 HITL `ask_user` 交互中断。
- **动态编译工作流（`yanshi_agents::dynamic`）** — 由 LLM 在运行期一次性规划声明式 `WorkflowPlan`，即时编译为可重放、可并行调度的 `Flow`（`compile_plan_to_flow` / `yanshi workflow dynamic`），兼具智能体的灵活性与工作流的确定性。
- **Harness 受控治理壳（`yanshi-harness`）** — 为任意运行时包裹前置/后置钩子、交互式或远程人工审批、OS 级沙箱、防篡改审计日志与后台任务限额。

工作流与智能体支持**双向无缝嵌套**：
- 使用 **`AgentNode`** 将智能体作为单个步骤嵌入确定性 DAG 工作流中；
- 使用 **`WorkflowTool`** 将整条 DAG 工作流封装为标准工具供智能体按需调用。

完整三轴心智模型详见 [`docs/ARCHITECTURE.md` § Four Execution Paradigms](docs/ARCHITECTURE.md#four-execution-paradigms--mental-model)。

---

## 🎯 两种构建方式

### 📋 配置优先（声明式 YAML + CLI）

```yaml
# workflow_v2.yml
name: "Source-Grounded Brief"
cost_limit_usd: 0.50
nodes:
  - id: build_prompt
    type: template
    parameters:
      template: "用 3 条要点总结 {{ topic }} 的核心工程权衡。"
      topic: "工作窃取（Work-Stealing）异步调度器"

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

### 💻 代码优先（Rust Core SDK）

```rust
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use yanshi_core::{
  FlowExt, // 为 L0 `Flow` IR 引入 `.run()` 执行扩展方法
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
      "请写一首关于 {{topic}} 的简短技术俳句。",
    ))),
    dependencies: vec![],
    input_mapping: None,
    run_if: None,
    initial_inputs: HashMap::from([(
      "topic".to_string(),
      FlowValue::Json(json!("确定性有向无环图")),
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

## 🛡️ 生产级可靠性、纵深安全与可观测性

### 🔄 状态感知自动重试（避免无效重复计费）
计费型 LLM 与多模态媒体节点按 HTTP 状态码精准分类错误：仅对 `429` 限流、`5xx` 服务端抖动与网络超时执行退避重试；`4xx` 参数或内容审核拒绝立即失败返回，杜绝双倍计费：

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

### 💾 状态内存预算告警与超时控制
`ResourceLimits` 接入 `FlowExecutionConfig::resource_limits`，在不破坏 DAG 下游节点输出完整性的前提下发出结构化 `WorkflowEvent::ResourceWarning` 预算告警，并配合 `TimeoutConfig` 管控全局及单节点超时：

```rust
use yanshi_core::resource_limits::ResourceLimits;
use yanshi_core::timeout::{TimeoutConfig, with_timeout};

let limits = ResourceLimits::builder()
  .max_state_size(100 * 1024 * 1024) // 100 MiB 状态池告警阈值
  .max_value_size(10 * 1024 * 1024)  // 10 MiB 单节点输出告警阈值
  .build();

let timeouts = TimeoutConfig::production();
let output = with_timeout(run_pipeline(), timeouts.workflow_execution_timeout).await?;
```

### 🔁 断点续跑、调试诊断与终端 Trace 回放
通过 CLI 直接完成严格模式校验、DAG 拓扑可视化、空跑预演、断点恢复与终端交互式回放：

```bash
# 严格模式 Schema 校验、DAG 拓扑可视化与 Dry-run 预演
yanshi workflow validate workflow.yml --strict
yanshi workflow debug workflow.yml --visualize
yanshi workflow debug workflow.yml --dry-run --verbose

# 从最近一次成功节点恢复中断的工作流（保留已产生的花销统计与已完成产物）
yanshi workflow resume <run_id> examples/generators/tech_slides_video.yml --output-dir out/video

# 在终端 TUI 中逐步回放运行轨迹与工具调用明细
yanshi trace replay <run_id>
```

### 🔒 纵深防御沙箱与安全基线
- **`SecurityProfile`（`Dev` / `Local` / `Production`）**：`Production` 模式默认遵循失败关闭（Fail-Closed）原则，自动拒绝无鉴权非环回地址监听、未开启沙箱的宿主机访问节点，以及未经 Ed25519 签名校验的插件与市场能力包（详见 [`docs/SECURITY_PROFILES.md`](docs/SECURITY_PROFILES.md)）。
- **OS 原生沙箱与强隔离容器**：内置操作系统级沙箱默认**开启**（macOS `sandbox-exec`；Linux `seccomp` + `Landlock` + `cgroup v2`）。LLM 生成的 Python 代码（`code_exec` 工具与 Python `figure` 绘图）强制运行于断网、硬限 CPU/内存/进程数的 `ContainerBackend` 容器内。
- **防 SSRF 与防 DNS 重绑定**：`HttpTool` 先归一化 IPv4-mapped IPv6 地址再校验私网网段，并在单次请求周期内锁定 DNS 解析结果以彻底消除 TOCTOU 窗口。

---

## 📦 安装与工程质量门禁

### 安装 CLI 或桌面端应用
```bash
# 从源码编译安装 `yanshi` 命令行工具并自检本地渲染环境
cargo install --path yanshi-cli
yanshi doctor

# 一键构建并安装 macOS 桌面应用 `Yanshi Studio.app`（内置本地零 Postgres 引擎）
./scripts/install_desktop.sh --open
```

### 作为 Rust SDK 依赖引入
```toml
[dependencies]
yanshi-core = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-nodes = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-nodes-ai = { git = "https://github.com/yeying-ai/yanshi" }
yanshi-agents = { git = "https://github.com/yeying-ai/yanshi" }
```

### 工作区质量门禁（Pre-Commit Checks）
```bash
cargo fmt --check
cargo clippy --workspace --all-features -- -D warnings
cargo xtask check-arch           # 校验五层架构的八条 Crate 依赖铁律
cargo xtask println-lint         # 确保库代码中零遗留 println!/eprintln!
cargo xtask redaction-lint       # 审计日志宏，防止原始 Prompt 或密钥泄露
cargo test --workspace --all-features
```

---

## 📚 核心文档导航

### 产品入口与上手指南
- **[产品官网（中文首页 `yanshi.dev`）](https://yanshi.dev/)** 与 **[英文首页（`yanshi.dev/en/`）](https://yanshi.dev/en/)**：交互式五阶段工作台、在线 YAML 源码检视器与三路视觉路由画廊。
- **[在线完整手册（`yanshi.dev/docs/`）](https://yanshi.dev/docs/)** 与 **[文档总索引](docs/README.md)**：基于 mdBook 构建的完整技术文档库。
- **[Content Studio 模板图谱与桌面端指南](docs/STUDIO_CATALOGUE.md)**：15 套内置生成器详解、幻灯片视觉路由预设、Web Studio 与桌面端使用说明。
- **[Yanshi 试用手册（中文）](docs/TRIAL_GUIDE.md)** 与 **[可运行中文教程](docs/examples/runnable_tutorials_zh.md)**：从零 API Key Mock 体验到真实多模态产出的分步实操指南。
- **[当前项目状态（`CURRENT_STATUS.md`）](docs/CURRENT_STATUS.md)** 与 **[路线图（`RoadMap.md`）](RoadMap.md)**：已收官里程碑（`Y0–Y10`）与后续演进评估条件。

### 架构设计、契约规范与生产运维
- **[Content Studio RFC](docs/RFC_CONTENT_STUDIO.md)**：五阶段流水线、`FlowValue::File` 产物契约、原文引文溯源模型与渲染器策略。
- **[系统架构总览](docs/ARCHITECTURE.md)** 与 **[Crate 分层架构 RFC](docs/RFC_CRATE_ARCHITECTURE.md)**：五层工作区布局、四种执行范式与 L0 窄腰契约内核设计。
- **[配置指南](docs/CONFIGURATION.md)** 与 **[工作流 YAML Schema](docs/WORKFLOW_SCHEMA.md)**：模型配置、`~/.yanshi/.env`、严格模式校验与运行目录结构。
- **[LLM 供应商能力矩阵](docs/LLM_PROVIDERS_MATRIX.md)**：OpenAI、Anthropic、Google、Moonshot、StepFun 与 Mock 六大供应商能力对照表。
- **[智能体运行时](docs/AGENT_RUNTIME.md)** 与 **[Skills 能力包指南](docs/SKILLS.md)**：ReAct / Plan-Execute 循环、HITL 中断恢复、`SKILL.md` 声明式封装、分层知识库与 MCP 集成。
- **[安全基线配置](docs/SECURITY_PROFILES.md)** 与 **[生产部署指南](docs/DEPLOYMENT.md)**：OS 沙箱、网关准入策略、Docker / Helm 多副本 Postgres 部署与 Pingora TLS 网关。
- **[稳定性与 API 兼容性承诺](docs/STABILITY.md)**：Stable、Beta、Experimental 与 Internal 接口分级说明。

### 精选可运行示例（[完整示例矩阵](examples/README.md)）
- [`examples/generators/`](examples/generators/)：全部 15 套生产级 Content Studio YAML 生成器模板。
- [`examples/applications/content-studio/`](examples/applications/content-studio/)：一次调度多个生成器的多交付物 Planner Skill 示例。
- [`yanshi-core/examples/fixed_dag_workflow.rs`](yanshi-core/examples/fixed_dag_workflow.rs) 与 [`checkpoint_recovery.rs`](yanshi-core/examples/checkpoint_recovery.rs)：确定性 DAG 编排与断点恢复示例。
- [`yanshi-agents/examples/agent_native_react.rs`](yanshi-agents/examples/agent_native_react.rs) 与 [`hybrid_workflow_agent.rs`](yanshi-agents/examples/hybrid_workflow_agent.rs)：原生 ReAct 循环及 `AgentNode` / `WorkflowTool` 双向组合示例。
- [`yanshi-agents/examples/dynamic_workflow_plan.rs`](yanshi-agents/examples/dynamic_workflow_plan.rs)：运行期声明式 `WorkflowPlan` 编译为并行 `Flow` 执行示例。

---

## 📄 开源许可

本项目基于 **MIT License** 开源（详见 [`Cargo.toml`](Cargo.toml)）。
