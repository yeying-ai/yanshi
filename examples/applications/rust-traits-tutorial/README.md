# Rust Traits 深度讲解 — 多模态图文视频课程生成示例

本示例展示如何使用 **Yanshi** 的多模态生产级管线（`LLM` + `Text-to-Image` + `Image-to-Video (I2V)` + `VLM 审片` + `TTS 语音合成` + `FFmpeg 音视频混流`），围绕 **《Rust Traits 深度讲解》** 自动生产一整套交付物：

1. **6 章深度技术专题**：
   - **第 1 章 · 介绍**：Traits 的本质——共享行为契约与 Typeclass 思想（数据与行为彻底解耦）
   - **第 2 章 · 描述方式**：方法签名、默认实现、关联类型（Associated Types）vs 泛型参数、Supertraits
   - **第 3 章 · 应用方法** *(配 Veo I2V 视频)*：Trait Bounds、编译期单态化（Static Dispatch）vs 运行期胖指针 `&dyn Trait` 虚表（Dynamic Dispatch）、Blanket Implementations
   - **第 4 章 · 核心优点** *(配 Veo I2V 视频)*：深度对比 OOP（Java/C++ 继承树与菱形继承困境）与 C 语言（`void*` + 手写函数指针表的安全盲区）
   - **第 5 章 · 注意事项**：孤儿规则（Orphan Rule / Coherence）、对象安全（`dyn` 兼容性）、单态化二进制膨胀治理与“单实现提前抽 Trait”反模式
   - **第 6 章 · 总结与深思** *(配 Veo I2V 视频)*：把运行时的不确定性前移为编译期证明——从“它是什么”到“它能做什么”的工程哲学

2. **四种运行与复用方式**：
   - **1080p 技术 PPT 演播级课程工坊（推荐技术深度讲解）**：[`yanshi-config/examples/rust_traits_ppt_studio.rs`](../../../yanshi-config/examples/rust_traits_ppt_studio.rs)（基于纯原生 `render_slides` 矢量渲染引擎，自动生成 10 页 × 22 步渐进式动画技术幻灯片——含语法高亮代码焦点框、拓扑流程图、Stack/Heap/VTable 胖指针内存布局图、对比矩阵与贝塞尔思维导图——输出 1080p MP4 视频、双模交互式 Web Keynote 播放器 `index.html`、可编辑 PowerPoint `rust_traits_slides.pptx` 与高清矢量讲义 `rust_traits_handout.pdf`）
   - **通用可复用技术幻灯片视频生成器模板**：[`examples/generators/tech_slides_video.yml`](../../generators/tech_slides_video.yml)（给定任意技术主题 `topic`，自动执行 `LLM JSON Schema 分镜 -> render_slides 逐帧渲染 -> TTS 配音 -> render_video 多帧渐进混流 + PPTX/PDF 导出`）
   - **配置驱动多模态 YAML 工作流**：[`workflow.yml`](workflow.yml)
   - **Rust SDK 概念艺术 + Veo I2V 管线**：[`yanshi-llm/examples/rust_traits_studio.rs`](../../../yanshi-llm/examples/rust_traits_studio.rs)（支持离线 Mock 冒烟与线上 Vertex AI 全链路生成自包含交互式 `index.html` + `tutorial.md` + 带字幕长视频 `full_course.mp4`）

---

## 快速使用

### 1. 生成 1080p 技术 PPT 演播视频 + PPTX + PDF（推荐）

```bash
# 默认自动复用 runs/rust-traits-course/assets/ch01..06.wav 真实中文语音（若无缓存且未开启 LIVE 则走离线 Mock 语音）
cargo run -p yanshi-config --example rust_traits_ppt_studio

# 校验通用技术 PPT 视频生成模板（零外部依赖）
cargo run -p yanshi-cli -- workflow validate \
  examples/generators/tech_slides_video.yml --strict
```

### 2. 严格校验多模态 YAML 工作流（离线零依赖）

```bash
cargo run -p yanshi-cli -- workflow validate \
  examples/applications/rust-traits-tutorial/workflow.yml --strict
```

### 2. 运行 Rust SDK 示例（默认离线 Mock，CI 友好）

```bash
cargo run -p yanshi-llm --example rust_traits_studio
```

### 3. 连接 Google Vertex AI / Agent Platform 全量实测渲染

需要环境变量 `GEMINI_API_KEY`（支持 `AQ.` Vertex AI 密钥或 Gemini API Key）以及本机已安装 `ffmpeg`：

```bash
# 方式 A：运行 Rust SDK 课程工坊（生成响应式交互网页 index.html + tutorial.md + full_course.mp4 + 字幕）
YANSHI_LIVE_PROVIDER=1 YANSHI_OUTPUT_DIR=runs/rust-traits-course \
  cargo run -p yanshi-llm --example rust_traits_studio

# 方式 B：直接通过 Yanshi CLI 执行声明式 DAG 工作流
yanshi workflow run examples/applications/rust-traits-tutorial/workflow.yml \
  --output-dir runs/rust-traits-workflow
```

---

## 产物目录结构

### A. 1080p 技术 PPT 演播套件（`runs/rust-traits-ppt/`）

```text
runs/rust-traits-ppt/
├── full_course.mp4             # 1920×1080 渐进动画幻灯片讲课视频（H.264 + AAC 中文配音 + SRT 软字幕轨）
├── rust_traits_slides.pptx     # 可编辑 PowerPoint 课件（含每页高清矢量帧与讲师逐字稿备注）
├── rust_traits_handout.pdf     # 16:9 宽屏高清讲义 PDF（Typst 矢量排版：幻灯片 + 批注页）
├── index.html                  # 双模交互式 Web Keynote 播放器（支持视频时间轴同步高亮 & 键盘方向键翻页）
├── full_course.srt             # 22 步逐句对齐时间轴字幕 (SRT)
├── full_course.vtt             # Web 播放器时间轴字幕 (WebVTT)
├── tutorial.md                 # 结构化图文课程讲义 (Markdown)
└── slides/
    └── s01_step01.png .. s10_step01.png  # 22 张 1920×1080 原生矢量渲染渐进帧（代码高亮/流程图/胖指针内存图/对比表/思维导图）
```

### B. 概念插图 + Veo I2V 多模态套件（`runs/rust-traits-course/`）

```text
runs/rust-traits-course/
├── index.html                  # 自包含暗色工程风交互式课程专题页（内嵌完整长视频、分章视频、语音导读与代码对比）
├── tutorial.md                 # 完整图文视频 Markdown 教程
├── full_course.mp4             # FFmpeg 合成的带中文配音与软字幕轨完整讲解长视频
├── full_course.srt             # 全片时间轴字幕文件 (SRT)
├── full_course.vtt             # 网页播放器时间轴字幕文件 (WebVTT)
└── assets/
    ├── ch01.png .. ch06.png    # 6 张 16:9 架构概念首帧插图 (gemini-3.1-flash-image)
    ├── ch03_dispatch.mp4       # 第 3 章首帧锚定 I2V 动态视频：单态化 vs dyn vtable (veo-3.1-fast-generate-001)
    ├── ch04_comparison.mp4     # 第 4 章首帧锚定 I2V 动态视频：OOP vs C void* vs Rust Traits
    ├── ch06_philosophy.mp4     # 第 6 章首帧锚定 I2V 动态视频：编译期契约与零成本抽象
    └── ch01.wav .. ch06.wav    # 6 段 24kHz RIFF/WAV 中文语音旁白 (gemini-2.5-flash-preview-tts)
```
