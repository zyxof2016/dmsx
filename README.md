# DMSX — Rust 集控 DMS 平台

面向多租户的**设备管理 / 策略 / 命令 / 制品 / 合规 / 远程桌面 / AI 智慧管控**一体化平台。

## 文档

- [架构与服务边界](docs/ARCHITECTURE.md)
- [领域模型与表结构](docs/DOMAIN_MODEL.md)
- [API 契约](docs/API.md)（JWT：`tenant_id` / `allowed_tenant_ids` / `tenant_roles` 与 RBAC 约定）
- [安全设计](docs/SECURITY.md)
- [AI 智慧管控设计](docs/AI_DESIGN.md)
- [前端架构](docs/FRONTEND.md)
- [部署与可观测性](docs/DEPLOYMENT.md)
- [Android 设备接入](docs/ANDROID_DEPLOY.md)
- [容量与路线图](docs/SCALING_AND_ROADMAP.md)
- [功能清单与完成状态](docs/CHECKLIST.md)（含 **内测 / 多租户公测 DoD**）

## 快速开始

### 系统依赖

**Linux / WSL**（基础）：

```bash
sudo apt install -y build-essential pkg-config libssl-dev protobuf-compiler
```

**Linux / WSL**（Agent 远程桌面屏幕采集，需要 X11 库）：

```bash
sudo apt install -y libxcb1-dev libxcb-shm0-dev libxcb-randr0-dev libxdo-dev
```

### 依赖服务（一键拉起）

在项目根目录执行：

```bash
docker compose -f deploy/docker-compose.yml up -d
```

包含 Postgres / Redis / NATS / ClickHouse / MinIO / RustDesk / LiveKit / OTel Collector。

**环境可复现校验**（确认主机 **5432** 上 `dmsx`/`dmsx` 可连）：

```bash
chmod +x scripts/reproduce-dev-env.sh   # 首次
./scripts/reproduce-dev-env.sh          # 全栈
# 或仅数据库：
REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh
```

**由本仓库 compose 独占主机 5432**（会先 `docker stop` 掉当前所有映射 **5432** 的容器，再启动 compose 里的 Postgres；会中断其它占用该端口的库，**仅建议本机开发**使用）：

```bash
REPRODUCE_TAKE_PORT_5432=1 REPRODUCE_MINIMAL=1 ./scripts/reproduce-dev-env.sh
# 全栈同理：
# REPRODUCE_TAKE_PORT_5432=1 ./scripts/reproduce-dev-env.sh
```

若未加 `REPRODUCE_TAKE_PORT_5432=1` 且 5432 已被占用，compose 可能无法绑定；脚本仍会检测已有映射 **5432** 的容器是否在 `pg_isready` 意义上满足依赖（不保证来自本 compose）。

### 内测基线（可选）

**库级回归**（无需起 Docker，仅需 Rust 工具链）：

```bash
chmod +x scripts/internal-beta-verify.sh   # 首次
./scripts/internal-beta-verify.sh
```

**Postgres RLS 会话绑定**（可选；需已应用迁移的 `DATABASE_URL` 指向的库）：

```bash
DMSX_TEST_DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" cargo test -p dmsx-api --test rls_tenant_session
```

**主链路 HTTP 冒烟**（需 **curl**、**python3**；`docker compose` 已起 Postgres 且本机已跑 `dmsx-api`；默认 `DMSX_API_AUTH_MODE=disabled`；若启用 `jwt` 请设置 `DMSX_SMOKE_BEARER`）：

```bash
chmod +x scripts/internal-beta-smoke-http.sh   # 首次
./scripts/internal-beta-smoke-http.sh
```

结果与内测 DoD 其余项见 [`docs/CHECKLIST.md`](docs/CHECKLIST.md) 开篇「内测阶段目标与完成定义」。

**真实 Agent 最小 E2E**（需本机已起 `dmsx-api` + Postgres；脚本预置与本机 `hostname` 一致的设备后下发 `smoke_noop`，再 `cargo run` Agent 若干秒并断言命令 **succeeded**）：

```bash
chmod +x scripts/agent-dev-e2e.sh   # 首次
DMSX_E2E_API="http://127.0.0.1:8080" ./scripts/agent-dev-e2e.sh
```

### 多租户公测门禁（`jwt` + 双租户，可选）

面向 **`DMSX_API_AUTH_MODE=jwt`**：数据库须已应用迁移至含 **`004_second_tenant_seed.sql`**（及之后的 **`006_commands_hash_partition.sql`** 等，由 `dmsx-api` 启动时 `sqlx::migrate!` 执行；**新增/改过 `migrations/*.sql` 后请重新 `cargo build -p dmsx-api` 再启动**，否则新 SQL 不会进库）。

```bash
chmod +x scripts/public-beta-multi-tenant-smoke.sh   # 首次
# 终端 1：与脚本共用同一 DMSX_API_JWT_SECRET（示例为开发回退密钥）
DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" \
  DMSX_API_AUTH_MODE=jwt \
  DMSX_API_JWT_SECRET="dmsx-dev-jwt-secret-change-me-please" \
  DMSX_API_BIND="127.0.0.1:8080" \
  cargo run -p dmsx-api

# 终端 2
DMSX_SMOKE_API="http://127.0.0.1:8080" \
  DMSX_API_JWT_SECRET="dmsx-dev-jwt-secret-change-me-please" \
  ./scripts/public-beta-multi-tenant-smoke.sh
```

