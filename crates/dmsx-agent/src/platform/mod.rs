use sysinfo::System;

pub fn detect_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "linux") {
        if is_android_runtime() {
            "android"
        } else {
            "linux"
        }
    } else {
        "other"
    }
}

pub fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".into())
}

pub fn os_version() -> Option<String> {
    System::os_version()
}

fn is_android_runtime() -> bool {
    std::path::Path::new("/system/build.prop").exists()
        || std::env::var("ANDROID_ROOT").is_ok()
        || std::env::var("PREFIX")
            .map(|p| p.contains("com.termux"))
            .unwrap_or(false)
}
