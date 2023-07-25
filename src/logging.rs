use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Setup logging and error reporting
pub fn init(default: Level, filter: Option<&str>) {
    let filter = filter.unwrap_or_else(|| default.as_str());

    Registry::default()
        .with(
            EnvFilter::builder()
                .with_default_directive(default.into())
                .parse_lossy(filter),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .with_file(true)
                .with_line_number(true)
                .with_target(true),
        )
        .with(ErrorLayer::default())
        .init();
}
