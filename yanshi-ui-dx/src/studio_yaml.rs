//! Pure-Rust Workflow Studio preset catalogue, YAML topology inspector,
//! DAG graph mutator (add/connect/disconnect/delete/update with cycle checks &
//! cascade renames), and personal template preference persistence.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use serde_yaml::{Mapping, Value as YamlValue};

// ── Inspected Workflow Topology Schema ───────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectedInputKind {
  #[default]
  String,
  Number,
  Boolean,
  Json,
}

impl InspectedInputKind {
  pub const fn as_str(&self) -> &'static str {
    match self {
      Self::String => "string",
      Self::Number => "number",
      Self::Boolean => "boolean",
      Self::Json => "json",
    }
  }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InspectedInput {
  pub name: String,
  #[serde(default)]
  pub description: Option<String>,
  #[serde(default)]
  pub required: bool,
  #[serde(default)]
  pub default: Option<serde_json::Value>,
  #[serde(default)]
  pub kind: InspectedInputKind,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InspectedNode {
  pub id: String,
  #[serde(rename = "type")]
  pub node_type: String,
  #[serde(default)]
  pub dependencies: Vec<String>,
  #[serde(default)]
  pub input_mapping: BTreeMap<String, String>,
  #[serde(default)]
  pub run_if: Option<String>,
  #[serde(default)]
  pub timeout_ms: Option<u64>,
  #[serde(default)]
  pub max_retries: Option<u32>,
  #[serde(default)]
  pub parameters: serde_json::Value,
  #[serde(default)]
  pub sub_nodes: Vec<InspectedNode>,
  #[serde(default)]
  pub level: usize,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InspectedEdgeKind {
  #[default]
  Dependency,
  InputMapping,
  Condition,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectedEdge {
  pub from: String,
  pub to: String,
  #[serde(default)]
  pub kind: InspectedEdgeKind,
  #[serde(default)]
  pub label: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectDiagnostics {
  #[serde(default)]
  pub valid: bool,
  #[serde(default)]
  pub issues: Vec<String>,
  #[serde(default)]
  pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InspectWorkflowResponse {
  #[serde(default)]
  pub valid: bool,
  #[serde(default)]
  pub name: Option<String>,
  #[serde(default)]
  pub cost_limit_usd: Option<f64>,
  #[serde(default)]
  pub inputs: Vec<InspectedInput>,
  #[serde(default)]
  pub nodes: Vec<InspectedNode>,
  #[serde(default)]
  pub edges: Vec<InspectedEdge>,
  #[serde(default)]
  pub execution_order: Vec<String>,
  #[serde(default)]
  pub levels: Vec<Vec<String>>,
  #[serde(default)]
  pub diagnostics: InspectDiagnostics,
}

// ── Built-in 19 Official Generators + 2 Instant Zero-Key Presets ─────────────

pub const PRESET_CATEGORIES: &[&str] = &[
  "全部",
  "课件与视频",
  "创意制片与设定",
  "研究报告与信息图",
  "导图与工程图",
  "播客音频",
  "互动学习",
  "免 Key 演练",
  "个人收藏",
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowPreset {
  pub id: String,
  pub title: String,
  pub subtitle: String,
  pub badge: String,
  pub category: String,
  pub yaml: String,
  pub personal_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonalWorkflowTemplate {
  pub id: String,
  pub title: String,
  pub yaml: String,
  #[serde(default, rename = "updatedAt")]
  pub updated_at: String,
}

pub const PERSONAL_TEMPLATES_INDEX_KEY: &str = "studio.templates.index";
const PERSONAL_TEMPLATE_KEY_PREFIX: &str = "studio.template.";
const MAX_SAFE_TEMPLATE_BYTES: usize = 15 * 1024;

pub fn personal_template_yaml_key(id: &str) -> String {
  format!("{PERSONAL_TEMPLATE_KEY_PREFIX}{id}")
}

pub const INSTANT_STEP_DEBUG_YAML: &str = r#"name: instant_step_debug_demo
inputs:
  topic:
    description: "演练主题（免 API Key 本地秒级单步观测）"
    default: "Rust 所有权与借用检查器"
  difficulty:
    description: "目标深度（入门 / 进阶 / 架构级）"
    default: "架构级"
  include_quiz:
    description: "是否生成随堂思考题分支（触发 run_if 条件边）"
    default: true

nodes:
  - id: normalize_brief
    type: template
    parameters:
      output_format: json
      template: |
        {
          "headline": {{ topic | json_encode() }},
          "level": {{ difficulty | json_encode() }},
          "quiz_enabled": {{ include_quiz }}
        }

  - id: core_outline
    type: template
    dependencies: ["normalize_brief"]
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
      level: "{{ nodes.normalize_brief.outputs.level }}"
    parameters:
      template: |
        ## {{ headline }}（{{ level }}）
        1. 核心心智模型与不变式
        2. 编译器静态消解 vs 运行期开销对比
        3. 工程边界与常见反模式

  - id: quiz_branch
    type: template
    dependencies: ["normalize_brief"]
    run_if: "{{ nodes.normalize_brief.outputs.quiz_enabled }}"
    input_mapping:
      headline: "{{ nodes.normalize_brief.outputs.headline }}"
    parameters:
      template: |
        ### 随堂自测（{{ headline }}）
        - 为什么可变引用在同一时刻具有独占别名语义？
        - 何时应当把生命周期上界收拢到方法签名而非结构体定义？

  - id: assemble_handout
    type: template
    dependencies: ["core_outline", "quiz_branch"]
    input_mapping:
      outline: "{{ nodes.core_outline.outputs.output }}"
      quiz: "{{ nodes.quiz_branch.outputs.output }}"
    parameters:
      template: |
        # 课程讲义汇总

        {{ outline }}

        {% if quiz is defined %}{{ quiz }}{% else %}_（已跳过随堂思考题分支）_{% endif %}
"#;

pub const PARALLEL_SLIDE_PREVIEW_YAML: &str = r#"name: instant_slide_render_demo
inputs:
  deck_title:
    description: "幻灯片主标题（免 LLM 直接验证 SVG/PNG 课件渲染与预览）"
    default: "Rust Trait 分发机制可视化"
  slide_theme:
    description: "配色主题：dark 或 light"
    default: "dark"

nodes:
  - id: sample_deck
    type: template
    parameters:
      output_format: json
      template: |
        {
          "deck": {
            "title": {{ deck_title | json_encode() }},
            "subtitle": "静态单态化 vs vtable 胖指针内存模型",
            "author": "Yanshi Studio",
            "theme": {{ slide_theme | json_encode() }},
            "slides": [
              {
                "section": "01 · 核心抽象",
                "title": "用行为契约替代继承层级",
                "left": {
                  "kind": "code",
                  "language": "rust",
                  "title": "trait Summary",
                  "code": "pub trait Summary {\n  fn summarize(&self) -> String;\n}\n\npub fn notify(item: &impl Summary) {\n  println!(\"Breaking: {}\", item.summarize());\n}",
                  "callouts": [{"line": 5, "text": "静态分发：编译期单态化零虚表开销", "tone": "rust"}]
                },
                "right": {
                  "kind": "table",
                  "columns": ["维度", "Rust Trait", "OOP 继承", "C 函数指针"],
                  "highlight_col": 1,
                  "rows": [
                    ["编译期类型安全", "✓ 强约束", "✓ 强约束", "✗ void* 易出错"],
                    ["运行期零开销", "✓ 单态化内联", "✗ 虚表跳转", "✗ 间接调用"],
                    ["为外部类型扩展", "✓ 孤儿规则内支持", "✗ 需改基类", "⚠ 手工封装"]
                  ]
                },
                "takeaway": "优先使用泛型静态分发，仅在异构集合边界按需退化到 &dyn Trait。",
                "notes": "我们先看 Trait 的声明方式。编译器通过单态化把泛型调用直接内联。右边对比了三种抽象方案的取舍。"
              }
            ]
          }
        }

  - id: render_preview
    type: render_slides
    dependencies: ["sample_deck"]
    input_mapping:
      deck: "{{ nodes.sample_deck.outputs.deck }}"
    parameters:
      width: 1920
      progressive: true
"#;

pub fn built_in_workflow_presets() -> Vec<WorkflowPreset> {
  let rows: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
      "illustrated_slides",
      "主题调研与精美配图 PPT (Illustrated Slides)",
      "联网检索 → 规划大纲 → 艺术总监精炼配图 + D2 架构图 → 原生 OOXML 可编辑 PPTX + 16:9 PDF + HTML",
      "日常生产力",
      "课件与视频",
      include_str!("../../examples/generators/illustrated_slides.yml"),
    ),
    (
      "repo_architecture",
      "源码 / GitHub 仓库架构与依赖图 (Repo Architecture)",
      "自动扫描代码树、Manifest 与依赖边 → D2 系统分层架构图 + 模块依赖 DAG 图 + 深度报告",
      "日常生产力",
      "导图与工程图",
      include_str!("../../examples/generators/repo_architecture.yml"),
    ),
    (
      "backend_project_planner",
      "多轮对话后端项目全套规划 (Backend Planner)",
      "多轮交互澄清需求 → 系统架构图 / 模块图 / UML 类图与时序图 / 流程图 / ER 图 + PG DDL + 导图",
      "日常生产力",
      "导图与工程图",
      include_str!("../../examples/generators/backend_project_planner.yml"),
    ),
    (
      "paper_reader",
      "学术论文双语精读与实验数据提取 (Paper Reader)",
      "支持 PDF 文件或 arXiv 链接 → 双语精译 + 核心思想论点 + 实验数据表格 XLSX/CSV + 导图 + PDF",
      "日常生产力",
      "研究报告与信息图",
      include_str!("../../examples/generators/paper_reader.yml"),
    ),
    (
      "video_summarizer",
      "视频观点提炼与字幕导图生成 (Video Summarizer)",
      "支持视频 URL 或本地文件 → 原生字幕直提或 FFmpeg+ASR 生成 .srt → 时间轴观点精要 + 思维导图",
      "日常生产力",
      "课件与视频",
      include_str!("../../examples/generators/video_summarizer.yml"),
    ),
    (
      "tech_slides_video",
      "拟真人 PPT 演播级技术精讲视频 (Keynote Presenter Video)",
      "多源支撑 → 视线引导口播 + 精美插图/D2 架构图 → 按步配音三模式 1080p 视频 + 原生可编辑 PPTX / 16:9 PDF",
      "旗舰工作流",
      "课件与视频",
      include_str!("../../examples/generators/tech_slides_video.yml"),
    ),
    (
      "episodic_drama_studio",
      "角色一致多集连续剧制片厂 (Episodic Drama Studio)",
      "黑板编剧室 → VLM 闭环定妆母图 → 跨集图生图/图生视频锚定角色外观 + 专属声纹配音 → 连续剧成片 MP4 + 设定圣经 PDF",
      "多范式创意",
      "创意制片与设定",
      include_str!("../../examples/generators/episodic_drama_studio.yml"),
    ),
    (
      "debate_podcast_show",
      "多智能体三声线圆桌辩论播客 (Debate Podcast Show)",
      "正反双方 2 轮对抗辩论 + 主持终审裁决 → 三声线 TTS + EBU R128 响度母带 MP3 + 概念封面 + 思辨特刊 PDF",
      "多范式思辨",
      "播客音频",
      include_str!("../../examples/generators/debate_podcast_show.yml"),
    ),
    (
      "worldbuilding_bible",
      "多专家接力原创世界观设定圣经 (Worldbuilding Bible)",
      "三专家 Handoff 接力（纪元法则 → 阵营人物 → 分支任务）→ VLM 审片主视觉 + D2 势力/叙事状态图 + 导图 + 精装设定集 PDF",
      "多范式创意",
      "创意制片与设定",
      include_str!("../../examples/generators/worldbuilding_bible.yml"),
    ),
    (
      "slides",
      "通用主题演示课件 (Slides)",
      "来源资料自动提炼大纲、结构化卡片与矢量图表，直出原生可编辑 OOXML PPTX 与 HTML 放映页",
      "官方生成器",
      "课件与视频",
      include_str!("../../examples/generators/slides.yml"),
    ),
    (
      "video_overview",
      "多镜头视频概览 (Video Overview)",
      "自动编排分镜脚本、视觉画面、配音旁白与字幕合成完整 MP4 解说视频",
      "官方生成器",
      "课件与视频",
      include_str!("../../examples/generators/video_overview.yml"),
    ),
    (
      "report",
      "深度溯源研究报告 (Report)",
      "大纲规划 → 多章节并行撰写与原文引用脚注 → 导出 Markdown / HTML / PDF",
      "官方生成器",
      "研究报告与信息图",
      include_str!("../../examples/generators/report.yml"),
    ),
    (
      "infographic",
      "结构化信息长图 (Infographic)",
      "提炼核心指标与分栏对比结论，排版渲染高清 SVG 与 PNG 视觉信息长图",
      "官方生成器",
      "研究报告与信息图",
      include_str!("../../examples/generators/infographic.yml"),
    ),
    (
      "data_table",
      "结构化数据提取表 (Data Table)",
      "从网页或文档精准抽取对比维度与指标表格，附带逐行原文引用并导出 XLSX / CSV / HTML",
      "官方生成器",
      "研究报告与信息图",
      include_str!("../../examples/generators/data_table.yml"),
    ),
    (
      "mindmap",
      "知识全景思维导图 (Mindmap)",
      "从网页或长文本抽取多层概念树，输出矢量 SVG、PNG 与可交互缩放 Markmap HTML",
      "官方生成器",
      "导图与工程图",
      include_str!("../../examples/generators/mindmap.yml"),
    ),
    (
      "circuit_diagram",
      "工程电路原理图 (Circuit Diagram)",
      "元器件清单抽取 + CircuiTikZ 编译闭环自检，输出工程级 SVG 与 PNG 原理图",
      "工程制图",
      "导图与工程图",
      include_str!("../../examples/generators/circuit_diagram.yml"),
    ),
    (
      "audio_overview",
      "双人播客对谈 (Audio Overview)",
      "主持人与嘉宾自然问答脚本生成 + 多音色并行合成 + FFmpeg 混音导出播客音频",
      "官方生成器",
      "播客音频",
      include_str!("../../examples/generators/audio_overview.yml"),
    ),
    (
      "flashcards",
      "互动翻转记忆闪卡 (Flashcards)",
      "围绕核心概念生成问答卡片集与免依赖点击翻转复习 HTML 单页",
      "互动学习",
      "互动学习",
      include_str!("../../examples/generators/flashcards.yml"),
    ),
    (
      "quiz",
      "即时反馈自测题库 (Quiz)",
      "自动生成多选题、干扰项与答案解析，输出可交互判分 HTML 测验页",
      "互动学习",
      "互动学习",
      include_str!("../../examples/generators/quiz.yml"),
    ),
    (
      "instant_slide_render_demo",
      "免 Key 幻灯片秒级渲染演练",
      "2 节点直出 1080p 代码/对比矩阵幻灯片与交互式 HTML，立即可观测图像预览",
      "秒级图像观测",
      "免 Key 演练",
      PARALLEL_SLIDE_PREVIEW_YAML,
    ),
    (
      "instant_step_debug_demo",
      "免 Key 单步与条件分支演练",
      "4 节点含 run_if 条件边，零外部依赖体验每步暂停核查输入/输出",
      "调试入门",
      "免 Key 演练",
      INSTANT_STEP_DEBUG_YAML,
    ),
  ];

  rows
    .iter()
    .map(
      |(id, title, subtitle, badge, category, yaml)| WorkflowPreset {
        id: (*id).to_string(),
        title: (*title).to_string(),
        subtitle: (*subtitle).to_string(),
        badge: (*badge).to_string(),
        category: (*category).to_string(),
        yaml: (*yaml).to_string(),
        personal_id: None,
      },
    )
    .collect()
}

// ── 27-Item Node Palette ─────────────────────────────────────────────────────

pub const NODE_PALETTE_CATEGORIES: &[&str] =
  &["LLM / Agent", "多模态生成", "内容渲染器", "工具与控制流"];

#[derive(Clone, Debug, PartialEq)]
pub struct NodePaletteItem {
  pub node_type: &'static str,
  pub label: &'static str,
  pub category: &'static str,
  pub description: &'static str,
  pub default_input_key: &'static str,
  pub default_parameters: serde_json::Value,
}

pub fn node_palette() -> Vec<NodePaletteItem> {
  vec![
    NodePaletteItem {
      node_type: "llm",
      label: "LLM 推理 (llm)",
      category: "LLM / Agent",
      description: "调用大模型执行提示词生成、结构化 JSON 抽取或改写",
      default_input_key: "context",
      default_parameters: serde_json::json!({
        "prompt": "请围绕以下上下文生成结构化分析：\n\n{{ context | default(\"\") }}"
      }),
    },
    NodePaletteItem {
      node_type: "agent",
      label: "自主循环 Agent (agent)",
      category: "LLM / Agent",
      description: "ReAct / Plan-Execute 多步工具调用智能体节点",
      default_input_key: "task",
      default_parameters: serde_json::json!({
        "prompt": "{{ task | default(\"完成调研并输出结论\") }}",
        "max_steps": 6
      }),
    },
    NodePaletteItem {
      node_type: "skill_agent",
      label: "Skill 专家 (skill_agent)",
      category: "LLM / Agent",
      description: "挂载 SKILL.md 领域人设、知识与工具集的智能体",
      default_input_key: "task",
      default_parameters: serde_json::json!({
        "prompt": "{{ task | default(\"执行领域专家分析\") }}"
      }),
    },
    NodePaletteItem {
      node_type: "tts",
      label: "语音合成 (tts)",
      category: "多模态生成",
      description: "将文稿旁白合成为 WAV/MP3 音频文件",
      default_input_key: "text",
      default_parameters: serde_json::json!({
        "voice": "alloy",
        "input_template": "{{ text }}"
      }),
    },
    NodePaletteItem {
      node_type: "illustration",
      label: "概念艺术插图 (illustration)",
      category: "多模态生成",
      description: "艺术指导级概念配图与批量幻灯片插画生成（支持无杂乱文字约束与统一画风）",
      default_input_key: "concept",
      default_parameters: serde_json::json!({
        "concept": "{{ concept }}",
        "style": "editorial_tech",
        "aspect_ratio": "16:9"
      }),
    },
    NodePaletteItem {
      node_type: "text_to_image",
      label: "文生图 (text_to_image)",
      category: "多模态生成",
      description: "根据视觉提示词生成插画或封面图",
      default_input_key: "visual_prompt",
      default_parameters: serde_json::json!({
        "prompt": "{{ visual_prompt }}",
        "size": "1024x1024"
      }),
    },
    NodePaletteItem {
      node_type: "text_to_video",
      label: "文生视频 (text_to_video)",
      category: "多模态生成",
      description: "生成动态视频片段（支持时长与画幅设置）",
      default_input_key: "video_prompt",
      default_parameters: serde_json::json!({
        "prompt": "{{ video_prompt }}",
        "aspect_ratio": "16:9",
        "duration_seconds": 5
      }),
    },
    NodePaletteItem {
      node_type: "asr",
      label: "语音转写 (asr)",
      category: "多模态生成",
      description: "将音频输入转写为带时间戳的文本",
      default_input_key: "audio_source",
      default_parameters: serde_json::json!({
        "audio_source": "{{ audio_source }}"
      }),
    },
    NodePaletteItem {
      node_type: "render_slides",
      label: "幻灯片渲染 (render_slides)",
      category: "内容渲染器",
      description: "将结构化 deck JSON 渲染为原生 OOXML .pptx、16:9 PDF 讲义、PNG/SVG 与 HTML 课件",
      default_input_key: "deck",
      default_parameters: serde_json::json!({
        "width": 1920,
        "progressive": true,
        "export_pptx": true
      }),
    },
    NodePaletteItem {
      node_type: "render_video",
      label: "课件视频合成 (render_video)",
      category: "内容渲染器",
      description: "将幻灯片帧与分段配音通过受控 ffmpeg 合成为旁白视频",
      default_input_key: "slides",
      default_parameters: serde_json::json!({ "fps": 30 }),
    },
    NodePaletteItem {
      node_type: "render_document",
      label: "文档/PDF 排版 (render_document)",
      category: "内容渲染器",
      description: "通过受控 pandoc / typst 将 Markdown 排版为 PDF 或 HTML 报告",
      default_input_key: "markdown",
      default_parameters: serde_json::json!({ "format": "html" }),
    },
    NodePaletteItem {
      node_type: "render_mindmap",
      label: "思维导图渲染 (render_mindmap)",
      category: "内容渲染器",
      description: "将层级主题树渲染为高清 SVG 思维导图",
      default_input_key: "tree",
      default_parameters: serde_json::json!({}),
    },
    NodePaletteItem {
      node_type: "render_study",
      label: "闪卡/测验渲染 (render_study)",
      category: "内容渲染器",
      description: "将题库或闪卡 JSON 渲染为单文件交互式学习网页",
      default_input_key: "study",
      default_parameters: serde_json::json!({ "mode": "flashcards" }),
    },
    NodePaletteItem {
      node_type: "render_audio",
      label: "播客混音拼接 (render_audio)",
      category: "内容渲染器",
      description: "拼接多段对谈音频并执行响度标准化",
      default_input_key: "segments",
      default_parameters: serde_json::json!({}),
    },
    NodePaletteItem {
      node_type: "render_svg",
      label: "SVG 光栅化 (render_svg)",
      category: "内容渲染器",
      description: "校验 SVG 矢量图并输出 PNG 预览",
      default_input_key: "svg",
      default_parameters: serde_json::json!({ "width": 1600 }),
    },
    NodePaletteItem {
      node_type: "render_markdown",
      label: "Markdown 渲染 (render_markdown)",
      category: "内容渲染器",
      description: "将结构化章节拼接输出干净的 Markdown 成品文件",
      default_input_key: "markdown",
      default_parameters: serde_json::json!({}),
    },
    NodePaletteItem {
      node_type: "render_table",
      label: "数据表格渲染 (render_table)",
      category: "内容渲染器",
      description: "将行列数据渲染为精美 SVG/HTML 对比矩阵",
      default_input_key: "table",
      default_parameters: serde_json::json!({}),
    },
    NodePaletteItem {
      node_type: "figure",
      label: "代码绘图 (figure)",
      category: "内容渲染器",
      description: "LLM 编写 SVG/Typst/TikZ 源码并带编译错误自修复闭环",
      default_input_key: "spec",
      default_parameters: serde_json::json!({ "format": "svg" }),
    },
    NodePaletteItem {
      node_type: "template",
      label: "Jinja 模板 (template)",
      category: "工具与控制流",
      description: "零外部调用的确定性文本/JSON 组装与转换节点",
      default_input_key: "upstream",
      default_parameters: serde_json::json!({ "template": "{{ upstream | default(\"\") }}" }),
    },
    NodePaletteItem {
      node_type: "http",
      label: "HTTP 请求 (http)",
      category: "工具与控制流",
      description: "带 SSRF 防护与 DNS 锁定的外部 API 请求节点",
      default_input_key: "query",
      default_parameters: serde_json::json!({ "method": "GET", "url": "https://example.com" }),
    },
    NodePaletteItem {
      node_type: "web_search",
      label: "联网检索 (web_search)",
      category: "工具与控制流",
      description: "多厂商联网搜索（Tavily/Bocha/Serper/Brave + 免 Key 聚合回退）",
      default_input_key: "query",
      default_parameters: serde_json::json!({
        "query": "{{ query | default(\"\") }}",
        "provider": "auto",
        "max_results": 6
      }),
    },
    NodePaletteItem {
      node_type: "repo_snapshot",
      label: "代码仓库快照 (repo_snapshot)",
      category: "工具与控制流",
      description: "提取 GitHub 仓库或受控本地源码目录的模块树、Manifest 与依赖边",
      default_input_key: "source",
      default_parameters: serde_json::json!({ "source": "https://github.com/tokio-rs/axum" }),
    },
    NodePaletteItem {
      node_type: "pdf_extract",
      label: "PDF 论文解析 (pdf_extract)",
      category: "工具与控制流",
      description: "沙箱提取 arXiv/OpenReview/本地 PDF 全文与分页结构",
      default_input_key: "source",
      default_parameters: serde_json::json!({ "source": "https://arxiv.org/abs/1706.03762" }),
    },
    NodePaletteItem {
      node_type: "video_ingest",
      label: "视频/字幕采集 (video_ingest)",
      category: "工具与控制流",
      description: "优先提取原生软字幕，自动回退沙箱 ffmpeg 16kHz 音频抽取供 ASR 转写",
      default_input_key: "source",
      default_parameters: serde_json::json!({
        "source": "{{ source | default(\"\") }}",
        "prefer_subtitles": true
      }),
    },
    NodePaletteItem {
      node_type: "file",
      label: "受控文件读写 (file)",
      category: "工具与控制流",
      description: "在白名单目录内读取或落盘本地文件",
      default_input_key: "content",
      default_parameters: serde_json::json!({
        "allowed_paths": ["./workspace"],
        "operation": "write",
        "path": "./workspace/output.txt"
      }),
    },
    NodePaletteItem {
      node_type: "map",
      label: "Map 并发子流 (map)",
      category: "工具与控制流",
      description: "对数组输入逐项（支持并发上限）执行内嵌子流水线",
      default_input_key: "items",
      default_parameters: serde_json::json!({
        "parallel": true,
        "max_concurrent": 4,
        "template": [{
          "id": "item_step",
          "type": "template",
          "parameters": { "template": "{{ item }}" }
        }]
      }),
    },
    NodePaletteItem {
      node_type: "while",
      label: "While 迭代循环 (while)",
      category: "工具与控制流",
      description: "在满足条件且未超最大轮次时循环执行子流水线",
      default_input_key: "seed",
      default_parameters: serde_json::json!({
        "condition": "false",
        "max_iterations": 3,
        "do": [{
          "id": "loop_step",
          "type": "template",
          "parameters": { "template": "{{ seed | default(\"\") }}" }
        }]
      }),
    },
  ]
}

// ── Helpers for Node Reference Parsing & Rewriting ───────────────────────────

fn is_node_id_byte(b: u8) -> bool {
  b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

pub fn is_valid_node_id(id: &str) -> bool {
  !id.is_empty() && id.bytes().all(is_node_id_byte)
}

/// Extracts unique `<id>` occurrences from `nodes.<id>` references inside Tera expressions.
pub fn extract_node_refs(expr: &str) -> Vec<String> {
  let mut refs = Vec::new();
  let bytes = expr.as_bytes();
  let needle = b"nodes.";
  let mut i = 0usize;
  while i + needle.len() < bytes.len() {
    if &bytes[i..i + needle.len()] == needle {
      let boundary_ok = i == 0 || !is_node_id_byte(bytes[i - 1]);
      if boundary_ok {
        let start = i + needle.len();
        let mut end = start;
        while end < bytes.len() && is_node_id_byte(bytes[end]) {
          end += 1;
        }
        if end > start {
          let candidate = &expr[start..end];
          if !refs.iter().any(|existing| existing == candidate) {
            refs.push(candidate.to_string());
          }
          i = end;
          continue;
        }
      }
    }
    i += 1;
  }
  refs
}

pub fn rewrite_node_id_references(expr: &str, old_id: &str, new_id: &str) -> String {
  if old_id.is_empty() || old_id == new_id {
    return expr.to_string();
  }
  let target = format!("nodes.{old_id}");
  let replacement = format!("nodes.{new_id}");
  let bytes = expr.as_bytes();
  let target_bytes = target.as_bytes();
  let mut out = String::with_capacity(expr.len());
  let mut i = 0usize;
  while i < bytes.len() {
    if i + target_bytes.len() <= bytes.len() && &bytes[i..i + target_bytes.len()] == target_bytes {
      let left_ok = i == 0 || !is_node_id_byte(bytes[i - 1]);
      let right_idx = i + target_bytes.len();
      let right_ok = right_idx == bytes.len() || !is_node_id_byte(bytes[right_idx]);
      if left_ok && right_ok {
        out.push_str(&replacement);
        i = right_idx;
        continue;
      }
    }
    let ch = expr[i..].chars().next().unwrap_or('\0');
    out.push(ch);
    i += ch.len_utf8();
  }
  out
}

fn parse_mapping_source(expr: &str) -> Option<(String, String)> {
  let inner = expr
    .trim()
    .strip_prefix("{{")
    .unwrap_or(expr.trim())
    .strip_suffix("}}")
    .unwrap_or(expr.trim())
    .trim();
  let parts: Vec<&str> = inner.split('.').collect();
  if parts.len() == 4 && parts[0] == "nodes" && parts[2] == "outputs" {
    Some((parts[1].to_string(), parts[3].to_string()))
  } else {
    None
  }
}

fn infer_input_kind(default_val: Option<&serde_json::Value>) -> InspectedInputKind {
  match default_val {
    Some(serde_json::Value::Bool(_)) => InspectedInputKind::Boolean,
    Some(serde_json::Value::Number(_)) => InspectedInputKind::Number,
    Some(serde_json::Value::Array(_)) | Some(serde_json::Value::Object(_)) => {
      InspectedInputKind::Json
    }
    _ => InspectedInputKind::String,
  }
}

pub fn default_input_string(spec: &InspectedInput) -> String {
  match &spec.default {
    None | Some(serde_json::Value::Null) => match spec.kind {
      InspectedInputKind::Boolean => "false".to_string(),
      InspectedInputKind::Json => "{}".to_string(),
      InspectedInputKind::String | InspectedInputKind::Number => String::new(),
    },
    Some(val) => match spec.kind {
      InspectedInputKind::Json => serde_json::to_string_pretty(val).unwrap_or_default(),
      _ => match val {
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
      },
    },
  }
}

pub fn coerce_inputs_payload(
  specs: &[InspectedInput],
  draft_values: &HashMap<String, String>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
  let mut payload = serde_json::Map::new();
  for spec in specs {
    let raw = draft_values
      .get(&spec.name)
      .cloned()
      .unwrap_or_else(|| default_input_string(spec));
    match spec.kind {
      InspectedInputKind::Boolean => {
        payload.insert(spec.name.clone(), serde_json::Value::Bool(raw == "true"));
      }
      InspectedInputKind::Number => {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
          if spec.required {
            return Err(format!("必填数值参数「{}」不能为空", spec.name));
          }
          continue;
        }
        let num = trimmed
          .parse::<f64>()
          .ok()
          .filter(|n| n.is_finite())
          .ok_or_else(|| format!("参数「{}」需要合法数字，当前为 \"{raw}\"", spec.name))?;
        let json_num = serde_json::Number::from_f64(num)
          .ok_or_else(|| format!("参数「{}」数值超出 JSON 范围", spec.name))?;
        payload.insert(spec.name.clone(), serde_json::Value::Number(json_num));
      }
      InspectedInputKind::Json => {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
          continue;
        }
        let parsed: serde_json::Value = serde_json::from_str(trimmed)
          .map_err(|err| format!("参数「{}」JSON 解析失败：{err}", spec.name))?;
        payload.insert(spec.name.clone(), parsed);
      }
      InspectedInputKind::String => {
        if spec.required && raw.trim().is_empty() {
          return Err(format!("必填参数「{}」不能为空", spec.name));
        }
        payload.insert(spec.name.clone(), serde_json::Value::String(raw));
      }
    }
  }
  Ok(payload)
}

// ── Local Client-Side YAML Topology Inspector ────────────────────────────────

pub fn inspect_yaml_locally(yaml_text: &str) -> InspectWorkflowResponse {
  let raw_doc: serde_json::Value = match serde_yaml::from_str(yaml_text) {
    Ok(v) => v,
    Err(err) => {
      return InspectWorkflowResponse {
        valid: false,
        diagnostics: InspectDiagnostics {
          valid: false,
          issues: vec![format!("YAML 语法错误: {err}")],
          warnings: Vec::new(),
        },
        ..Default::default()
      };
    }
  };

  let Some(doc) = raw_doc.as_object() else {
    return InspectWorkflowResponse {
      valid: false,
      diagnostics: InspectDiagnostics {
        valid: false,
        issues: vec!["工作流顶层必须是 YAML 对象（包含 name 与 nodes）".to_string()],
        warnings: Vec::new(),
      },
      ..Default::default()
    };
  };

  let name = doc
    .get("name")
    .and_then(serde_json::Value::as_str)
    .map(str::to_string);
  let cost_limit_usd = doc
    .get("cost_limit_usd")
    .and_then(serde_json::Value::as_f64);

  let mut inputs: Vec<InspectedInput> = doc
    .get("inputs")
    .and_then(serde_json::Value::as_object)
    .map(|map| {
      map
        .iter()
        .map(|(key, val)| {
          let spec = val.as_object();
          let default_val = spec.and_then(|s| s.get("default")).cloned();
          let kind = infer_input_kind(default_val.as_ref());
          InspectedInput {
            name: key.clone(),
            description: spec
              .and_then(|s| s.get("description"))
              .and_then(serde_json::Value::as_str)
              .map(str::to_string),
            required: spec
              .and_then(|s| s.get("required"))
              .and_then(serde_json::Value::as_bool)
              .unwrap_or(false),
            default: default_val,
            kind,
          }
        })
        .collect()
    })
    .unwrap_or_default();
  inputs.sort_by(|a, b| {
    b.required
      .cmp(&a.required)
      .then_with(|| a.name.cmp(&b.name))
  });

  let raw_nodes = doc
    .get("nodes")
    .and_then(serde_json::Value::as_array)
    .cloned()
    .unwrap_or_default();

  let mut known_ids = HashSet::new();
  for item in &raw_nodes {
    if let Some(id) = item
      .as_object()
      .and_then(|o| o.get("id"))
      .and_then(serde_json::Value::as_str)
      && !id.is_empty()
    {
      known_ids.insert(id.to_string());
    }
  }

  let mut edges: Vec<InspectedEdge> = Vec::new();
  let mut base_nodes: Vec<InspectedNode> = Vec::new();

  for item in &raw_nodes {
    let Some(obj) = item.as_object() else {
      continue;
    };
    let id = obj
      .get("id")
      .and_then(serde_json::Value::as_str)
      .unwrap_or_default()
      .to_string();
    if id.is_empty() {
      continue;
    }
    let node_type = obj
      .get("type")
      .and_then(serde_json::Value::as_str)
      .unwrap_or("unknown")
      .to_string();

    let dependencies: Vec<String> = obj
      .get("dependencies")
      .and_then(serde_json::Value::as_array)
      .map(|arr| {
        arr
          .iter()
          .filter_map(serde_json::Value::as_str)
          .map(str::to_string)
          .collect()
      })
      .unwrap_or_default();

    let mut input_mapping = BTreeMap::new();
    if let Some(map_obj) = obj
      .get("input_mapping")
      .and_then(serde_json::Value::as_object)
    {
      for (k, v) in map_obj {
        if let Some(s) = v.as_str() {
          input_mapping.insert(k.clone(), s.to_string());
        }
      }
    }

    let run_if = obj
      .get("run_if")
      .and_then(serde_json::Value::as_str)
      .map(str::to_string);

    let mut connected_from = HashSet::new();
    let mut by_source: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (target_key, expr) in &input_mapping {
      if let Some((src, out_field)) = parse_mapping_source(expr)
        && src != id
        && known_ids.contains(&src)
      {
        by_source
          .entry(src)
          .or_default()
          .push(format!("{out_field} → {target_key}"));
      }
    }
    for (src, labels) in by_source {
      connected_from.insert(src.clone());
      edges.push(InspectedEdge {
        from: src,
        to: id.clone(),
        kind: InspectedEdgeKind::InputMapping,
        label: Some(labels.join(", ")),
      });
    }

    if let Some(cond) = &run_if {
      for src in extract_node_refs(cond) {
        if src != id && known_ids.contains(&src) {
          connected_from.insert(src.clone());
          edges.push(InspectedEdge {
            from: src,
            to: id.clone(),
            kind: InspectedEdgeKind::Condition,
            label: Some("run_if".to_string()),
          });
        }
      }
    }

    for dep in &dependencies {
      if dep != &id && known_ids.contains(dep) && !connected_from.contains(dep) {
        connected_from.insert(dep.clone());
        edges.push(InspectedEdge {
          from: dep.clone(),
          to: id.clone(),
          kind: InspectedEdgeKind::Dependency,
          label: None,
        });
      }
    }

    let mut params_map = obj
      .get("parameters")
      .and_then(serde_json::Value::as_object)
      .cloned()
      .unwrap_or_default();

    let mut sub_nodes = Vec::new();
    let sub_key = match node_type.as_str() {
      "map" => Some("template"),
      "while" => Some("do"),
      _ => None,
    };
    if let Some(sk) = sub_key
      && let Some(arr) = params_map.get(sk).and_then(serde_json::Value::as_array)
    {
      for (idx, child) in arr.iter().enumerate() {
        if let Some(cobj) = child.as_object()
          && let Some(cid) = cobj.get("id").and_then(serde_json::Value::as_str)
        {
          sub_nodes.push(InspectedNode {
            id: cid.to_string(),
            node_type: cobj
              .get("type")
              .and_then(serde_json::Value::as_str)
              .unwrap_or("unknown")
              .to_string(),
            parameters: cobj
              .get("parameters")
              .cloned()
              .unwrap_or_else(|| serde_json::json!({})),
            level: idx,
            ..Default::default()
          });
        }
      }
      params_map.remove(sk);
    }

    base_nodes.push(InspectedNode {
      id,
      node_type,
      dependencies,
      input_mapping,
      run_if,
      timeout_ms: obj.get("timeout_ms").and_then(serde_json::Value::as_u64),
      max_retries: obj
        .get("max_retries")
        .and_then(serde_json::Value::as_u64)
        .and_then(|v| u32::try_from(v).ok()),
      parameters: serde_json::Value::Object(params_map),
      sub_nodes,
      level: 0,
    });
  }

  // Kahn's topological sort + layered level assignment.
  let mut in_degree: HashMap<String, usize> = HashMap::new();
  let mut adj: HashMap<String, BTreeSet<String>> = HashMap::new();
  let mut level_map: HashMap<String, usize> = HashMap::new();

  for node in &base_nodes {
    in_degree.insert(node.id.clone(), 0);
    adj.insert(node.id.clone(), BTreeSet::new());
    level_map.insert(node.id.clone(), 0);
  }
  for edge in &edges {
    if let Some(set) = adj.get_mut(&edge.from)
      && set.insert(edge.to.clone())
      && let Some(deg) = in_degree.get_mut(&edge.to)
    {
      *deg += 1;
    }
  }

  let mut seeds: Vec<String> = in_degree
    .iter()
    .filter(|&(_, deg)| *deg == 0)
    .map(|(id, _)| id.clone())
    .collect();
  seeds.sort();
  let mut queue: VecDeque<String> = seeds.into_iter().collect();
  let mut order = Vec::new();

  while let Some(u) = queue.pop_front() {
    let u_level = level_map.get(&u).copied().unwrap_or(0);
    order.push(u.clone());
    let neighbors: Vec<String> = adj
      .get(&u)
      .map(|s| s.iter().cloned().collect())
      .unwrap_or_default();
    for v in neighbors {
      let cur_v_level = level_map.get(&v).copied().unwrap_or(0);
      level_map.insert(v.clone(), cur_v_level.max(u_level + 1));
      if let Some(deg) = in_degree.get_mut(&v) {
        *deg = deg.saturating_sub(1);
        if *deg == 0 {
          queue.push_back(v);
        }
      }
    }
  }

  let mut issues = Vec::new();
  if base_nodes.is_empty() {
    issues.push("工作流至少需要定义 1 个节点".to_string());
  }
  if order.len() != base_nodes.len() {
    issues.push("检测到节点循环依赖（DAG 必须无环）".to_string());
  }

  let mut max_level = 0usize;
  for node in &mut base_nodes {
    let lvl = level_map.get(&node.id).copied().unwrap_or(0);
    node.level = lvl;
    max_level = max_level.max(lvl);
  }

  let mut levels = if order.is_empty() {
    Vec::new()
  } else {
    vec![Vec::new(); max_level + 1]
  };
  for id in &order {
    let lvl = level_map.get(id).copied().unwrap_or(0);
    if let Some(col) = levels.get_mut(lvl) {
      col.push(id.clone());
    }
  }

  let valid = issues.is_empty();
  InspectWorkflowResponse {
    valid,
    name,
    cost_limit_usd,
    inputs,
    nodes: base_nodes,
    edges,
    execution_order: order,
    levels,
    diagnostics: InspectDiagnostics {
      valid,
      issues,
      warnings: Vec::new(),
    },
  }
}

// ── YAML Graph Mutator ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NodeUpdatePatch {
  pub new_id: Option<String>,
  pub node_type: Option<String>,
  pub dependencies: Option<Vec<String>>,
  pub input_mapping: Option<BTreeMap<String, String>>,
  pub run_if: Option<Option<String>>,
  pub timeout_ms: Option<Option<u64>>,
  pub max_retries: Option<Option<u32>>,
  pub parameters: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YamlMutationSuccess {
  pub yaml: String,
  pub created_node_id: Option<String>,
  pub renamed_to: Option<String>,
}

fn ykey(k: &str) -> YamlValue {
  YamlValue::String(k.to_string())
}

fn json_to_yaml(val: &serde_json::Value) -> YamlValue {
  serde_yaml::to_value(val).unwrap_or(YamlValue::Null)
}

fn parse_workflow_root_and_nodes(yaml_text: &str) -> Result<(Mapping, Vec<YamlValue>), String> {
  let doc: YamlValue =
    serde_yaml::from_str(yaml_text).map_err(|err| format!("YAML 语法解析失败：{err}"))?;
  let mut root = match doc {
    YamlValue::Mapping(m) => m,
    _ => return Err("工作流顶层必须是 YAML 映射对象".to_string()),
  };
  let nodes_val = root
    .remove(ykey("nodes"))
    .unwrap_or_else(|| YamlValue::Sequence(Vec::new()));
  let nodes = match nodes_val {
    YamlValue::Sequence(seq) => seq,
    _ => return Err("工作流 `nodes` 字段必须是数组序列".to_string()),
  };
  Ok((root, nodes))
}

fn serialize_workflow(mut root: Mapping, nodes: Vec<YamlValue>) -> Result<String, String> {
  root.insert(ykey("nodes"), YamlValue::Sequence(nodes));
  serde_yaml::to_string(&YamlValue::Mapping(root)).map_err(|err| format!("YAML 序列化失败：{err}"))
}

fn node_map_id(map: &Mapping) -> Option<String> {
  map
    .get(ykey("id"))
    .and_then(YamlValue::as_str)
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_string)
}

fn build_adjacency(nodes: &[YamlValue]) -> HashMap<String, HashSet<String>> {
  let known: HashSet<String> = nodes
    .iter()
    .filter_map(YamlValue::as_mapping)
    .filter_map(node_map_id)
    .collect();
  let mut adj: HashMap<String, HashSet<String>> = known
    .iter()
    .map(|id| (id.clone(), HashSet::new()))
    .collect();

  for item in nodes {
    let Some(map) = item.as_mapping() else {
      continue;
    };
    let Some(target_id) = node_map_id(map) else {
      continue;
    };
    if let Some(YamlValue::Sequence(deps)) = map.get(ykey("dependencies")) {
      for d in deps {
        if let Some(dep_id) = d.as_str()
          && dep_id != target_id
          && known.contains(dep_id)
          && let Some(set) = adj.get_mut(dep_id)
        {
          set.insert(target_id.clone());
        }
      }
    }
    if let Some(YamlValue::Mapping(mapping)) = map.get(ykey("input_mapping")) {
      for (_, val) in mapping {
        if let Some(expr) = val.as_str() {
          for src in extract_node_refs(expr) {
            if src != target_id
              && known.contains(&src)
              && let Some(set) = adj.get_mut(&src)
            {
              set.insert(target_id.clone());
            }
          }
        }
      }
    }
    if let Some(run_if) = map.get(ykey("run_if")).and_then(YamlValue::as_str) {
      for src in extract_node_refs(run_if) {
        if src != target_id
          && known.contains(&src)
          && let Some(set) = adj.get_mut(&src)
        {
          set.insert(target_id.clone());
        }
      }
    }
  }
  adj
}

fn can_reach(adj: &HashMap<String, HashSet<String>>, start: &str, goal: &str) -> bool {
  if start == goal {
    return true;
  }
  let mut visited = HashSet::new();
  visited.insert(start.to_string());
  let mut queue = VecDeque::new();
  queue.push_back(start.to_string());
  while let Some(cur) = queue.pop_front() {
    if let Some(nexts) = adj.get(&cur) {
      for n in nexts {
        if n == goal {
          return true;
        }
        if visited.insert(n.clone()) {
          queue.push_back(n.clone());
        }
      }
    }
  }
  false
}

fn has_any_cycle(adj: &HashMap<String, HashSet<String>>) -> bool {
  let mut in_deg: HashMap<String, usize> = adj.keys().map(|k| (k.clone(), 0)).collect();
  for targets in adj.values() {
    for t in targets {
      if let Some(d) = in_deg.get_mut(t) {
        *d += 1;
      }
    }
  }
  let mut queue: VecDeque<String> = in_deg
    .iter()
    .filter(|&(_, d)| *d == 0)
    .map(|(k, _)| k.clone())
    .collect();
  let mut visited = 0usize;
  while let Some(u) = queue.pop_front() {
    visited += 1;
    if let Some(targets) = adj.get(&u) {
      for v in targets {
        if let Some(d) = in_deg.get_mut(v) {
          *d = d.saturating_sub(1);
          if *d == 0 {
            queue.push_back(v.clone());
          }
        }
      }
    }
  }
  visited != adj.len()
}

fn default_mapping_for_target(
  source_id: &str,
  source_type: &str,
  target_type: &str,
  existing_keys: &HashSet<String>,
) -> (String, String) {
  let palette = node_palette();
  let preferred = palette
    .iter()
    .find(|p| p.node_type == target_type)
    .map(|p| p.default_input_key)
    .unwrap_or("input");
  let key = if existing_keys.contains(preferred) {
    format!("from_{source_id}")
  } else {
    preferred.to_string()
  };

  if target_type == "render_slides" && key == "deck" {
    let out_field = if source_type == "template" || source_type == "llm" {
      "deck"
    } else {
      "output"
    };
    return (
      key,
      format!("{{{{ nodes.{source_id}.outputs.{out_field} }}}}"),
    );
  }
  (key, format!("{{{{ nodes.{source_id}.outputs.output }}}}"))
}

fn generate_unique_node_id(
  existing: &HashSet<String>,
  node_type: &str,
  preferred: Option<&str>,
) -> String {
  let raw_base = preferred
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .unwrap_or(if node_type.is_empty() {
      "step"
    } else {
      node_type
    });
  let base: String = raw_base
    .chars()
    .map(|c| {
      if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
        c
      } else {
        '_'
      }
    })
    .collect();
  if preferred.is_some() && !existing.contains(&base) {
    return base;
  }
  let mut idx = 1usize;
  while existing.contains(&format!("{base}_{idx}")) {
    idx += 1;
  }
  format!("{base}_{idx}")
}

fn strip_upstream_refs_from_node_map(map: &mut Mapping, removed_id: &str) {
  if let Some(YamlValue::Sequence(deps)) = map.get_mut(ykey("dependencies")) {
    deps.retain(|item| item.as_str() != Some(removed_id));
    if deps.is_empty() {
      map.remove(ykey("dependencies"));
    }
  }
  if let Some(YamlValue::Mapping(mapping)) = map.get_mut(ykey("input_mapping")) {
    let keys_to_remove: Vec<YamlValue> = mapping
      .iter()
      .filter(|&(_, v)| {
        v.as_str()
          .is_some_and(|expr| extract_node_refs(expr).iter().any(|r| r == removed_id))
      })
      .map(|(k, _)| k.clone())
      .collect();
    for k in keys_to_remove {
      mapping.remove(&k);
    }
    if mapping.is_empty() {
      map.remove(ykey("input_mapping"));
    }
  }
  if let Some(run_if) = map.get(ykey("run_if")).and_then(YamlValue::as_str)
    && extract_node_refs(run_if).iter().any(|r| r == removed_id)
  {
    map.remove(ykey("run_if"));
  }
}

pub fn add_node_to_yaml(
  yaml_text: &str,
  node_type: &str,
  after_node_id: Option<&str>,
  preferred_id: Option<&str>,
) -> Result<YamlMutationSuccess, String> {
  let (root, mut nodes) = parse_workflow_root_and_nodes(yaml_text)?;
  let existing_ids: HashSet<String> = nodes
    .iter()
    .filter_map(YamlValue::as_mapping)
    .filter_map(node_map_id)
    .collect();
  let created_id = generate_unique_node_id(&existing_ids, node_type, preferred_id);
  let palette_item = node_palette()
    .into_iter()
    .find(|p| p.node_type == node_type);

  let mut node_map = Mapping::new();
  node_map.insert(ykey("id"), YamlValue::String(created_id.clone()));
  node_map.insert(ykey("type"), YamlValue::String(node_type.to_string()));

  if let Some(up_id) = after_node_id
    && let Some(up_map) = nodes
      .iter()
      .filter_map(YamlValue::as_mapping)
      .find(|m| node_map_id(m).as_deref() == Some(up_id))
  {
    let up_type = up_map
      .get(ykey("type"))
      .and_then(YamlValue::as_str)
      .unwrap_or("unknown");
    let (m_key, m_expr) = default_mapping_for_target(up_id, up_type, node_type, &HashSet::new());
    node_map.insert(
      ykey("dependencies"),
      YamlValue::Sequence(vec![YamlValue::String(up_id.to_string())]),
    );
    let mut imap = Mapping::new();
    imap.insert(ykey(&m_key), YamlValue::String(m_expr));
    node_map.insert(ykey("input_mapping"), YamlValue::Mapping(imap));
  }

  let default_params = palette_item
    .map(|p| json_to_yaml(&p.default_parameters))
    .unwrap_or_else(|| YamlValue::Mapping(Mapping::new()));
  node_map.insert(ykey("parameters"), default_params);

  nodes.push(YamlValue::Mapping(node_map));
  let yaml = serialize_workflow(root, nodes)?;
  Ok(YamlMutationSuccess {
    yaml,
    created_node_id: Some(created_id),
    renamed_to: None,
  })
}

pub fn connect_nodes_in_yaml(
  yaml_text: &str,
  source_id: &str,
  target_id: &str,
) -> Result<YamlMutationSuccess, String> {
  if source_id == target_id {
    return Err("节点不能连接到自身（禁止自环）".to_string());
  }
  let (root, mut nodes) = parse_workflow_root_and_nodes(yaml_text)?;

  let source_type = nodes
    .iter()
    .filter_map(YamlValue::as_mapping)
    .find(|m| node_map_id(m).as_deref() == Some(source_id))
    .and_then(|m| m.get(ykey("type")).and_then(YamlValue::as_str))
    .map(str::to_string)
    .ok_or_else(|| format!("未找到待连接节点（{source_id} → {target_id}）"))?;

  let target_idx = nodes
    .iter()
    .position(|v| v.as_mapping().and_then(node_map_id).as_deref() == Some(target_id))
    .ok_or_else(|| format!("未找到待连接节点（{source_id} → {target_id}）"))?;

  let adj = build_adjacency(&nodes);
  if can_reach(&adj, target_id, source_id) {
    return Err(format!(
      "无法连接 {source_id} → {target_id}：该连线会形成有向环路（DAG 必须无环）"
    ));
  }

  let target_map = nodes
    .get_mut(target_idx)
    .and_then(YamlValue::as_mapping_mut)
    .ok_or_else(|| format!("节点 {target_id} 结构无效"))?;

  let target_type = target_map
    .get(ykey("type"))
    .and_then(YamlValue::as_str)
    .unwrap_or("unknown")
    .to_string();

  match target_map.get_mut(ykey("dependencies")) {
    Some(YamlValue::Sequence(seq)) => {
      if !seq.iter().any(|v| v.as_str() == Some(source_id)) {
        seq.push(YamlValue::String(source_id.to_string()));
      }
    }
    _ => {
      target_map.insert(
        ykey("dependencies"),
        YamlValue::Sequence(vec![YamlValue::String(source_id.to_string())]),
      );
    }
  }

  let mut existing_keys = HashSet::new();
  let mut already_refs_source = false;
  if let Some(YamlValue::Mapping(imap)) = target_map.get(ykey("input_mapping")) {
    for (k, v) in imap {
      if let Some(ks) = k.as_str() {
        existing_keys.insert(ks.to_string());
      }
      if let Some(vs) = v.as_str()
        && extract_node_refs(vs).iter().any(|r| r == source_id)
      {
        already_refs_source = true;
      }
    }
  }

  if !already_refs_source {
    let (m_key, m_expr) =
      default_mapping_for_target(source_id, &source_type, &target_type, &existing_keys);
    match target_map.get_mut(ykey("input_mapping")) {
      Some(YamlValue::Mapping(imap)) => {
        imap.insert(ykey(&m_key), YamlValue::String(m_expr));
      }
      _ => {
        let mut imap = Mapping::new();
        imap.insert(ykey(&m_key), YamlValue::String(m_expr));
        target_map.insert(ykey("input_mapping"), YamlValue::Mapping(imap));
      }
    }
  }

  let yaml = serialize_workflow(root, nodes)?;
  Ok(YamlMutationSuccess {
    yaml,
    created_node_id: None,
    renamed_to: None,
  })
}

pub fn disconnect_edge_in_yaml(
  yaml_text: &str,
  source_id: &str,
  target_id: &str,
) -> Result<YamlMutationSuccess, String> {
  let (root, mut nodes) = parse_workflow_root_and_nodes(yaml_text)?;
  let target_map = nodes
    .iter_mut()
    .filter_map(YamlValue::as_mapping_mut)
    .find(|m| node_map_id(m).as_deref() == Some(target_id))
    .ok_or_else(|| format!("未找到目标节点 {target_id}"))?;
  strip_upstream_refs_from_node_map(target_map, source_id);
  let yaml = serialize_workflow(root, nodes)?;
  Ok(YamlMutationSuccess {
    yaml,
    created_node_id: None,
    renamed_to: None,
  })
}

pub fn delete_node_from_yaml(
  yaml_text: &str,
  node_id: &str,
) -> Result<YamlMutationSuccess, String> {
  let (root, mut nodes) = parse_workflow_root_and_nodes(yaml_text)?;
  let pos = nodes
    .iter()
    .position(|v| v.as_mapping().and_then(node_map_id).as_deref() == Some(node_id))
    .ok_or_else(|| format!("节点 \"{node_id}\" 不存在"))?;
  nodes.remove(pos);
  for item in &mut nodes {
    if let Some(map) = item.as_mapping_mut() {
      strip_upstream_refs_from_node_map(map, node_id);
    }
  }
  let yaml = serialize_workflow(root, nodes)?;
  Ok(YamlMutationSuccess {
    yaml,
    created_node_id: None,
    renamed_to: None,
  })
}

pub fn update_node_in_yaml(
  yaml_text: &str,
  node_id: &str,
  patch: &NodeUpdatePatch,
) -> Result<YamlMutationSuccess, String> {
  let (root, mut nodes) = parse_workflow_root_and_nodes(yaml_text)?;
  let target_idx = nodes
    .iter()
    .position(|v| v.as_mapping().and_then(node_map_id).as_deref() == Some(node_id))
    .ok_or_else(|| format!("未找到节点 \"{node_id}\""))?;

  let mut effective_id = node_id.to_string();
  let mut renamed_to = None;

  if let Some(raw_new_id) = &patch.new_id {
    let candidate = raw_new_id.trim();
    if candidate != node_id {
      if !is_valid_node_id(candidate) {
        return Err("节点 ID 仅允许字母、数字、下划线与连字符".to_string());
      }
      if nodes
        .iter()
        .filter_map(YamlValue::as_mapping)
        .any(|m| node_map_id(m).as_deref() == Some(candidate))
      {
        return Err(format!("节点 ID \"{candidate}\" 已存在，请换一个名称"));
      }
      if let Some(map) = nodes
        .get_mut(target_idx)
        .and_then(YamlValue::as_mapping_mut)
      {
        map.insert(ykey("id"), YamlValue::String(candidate.to_string()));
      }
      effective_id = candidate.to_string();
      renamed_to = Some(candidate.to_string());

      for (idx, item) in nodes.iter_mut().enumerate() {
        if idx == target_idx {
          continue;
        }
        let Some(map) = item.as_mapping_mut() else {
          continue;
        };
        if let Some(YamlValue::Sequence(deps)) = map.get_mut(ykey("dependencies")) {
          for dep in deps.iter_mut() {
            if dep.as_str() == Some(node_id) {
              *dep = YamlValue::String(candidate.to_string());
            }
          }
        }
        if let Some(YamlValue::Mapping(imap)) = map.get_mut(ykey("input_mapping")) {
          for (_, val) in imap.iter_mut() {
            if let Some(expr) = val.as_str() {
              *val = YamlValue::String(rewrite_node_id_references(expr, node_id, candidate));
            }
          }
        }
        if let Some(run_if) = map
          .get(ykey("run_if"))
          .and_then(YamlValue::as_str)
          .map(str::to_string)
        {
          map.insert(
            ykey("run_if"),
            YamlValue::String(rewrite_node_id_references(&run_if, node_id, candidate)),
          );
        }
      }
    }
  }

  let target_map = nodes
    .get_mut(target_idx)
    .and_then(YamlValue::as_mapping_mut)
    .ok_or_else(|| format!("节点 \"{effective_id}\" 结构无效"))?;

  if let Some(nt) = &patch.node_type
    && !nt.trim().is_empty()
  {
    target_map.insert(ykey("type"), YamlValue::String(nt.trim().to_string()));
  }

  if let Some(deps) = &patch.dependencies {
    let mut cleaned = Vec::new();
    let mut seen = HashSet::new();
    for d in deps {
      let trimmed = d.trim();
      if !trimmed.is_empty() && trimmed != effective_id && seen.insert(trimmed.to_string()) {
        cleaned.push(YamlValue::String(trimmed.to_string()));
      }
    }
    if cleaned.is_empty() {
      target_map.remove(ykey("dependencies"));
    } else {
      target_map.insert(ykey("dependencies"), YamlValue::Sequence(cleaned));
    }
  }

  if let Some(imap) = &patch.input_mapping {
    let mut next_map = Mapping::new();
    for (k, v) in imap {
      let trimmed_k = k.trim();
      if !trimmed_k.is_empty() {
        next_map.insert(ykey(trimmed_k), YamlValue::String(v.clone()));
      }
    }
    if next_map.is_empty() {
      target_map.remove(ykey("input_mapping"));
    } else {
      target_map.insert(ykey("input_mapping"), YamlValue::Mapping(next_map));
    }
  }

  if let Some(run_if_opt) = &patch.run_if {
    match run_if_opt
      .as_deref()
      .map(str::trim)
      .filter(|s| !s.is_empty())
    {
      Some(expr) => {
        target_map.insert(ykey("run_if"), YamlValue::String(expr.to_string()));
      }
      None => {
        target_map.remove(ykey("run_if"));
      }
    }
  }

  if let Some(timeout_opt) = patch.timeout_ms {
    match timeout_opt {
      Some(ms) => {
        target_map.insert(ykey("timeout_ms"), YamlValue::Number(ms.into()));
      }
      None => {
        target_map.remove(ykey("timeout_ms"));
      }
    }
  }

  if let Some(retries_opt) = patch.max_retries {
    match retries_opt {
      Some(n) => {
        target_map.insert(ykey("max_retries"), YamlValue::Number(n.into()));
      }
      None => {
        target_map.remove(ykey("max_retries"));
      }
    }
  }

  if let Some(params) = &patch.parameters {
    // Preserve nested `template` / `do` sub-pipelines for `map` / `while` nodes.
    let existing_type = target_map
      .get(ykey("type"))
      .and_then(YamlValue::as_str)
      .unwrap_or("");
    let preserved_sub = match existing_type {
      "map" => target_map
        .get(ykey("parameters"))
        .and_then(YamlValue::as_mapping)
        .and_then(|pm| pm.get(ykey("template")).cloned())
        .map(|v| ("template", v)),
      "while" => target_map
        .get(ykey("parameters"))
        .and_then(YamlValue::as_mapping)
        .and_then(|pm| pm.get(ykey("do")).cloned())
        .map(|v| ("do", v)),
      _ => None,
    };

    let mut next_params = Mapping::new();
    for (k, v) in params {
      next_params.insert(ykey(k), json_to_yaml(v));
    }
    if let Some((sk, sv)) = preserved_sub
      && !next_params.contains_key(ykey(sk))
    {
      next_params.insert(ykey(sk), sv);
    }
    target_map.insert(ykey("parameters"), YamlValue::Mapping(next_params));
  }

  let adj = build_adjacency(&nodes);
  if has_any_cycle(&adj) {
    return Err("本次修改会导致节点循环依赖（DAG 必须无环），请调整上游依赖或映射引用".to_string());
  }

  let yaml = serialize_workflow(root, nodes)?;
  Ok(YamlMutationSuccess {
    yaml,
    created_node_id: None,
    renamed_to,
  })
}

// ── YAML Download Filename & Personal Templates Preferences ──────────────────

pub fn build_yaml_download_filename(raw_name: Option<&str>) -> String {
  let trimmed = raw_name.unwrap_or("").trim();
  if trimmed.is_empty() {
    return "workflow.yml".to_string();
  }
  let lower = trimmed.to_ascii_lowercase();
  if (lower.ends_with(".yml") || lower.ends_with(".yaml"))
    && trimmed
      .bytes()
      .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'-'))
  {
    return trimmed.to_string();
  }
  let stripped = lower
    .strip_suffix(".yml")
    .or_else(|| lower.strip_suffix(".yaml"))
    .unwrap_or(&lower);
  let mut slug = String::new();
  let mut prev_sep = false;
  for ch in stripped.chars() {
    if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
      slug.push(ch);
      prev_sep = false;
    } else if !prev_sep && !slug.is_empty() {
      slug.push('_');
      prev_sep = true;
    }
  }
  let slug = slug.trim_matches('_');
  if slug.is_empty() {
    "workflow.yml".to_string()
  } else {
    format!("{slug}.yml")
  }
}

