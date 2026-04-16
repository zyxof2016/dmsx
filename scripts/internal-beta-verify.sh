#!/usr/bin/env bash
# 内测自动化基线：控制面 + Agent 库级测试（无需 Docker）。
# 用法：在项目根目录执行  ./scripts/internal-beta-verify.sh
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

echo "== dmsx-api (lib) =="
cargo test -p dmsx-api --lib

echo "== dmsx-agent (lib) =="
cargo test -p dmsx-agent --lib

echo ""
echo "内测库级基线通过。主链路 HTTP 冒烟: scripts/internal-beta-smoke-http.sh（需 curl + python3 + 已启动 dmsx-api）。"
echo "手工 DoD 见 docs/CHECKLIST.md「内测阶段目标与完成定义」。"
