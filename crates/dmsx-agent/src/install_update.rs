use std::path::{Path, PathBuf};

use futures_util::StreamExt;
use reqwest::Client;
use sha2::{Digest, Sha256};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tracing::{info, warn};
use uuid::Uuid;

use crate::script::run_script;

struct InstallUpdateSpec {
    download_url: String,
    expected_sha256: Option<String>,
    expected_version: Option<String>,
    file_name: Option<String>,
    installer_kind: Option<String>,
    install_command: Option<String>,
    interpreter: Option<String>,
    timeout_secs: u64,
}

impl InstallUpdateSpec {
    fn from_params(params: &serde_json::Value) -> Result<Self, String> {
        let download_url = params
            .get("download_url")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "missing download_url parameter".to_string())?
            .to_string();
        let expected_sha256 = params
            .get("sha256")
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        if let Some(value) = expected_sha256.as_deref() {
            if value.len() != 64 || !value.chars().all(|ch| ch.is_ascii_hexdigit()) {
                return Err("sha256 must be a 64-character hex string".into());
            }
        }
        Ok(Self {
            download_url,
            expected_sha256,
            expected_version: params
                .get("expected_version")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            file_name: params
                .get("file_name")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            installer_kind: params
                .get("installer_kind")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| value.to_ascii_lowercase()),
            install_command: params
                .get("install_command")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            interpreter: params
                .get("interpreter")
                .and_then(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string),
            timeout_secs: params
                .get("timeout")
                .and_then(|value| value.as_u64())
                .unwrap_or(900),
        })
    }
}

pub async fn run_install_update(client: &Client, params: &serde_json::Value) -> (i32, String, String) {
    let spec = match InstallUpdateSpec::from_params(params) {
        Ok(spec) => spec,
        Err(error) => return (1, String::new(), error),
    };

    let download_path = build_download_path(&spec);
    info!(download_url = %spec.download_url, path = %download_path.display(), "downloading update payload");

    let download_result = download_to_path(client, &spec, &download_path).await;
    let (actual_sha256, download_stdout) = match download_result {
        Ok(value) => value,
        Err(error) => {
            cleanup_download(&download_path).await;
            return (1, String::new(), error);
        }
    };

    let install_result = execute_install(&spec, &download_path, &actual_sha256).await;
    cleanup_download(&download_path).await;

    let (exit_code, stdout, stderr) = install_result;
    let mut stdout = if stdout.trim().is_empty() {
        download_stdout
    } else {
        format!("{download_stdout}\n{stdout}")
    };
    if let Some(expected_version) = spec.expected_version.as_deref() {
        stdout.push_str(&format!("\nexpected_version={expected_version}"));
        stdout.push_str("\nversion_confirmation=wait_for_next_heartbeat");
    }
    (exit_code, stdout, stderr)
}

fn build_download_path(spec: &InstallUpdateSpec) -> PathBuf {
    let file_name = spec
        .file_name
        .as_deref()
        .map(sanitize_file_name)
        .filter(|value| !value.is_empty())
        .or_else(|| infer_file_name_from_url(&spec.download_url))
        .unwrap_or_else(|| {
            let suffix = infer_extension(spec).unwrap_or("bin");
            format!("dmsx-agent-update.{suffix}")
        });
    std::env::temp_dir().join(format!("dmsx-update-{}-{file_name}", Uuid::new_v4()))
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' | '-' => ch,
            _ => '_',
        })
        .collect::<String>();
    Path::new(&sanitized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("update.bin")
        .to_string()
}

fn infer_file_name_from_url(download_url: &str) -> Option<String> {
    let url = reqwest::Url::parse(download_url).ok()?;
    let segment = url.path_segments()?.next_back()?;
    let sanitized = sanitize_file_name(segment);
    if sanitized.is_empty() {
        None
    } else {
        Some(sanitized)
    }
}

fn infer_extension(spec: &InstallUpdateSpec) -> Option<&'static str> {
    match spec.installer_kind.as_deref() {
        Some("msi") => Some("msi"),
        Some("exe") => Some("exe"),
        Some("ps1") => Some("ps1"),
        Some("sh") | Some("script") => Some("sh"),
        Some("deb") => Some("deb"),
        Some("rpm") => Some("rpm"),
        Some("pkg") => Some("pkg"),
        Some("apk") => Some("apk"),
        _ => None,
    }
}

async fn download_to_path(client: &Client, spec: &InstallUpdateSpec, path: &Path) -> Result<(String, String), String> {
    let response = client
        .get(&spec.download_url)
        .send()
        .await
        .map_err(|error| format!("download request failed: {error}"))?;

    if !response.status().is_success() {
        return Err(format!("download failed with status {}", response.status()));
    }

    let mut file = fs::File::create(path)
        .await
        .map_err(|error| format!("failed to create download file: {error}"))?;
    let mut hasher = Sha256::new();
    let mut bytes_written: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|error| format!("download stream failed: {error}"))?;
        file.write_all(&chunk)
            .await
            .map_err(|error| format!("failed to write update payload: {error}"))?;
        hasher.update(&chunk);
        bytes_written += chunk.len() as u64;
    }
    file.flush()
        .await
        .map_err(|error| format!("failed to flush update payload: {error}"))?;

    let actual_sha256 = format!("{:x}", hasher.finalize());
    if let Some(expected) = spec.expected_sha256.as_deref() {
        if actual_sha256 != expected {
            return Err(format!(
                "sha256 mismatch: expected {expected}, got {actual_sha256}"
            ));
        }
    }

    Ok((
        actual_sha256.clone(),
        format!(
            "downloaded update payload: {} bytes -> {}\nsha256={actual_sha256}",
            bytes_written,
            path.display()
        ),
    ))
}

