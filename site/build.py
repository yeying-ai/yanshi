#!/usr/bin/env python3
"""Assemble the GitHub Pages site into site/book/.

Layout produced under site/book/:
  index.html                               -> Chinese landing page (https://yanshi.dev/)
  en/index.html                            -> English landing page (https://yanshi.dev/en/)
  styles.css, app.js, assets/              -> Shared landing assets + 1200x630 og-card.png
  workflows/*.yml + workflows-data.js      -> Bundled Studio YAML workflows (lazy-loaded)
  CNAME, sitemap.xml, robots.txt, llms.txt -> Custom domain & search/AI discovery manifests
  docs/                                    -> mdBook manual (https://yanshi.dev/docs/)
"""

from datetime import datetime, timezone
from html import escape as html_escape
from html.parser import HTMLParser
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
from typing import Any

CUSTOM_DOMAIN = "yanshi.dev"
SITE_URL = f"https://{CUSTOM_DOMAIN}"
REPO_URL = "https://github.com/yeying-ai/yanshi"

SITE_DIR = Path(__file__).resolve().parent
REPO_ROOT = SITE_DIR.parent
LANDING_DIR = SITE_DIR / "landing"
GENERATORS_DIR = REPO_ROOT / "examples" / "generators"
OUT_DIR = SITE_DIR / "book"
EN_OUT_DIR = OUT_DIR / "en"
DOCS_OUT_DIR = OUT_DIR / "docs"
WORKFLOWS_OUT_DIR = OUT_DIR / "workflows"

NODE_ID_RE = re.compile(r"^  -\s+id:\s*['\"]?([a-zA-Z0-9_-]+)['\"]?\s*$")
NODE_TYPE_RE = re.compile(r"^    type:\s*['\"]?([a-zA-Z0-9_-]+)['\"]?\s*$")
SEO_SLOT_RE = re.compile(
  r"<!-- SEO_HEAD_START -->[\s\S]*?<!-- SEO_HEAD_END -->",
  re.MULTILINE,
)

VOID_TAGS = frozenset({
  "area",
  "base",
  "br",
  "col",
  "embed",
  "hr",
  "img",
  "input",
  "link",
  "meta",
  "param",
  "source",
  "track",
  "wbr",
})

RELATIVE_ASSET_PREFIXES = (
  "styles.css",
  "app.js",
  "workflows-data.js",
  "llms.txt",
  "sitemap.xml",
  "assets/",
  "docs/",
  "workflows/",
)

