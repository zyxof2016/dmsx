# 前端架构

## 技术栈

| 层 | 选型 | 说明 |
|----|------|------|
| 框架 | React 19 + TypeScript | 类型安全、生态丰富 |
| 构建 | Vite 6 | 快速 HMR、ESM 原生，`/v1/*` 代理到后端 |
| 路由 | TanStack Router v1 | 类型安全路由；设备详情支持 **`?tab=`** 查询参数（深链、刷新、分享后恢复 Tab） |
| 数据层 | TanStack Query v5 | 缓存、重试、轮询、乐观更新 |
| UI 组件 | Ant Design 5 | 企业级组件、中文本地化 |
| 图表 | Recharts | 轻量、声明式 |
| 日期 | Day.js | 轻量时间格式化 |
| 远程桌面 | LiveKit WebRTC + `livekit-client` | 浏览器直连房间订阅远端视频轨 |
| 远程桌面备用 | RustDesk | 作为跨网络/人工接管的备选方案 |

## 认证与多租户（与 `docs/API.md` 一致）

- 前端将 **活动租户** 持久化在 `localStorage['dmsx.tenant_id']`，所有租户接口通过 `tenantPathFor(tenantId, ...)` 拼出 `/v1/tenants/{tid}/...`。默认值仍为种子租户 `00000000-0000-0000-0000-000000000001`。
- 前端将 JWT 持久化在 `localStorage['dmsx.jwt']`；若存在则自动附带 `Authorization: Bearer <JWT>`，若用户直接粘贴了带 `Bearer ` 前缀的值也会原样接受。
- 顶栏用户菜单支持直接设置 **活动租户** 与 **JWT**，用于 `disabled` / `jwt` 两种模式下的本地联调；不再需要手改源码中的租户常量。
- 生产语义不变：签发方在 JWT 中写入路径租户许可（**`tenant_id` ∪ `allowed_tenant_ids`**）及可选 **`tenant_roles`**（按活动租户覆盖角色）；前端切换租户本质上仍是切换请求路径中的 `{tenant_id}`。

## 目录结构

```
web/
├── index.html
├── package.json
├── vite.config.ts          # 含 /v1 代理到 :8080
├── tsconfig.json
└── src/
    ├── main.tsx             # 入口：QueryClient + AppProviders + Router
    ├── appProviders.tsx     # 主题 / i18n / 会话（tenantId、JWT）上下文
    ├── router.tsx           # TanStack Router 路由树定义
    ├── api/
    │   ├── client.ts        # fetch 封装 + JWT / tenant 本地持久化 + 租户路径工具
    │   ├── types.ts         # 所有 DTO TypeScript 接口
    │   └── hooks.ts         # TanStack Query hooks（设备/策略/命令/影子/远控/桌面等）
    ├── pages/
    │   ├── Dashboard.tsx        # 态势总览
    │   ├── Devices.tsx          # 设备管理列表页
    │   ├── Policies.tsx         # 策略中心列表页
    │   ├── Commands.tsx         # 远程命令列表页
    │   ├── Artifacts.tsx        # 应用分发列表页
    │   ├── Compliance.tsx       # 安全合规页
    │   ├── Network.tsx          # 网络管控页
    │   └── AiCenter.tsx         # AI 智慧中心
    └── components/
        ├── DeviceDetail.tsx     # 设备详情抽屉（Tabs：基本信息/影子/远控/远程桌面）
        ├── ShadowPanel.tsx      # 设备影子面板（三列对比 + JSON 编辑器）
        ├── RemoteControl.tsx    # 远控面板（快捷操作 + 脚本 + 历史 + 结果）
        ├── RemoteDesktop.tsx    # 远程桌面（LiveKit 房间 + Data Channel 输入 + RustDesk 备选）
        ├── PolicyDetail.tsx     # 策略详情抽屉
        └── CommandDetail.tsx    # 命令详情抽屉（payload + 执行结果）
```

## 页面设计

### 态势总览（Dashboard）
- 4 个 KPI 卡片：在线设备 / 合规率 / 待处理命令 / 合规发现
- AI 洞察摘要列表
- 安全态势仪表盘（饼图、柱状图）

### 设备管理（Devices）
- 服务端分页 + 搜索 + 平台/状态筛选
- 批量删除，CSV 导出
- 点击行展开设备详情抽屉（4 个 Tab）

### 设备详情抽屉（DeviceDetail）