fn make_template_slug(title: &str, existing_len: usize) -> String {
  let mut ascii = String::new();
  let mut prev_sep = false;
  for ch in title.to_ascii_lowercase().chars() {
    if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
      ascii.push(ch);
      prev_sep = false;
    } else if !prev_sep && !ascii.is_empty() {
      ascii.push('_');
      prev_sep = true;
    }
    if ascii.len() >= 32 {
      break;
    }
  }
  let ascii = ascii.trim_matches('_');
  let suffix = format!("{:x}", (title.len() * 31 + existing_len * 17 + 42) & 0xffff);
  if ascii.is_empty() {
    format!("tpl_{suffix}")
  } else {
    format!("{ascii}_{suffix}")
  }
}

pub fn build_personal_templates_preference_patch(
  existing: &[PersonalWorkflowTemplate],
  existing_id: Option<&str>,
  title_raw: &str,
  yaml: &str,
  now_iso: &str,
) -> Result<(Vec<PersonalWorkflowTemplate>, serde_json::Value), String> {
  let title = if title_raw.trim().is_empty() {
    "未命名个人模板".to_string()
  } else {
    title_raw.trim().to_string()
  };
  if yaml.trim().is_empty() {
    return Err("工作流 YAML 内容不能为空".to_string());
  }
  let payload_bytes = serde_json::json!({ "yaml": yaml }).to_string().len();
  if payload_bytes > MAX_SAFE_TEMPLATE_BYTES {
    return Err("单个收藏模板大小不能超过 15 KiB，请导出为本地 .yml 文件保存".to_string());
  }

  let id = existing_id
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_string)
    .unwrap_or_else(|| make_template_slug(&title, existing.len()));

  let record = PersonalWorkflowTemplate {
    id: id.clone(),
    title,
    yaml: yaml.to_string(),
    updated_at: now_iso.to_string(),
  };

  let mut next = vec![record];
  for item in existing {
    if item.id != id {
      next.push(item.clone());
    }
  }
  next.truncate(24);

  let index_entries: Vec<serde_json::Value> = next
    .iter()
    .map(|item| {
      serde_json::json!({
        "id": item.id,
        "title": item.title,
        "updatedAt": item.updated_at,
      })
    })
    .collect();

  let mut patch = serde_json::Map::new();
  patch.insert(
    PERSONAL_TEMPLATES_INDEX_KEY.to_string(),
    serde_json::json!({ "items": index_entries }),
  );
  patch.insert(
    personal_template_yaml_key(&id),
    serde_json::json!({ "yaml": yaml }),
  );
  Ok((next, serde_json::Value::Object(patch)))
}

