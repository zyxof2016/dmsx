use chrono::Utc;
use sysinfo::{Disks, System};

use crate::platform::{detect_platform, hostname, os_version};
use crate::rustdesk::detect_rustdesk;

pub fn collect_telemetry() -> serde_json::Value {
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();
    let disk_info: Vec<serde_json::Value> = disks
        .list()
        .iter()
        .map(|d| {
            serde_json::json!({
                "mount": d.mount_point().to_string_lossy(),
                "total_gb": d.total_space() as f64 / 1_073_741_824.0,
                "free_gb": d.available_space() as f64 / 1_073_741_824.0,
            })
        })
        .collect();

    let mut telemetry = serde_json::json!({
        "agent_version": env!("CARGO_PKG_VERSION"),
        "platform": detect_platform(),
        "os_name": System::name().unwrap_or_default(),
        "os_version": os_version().unwrap_or_default(),
        "kernel_version": System::kernel_version().unwrap_or_default(),
        "hostname": hostname(),
        "cpu_count": sys.cpus().len(),
        "cpu_brand": sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        "total_memory_mb": sys.total_memory() / 1_048_576,
        "used_memory_mb": sys.used_memory() / 1_048_576,
        "total_swap_mb": sys.total_swap() / 1_048_576,
        "uptime_secs": System::uptime(),
        "disks": disk_info,
        "collected_at": Utc::now().to_rfc3339(),
    });

    if detect_platform() == "android" {
        if let Some(android_info) = collect_android_props() {
            telemetry
                .as_object_mut()
                .expect("telemetry root is object")
                .insert("android".into(), android_info);
        }
    }

    let rd = detect_rustdesk();
    telemetry
        .as_object_mut()
        .expect("telemetry root is object")
        .insert("rustdesk".into(), rd);

    telemetry
}

fn collect_android_props() -> Option<serde_json::Value> {
    let read_prop = |key: &str| -> String {
        std::process::Command::new("getprop")
            .arg(key)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };

    let model = read_prop("ro.product.model");
    if model.is_empty() {
        return None;
    }

    Some(serde_json::json!({
        "model": model,
        "manufacturer": read_prop("ro.product.manufacturer"),
        "brand": read_prop("ro.product.brand"),
        "sdk_version": read_prop("ro.build.version.sdk"),
        "android_version": read_prop("ro.build.version.release"),
        "build_fingerprint": read_prop("ro.build.fingerprint"),
        "security_patch": read_prop("ro.build.version.security_patch"),
        "serial": read_prop("ro.serialno"),
        "abi": read_prop("ro.product.cpu.abi"),
    }))
}
