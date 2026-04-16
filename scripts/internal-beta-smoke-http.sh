#!/usr/bin/env bash
# 主链路 HTTP 冒烟：health → 创建设备 → shadow reported → 下发命令 → 设备侧命令列表
# → 模拟 Agent 更新状态与结果（不启动真实 Agent、不触发 reboot/桌面）。
#
# 前置：Postgres 已迁移且默认租户存在；本机已启动 dmsx-api（默认 AUTH_MODE=disabled 最省事）。
# 依赖：curl、python3（用于 JSON，无需 jq）。
# 环境变量：
#   DMSX_SMOKE_API    默认 http://127.0.0.1:8080
#   DMSX_SMOKE_TENANT 默认 00000000-0000-0000-0000-000000000001
#   DMSX_SMOKE_BEARER 若 API 为 jwt 模式，设为 Bearer 令牌（不含前缀 Bearer）
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BASE="${DMSX_SMOKE_API:-http://127.0.0.1:8080}"
TENANT="${DMSX_SMOKE_TENANT:-00000000-0000-0000-0000-000000000001}"
HDR=(-H "Content-Type: application/json")
if [[ -n "${DMSX_SMOKE_BEARER:-}" ]]; then
  HDR+=(-H "Authorization: Bearer ${DMSX_SMOKE_BEARER}")
fi

if ! command -v python3 >/dev/null 2>&1; then
  echo "需要 python3（解析 JSON）。" >&2
  exit 1
fi

die() { echo "smoke failed: $*" >&2; exit 1; }

echo "== GET /health =="
curl -sfS "${HDR[@]}" "$BASE/health" | python3 -c 'import sys,json; j=json.load(sys.stdin); assert j.get("status")=="ok", j' \
  || die "/health 非预期"

STAMP="$(date +%s)-$$"
HOST="smoke-$STAMP"

echo "== POST /v1/tenants/.../devices (注册) =="
CREATE_BODY="$(H="$HOST" python3 -c "import json,os; h=os.environ['H']; print(json.dumps({'platform':'linux','hostname':h,'os_version':'smoke','agent_version':'internal-beta-smoke','labels':{'smoke':True}}))")"
DEV_JSON="$(curl -sfS "${HDR[@]}" -X POST "$BASE/v1/tenants/$TENANT/devices" -d "$CREATE_BODY")"
DEVICE_ID="$(echo "$DEV_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "device_id=$DEVICE_ID"

echo "== PATCH .../shadow/reported (心跳等效之一) =="
curl -sfS "${HDR[@]}" -X PATCH \
  "$BASE/v1/tenants/$TENANT/devices/$DEVICE_ID/shadow/reported" \
  -d "$(S="$STAMP" python3 -c "import json,os; print(json.dumps({'reported':{'smoke':True,'ts':os.environ['S']}}))")" \
  | python3 -c 'import sys,json; json.load(sys.stdin)' >/dev/null

echo "== POST .../commands (下发 smoke 命令) =="
CMD_BODY="$(DID="$DEVICE_ID" python3 -c "import json,os; did=os.environ['DID']; print(json.dumps({'target_device_id':did,'payload':{'action':'smoke_noop','params':{}},'ttl_seconds':3600}))")"
CMD_JSON="$(curl -sfS "${HDR[@]}" -X POST "$BASE/v1/tenants/$TENANT/commands" -d "$CMD_BODY")"
CMD_ID="$(echo "$CMD_JSON" | python3 -c 'import sys,json; print(json.load(sys.stdin)["id"])')"
echo "command_id=$CMD_ID"

echo "== GET .../devices/{id}/commands (轮询等效) =="
LIST="$(curl -sfS "${HDR[@]}" "$BASE/v1/tenants/$TENANT/devices/$DEVICE_ID/commands?limit=10")"
echo "$LIST" | CID="$CMD_ID" python3 -c 'import sys,json,os; cid=os.environ["CID"]; items=json.load(sys.stdin).get("items",[]); ids=[str(x.get("id")) for x in items]; assert str(cid) in ids, (cid, ids)' \
  || die "设备命令列表中未找到新建命令"

echo "== PATCH .../commands/{id}/status (running → succeeded) =="
curl -sfS "${HDR[@]}" -X PATCH \
  "$BASE/v1/tenants/$TENANT/commands/$CMD_ID/status" \
  -d '{"status":"running"}' >/dev/null
curl -sfS "${HDR[@]}" -X PATCH \
  "$BASE/v1/tenants/$TENANT/commands/$CMD_ID/status" \
  -d '{"status":"succeeded"}' >/dev/null

echo "== POST .../commands/{id}/result =="
curl -sfS "${HDR[@]}" -X POST \
  "$BASE/v1/tenants/$TENANT/commands/$CMD_ID/result" \
  -d '{"exit_code":0,"stdout":"internal-beta-smoke-http","stderr":""}' >/dev/null

echo ""
echo "主链路 HTTP 冒烟通过（API=$BASE tenant=$TENANT）。"
