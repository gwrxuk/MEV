use anyhow::{Context, Result};
use std::io;
use tracing_subscriber::{
    filter::EnvFilter,
    fmt::{self, format::FmtSpan},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::config::LoggingConfig;

/// Initialize the logging subsystem based on configuration
pub fn init(config: &LoggingConfig) -> Result<()> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.level));

    let fmt_layer = fmt::Layer::new()
        .with_span_events(FmtSpan::CLOSE)
        .with_target(true);

    let subscriber = tracing_subscriber::registry()
        .with(env_filter);

    if config.json_format {
        let json_layer = fmt::Layer::new()
            .json()
            .with_current_span(true)
            .with_span_list(true);
        
        subscriber.with(json_layer).init();
    } else {
        subscriber.with(fmt_layer).init();
    }

    // If a file path is provided, add file logging
    if let Some(file_path) = &config.file_path {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_path)
            .context("Failed to open log file")?;
        
        let file_layer = fmt::Layer::new()
            .with_writer(io::BufWriter::new(file))
            .with_ansi(false);
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }

    Ok(())
}

/// Helper to log unhandled errors within async contexts
pub fn log_error<E: std::fmt::Display>(err: E) {
    tracing::error!("Error: {}", err);
} 