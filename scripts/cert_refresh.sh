#!/usr/bin/env bash
# Refresh renewed TLS certificate on `cu`, verify SAN + private-key match + chain + expiry,
# atomically replace the active certificate, and restart `yanshi-gateway` (Pingora).
# Modeled after `rustineverything.app/scripts/cert_refresh.sh`.
#
# Environment variables:
#   CERT_ENV_FILE    Optional env file (default: /data/yanshi-prod/cert.env)
#   CERT_DIR         Certificate directory (default: /data/yanshi-prod/certs)
#   CERT_FILE        Active cert filename (default: yeying.dev.cert)
#   KEY_FILE         Active key filename (default: yeying.dev.key)
#   DOMAIN           Expected SAN DNS entry (default: yeying.dev)
#   RESTART_CMD      Command run after cert swap (default: docker restart yanshi-gateway)
#   CERT_URL         Override download URL (e.g. file:// for tests)
#   DYNADOT_API_KEY  Dynadot certificate download key when CERT_URL is unset
set -euo pipefail

CERT_ENV_FILE="${CERT_ENV_FILE:-/data/yanshi-prod/cert.env}"
# shellcheck disable=SC1090
[[ -f "$CERT_ENV_FILE" ]] && . "$CERT_ENV_FILE"

CERT_DIR="${CERT_DIR:-/data/yanshi-prod/certs}"
CERT_FILE="${CERT_FILE:-yeying.dev.cert}"
KEY_FILE="${KEY_FILE:-yeying.dev.key}"
DOMAIN="${DOMAIN:-yeying.dev}"
RESTART_CMD="${RESTART_CMD:-docker restart yanshi-gateway}"

log() {
  echo "cert_refresh: $*"
}

fail() {
  log "ERROR: $*" >&2
  exit 1
}

current="${CERT_DIR}/${CERT_FILE}"
key="${CERT_DIR}/${KEY_FILE}"
[[ -f "$current" ]] && [[ -f "$key" ]] || fail "missing ${CERT_FILE} or ${KEY_FILE} in ${CERT_DIR}"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
new="${tmp}/fullchain.pem"

if [[ -n "${CERT_URL:-}" ]]; then
  url="$CERT_URL"
else
  [[ -n "${DYNADOT_API_KEY:-}" ]] || fail "DYNADOT_API_KEY is not set"
  url="https://www.dynadot.com/letsencrypt/download_cert?key=${DYNADOT_API_KEY}&domain=${DOMAIN}"
fi

printf 'url = "%s"\n' "$url" | curl -fsS --max-time 60 -K - -o "$new" \
  || fail "certificate download failed"
grep -q -- '-----BEGIN CERTIFICATE-----' "$new" \
  || fail "downloaded file is not PEM certificate data"

leaf="${tmp}/leaf.pem"
openssl x509 -in "$new" -out "$leaf" 2>/dev/null || fail "failed to parse leaf X509 certificate"

fingerprint() {
  openssl x509 -in "$1" -noout -fingerprint -sha256
}

if [[ "$(fingerprint "$leaf")" == "$(fingerprint "$current")" ]]; then
  log "certificate unchanged; nothing to do"
  exit 0
fi

san_text="$(openssl x509 -in "$leaf" -noout -ext subjectAltName | tr ',' '\n' | sed 's/^ *//')"
if ! grep -qxF "DNS:${DOMAIN}" <<<"$san_text" && ! grep -qxF "DNS:*.${DOMAIN}" <<<"$san_text"; then
  fail "new certificate SAN does not cover ${DOMAIN}"
fi

pub_of_cert="$(openssl x509 -in "$leaf" -noout -pubkey | openssl sha256)"
pub_of_key="$(openssl pkey -in "$key" -pubout | openssl sha256)"
[[ "$pub_of_cert" == "$pub_of_key" ]] \
  || fail "new certificate public key does not match existing private key"

verify_args=(-untrusted "$new")
[[ -n "${CA_FILE:-}" ]] && verify_args+=(-CAfile "$CA_FILE")
openssl verify "${verify_args[@]}" "$leaf" >/dev/null 2>&1 || fail "certificate chain verification failed"

not_after() {
  openssl x509 -in "$1" -noout -enddate | cut -d= -f2
}

newer="$(python3 -c 'import ssl, sys; print(int(ssl.cert_time_to_seconds(sys.argv[1]) > ssl.cert_time_to_seconds(sys.argv[2])))' \
  "$(not_after "$leaf")" "$(not_after "$current")")"
[[ "$newer" == "1" ]] || fail "new certificate expiry ($(not_after "$leaf")) is not newer than current ($(not_after "$current"))"

cp "$current" "${current}.bak"
cp "$new" "${current}.new"
chmod 0644 "${current}.new"
mv "${current}.new" "$current"
log "updated certificate; expires $(not_after "$leaf")"
sh -c "$RESTART_CMD" || fail "certificate updated, but gateway restart failed: ${RESTART_CMD}"
log "restarted gateway"