LOCALE_META: dict[str, dict[str, Any]] = {
  "zh": {
    "html_lang": "zh-CN",
    "url": f"{SITE_URL}/",
    "og_locale": "zh_CN",
    "og_locale_alt": "en_US",
    "title": "Yanshi 偃师 · Rust 确定性 DAG 工作流与智能体多模态内容生产引擎",
    "description": (
      "Yanshi（偃师）是基于 Rust 构建的开源确定性 DAG 工作流与原生智能体多模态内容生产引擎。"
      "输入主题、网页、GitHub 仓库、学术 PDF 或音视频，一键生成原生可编辑 OOXML PPTX、"
      "1080p 配音讲解视频、16:9 PDF、Excel 实验表、双人播客 MP3 与自校验 D2/Typst/TikZ 架构图。"
    ),
    "og_description": (
      "把原始素材转化为可交付成品：原生可编辑 OOXML .pptx、1080p 配音讲解视频、"
      "16:9 .pdf、.xlsx、双人播客 .mp3 与自校验 D2/Typst/TikZ 架构图。"
    ),
    "image_alt": "Yanshi 偃师 — Rust 确定性 DAG 与多模态智能体内容生产引擎概览",
    "faqs": [
      (
        "Yanshi 生成的 .pptx 幻灯片是整页 AI 图片贴片，还是 PowerPoint / Keynote 原生可编辑组件？",
        "是原生可编辑的 OOXML 演示文稿。Yanshi 内置纯 Rust 编写的 OOXML 写入器，直接输出标准 PowerPoint 形状（<p:sp>）、原生表格（<a:tbl>）、代码卡片与连线，在 PowerPoint、Keynote 或 WPS 中打开即可逐字编辑；同时按每页意图分流融合 D2/Typst/TikZ 矢量架构图与 6 种艺术风格概念插画，并同步导出 16:9 .pdf 与交互式 .html。",
      ),
      (
        "没有图像生成或语音 TTS API Key 可以运行么？支持哪些大模型服务商？",
        "完全可以。只需配置任一文本大模型 Key 即可开箱运行 slides.yml、report.yml、repo_architecture.yml、backend_project_planner.yml、paper_reader.yml、mindmap.yml 等绝大多数生成器。运行时原生支持 OpenAI、Anthropic Claude、Google Gemini、Moonshot（Kimi）、StepFun，以及任意兼容 OpenAI 接口的服务（如 DeepSeek、Qwen、本地 Ollama）。",
      ),
      (
        "Yanshi 与 LangGraph、Dify 或单一 AI PPT 工具有何核心区别？",
        "Yanshi 将「确定性并发 DAG 工作流」、「原生 ReAct / Plan-Execute / 多智能体循环」与「运行期动态编译工作流」统一在同一套零运行时依赖耦合的 Rust L0 窄腰内核上：既编译为单一高性能静态二进制（调度与预算护栏开销低于 1ms，内置 44 模型硬性 cost_limit_usd 熔断），又以一等公民磁盘文件句柄（FlowValue::File）打通 PPTX、1080p 配音视频、PDF、Excel 与播客渲染，免除 Python 环境冲突与 Base64 内存膨胀。",
      ),
      (
        "在抓取外部网页、分析陌生代码仓库或运行绘图代码时如何保障宿主机安全？",
        "SecurityProfile::Production 采用默认闭锁（fail-closed）策略：宿主机工具自动运行于 macOS sandbox-exec 或 Linux Landlock + seccomp + cgroup v2 操作系统沙箱中；code_exec 绘图代码强制运行在无网络隔离容器（ContainerBackend）内；repo_snapshot 仅静态解析清单与导入树而不执行仓库代码；HttpTool 逐请求绑定 DNS 解析并归一化 IPv6 地址以阻断 DNS 重绑定 SSRF；技能包与插件默认强制校验 Ed25519 数字签名。",
      ),
      (
        "多阶段长视频或深度报告生成中途失败，恢复执行会重复消耗大模型费用么？",
        "不会。开启 --checkpoint（或通过网关与桌面端运行）后，每个节点完成即刻落盘检查点及已累计账单金额与产物 SHA-256 清单。执行 yanshi workflow resume <run_id> 时仅从尚未完成的下游节点继续并发调度，已成功的大模型与生图/配音节点直接复用落盘结果，不重复计费。",
      ),
    ],
  },
  "en": {
    "html_lang": "en",
    "url": f"{SITE_URL}/en/",
    "og_locale": "en_US",
    "og_locale_alt": "zh_CN",
    "title": "Yanshi · Deterministic DAG Workflow & Multimodal AI Agent Runtime in Rust",
    "description": (
      "Yanshi (偃师) is an open-source Rust runtime unifying deterministic DAG workflows "
      "and autonomous AI agents — turning raw topics, GitHub repos, PDFs, and recordings "
      "into native editable OOXML .pptx, 1080p narrated .mp4, 16:9 .pdf, .xlsx workbooks, "
      ".mp3 podcasts, and verified D2/Typst/TikZ diagrams."
    ),
    "og_description": (
      "Sources in, finished deliverables out: native editable OOXML .pptx, narrated 1080p .mp4, "
      "16:9 .pdf, .xlsx, EBU R128 .mp3, and verified D2/Typst/TikZ diagrams on one Rust runtime."
    ),
    "image_alt": "Yanshi (偃师) — Deterministic DAG & Multimodal AI Agent Runtime in Rust",
    "faqs": [
      (
        "Are generated .pptx decks flat AI images or native editable PowerPoint shapes?",
        "They are real, native OOXML presentations. Yanshi's pure-Rust OOXML writer emits standard PowerPoint shapes (<p:sp>), native tables (<a:tbl>), code callouts, and connectors that open directly for text and layout editing in PowerPoint or Keynote — combined per-slide with verified D2/Typst/TikZ diagrams and 6 art-directed illustration styles, plus matching 16:9 .pdf and .html exports.",
      ),
      (
        "Can I run workflows without image or TTS API keys, and which LLM providers are supported?",
        "Yes. A single text LLM key runs most Studio generators out of the box — including slides.yml, report.yml, repo_architecture.yml, backend_project_planner.yml, paper_reader.yml, and mindmap.yml. Yanshi natively supports OpenAI, Anthropic Claude, Google Gemini, Moonshot, StepFun, and any OpenAI-compatible endpoint (DeepSeek, Qwen, or local Ollama).",
      ),
      (
        "How does Yanshi differ from LangGraph, Dify, or single-purpose AI slide tools?",
        "Yanshi unifies deterministic concurrent DAGs, native autonomous agent loops (ReAct, Plan-Execute, Handoff/Blackboard/Debate), and runtime-compiled dynamic workflows on one narrow-waist Rust kernel. You get a single fast binary (< 1ms scheduling overhead, built-in 44-model cost_limit_usd enforcement) and first-class FlowValue::File multimodal renderers (.pptx, 1080p .mp4, .pdf, .xlsx, .mp3) with zero Python dependency drift or Base64 memory bloat.",
      ),
      (
        "How does Yanshi isolate untrusted repos, web fetches, and LLM-generated code?",
        "Under SecurityProfile::Production, security is fail-closed by default: shell and renderer subprocesses run inside OS sandboxes (macOS sandbox-exec or Linux Landlock + seccomp + cgroup v2), code_exec requires a zero-network ContainerBackend, repo_snapshot inspects manifests statically without executing code, HttpTool pins DNS per request to block TOCTOU SSRF, and marketplace skills/plugins verify Ed25519 signatures.",
      ),
      (
        "If a long multi-stage video or report run is interrupted, do resumed runs re-bill completed steps?",
        "No. With --checkpoint (and automatically on the gateway/Studio), Yanshi persists node outputs, SHA-256 file manifests, and accrued spend after every node. Running yanshi workflow resume <run_id> replays completed upstream outputs from disk and executes only unfinished downstream nodes under the remaining cost budget.",
      ),
    ],
  },
}

