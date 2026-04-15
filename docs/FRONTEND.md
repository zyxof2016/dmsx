# 前端架构

## 技术栈

| 层 | 选型 | 说明 |
|----|------|------|
| 框架 | React 19 + TypeScript | 类型安全、生态丰富 |
| 构建 | Vite 6 | 快速 HMR、ESM 原生，`/v1/*` 代理到后端 |
| 路由 | TanStack Router v1 | 类型安全路由，URL 路径参数，浏览器前进/后退 |
| 数据层 | TanStack Query v5 | 缓存、重试、轮询、乐观更新 |
| UI 组件 | Ant Design 5 | 企业级组件、中文本地化 |
| 图表 | Recharts | 轻量、声明式 |
| 日期 | Day.js | 轻量时间格式化 |
| 远程桌面 | WebSocket + Canvas（JPEG 帧流）| 内置 WebSocket 方案 |
| 远程桌面备用 | livekit-client（已安装，待 WebRTC 迁移）| 官方 LiveKit JS SDK |

## 目录结构

```
web/
├── index.html
├── package.json
├── vite.config.ts          # 含 /v1 代理到 :8080
├── tsconfig.json
└── src/
    ├── main.tsx             # 入口：QueryClient + AntD ConfigProvider + Router
    ├── routes.tsx           # TanStack Router 路由树定义
    ├── api/
    │   ├── client.ts        # fetch 封装 + 租户路径工具 + TENANT_ID
    │   ├── types.ts         # 所有 DTO TypeScript 接口
    │   └── hooks.ts         # TanStack Query hooks（设备/策略/命令/影子/远控/桌面等）
    └── components/
        ├── Dashboard.tsx        # 态势总览（KPI + AI洞察 + 安全态势 + 图表）
        ├── Devices.tsx          # 设备管理（表格 + 服务端筛选 + 批量操作 + CSV）
        ├── DeviceDetail.tsx     # 设备详情抽屉（Tabs：基本信息/影子/远控/远程桌面）
        ├── ShadowPanel.tsx      # 设备影子面板（三列对比 + JSON 编辑器）
        ├── RemoteControl.tsx    # 远控面板（快捷操作 + 脚本 + 历史 + 结果）
        ├── RemoteDesktop.tsx    # 远程桌面（WebSocket 画面 + 键鼠控制 + RustDesk 备选）
        ├── Policies.tsx         # 策略中心（CRUD + 发布）
        ├── PolicyDetail.tsx     # 策略详情抽屉
        ├── Commands.tsx         # 远程命令（下发 + 回执 + 状态筛选）
        ├── CommandDetail.tsx    # 命令详情抽屉（payload + 执行结果）
        ├── Artifacts.tsx        # 应用分发（上传 + SHA256）
        ├── Compliance.tsx       # 安全合规（发现项 + 合规率）
        ├── Network.tsx          # 网络管控（站点 + 隧道）
        └── AiCenter.tsx         # AI 智慧中心（四大功能 Tab）
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
- **WebSocket 方案（主）**：点击「连接」→ POST 创建会话 → 等待 Agent 开始采集 → Canvas 渲染 JPEG 帧
- 键鼠控制：Canvas 捕获事件 → 坐标映射（Canvas 尺寸→远端分辨率）→ WS 发送 JSON
- 工具栏：全屏、Ctrl+Alt+Del、断开连接
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
