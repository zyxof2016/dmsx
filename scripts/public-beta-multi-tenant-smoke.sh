#!/usr/bin/env bash
# 多租户公测冒烟（HTTP）：JWT 模式下双租户主链路 + 跨租户隔离（403）。
#
# 前置：
#   - Postgres 已迁移（含 migrations/004_second_tenant_seed.sql，由 dmsx-api 启动时自动 migrate）
#   - 本机已启动 dmsx-api，且 **DMSX_API_AUTH_MODE=jwt**
#   - API 与脚本使用 **同一** DMSX_API_JWT_SECRET（未设置时 API 使用开发回退密钥，脚本须与之显式对齐）
# 依赖：curl、python3（stdlib 签发 HS256 JWT，无需 PyJWT）。
#
# 环境变量：
#   DMSX_SMOKE_API              默认 http://127.0.0.1:8080
#   DMSX_API_JWT_SECRET         与运行中的 dmsx-api 一致（建议公测/CI 显式设置）
#   DMSX_PUBLIC_BETA_TENANT_A   默认 00000000-0000-0000-0000-000000000001
#   DMSX_PUBLIC_BETA_TENANT_B   默认 22222222-2222-2222-2222-222222222222
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

BASE="${DMSX_SMOKE_API:-http://127.0.0.1:8080}"
export DMSX_API_JWT_SECRET="${DMSX_API_JWT_SECRET:-dmsx-dev-jwt-secret-change-me-please}"
export DMSX_PUBLIC_BETA_TENANT_A="${DMSX_PUBLIC_BETA_TENANT_A:-00000000-0000-0000-0000-000000000001}"
export DMSX_PUBLIC_BETA_TENANT_B="${DMSX_PUBLIC_BETA_TENANT_B:-22222222-2222-2222-2222-222222222222}"

die() { echo "public-beta smoke failed: $*" >&2; exit 1; }

if ! command -v python3 >/dev/null 2>&1; then
  echo "需要 python3。" >&2
  exit 1
fi

mint_ab() {
  python3 <<'PY'
import base64, hashlib, hmac, json, os, time

def b64u(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")

def sign(secret: str, claims: dict) -> str:
    now = int(time.time())
    claims.setdefault("sub", "public-beta-smoke")
    claims.setdefault("iat", now)
    claims.setdefault("exp", now + 7200)
    header = {"alg": "HS256", "typ": "JWT"}
    segments = [
        b64u(json.dumps(header, separators=(",", ":")).encode()),
        b64u(json.dumps(claims, separators=(",", ":")).encode()),
    ]
    msg = ".".join(segments).encode("ascii")
    sig = hmac.new(secret.encode("utf-8"), msg, hashlib.sha256).digest()
    return ".".join(segments + [b64u(sig)])

secret = os.environ["DMSX_API_JWT_SECRET"]
a = os.environ["DMSX_PUBLIC_BETA_TENANT_A"]
b = os.environ["DMSX_PUBLIC_BETA_TENANT_B"]
claims = {
    "sub": "public-beta-smoke-ab",
    "tenant_id": a,
    "allowed_tenant_ids": [b],
    "roles": ["TenantAdmin"],
}
print(sign(secret, claims))
PY
}

mint_a_only() {
  python3 <<'PY'
import base64, hashlib, hmac, json, os, time

def b64u(raw: bytes) -> str:
    return base64.urlsafe_b64encode(raw).decode("ascii").rstrip("=")

def sign(secret: str, claims: dict) -> str:
    now = int(time.time())
    claims.setdefault("sub", "public-beta-smoke")
    claims.setdefault("iat", now)
    claims.setdefault("exp", now + 7200)
    header = {"alg": "HS256", "typ": "JWT"}
    segments = [
        b64u(json.dumps(header, separators=(",", ":")).encode()),
        b64u(json.dumps(claims, separators=(",", ":")).encode()),
    ]
    msg = ".".join(segments).encode("ascii")
    sig = hmac.new(secret.encode("utf-8"), msg, hashlib.sha256).digest()
    return ".".join(segments + [b64u(sig)])

secret = os.environ["DMSX_API_JWT_SECRET"]
a = os.environ["DMSX_PUBLIC_BETA_TENANT_A"]
claims = {"sub": "public-beta-smoke-a", "tenant_id": a, "roles": ["TenantAdmin"]}
print(sign(secret, claims))
PY
}

echo "== mint JWT（双租户：tenant_id=A，allowed 含 B，TenantAdmin）=="
TOKEN_AB="$(mint_ab)"

echo "== mint JWT（单租户：仅 A）=="
TOKEN_A="$(mint_a_only)"

run_smoke() {
  local tenant="$1"
  local bearer="$2"
  DMSX_SMOKE_API="$BASE" DMSX_SMOKE_TENANT="$tenant" DMSX_SMOKE_BEARER="$bearer" \
    "$ROOT/scripts/internal-beta-smoke-http.sh"
}

echo "== 租户 A 主链路（双租户令牌）=="
run_smoke "$DMSX_PUBLIC_BETA_TENANT_A" "$TOKEN_AB"

echo "== 租户 B 主链路（同一令牌切换路径租户）=="
run_smoke "$DMSX_PUBLIC_BETA_TENANT_B" "$TOKEN_AB"

echo "== 跨租户隔离：仅 A 的令牌访问 B 的 devices 列表须 403 =="
code="$(curl -sS -o /dev/null -w '%{http_code}' \
  -H "Authorization: Bearer ${TOKEN_A}" \
  "$BASE/v1/tenants/${DMSX_PUBLIC_BETA_TENANT_B}/devices?limit=1")"
[[ "$code" == "403" ]] || die "期望 HTTP 403，实际 $code"

echo ""
echo "多租户公测 HTTP 冒烟通过（API=$BASE；租户 A=$DMSX_PUBLIC_BETA_TENANT_A B=$DMSX_PUBLIC_BETA_TENANT_B；jwt 双租户 + 403 隔离）。"