REQUIRED_OUTPUTS = (
  OUT_DIR / "index.html",
  EN_OUT_DIR / "index.html",
  OUT_DIR / "styles.css",
  OUT_DIR / "app.js",
  OUT_DIR / "workflows-data.js",
  OUT_DIR / "CNAME",
  OUT_DIR / "sitemap.xml",
  OUT_DIR / "robots.txt",
  OUT_DIR / "llms.txt",
  OUT_DIR / "assets" / "favicon.svg",
  OUT_DIR / "assets" / "og-card.png",
  DOCS_OUT_DIR / "index.html",
  DOCS_OUT_DIR / "STUDIO_CATALOGUE.html",
)


def rewrite_relative_url(val: str, asset_prefix: str) -> str:
  if not asset_prefix:
    return val
  if any(val.startswith(prefix) for prefix in RELATIVE_ASSET_PREFIXES):
    return f"{asset_prefix}{val}"
  return val


class LocaleHtmlRenderer(HTMLParser):
  """Strip inactive-language spans and localize attributes cleanly."""

  def __init__(self, lang: str, asset_prefix: str) -> None:
    super().__init__(convert_charrefs=False)
    self.lang = lang
    self.drop_attr = "data-i18n-en" if lang == "zh" else "data-i18n-zh"
    self.asset_prefix = asset_prefix
    self.skip_depth = 0
    self.chunks: list[str] = []

  def _transform_attrs(
    self,
    tag: str,
    attrs: list[tuple[str, str | None]],
  ) -> list[tuple[str, str | None]]:
    attr_map = dict(attrs)

    if tag == "html":
      meta = LOCALE_META[self.lang]
      return [
        ("lang", meta["html_lang"]),
        ("data-lang", self.lang),
        ("data-asset-prefix", self.asset_prefix),
      ]

    if tag == "a" and attr_map.get("id") == "lang-toggle":
      if self.lang == "zh":
        target_href, target_hreflang, label = "en/", "en", "Switch to English"
      else:
        target_href, target_hreflang, label = "../", "zh-CN", "切换至中文"
      return [
        ("id", "lang-toggle"),
        ("class", attr_map.get("class") or "lang-btn"),
        ("href", target_href),
        ("hreflang", target_hreflang),
        ("aria-label", label),
      ]

    alt_override = attr_map.get(f"data-alt-{self.lang}")
    rendered: list[tuple[str, str | None]] = []
    has_alt = False

    for key, val in attrs:
      if key in ("data-alt-zh", "data-alt-en"):
        continue
      if key == "alt" and alt_override is not None:
        rendered.append(("alt", alt_override))
        has_alt = True
        continue
      if key in ("href", "src") and val is not None:
        val = rewrite_relative_url(val, self.asset_prefix)
      rendered.append((key, val))

    if alt_override is not None and not has_alt:
      rendered.append(("alt", alt_override))

    return rendered

  @staticmethod
  def _format_attrs(attrs: list[tuple[str, str | None]]) -> str:
    parts: list[str] = []
    for key, val in attrs:
      if val is None:
        parts.append(f" {key}")
      else:
        escaped = html_escape(val, quote=True)
        parts.append(f' {key}="{escaped}"')
    return "".join(parts)

  def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
    if self.skip_depth > 0:
      if tag not in VOID_TAGS:
        self.skip_depth += 1
      return
    if any(k == self.drop_attr for k, _ in attrs):
      if tag not in VOID_TAGS:
        self.skip_depth = 1
      return
    clean_attrs = self._transform_attrs(tag, attrs)
    self.chunks.append(f"<{tag}{self._format_attrs(clean_attrs)}>")

  def handle_startendtag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
    if self.skip_depth > 0 or any(k == self.drop_attr for k, _ in attrs):
      return
    clean_attrs = self._transform_attrs(tag, attrs)
    self.chunks.append(f"<{tag}{self._format_attrs(clean_attrs)} />")

  def handle_endtag(self, tag: str) -> None:
    if self.skip_depth > 0:
      if tag not in VOID_TAGS:
        self.skip_depth -= 1
      return
    self.chunks.append(f"</{tag}>")

  def handle_data(self, data: str) -> None:
    if self.skip_depth == 0:
      self.chunks.append(data)

  def handle_entityref(self, name: str) -> None:
    if self.skip_depth == 0:
      self.chunks.append(f"&{name};")

  def handle_charref(self, name: str) -> None:
    if self.skip_depth == 0:
      self.chunks.append(f"&#{name};")

  def handle_comment(self, data: str) -> None:
    if self.skip_depth == 0:
      self.chunks.append(f"<!--{data}-->")

  def handle_decl(self, decl: str) -> None:
    self.chunks.append(f"<!{decl}>")