路由：`/devices/$deviceId`，可选查询参数 **`tab`**，取值与 Tab `key` 一致：

- `info`：基本信息（默认）
- `shadow`：设备影子
- `remote`：远控面板
- `desktop`：远程桌面

切换 Tab 时使用 `navigate({ search: { tab }, replace: true })` 写回 URL，保证浏览器后退与外链打开时落在正确面板。

**基本信息 Tab**：设备全量字段 Descriptions 展示。

**设备影子 Tab**（ShadowPanel）：
- 三列对比：Reported（Agent 上报） / Desired（期望状态）/ Delta（差异）
- JSON 语法高亮（灰色背景）
- 编辑 Modal：修改 Desired 状态并立即提交

**远控面板 Tab**（RemoteControl）：
- 快捷操作网格：重启、锁屏、关机、收集日志、安装更新等
- 脚本执行器：选解释器、输入脚本、超时设置
- 危险操作（wipe）三重确认：弹框 + 主机名输入 + 二次确认
- 命令历史表格 + 执行结果查看（exit_code / stdout / stderr）

**远程桌面 Tab**（RemoteDesktop）：
- **LiveKit 主链路**：点击「连接」→ POST 创建会话 → 浏览器基于 `token/livekit_url` 入房 → 订阅远端视频轨
- 键鼠控制：前端采集鼠标/键盘事件 → LiveKit Data Channel → Agent 注入输入
- 状态反馈：创建中、等待设备接入、已连接、重连中、已断开、错误
- 当前仓库已在本机验证过最小真实闭环：`dmsx-api` + LiveKit + Redis + 真实 `dmsx-agent` 能完成 `POST /desktop/session`、Agent 入房、发布屏幕轨；坏 LiveKit 地址下 `start_desktop` 会回报失败而不是假成功。
- 删除会话时当前语义是“控制面立即清理会话映射并投递 `stop_desktop`”；Agent 会按创建顺序执行该设备的排队命令，因此 `stop_desktop` 不会抢在对应 `start_desktop` 之前执行，但如果设备已取到先前的 `start_desktop`，仍可能短暂建连后再停止，最终由 `stop_desktop` 收敛。
- **RustDesk 方案（备选）**：显示 RustDesk ID、一键打开 Web Client 或本地客户端

### AI 智慧中心（AiCenter）
四个 Tab：
1. **异常检测**：实时异常列表 + 级别统计 + 一键处置
2. **策略推荐**：AI 生成的策略建议 + 置信度 + 风险评估 + 一键采纳
3. **智能助手**：对话式交互（NL → API 操作）+ 操作按钮
4. **预测维护**：时间线展示预测风险 + 概率 + ETA + 智能处置

### 全局 AI 入口
- 右下角 `FloatButton`（机器人图标）：任意页面一键跳转 AI 中心

## API Hooks 概览

| Hook | 用途 |
|------|------|
| `useDevices` / `useDevice` | 设备列表 / 单设备（10s 轮询） |
| `useCreateDevice` / `useDeleteDevice` | 设备增删 |
| `useShadow` | 设备影子（10s 轮询） |
| `useUpdateShadowDesired` | 更新期望状态 |
| `useDeviceAction` | 下发远控操作 |
| `useDeviceCommands` | 设备命令历史 |
| `useCommandResult` | 命令执行结果 |
| `useCreateDesktopSession` | 创建远程桌面会话 |
| `useDeleteDesktopSession` | 终止远程桌面会话 |
| `useLivekitConfig` | 查询 LiveKit 配置 |
| `useCommands` / `useCreateCommand` | 命令管理 |
| `usePolicies` / `useCreatePolicy` | 策略管理 |
| `useArtifacts` / `useCreateArtifact` | 制品管理 |
| `useFindings` | 合规发现 |
| `useStats` | Dashboard 统计（15s 轮询） |

## 与后端对接

- 开发时 Vite proxy 转发 `/v1/*` 到 `http://127.0.0.1:8080`
- 生产：Nginx/Ingress 统一入口，前端静态资源 + API 反代
- API 客户端（`api/client.ts`）统一处理 Problem Details 错误
- 所有 TypeScript 类型定义在 `api/types.ts`，与后端 DTO 严格对齐
- `tsconfig.json` 使用 `noEmit: true`，避免将编译产物写回 `src/`
- `App.tsx`、系统设置页、审计页等依赖 `AppProviders` 上下文；若调整入口挂载顺序，需保证 `RouterProvider` 仍包在 `AppProviders` 内。
