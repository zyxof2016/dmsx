#!/usr/bin/env bash
# 内测数据面最小闭环：
#   创建设备 -> Enroll 签发设备证书 -> FetchDesiredState -> StreamCommands -> ReportResult -> API 查结果
#
# 前置：
# - 已启动 dmsx-api / dmsx-device-gw
# - Postgres 已迁移，NATS JetStream 已就绪，且 API/GW 都配置了同一个 DMSX_NATS_URL
# - GW 已配置 DMSX_GW_ENROLL_HMAC_SECRET + DMSX_GW_ENROLL_CA_CERT/KEY
# - 若要验证 mTLS，建议 GW 开启 TLS（DMSX_GW_TLS_CERT/KEY），本脚本默认按 TLS 跑
#
# 依赖：curl、python3、openssl、grpcurl、timeout
#
# 环境变量：
# - DMSX_E2E_API        默认 http://127.0.0.1:8080
# - DMSX_E2E_TENANT     默认 00000000-0000-0000-0000-000000000001
# - DMSX_E2E_BEARER     可选，API Bearer token（不含 Bearer 前缀）
# - GW_ADDR             默认 127.0.0.1:50051
# - GW_GRPC_MODE        默认 tls（plaintext|tls）
# - GW_TLS_CA_CERT      可选，grpcurl 校验服务端证书链
# - GW_TLS_INSECURE     默认 1；TLS 模式下未提供 CA 时使用 -insecure
# - ENROLL_SECRET       必填；对应 DMSX_GW_ENROLL_HMAC_SECRET
# - STREAM_WAIT_SECS    默认 15
# - RESULT_WAIT_SECS    默认 15
# - KEEP_WORKDIR        默认 0；设为 1 时保留临时目录
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

API="${DMSX_E2E_API:-http://127.0.0.1:8080}"
TENANT="${DMSX_E2E_TENANT:-00000000-0000-0000-0000-000000000001}"
GW_ADDR="${GW_ADDR:-127.0.0.1:50051}"
GW_GRPC_MODE="${GW_GRPC_MODE:-tls}"
GW_TLS_CA_CERT="${GW_TLS_CA_CERT:-}"
GW_TLS_INSECURE="${GW_TLS_INSECURE:-1}"
ENROLL_SECRET="${ENROLL_SECRET:-}"
STREAM_WAIT_SECS="${STREAM_WAIT_SECS:-15}"
RESULT_WAIT_SECS="${RESULT_WAIT_SECS:-15}"
KEEP_WORKDIR="${KEEP_WORKDIR:-0}"

die() { echo "data-plane e2e failed: $*" >&2; exit 1; }

for cmd in curl python3 openssl grpcurl timeout; do
  command -v "$cmd" >/dev/null 2>&1 || die "需要依赖: $cmd"
done

[[ -n "$ENROLL_SECRET" ]] || die "需要设置 ENROLL_SECRET"

HDR=(-H "Content-Type: application/json")
if [[ -n "${DMSX_E2E_BEARER:-}" ]]; then
  HDR+=(-H "Authorization: Bearer ${DMSX_E2E_BEARER}")
fi

WORKDIR="${WORKDIR:-$(mktemp -d "${TMPDIR:-/tmp}/dmsx-data-plane-e2e.XXXXXX")}"
cleanup() {
  if [[ "$KEEP_WORKDIR" != "1" ]]; then
    rm -rf "$WORKDIR"
  else
    echo "保留工作目录: $WORKDIR"
  fi
}
trap cleanup EXIT

grpc_transport_args=()
case "$GW_GRPC_MODE" in
  plaintext)
    grpc_transport_args+=(-plaintext)
    ;;
  tls)
    if [[ -n "$GW_TLS_CA_CERT" ]]; then
      grpc_transport_args+=(-cacert "$GW_TLS_CA_CERT")
    elif [[ "$GW_TLS_INSECURE" == "1" || "$GW_TLS_INSECURE" == "true" ]]; then
      grpc_transport_args+=(-insecure)
    fi
    ;;
  *)
    die "不支持的 GW_GRPC_MODE: $GW_GRPC_MODE（期望 plaintext|tls）"
    ;;
esac

echo "== 检查 API 健康 =="
curl -sfS "${HDR[@]}" "$API/health" | python3 -c 'import sys,json; j=json.load(sys.stdin); assert j.get("status")=="ok", j' \
  || die "API 不可用: $API"