def build_json_ld(lang: str) -> str:
  meta = LOCALE_META[lang]
  faq_entities = [
    {
      "@type": "Question",
      "name": question,
      "acceptedAnswer": {
        "@type": "Answer",
        "text": answer,
      },
    }
    for question, answer in meta["faqs"]
  ]
  graph = {
    "@context": "https://schema.org",
    "@graph": [
      {
        "@type": "WebSite",
        "@id": f"{SITE_URL}/#website",
        "name": "Yanshi 偃师",
        "alternateName": ["Yanshi", "偃师", "Yanshi Studio"],
        "url": meta["url"],
        "inLanguage": meta["html_lang"],
        "publisher": {
          "@type": "Organization",
          "name": "Yeying AI",
          "url": "https://github.com/yeying-ai",
        },
      },
      {
        "@type": ["SoftwareApplication", "SoftwareSourceCode"],
        "@id": f"{SITE_URL}/#software",
        "name": "Yanshi",
        "alternateName": "Yanshi 偃师",
        "description": meta["description"],
        "url": meta["url"],
        "image": f"{SITE_URL}/assets/og-card.png",
        "screenshot": f"{SITE_URL}/assets/slide-route-illustration.webp",
        "applicationCategory": "DeveloperApplication",
        "operatingSystem": "macOS, Linux, Windows",
        "programmingLanguage": "Rust",
        "runtimePlatform": "Rust / Tokio / Tauri v2",
        "codeRepository": REPO_URL,
        "softwareHelp": f"{SITE_URL}/docs/",
        "license": "https://opensource.org/licenses/MIT",
        "inLanguage": ["zh-CN", "en"],
        "offers": {
          "@type": "Offer",
          "price": "0",
          "priceCurrency": "USD",
        },
        "featureList": [
          "Deterministic concurrent DAG workflows in YAML and Rust",
          "Autonomous AI agent loops (ReAct, Plan-Execute, Reflection, Multi-Agent Supervisors)",
          "Pure-Rust native editable OOXML .pptx and typed .xlsx writers",
          "1080p narrated slide video (.mp4) and EBU R128 podcast (.mp3) synthesis",
          "Self-repairing D2, Typst, TikZ, and sandboxed Python vector diagrams",
          "Fail-closed OS sandboxing (macOS sandbox-exec, Linux Landlock + seccomp + cgroup v2)",
          "First-class MCP client, Qdrant/ONNX RAG, and Ed25519-verified SKILL.md packages",
        ],
      },
      {
        "@type": "FAQPage",
        "@id": f"{meta['url']}#faqpage",
        "inLanguage": meta["html_lang"],
        "mainEntity": faq_entities,
      },
    ],
  }
  return json.dumps(graph, ensure_ascii=False, indent=2)


