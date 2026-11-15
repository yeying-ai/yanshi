#!/usr/bin/env bash
# ==============================================================================
# Yanshi Production Deploy & End-to-End Verification Script (Pingora + Axum + PG16)
# Modeled after `rustineverything.app/publish.sh`.
#
# Usage:
#   ./publish.sh                  # Full deploy: build UI + sync certs & source + build/restart containers + verify
#   ./publish.sh --skip-ui-build  # Reuse existing yanshi-ui/dist bundle
#   ./publish.sh --certs-only     # Only sync renewed local certs and restart Pingora gateway
# ==============================================================================
set -euo pipefail

REMOTE_HOST="${REMOTE_HOST:-cu}"
REMOTE_BASE="${REMOTE_BASE:-/data/yanshi-prod}"
REMOTE_SRC_DIR="${REMOTE_BASE}/src"
REMOTE_CERT_DIR="${REMOTE_BASE}/certs"
REMOTE_DATA_DIR="${REMOTE_BASE}/data"
LOCAL_REPO_DIR="$(cd "$(dirname "$0")" && pwd)"

CERT_SRC_DIR="${CERT_SRC_DIR:-/Users/hal/conf/certs}"
CERT_DOMAIN="${CERT_DOMAIN:-yeying.dev}"
SITE_HOST="${SITE_HOST:-yanshi.yeying.dev}"
SITE_URL="${SITE_URL:-https://${SITE_HOST}}"
LOCAL_UPSTREAM_PORT="${LOCAL_UPSTREAM_PORT:-18080}"

SSH_MUX_SOCK="/tmp/yanshi-publish-ssh-%C"
SSH_OPTS=(-o ControlMaster=auto -o "ControlPath=${SSH_MUX_SOCK}" -o ControlPersist=300 -o ServerAliveInterval=15 -o ServerAliveCountMax=6 -o ConnectTimeout=20)

cleanup_ssh() {
  ssh -o "ControlPath=${SSH_MUX_SOCK}" -O exit "$REMOTE_HOST" >/dev/null 2>&1 || true
}
trap cleanup_ssh EXIT

rssh() {
  ssh "${SSH_OPTS[@]}" "$REMOTE_HOST" "$@"
}

DO_UI_BUILD=1
CERTS_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-ui-build)
      DO_UI_BUILD=0
      shift
      ;;
    --certs-only)
      CERTS_ONLY=1
      shift
      ;;
    -h|--help)
      sed -n '2,10p' "$0"
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      exit 1
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

LOCAL_CERT="${CERT_SRC_DIR}/${CERT_DOMAIN}.cert"
LOCAL_KEY="${CERT_SRC_DIR}/${CERT_DOMAIN}.key"

# ------------------------------------------------------------------------------
# 1. Validate & sync TLS certificates
# ------------------------------------------------------------------------------
log "Validating local TLS certificate (${LOCAL_CERT})"
[[ -f "$LOCAL_CERT" ]] || { err "Missing certificate: ${LOCAL_CERT}"; exit 1; }
[[ -f "$LOCAL_KEY" ]] || { err "Missing private key: ${LOCAL_KEY}"; exit 1; }

pub_cert="$(openssl x509 -in "$LOCAL_CERT" -noout -pubkey | openssl sha256)"
pub_key="$(openssl pkey -in "$LOCAL_KEY" -pubout | openssl sha256)"
if [[ "$pub_cert" != "$pub_key" ]]; then
  err "Certificate and private key public-key hashes do not match"
  exit 1
fi
ok "TLS certificate/key pair verified ($(openssl x509 -in "$LOCAL_CERT" -noout -enddate | cut -d= -f2))"

log "Preparing remote directories on ${REMOTE_HOST}:${REMOTE_BASE}"
rssh "mkdir -p '${REMOTE_SRC_DIR}' '${REMOTE_CERT_DIR}' '${REMOTE_DATA_DIR}/runs' && chmod 700 '${REMOTE_CERT_DIR}' && chown -R 10001:10001 '${REMOTE_DATA_DIR}'"

scp "${SSH_OPTS[@]}" "$LOCAL_CERT" "${REMOTE_HOST}:${REMOTE_CERT_DIR}/${CERT_DOMAIN}.cert"
scp "${SSH_OPTS[@]}" "$LOCAL_KEY" "${REMOTE_HOST}:${REMOTE_CERT_DIR}/${CERT_DOMAIN}.key"
rssh "chmod 644 '${REMOTE_CERT_DIR}/${CERT_DOMAIN}.cert' && chmod 600 '${REMOTE_CERT_DIR}/${CERT_DOMAIN}.key'"
ok "Synced TLS certificates to ${REMOTE_HOST}:${REMOTE_CERT_DIR}"

