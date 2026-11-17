# Yanshi Studio (`yanshi-ui` + `src-tauri`)

跨平台可视化工作流编排、**单步 I/O 调试（调试状态 Studio）** 与 **一键全速运行（发布状态 App Runner）** 前端及 Tauri v2 壳工程，一套代码同时覆盖 **Web、Desktop（macOS / Windows / Linux）与 Mobile（iOS / Android）**。

---

## 两种核心工作姿态

1. **🔍 调试状态（Studio · `/ui/studio`）**
   - 交互式 `@xyflow/react` DAG 拓扑画布（自动拓扑分层布局、数据映射实线边、`run_if` 条件虚线边、`map`/`while` 子流水线标注）。
   - **每步自动暂停观测**：勾选「每步暂停观测」后，每执行完一个顶层节点自动挂起（亦支持在任意节点右上角点击圆点设置执行前断点），右侧 **I/O 观测台** 实时展示表达式求值后的 **实际输入 Input**、**实际输出 Output** 以及内联图像/音频/视频/交互式 HTML 预览。
   - 控台支持 `▶ 单步下一步`、`⏭ 继续跑完`、`⏹ 终止`。

2. **🚀 发布状态（App Runner · `/ui/app`）**
   - 自动根据工作流 `inputs:` 声明生成类型化参数表单（隐藏底层 YAML 与断点细节）。
   - 点击 **「🚀 一键运行整个流程」** 全速并发调度无依赖阶段，实时追踪阶段进度并在右侧 **成果展台** 直接播放合成好的 1080p 视频、交互式幻灯片或下载 PPTX/PDF。

---

## 1. Web 端运行（内嵌于 `yanshi-server`）

```bash
# 1. 构建前端静态产物（同时生成 dist/assets/* 与 dist/ui/assets/*）
npm --prefix yanshi-ui run build

# 2. 启动 yanshi-server 网关后浏览器访问：
#    - 调试工作台：http://127.0.0.1:8080/ui/studio
#    - 一键运行端：http://127.0.0.1:8080/ui/app
#    - 经典事件台：http://127.0.0.1:8080/ui/console
```

---

## 2. Desktop 桌面端运行（macOS / Windows / Linux · 免 Postgres 零配置）

桌面端通过 `yanshi-ui/src-tauri/src/local_engine.rs` 在进程内自动拉起零依赖 Loopback 执行引擎（直连 `yanshi-config` + `yanshi-core`，自动加载 `~/.yanshi/.env`），无需安装 PostgreSQL 即可开箱进行可视化单步调试与多媒体出片；也可在顶部「网关连接」弹窗随时切换至远端集群网关。

```bash
# 安装 Tauri CLI（首次）
cargo install tauri-cli --version "^2.0.0"

# 开发调试桌面应用
cd yanshi-ui
cargo tauri dev

# 打包原生安装包（.dmg / .app / .msi / .AppImage）
cargo tauri build

# macOS 一键构建并安装到 /Applications/Yanshi Studio.app（已构建过可加 --skip-build）
../scripts/install_desktop.sh
```

仅编译校验或运行桌面端内置引擎单步调试测试：

```bash
cargo test --manifest-path yanshi-ui/src-tauri/Cargo.toml
```

---

## 3. Mobile 移动端运行（iOS / Android）

移动端自动采用自适应底部安全区标签导航（`配置` · `拓扑调试` · `I/O 观测` · `成果`），作为轻量客户端连接局域网或云端 `yanshi-server` 网关（首次打开点击右上角连接按钮填入网关地址与 Token 即可）。

```bash
cd yanshi-ui

# iOS（需 macOS + Xcode）
cargo tauri ios init
cargo tauri ios dev

# Android（需 Android Studio + NDK）
cargo tauri android init
cargo tauri android dev
```