def build_seo_head_block(lang: str) -> str:
  meta = LOCALE_META[lang]
  title = html_escape(meta["title"], quote=True)
  desc = html_escape(meta["description"], quote=True)
  og_desc = html_escape(meta["og_description"], quote=True)
  img_alt = html_escape(meta["image_alt"], quote=True)
  og_img = f"{SITE_URL}/assets/og-card.png"
  json_ld = build_json_ld(lang)

  lines = [
    f"<title>{title}</title>",
    f'<meta name="description" content="{desc}" />',
    '<meta name="robots" content="index, follow, max-image-preview:large, max-snippet:-1, max-video-preview:-1" />',
    f'<link rel="canonical" href="{meta["url"]}" />',
    f'<link rel="alternate" hreflang="zh-CN" href="{SITE_URL}/" />',
    f'<link rel="alternate" hreflang="zh" href="{SITE_URL}/" />',
    f'<link rel="alternate" hreflang="en" href="{SITE_URL}/en/" />',
    f'<link rel="alternate" hreflang="x-default" href="{SITE_URL}/en/" />',
    f'<link rel="sitemap" type="application/xml" title="Sitemap" href="{SITE_URL}/sitemap.xml" />',
    '<meta property="og:type" content="website" />',
    '<meta property="og:site_name" content="Yanshi 偃师" />',
    f'<meta property="og:locale" content="{meta["og_locale"]}" />',
    f'<meta property="og:locale:alternate" content="{meta["og_locale_alt"]}" />',
    f'<meta property="og:title" content="{title}" />',
    f'<meta property="og:description" content="{og_desc}" />',
    f'<meta property="og:url" content="{meta["url"]}" />',
    f'<meta property="og:image" content="{og_img}" />',
    '<meta property="og:image:type" content="image/png" />',
    '<meta property="og:image:width" content="1200" />',
    '<meta property="og:image:height" content="630" />',
    f'<meta property="og:image:alt" content="{img_alt}" />',
    '<meta name="twitter:card" content="summary_large_image" />',
    f'<meta name="twitter:title" content="{title}" />',
    f'<meta name="twitter:description" content="{og_desc}" />',
    f'<meta name="twitter:image" content="{og_img}" />',
    f'<meta name="twitter:image:alt" content="{img_alt}" />',
    f'<script type="application/ld+json">\n{json_ld}\n    </script>',
  ]
  return "\n    ".join(lines)


