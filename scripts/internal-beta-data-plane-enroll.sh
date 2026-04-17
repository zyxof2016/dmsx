#!/usr/bin/env bash
set -euo pipefail

# Internal beta: generate enrollment token + CSR, then call device-gw Enroll via grpcurl.
#
# Requirements:
# - openssl
# - python3
# - grpcurl (optional but recommended)
#
# Env:
# - GW_ADDR (default 127.0.0.1:50051)
# - TENANT_ID (required)
# - ENROLL_SECRET (required) -> maps to DMSX_GW_ENROLL_HMAC_SECRET on server
# - DEVICE_ID (optional; if set, token pins the device_id)

GW_ADDR="${GW_ADDR:-127.0.0.1:50051}"
TENANT_ID="${TENANT_ID:-}"
ENROLL_SECRET="${ENROLL_SECRET:-}"
DEVICE_ID="${DEVICE_ID:-}"

if [[ -z "${TENANT_ID}" || -z "${ENROLL_SECRET}" ]]; then
  echo "usage: TENANT_ID=<uuid> ENROLL_SECRET=<secret> [DEVICE_ID=<uuid>] $0" >&2
  exit 2
fi

WORKDIR="${WORKDIR:-/tmp/dmsx-enroll}"
mkdir -p "${WORKDIR}"

CSR_KEY="${WORKDIR}/device.key"
CSR_PEM="${WORKDIR}/device.csr"
OUT_JSON="${WORKDIR}/enroll.json"

echo "Generating RSA key + CSR in ${WORKDIR}"
openssl genrsa -out "${CSR_KEY}" 2048 >/dev/null 2>&1
openssl req -new -key "${CSR_KEY}" -out "${CSR_PEM}" -subj "/CN=dmsx-device" >/dev/null 2>&1

echo "Generating enrollment token"
TOKEN="$(
python3 - <<'PY'
import base64, hashlib, hmac, json, os, time, uuid

tenant_id=os.environ["TENANT_ID"].strip()
secret=os.environ["ENROLL_SECRET"].encode()
device_id=os.environ.get("DEVICE_ID","").strip() or None

payload={
  "tenant_id": tenant_id,
  "device_id": device_id,
  "exp": int(time.time()) + 3600,
}
payload_raw=json.dumps(payload,separators=(",",":")).encode()
payload_b64=base64.urlsafe_b64encode(payload_raw).decode().rstrip("=")
sig=hmac.new(secret, payload_b64.encode(), hashlib.sha256).digest()
sig_b64=base64.urlsafe_b64encode(sig).decode().rstrip("=")
print(f"v1.{payload_b64}.{sig_b64}")
PY
)"

CSR_CONTENT="$(cat "${CSR_PEM}")"

if ! command -v grpcurl >/dev/null 2>&1; then
  echo "grpcurl not found. Token and CSR generated, but cannot call Enroll." >&2
  echo "TOKEN=${TOKEN}" >&2
  echo "CSR_PEM=${CSR_PEM}" >&2
  exit 0
fi

echo "Calling Enroll on ${GW_ADDR}"
grpcurl -plaintext \
  -d "$(cat <<EOF
{
  \"enrollment_token\": \"${TOKEN}\",
  \"public_key_pem\": $(python3 - <<PY
import json,sys
print(json.dumps(open("${CSR_PEM}","r",encoding="utf-8").read()))
PY
),
  \"platform\": \"DEVICE_PLATFORM_LINUX\",
  \"attributes\": {\"internal_beta\":\"1\"}
}
EOF
)" \
  "${GW_ADDR}" dmsx.agent.v1.AgentService/Enroll | tee "${OUT_JSON}"

echo "Done. Output: ${OUT_JSON}"

