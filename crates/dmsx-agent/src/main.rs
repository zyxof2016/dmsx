mod app;
mod command_runner;
mod desktop;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dmsx_agent=info".into()),
        )
        .init();

    app::run().await;
}