def render_localized_landing(template_html: str, lang: str, asset_prefix: str) -> str:
  seo_head = build_seo_head_block(lang)
  with_seo = SEO_SLOT_RE.sub(seo_head, template_html, count=1)
  parser = LocaleHtmlRenderer(lang=lang, asset_prefix=asset_prefix)
  parser.feed(with_seo)
  parser.close()
  return "".join(parser.chunks)


def extract_cli_example(lines: list[str], filename: str) -> str:
  collected: list[str] = []
  in_cmd = False
  for line in lines[:45]:
    if not line.startswith("#"):
      if in_cmd:
        break
      continue
    stripped = line.lstrip("#").strip()
    if not in_cmd and stripped.startswith("yanshi workflow run"):
      in_cmd = True
      collected.append(stripped.rstrip("\\").strip())
      if not stripped.endswith("\\"):
        break
    elif in_cmd:
      collected.append(stripped.rstrip("\\").strip())
      if not stripped.endswith("\\"):
        break
  if collected:
    return " ".join(collected)
  return f"yanshi workflow run examples/generators/{filename} --output-dir out"


def extract_top_nodes(lines: list[str]) -> list[dict[str, str]]:
  nodes: list[dict[str, str]] = []
  in_nodes = False
  current_id: str | None = None
  for line in lines:
    if line.startswith("nodes:"):
      in_nodes = True
      continue
    if in_nodes and line and not line.startswith((" ", "#")):
      break
    if not in_nodes:
      continue
    id_match = NODE_ID_RE.match(line)
    if id_match:
      current_id = id_match.group(1)
      nodes.append({"id": current_id, "type": "node"})
      continue
    type_match = NODE_TYPE_RE.match(line)
    if type_match and nodes and current_id is not None:
      nodes[-1]["type"] = type_match.group(1)
      current_id = None
  return nodes


def bundle_workflows() -> list[str]:
  if not GENERATORS_DIR.is_dir():
    print(f"site/build.py: missing generators directory: {GENERATORS_DIR}", file=sys.stderr)
    sys.exit(1)

  WORKFLOWS_OUT_DIR.mkdir(parents=True, exist_ok=True)
  bundle: dict[str, dict[str, Any]] = {}
  filenames: list[str] = []

  for yml_path in sorted(GENERATORS_DIR.glob("*.yml")):
    content = yml_path.read_text(encoding="utf-8")
    lines = content.splitlines()
    shutil.copy2(yml_path, WORKFLOWS_OUT_DIR / yml_path.name)
    filenames.append(yml_path.name)
    bundle[yml_path.name] = {
      "file": yml_path.name,
      "path": f"examples/generators/{yml_path.name}",
      "lines": len(lines),
      "cli": extract_cli_example(lines, yml_path.name),
      "nodes": extract_top_nodes(lines),
      "yaml": content,
    }

  js_payload = (
    "// Auto-generated by site/build.py from examples/generators/*.yml\n"
    f"window.YANSHI_WORKFLOWS = {json.dumps(bundle, ensure_ascii=False)};\n"
  )
  (OUT_DIR / "workflows-data.js").write_text(js_payload, encoding="utf-8")
  return filenames


def mark_mdbook_utility_pages_noindex() -> None:
  """Prevent mdBook print.html / 404.html / toc.html from duplicating chapter indexation."""
  noindex_tag = '<meta name="robots" content="noindex, follow">'
  for name in ("print.html", "404.html", "toc.html"):
    page = DOCS_OUT_DIR / name
    if not page.is_file():
      continue
    html = page.read_text(encoding="utf-8")
    if 'name="robots"' in html:
      continue
    updated = html.replace("<head>", f"<head>\n        {noindex_tag}", 1)
    page.write_text(updated, encoding="utf-8")


