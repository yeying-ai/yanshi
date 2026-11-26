#!/usr/bin/env bash
# ==============================================================================
# Build the Phase 0 Dioxus Web Spike (`yanshi-ui-dx`) with Tailwind CSS v4.
#
# - Prefers Tailwind CSS v4 standalone CLI (zero Node.js) and caches it under
#   `${YANSHI_TAILWIND_CACHE:-$HOME/.cache/yanshi-tailwind}`.
# - Automatically resolves `@source` for `dioxus-shadcn` via `cargo metadata`
#   so both crates.io and local `[patch.crates-io]` builds scan exact sources.
# - Runs `dx build --platform web --release --base-path ui-dx` and emits a
#   compact raw + gzip artifact size report.
# ==============================================================================
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
APP_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
OUT_DIR="${APP_DIR}/dist"
CSS_INPUT="${APP_DIR}/assets/tailwind.input.css"
CSS_RESOLVED="${APP_DIR}/assets/.tailwind.resolved.css"
CSS_OUTPUT="${APP_DIR}/assets/app.generated.css"
TAILWIND_VERSION="${TAILWIND_VERSION:-v4.1.18}"
TAILWIND_CACHE="${YANSHI_TAILWIND_CACHE:-${HOME}/.cache/yanshi-tailwind}"

log() {
  printf "\033[1;34m==>\033[0m %s\n" "$*"
}

ensure_macos_adhoc_signed() {
  local bin="$1"
  if [[ "$(uname -s)" == "Darwin" ]]; then
    if ! "${bin}" --help >/dev/null 2>&1; then
      xattr -d com.apple.quarantine "${bin}" 2>/dev/null || true
      codesign --force --sign - "${bin}" >/dev/null 2>&1 || true
    fi
  fi
}

resolve_tailwind_bin() {
  if [[ -n "${TAILWIND_BIN:-}" && -x "${TAILWIND_BIN}" ]]; then
    ensure_macos_adhoc_signed "${TAILWIND_BIN}"
    printf '%s\n' "${TAILWIND_BIN}"
    return 0
  fi
  if command -v tailwindcss >/dev/null 2>&1; then
    command -v tailwindcss
    return 0
  fi

  local os arch asset cached
  os="$(uname -s)"
  arch="$(uname -m)"
  case "${os}:${arch}" in
    Darwin:arm64) asset="tailwindcss-macos-arm64" ;;
    Darwin:x86_64) asset="tailwindcss-macos-x64" ;;
    Linux:x86_64) asset="tailwindcss-linux-x64" ;;
    Linux:aarch64|Linux:arm64) asset="tailwindcss-linux-arm64" ;;
    *) return 1 ;;
  esac

  cached="${TAILWIND_CACHE}/${TAILWIND_VERSION}/${asset}"
  if [[ -x "${cached}" ]]; then
    ensure_macos_adhoc_signed "${cached}"
    if "${cached}" --help >/dev/null 2>&1; then
      printf '%s\n' "${cached}"
      return 0
    fi
  fi

  if [[ "${YANSHI_NO_DOWNLOAD:-0}" == "1" ]]; then
    return 1
  fi

  mkdir -p "$(dirname "${cached}")"
  local url="https://github.com/tailwindlabs/tailwindcss/releases/download/${TAILWIND_VERSION}/${asset}"
  log "Downloading Tailwind CSS v4 standalone CLI (${TAILWIND_VERSION}/${asset})..." >&2
  if curl -fL --connect-timeout 10 --max-time 90 -o "${cached}.tmp" "${url}"; then
    chmod +x "${cached}.tmp"
    mv "${cached}.tmp" "${cached}"
    ensure_macos_adhoc_signed "${cached}"
    if "${cached}" --help >/dev/null 2>&1; then
      printf '%s\n' "${cached}"
      return 0
    fi
  fi
  rm -f "${cached}.tmp"
  return 1
}

resolve_shadcn_source_dir() {
  # Run inside APP_DIR so Cargo picks up `yanshi-ui-dx/.cargo/config.toml`
  # (`[patch.crates-io]`) when developing locally against `dioxus-ui`.
  (
    cd "${APP_DIR}"
    cargo metadata --format-version 1
  ) | python3 -c '
import json, os, sys
meta = json.load(sys.stdin)
for pkg in meta.get("packages", []):
  if pkg.get("name") == "dioxus-shadcn":
    print(os.path.join(os.path.dirname(pkg["manifest_path"]), "src"))
    sys.exit(0)
sys.exit(1)
'
}

log "Resolving dioxus-shadcn source path from cargo metadata..."
SHADCN_SRC="$(resolve_shadcn_source_dir)"
log "dioxus-shadcn source: ${SHADCN_SRC}"

