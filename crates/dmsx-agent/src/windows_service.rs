#![cfg(windows)]

use std::path::PathBuf;
use std::sync::mpsc;

use dmsx_agent::config::AgentConfig;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::{define_windows_service, service_dispatcher};

use crate::app;

const SERVICE_NAME: &str = "DMSXAgent";
const SERVICE_TYPE: ServiceType = ServiceType::OWN_PROCESS;

define_windows_service!(ffi_service_main, service_main);

pub(crate) fn run_service() -> windows_service::Result<()> {
    service_dispatcher::start(SERVICE_NAME, ffi_service_main)
}

fn service_main(arguments: Vec<std::ffi::OsString>) {
    if let Err(error) = run(arguments) {
        tracing::error!(%error, "DMSX Windows service exited with error");
    }
}

fn run(arguments: Vec<std::ffi::OsString>) -> windows_service::Result<()> {
    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>();
    let status_handle =
        service_control_handler::register(
            SERVICE_NAME,
            move |control_event| match control_event {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    let _ = shutdown_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                _ => ServiceControlHandlerResult::NotImplemented,
            },
        )?;

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::StartPending,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::from_secs(10),
        process_id: None,
    })?;

    let config_path = parse_config_path(arguments);
    let cfg = AgentConfig::from_sources(config_path.as_deref());

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Running,
        controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;

    let runtime = tokio::runtime::Runtime::new().expect("create tokio runtime");
    runtime.block_on(async move {
        app::run_with_shutdown(cfg, async move {
            let _ = tokio::task::spawn_blocking(move || shutdown_rx.recv()).await;
        })
        .await;
    });

    status_handle.set_service_status(ServiceStatus {
        service_type: SERVICE_TYPE,
        current_state: ServiceState::Stopped,
        controls_accepted: ServiceControlAccept::empty(),
        exit_code: ServiceExitCode::Win32(0),
        checkpoint: 0,
        wait_hint: std::time::Duration::default(),
        process_id: None,
    })?;
    Ok(())
}

fn parse_config_path(arguments: Vec<std::ffi::OsString>) -> Option<PathBuf> {
    arguments
        .windows(2)
        .find(|pair| pair[0].to_string_lossy() == "--config")
        .map(|pair| PathBuf::from(&pair[1]))
}
