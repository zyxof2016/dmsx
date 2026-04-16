#!/usr/bin/env bash
# 真实 Agent 与 dmsx-api 的最小闭环（开发机）：设备已由脚本预置（与 Agent 同 hostname）→
# 下发 smoke_noop → 启动 Agent → 轮询命令至 succeeded。
#
# 前置：Postgres + 已启动 dmsx-api（建议 DMSX_API_AUTH_MODE=disabled）；本机可 `cargo run -p dmsx-agent`。
# 依赖：curl、python3、timeout（coreutils）。
#
# 环境变量：
#   DMSX_E2E_API       默认 http://127.0.0.1:8080
#   DMSX_E2E_TENANT    默认 00000000-0000-0000-0000-000000000001
#   DMSX_E2E_AGENT_SEC 默认 35（Agent 运行秒数上限）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

API="${DMSX_E2E_API:-http://127.0.0.1:8080}"
TENANT="${DMSX_E2E_TENANT:-00000000-0000-0000-0000-000000000001}"
AGENT_SEC="${DMSX_E2E_AGENT_SEC:-35}"

die() { echo "agent e2e failed: $*" >&2; exit 1; }

command -v python3 >/dev/null 2>&1 || die "需要 python3"
curl -sfS "$API/health" | python3 -c 'import sys,json; j=json.load(sys.stdin); assert j.get("status")=="ok", j' \
  || die "API 不可用: $API"

HOST="$(hostname)"

echo "== 预置设备（hostname 与 Agent 一致: $HOST）=="
CREATE_BODY="$(H="$HOST" python3 -c "import json,os; print(json.dumps({'platform':'linux','hostname':os.environ['H'],'os_version':'e2e','agent_version':'script-seed','labels':{'e2e_agent_script':True}}))")"
DEV_JSON="$(curl -sfS -H "Content-Type: application/json" -X POST "$API/v1/tenants/$TENANT/devices" -d "$CREATE_BODY")"
DEVICE_ID="$(echo "$DEV_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "device_id=$DEVICE_ID"

echo "== 下发 smoke_noop 命令 =="
CMD_BODY="$(DID="$DEVICE_ID" python3 -c "import json,os; print(json.dumps({'target_device_id':os.environ['DID'],'payload':{'action':'smoke_noop','params':{}},'ttl_seconds':3600}))")"
CMD_JSON="$(curl -sfS -H "Content-Type: application/json" -X POST "$API/v1/tenants/$TENANT/commands" -d "$CMD_BODY")"
CMD_ID="$(echo "$CMD_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "command_id=$CMD_ID"

echo "== 启动 Agent（最长 ${AGENT_SEC}s）=="
export DMSX_API_URL="$API"
export DMSX_TENANT_ID="$TENANT"
export DMSX_HEARTBEAT_SECS="${DMSX_HEARTBEAT_SECS:-60}"
export DMSX_POLL_SECS="${DMSX_POLL_SECS:-2}"
set +e
timeout "${AGENT_SEC}s" cargo run -p dmsx-agent 2>&1 | tail -40
agent_rc=$?
set -e
echo "(agent 结束码: $agent_rc，124 多为 timeout 正常)"

echo "== 校验命令状态 =="
STATUS_JSON="$(curl -sfS "$API/v1/tenants/$TENANT/commands/$CMD_ID")"
STATUS="$(echo "$STATUS_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin).get("status",""))')"
[[ "$STATUS" == "succeeded" ]] || die "期望命令 status=succeeded，实际: $STATUS"

echo ""
echo "Agent 开发 E2E 通过（API=$API tenant=$TENANT command=$CMD_ID）。"