if [[ "$CERTS_ONLY" -eq 1 ]]; then
  log "Restarting Pingora gateway container (yanshi-gateway)"
  rssh "docker restart yanshi-gateway"
  ok "Gateway restarted with updated certificates"
  exit 0
fi

# ------------------------------------------------------------------------------
# 2. Build embedded Web UI bundle & sync repository source
# ------------------------------------------------------------------------------
if [[ "$DO_UI_BUILD" -eq 1 ]]; then
  log "Building embedded Yanshi Studio Web bundle (yanshi-ui/dist)"
  npm --prefix "${LOCAL_REPO_DIR}/yanshi-ui" run build
  ok "Frontend bundle built"
fi

log "Syncing source workspace to ${REMOTE_HOST}:${REMOTE_SRC_DIR}"
rsync -az --delete \
  -e "ssh ${SSH_OPTS[*]}" \
  --exclude 'target/' \
  --exclude 'crates/gateway/target/' \
  --exclude 'yanshi-ui/node_modules/' \
  --exclude 'yanshi-ui/src-tauri/target/' \
  --exclude '.git/' \
  --exclude '.claude/' \
  --exclude '.playwright-mcp/' \
  --exclude 'runs/' \
  --exclude 'yanshi-cli/runs/' \
  --exclude 'e2e_*' \
  --exclude 'TODOs.md' \
  --exclude '.env' \
  --exclude '.env.*' \
  --include '.env.prod.example' \
  "${LOCAL_REPO_DIR}/" "${REMOTE_HOST}:${REMOTE_SRC_DIR}/"
ok "Workspace synced"

# ------------------------------------------------------------------------------
# 3. Bootstrap remote .env (preserving secrets) & launch Compose stack
# ------------------------------------------------------------------------------
log "Ensuring production .env and building/restarting containers on ${REMOTE_HOST}"
rssh "REMOTE_BASE='${REMOTE_BASE}' REMOTE_SRC_DIR='${REMOTE_SRC_DIR}' SITE_HOST='${SITE_HOST}' SITE_URL='${SITE_URL}' CERT_DOMAIN='${CERT_DOMAIN}' LOCAL_UPSTREAM_PORT='${LOCAL_UPSTREAM_PORT}' bash -s" <<'EOF'
set -euo pipefail

ENV_FILE="${REMOTE_BASE}/.env"
if [[ ! -f "$ENV_FILE" ]]; then
  PG_PASS="$(openssl rand -hex 24)"
  API_TOK="$(openssl rand -hex 32)"
  umask 077
  cat >"$ENV_FILE" <<ENVEOF
POSTGRES_DB=yanshi
POSTGRES_USER=yanshi
POSTGRES_PASSWORD=${PG_PASS}

YANSHI_HOST_PORT=${LOCAL_UPSTREAM_PORT}
YANSHI_DATA_HOST_DIR=${REMOTE_BASE}/data
CERT_HOST_DIR=${REMOTE_BASE}/certs
TLS_CERT_FILE=${CERT_DOMAIN}.cert
TLS_KEY_FILE=${CERT_DOMAIN}.key

HOST_UPSTREAMS=${SITE_HOST}=127.0.0.1:${LOCAL_UPSTREAM_PORT}
HOST_REDIRECTS=www.${SITE_HOST}=${SITE_URL}
GATEWAY_RUST_LOG=info

YANSHI_CORS_ALLOWED_ORIGINS=${SITE_URL}
YANSHI_API_TOKEN=${API_TOK}
YANSHI_GATEWAY_HOST_NODE_TYPES=file,agent,skill_agent,multi_agent,shell
YANSHI_GATEWAY_ALLOWED_COMMANDS=ffmpeg,ffprobe,typst,pandoc
YANSHI_RUN_MAX_CONCURRENCY=4
CARGO_BUILD_JOBS=2
RUST_LOG=info
ENVEOF

  # Import non-empty provider credentials from /root/.bashrc if present without printing values
  python3 - "$ENV_FILE" <<'PY'
import pathlib, re, sys

env_path = pathlib.Path(sys.argv[1])
bashrc = pathlib.Path("/root/.bashrc")
wanted = [
  "OPENAI_API_KEY", "OPENAI_BASE_URL",
  "ANTHROPIC_API_KEY", "ANTHROPIC_BASE_URL",
  "DEEPSEEK_API_KEY", "DEEPSEEK_BASE_URL",
  "GEMINI_API_KEY", "GOOGLE_API_KEY",
  "GLM_API_KEY", "ZHIPU_API_KEY",
  "MOONSHOT_API_KEY", "STEPFUN_API_KEY",
  "DASHSCOPE_API_KEY", "MINIMAX_API_KEY", "KLING_API_KEY",
]
found = {}
if bashrc.is_file():
  for line in bashrc.read_text(encoding="utf-8", errors="ignore").splitlines():
    m = re.match(r"^\s*(?:export\s+)?([A-Z0-9_]+)=(.+?)\s*$", line)
    if m and m.group(1) in wanted:
      val = m.group(2).strip().strip("'").strip('"')
      if val and "$" not in val:
        found[m.group(1)] = val

