#!/usr/bin/env python3
"""Multi-vendor end-to-end benchmark runner for Yanshi's 5 daily workflows.

Supports two execution backends:
1. HTTP Gateway (`--server https://yanshi.yeying.dev`, token via `YANSHI_API_TOKEN`),
   including live multi-turn HITL `ask_user` polling (`GET/POST /v1/runs/{id}/questions`).
2. Local CLI (`--cli-bin /Users/hal/.target/debug/yanshi`), useful when benchmarking
   vendors reachable through a local developer proxy (e.g., Google Gemini).
"""

import argparse
import base64
from dataclasses import asdict, dataclass, field
import json
import os
import pathlib
import subprocess
import sys
import time
from typing import Any, Optional
import urllib.error
import urllib.request


REPO_ROOT = pathlib.Path(__file__).resolve().parent.parent
GENERATORS_DIR = REPO_ROOT / "examples" / "generators"

WORKFLOW_FILES: dict[str, str] = {
  "illustrated_slides": "illustrated_slides.yml",
  "repo_architecture": "repo_architecture.yml",
  "backend_project_planner": "backend_project_planner.yml",
  "paper_reader": "paper_reader.yml",
  "video_summarizer": "video_summarizer.yml",
}

HITL_CANNED_ANSWERS: list[str] = [
  (
    "1. 峰值并发目标：秒杀瞬时下单峰值 15,000 QPS，日常稳态 1,200 QPS，读写比约 8:1；"
    "2. 一致性策略：Redis Lua 预扣减防超卖 + RocketMQ/Kafka 异步落库 + PostgreSQL 乐观锁最终一致，超时 15 分钟自动回滚库存；"
    "3. 多租户隔离：共享集群 + 行级 `tenant_id` 强制隔离（RLS 双重校验），大客户支持独立 Redis Key 前缀与独立限流桶。"
  ),
  (
    "补充确认：需要对接微信/支付宝异步回调幂等对账表（T+1 日终批次核对），"
    "审计日志保留 180 天，核心 API 采用 JWT + 商户 HMAC 签名鉴权，请据此收敛并输出完整架构规格纪要。"
  ),
]


@dataclass
class VendorSpec:
  label: str
  chat_model: str
  image_model: str = ""
  asr_model: str = "glm-asr"


@dataclass
class ArtifactRecord:
  name: str
  node_id: str
  mime_type: str
  size_bytes: int


@dataclass
class BenchResult:
  vendor: str
  chat_model: str
  workflow: str
  mode: str
  status: str
  duration_sec: float
  cost_usd: float
  artifact_count: int
  artifacts: list[ArtifactRecord] = field(default_factory=list)
  hitl_rounds: int = 0
  hitl_questions: list[str] = field(default_factory=list)
  run_id: str = ""
  error: str = ""


def parse_vendor_spec(raw: str) -> VendorSpec:
  parts = raw.split(":")
  if len(parts) < 2:
    raise ValueError(
      f"Invalid vendor spec '{raw}', expected label:chat_model[:image_model[:asr_model]]"
    )
  label = parts[0].strip()
  chat_model = parts[1].strip()
  image_model = parts[2].strip() if len(parts) >= 3 else ""
  asr_model = parts[3].strip() if len(parts) >= 4 and parts[3].strip() else "glm-asr"
  return VendorSpec(
    label=label,
    chat_model=chat_model,
    image_model=image_model,
    asr_model=asr_model,
  )


def build_opener(no_proxy: bool) -> urllib.request.OpenerDirector:
  if no_proxy:
    return urllib.request.build_opener(urllib.request.ProxyHandler({}))
  return urllib.request.build_opener()


def http_json(
  opener: urllib.request.OpenerDirector,
  method: str,
  url: str,
  token: str,
  payload: Optional[dict[str, Any]] = None,
  timeout: float = 30.0,
) -> dict[str, Any]:
  data = None
  headers = {
    "Accept": "application/json",
    "User-Agent": "yanshi-workflow-bench/1.0",
  }
  if token:
    headers["Authorization"] = f"Bearer {token}"
  if payload is not None:
    data = json.dumps(payload).encode("utf-8")
    headers["Content-Type"] = "application/json"
  req = urllib.request.Request(url, data=data, headers=headers, method=method)
  with opener.open(req, timeout=timeout) as resp:
    raw = resp.read().decode("utf-8")
    return json.loads(raw) if raw.strip() else {}


