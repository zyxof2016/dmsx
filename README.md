# DMSX — Rust 集控 DMS 平台

面向多租户的**设备管理 / 策略 / 命令 / 制品 / 合规 / 远程桌面 / AI 智慧管控**一体化平台。

## 文档

- [架构与服务边界](docs/ARCHITECTURE.md)
- [领域模型与表结构](docs/DOMAIN_MODEL.md)
- [API 契约](docs/API.md)
- [安全设计](docs/SECURITY.md)
- [AI 智慧管控设计](docs/AI_DESIGN.md)
- [前端架构](docs/FRONTEND.md)
- [部署与可观测性](docs/DEPLOYMENT.md)
- [Android 设备接入](docs/ANDROID_DEPLOY.md)
- [容量与路线图](docs/SCALING_AND_ROADMAP.md)
- [功能清单与完成状态](docs/CHECKLIST.md)

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

```bash
docker compose -f deploy/docker-compose.yml up -d
```

包含 Postgres / Redis / NATS / ClickHouse / MinIO / RustDesk / LiveKit / OTel Collector。

### 后端

```bash
# 控制面 HTTP API（含桌面会话、LiveKit token 签发、管理接口）
DATABASE_URL="postgres://dmsx:dmsx@127.0.0.1:5432/dmsx" \
LIVEKIT_URL="ws://127.0.0.1:7880" \
LIVEKIT_API_KEY="dmsx-api-key" \
LIVEKIT_API_SECRET="dmsx-api-secret-that-is-at-least-32-chars" \
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
| `migrations/` | Postgres 初始化 SQL（含 device_shadows / command_results） |
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
