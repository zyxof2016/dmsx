# DMSX Android Agent

这是可安装在 Android 手机上的最小原生 APK Agent，用于在不依赖 Termux 的情况下接入 DMSX 控制面。

## 功能

- 配置页：保存 API URL、Tenant ID、Enrollment Token。
- 前台服务：常驻通知、wake lock、可开机自启。
- 设备认领：调用 `POST /v1/tenants/{tenant_id}/devices/claim-with-enrollment-token`。
- 设备写回：携带 `X-DMSX-Device-Token` 更新设备状态、reported shadow、命令状态和命令结果。
- 遥测：Android 型号、版本、厂商、安全补丁、电池、网络、uptime。
- 命令：支持 `smoke_noop` 与 `collect_logs`。

## 构建

需要 Android SDK、JDK 17、Gradle 或 Android Studio。

```bash
cd android-agent
gradle assembleDebug
```

安装：

```bash
adb install -r app/build/outputs/apk/debug/app-debug.apk
```

## 单 APK 自动注册

运维侧可以把控制面地址、租户和 enrollment token 直接打进 APK：

```powershell
.\scripts\package-android-agent.ps1 `
  -ApiUrl "http://<server-ip>:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001" `
  -EnrollmentToken "<token>" `
  -OutputPath ".\target\packages\DMSX-Agent-Android.apk"
```

生成的 APK 安装后首次打开会自动载入内置配置、启动前台服务并认领设备。普通 Android 侧载安装后，系统不会允许应用在用户完全未打开的情况下自行启动后台服务；Android Enterprise / Device Owner 或 ADB 场景可在安装后由管理端拉起应用。

也可以直接用 Gradle 属性构建：

```bash
cd android-agent
gradle assembleDebug \
  -PdmsxApiUrl="http://<server-ip>:8080" \
  -PdmsxTenantId="00000000-0000-0000-0000-000000000001" \
  -PdmsxEnrollmentToken="<token>"
```

## 使用

1. 在控制台预注册 `platform=android` 设备并签发 enrollment token。
2. 生成并安装专属 APK。
3. 打开手机上的 DMSX Agent，应用会自动启动注册流程。

未内置配置的开发 APK 仍可手动填入 `API URL`、`Tenant ID`、`Enrollment Token` 后点击“保存配置”和“启动 Agent”。

## 限制

- 当前是基础可安装 Agent，不是完整 Android Enterprise / Device Owner MDM。
- 不执行任意脚本、不做静默装包、不做锁屏/擦除。
- 为本地联调允许 cleartext HTTP；生产请改为 HTTPS。