def build_workflow_inputs(
  workflow_name: str,
  vendor: VendorSpec,
  interactive_hitl: bool,
  video_data_uri: str,
) -> dict[str, Any]:
  if workflow_name == "illustrated_slides":
    has_img = bool(vendor.image_model)
    inputs: dict[str, Any] = {
      "subject": "Cloudflare Pingora Rust 高性能反向代理与零停机热升级架构",
      "audience": "后端基础设施与云原生网关工程师",
      "narration_style": "深入浅出、数据驱动、对比 Nginx 多进程与 Tokio 多线程工作窃取模型",
      "slide_count": 5,
      "slide_theme": "dark",
      "enable_illustrations": has_img,
      "export_pptx": True,
      "export_pdf": True,
    }
    if has_img:
      inputs["image_model"] = vendor.image_model
    return inputs

  if workflow_name == "repo_architecture":
    return {
      "source": "https://github.com/tokio-rs/axum",
      "subpath": "axum",
      "focus": "Router/Handler/Extractor 核心抽象分层、Tower Service 中间件组合链路与模块依赖拓扑",
      "export_pdf": True,
    }

  if workflow_name == "backend_project_planner":
    return {
      "idea": (
        "设计一个面向多商户的电商高并发秒杀与订单履约后端系统"
        + ("" if not interactive_hitl else "（峰值规模、扣减一致性与隔离策略待与架构师确认）")
      ),
      "tech_preference": "Rust (Axum + Tokio + SQLx) + PostgreSQL + Redis + 消息队列",
      "interactive": interactive_hitl,
      "max_questions": 2,
      "export_pdf": True,
    }

  if workflow_name == "paper_reader":
    return {
      "source": "https://export.arxiv.org/pdf/1706.03762.pdf",
      "target_lang": "简体中文",
      "max_pages": 15,
      "focus": "Scaled Dot-Product Attention 与 Multi-Head Attention 机制、复杂度对比及 WMT 2014 实验指标提取",
      "export_pdf": True,
    }

  if workflow_name == "video_summarizer":
    return {
      "source": video_data_uri,
      "asr_model": vendor.asr_model,
      "summary_lang": "简体中文",
      "polish_bilingual_srt": True,
      "export_pdf": True,
    }

  raise ValueError(f"Unknown workflow '{workflow_name}'")


