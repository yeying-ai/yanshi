# Rust Traits 深度讲解 — 多模态图文视频课程生成示例

本示例展示如何使用 **Yanshi** 的多模态生产级管线（`LLM` + `Text-to-Image` + `Image-to-Video (I2V)` + `VLM 审片` + `TTS 语音合成` + `FFmpeg 音视频混流`），围绕 **《Rust Traits 深度讲解》** 自动生产一整套交付物：

1. **6 章深度技术专题**：
   - **第 1 章 · 介绍**：Traits 的本质——共享行为契约与 Typeclass 思想（数据与行为彻底解耦）
   - **第 2 章 · 描述方式**：方法签名、默认实现、关联类型（Associated Types）vs 泛型参数、Supertraits
   - **第 3 章 · 应用方法** *(配 Veo I2V 视频)*：Trait Bounds、编译期单态化（Static Dispatch）vs 运行期胖指针 `&dyn Trait` 虚表（Dynamic Dispatch）、Blanket Implementations
   - **第 4 章 · 核心优点** *(配 Veo I2V 视频)*：深度对比 OOP（Java/C++ 继承树与菱形继承困境）与 C 语言（`void*` + 手写函数指针表的安全盲区）
   - **第 5 章 · 注意事项**：孤儿规则（Orphan Rule / Coherence）、对象安全（`dyn` 兼容性）、单态化二进制膨胀治理与“单实现提前抽 Trait”反模式
   - **第 6 章 · 总结与深思** *(配 Veo I2V 视频)*：把运行时的不确定性前移为编译期证明——从“它是什么”到“它能做什么”的工程哲学

2. **三种运行与复用方式**：
   - **配置驱动 YAML 工作流**：[`workflow.yml`](workflow.yml)
   - **Rust SDK 代码级管线**：[`yanshi-llm/examples/rust_traits_studio.rs`](../../../yanshi-llm/examples/rust_traits_studio.rs)（支持离线 Mock 冒烟与线上 Vertex AI 全链路生成自包含交互式 `index.html` + `tutorial.md` + 带字幕长视频 `full_course.mp4`）

---

## 快速使用

### 1. 严格校验 YAML 工作流（离线零依赖）

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

## 产物目录结构（`runs/rust-traits-course/`）

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