STAMP="$(date +%s)-$$"
HOST="dp-e2e-$STAMP"

echo "== 创建设备 =="
CREATE_BODY="$(H="$HOST" python3 -c "import json,os; print(json.dumps({'platform':'linux','hostname':os.environ['H'],'os_version':'e2e','agent_version':'internal-beta-data-plane-e2e','labels':{'data_plane_e2e':True}}))")"
DEV_JSON="$(curl -sfS "${HDR[@]}" -X POST "$API/v1/tenants/$TENANT/devices" -d "$CREATE_BODY")"
DEVICE_ID="$(echo "$DEV_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "device_id=$DEVICE_ID"

echo "== Enroll（签发设备证书） =="
ENROLL_DIR="$WORKDIR/enroll"
mkdir -p "$ENROLL_DIR"
GW_ADDR="$GW_ADDR" \
GW_GRPC_MODE="$GW_GRPC_MODE" \
GW_TLS_CA_CERT="$GW_TLS_CA_CERT" \
GW_TLS_INSECURE="$GW_TLS_INSECURE" \
TENANT_ID="$TENANT" \
DEVICE_ID="$DEVICE_ID" \
ENROLL_SECRET="$ENROLL_SECRET" \
WORKDIR="$ENROLL_DIR" \
./scripts/internal-beta-data-plane-enroll.sh >"$WORKDIR/enroll.log"

ENROLL_JSON="$ENROLL_DIR/enroll.json"
CLIENT_KEY="$ENROLL_DIR/device.key"
CLIENT_CERT="$WORKDIR/device.crt"
DEVICE_CA_CERT="$WORKDIR/device-ca.crt"

python3 - "$ENROLL_JSON" "$CLIENT_CERT" "$DEVICE_CA_CERT" <<'PY'
import json, sys
src, cert_path, ca_path = sys.argv[1:4]
data = json.load(open(src, encoding="utf-8"))
issued = data.get("issued_cert_pem") or data.get("issuedCertPem")
ca = data.get("ca_cert_pem") or data.get("caCertPem")
if not issued or not ca:
    raise SystemExit("enroll 响应缺少证书字段")
open(cert_path, "w", encoding="utf-8").write(issued)
open(ca_path, "w", encoding="utf-8").write(ca)
PY

grpc_mtls_args=("${grpc_transport_args[@]}")
if [[ "$GW_GRPC_MODE" == "tls" ]]; then
  grpc_mtls_args+=(-cert "$CLIENT_CERT" -key "$CLIENT_KEY")
fi

echo "== FetchDesiredState（验证已签发身份可访问数据面 RPC） =="
grpcurl "${grpc_mtls_args[@]}" \
  -import-path "$ROOT/proto" \
  -proto dmsx/agent.proto \
  -d "$(python3 - <<PY
import json
print(json.dumps({"device_id":"$DEVICE_ID","last_policy_revision_id":""}))
PY
)" \
  "$GW_ADDR" dmsx.agent.v1.AgentService/FetchDesiredState >"$WORKDIR/fetch_desired_state.json"

echo "== 启动 StreamCommands =="
STREAM_OUT="$WORKDIR/stream.out"
STREAM_ERR="$WORKDIR/stream.err"
timeout "${STREAM_WAIT_SECS}s" \
  grpcurl "${grpc_mtls_args[@]}" \
    -import-path "$ROOT/proto" \
    -proto dmsx/agent.proto \
    -d "$(python3 - <<PY
import json
print(json.dumps({"device_id":"$DEVICE_ID","tenant_id":"$TENANT","cursor":""}))
PY
)" \
    "$GW_ADDR" dmsx.agent.v1.AgentService/StreamCommands >"$STREAM_OUT" 2>"$STREAM_ERR" &
STREAM_PID=$!
sleep 1

echo "== 通过 API 下发命令 =="
CMD_BODY="$(DID="$DEVICE_ID" python3 -c "import json,os; print(json.dumps({'target_device_id':os.environ['DID'],'payload':{'action':'smoke_noop','params':{'source':'internal-beta-data-plane-e2e'}},'ttl_seconds':3600}))")"
CMD_JSON="$(curl -sfS "${HDR[@]}" -X POST "$API/v1/tenants/$TENANT/commands" -d "$CMD_BODY")"
CMD_ID="$(echo "$CMD_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "command_id=$CMD_ID"