if found:
  with env_path.open("a", encoding="utf-8") as f:
    f.write("\n# Imported LLM provider credentials\n")
    for k, v in sorted(found.items()):
      f.write(f"{k}={v}\n")
  print("Imported provider keys:", ", ".join(sorted(found.keys())))
PY
  chmod 600 "$ENV_FILE"
  echo "Created fresh production .env at ${ENV_FILE}"
fi

cd "$REMOTE_SRC_DIR"
DOCKER_BUILDKIT=1 docker compose --env-file "$ENV_FILE" \
  -f docker-compose.yml -f docker-compose.prod.yml \
  build gateway
DOCKER_BUILDKIT=1 docker compose --env-file "$ENV_FILE" \
  -f docker-compose.yml -f docker-compose.prod.yml \
  build yanshi-server
DOCKER_BUILDKIT=1 docker compose --env-file "$ENV_FILE" \
  -f docker-compose.yml -f docker-compose.prod.yml \
  up -d --remove-orphans

docker ps --format "table {{.Names}}\t{{.Image}}\t{{.Status}}\t{{.Ports}}" | grep -E "NAMES|yanshi-"
EOF
ok "Containers built and started"

# ------------------------------------------------------------------------------
# 4. End-to-end health, HTTP->HTTPS redirect, TLS, UI & diagnostics verification
# ------------------------------------------------------------------------------
log "Verifying upstream readiness & public HTTPS endpoints (${SITE_URL})"
rssh "LOCAL_UPSTREAM_PORT='${LOCAL_UPSTREAM_PORT}' SITE_HOST='${SITE_HOST}' SITE_URL='${SITE_URL}' REMOTE_BASE='${REMOTE_BASE}' bash -s" <<'EOF'
set -euo pipefail

for i in $(seq 1 30); do
  if curl -fsS -o /dev/null "http://127.0.0.1:${LOCAL_UPSTREAM_PORT}/health/ready"; then
    echo "Loopback 127.0.0.1:${LOCAL_UPSTREAM_PORT}/health/ready ready (attempt ${i})"
    break
  fi
  if [[ "$i" -eq 30 ]]; then
    echo "ERROR: yanshi-server did not become ready in time" >&2
    docker logs --tail 60 yanshi-server >&2
    exit 1
  fi
  sleep 1
done

http_code="$(curl -sS -o /dev/null -w '%{http_code}' --resolve "${SITE_HOST}:80:127.0.0.1" "http://${SITE_HOST}/")"
[[ "$http_code" == "301" ]] || { echo "Expected 301 on :80, got ${http_code}" >&2; exit 1; }
echo "HTTP :80 -> 301 redirect verified"

https_health="$(curl -fsS --resolve "${SITE_HOST}:443:127.0.0.1" "${SITE_URL}/health/ready")"
echo "HTTPS /health/ready: ${https_health}"

ui_code="$(curl -fsSL -o /tmp/yanshi_ui_check.$$ -w '%{http_code}' --resolve "${SITE_HOST}:443:127.0.0.1" "${SITE_URL}/")"
grep -q "yanshi-debugger" /tmp/yanshi_ui_check.$$
rm -f /tmp/yanshi_ui_check.$$
echo "HTTPS / -> /ui/studio SPA verified (HTTP ${ui_code})"

api_token="$(grep -E '^YANSHI_API_TOKEN=' "${REMOTE_BASE}/.env" | head -n 1 | cut -d= -f2-)"
diag_code="$(curl -fsS -o /dev/null -w '%{http_code}' \
  --resolve "${SITE_HOST}:443:127.0.0.1" \
  -H "Authorization: Bearer ${api_token}" \
  "${SITE_URL}/v1/diagnostics")"
echo "HTTPS /v1/diagnostics authenticated check: HTTP ${diag_code}"
EOF
ok "Remote loopback & Pingora TLS verification passed"

log "Verifying public internet reachability from local machine (${SITE_URL})"
pub_code="$(curl -fsSL -o /tmp/yanshi_pub_ui.$$ -w '%{http_code}' "${SITE_URL}/")"
grep -q "yanshi-debugger" /tmp/yanshi_pub_ui.$$
rm -f /tmp/yanshi_pub_ui.$$
ok "Public ${SITE_URL}/ -> /ui/studio HTTP ${pub_code} verified!"
