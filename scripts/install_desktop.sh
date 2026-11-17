#!/usr/bin/env bash
# ==============================================================================
# Build and install Yanshi Studio.app into /Applications on macOS.
#
# Usage:
#   ./scripts/install_desktop.sh               # Build release .app bundle and install to /Applications
#   ./scripts/install_desktop.sh --skip-build  # Install existing built Yanshi Studio.app without rebuilding
#   ./scripts/install_desktop.sh --open        # Launch Yanshi Studio.app after installation
#   ./scripts/install_desktop.sh --dest <dir>  # Override destination directory (default: /Applications)
# ==============================================================================
set -euo pipefail

REPO_DIR="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST_PATH="${REPO_DIR}/yanshi-ui/src-tauri/Cargo.toml"
APP_NAME="Yanshi Studio.app"
DEST_DIR="${DEST_DIR:-/Applications}"
DO_BUILD=1
OPEN_AFTER=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build)
      DO_BUILD=0
      shift
      ;;
    --open)
      OPEN_AFTER=1
      shift
      ;;
    --dest)
      [[ $# -ge 2 ]] || { echo "Error: --dest requires a directory path" >&2; exit 2; }
      DEST_DIR="$2"
      shift 2
      ;;
    --dest=*)
      DEST_DIR="${1#*=}"
      shift
      ;;
    -h|--help)
      sed -n '2,10p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1 (use --help for usage)" >&2
      exit 2
      ;;
  esac
done

log() {
  printf "\n\033[1;34m==> [%s] %s\033[0m\n" "$(date '+%H:%M:%S')" "$*"
}

ok() {
  printf "\033[1;32m✓ %s\033[0m\n" "$*"
}

err() {
  printf "\033[1;31m✗ %s\033[0m\n" "$*" >&2
}

if [[ "$(uname -s)" != "Darwin" ]]; then
  err "install_desktop.sh only targets macOS (.app -> /Applications)."
  exit 1
fi

if [[ "$DO_BUILD" -eq 1 ]]; then
  log "Building release macOS bundle (${APP_NAME})"
  npm --prefix "${REPO_DIR}/yanshi-ui" run tauri:build -- --bundles app
  ok "Built ${APP_NAME}"
fi

resolve_target_dir() {
  if [[ -n "${CARGO_TARGET_DIR:-}" ]]; then
    printf '%s\n' "$CARGO_TARGET_DIR"
    return
  fi
  cargo metadata --manifest-path "$MANIFEST_PATH" --no-deps --format-version 1 \
    | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])'
}

TARGET_DIR="$(resolve_target_dir)"
SRC_APP="${TARGET_DIR}/release/bundle/macos/${APP_NAME}"

if [[ ! -d "$SRC_APP" ]]; then
  err "Bundle not found at: ${SRC_APP}"
  err "Run ./scripts/install_desktop.sh without --skip-build first."
  exit 1
fi

mkdir -p "$DEST_DIR"
if [[ ! -w "$DEST_DIR" ]]; then
  err "Destination directory is not writable: ${DEST_DIR}"
  err "Re-run with elevated permissions (e.g. sudo ./scripts/install_desktop.sh --skip-build) or pass --dest ~/Applications."
  exit 1
fi

DEST_APP="${DEST_DIR}/${APP_NAME}"
STAGING_APP="${DEST_DIR}/.YanshiStudio.staging.$$.app"
cleanup_staging() {
  rm -rf "$STAGING_APP"
}
trap cleanup_staging EXIT

log "Staging ${APP_NAME} into ${DEST_DIR}"
ditto "$SRC_APP" "$STAGING_APP"
xattr -dr com.apple.quarantine "$STAGING_APP" 2>/dev/null || true

INNER_BIN="${STAGING_APP}/Contents/MacOS/yanshi-studio"
if [[ ! -x "$INNER_BIN" ]]; then
  err "Missing executable inside staged bundle: ${INNER_BIN}"
  exit 1
fi
"$INNER_BIN" --help >/dev/null
ok "Verified staged Mach-O binary (${INNER_BIN})"

if pgrep -x "yanshi-studio" >/dev/null 2>&1; then
  log "Stopping running Yanshi Studio instance before swap"
  osascript -e 'tell application "Yanshi Studio" to quit' >/dev/null 2>&1 || true
  for _ in {1..15}; do
    pgrep -x "yanshi-studio" >/dev/null 2>&1 || break
    sleep 0.2
  done
  pkill -x "yanshi-studio" >/dev/null 2>&1 || true
fi

rm -rf "$DEST_APP"
mv "$STAGING_APP" "$DEST_APP"

LSREGISTER="/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister"
if [[ -x "$LSREGISTER" ]]; then
  "$LSREGISTER" -f "$DEST_APP" >/dev/null 2>&1 || true
fi

ok "Installed ${DEST_APP}"

if [[ "$OPEN_AFTER" -eq 1 ]]; then
  log "Launching ${DEST_APP}"
  open "$DEST_APP"
fi
