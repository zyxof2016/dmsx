use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn init_tracing(service_name: &'static str) -> TelemetryGuard {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "dmsx_device_gw=info".into());
    let fmt_layer = tracing_subscriber::fmt::layer();

    let otlp_endpoint = std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    match otlp_endpoint {
        None => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .init();
            TelemetryGuard::noop()
        }
        Some(endpoint) => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_tonic()
                .with_endpoint(endpoint)
                .build()
                .expect("build OTLP span exporter");

            let resource = Resource::builder()
                .with_service_name(service_name)
                .build();

            let tracer_provider = SdkTracerProvider::builder()
                .with_resource(resource)
                .with_batch_exporter(exporter)
                .build();

            let tracer = tracer_provider.tracer("dmsx-device-gw");
            let otel_layer = tracing_opentelemetry::OpenTelemetryLayer::new(tracer);

            tracing_subscriber::registry()
                .with(filter)
                .with(fmt_layer)
                .with(otel_layer)
                .init();

            TelemetryGuard::otel(tracer_provider)
        }
    }
}

pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl TelemetryGuard {
    fn otel(tracer_provider: SdkTracerProvider) -> Self {
        Self {
            tracer_provider: Some(tracer_provider),
        }
    }
    fn noop() -> Self {
        Self { tracer_provider: None }
    }
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(tp) = self.tracer_provider.take() {
            let _ = tp.shutdown();
        }
    }
}