def run_via_gateway(
  opener: urllib.request.OpenerDirector,
  server_url: str,
  token: str,
  workflow_name: str,
  vendor: VendorSpec,
  interactive_hitl: bool,
  video_data_uri: str,
  timeout_sec: float,
) -> BenchResult:
  yaml_path = GENERATORS_DIR / WORKFLOW_FILES[workflow_name]
  workflow_yaml = yaml_path.read_text(encoding="utf-8")
  inputs = build_workflow_inputs(workflow_name, vendor, interactive_hitl, video_data_uri)
  mode = "gateway-hitl" if interactive_hitl else "gateway"

  t0 = time.monotonic()
  answered_ids: set[str] = set()
  hitl_questions: list[str] = []
  run_id = ""

  try:
    req_payload: dict[str, Any] = {
      "workflow": workflow_yaml,
      "model": vendor.chat_model,
      "inputs": inputs,
    }
    if vendor.label.lower() in ("moonshot", "glm"):
      req_payload["max_concurrency"] = 1
    created = http_json(
      opener,
      "POST",
      f"{server_url.rstrip('/')}/v1/runs",
      token,
      req_payload,
      timeout=30.0,
    )
    run_id = str(created.get("id") or created.get("run_id") or "")
    if not run_id:
      raise RuntimeError(f"POST /v1/runs returned no run id: {created}")

    status = "queued"
    cost_usd = 0.0
    error_msg = ""
    deadline = t0 + timeout_sec

    while time.monotonic() < deadline:
      run_doc = http_json(
        opener,
        "GET",
        f"{server_url.rstrip('/')}/v1/runs/{run_id}",
        token,
        timeout=25.0,
      )
      status = str(run_doc.get("status", "unknown"))
      cost_usd = float(run_doc.get("cost_usd") or 0.0)
      error_msg = str(run_doc.get("error_message") or run_doc.get("error") or "")

      if status in ("succeeded", "completed", "failed", "cancelled", "timed_out"):
        break

      if interactive_hitl and status == "running":
        q_doc = http_json(
          opener,
          "GET",
          f"{server_url.rstrip('/')}/v1/runs/{run_id}/questions",
          token,
          timeout=15.0,
        )
        q_list = q_doc.get("pending") if isinstance(q_doc.get("pending"), list) else q_doc.get("questions", [])
        for q in q_list:
          qid = str(q.get("question_id", ""))
          prompt_text = str(q.get("question", "")).strip()
          if qid and qid not in answered_ids:
            ans_idx = min(len(answered_ids), len(HITL_CANNED_ANSWERS) - 1)
            answer_text = HITL_CANNED_ANSWERS[ans_idx]
            http_json(
              opener,
              "POST",
              f"{server_url.rstrip('/')}/v1/runs/{run_id}/questions/{qid}",
              token,
              {"answer": answer_text},
              timeout=15.0,
            )
            answered_ids.add(qid)
            hitl_questions.append(prompt_text[:200])
            print(
              f"    [HITL] Answered question #{len(answered_ids)} ({qid[:8]}): {prompt_text[:80]}...",
              flush=True,
            )

      time.sleep(2.5)
    else:
      status = "poll_timeout"
      error_msg = f"Exceeded benchmark poll timeout ({timeout_sec}s)"

    elapsed = round(time.monotonic() - t0, 2)
    artifacts: list[ArtifactRecord] = []
    if run_id:
      art_doc = http_json(
        opener,
        "GET",
        f"{server_url.rstrip('/')}/v1/runs/{run_id}/artifacts",
        token,
        timeout=25.0,
      )
      for item in art_doc.get("artifacts", []):
        artifacts.append(
          ArtifactRecord(
            name=str(item.get("name", "")),
            node_id=str(item.get("node_id", "")),
            mime_type=str(item.get("mime_type", "")),
            size_bytes=int(item.get("size_bytes") or 0),
          )
        )

    return BenchResult(
      vendor=vendor.label,
      chat_model=vendor.chat_model,
      workflow=workflow_name,
      mode=mode,
      status=status,
      duration_sec=elapsed,
      cost_usd=round(cost_usd, 6),
      artifact_count=len(artifacts),
      artifacts=artifacts,
      hitl_rounds=len(answered_ids),
      hitl_questions=hitl_questions,
      run_id=run_id,
      error=error_msg,
    )
  except urllib.error.HTTPError as exc:
    body = exc.read().decode("utf-8", errors="replace")[:400]
    return BenchResult(
      vendor=vendor.label,
      chat_model=vendor.chat_model,
      workflow=workflow_name,
      mode=mode,
      status="http_error",
      duration_sec=round(time.monotonic() - t0, 2),
      cost_usd=0.0,
      artifact_count=0,
      run_id=run_id,
      error=f"HTTP {exc.code}: {body}",
    )
  except Exception as exc:
    return BenchResult(
      vendor=vendor.label,
      chat_model=vendor.chat_model,
      workflow=workflow_name,
      mode=mode,
      status="client_error",
      duration_sec=round(time.monotonic() - t0, 2),
      cost_usd=0.0,
      artifact_count=0,
      run_id=run_id,
      error=f"{type(exc).__name__}: {exc}",
    )


def ensure_builtin_models_config(out_root: pathlib.Path) -> pathlib.Path:
  """Materialize bundled templates into a standalone models.yml so local CLI
  benchmarks are not shadowed by a stale ~/.yanshi/models.yml."""
  out_root.mkdir(parents=True, exist_ok=True)
  target = out_root / "builtin_models.yml"
  templates_dir = REPO_ROOT / "yanshi-llm" / "templates"
  base_txt = (templates_dir / "default_models.yml").read_text(encoding="utf-8")
  chunks = [base_txt.rstrip(), "", "models:"]
  for yml in sorted((templates_dir / "models").glob("*.yml")):
    lines = yml.read_text(encoding="utf-8").splitlines()
    for idx, line in enumerate(lines):
      if line.strip() == "models:":
        chunks.extend(lines[idx + 1 :])
        break
  target.write_text("\n".join(chunks) + "\n", encoding="utf-8")
  return target