def generate_sitemap(workflow_files: list[str]) -> int:
  today = datetime.now(timezone.utc).strftime("%Y-%m-%d")
  hreflang_links = (
    f'    <xhtml:link rel="alternate" hreflang="zh-CN" href="{SITE_URL}/" />\n'
    f'    <xhtml:link rel="alternate" hreflang="zh" href="{SITE_URL}/" />\n'
    f'    <xhtml:link rel="alternate" hreflang="en" href="{SITE_URL}/en/" />\n'
    f'    <xhtml:link rel="alternate" hreflang="x-default" href="{SITE_URL}/en/" />'
  )

  entries: list[str] = [
    (
      "  <url>\n"
      f"    <loc>{SITE_URL}/</loc>\n"
      f"    <lastmod>{today}</lastmod>\n"
      "    <priority>1.0</priority>\n"
      f"{hreflang_links}\n"
      "  </url>"
    ),
    (
      "  <url>\n"
      f"    <loc>{SITE_URL}/en/</loc>\n"
      f"    <lastmod>{today}</lastmod>\n"
      "    <priority>1.0</priority>\n"
      f"{hreflang_links}\n"
      "  </url>"
    ),
    (
      "  <url>\n"
      f"    <loc>{SITE_URL}/docs/</loc>\n"
      f"    <lastmod>{today}</lastmod>\n"
      "    <priority>0.9</priority>\n"
      "  </url>"
    ),
  ]

  excluded_docs = {"index.html", "INTRODUCTION.html", "print.html", "404.html", "toc.html"}
  for doc_html in sorted(DOCS_OUT_DIR.rglob("*.html")):
    rel = doc_html.relative_to(DOCS_OUT_DIR).as_posix()
    if rel in excluded_docs:
      continue
    loc = f"{SITE_URL}/docs/{rel}"
    priority = "0.85" if rel in ("STUDIO_CATALOGUE.html", "TRIAL_GUIDE.html", "ARCHITECTURE.html") else "0.7"
    entries.append(
      "  <url>\n"
      f"    <loc>{html_escape(loc)}</loc>\n"
      f"    <lastmod>{today}</lastmod>\n"
      f"    <priority>{priority}</priority>\n"
      "  </url>"
    )

  for wf_file in workflow_files:
    entries.append(
      "  <url>\n"
      f"    <loc>{SITE_URL}/workflows/{html_escape(wf_file)}</loc>\n"
      f"    <lastmod>{today}</lastmod>\n"
      "    <priority>0.65</priority>\n"
      "  </url>"
    )

  xml = (
    '<?xml version="1.0" encoding="UTF-8"?>\n'
    '<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"\n'
    '        xmlns:xhtml="http://www.w3.org/1999/xhtml">\n'
    + "\n".join(entries)
    + "\n</urlset>\n"
  )
  (OUT_DIR / "sitemap.xml").write_text(xml, encoding="utf-8")
  return len(entries)


def generate_robots_txt() -> None:
  robots = (
    "User-agent: *\n"
    "Allow: /\n"
    "Disallow: /docs/print.html\n"
    "Disallow: /docs/toc.html\n"
    "\n"
    f"Sitemap: {SITE_URL}/sitemap.xml\n"
  )
  (OUT_DIR / "robots.txt").write_text(robots, encoding="utf-8")


