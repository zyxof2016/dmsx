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

## 方案 C：原生 Android App（生产级 MDM）

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