def run_via_cli(
  cli_bin: pathlib.Path,
  out_root: pathlib.Path,
  workflow_name: str,
  vendor: VendorSpec,
  video_data_uri: str,
  timeout_sec: float,
) -> BenchResult:
  yaml_path = GENERATORS_DIR / WORKFLOW_FILES[workflow_name]
  inputs = build_workflow_inputs(workflow_name, vendor, False, video_data_uri)
  run_dir = out_root / f"{vendor.label}_{workflow_name}"
  run_dir.mkdir(parents=True, exist_ok=True)
  models_cfg = ensure_builtin_models_config(out_root)

  cmd = [
    str(cli_bin),
    "workflow",
    "run",
    str(yaml_path),
    "--model",
    vendor.chat_model,
    "--execution-mode",
    "concurrent",
    "--timeout",
    f"{int(timeout_sec)}s",
    "--output-dir",
    str(run_dir),
  ]
  for k, v in inputs.items():
    val_str = json.dumps(v, ensure_ascii=False) if isinstance(v, (bool, int, float, dict, list)) else str(v)
    cmd.extend(["--input", k, val_str])

  env = os.environ.copy()
  env.setdefault("YANSHI_MODELS_CONFIG", str(models_cfg))
  env.setdefault("YANSHI_PRICING_TABLE", str(REPO_ROOT / "config" / "pricing.prod.yml"))

  t0 = time.monotonic()
  try:
    proc = subprocess.run(
      cmd,
      cwd=str(REPO_ROOT),
      env=env,
      capture_output=True,
      text=True,
      timeout=timeout_sec,
      check=False,
    )
    elapsed = round(time.monotonic() - t0, 2)
    artifacts: list[ArtifactRecord] = []
    art_root = run_dir / "artifacts"
    scan_dir = art_root if art_root.is_dir() else run_dir
    for fpath in sorted(scan_dir.rglob("*")):
      if fpath.is_file() and not fpath.name.startswith(".") and fpath.name != "artifacts.json":
        rel_parts = fpath.relative_to(scan_dir).parts
        node_id = rel_parts[0] if len(rel_parts) > 1 else "output"
        artifacts.append(
          ArtifactRecord(
            name=fpath.name,
            node_id=node_id,
            mime_type=fpath.suffix.lstrip(".") or "bin",
            size_bytes=fpath.stat().st_size,
          )
        )

    status = "succeeded" if proc.returncode == 0 else "failed"
    err = ""
    if proc.returncode != 0:
      combined = (proc.stderr or proc.stdout or "").strip()
      err = combined[-500:]
    return BenchResult(
      vendor=vendor.label,
      chat_model=vendor.chat_model,
      workflow=workflow_name,
      mode="local-cli",
      status=status,
      duration_sec=elapsed,
      cost_usd=0.0,
      artifact_count=len(artifacts),
      artifacts=artifacts,
      error=err,
    )
  except subprocess.TimeoutExpired:
    return BenchResult(
      vendor=vendor.label,
      chat_model=vendor.chat_model,
      workflow=workflow_name,
      mode="local-cli",
      status="timeout",
      duration_sec=round(time.monotonic() - t0, 2),
      cost_usd=0.0,
      artifact_count=0,
      error=f"CLI timed out after {timeout_sec}s",
    )


def render_markdown_report(results: list[BenchResult]) -> str:
  lines = [
    "# Yanshi 五大日常工作流 × 多模型厂商实测报告",
    "",
    "| 厂商 | 主模型 (`chat_model`) | 工作流 | 运行模式 | 状态 | 耗时 (s) | 估算成本 ($) | 产物数 | 关键产物明细 |",
    "| --- | --- | --- | --- | --- | ---: | ---: | ---: | --- |",
  ]
  for r in results:
    sample_arts = ", ".join(
      f"`{a.name}` ({a.size_bytes // 1024 or 1}KB)" for a in r.artifacts[:6]
    )
    if len(r.artifacts) > 6:
      sample_arts += f" +{len(r.artifacts) - 6} more"
    if r.hitl_rounds > 0:
      sample_arts = f"[HITL {r.hitl_rounds}轮] " + sample_arts
    if r.error and r.status not in ("succeeded", "completed"):
      sample_arts = r.error[-180:].replace("\n", " ")
    lines.append(
      f"| {r.vendor} | `{r.chat_model}` | `{r.workflow}` | `{r.mode}` | **{r.status}** | "
      f"{r.duration_sec:.1f} | {r.cost_usd:.4f} | {r.artifact_count} | {sample_arts} |"
    )
  return "\n".join(lines) + "\n"


def load_video_source(video_file: Optional[str], fallback_url: str) -> str:
  if video_file:
    p = pathlib.Path(video_file)
    b64 = base64.b64encode(p.read_bytes()).decode("ascii")
    mime = "video/mp4" if p.suffix.lower() != ".mp3" else "audio/mpeg"
    return f"data:{mime};base64,{b64}"
  return fallback_url