async fn execute_install(spec: &InstallUpdateSpec, path: &Path, actual_sha256: &str) -> (i32, String, String) {
    let script = if let Some(command) = spec.install_command.as_deref() {
        materialize_install_command(command, &spec.download_url, path, actual_sha256)
    } else {
        match default_install_command(spec, path) {
            Ok(command) => command,
            Err(error) => return (1, String::new(), error),
        }
    };

    let interpreter = spec
        .interpreter
        .clone()
        .unwrap_or_else(default_interpreter);

    info!(
        download_url = %spec.download_url,
        interpreter = %interpreter,
        timeout_secs = spec.timeout_secs,
        "executing install_update"
    );

    run_script(&serde_json::json!({
        "script": script,
        "interpreter": interpreter,
        "timeout": spec.timeout_secs,
    }))
    .await
}

fn materialize_install_command(template: &str, download_url: &str, path: &Path, actual_sha256: &str) -> String {
    template
        .replace("{{file_path}}", &path.display().to_string())
        .replace("{{download_url}}", download_url)
        .replace("{{sha256}}", actual_sha256)
}

fn default_interpreter() -> String {
    if cfg!(target_os = "windows") {
        "powershell".into()
    } else {
        "sh".into()
    }
}

fn default_install_command(spec: &InstallUpdateSpec, path: &Path) -> Result<String, String> {
    let kind = spec
        .installer_kind
        .clone()
        .or_else(|| infer_installer_kind(path))
        .ok_or_else(|| "install_update requires installer_kind or install_command".to_string())?;
    let file_path = path.display();

    let script = match kind.as_str() {
        "sh" | "script" => format!("sh '{file_path}'"),
        "ps1" => format!("& '{file_path}'"),
        "msi" => format!(
            "Start-Process msiexec.exe -Wait -ArgumentList @('/i','{file_path}','/qn','/norestart')"
        ),
        "exe" => format!("Start-Process -FilePath '{file_path}' -Wait -ArgumentList @('/quiet','/norestart')"),
        "deb" => format!("sudo dpkg -i '{file_path}'"),
        "rpm" => format!("sudo rpm -Uvh '{file_path}'"),
        "pkg" => format!("sudo installer -pkg '{file_path}' -target /"),
        "apk" => format!("pm install -r '{file_path}'"),
        other => return Err(format!("unsupported installer_kind: {other}")),
    };

    Ok(script)
}

fn infer_installer_kind(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "sh" => Some("sh".into()),
        "ps1" => Some("ps1".into()),
        "msi" => Some("msi".into()),
        "exe" => Some("exe".into()),
        "deb" => Some("deb".into()),
        "rpm" => Some("rpm".into()),
        "pkg" => Some("pkg".into()),
        "apk" => Some("apk".into()),
        _ => None,
    }
}

async fn cleanup_download(path: &Path) {
    if let Err(error) = fs::remove_file(path).await {
        if error.kind() != std::io::ErrorKind::NotFound {
            warn!(path = %path.display(), error = %error, "failed to remove downloaded update payload");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{infer_installer_kind, materialize_install_command, run_install_update};
    use reqwest::Client;
    use sha2::{Digest, Sha256};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn infer_installer_kind_from_extension() {
        let kind = infer_installer_kind(std::path::Path::new("/tmp/update.pkg")).unwrap();
        assert_eq!(kind, "pkg");
    }

    #[test]
    fn materialize_install_command_replaces_placeholders() {
        let path = std::path::Path::new("/tmp/update.sh");
        let rendered = materialize_install_command(
            "sh {{file_path}} --source {{download_url}} --sha {{sha256}}",
            "https://example.com/update.sh",
            path,
            "abc123",
        );
        assert!(rendered.contains("/tmp/update.sh"));
        assert!(rendered.contains("https://example.com/update.sh"));
        assert!(rendered.contains("abc123"));
    }

    #[tokio::test]
    async fn run_install_update_requires_download_url() {
        let client = Client::new();
        let result = run_install_update(&client, &serde_json::json!({})).await;
        assert_eq!(result.0, 1);
        assert_eq!(result.2, "missing download_url parameter");
    }

    #[tokio::test]
    async fn run_install_update_rejects_sha_mismatch() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/update.sh"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"echo hello"))
            .mount(&server)
            .await;

        let client = Client::new();
        let result = run_install_update(
            &client,
            &serde_json::json!({
                "download_url": format!("{}/update.sh", server.uri()),
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "installer_kind": "sh"
            }),
        )
        .await;

        assert_eq!(result.0, 1);
        assert!(result.2.contains("sha256 mismatch"));
    }

    #[tokio::test]
    async fn run_install_update_downloads_and_executes_shell_script() {
        if cfg!(target_os = "windows") {
            return;
        }

        let script = b"#!/bin/sh\necho install ok\n";
        let sha256 = format!("{:x}", Sha256::digest(script));
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/update.sh"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(script.as_slice()))
            .mount(&server)
            .await;

        let client = Client::new();
        let result = run_install_update(
            &client,
            &serde_json::json!({
                "download_url": format!("{}/update.sh", server.uri()),
                "sha256": sha256,
                "installer_kind": "sh",
                "timeout": 10
            }),
        )
        .await;

        assert_eq!(result.0, 0);
        assert!(result.1.contains("downloaded update payload"));
        assert!(result.1.contains("install ok"));
        assert!(result.2.is_empty());
    }
}
