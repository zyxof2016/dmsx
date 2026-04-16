#!/usr/bin/env bash
# 本地「环境可复现」校验：按 README 执行 docker compose，并确认 **主机 127.0.0.1:5432**
# 上存在可用的 Postgres（`dmsx` / `dmsx`），与 `DATABASE_URL=postgres://dmsx:dmsx@127.0.0.1:5432/dmsx` 一致。
#
# 用法：
#   ./scripts/reproduce-dev-env.sh              # 启动 compose 文件中的全部服务（与 README 一致）
#   REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh   # 仅 Postgres（内测主链路最小依赖）
#
# **compose 独占主机 5432**（会先停止当前所有「映射主机 5432」的容器，再 compose up；危险：会停掉
# 任意占用该端口的 DB 容器，仅用于本机开发）：
#   REPRODUCE_TAKE_PORT_5432=1 REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh
#
# 若未设置 REPRODUCE_TAKE_PORT_5432，且 5432 已被占用，compose 可能无法绑定；脚本会检测已有映射
# 5432 的容器若 `pg_isready -U dmsx -d dmsx` 成功即视为依赖满足（不保证来自本 compose）。
#
# 成功后可用（示例）：
#   DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" DMSX_API_BIND="127.0.0.1:8080" cargo run -p dmsx-api
#   DMSX_SMOKE_API="http://127.0.0.1:8080" ./scripts/internal-beta-smoke-http.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

COMPOSE=(docker compose -f "$ROOT/deploy/docker-compose.yml")

if ! command -v docker >/dev/null 2>&1; then
  echo "需要已安装并运行的 Docker（含 compose 插件）。" >&2
  exit 1
fi

stop_publishers_of_5432() {
  local ids id names
  names="$(docker ps --filter publish=5432 --format '{{.Names}}')"
  ids="$(docker ps -q --filter publish=5432)"
  if [[ -z "$ids" ]]; then
    echo "== 无容器映射主机 5432，跳过抢占 =="
    return 0
  fi
  echo "== REPRODUCE_TAKE_PORT_5432：即将停止以下占用主机 5432 的容器 ==" >&2
  echo "$names" >&2
  while read -r id; do
    [[ -z "$id" ]] && continue
    docker stop "$id" >/dev/null
  done <<< "$ids"
  echo "== 已停止，主机 5432 应已释放 ==" >&2
}

pg_on_published_5432() {
  local name
  name="$(docker ps --filter publish=5432 --format '{{.Names}}' | head -n1)"
  [[ -z "$name" ]] && return 1
  docker exec "$name" pg_isready -U dmsx -d dmsx >/dev/null 2>&1
}

wait_host_postgres() {
  local i name
  for i in $(seq 1 90); do
    if pg_on_published_5432; then
      name="$(docker ps --filter publish=5432 --format '{{.Names}}' | head -n1)"
      docker exec "$name" pg_isready -U dmsx -d dmsx
      echo "Postgres 就绪（映射主机 5432 的容器: $name）"
      return 0
    fi
    sleep 1
  done
  return 1
}

if [[ "${REPRODUCE_TAKE_PORT_5432:-0}" == "1" ]]; then
  stop_publishers_of_5432
  docker rm -f deploy-postgres-1 2>/dev/null || true
fi

if [[ "${REPRODUCE_MINIMAL:-0}" == "1" ]]; then
  echo "== docker compose up -d postgres（最小集）=="
else
  echo "== docker compose up -d（全栈依赖，与 README 一致）=="
fi

set +e
if [[ "${REPRODUCE_MINIMAL:-0}" == "1" ]]; then
  "${COMPOSE[@]}" up -d postgres
else
  "${COMPOSE[@]}" up -d
fi
up_rc=$?
set -e

if [[ "$up_rc" != "0" ]]; then
  echo "== compose 返回非 0（常见：5432 仍被占用）；清理可能残留的 deploy-postgres-1 ==" >&2
  docker rm -f deploy-postgres-1 2>/dev/null || true
fi

if wait_host_postgres; then
  echo ""
  echo "环境可复现校验通过：主机 5432 可用（DATABASE_URL=postgres://dmsx:dmsx@127.0.0.1:5432/dmsx）。"
  echo "应用进程请本机 cargo run（见 README）；迁移由 dmsx-api 启动时 sqlx 执行。"
  exit 0
fi

echo "未检测到映射到主机 5432 且用户/库为 dmsx 的 Postgres。" >&2
if [[ "${REPRODUCE_TAKE_PORT_5432:-0}" != "1" ]]; then
  echo "若希望由本仓库 compose 独占 5432，可先：" >&2
  echo "  REPRODUCE_TAKE_PORT_5432=1 REPRODUCE_MINIMAL=1 $0" >&2
fi
echo "或手动: docker compose -f deploy/docker-compose.yml up -d postgres" >&2
echo "或: docker ps --filter publish=5432" >&2
exit 1
