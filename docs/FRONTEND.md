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

- 管理台信息架构遵循 [`IOT_PLATFORM_BEST_PRACTICES.md`](IOT_PLATFORM_BEST_PRACTICES.md) 与 [`plans/2026-04-30-management-console-v2-redesign.md`](plans/2026-04-30-management-console-v2-redesign.md)：先认证再选择平台/租户工作范围；平台面承载全局治理，租户面承载设备生命周期；权限、配额、设备运营、策略安全、交付配置按对象和职责拆开。
- 前端将 **活动租户** 持久化在 `localStorage['dmsx.tenant_id']`，所有租户接口通过 `tenantPathFor(tenantId, ...)` 拼出 `/v1/tenants/{tid}/...`。默认值仍为种子租户 `00000000-0000-0000-0000-000000000001`。
- 前端将 JWT 持久化在 `localStorage['dmsx.jwt']`；若存在则自动附带 `Authorization: Bearer <JWT>`，若用户直接粘贴了带 `Bearer ` 前缀的值也会原样接受。
- 前端额外持久化 `localStorage['dmsx.last_tenant_id']` 与 `localStorage['dmsx.display_name']`：登录成功后会记录本次进入的租户和显示名；当账号同时具备多个租户权限时，租户选择页默认优先回显“上次退出租户”，若当前账号已不再拥有该租户权限，则回退到后端返回的 `preferred_tenant_id`。
- 前端额外将认证模式持久化在 `localStorage['dmsx.auth_mode']`：`jwt` 模式下若没有本地 JWT，会直接跳转到 `/login?redirect=...`；`disabled` 模式则允许本地联调直接进入控制台。
- `/login` 已从“手填 JWT / 手填租户”改为真实账号密码登录页：第一步只做账号密码认证；认证通过后展示工作台选择，用户再根据账号拥有的平台权限和租户权限选择进入平台管理或某个租户管理。即使账号仅有平台权限或仅有一个租户权限，也会先经过登录后的确认入口，再调用 `/v1/auth/login/select` 签发正式 JWT。
- 进入系统后，顶部“平台 / 租户”工作模式切换只会在当前登录账号同时具备平台和租户两类权限时展示；若当前账号仅有其中一类权限，则不再显示切换入口，也不会暴露另一类模式的空入口。
- 顶栏“退出登录”现在会先调用后端 `POST /v1/auth/logout` 记录当前活动租户，再清理本地 JWT 和显示名；下次登录时优先默认选中上次退出时的租户。
- 顶栏用户菜单支持直接设置 **活动租户** 与 **JWT**，用于 `disabled` / `jwt` 两种模式下的本地联调；不再需要手改源码中的租户常量。
- 活动租户切换现在不再要求用户手填 UUID，而是基于当前会话可见的租户候选下拉选择：优先使用 JWT 中的 `tenant_id ∪ allowed_tenant_ids`，并合并当前浏览器最近创建过的租户记录，降低误填和越权切换风险。
- 租户下拉项现在会尽量展示“租户名称 + UUID + 来源 + 切过去后的有效角色”。当某个租户来自本浏览器最近创建记录时，会显示其创建时的名称；否则回退为 UUID 缩写。角色标签基于当前 JWT 的 `tenant_roles` / `roles` 计算，便于用户在切换前理解权限变化。
- 生产语义不变：签发方在 JWT 中写入路径租户许可（**`tenant_id` ∪ `allowed_tenant_ids`**）及可选 **`tenant_roles`**（按活动租户覆盖角色）；前端切换租户本质上仍是切换请求路径中的 `{tenant_id}`。
- 前端显式区分 **平台管理模式** 与 **租户管理模式**：平台模式仅展示全局配置类入口（如系统设置、RBAC 角色），租户模式展示 `/v1/tenants/{tenant_id}/...` 资源页。模式切换仅影响导航和默认落点，后端 JWT/RBAC 仍是唯一权限裁决者。
- JWT 模式下，非登录接口返回 401 时，前端会触发全局认证过期事件并清理本地 JWT / 显示名 / 可见 scope，随后回到未登录态，避免继续使用过期会话。
- 平台模式现在有独立首页 `/platform`，用于承载平台级摘要和后续扩展入口，不再默认直接落到“系统设置”。
- 平台模式已按职责拆成独立模块：`/platform` 平台总览、`/platform/permissions` 权限总览、`/platform/permissions/roles` 角色管理、`/platform/permissions/users` 用户管理、`/platform/permissions/menus` 菜单管理、`/platform/permissions/policy` 权限策略、`/platform/tenants` 租户管理、`/platform/quotas` 配额治理、`/platform/audit` 全局审计、`/platform/health` 运行健康。平台首页只承载摘要和模块导航，不再混放具体维护表单。
- 平台页当前接入的真实平台接口包括：`GET /v1/config/rbac/roles`、`GET /v1/config/livekit`、`GET /v1/config/settings/{key}`、`POST /v1/tenants`、`GET /v1/config/tenants`、`GET /v1/config/audit-logs`、`GET /v1/config/platform-health`、`GET /v1/config/quotas`。
- 租户创建入口已收敛到平台“租户管理”模块：用户可输入租户名称并直接调用 `POST /v1/tenants`，创建成功后刷新租户目录。
- 当前前端会从 JWT **本地解析** `tenant_id`、`allowed_tenant_ids`、`roles`、`tenant_roles` 以收敛导航：平台模式由令牌级 `roles` 控制（当前支持 `PlatformAdmin`、`PlatformViewer`）；租户模式按活动租户的 `tenant_roles` 计算，若缺失该租户键，仅兼容令牌级 `roles` 中的非平台角色。`PlatformAdmin` / `PlatformViewer` 不会被当作租户角色。若当前 JWT 不具备平台级角色，则平台模式入口会被禁用并自动回落到租户模式；若当前活动租户不在 JWT 许可集合内，前端会回退到 JWT 主租户。
- 平台级按钮态和导航判断只使用令牌级 `roles`（`PlatformAdmin` / `PlatformViewer`），不会把当前租户的 `tenant_roles`、租户绑定角色或租户自定义角色当作平台权限。
- 租户模式下的“用户 / 角色管理”页已接入当前租户自定义角色管理：前端会读取 `GET /v1/tenants/{tid}/rbac/roles`，支持编辑角色名、说明和权限集合，并通过 `PUT /v1/tenants/{tid}/rbac/roles` 保存。保存后这些角色不仅影响前端按钮态，也会进入后端真实 RBAC 判定。
- 同一页面还接入了租户内“用户(subject) -> 角色”绑定管理：前端会读写 `GET/PUT /v1/tenants/{tid}/rbac/bindings`，并通过 `GET /v1/tenants/{tid}/rbac/me` 获取当前 JWT `sub` 在该租户的最终生效角色，使页面按钮态尽量以后端真实授权结果为准，而不是只依赖本地解析 JWT。
- 当用户直接输入 URL 访问不属于当前模式或角色不允许的页面时，前端不再静默跳页，而是展示明确的 **403 风格访问受限页**，并提供“返回当前模式首页 / 切换模式”操作。这样能区分“页面不存在”和“当前模式/权限不匹配”。
- 系统设置页会直接展示当前 JWT 的本地解析结果，包括主租户、允许租户集合、令牌级 `roles`、按租户覆盖的 `tenant_roles` 以及当前活动租户的有效角色，方便联调 JWT/RBAC 问题。
- 主要页面和关键设备操作面板已开始做按钮级权限收敛：当当前有效角色只允许读、不允许写时，创建/删除/下发/编辑类按钮会直接禁用，避免菜单已收敛但页面内仍保留写入口。
- 对这些禁用按钮，前端现在会显示统一的 tooltip 原因，明确提示“当前有效角色仅允许查看，不能执行写操作”，减少用户对灰态按钮的困惑。
- 对于主要租户页面（设备 / 策略 / 命令 / 制品 / 策略编辑器）以及平台首页，当前若角色只读，页面顶部会直接出现统一的只读提示条，不必等到 hover 按钮才知道当前页不可写。
- 设备详情中的三个写面板（设备影子 / 远控面板 / 远程桌面）也已补上同样的只读提示条，保证从列表页到详情页的权限反馈一致。
- 设备列表现在展示 `registration_code`（注册码），搜索框也支持按注册码查找；注册弹窗可显式录入该码，留空则后端自动生成，便于用户先在平台预注册、再在设备侧 Agent 输入同一注册码完成精确绑定。
- 设备详情抽屉现在支持：复制注册码、重置注册码、签发 enrollment token，并展示可直接复制的 enrollment URI 文本，便于给安装中的 Agent 做首次绑定。
- 设备详情抽屉还支持下载 Windows 一键安装脚本：脚本会内嵌 `api_url`、`tenant_id`、`enrollment_token`，并优先使用推荐 Agent 制品 `metadata.download_url` 自动下载 `dmsx-agent.exe`、写入配置、注册并启动 Windows Service；若没有下载 URL，则要求 `dmsx-agent.exe` 与脚本放在同一目录。内网或离线交付可用 `scripts/package-windows-agent.ps1` 生成包含 exe 和免参数入口脚本的 zip，用户解压运行后即可自动注册。
- 设备详情抽屉还支持直接复制 Agent 启动命令，并可跳转到独立“零接触安装页”；设备列表新增“批量预注册”弹窗，支持下载 CSV 模板、表头自动识别、粘贴或上传 CSV/TXT 批量导入、列级校验错误提示、最近批次历史回填，以及导出 `enrollment_token`、`enrollment_uri`、Agent 启动命令 CSV，同时支持一键复制 Linux/macOS、Windows、Android 专属 APK 三种零接触交付脚本，覆盖批量部署与零接触注册场景。
- 零接触安装页现在支持平台切换（Linux/macOS、Windows、Android/ADB），并会基于链接中的 `tenant_id` 直接查询该租户 `GET /v1/tenants/{tid}/artifacts` 制品列表，按平台优先推荐最近的稳定渠道 Agent 制品。若制品 `metadata` 中提供 `install_commands` / `upgrade_commands` / `download_url` 等字段，页面会直接生成“推荐首装命令”和“升级命令”；Windows 场景还会生成可下载的 `Install-DMSX-Agent.ps1`，把注册压缩为“下载脚本并运行”即可；Android 场景会生成 `scripts/package-android-agent.ps1` 命令，用内置 `api_url`、`tenant_id`、`enrollment_token` 的专属 APK 完成首次启动自动注册。当前 Agent 侧 `install_update` 已支持最小下载 / SHA256 校验 / 安装执行链路，但仍属于最小 OTA 基础能力，而不是完整的灰度发布 / 回滚体系。
- “应用分发”页上传制品时现在改为结构化填写安装/升级元数据，不再要求用户手写整段 `metadata` JSON。可分别录入适用平台、`download_url`、`installer_kind`、按平台区分的 `install_commands` / `upgrade_commands`，再由前端统一组装为后端 `metadata`。
- 设备详情页现在会基于设备平台自动推荐一个最合适的 stable 制品；在基本信息区可直接点“升级到 x.y.z”跳到远控面板并预填升级参数，减少“找制品 -> 选制品 -> 带参数”的步骤。
- 命令详情页现在会对 `install_update` 额外展示“期望版本”和“设备当前 Agent 版本”的确认状态：命令执行成功后若设备下一次心跳把 `agent_version` 更新到 `expected_version`，前端会显示“设备已确认新版本”；否则会继续提示等待心跳或显示版本不一致。
- 前端新增 `TerminalBlock` 通用组件，统一承载零接触命令、Enrollment URI、命令 payload、stdout/stderr 等长文本展示。该组件默认提供复制按钮、自动换行、水平滚动和与 Ant Design 主题联动的背景/边框色，避免 `<pre>`、`<Text code>` 在不同页面里重复堆样式。
- `App.tsx` 的全局主题 token 也同步微调：卡片与弹窗圆角统一抬高到 `borderRadiusLG: 8`，并针对暗色模式单独设置 `colorBgContainer` 与 `colorBgElevated`，保证零接触命令块、远控结果和详情抽屉在亮暗主题下都有一致层次感。
- 参考 Vben Admin 的后台工作台体验，控制台外壳新增独立样式层 `web/src/App.css`：侧栏品牌区、顶部折叠/刷新/菜单搜索、平台/租户模式切换、访问标签栏、页面标题与角色标签统一收敛到应用壳中。窄屏下侧栏隐藏但保留菜单搜索入口，避免移动端无法导航。
- 左侧菜单按模块分组展示，而不是按路由平铺：平台模式分为“平台工作台 / 权限中心 / 运营治理 / 平台运维”，租户模式分为“租户工作台 / 设备运营 / 策略安全 / 交付配置”。顶部搜索菜单仍保留全量路由快速跳转，并在结果中展示“模块 / 页面”以帮助用户理解页面归属。
- 产品导航模型已从 `App.tsx` 抽离到 `web/src/navigation.tsx`：该文件是菜单分组、路径、图标、工作模式和前端角色可见性的单一来源；后续新增或重做页面时，应先更新该模型，再补路由和页面实现。
- 平台首页会在本地 `localStorage` 中记录最近创建成功的租户，作为浏览器侧会话辅助；平台租户目录页使用后端 `GET /v1/config/tenants` 获取真实跨租户汇总。
- 平台租户目录页展示跨租户汇总视图：支持按租户名称或 UUID 搜索、分页浏览，并可一键切换当前活动租户后跳转到设备页继续排查。
- `/zero-touch-enroll` 与 `/login` 一样使用独立页面布局，不渲染控制台侧栏、顶栏或受保护导航，适合从 enrollment URI 直接打开。
- React 19 + Ant Design 5 的兼容层由 `src/antdReact19Compat.ts` 本地设置 `unstableSetRender`；不要改回官方 patch 包，当前官方 patch 在本项目生产 preview 中曾触发主题 token 运行时错误。
- 平台权限已按对象拆为独立页面：权限总览只展示摘要和入口；角色管理只维护角色信息；用户管理只维护用户信息；菜单管理只维护菜单权限映射；权限策略只维护 `platform.rbac.policy`。各对象页采用单列表 + 抽屉模式承载详情或维护表单。
- 平台配额页当前提供统一配额表和使用率条形进度：已用量来自后端真实计数（租户 / 设备 / 命令 / 制品），上限可由 PlatformAdmin 在页面内维护并保存到全局设置 `platform.quotas`；若未配置，则回退到控制面环境变量默认值。
- 平台全局审计页支持按 `action`、`resource_type` 过滤 `GET /v1/config/audit-logs`，作为租户级审计页之外的跨租户排障入口。
- 平台健康页聚合 `GET /v1/config/platform-health` 与 `GET /v1/config/livekit`，展示租户 / 设备 / 策略 / 命令 / 制品 / 审计总量，以及 LiveKit / Redis / Command Bus 的平台级运行摘要。
- 远程桌面页已额外做两处稳定化：卸载清理不再因 callback 引用变化而误触发；高频 `mousemove` 输入改为按 `requestAnimationFrame` 节流，减少切 tab 和连接后主线程被输入事件洪峰压住的风险。

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
    │   └── hooks.ts         # TanStack Query hooks（设备/策略/命令/影子/远控/桌面/租户 RBAC 等）
    ├── pages/
    │   ├── Dashboard.tsx        # 态势总览
    │   ├── Devices.tsx          # 设备管理列表页
    │   ├── Policies.tsx         # 策略中心列表页
    │   ├── Commands.tsx         # 远程命令列表页
    │   ├── Artifacts.tsx        # 应用分发列表页
    │   ├── Compliance.tsx       # 安全合规页
    │   ├── Network.tsx          # 网络管控页
    │   ├── AiCenter.tsx         # AI 智慧中心
    │   ├── PlatformOverview.tsx # 平台首页
    │   ├── PlatformPermissions.tsx # 平台权限总览
    │   ├── PlatformPermissionRoles.tsx # 平台角色管理
    │   ├── PlatformPermissionUsers.tsx # 平台用户管理
    │   ├── PlatformPermissionMenus.tsx # 平台菜单管理
    │   ├── PlatformPermissionPolicy.tsx # 平台权限策略
    │   ├── PlatformTenants.tsx  # 平台租户管理
    │   ├── PlatformQuotas.tsx   # 平台配额
    │   ├── PlatformAuditLogs.tsx# 平台全局审计
    │   ├── PlatformHealth.tsx   # 平台健康
    │   └── Login.tsx            # 登录页（JWT 模式未登录重定向入口）
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

