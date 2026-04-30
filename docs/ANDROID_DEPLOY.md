# Android 设备接入指南

## 快速路径：Termux（推荐用于测试）

### 前提条件
- Android 7.0+ 设备
- 安装 [Termux](https://f-droid.org/packages/com.termux/)（从 F-Droid 获取，不要用 Play Store 版本）
- PC 端的 dmsx-api 服务已启动，且手机和 PC 在同一局域网

### 步骤

```bash
# 1. 在 Termux 中安装编译工具
pkg update && pkg install -y rust binutils git openssh

# 2. 克隆项目
git clone <your-repo-url> dmsx && cd dmsx

# 3. 编译 Agent（首次约 5-10 分钟）
cargo build --bin dmsx-agent --release

# 4. 查找 PC 的局域网 IP
#    在 PC 上执行: ip addr show | grep 192
#    或 Windows: ipconfig

# 5. 运行 Agent
export DMSX_API_URL="http://<PC局域网IP>:8080"
# 若平台已预注册并生成 enrollment token，推荐同时注入：
# export DMSX_DEVICE_ENROLLMENT_TOKEN="<token>"
./target/release/dmsx-agent
```

Agent 将自动：
- 检测 Android 环境（通过 `/system/build.prop` 和 Termux 特征）
- 采集设备信息（型号、制造商、Android 版本、安全补丁级别等）
- 注册到集控平台，platform 显示为 `android`
- 周期性上报遥测到设备影子
- 接收并执行远控命令

### Termux 后台运行

```bash
# 获取 Termux wake lock（防止 Android 杀掉进程）
termux-wake-lock

# 后台运行
nohup ./target/release/dmsx-agent > /data/data/com.termux/files/home/agent.log 2>&1 &

# 开机自启（Termux:Boot 插件）
mkdir -p ~/.termux/boot
cat > ~/.termux/boot/start-agent.sh << 'EOF'
#!/data/data/com.termux/files/usr/bin/bash
termux-wake-lock
export DMSX_API_URL="http://<PC局域网IP>:8080"
cd ~/dmsx
./target/release/dmsx-agent >> ~/agent.log 2>&1
EOF
chmod +x ~/.termux/boot/start-agent.sh
```

---

## 方案 B：PC 交叉编译 + ADB 推送

适用于批量设备部署、零接触注册、ADB 批量下发或无法在设备上编译的场景。

### 前提条件
- Android NDK（通过 Android Studio 或独立下载）
- `cargo-ndk` 工具
- ADB 调试已开启

### 步骤

```bash
# 1. 安装 Android target
rustup target add aarch64-linux-android   # 64-bit ARM（绝大多数现代手机）
rustup target add armv7-linux-androideabi # 32-bit ARM（旧设备）

# 2. 安装 cargo-ndk
cargo install cargo-ndk

# 3. 设置 NDK 路径
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/<version>

# 4. 交叉编译
cargo ndk -t arm64-v8a build --bin dmsx-agent --release

# 5. 推送到设备
adb push target/aarch64-linux-android/release/dmsx-agent /data/local/tmp/
adb shell chmod +x /data/local/tmp/dmsx-agent

# 6. 在设备上运行
adb shell "DMSX_API_URL=http://<PC_IP>:8080 DMSX_DEVICE_ENROLLMENT_TOKEN=<token> /data/local/tmp/dmsx-agent"
```

---

## 方案 C：原生 Android APK Agent（可安装 App）

仓库现在包含一个最小原生 Android Agent 工程：`android-agent/`。它是可安装 APK，不依赖 Termux，适合手机侧长期运行的基础接入验证。

### 已实现能力

- 配置页：可手动输入 `API URL`、`Tenant ID`、`Enrollment Token`，也可读取 APK 构建时内置的注册配置并自动启动 Agent。
- 前台服务：常驻通知 + wake lock，降低后台被杀概率。
- 开机自启：可在配置页启用，收到 `BOOT_COMPLETED` 后自动启动。
- Enrollment claim：调用 `POST /v1/tenants/{tid}/devices/claim-with-enrollment-token` 认领预注册设备。
- 心跳写回：携带 `X-DMSX-Device-Token` 更新设备 `online_state`、`agent_version`、`os_version`。
- Shadow reported：上报 Android 版本、型号、厂商、电池、网络、uptime 等遥测。
- 命令轮询：支持 `smoke_noop` 与 `collect_logs` 两个安全命令，并写回状态/结果。

### 构建 APK

前提：安装 Android Studio 或命令行 Android SDK，确保有 JDK 17 与 Android Gradle Plugin 可用。

```bash
cd android-agent
gradle assembleDebug
```

如果本机没有全局 `gradle`，可在 Android Studio 中打开 `android-agent/`，等待 Gradle 同步后执行 `assembleDebug`。生成物通常位于：

```bash
android-agent/app/build/outputs/apk/debug/app-debug.apk
```

生成单设备专属 APK：

```powershell
.\scripts\package-android-agent.ps1 `
  -ApiUrl "http://<server-ip>:8080" `
  -TenantId "00000000-0000-0000-0000-000000000001" `
  -EnrollmentToken "<token>" `
  -OutputPath ".\target\packages\DMSX-Agent-Android.apk"
```

该脚本会通过 Gradle 属性把 `api_url`、`tenant_id`、`enrollment_token` 写入 `BuildConfig`，再复制出可交付的 APK。直接使用 Gradle 时也可传同名属性：

```bash
cd android-agent
gradle assembleDebug \
  -PdmsxApiUrl="http://<server-ip>:8080" \
  -PdmsxTenantId="00000000-0000-0000-0000-000000000001" \
  -PdmsxEnrollmentToken="<token>"
```

当前仓库没有提交 Gradle Wrapper；如果希望 CI 或无全局 Gradle 的机器直接构建，可在已安装 Gradle 的环境中执行：

```bash
cd android-agent
gradle wrapper --gradle-version 8.10.2
```

安装到手机：

```bash
adb install -r android-agent/app/build/outputs/apk/debug/app-debug.apk
```

### 使用步骤

1. 在控制台预注册一台 `platform=android` 的设备并签发 enrollment token。
2. 用 `scripts/package-android-agent.ps1` 生成专属 APK。
3. 安装 APK 后打开 **DMSX Agent**。
4. App 会自动载入内置配置、启动前台服务并认领设备。
5. 控制台应看到设备被认领、在线状态更新、reported shadow 有 Android 遥测。

未内置配置的开发 APK 仍可手动填入 `API URL`、`Tenant ID`、`Enrollment Token`，再点击“保存配置”和“启动 Agent”。

### 限制

- 当前 APK Agent 不执行任意脚本，不做静默安装、不锁屏、不擦除设备；普通侧载安装后仍需要用户首次打开 App，完全无用户动作的静默安装和自动拉起需要 Device Owner / Android Enterprise 管理模式。
- 当前命令集只包含 `smoke_noop` 与 `collect_logs`，用于安全验证链路。
- 当前使用 HTTP polling；生产数据面后续仍建议走 mTLS/gRPC 或 FCM 唤醒。
- 为方便本地联调，Manifest 允许 cleartext HTTP；生产应改用 HTTPS 并限制网络安全配置。

---

## 方案 D：原生 Android App（生产级 MDM）

如需完整的企业移动设备管理（EMM），需要原生 Android 应用：

### 架构

```
┌─────────────────────────────────────────────┐
│              Android App                     │
│  ┌─────────────┐  ┌──────────────────────┐  │
│  │ AgentService│  │ DeviceAdminReceiver   │  │
│  │ (前台服务)   │  │ (设备管理员权限)       │  │
│  └──────┬──────┘  └──────────┬───────────┘  │
│         │                    │               │
│  ┌──────┴──────┐  ┌─────────┴────────────┐  │
│  │ REST Client │  │ Policy Enforcer       │  │
│  │ (Retrofit)  │  │ (WiFi/VPN/密码策略)    │  │
│  └──────┬──────┘  └──────────────────────┘  │
│         │         ┌──────────────────────┐  │
│         │         │ Telemetry Collector   │  │
│         │         │ (电池/网络/位置/App)   │  │
│         │         └──────────┬───────────┘  │
│         └────────────────────┴───────┐      │
│                                      │      │
└──────────────────────────────────────┼──────┘
                                       │
                              DMSX REST API
```

### 核心能力

| 能力 | Termux Agent | 原生 App |
|------|:---:|:---:|
| 系统遥测（CPU/内存/磁盘） | ✅ | ✅ |
| 设备型号/Android 版本 | ✅ | ✅ |
| 远程脚本执行 | ✅ | ❌ (沙箱限制) |
| 远程锁屏 | ❌ | ✅ (DeviceAdmin) |
| 远程擦除 | ❌ | ✅ (DeviceOwner) |
| 强制密码策略 | ❌ | ✅ |
| WiFi/VPN 配置下发 | ❌ | ✅ |
| App 黑白名单 | ❌ | ✅ |
| 电池/网络状态 | 部分 | ✅ |
| GPS 定位 | ❌ | ✅ |
| 推送通知（FCM） | ❌ | ✅ |
| Play Store 静默安装 | ❌ | ✅ (Managed Google Play) |
| 无需用户交互后台运行 | 需 wake-lock | ✅ (前台服务) |

### 技术栈建议

- **语言**: Kotlin
- **网络**: Retrofit + OkHttp (REST) / gRPC-kotlin (数据面)
- **后台**: WorkManager (周期任务) + Foreground Service (常驻)
- **推送**: Firebase Cloud Messaging (替代轮询)
- **MDM API**: Android DevicePolicyManager + Device Owner mode
- **分发**: Managed Google Play / 企业侧载 APK

---

## 当前推荐

对于**开发验证阶段**，直接用 **Termux + Rust Agent** 即可在 5 分钟内让安卓设备接入集控系统，所有远控和遥测功能都能正常工作。

原生 Android App 适合在产品化阶段投入开发。