验收口径与记录见 [`docs/CHECKLIST.md`](docs/CHECKLIST.md)「多租户公测」专节。

### Kubernetes（可选，模板清单）

仓库内提供 `deploy/kubernetes/` 的示例清单（用于参考/快速试跑；生产建议迁移到你的 GitOps 仓库，并用 External Secrets 管理敏感配置）：

```bash
kubectl apply -f deploy/kubernetes/namespace-dmsx.yaml
kubectl -n dmsx apply -f deploy/kubernetes/dmsx-api-secrets.example.yaml   # 先替换内容
kubectl apply -f deploy/kubernetes/dmsx-api.yaml
kubectl apply -f deploy/kubernetes/dmsx-api-networkpolicy.yaml            # 可选
kubectl apply -f deploy/kubernetes/dmsx-api-ingress.yaml                  # 可选：替换域名与证书 Secret
```

### 后端

```bash
# 控制面 HTTP API（含桌面会话、LiveKit token 签发、管理接口）
DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" \
LIVEKIT_URL="ws://127.0.0.1:7880" \
LIVEKIT_API_KEY="dmsx-api-key" \
LIVEKIT_API_SECRET="dmsx-api-secret-that-is-at-least-32-chars" \
DMSX_API_REQUEST_BODY_LIMIT_BYTES="1048576" \
DMSX_API_RATE_LIMIT_ENABLED="false" \
DMSX_API_RATE_LIMIT_PER_SECOND="50" \
DMSX_API_RATE_LIMIT_BURST="100" \
cargo run -p dmsx-api

# 数据面 gRPC 网关（当前为演进骨架，默认可不启动）
cargo run -p dmsx-device-gw
```

### 设备代理（Agent）

```bash
# 在被管设备上运行（默认连接本机 :8080）
DMSX_API_URL="http://YOUR_SERVER_IP:8080" cargo run -p dmsx-agent

# 启用 RustDesk 自建中继（可选）
DMSX_API_URL="http://YOUR_SERVER_IP:8080" \
DMSX_RUSTDESK_RELAY="YOUR_SERVER_IP" \
cargo run -p dmsx-agent
```

Agent 会自动：注册设备 → 定期发送心跳（含系统遥测）→ 轮询并执行命令 → 收到 `start_desktop` 命令后加入 LiveKit 房间、发布屏幕轨并通过 Data Channel 接收输入事件。

### 前端

```bash
cd web && npm install && npm run dev  # http://localhost:3000
```

---

## 目录结构

| 路径 | 说明 |
|------|------|
| `crates/dmsx-core` | 共享领域类型、统一错误（Problem Details） |
| `crates/dmsx-ai` | AI 智慧管控引擎（异常检测、策略推荐、LLM 助手、预测维护） |
| `crates/dmsx-api` | 控制面 ApiGateway（Axum）— 会话管理、REST API、AI API、LiveKit token 签发 |
| `crates/dmsx-device-gw` | 数据面 DeviceGateway（Tonic gRPC + Health）— 当前为后续长连接数据面的演进骨架 |
| `crates/dmsx-agent` | 跨平台设备代理（Windows / Linux / Android）— 遥测 + 命令 + 远程桌面 |
| `web/` | 管理台前端（React + TypeScript + Ant Design + TanStack Router） |
| `proto/` | gRPC Proto 定义（Agent + Health） |
| `migrations/` | Postgres 迁移 SQL（由 **`dmsx-api` 启动时 sqlx 执行**；compose 内 Postgres **不再**挂载到 initdb，避免与 sqlx 重复建表） |
| `migrations/ch/` | ClickHouse 初始化 SQL |
| `openapi/` | OpenAPI 3.1 契约 |
| `deploy/` | Docker Compose（含 LiveKit / RustDesk）、K8s、Dockerfile、OTel |
| `docs/` | 设计文档、部署指南、功能清单 |
| `.github/workflows/` | CI（clippy + test + Docker build） |

## 主要功能

| 功能 | 状态 | 说明 |
|------|------|------|
| 设备注册与管理 | ✅ | 多租户、CRUD、在线状态追踪 |
| 系统遥测 | ✅ | CPU / 内存 / 磁盘 / 进程信息，定期心跳上报 |
| 设备影子 | ✅ | Reported / Desired 双态，Delta 计算，版本乐观并发 |
| 远程控制 | ✅ | 重启/关机/锁屏/执行脚本/收集日志，命令结果回报 |
| 远程桌面 | ✅ | LiveKit WebRTC 主链路 + Data Channel 键鼠控制，RustDesk 备选 |
| 策略管理 | ✅ | 声明式策略，版本发布，灰度发布元数据 |
| 命令管理 | ✅ | 幂等键去重，TTL，状态机，执行结果全文存储 |
| 制品分发 | ✅ | SHA256 校验，渠道（channel），对象存储集成 |
| 合规扫描 | ✅ | 发现项记录，严重度，合规率统计 |
| AI 智慧管控 | 🔨 | 异常检测/策略推荐/智能助手/预测维护（框架完整，LLM 待接入） |
| Android 接入 | ✅ | Termux 运行、NDK 交叉编译指南、原生 App 方案 |

## 许可证

MIT OR Apache-2.0