def generate_llms_txt(workflow_files: list[str]) -> None:
  wf_bullets = "\n".join(
    f"- [{wf}]({SITE_URL}/workflows/{wf}): `examples/generators/{wf}`"
    for wf in workflow_files
  )
  llms_txt = f"""# Yanshi (偃师)

> Yanshi is an open-source multimodal content-production runtime in Rust unifying deterministic concurrent DAG workflows, autonomous AI agent loops (ReAct, Plan-Execute, Reflection, multi-agent supervisors), runtime-compiled dynamic workflows, and fail-closed OS sandboxing.

Named after the artificer Yanshi (偃师) in *Liezi · Tangwen* (《列子·汤问》), every pipeline is assembled from explicit, inspectable, checkpoint-replayable components — turning raw topics, web URLs, GitHub repositories, academic PDFs, or audio/video recordings into native editable OOXML `.pptx`, narrated `1080p .mp4`, `16:9 .pdf`, `.xlsx` workbooks, EBU R128 `.mp3` podcasts, and verified D2/Typst/TikZ vector diagrams.

## Entry Points

- [Product Landing Page (中文)]({SITE_URL}/)
- [Product Landing Page (English)]({SITE_URL}/en/)
- [Online Manual (mdBook)]({SITE_URL}/docs/)
- [GitHub Repository]({REPO_URL})

## Core Documentation

- [5-Minute Trial Guide]({SITE_URL}/docs/TRIAL_GUIDE.html)
- [Content Studio & 19 Generator Catalogue]({SITE_URL}/docs/STUDIO_CATALOGUE.html)
- [Five-Layer Architecture & Four Execution Paradigms]({SITE_URL}/docs/ARCHITECTURE.html)
- [Workflow YAML Schema Reference]({SITE_URL}/docs/WORKFLOW_SCHEMA.html)
- [Agent Runtime & HITL (`ask_user`)]({SITE_URL}/docs/AGENT_RUNTIME.html)
- [Multimodal Guide (Slides, Video, Audio, Diagrams)]({SITE_URL}/docs/MULTIMODAL_GUIDE.html)
- [Web Studio & Tauri Desktop App (`Yanshi Studio`)]({SITE_URL}/docs/WEB_UI.html)
- [Security Profiles & OS Sandboxing]({SITE_URL}/docs/SECURITY_PROFILES.html)

## 19 Built-in Content Studio YAML Workflows

{wf_bullets}
"""
  (OUT_DIR / "llms.txt").write_text(llms_txt, encoding="utf-8")


def build_site() -> None:
  if not LANDING_DIR.is_dir():
    print(f"site/build.py: missing landing directory: {LANDING_DIR}", file=sys.stderr)
    sys.exit(1)

  if OUT_DIR.exists():
    shutil.rmtree(OUT_DIR)
  OUT_DIR.mkdir(parents=True, exist_ok=True)
  EN_OUT_DIR.mkdir(parents=True, exist_ok=True)

  shutil.copytree(LANDING_DIR, OUT_DIR, dirs_exist_ok=True)

  template_html = (LANDING_DIR / "index.html").read_text(encoding="utf-8")
  (OUT_DIR / "index.html").write_text(
    render_localized_landing(template_html, lang="zh", asset_prefix=""),
    encoding="utf-8",
  )
  (EN_OUT_DIR / "index.html").write_text(
    render_localized_landing(template_html, lang="en", asset_prefix="../"),
    encoding="utf-8",
  )

  workflow_files = bundle_workflows()
  subprocess.run(["mdbook", "build", str(SITE_DIR)], check=True)
  mark_mdbook_utility_pages_noindex()

  sitemap_count = generate_sitemap(workflow_files)
  generate_robots_txt()
  generate_llms_txt(workflow_files)
  (OUT_DIR / "CNAME").write_text(f"{CUSTOM_DOMAIN}\n", encoding="utf-8")
  (OUT_DIR / ".nojekyll").write_text("", encoding="utf-8")

  missing = [str(path.relative_to(SITE_DIR)) for path in REQUIRED_OUTPUTS if not path.is_file()]
  if missing:
    print("site/build.py: missing expected build artifacts:", file=sys.stderr)
    for item in missing:
      print(f"  {item}", file=sys.stderr)
    sys.exit(1)

  print(
    f"Built landing pages -> {OUT_DIR / 'index.html'} (zh-CN) & "
    f"{EN_OUT_DIR / 'index.html'} (en) ({len(workflow_files)} workflows bundled)"
  )
  print(f"Built SEO manifests -> sitemap.xml ({sitemap_count} URLs), robots.txt, llms.txt")
  print(f"Built mdBook manual -> {DOCS_OUT_DIR / 'index.html'}")


if __name__ == "__main__":
  build_site()