def main() -> int:
  parser = argparse.ArgumentParser(description="Benchmark Yanshi daily workflows across LLM vendors.")
  parser.add_argument("--server", default="", help="Gateway base URL, e.g. https://yanshi.yeying.dev")
  parser.add_argument("--cli-bin", default="", help="Local yanshi CLI binary path (alternative to --server)")
  parser.add_argument(
    "--vendors",
    required=True,
    help="Comma-separated vendor specs: label:chat_model[:image_model[:asr_model]]",
  )
  parser.add_argument(
    "--workflows",
    default=",".join(WORKFLOW_FILES.keys()),
    help="Comma-separated workflow names",
  )
  parser.add_argument("--hitl-vendor", default="", help="Vendor label on which to ALSO run interactive=true HITL test")
  parser.add_argument("--hitl-only", action="store_true", help="Only run interactive=true HITL mode for backend_project_planner")
  parser.add_argument("--video-file", default="", help="Local MP4/MP3 sample encoded as data URI for video_summarizer")
  parser.add_argument(
    "--video-url",
    default="https://media.w3.org/2010/05/sintel/trailer.mp4",
    help="Fallback online video URL when --video-file is omitted",
  )
  parser.add_argument("--timeout", type=float, default=420.0, help="Per-run timeout in seconds")
  parser.add_argument("--no-proxy", action="store_true", help="Bypass HTTP_PROXY/HTTPS_PROXY for gateway calls")
  parser.add_argument("--out-dir", default="/tmp/yanshi-bench-out", help="Directory for local CLI outputs")
  parser.add_argument("--report-json", default="", help="Path to write structured JSON results")
  parser.add_argument("--report-md", default="", help="Path to write Markdown summary table")
  args = parser.parse_args()

  if bool(args.server) == bool(args.cli_bin):
    sys.stderr.write("Specify exactly one of --server or --cli-bin\n")
    return 2

  vendors = [parse_vendor_spec(s) for s in args.vendors.split(",") if s.strip()]
  workflows = [w.strip() for w in args.workflows.split(",") if w.strip()]
  for w in workflows:
    if w not in WORKFLOW_FILES:
      sys.stderr.write(f"Unknown workflow '{w}'. Valid: {', '.join(WORKFLOW_FILES)}\n")
      return 2

  video_src = load_video_source(args.video_file or None, args.video_url)
  opener = build_opener(args.no_proxy)
  token = os.environ.get("YANSHI_API_TOKEN", "").strip()
  out_root = pathlib.Path(args.out_dir)

  results: list[BenchResult] = []
  for vendor in vendors:
    for wf in workflows:
      if not args.hitl_only:
        print(f"==> [{vendor.label} / {vendor.chat_model}] Running {wf} ...", flush=True)
        if args.server:
          res = run_via_gateway(
            opener, args.server, token, wf, vendor, False, video_src, args.timeout
          )
        else:
          res = run_via_cli(
            pathlib.Path(args.cli_bin), out_root, wf, vendor, video_src, args.timeout
          )
        results.append(res)
        print(
          f"    -> {res.status} in {res.duration_sec:.1f}s | cost=${res.cost_usd:.4f} | artifacts={res.artifact_count}"
          + (f" | err={res.error[-220:].replace(chr(10), ' ')}" if res.error else ""),
          flush=True,
        )

      if args.server and wf == "backend_project_planner" and (args.hitl_only or args.hitl_vendor == vendor.label):
        print(
          f"==> [{vendor.label} / {vendor.chat_model}] Running {wf} (interactive HITL ask_user=true) ...",
          flush=True,
        )
        hitl_res = run_via_gateway(
          opener, args.server, token, wf, vendor, True, video_src, args.timeout
        )
        results.append(hitl_res)
        print(
          f"    -> {hitl_res.status} in {hitl_res.duration_sec:.1f}s | hitl_rounds={hitl_res.hitl_rounds} "
          f"| artifacts={hitl_res.artifact_count}"
          + (f" | err={hitl_res.error[-220:].replace(chr(10), ' ')}" if hitl_res.error else ""),
          flush=True,
        )

  md_table = render_markdown_report(results)
  print("\n" + md_table, flush=True)

  if args.report_json:
    p = pathlib.Path(args.report_json)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(json.dumps([asdict(r) for r in results], ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
  if args.report_md:
    p = pathlib.Path(args.report_md)
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(md_table, encoding="utf-8")

  failed = [r for r in results if r.status not in ("succeeded", "completed")]
  return 1 if failed else 0


if __name__ == "__main__":
  sys.exit(main())