pub fn remove_personal_template_patch(
  existing: &[PersonalWorkflowTemplate],
  template_id: &str,
) -> (Vec<PersonalWorkflowTemplate>, serde_json::Value) {
  let next: Vec<PersonalWorkflowTemplate> = existing
    .iter()
    .filter(|item| item.id != template_id)
    .cloned()
    .collect();
  let index_entries: Vec<serde_json::Value> = next
    .iter()
    .map(|item| {
      serde_json::json!({
        "id": item.id,
        "title": item.title,
        "updatedAt": item.updated_at,
      })
    })
    .collect();
  let mut patch = serde_json::Map::new();
  patch.insert(
    PERSONAL_TEMPLATES_INDEX_KEY.to_string(),
    serde_json::json!({ "items": index_entries }),
  );
  patch.insert(
    personal_template_yaml_key(template_id),
    serde_json::json!({ "yaml": "" }),
  );
  (next, serde_json::Value::Object(patch))
}

pub fn restore_personal_templates_from_preferences(
  preferences: Option<&serde_json::Value>,
) -> Vec<PersonalWorkflowTemplate> {
  let Some(prefs) = preferences.and_then(serde_json::Value::as_object) else {
    return Vec::new();
  };
  let Some(items) = prefs
    .get(PERSONAL_TEMPLATES_INDEX_KEY)
    .and_then(serde_json::Value::as_object)
    .and_then(|o| o.get("items"))
    .and_then(serde_json::Value::as_array)
  else {
    return Vec::new();
  };

  let mut out = Vec::new();
  for raw in items {
    let Some(entry) = raw.as_object() else {
      continue;
    };
    let id = entry
      .get("id")
      .and_then(serde_json::Value::as_str)
      .unwrap_or("")
      .trim();
    let title = entry
      .get("title")
      .and_then(serde_json::Value::as_str)
      .unwrap_or("")
      .trim();
    let updated_at = entry
      .get("updatedAt")
      .and_then(serde_json::Value::as_str)
      .unwrap_or("");
    if id.is_empty() || title.is_empty() {
      continue;
    }
    let yaml = prefs
      .get(&personal_template_yaml_key(id))
      .and_then(serde_json::Value::as_object)
      .and_then(|o| o.get("yaml"))
      .and_then(serde_json::Value::as_str)
      .unwrap_or("");
    if yaml.trim().is_empty() {
      continue;
    }
    out.push(PersonalWorkflowTemplate {
      id: id.to_string(),
      title: title.to_string(),
      yaml: yaml.to_string(),
      updated_at: updated_at.to_string(),
    });
  }
  out
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn built_in_presets_all_parse_valid_dag_locally() {
    let presets = built_in_workflow_presets();
    assert_eq!(presets.len(), 21);
    for preset in &presets {
      let inspected = inspect_yaml_locally(&preset.yaml);
      assert!(
        inspected.valid,
        "preset '{}' failed local inspect: {:?}",
        preset.id, inspected.diagnostics.issues
      );
      assert!(
        !inspected.nodes.is_empty(),
        "preset '{}' has no nodes",
        preset.id
      );
    }
  }

  #[test]
  fn local_inspector_detects_run_if_edges_and_levels() {
    let inspected = inspect_yaml_locally(INSTANT_STEP_DEBUG_YAML);
    assert!(inspected.valid);
    assert_eq!(inspected.nodes.len(), 4);
    assert_eq!(inspected.levels.len(), 3);
    assert!(inspected.edges.iter().any(|e| e.from == "normalize_brief"
      && e.to == "quiz_branch"
      && e.kind == InspectedEdgeKind::Condition));
  }

  #[test]
  fn graph_mutator_adds_connects_detects_cycles_renames_and_deletes() {
    let added = add_node_to_yaml(
      INSTANT_STEP_DEBUG_YAML,
      "llm",
      Some("assemble_handout"),
      None,
    )
    .expect("add node");
    let new_id = added.created_node_id.expect("created id");
    assert_eq!(new_id, "llm_1");

    // Cycle should be rejected: connecting downstream back to normalize_brief
    let cyc = connect_nodes_in_yaml(&added.yaml, "llm_1", "normalize_brief");
    assert!(cyc.is_err());

    // Valid extra edge
    let connected = connect_nodes_in_yaml(&added.yaml, "core_outline", "llm_1").expect("connect");
    let insp = inspect_yaml_locally(&connected.yaml);
    assert!(
      insp
        .edges
        .iter()
        .any(|e| e.from == "core_outline" && e.to == "llm_1")
    );

    // Rename node and verify cascade rewrite
    let renamed = update_node_in_yaml(
      &connected.yaml,
      "normalize_brief",
      &NodeUpdatePatch {
        new_id: Some("prep_brief".to_string()),
        ..Default::default()
      },
    )
    .expect("rename");
    assert_eq!(renamed.renamed_to.as_deref(), Some("prep_brief"));
    assert!(renamed.yaml.contains("nodes.prep_brief.outputs.headline"));
    assert!(!renamed.yaml.contains("nodes.normalize_brief."));

    // Disconnect edge
    let disc = disconnect_edge_in_yaml(&renamed.yaml, "core_outline", "llm_1").expect("disconnect");
    let insp_disc = inspect_yaml_locally(&disc.yaml);
    assert!(
      !insp_disc
        .edges
        .iter()
        .any(|e| e.from == "core_outline" && e.to == "llm_1")
    );

    // Delete node cascades cleanup
    let deleted = delete_node_from_yaml(&disc.yaml, "quiz_branch").expect("delete");
    let insp_del = inspect_yaml_locally(&deleted.yaml);
    assert_eq!(insp_del.nodes.len(), 4);
    assert!(!insp_del.nodes.iter().any(|n| n.id == "quiz_branch"));
  }

  #[test]
  fn personal_templates_round_trip_and_filename_slug() {
    assert_eq!(
      build_yaml_download_filename(Some("My Flow!")),
      "my_flow.yml"
    );
    assert_eq!(build_yaml_download_filename(Some("demo.yaml")), "demo.yaml");

    let (list, patch) = build_personal_templates_preference_patch(
      &[],
      Some("custom_1"),
      "My Demo",
      INSTANT_STEP_DEBUG_YAML,
      "2026-11-26T12:00:00Z",
    )
    .expect("patch");
    assert_eq!(list.len(), 1);
    let restored = restore_personal_templates_from_preferences(Some(&patch));
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, "custom_1");
    assert_eq!(restored[0].title, "My Demo");
  }
}
