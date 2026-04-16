#!/usr/bin/env bash
# 内测 DoD 一键校验（合并/打标签前跑）
# - 库级基线：dmsx-api --lib + dmsx-agent --lib
# - 主链路 HTTP 冒烟：health → 创建设备 → shadow reported → 下发命令 → 更新状态/结果
#
# 前置（冒烟部分）：
# - 已启动 dmsx-api（默认 http://127.0.0.1:8080）
# - Postgres 已就绪且迁移已应用（由 dmsx-api 启动时 sqlx::migrate! 执行）
# - 本机有 curl、python3
#
# 可选环境变量（透传给 internal-beta-smoke-http.sh）：
#   DMSX_SMOKE_API
#   DMSX_SMOKE_TENANT
#   DMSX_SMOKE_BEARER
#
# 可选跳过：
#   DMSX_DOD_SKIP_SMOKE=1
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

say() { echo "== $* =="; }
die() { echo "dod failed: $*" >&2; exit 1; }

if [[ "${DMSX_DOD_SKIP_SMOKE:-0}" != "1" ]]; then
  command -v curl >/dev/null 2>&1 || die "需要 curl（用于主链路 HTTP 冒烟）"
  command -v python3 >/dev/null 2>&1 || die "需要 python3（用于主链路 HTTP 冒烟 JSON 解析）"
fi

say "库级基线（internal-beta-verify.sh）"
chmod +x scripts/internal-beta-verify.sh >/dev/null 2>&1 || true
./scripts/internal-beta-verify.sh

if [[ "${DMSX_DOD_SKIP_SMOKE:-0}" == "1" ]]; then
  echo ""
  echo "DoD 完成：已跳过主链路 HTTP 冒烟（DMSX_DOD_SKIP_SMOKE=1）。"
  exit 0
fi

BASE="${DMSX_SMOKE_API:-http://127.0.0.1:8080}"
say "检查 dmsx-api 可达（GET /health: $BASE）"
curl -sfS "$BASE/health" >/dev/null || die "无法访问 $BASE/health（请先启动 dmsx-api）"

say "主链路 HTTP 冒烟（internal-beta-smoke-http.sh）"
chmod +x scripts/internal-beta-smoke-http.sh >/dev/null 2>&1 || true
./scripts/internal-beta-smoke-http.sh

echo ""
echo "内测 DoD 一键校验通过。"

