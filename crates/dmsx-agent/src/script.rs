use std::process::Stdio;

use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::Command as TokioCommand;
use tracing::{info, warn};

pub async fn run_script(params: &serde_json::Value) -> (i32, String, String) {
    let script = match params.get("script").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (1, String::new(), "missing script parameter".into()),
    };
    let interpreter = params
        .get("interpreter")
        .and_then(|v| v.as_str())
        .unwrap_or(if cfg!(target_os = "windows") {
            "powershell"
        } else {
            "bash"
        });
    let timeout_secs = params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);

    let (program, args) = match resolve_script_command(interpreter, script) {
        Ok(command) => command,
        Err(e) => return (1, String::new(), e),
    };

    info!(interpreter, timeout_secs, "running script");

    let child = TokioCommand::new(program)
        .args(args.iter().map(String::as_str))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let mut child = match child {
        Ok(c) => c,
        Err(e) => return (1, String::new(), format!("spawn failed: {e}")),
    };

    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();

    match tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), child.wait()).await {
        Ok(Ok(status)) => {
            let stdout = read_pipe_to_string(&mut stdout).await;
            let stderr = read_pipe_to_string(&mut stderr).await;
            let code = status.code().unwrap_or(-1);
            (code, stdout, stderr)
        }
        Ok(Err(e)) => (1, String::new(), format!("process error: {e}")),
        Err(_) => {
            warn!(interpreter, timeout_secs, "script exceeded timeout; terminating child process");
            if let Err(e) = child.kill().await {
                warn!(interpreter, timeout_secs, error = %e, "failed to kill timed out script process");
            }
            let _ = child.wait().await;
            (124, String::new(), format!("timeout after {timeout_secs}s; process terminated"))
        }
    }
}

async fn read_pipe_to_string<R>(pipe: &mut Option<R>) -> String
where
    R: AsyncRead + Unpin,
{
    let mut bytes = Vec::new();
    if let Some(reader) = pipe.as_mut() {
        let _ = reader.read_to_end(&mut bytes).await;
    }
    String::from_utf8_lossy(&bytes).to_string()
}

pub fn resolve_script_command(
    interpreter: &str,
    script: &str,
) -> Result<(String, Vec<String>), String> {
    match interpreter {
        "powershell" | "pwsh" => {
            if cfg!(target_os = "windows") {
                Ok((
                    "powershell.exe".into(),
                    vec!["-NoProfile".into(), "-Command".into(), script.into()],
                ))
            } else {
                Ok((
                    "pwsh".into(),
                    vec!["-NoProfile".into(), "-Command".into(), script.into()],
                ))
            }
        }
        "bash" => Ok(("bash".into(), vec!["-c".into(), script.into()])),
        "sh" => Ok(("sh".into(), vec!["-c".into(), script.into()])),
        "python" | "python3" => Ok(("python3".into(), vec!["-c".into(), script.into()])),
        other => Err(format!("unsupported interpreter: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::{resolve_script_command, run_script};

    #[test]
    fn resolve_script_command_supports_bash() {
        let (program, args) = resolve_script_command("bash", "echo hi").unwrap();
        assert_eq!(program, "bash");
        assert_eq!(args, vec!["-c", "echo hi"]);
    }

    #[test]
    fn resolve_script_command_rejects_unknown_interpreter() {
        let error = resolve_script_command("ruby", "puts 1").unwrap_err();
        assert_eq!(error, "unsupported interpreter: ruby");
    }

    #[tokio::test]
    async fn run_script_requires_script_parameter() {
        let params = serde_json::json!({});
        let result = run_script(&params).await;

        assert_eq!(result.0, 1);
        assert!(result.1.is_empty());
        assert_eq!(result.2, "missing script parameter");
    }

    #[tokio::test]
    async fn run_script_kills_process_on_timeout() {
        let params = if cfg!(target_os = "windows") {
            serde_json::json!({
                "script": "Start-Sleep -Seconds 3",
                "interpreter": "powershell",
                "timeout": 1
            })
        } else {
            serde_json::json!({
                "script": "sleep 3",
                "interpreter": "sh",
                "timeout": 1
            })
        };

        let result = run_script(&params).await;
        assert_eq!(result.0, 124);
        assert!(result.1.is_empty());
        assert!(result.2.contains("process terminated"));
    }
}
