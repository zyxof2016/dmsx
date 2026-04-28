mod app;
mod command_runner;
mod desktop;
#[cfg(windows)]
mod windows_service;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dmsx_agent=info".into()),
        )
        .init();

    #[cfg(windows)]
    {
        if let Some(command) = std::env::args().nth(1) {
            if command == "--windows-service" {
                if let Err(error) = windows_service::run_service() {
                    tracing::error!(%error, "windows service failed");
                }
                return;
            }
        }
    }

    app::run().await;
}