补充交互：
- 可直接生成 Enrollment Token 与二维码。
- 若当前租户下存在匹配平台的稳定制品，会在 Agent 版本旁显示“升级到 x.y.z”入口，并跳转到远控面板预填升级参数。
- `labels` / `capabilities` / Agent 启动命令改用统一终端块展示，便于复制和查看长文本。

**设备影子 Tab**（ShadowPanel）：
- 三列对比：Reported（Agent 上报） / Desired（期望状态）/ Delta（差异）
- JSON 语法高亮（灰色背景）
- 编辑 Modal：修改 Desired 状态并立即提交

**远控面板 Tab**（RemoteControl）：
- 快捷操作网格：重启、锁屏、关机、收集日志、安装更新等
- 脚本执行器：选解释器、输入脚本、超时设置
- 安装更新弹窗：可直接选择已上传制品，自动带入 `download_url`、`sha256`、`expected_version`、`installer_kind`、自定义安装命令与推荐解释器
- 危险操作（wipe）三重确认：弹框 + 主机名输入 + 二次确认
- 命令历史表格 + 执行结果查看（exit_code / stdout / stderr）
- 执行结果里的 `stdout` / `stderr` 统一使用终端块组件展示，并保留成功/失败流的颜色区分

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

### 平台页（Platform）
- **PlatformOverview**：平台摘要与模块导航，只展示关键计数、最高配额使用率和模块入口。
- **PlatformPermissions**：平台权限总览，只展示摘要和对象入口。
- **PlatformPermissionRoles**：平台角色管理，单列表展示平台角色，抽屉查看角色说明、读写能力和 `platform.*` 权限点。
- **PlatformPermissionUsers**：平台用户管理，单列表展示用户权限快照，抽屉查看平台角色和租户覆盖。当前后端尚未提供平台用户目录，先以当前登录用户承载页面结构。
- **PlatformPermissionMenus**：平台菜单管理，单列表展示菜单、路由、权限点和允许角色，抽屉查看菜单详情。
- **PlatformPermissionPolicy**：平台权限策略，单列表展示 `platform.rbac.policy`，抽屉维护平台角色启用、登录后范围选择和默认进入范围。
- **PlatformTenants**：跨租户目录与租户创建，支持搜索、分页，以及“切换当前活动租户并进入设备页”的联动入口。
- **PlatformQuotas**：平台配额表，展示配额项、已用量、总量、单位和使用率；PlatformAdmin 可直接编辑并保存租户、设备、命令、制品四类平台上限。
- **PlatformAuditLogs**：全局审计表，支持 `action` / `resource_type` 过滤与分页。
- **PlatformHealth**：平台运行摘要，展示租户数、设备数、策略数、命令数、制品数、审计数，以及 LiveKit / Redis / Command Bus 状态。
- **UsersRoles**：平台模式下展示内置 RBAC 角色矩阵；租户模式下可直接维护当前租户自定义角色（角色名、说明、权限集合）以及用户-角色绑定（subject、显示名、角色数组）。

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
| `useRbacRoles` | RBAC 角色模板 |
| `useSystemSetting` / `useUpsertSystemSetting` | 系统设置 |
| `useCreateTenant` | 创建租户 |
| `usePlatformTenants` | 平台租户目录 |
| `usePlatformQuotas` | 平台配额 |
| `usePlatformQuotaSettings` / `useUpsertPlatformQuotaSettings` | 平台配额上限维护（`platform.quotas`） |
| `usePlatformRbacPolicy` / `useUpsertPlatformRbacPolicy` | 平台权限策略维护（`platform.rbac.policy`） |
| `usePlatformAuditLogs` | 平台全局审计 |
| `usePlatformHealth` | 平台健康摘要 |

## 与后端对接

- 开发时 Vite proxy 转发 `/v1/*` 到 `http://127.0.0.1:8080`
- 生产：Nginx/Ingress 统一入口，前端静态资源 + API 反代
- API 客户端（`api/client.ts`）统一处理 Problem Details 错误
- 所有 TypeScript 类型定义在 `api/types.ts`，与后端 DTO 严格对齐
- `tsconfig.json` 使用 `noEmit: true`，避免将编译产物写回 `src/`
- `App.tsx`、系统设置页、审计页等依赖 `AppProviders` 上下文；若调整入口挂载顺序，需保证 `RouterProvider` 仍包在 `AppProviders` 内。