STREAM_CMD_ID=""
for _ in $(seq 1 "$STREAM_WAIT_SECS"); do
  STREAM_CMD_ID="$(python3 - "$STREAM_OUT" <<'PY'
import json, sys
from json import JSONDecoder
path = sys.argv[1]
try:
    text = open(path, encoding="utf-8").read()
except FileNotFoundError:
    print("")
    raise SystemExit(0)
dec = JSONDecoder()
i = 0
while i < len(text):
    while i < len(text) and text[i].isspace():
        i += 1
    if i >= len(text):
        break
    try:
        obj, j = dec.raw_decode(text, i)
    except json.JSONDecodeError:
        i += 1
        continue
    cid = obj.get("command_id") or obj.get("commandId")
    if cid:
        print(cid)
        raise SystemExit(0)
    i = j
print("")
PY
)"
  if [[ -n "$STREAM_CMD_ID" ]]; then
    break
  fi
  sleep 1
done

kill "$STREAM_PID" >/dev/null 2>&1 || true
wait "$STREAM_PID" || true

[[ -n "$STREAM_CMD_ID" ]] || die "StreamCommands 未在 ${STREAM_WAIT_SECS}s 内收到命令（详见 $STREAM_ERR）"
[[ "$STREAM_CMD_ID" == "$CMD_ID" ]] || die "StreamCommands 收到的 command_id 与 API 不一致: $STREAM_CMD_ID != $CMD_ID"

echo "== ReportResult（回执 JetStream） =="
REPORT_JSON="$WORKDIR/report_result.json"
grpcurl "${grpc_mtls_args[@]}" \
  -import-path "$ROOT/proto" \
  -proto dmsx/agent.proto \
  -d "$(python3 - <<PY
import json
print(json.dumps({
  "device_id":"$DEVICE_ID",
  "tenant_id":"$TENANT",
  "command_id":"$CMD_ID",
  "status":"COMMAND_STATUS_SUCCEEDED",
  "exit_code":0,
  "stdout_snippet":"internal-beta-data-plane-e2e",
  "stderr_snippet":"",
  "evidence_object_key":""
}))
PY
)" \
  "$GW_ADDR" dmsx.agent.v1.AgentService/ReportResult >"$REPORT_JSON"

python3 - "$REPORT_JSON" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
if data.get("accepted") is not True:
    raise SystemExit(f"ReportResult 未被接受: {data}")
PY

echo "== 轮询控制面命令状态与结果 =="
STATUS=""
for _ in $(seq 1 "$RESULT_WAIT_SECS"); do
  STATUS_JSON="$(curl -sfS "${HDR[@]}" "$API/v1/tenants/$TENANT/commands/$CMD_ID")"
  STATUS="$(echo "$STATUS_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("status",""))')"
  if [[ "$STATUS" == "succeeded" ]]; then
    break
  fi
  sleep 1
done
[[ "$STATUS" == "succeeded" ]] || die "命令状态未在 ${RESULT_WAIT_SECS}s 内变为 succeeded，实际: $STATUS"

RESULT_CODE=""
RESULT_STDOUT=""
for _ in $(seq 1 "$RESULT_WAIT_SECS"); do
  http_code="$(curl -sS -o "$WORKDIR/result.json" -w "%{http_code}" "${HDR[@]}" "$API/v1/tenants/$TENANT/commands/$CMD_ID/result" || true)"
  if [[ "$http_code" == "200" ]]; then
    RESULT_CODE="$(python3 - "$WORKDIR/result.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
value = data.get("exit_code")
print("" if value is None else value)
PY
)"
    RESULT_STDOUT="$(python3 - "$WORKDIR/result.json" <<'PY'
import json, sys
data = json.load(open(sys.argv[1], encoding="utf-8"))
print(data.get("stdout",""))
PY
)"
    if [[ "$RESULT_CODE" == "0" && "$RESULT_STDOUT" == *"internal-beta-data-plane-e2e"* ]]; then
      break
    fi
  fi
  sleep 1
done

[[ "$RESULT_CODE" == "0" ]] || die "结果 exit_code 非 0：$RESULT_CODE"
[[ "$RESULT_STDOUT" == *"internal-beta-data-plane-e2e"* ]] || die "结果 stdout 未命中标记"

echo ""
echo "数据面最小闭环通过（api=$API gw=$GW_ADDR tenant=$TENANT device=$DEVICE_ID command=$CMD_ID mode=$GW_GRPC_MODE）。"
