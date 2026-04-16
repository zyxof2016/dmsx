#!/usr/bin/env bash
# 生产化加码冒烟（HTTP）：面向 OIDC/JWKS（RS256/ES256 等）令牌的最小断言集。
#
# 依赖：curl
#
# 环境变量：
#   DMSX_SMOKE_API                  默认 https://api.example.com（建议走 Ingress 域名）
#   DMSX_SMOKE_TENANT_A             默认 00000000-0000-0000-0000-000000000001
#   DMSX_SMOKE_TENANT_B             默认 22222222-2222-2222-2222-222222222222
#   DMSX_SMOKE_BEARER_VALID_AB      有效 JWT：tenant_id=A，allowed_tenant_ids 包含 B，角色含 TenantAdmin
#   DMSX_SMOKE_BEARER_A_ONLY        仅 A 的有效 JWT：allowed_tenant_ids 不含 B（用于跨租户 403）
#   DMSX_SMOKE_BEARER_BAD_ISS       iss 不匹配但签名有效（预期 401）
#   DMSX_SMOKE_BEARER_BAD_AUD       aud 不匹配但签名有效（预期 401）
#   DMSX_SMOKE_BEARER_PLATFORM      PlatformAdmin 有效 JWT（用于 /v1/config/livekit）
set -euo pipefail

BASE="${DMSX_SMOKE_API:-https://api.example.com}"
TA="${DMSX_SMOKE_TENANT_A:-00000000-0000-0000-0000-000000000001}"
TB="${DMSX_SMOKE_TENANT_B:-22222222-2222-2222-2222-222222222222}"

need() {
  local name="$1"
  if [[ -z "${!name:-}" ]]; then
    echo "缺少环境变量：$name" >&2
    exit 1
  fi
}

need DMSX_SMOKE_BEARER_VALID_AB
need DMSX_SMOKE_BEARER_A_ONLY
need DMSX_SMOKE_BEARER_BAD_ISS
need DMSX_SMOKE_BEARER_BAD_AUD
need DMSX_SMOKE_BEARER_PLATFORM

die() { echo "oidc/jwks prod smoke failed: $*" >&2; exit 1; }

code() {
  local method="$1"
  local url="$2"
  local bearer="${3:-}"
  if [[ -n "$bearer" ]]; then
    curl -sS -o /dev/null -w '%{http_code}' -X "$method" -H "Authorization: Bearer $bearer" "$url"
  else
    curl -sS -o /dev/null -w '%{http_code}' -X "$method" "$url"
  fi
}

echo "== public probes (no auth) =="
[[ "$(code GET "$BASE/health")" == "200" ]] || die "/health should be 200"
[[ "$(code GET "$BASE/ready")" == "200" ]] || die "/ready should be 200"

echo "== protected route requires Authorization =="
[[ "$(code GET "$BASE/v1/tenants/$TA/devices?limit=1")" == "401" ]] || die "missing token should be 401"

echo "== issuer/audience mismatch should be 401 =="
[[ "$(code GET "$BASE/v1/tenants/$TA/devices?limit=1" "$DMSX_SMOKE_BEARER_BAD_ISS")" == "401" ]] || die "bad iss should be 401"
[[ "$(code GET "$BASE/v1/tenants/$TA/devices?limit=1" "$DMSX_SMOKE_BEARER_BAD_AUD")" == "401" ]] || die "bad aud should be 401"

echo "== valid AB token can switch path tenant (A then B) =="
[[ "$(code GET "$BASE/v1/tenants/$TA/devices?limit=1" "$DMSX_SMOKE_BEARER_VALID_AB")" == "200" ]] || die "valid token on tenant A should be 200"
[[ "$(code GET "$BASE/v1/tenants/$TB/devices?limit=1" "$DMSX_SMOKE_BEARER_VALID_AB")" == "200" ]] || die "valid token on tenant B should be 200"

echo "== cross-tenant isolation: A-only token must not access tenant B =="
[[ "$(code GET "$BASE/v1/tenants/$TB/devices?limit=1" "$DMSX_SMOKE_BEARER_A_ONLY")" == "403" ]] || die "A-only token to tenant B should be 403"

echo "== platform admin only route =="
[[ "$(code GET "$BASE/v1/config/livekit" "$DMSX_SMOKE_BEARER_VALID_AB")" == "403" ]] || die "TenantAdmin token should be 403 on /v1/config/livekit"
[[ "$(code GET "$BASE/v1/config/livekit" "$DMSX_SMOKE_BEARER_PLATFORM")" == "200" ]] || die "PlatformAdmin token should be 200 on /v1/config/livekit"

echo ""
echo "OIDC/JWKS 生产化冒烟通过（API=$BASE）。"

