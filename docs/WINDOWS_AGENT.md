# Windows Agent 部署指南

DMSX Windows Agent 使用 Rust `dmsx-agent.exe`，可直接前台运行，也可注册为 Windows Service 常驻运行。

## 编译

在 Windows 开发机上安装 Rust stable 后执行：

```powershell
cargo build -p dmsx-agent --release
```

如果要在 Windows 本地开发环境直接调试远程桌面链路，`libwebrtc` 依赖通常需要静态 CRT 链接。仓库提供了开发启动脚本，会自动设置 `RUSTFLAGS="-C target-feature=+crt-static"`：

```powershell
$env:DMSX_DEVICE_ENROLLMENT_TOKEN = "<token>"
.\scripts\run-windows-agent-dev.ps1 `
  -ApiUrl "http://127.0.0.1:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001"
```

产物：

```text
target\release\dmsx-agent.exe
```

## 前台运行

适合开发验证：

```powershell
$env:DMSX_API_URL = "http://<server-ip>:8080"
$env:DMSX_TENANT_ID = "00000000-0000-0000-0000-000000000001"
$env:DMSX_DEVICE_ENROLLMENT_TOKEN = "<token>"
.\target\release\dmsx-agent.exe
```

Agent 会优先用 enrollment token 调用 `claim-with-enrollment-token` 认领设备，然后携带 `X-DMSX-Device-Token` 上报心跳、reported shadow，并轮询命令。

## 安装为 Windows Service

用管理员 PowerShell 执行：

```powershell
.\scripts\install-windows-agent.ps1 `
  -ApiUrl "http://<server-ip>:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001" `
  -EnrollmentToken "<token>" `
  -AgentExe ".\target\release\dmsx-agent.exe"
```

也可以让脚本直接下载安装二进制：

```powershell
.\scripts\install-windows-agent.ps1 `
  -ApiUrl "http://<server-ip>:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001" `
  -EnrollmentToken "<token>" `
  -AgentExeUrl "https://example.com/dmsx-agent.exe"
```

如果不是管理员运行，脚本会自动触发 UAC 提权后继续安装。

需要交付给终端用户时，可以在平台侧或运维机上先打成免参数安装包：

```powershell
.\scripts\package-windows-agent.ps1 `
  -ApiUrl "http://<server-ip>:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001" `
  -EnrollmentToken "<token>" `
  -AgentExe ".\target\release\dmsx-agent.exe" `
  -OutputPath ".\target\packages\DMSX-Agent-Install.zip"
```

生成的 zip 内包含 `dmsx-agent.exe`、通用安装脚本和免参数入口 `Install-DMSX-Agent.ps1`。用户只需要解压并运行入口脚本；脚本会写入 enrollment 配置、注册 Windows Service 并启动 Agent，Agent 首次启动后会自动认领设备。

安装脚本会：

- 复制二进制到 `C:\Program Files\DMSX\Agent\dmsx-agent.exe`
- 写入配置到 `C:\ProgramData\DMSX\Agent\agent.json`
- 注册并启动 `DMSXAgent` 服务

查看服务：

```powershell
Get-Service DMSXAgent
Get-Content "C:\ProgramData\DMSX\Agent\agent.json"
```

卸载：

```powershell
.\scripts\uninstall-windows-agent.ps1
```

## 配置文件

服务模式使用 JSON 配置文件，字段与环境变量等价：

```json
{
  "api_base": "http://<server-ip>:8080",
  "tenant_id": "00000000-0000-0000-0000-000000000001",
  "enrollment_token": "<token>",
  "heartbeat_secs": 30,
  "poll_secs": 10,
  "command_execution_timeout_secs": 300
}
```

也可以手动运行：

```powershell
.\dmsx-agent.exe --windows-service --config "C:\ProgramData\DMSX\Agent\agent.json"
```

该入口由 Windows Service Control Manager 调用，普通控制台调试仍建议使用环境变量前台运行。

## 当前能力

- 设备 enrollment token 认领
- 心跳和 reported shadow
- 命令轮询与结果回写
- PowerShell `run_script`
- `reboot` / `shutdown` / `lock_screen`
- LiveKit 远程桌面链路（依赖桌面会话和权限）
- RustDesk relay 配置（可选）

## 限制

- Windows Service 默认运行在 Session 0，直接屏幕采集/键鼠注入可能受桌面会话隔离影响；远程桌面生产部署通常需要用户会话辅助进程或专用远控组件。
- 安装脚本写入 enrollment token 到本机 `ProgramData`，生产环境应结合 ACL、设备会话 token 或证书身份做进一步收口。