python3 - "${CSS_INPUT}" "${CSS_RESOLVED}" "${SHADCN_SRC}" <<'PY'
import sys
src_file, dst_file, shadcn_src = sys.argv[1], sys.argv[2], sys.argv[3]
text = open(src_file, "r", encoding="utf-8").read()
replacement = f'@source "{shadcn_src}";'
if "/* __DXUI_SOURCES__ */" not in text:
  raise SystemExit("missing /* __DXUI_SOURCES__ */ placeholder in tailwind.input.css")
open(dst_file, "w", encoding="utf-8").write(text.replace("/* __DXUI_SOURCES__ */", replacement, 1))
PY

log "Compiling Tailwind CSS v4..."
if TW_BIN="$(resolve_tailwind_bin)"; then
  log "Using standalone Tailwind CLI: ${TW_BIN}"
  "${TW_BIN}" -i "${CSS_RESOLVED}" -o "${CSS_OUTPUT}" --minify
else
  log "Standalone Tailwind CLI unavailable; falling back to Node @tailwindcss/node"
  NODE_TW_BASE="/Users/hal/arch/dioxus-ui"
  if [[ -d "${NODE_TW_BASE}/node_modules/tailwindcss" ]]; then
    node - "${CSS_RESOLVED}" "${CSS_OUTPUT}" "${NODE_TW_BASE}" <<'JS'
const fs = require("node:fs");
const path = require("node:path");
const { createRequire } = require("node:module");
const [inputPath, outputPath, baseDir] = process.argv.slice(2);
const req = createRequire(path.join(baseDir, "package.json"));
const { compile } = req("@tailwindcss/node");
const { Scanner } = req("@tailwindcss/oxide");
(async () => {
  const css = fs.readFileSync(inputPath, "utf8");
  const compiler = await compile(css, {
    base: path.dirname(inputPath),
    onDependency() {},
  });
  const sources = compiler.sources.map((s) => ({
    base: s.base,
    pattern: s.pattern,
    negated: s.negated,
  }));
  const scanner = new Scanner({ sources });
  const candidates = scanner.scan();
  const built = compiler.build(candidates);
  fs.writeFileSync(outputPath, built);
})().catch((err) => {
  console.error(err);
  process.exit(1);
});
JS
  else
    echo "Error: neither standalone tailwindcss CLI nor local @tailwindcss/node is available." >&2
    exit 1
  fi
fi

TARGET_DIR="$(cd "${APP_DIR}" && cargo metadata --no-deps --format-version 1 | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
DX_PUBLIC="${TARGET_DIR}/dx/yanshi-ui-dx/release/web/public"
rm -rf "${DX_PUBLIC}"

log "Building Dioxus Web WASM bundle (--release --base-path ui-dx)..."
mkdir -p "${OUT_DIR}"
(
  cd "${APP_DIR}"
  # Note: wasm-opt can SIGBUS on some local arm64 toolchains; allow opting out via DX_WASM_OPT=0.
  if [[ "${DX_WASM_OPT:-1}" == "0" ]]; then
    PATH="$(echo "$PATH" | tr ':' '\n' | grep -v '/opt/homebrew/bin' | paste -sd ':' -)" \
      dx build --platform web --release --base-path ui-dx
  else
    dx build --platform web --release --base-path ui-dx
  fi
)

# Copy Dioxus web bundle output into a stable `yanshi-ui-dx/dist/public` path.
if [[ ! -d "${DX_PUBLIC}" ]]; then
  echo "Error: expected dx output directory not found at ${DX_PUBLIC}" >&2
  exit 1
fi

rm -rf "${OUT_DIR}/public"
cp -R "${DX_PUBLIC}" "${OUT_DIR}/public"

log "Build complete: ${OUT_DIR}/public"
python3 - "${OUT_DIR}/public" <<'PY'
import gzip, os, sys
root = sys.argv[1]
rows = []
total_raw = 0
total_gz = 0
for dirpath, _, filenames in os.walk(root):
  for name in sorted(filenames):
    full = os.path.join(dirpath, name)
    rel = os.path.relpath(full, root)
    data = open(full, "rb").read()
    gz = len(gzip.compress(data, compresslevel=9))
    raw = len(data)
    total_raw += raw
    total_gz += gz
    rows.append((rel, raw, gz))
print(f"{'Artifact':<55} {'Raw':>10} {'Gzip':>10}")
print("-" * 77)
for rel, raw, gz in rows:
  print(f"{rel:<55} {raw/1024:>8.1f}KB {gz/1024:>8.1f}KB")
print("-" * 77)
print(f"{'TOTAL':<55} {total_raw/1024:>8.1f}KB {total_gz/1024:>8.1f}KB")
PY
